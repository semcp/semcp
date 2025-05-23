// opa.rs - Open Policy Agent integration for snpx
//
// This module provides functionality to:
// 1. Parse snpx.yaml security policies
// 2. Translate them to OPA Rego policies
// 3. Interact with OPA for policy enforcement

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::Path;
use std::process::Command;

#[cfg(feature = "opa")]
use reqwest::Client;

// Basic structure matching snpx.yaml (can be expanded)
#[derive(Debug, Serialize, Deserialize)]
pub struct SecurityPolicy {
    #[serde(rename = "apiVersion")]
    pub api_version: String,
    pub kind: String,
    pub metadata: Metadata,
    pub spec: PolicySpec,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Metadata {
    pub name: String,
    pub description: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PolicySpec {
    pub docker: DockerSpec,
    pub network: NetworkSpec,
    pub filesystem: FilesystemSpec,
    pub runtime: RuntimeSpec,
    pub audit: AuditSpec,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DockerSpec {
    pub capabilities: CapabilitiesSpec,
    pub security_opts: Vec<String>,
    pub user: String,
    pub read_only_root_filesystem: bool,
    pub tmpfs: Vec<String>,
    pub ulimits: UlimitsSpec,
    pub memory_limit: String,
    pub cpu_limit: String,
    pub pids_limit: u32,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CapabilitiesSpec {
    pub drop: Vec<String>,
    pub add: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct UlimitsSpec {
    pub nproc: u32,
    pub nofile: u32,
    pub fsize: u64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct NetworkSpec {
    pub policy: String,
    pub dns_servers: Vec<String>,
    pub allowed_domains: Vec<String>,
    pub blocked_ports: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct FilesystemSpec {
    pub mount_options: Vec<String>,
    pub allowed_paths: Vec<String>,
    pub blocked_paths: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RuntimeSpec {
    pub timeout: String,
    pub max_restart_attempts: u32,
    pub environment_whitelist: Vec<String>,
    pub signal_handling: SignalHandlingSpec,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SignalHandlingSpec {
    pub graceful_shutdown_timeout: String,
    pub force_kill_timeout: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AuditSpec {
    pub log_level: String,
    pub log_commands: bool,
    pub log_network_access: bool,
    pub log_file_access: bool,
}

// OPA Policy Manager
pub struct OpaManager {
    #[cfg(feature = "opa")]
    client: Option<Client>,
    enabled: bool,
}

impl OpaManager {
    pub fn new(enabled: bool) -> Self {
        OpaManager {
            #[cfg(feature = "opa")]
            client: if enabled { Some(Client::new()) } else { None },
            enabled,
        }
    }

    // Load policy from YAML
    pub fn load_policy_from_yaml(&mut self, yaml_path: &Path) -> Result<SecurityPolicy> {
        let yaml_content = std::fs::read_to_string(yaml_path)
            .context(format!("Failed to read policy file: {:?}", yaml_path))?;
        
        let policy: SecurityPolicy = serde_yaml::from_str(&yaml_content)
            .context("Failed to parse YAML policy")?;
        
        Ok(policy)
    }

    // Convert YAML policy to Rego format
    pub fn policy_to_rego(&self, policy: &SecurityPolicy) -> Result<String> {
        // Start with basic package declaration
        let mut rego = String::from(
            "package snpx.policy\n\n# Auto-generated from snpx.yaml\n\n"
        );

        // Convert filesystem policies
        rego.push_str("# Filesystem policies\n");
        rego.push_str("allow_filesystem_access(path) {\n");
        rego.push_str("    # Path is in allowed paths\n");
        rego.push_str("    allowed_paths = [\n");
        for path in &policy.spec.filesystem.allowed_paths {
            rego.push_str(&format!("        \"{}\",\n", path));
        }
        rego.push_str("    ]\n");
        rego.push_str("    startswith_any(path, allowed_paths)\n");
        rego.push_str("}\n\n");
        
        rego.push_str("deny_filesystem_access(path) {\n");
        rego.push_str("    # Path is in blocked paths\n");
        rego.push_str("    blocked_paths = [\n");
        for path in &policy.spec.filesystem.blocked_paths {
            rego.push_str(&format!("        \"{}\",\n", path));
        }
        rego.push_str("    ]\n");
        rego.push_str("    startswith_any(path, blocked_paths)\n");
        rego.push_str("}\n\n");

        // Network policies
        rego.push_str("# Network policies\n");
        rego.push_str("allow_network_access(domain) {\n");
        rego.push_str("    # Domain is in allowed domains\n");
        rego.push_str("    allowed_domains = [\n");
        for domain in &policy.spec.network.allowed_domains {
            rego.push_str(&format!("        \"{}\",\n", domain));
        }
        rego.push_str("    ]\n");
        rego.push_str("    domain_matches(domain, allowed_domains)\n");
        rego.push_str("}\n\n");
        
        rego.push_str("deny_network_port(port) {\n");
        rego.push_str("    # Port is in blocked ports\n");
        rego.push_str("    blocked_ports = [\n");
        for port in &policy.spec.network.blocked_ports {
            rego.push_str(&format!("        \"{}\",\n", port));
        }
        rego.push_str("    ]\n");
        rego.push_str("    port == blocked_ports[_]\n");
        rego.push_str("}\n\n");

        // Helper functions
        rego.push_str("# Helper functions\n");
        rego.push_str("startswith_any(str, prefixes) {\n");
        rego.push_str("    startswith(str, prefixes[_])\n");
        rego.push_str("}\n\n");
        
        rego.push_str("domain_matches(domain, allowed) {\n");
        rego.push_str("    allowed[_] == domain\n");
        rego.push_str("}\n");
        
        rego.push_str("domain_matches(domain, allowed) {\n");
        rego.push_str("    endswith(domain, concat(\".\", allowed[_]))\n");
        rego.push_str("}\n");

        Ok(rego)
    }

    // Check if OPA is available
    pub fn check_opa_available(&self) -> Result<bool> {
        if !self.enabled {
            return Ok(false);
        }

        match Command::new("opa").args(["version"]).output() {
            Ok(output) => Ok(output.status.success()),
            Err(_) => Ok(false),
        }
    }

    // Helper method to create Docker args for running OPA as a sidecar
    pub fn create_opa_sidecar_args(&self, container_name: &str) -> Vec<String> {
        if !self.enabled {
            return Vec::new();
        }

        vec![
            "run".to_string(),
            "-d".to_string(),
            "--name".to_string(),
            format!("{}-opa", container_name),
            "--network=container:".to_string() + container_name,
            "-p".to_string(),
            "8181:8181".to_string(),
            "openpolicyagent/opa:latest".to_string(),
            "run".to_string(),
            "--server".to_string(),
            "--log-level=debug".to_string(),
        ]
    }

    // In a real implementation, these methods would be used to interact with OPA
    #[cfg(feature = "opa")]
    pub async fn upload_policy(&self, policy_id: &str, rego_policy: &str) -> Result<()> {
        if !self.enabled || self.client.is_none() {
            return Ok(());
        }

        // This would call the OPA API to upload the policy
        // In a real implementation, this would:
        // self.client.unwrap().put(format!("http://localhost:8181/v1/policies/{}", policy_id))
        //    .body(rego_policy.to_string())
        //    .send()
        //    .await?;
        
        Ok(())
    }

    #[cfg(feature = "opa")]
    pub async fn check_policy(&self, input: serde_json::Value) -> Result<bool> {
        if !self.enabled || self.client.is_none() {
            return Ok(true); // Allow by default if OPA is not enabled
        }

        // This would call the OPA API to check the policy
        // In a real implementation, this would:
        // let response = self.client.unwrap().post("http://localhost:8181/v1/data/snpx/policy/allow")
        //    .json(&input)
        //    .send()
        //    .await?;
        // let result: serde_json::Value = response.json().await?;
        // Parse the result and return whether the policy allows the action
        
        Ok(true) // For now, always allow
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::NamedTempFile;
    use std::io::Write;

    #[test]
    fn test_load_policy_from_yaml() {
        // Create a temporary YAML file with test policy
        let mut temp_file = NamedTempFile::new().unwrap();
        write!(
            temp_file,
            r#"
apiVersion: v1
kind: SecurityPolicy
metadata:
  name: test-policy
  description: Test policy
spec:
  docker:
    capabilities:
      drop:
        - ALL
      add:
        - SETUID
    security_opts:
      - no-new-privileges:true
    user: "1000:1000"
    read_only_root_filesystem: false
    tmpfs:
      - /tmp:noexec,nosuid,size=100m
    ulimits:
      nproc: 1024
      nofile: 65536
      fsize: 1073741824
    memory_limit: "512m"
    cpu_limit: "1.0"
    pids_limit: 256
  network:
    policy: "bridge"
    dns_servers:
      - "8.8.8.8"
    allowed_domains:
      - "registry.npmjs.org"
    blocked_ports:
      - "22"
  filesystem:
    mount_options:
      - ro
    allowed_paths:
      - "/usr/local/lib"
    blocked_paths:
      - "/proc/sys"
  runtime:
    timeout: "300s"
    max_restart_attempts: 3
    environment_whitelist:
      - "NODE_ENV"
    signal_handling:
      graceful_shutdown_timeout: "10s"
      force_kill_timeout: "30s"
  audit:
    log_level: "info"
    log_commands: true
    log_network_access: false
    log_file_access: false
"#
        )
        .unwrap();

        let mut manager = OpaManager::new(true);
        let policy = manager.load_policy_from_yaml(temp_file.path()).unwrap();

        assert_eq!(policy.api_version, "v1");
        assert_eq!(policy.kind, "SecurityPolicy");
        assert_eq!(policy.metadata.name, "test-policy");
        assert_eq!(policy.spec.docker.capabilities.drop[0], "ALL");
        assert_eq!(policy.spec.filesystem.allowed_paths[0], "/usr/local/lib");
    }

    #[test]
    fn test_policy_to_rego_conversion() {
        // Create a simple policy for conversion testing
        let policy = SecurityPolicy {
            api_version: "v1".to_string(),
            kind: "SecurityPolicy".to_string(),
            metadata: Metadata {
                name: "test-policy".to_string(),
                description: "Test policy".to_string(),
            },
            spec: PolicySpec {
                docker: DockerSpec {
                    capabilities: CapabilitiesSpec {
                        drop: vec!["ALL".to_string()],
                        add: vec!["SETUID".to_string()],
                    },
                    security_opts: vec!["no-new-privileges:true".to_string()],
                    user: "1000:1000".to_string(),
                    read_only_root_filesystem: false,
                    tmpfs: vec!["/tmp:noexec,nosuid,size=100m".to_string()],
                    ulimits: UlimitsSpec {
                        nproc: 1024,
                        nofile: 65536,
                        fsize: 1073741824,
                    },
                    memory_limit: "512m".to_string(),
                    cpu_limit: "1.0".to_string(),
                    pids_limit: 256,
                },
                network: NetworkSpec {
                    policy: "bridge".to_string(),
                    dns_servers: vec!["8.8.8.8".to_string()],
                    allowed_domains: vec!["registry.npmjs.org".to_string(), "github.com".to_string()],
                    blocked_ports: vec!["22".to_string(), "3306".to_string()],
                },
                filesystem: FilesystemSpec {
                    mount_options: vec!["ro".to_string()],
                    allowed_paths: vec!["/usr/local/lib".to_string(), "/tmp".to_string()],
                    blocked_paths: vec!["/proc/sys".to_string(), "/dev/mem".to_string()],
                },
                runtime: RuntimeSpec {
                    timeout: "300s".to_string(),
                    max_restart_attempts: 3,
                    environment_whitelist: vec!["NODE_ENV".to_string()],
                    signal_handling: SignalHandlingSpec {
                        graceful_shutdown_timeout: "10s".to_string(),
                        force_kill_timeout: "30s".to_string(),
                    },
                },
                audit: AuditSpec {
                    log_level: "info".to_string(),
                    log_commands: true,
                    log_network_access: false,
                    log_file_access: false,
                },
            },
        };

        let manager = OpaManager::new(true);
        let rego = manager.policy_to_rego(&policy).unwrap();

        // Check that key elements are in the Rego policy
        assert!(rego.contains("package snpx.policy"));
        assert!(rego.contains("allow_filesystem_access(path)"));
        assert!(rego.contains("\"/usr/local/lib\""));
        assert!(rego.contains("\"/tmp\""));
        assert!(rego.contains("deny_filesystem_access(path)"));
        assert!(rego.contains("\"/proc/sys\""));
        assert!(rego.contains("\"/dev/mem\""));
        assert!(rego.contains("allow_network_access(domain)"));
        assert!(rego.contains("\"registry.npmjs.org\""));
        assert!(rego.contains("\"github.com\""));
        assert!(rego.contains("deny_network_port(port)"));
        assert!(rego.contains("\"22\""));
        assert!(rego.contains("\"3306\""));
    }
}