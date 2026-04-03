use anyhow::Result;
use secrecy::SecretString;
use std::process::Command;

use crate::core::token::{BackendType, TokenMeta};
use super::gh::ImportEntry;

/// Scan for Docker credentials
/// Docker on macOS uses the desktop credential helper, which we can't directly read.
/// Instead we check if docker is logged in and note it.
pub fn scan() -> Result<Vec<ImportEntry>> {
    // Check docker config for logged-in registries
    let config_path = dirs::home_dir()
        .map(|h| h.join(".docker/config.json"));

    let config_path = match config_path {
        Some(p) if p.exists() => p,
        _ => return Ok(Vec::new()),
    };

    let content = std::fs::read_to_string(&config_path)?;
    let config: serde_json::Value = serde_json::from_str(&content)?;

    let mut entries = Vec::new();

    // Check "auths" field for registries
    if let Some(auths) = config.get("auths").and_then(|v| v.as_object()) {
        for (registry, _auth_data) in auths {
            if registry.is_empty() {
                continue;
            }

            // Try to get the credential via docker credential helper
            let token = try_get_docker_credential(registry);

            if let Some((username, password)) = token {
                let service_name = if registry.contains("docker.io") || registry.contains("index.docker.io") {
                    "docker-hub".to_string()
                } else {
                    format!("docker-{}", registry.split('/').next().unwrap_or(registry))
                };

                let mut meta = TokenMeta::new(&service_name, "password", BackendType::EncryptedFile);
                meta.label = Some(format!("Docker Registry ({registry})"));
                meta.url = Some(registry.clone());
                meta.username = Some(username);
                meta.tags = vec!["docker".into(), "registry".into()];

                entries.push(ImportEntry {
                    meta,
                    value: SecretString::from(password),
                });
            }
        }
    }

    Ok(entries)
}

fn try_get_docker_credential(registry: &str) -> Option<(String, String)> {
    // Try using docker-credential-desktop or docker-credential-osxkeychain
    for helper in &["desktop", "osxkeychain"] {
        let cmd_name = format!("docker-credential-{helper}");
        let result = Command::new(&cmd_name)
            .arg("get")
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::null())
            .spawn();

        if let Ok(mut child) = result {
            use std::io::Write;
            if let Some(ref mut stdin) = child.stdin {
                let _ = write!(stdin, "{registry}");
            }

            if let Ok(output) = child.wait_with_output() {
                if output.status.success() {
                    let stdout = String::from_utf8_lossy(&output.stdout);
                    if let Ok(cred) = serde_json::from_str::<serde_json::Value>(&stdout) {
                        let username = cred.get("Username")
                            .and_then(|v| v.as_str())
                            .unwrap_or("")
                            .to_string();
                        let secret = cred.get("Secret")
                            .and_then(|v| v.as_str())
                            .unwrap_or("")
                            .to_string();
                        if !secret.is_empty() {
                            return Some((username, secret));
                        }
                    }
                }
            }
        }
    }
    None
}
