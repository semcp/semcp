use anyhow::{Context, Result};
use std::process::{Command, ExitStatus};
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::process::Command as AsyncCommand;

#[derive(Debug, Clone)]
pub enum Transport {
    Stdio,
    Http,
    SSE,
}

pub struct ImageVariants;

impl ImageVariants {
    // Node.js images
    pub const ALPINE: &'static str = "node:24-alpine";
    pub const SLIM: &'static str = "node:24-slim";
    pub const STANDARD: &'static str = "node:24";
    pub const DISTROLESS: &'static str = "gcr.io/distroless/nodejs24-debian12";

    // Python/uv images
    pub const PYTHON_ALPINE: &'static str = "python:3.12-alpine";
    pub const PYTHON_SLIM: &'static str = "python:3.12-slim-bookworm";
    pub const PYTHON_STANDARD: &'static str = "python:3.12-bookworm";
    
    pub fn get_recommended() -> &'static str {
        Self::ALPINE
    }
    
    pub fn get_recommended_python() -> &'static str {
        Self::PYTHON_ALPINE
    }
}

/// A trait for a runner, which runs a command in a container.
pub trait Runner {
    fn command(&self) -> &str;
    fn default_image(&self) -> &str;
    fn default_flags(&self) -> Vec<String>;
    fn detect_transport(&self, package: &str) -> Transport;
    fn requires_tty(&self, transport: &Transport) -> bool;
    fn additional_docker_args(&self) -> Vec<String> {
        vec![]
    }
    fn supports_fallback(&self) -> bool {
        true
    }
    /// flags are runner specific flags and arguments are the command arguments.
    /// e.g. for npx, flags are the npx flags and arguments are the command arguments.
    /// npx -y cowsay hello
    /// flags = ["-y"]
    /// args = ["cowsay", "hello"]
    fn build_command_args(&self, flags: &[String], args: &[String]) -> Vec<String> {
        let mut cmd_args = vec![self.command().to_string()];
        cmd_args.extend(flags.iter().cloned());
        cmd_args.extend(args.iter().cloned());
        cmd_args
    }
}

pub struct ContainerExecutor {
    docker_image: String,
    verbose: bool,
    container_name: String,
}

impl ContainerExecutor {
    pub fn new(docker_image: String, verbose: bool) -> Self {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let container_name = format!("snpx-{}-{}", std::process::id(), timestamp);
        Self {
            docker_image,
            verbose,
            container_name,
        }
    }

    pub fn new_with_prefix(docker_image: String, verbose: bool, prefix: &str) -> Self {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let container_name = format!("{}-{}-{}", prefix, std::process::id(), timestamp);
        Self {
            docker_image,
            verbose,
            container_name,
        }
    }

    pub fn new_optimized(verbose: bool) -> Self {
        Self::new_with_prefix(ImageVariants::get_recommended().to_string(), verbose, "snpx")
    }

    pub fn check_docker_available(&self) -> Result<bool> {
        match which::which("docker") {
            Ok(_) => {
                let output = Command::new("docker")
                    .args(["--version"])
                    .output()
                    .context("Failed to execute docker --version")?;
                Ok(output.status.success())
            }
            Err(_) => Ok(false),
        }
    }

    pub fn create_docker_args<R: Runner>(
        &self,
        runner: &R,
        cmd_args: &[String],
        transport: &Transport,
    ) -> Vec<String> {
        let mut docker_args = vec![
            "run".to_string(),
            "--rm".to_string(),
            "-i".to_string(),
            "--name".to_string(),
            self.container_name.clone(),
        ];

        if runner.requires_tty(transport) {
            docker_args.push("-t".to_string());
        }

        docker_args.extend(runner.additional_docker_args());
        docker_args.push(self.docker_image.clone());
        docker_args.extend(cmd_args.iter().cloned());

        docker_args
    }

