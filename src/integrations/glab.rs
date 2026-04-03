use anyhow::{Context, Result};
use secrecy::SecretString;
use std::process::Command;

use crate::core::token::{BackendType, TokenMeta};
use super::gh::ImportEntry;

/// Scan for GitLab CLI tokens
pub fn scan() -> Result<Vec<ImportEntry>> {
    let output = Command::new("glab")
        .args(["auth", "status", "--show-token"])
        .output()
        .context("failed to run `glab auth status` — is glab installed?")?;

    let stderr = String::from_utf8_lossy(&output.stderr);
    let stdout = String::from_utf8_lossy(&output.stdout);
    let combined = format!("{stdout}{stderr}");

    // Parse "Token: glpat-xxx" from output
    let token = combined
        .lines()
        .find_map(|line| {
            let line = line.trim();
            if line.starts_with("Token:") || line.starts_with("✓ Token:") {
                Some(
                    line.split(':')
                        .nth(1)?
                        .trim()
                        .to_string(),
                )
            } else {
                None
            }
        });

    let token = match token {
        Some(t) if !t.is_empty() => t,
        _ => return Ok(Vec::new()),
    };

    // Parse hostname
    let hostname = combined
        .lines()
        .find_map(|line| {
            let line = line.trim();
            if line.contains("Logged in to") {
                // "Logged in to git.tapsvc.com as ..."
                line.split("Logged in to ")
                    .nth(1)?
                    .split_whitespace()
                    .next()
                    .map(|s| s.to_string())
            } else {
                None
            }
        })
        .unwrap_or_else(|| "gitlab.com".to_string());

    let service_name = if hostname.contains("gitlab.com") {
        "gitlab".to_string()
    } else {
        format!("gitlab-{}", hostname.split('.').next().unwrap_or("custom"))
    };

    let mut meta = TokenMeta::new(&service_name, "token", BackendType::EncryptedFile);
    meta.label = Some(format!("GitLab Token ({hostname})"));
    meta.url = Some(format!("https://{hostname}"));
    meta.tags = vec!["vcs".into()];

    Ok(vec![ImportEntry {
        meta,
        value: SecretString::from(token),
    }])
}
