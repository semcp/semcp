use anyhow::{Context, Result};
use std::process::{Command, ExitStatus};
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::process::Command as AsyncCommand;

pub mod policy;
pub use policy::PolicyConfig;

#[derive(Debug, Clone)]
pub enum Transport {
    Stdio,
    Http,
    SSE,
}

pub struct ImageVariants;

impl ImageVariants {
    pub const NODE_ALPINE: &'static str = "node:24-alpine";
    pub const NODE_SLIM: &'static str = "node:24-slim";
    pub const NODE_STANDARD: &'static str = "node:24";
    pub const NODE_DISTROLESS: &'static str = "gcr.io/distroless/nodejs24-debian12";
    
    pub const PYTHON_ALPINE: &'static str = "ghcr.io/astral-sh/uv:python3.12-alpine";
    pub const PYTHON_SLIM: &'static str = "ghcr.io/astral-sh/uv:python3.12-bookworm-slim";
    pub const PYTHON_STANDARD: &'static str = "ghcr.io/astral-sh/uv:python3.12-bookworm";

    pub fn get_node_recommended() -> &'static str {
        Self::NODE_ALPINE
    }
    
    pub fn get_python_recommended() -> &'static str {
        Self::PYTHON_ALPINE
    }
}

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
        false
    }
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
    policy_config: PolicyConfig,
}

impl ContainerExecutor {
    pub fn new(docker_image: String, verbose: bool) -> Self {
        Self::with_policy(docker_image, verbose, PolicyConfig::new())
    }

    pub fn with_policy(docker_image: String, verbose: bool, policy_config: PolicyConfig) -> Self {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let container_name = format!("container-{}-{}", std::process::id(), timestamp);
        Self {
            docker_image,
            verbose,
            container_name,
            policy_config,
        }
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

        docker_args.extend(self.policy_config.get_all_docker_args());
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