    pub async fn run_containerized<R: Runner>(
        &self,
        runner: &R,
        flags: &[String],
        args: &[String],
    ) -> Result<ExitStatus> {
        let empty_string = String::new();
        let package_name = args.first().unwrap_or(&empty_string);
        let transport = runner.detect_transport(package_name);
        let cmd_args = runner.build_command_args(flags, args);
        let docker_args = self.create_docker_args(runner, &cmd_args, &transport);

        if self.verbose {
            let docker_cmd = format!("docker {}", docker_args.join(" "));
            eprintln!("Running: {}", docker_cmd);
        }

        let mut child = AsyncCommand::new("docker")
            .args(docker_args)
            .spawn()
            .context("Failed to spawn docker command")?;

        tokio::select! {
            result = child.wait() => {
                result.context("Failed to wait for docker command")
            }
            _ = tokio::signal::ctrl_c() => {
                if self.verbose {
                    eprintln!("Received Ctrl+C, cleaning up container...");
                }
                self.cleanup().await?;
                std::process::exit(130);
            }
        }
    }

    pub fn run_fallback<R: Runner>(
        &self,
        runner: &R,
        flags: &[String],
        args: &[String],
    ) -> Result<ExitStatus> {
        if !runner.supports_fallback() {
            return Err(anyhow::anyhow!("Fallback not supported for this runner"));
        }

        if self.verbose {
            eprintln!("Falling back to regular {}", runner.command());
        }

        let mut cmd = Command::new(runner.command());
        cmd.args(flags);
        cmd.args(args);

        let status = cmd.status().context("Failed to execute command")?;
        Ok(status)
    }

    pub async fn cleanup(&self) -> Result<()> {
        let _output = AsyncCommand::new("docker")
            .args(["stop", &self.container_name])
            .output()
            .await;
        Ok(())
    }

    pub fn verbose(&self) -> bool {
        self.verbose
    }

    pub fn container_name(&self) -> &str {
        &self.container_name
    }

    pub fn image(&self) -> &str {
        &self.docker_image
    }
}

pub struct NpxRunner {
    executor: ContainerExecutor,
}

impl NpxRunner {
    pub fn new(docker_image: String, verbose: bool) -> Self {
        Self {
            executor: ContainerExecutor::new_with_prefix(docker_image, verbose, "snpx"),
        }
    }

    pub fn new_optimized(verbose: bool) -> Self {
        Self {
            executor: ContainerExecutor::new_with_prefix(
                ImageVariants::get_recommended().to_string(), 
                verbose,
                "snpx"
            ),
        }
    }

    pub fn new_alpine(verbose: bool) -> Self {
        Self::new(ImageVariants::ALPINE.to_string(), verbose)
    }

    pub fn new_slim(verbose: bool) -> Self {
        Self::new(ImageVariants::SLIM.to_string(), verbose)
    }

    pub fn new_distroless(verbose: bool) -> Self {
        Self::new(ImageVariants::DISTROLESS.to_string(), verbose)
    }

    pub fn check_docker_available(&self) -> Result<bool> {
        self.executor.check_docker_available()
    }

    pub async fn run_containerized_npx(&self, npx_args: &[String]) -> Result<ExitStatus> {
        self.run_containerized_npx_with_flags(&["-y".to_string()], npx_args)
            .await
    }

    pub async fn run_containerized_npx_with_flags(
        &self,
        npx_flags: &[String],
        npx_args: &[String],
    ) -> Result<ExitStatus> {
        self.executor
            .run_containerized(self, npx_flags, npx_args)
            .await
    }

    pub fn run_fallback_npx(&self, npx_args: &[String]) -> Result<ExitStatus> {
        self.run_fallback_npx_with_flags(&["-y".to_string()], npx_args)
    }

    pub fn run_fallback_npx_with_flags(
        &self,
        npx_flags: &[String],
        npx_args: &[String],
    ) -> Result<ExitStatus> {
        self.executor.run_fallback(self, npx_flags, npx_args)
    }

