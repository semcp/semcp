use anyhow::{Context, Result};
use policy_mcp::{AccessType, PolicyDocument, PolicyParser};

#[derive(Debug, Clone)]
pub struct PolicyConfig {
    pub policy: Option<PolicyDocument>,
}

impl PolicyConfig {
    pub fn new() -> Self {
        Self { policy: None }
    }

    pub fn from_file(path: &str) -> Result<Self> {
        let policy = PolicyParser::parse_file(path).context("Failed to parse policy file")?;
        Ok(Self {
            policy: Some(policy),
        })
    }

    pub fn map_docker_security_args(&self) -> Vec<String> {
        let mut args = Vec::new();

        if let Some(ref policy) = self.policy {
            if let Some(ref runtime) = policy.permissions.runtime {
                if let Some(ref docker) = runtime.docker {
                    if let Some(ref security) = docker.security {
                        if let Some(privileged) = security.privileged {
                            if !privileged {
                                args.push("--security-opt".to_string());
                                args.push("no-new-privileges".to_string());
                            }
                        }
                        if let Some(ref capabilities) = security.capabilities {
                            if let Some(ref drop_caps) = capabilities.drop {
                                for cap in drop_caps {
                                    args.push("--cap-drop".to_string());
                                    args.push(format!("{:?}", cap));
                                }
                            }
                            if let Some(ref add_caps) = capabilities.add {
                                for cap in add_caps {
                                    args.push("--cap-add".to_string());
                                    args.push(format!("{:?}", cap));
                                }
                            }
                        }
                    }
                }
            }
        }
        args
    }

    pub fn map_file_mounts(&self) -> Vec<String> {
        let mut mounts = Vec::new();

        if let Some(ref policy) = self.policy {
            if let Some(ref storage) = policy.permissions.storage {
                if let Some(ref allow_list) = storage.allow {
                    for storage_permission in allow_list {
                        if storage_permission.uri.starts_with("fs://") {
                            let path = &storage_permission.uri[5..];
                            let readonly = !storage_permission.access.contains(&AccessType::Write);
                            let mode = if readonly { "ro" } else { "rw" };

                            mounts.push("-v".to_string());
                            mounts.push(format!("{}:{}:{}", path, path, mode));
                        }
                    }
                }
            }
        }
        mounts
    }

    pub fn get_all_docker_args(&self) -> Vec<String> {
        let mut args = Vec::new();
        args.extend(self.map_file_mounts());
        args.extend(self.map_docker_security_args());
        args
    }
}

impl Default for PolicyConfig {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_policy_config_new() {
        let config = PolicyConfig::new();
        assert!(config.policy.is_none());
    }

    #[test]
    fn test_policy_config_default() {
        let config = PolicyConfig::default();
        assert!(config.policy.is_none());
    }

    #[test]
    fn test_empty_policy_docker_args() {
        let config = PolicyConfig::new();
        let args = config.get_all_docker_args();
        assert!(args.is_empty());
    }
}
