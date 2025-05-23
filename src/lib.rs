use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitStatus};
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::process::Command as AsyncCommand;

#[derive(Debug, Clone)]
pub enum Transport {
    Stdio,
    Http,
    SSE,
}

// Security policy configuration structures
#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct SecurityPolicy {
    #[serde(rename = "apiVersion")]
    pub api_version: String,
    pub kind: String,
    pub metadata: Metadata,
    pub spec: PolicySpec,
}

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct Metadata {
    pub name: String,
    pub description: String,
}

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct PolicySpec {
    #[serde(default)]
    pub docker: DockerSpec,
    #[serde(default)]
    pub network: NetworkSpec,
    #[serde(default)]
    pub filesystem: FilesystemSpec,
    #[serde(default)]
    pub runtime: RuntimeSpec,
    #[serde(default)]
    pub audit: AuditSpec,
    #[serde(default)]
    pub falco: FalcoSpec,
}

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct DockerSpec {
    #[serde(default)]
    pub capabilities: DockerCapabilities,
    #[serde(default)]
    pub security_opts: Vec<String>,
    #[serde(default)]
    pub user: String,
    #[serde(default)]
    pub read_only_root_filesystem: bool,
    #[serde(default)]
    pub tmpfs: Vec<String>,
    #[serde(default)]
    pub ulimits: DockerUlimits,
    #[serde(default)]
    pub memory_limit: String,
    #[serde(default)]
    pub cpu_limit: String,
    #[serde(default)]
    pub pids_limit: u32,
}

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct DockerCapabilities {
    #[serde(default)]
    pub drop: Vec<String>,
    #[serde(default)]
    pub add: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct DockerUlimits {
    #[serde(default)]
    pub nproc: u32,
    #[serde(default)]
    pub nofile: u32,
    #[serde(default)]
    pub fsize: u64,
}

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct NetworkSpec {
    #[serde(default)]
    pub policy: String,
    #[serde(default)]
    pub dns_servers: Vec<String>,
    #[serde(default)]
    pub allowed_domains: Vec<String>,
    #[serde(default)]
    pub blocked_ports: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct FilesystemSpec {
    #[serde(default)]
    pub mount_options: Vec<String>,
    #[serde(default)]
    pub allowed_paths: Vec<String>,
    #[serde(default)]
    pub blocked_paths: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct RuntimeSpec {
    #[serde(default)]
    pub timeout: String,
    #[serde(default)]
    pub max_restart_attempts: u32,
    #[serde(default)]
    pub environment_whitelist: Vec<String>,
    #[serde(default)]
    pub signal_handling: SignalHandling,
}

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct SignalHandling {
    #[serde(default)]
    pub graceful_shutdown_timeout: String,
    #[serde(default)]
    pub force_kill_timeout: String,
}

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct AuditSpec {
    #[serde(default)]
    pub log_level: String,
    #[serde(default)]
    pub log_commands: bool,
    #[serde(default)]
    pub log_network_access: bool,
    #[serde(default)]
    pub log_file_access: bool,
}

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct FalcoSpec {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default)]
    pub rules: Vec<FalcoRuleSet>,
}

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct FalcoRuleSet {
    pub name: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub enabled: bool,
    #[serde(default)]
    pub rules: Vec<FalcoRule>,
    #[serde(default)]
    pub rule_content: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct FalcoRule {
    pub name: String,
    #[serde(default)]
    pub description: String,
    pub condition: String,
    #[serde(default)]
    pub output: String,
    #[serde(default)]
    pub priority: String,
    #[serde(default)]
    pub action: String,
}

impl SecurityPolicy {
    pub fn load_from_file<P: AsRef<Path>>(path: P) -> Result<Self> {
        let content = fs::read_to_string(path)?;
        let policy: SecurityPolicy = serde_yaml::from_str(&content)?;
        Ok(policy)
    }

    pub fn find_and_load() -> Result<Self> {
        // Search for the policy file in standard locations
        let locations = vec![
            Some(PathBuf::from("./snpx.yaml")),
            Some(PathBuf::from("./snpx.yml")),
            dirs_next::home_dir().map(|p| p.join(".snpx.yaml")),
            dirs_next::config_dir().map(|p| p.join("snpx/config.yaml")),
        ];

        for location in locations.into_iter().flatten() {
            if location.exists() {
                return Self::load_from_file(location);
            }
        }

        // If no policy file is found, return the default
        Ok(SecurityPolicy::default())
    }

    pub fn generate_falco_rule_file(&self) -> Result<Option<PathBuf>> {
        if !self.spec.falco.enabled || self.spec.falco.rules.is_empty() {
            return Ok(None);
        }

        let temp_dir = std::env::temp_dir();
        let rule_file_path = temp_dir.join(format!("snpx-falco-rules-{}.yaml", std::process::id()));

        let mut rule_content = "# Falco rules generated by snpx\n".to_string();

        for rule_set in &self.spec.falco.rules {
            if !rule_set.enabled {
                continue;
            }

            // Handle inline rule content
            if let Some(content) = &rule_set.rule_content {
                rule_content.push_str(content);
                rule_content.push('\n');
                continue;
            }

            // Handle structured rules
            for rule in &rule_set.rules {
                rule_content.push_str(&format!("- rule: {}\n", rule.name));

                if !rule.description.is_empty() {
                    rule_content.push_str(&format!("  desc: {}\n", rule.description));
                }

                rule_content.push_str(&format!("  condition: {}\n", rule.condition));

                if !rule.output.is_empty() {
                    rule_content.push_str(&format!("  output: {}\n", rule.output));
                }

                if !rule.priority.is_empty() {
                    rule_content.push_str(&format!("  priority: {}\n", rule.priority));
                }

                if !rule.action.is_empty() {
                    rule_content.push_str(&format!("  action: {}\n", rule.action));
                }

                rule_content.push('\n');
            }
        }

        fs::write(&rule_file_path, rule_content)?;
        Ok(Some(rule_file_path))
    }
}

pub struct ImageVariants;

impl ImageVariants {
    pub const ALPINE: &'static str = "node:24-alpine";
    pub const SLIM: &'static str = "node:24-slim";
    pub const STANDARD: &'static str = "node:24";
    pub const DISTROLESS: &'static str = "gcr.io/distroless/nodejs24-debian12";

    pub fn get_recommended() -> &'static str {
        Self::ALPINE
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
    security_policy: Option<SecurityPolicy>,
    falco_enabled: bool,
}

impl ContainerExecutor {
    pub fn new(docker_image: String, verbose: bool) -> Self {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let container_name = format!("snpx-{}-{}", std::process::id(), timestamp);

        // Try to load the security policy, but don't fail if it's not found
        let security_policy = SecurityPolicy::find_and_load().ok();
        let falco_enabled = security_policy
            .as_ref()
            .is_some_and(|p| p.spec.falco.enabled);

        Self {
            docker_image,
            verbose,
            container_name,
            security_policy,
            falco_enabled,
        }
    }

    // Enable Falco monitoring explicitly
    pub fn with_falco(mut self, enabled: bool) -> Self {
        self.falco_enabled = enabled;
        self
    }

    pub fn new_optimized(verbose: bool) -> Self {
        Self::new(ImageVariants::get_recommended().to_string(), verbose)
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
    ) -> Result<Vec<String>> {
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

        // Add Falco integration if enabled
        if self.falco_enabled {
            if let Some(policy) = &self.security_policy {
                if let Ok(Some(rule_file_path)) = policy.generate_falco_rule_file() {
                    // Add a label to the container for Falco to target
                    docker_args.push("--label".to_string());
                    docker_args.push(format!("falco.rules.file={}", rule_file_path.display()));

                    // Add a label to indicate this container should be monitored by Falco
                    docker_args.push("--label".to_string());
                    docker_args.push("io.kubernetes.pod.namespace=falco-monitored".to_string());

                    if self.verbose {
                        eprintln!(
                            "Falco monitoring enabled with rules: {}",
                            rule_file_path.display()
                        );
                    }
                }
            }
        }

        docker_args.extend(runner.additional_docker_args());
        docker_args.push(self.docker_image.clone());
        docker_args.extend(cmd_args.iter().cloned());

        Ok(docker_args)
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
        let docker_args = self.create_docker_args(runner, &cmd_args, &transport)?;

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
            executor: ContainerExecutor::new(docker_image, verbose),
        }
    }

    pub fn new_optimized(verbose: bool) -> Self {
        Self {
            executor: ContainerExecutor::new_optimized(verbose),
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

    pub fn with_falco(mut self, enabled: bool) -> Self {
        self.executor = self.executor.with_falco(enabled);
        self
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
        self.executor
            .create_docker_args(self, &cmd_args, transport)
            .unwrap_or_else(|_| {
                // Fallback to simpler docker args if error occurs
                let mut args = vec![
                    "run".to_string(),
                    "--rm".to_string(),
                    "-i".to_string(),
                    self.executor.image().to_string(),
                ];
                args.extend(cmd_args.iter().cloned());
                args
            })
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

    fn detect_transport(&self, _package: &str) -> Transport {
        // For now, always use Stdio transport, but could be expanded based on package name patterns
        Transport::Stdio
    }

    fn requires_tty(&self, transport: &Transport) -> bool {
        !matches!(transport, Transport::Stdio)
    }
}

pub type SnpxRunner = NpxRunner;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_image_variants() {
        assert_eq!(ImageVariants::ALPINE, "node:24-alpine");
        assert_eq!(ImageVariants::SLIM, "node:24-slim");
        assert_eq!(ImageVariants::STANDARD, "node:24");
        assert_eq!(ImageVariants::get_recommended(), "node:24-alpine");
    }

    #[test]
    fn test_optimized_constructors() {
        let alpine_runner = NpxRunner::new_alpine(false);
        let slim_runner = NpxRunner::new_slim(false);
        let optimized_runner = NpxRunner::new_optimized(false);

        assert_eq!(alpine_runner.image(), "node:24-alpine");
        assert_eq!(slim_runner.image(), "node:24-slim");
        assert_eq!(optimized_runner.image(), "node:24-alpine");
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
    }

    #[test]
    fn test_container_name_generation() {
        let runner1 = NpxRunner::new("node:20".to_string(), false);
        std::thread::sleep(std::time::Duration::from_nanos(1));
        let runner2 = NpxRunner::new("node:20".to_string(), false);

        assert_ne!(runner1.container_name(), runner2.container_name());
        assert!(runner1.container_name().starts_with("snpx-"));
        assert!(runner2.container_name().starts_with("snpx-"));
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
    }

    #[test]
    fn test_falco_integration() {
        let _runner = NpxRunner::new("node:20".to_string(), false).with_falco(true);

        // Create a minimal policy to test with
        let policy = SecurityPolicy {
            api_version: "v1".to_string(),
            kind: "SecurityPolicy".to_string(),
            metadata: Metadata {
                name: "test-policy".to_string(),
                description: "Test policy".to_string(),
            },
            spec: PolicySpec {
                falco: FalcoSpec {
                    enabled: true,
                    rules: vec![FalcoRuleSet {
                        name: "test-ruleset".to_string(),
                        description: "Test ruleset".to_string(),
                        enabled: true,
                        rules: vec![FalcoRule {
                            name: "test-rule".to_string(),
                            description: "Test rule".to_string(),
                            condition: "open_write and fd.name = \"/etc/passwd\"".to_string(),
                            output: "Write to /etc/passwd (user=%user.name)".to_string(),
                            priority: "WARNING".to_string(),
                            action: "terminate".to_string(),
                        }],
                        rule_content: None,
                    }],
                },
                ..Default::default()
            },
        };

        // Generate a Falco rule file
        let rule_file = policy.generate_falco_rule_file();
        assert!(rule_file.is_ok());

        if let Ok(Some(path)) = rule_file {
            assert!(path.exists());

            // Clean up
            let _ = std::fs::remove_file(path);
        }
    }
}