    pub async fn cleanup(&self) -> Result<()> {
        self.executor.cleanup().await
    }

    pub fn verbose(&self) -> bool {
        self.executor.verbose()
    }

    pub fn container_name(&self) -> &str {
        self.executor.container_name()
    }

    pub fn image(&self) -> &str {
        self.executor.image()
    }

    pub fn create_docker_args(&self, npx_args: &[String], transport: &Transport) -> Vec<String> {
        self.create_docker_args_with_flags(&[], npx_args, transport)
    }

    pub fn create_docker_args_with_flags(
        &self,
        npx_flags: &[String],
        npx_args: &[String],
        transport: &Transport,
    ) -> Vec<String> {
        let cmd_args = self.build_command_args(npx_flags, npx_args);
        self.executor.create_docker_args(self, &cmd_args, transport)
    }
}

impl Runner for NpxRunner {
    fn command(&self) -> &str {
        "npx"
    }

    fn default_image(&self) -> &str {
        ImageVariants::get_recommended()
    }

    fn default_flags(&self) -> Vec<String> {
        vec!["-y".to_string()]
    }

    fn detect_transport(&self, package: &str) -> Transport {
        if package.to_lowercase().contains("server")
            && (package.to_lowercase().contains("mcp")
                || package.to_lowercase().contains("modelcontextprotocol"))
        {
            Transport::Stdio
        } else {
            Transport::Stdio
        }
    }

    fn requires_tty(&self, transport: &Transport) -> bool {
        !matches!(transport, Transport::Stdio)
    }
}

pub type SnpxRunner = NpxRunner;

pub struct UvRunner {
    executor: ContainerExecutor,
}

impl UvRunner {
    pub fn new(docker_image: String, verbose: bool) -> Self {
        Self {
            executor: ContainerExecutor::new_with_prefix(docker_image, verbose, "suv"),
        }
    }

    pub fn new_optimized(verbose: bool) -> Self {
        Self {
            executor: ContainerExecutor::new_with_prefix(
                ImageVariants::get_recommended_python().to_string(), 
                verbose,
                "suv"
            ),
        }
    }

    pub fn new_alpine(verbose: bool) -> Self {
        Self::new(ImageVariants::PYTHON_ALPINE.to_string(), verbose)
    }

    pub fn new_slim(verbose: bool) -> Self {
        Self::new(ImageVariants::PYTHON_SLIM.to_string(), verbose)
    }

    pub fn new_standard(verbose: bool) -> Self {
        Self::new(ImageVariants::PYTHON_STANDARD.to_string(), verbose)
    }

    pub fn check_docker_available(&self) -> Result<bool> {
        self.executor.check_docker_available()
    }

    pub async fn run_containerized_uv(&self, uv_args: &[String]) -> Result<ExitStatus> {
        self.run_containerized_uv_with_flags(&[], uv_args)
            .await
    }

    pub async fn run_containerized_uv_with_flags(
        &self,
        uv_flags: &[String],
        uv_args: &[String],
    ) -> Result<ExitStatus> {
        self.executor
            .run_containerized(self, uv_flags, uv_args)
            .await
    }

    pub fn run_fallback_uv(&self, uv_args: &[String]) -> Result<ExitStatus> {
        self.run_fallback_uv_with_flags(&[], uv_args)
    }

    pub fn run_fallback_uv_with_flags(
        &self,
        uv_flags: &[String],
        uv_args: &[String],
    ) -> Result<ExitStatus> {
        self.executor.run_fallback(self, uv_flags, uv_args)
    }

    pub async fn cleanup(&self) -> Result<()> {
        self.executor.cleanup().await
    }

    pub fn verbose(&self) -> bool {
        self.executor.verbose()
    }

    pub fn container_name(&self) -> &str {
        self.executor.container_name()
    }

    pub fn image(&self) -> &str {
        self.executor.image()
    }

    pub fn create_docker_args(&self, uv_args: &[String], transport: &Transport) -> Vec<String> {
        self.create_docker_args_with_flags(&[], uv_args, transport)
    }

    pub fn create_docker_args_with_flags(
        &self,
        uv_flags: &[String],
        uv_args: &[String],
        transport: &Transport,
    ) -> Vec<String> {
        let cmd_args = self.build_command_args(uv_flags, uv_args);
        self.executor.create_docker_args(self, &cmd_args, transport)
    }
}

impl Runner for UvRunner {
    fn command(&self) -> &str {
        "uv"
    }

    fn default_image(&self) -> &str {
        ImageVariants::get_recommended_python()
    }

    fn default_flags(&self) -> Vec<String> {
        vec![]
    }

    fn detect_transport(&self, package: &str) -> Transport {
        if package.to_lowercase().contains("server")
            && (package.to_lowercase().contains("mcp")
                || package.to_lowercase().contains("modelcontextprotocol"))
        {
            Transport::Stdio
        } else {
            Transport::Stdio
        }
    }

    fn requires_tty(&self, transport: &Transport) -> bool {
        !matches!(transport, Transport::Stdio)
    }
}

pub type SuvRunner = UvRunner;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_image_variants() {
        assert_eq!(ImageVariants::ALPINE, "node:24-alpine");
        assert_eq!(ImageVariants::SLIM, "node:24-slim");
        assert_eq!(ImageVariants::STANDARD, "node:24");
        assert_eq!(ImageVariants::get_recommended(), "node:24-alpine");
        
        assert_eq!(ImageVariants::PYTHON_ALPINE, "python:3.12-alpine");
        assert_eq!(ImageVariants::PYTHON_SLIM, "python:3.12-slim-bookworm");
        assert_eq!(ImageVariants::PYTHON_STANDARD, "python:3.12-bookworm");
        assert_eq!(ImageVariants::get_recommended_python(), "python:3.12-alpine");
    }

    #[test]
    fn test_optimized_constructors() {
        let alpine_runner = NpxRunner::new_alpine(false);
        let slim_runner = NpxRunner::new_slim(false);
        let optimized_runner = NpxRunner::new_optimized(false);

        assert_eq!(alpine_runner.image(), "node:24-alpine");
        assert_eq!(slim_runner.image(), "node:24-slim");
        assert_eq!(optimized_runner.image(), "node:24-alpine");
        
        // Test UvRunner constructors
        let alpine_runner = UvRunner::new_alpine(false);
        let slim_runner = UvRunner::new_slim(false);
        let standard_runner = UvRunner::new_standard(false);
        let optimized_runner = UvRunner::new_optimized(false);

        assert_eq!(alpine_runner.image(), "python:3.12-alpine");
        assert_eq!(slim_runner.image(), "python:3.12-slim-bookworm");
        assert_eq!(standard_runner.image(), "python:3.12-bookworm");
        assert_eq!(optimized_runner.image(), "python:3.12-alpine");
    }

    #[test]
    fn test_mcp_transport_detection() {
        let runner = NpxRunner::new("node:20".to_string(), false);

        assert!(matches!(
            runner.detect_transport("@modelcontextprotocol/server-sequential-thinking"),
            Transport::Stdio
        ));

        assert!(matches!(
            runner.detect_transport("some-other-package"),
            Transport::Stdio
        ));
        
        // Test UvRunner transport detection
        let runner = UvRunner::new("python:3.12".to_string(), false);

        assert!(matches!(
            runner.detect_transport("mcp-server-time"),
            Transport::Stdio
        ));

        assert!(matches!(
            runner.detect_transport("some-other-package"),
            Transport::Stdio
        ));
    }

    #[test]
    fn test_docker_args_creation() {
        let runner = NpxRunner::new("node:20".to_string(), false);

        let npx_args = vec!["@modelcontextprotocol/server-sequential-thinking".to_string()];
        let stdio_transport = Transport::Stdio;

        let docker_args = runner.create_docker_args(&npx_args, &stdio_transport);

        assert!(docker_args.contains(&"run".to_string()));
        assert!(docker_args.contains(&"--rm".to_string()));
        assert!(docker_args.contains(&"-i".to_string()));
        assert!(!docker_args.contains(&"-t".to_string()));
        assert!(docker_args.contains(&"node:20".to_string()));
        assert!(docker_args.contains(&"npx".to_string()));
        assert!(
            docker_args.contains(&"@modelcontextprotocol/server-sequential-thinking".to_string())
        );

        let http_transport = Transport::Http;
        let docker_args_http = runner.create_docker_args(&npx_args, &http_transport);

        assert!(docker_args_http.contains(&"run".to_string()));
        assert!(docker_args_http.contains(&"--rm".to_string()));
        assert!(docker_args_http.contains(&"-i".to_string()));
        assert!(docker_args_http.contains(&"-t".to_string()));
        assert!(docker_args_http.contains(&"node:20".to_string()));
        assert!(docker_args_http.contains(&"npx".to_string()));
        
        // Test UvRunner docker args creation
        let runner = UvRunner::new("python:3.12".to_string(), false);

        let uv_args = vec!["mcp-server-time".to_string()];
        let stdio_transport = Transport::Stdio;

        let docker_args = runner.create_docker_args(&uv_args, &stdio_transport);

        assert!(docker_args.contains(&"run".to_string()));
        assert!(docker_args.contains(&"--rm".to_string()));
        assert!(docker_args.contains(&"-i".to_string()));
        assert!(!docker_args.contains(&"-t".to_string()));
        assert!(docker_args.contains(&"python:3.12".to_string()));
        assert!(docker_args.contains(&"uv".to_string()));
        assert!(docker_args.contains(&"mcp-server-time".to_string()));
    }

    #[test]
    fn test_container_name_generation() {
        let runner1 = NpxRunner::new("node:20".to_string(), false);
        std::thread::sleep(std::time::Duration::from_nanos(1));
        let runner2 = NpxRunner::new("node:20".to_string(), false);

        assert_ne!(runner1.container_name(), runner2.container_name());
        assert!(runner1.container_name().starts_with("snpx-"));
        assert!(runner2.container_name().starts_with("snpx-"));
        
        // Test UvRunner container name generation
        let runner1 = UvRunner::new("python:3.12".to_string(), false);
        std::thread::sleep(std::time::Duration::from_nanos(1));
        let runner2 = UvRunner::new("python:3.12".to_string(), false);

        assert_ne!(runner1.container_name(), runner2.container_name());
        assert!(runner1.container_name().starts_with("suv-"));
        assert!(runner2.container_name().starts_with("suv-"));
    }

    #[test]
    fn test_containerized_runner_trait() {
        let runner = NpxRunner::new("node:20".to_string(), false);

        assert_eq!(runner.command(), "npx");
        assert_eq!(runner.default_image(), "node:24-alpine");
        assert_eq!(runner.default_flags(), vec!["-y".to_string()]);
        assert!(runner.supports_fallback());

        let transport = Transport::Stdio;
        assert!(!runner.requires_tty(&transport));

        let transport = Transport::Http;
        assert!(runner.requires_tty(&transport));
        
        // Test UvRunner trait implementation
        let runner = UvRunner::new("python:3.12".to_string(), false);

        assert_eq!(runner.command(), "uv");
        assert_eq!(runner.default_image(), "python:3.12-alpine");
        assert_eq!(runner.default_flags(), Vec::<String>::new());
        assert!(runner.supports_fallback());

        let transport = Transport::Stdio;
        assert!(!runner.requires_tty(&transport));

        let transport = Transport::Http;
        assert!(runner.requires_tty(&transport));
    }
}
