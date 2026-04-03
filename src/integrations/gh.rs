use anyhow::{Context, Result};
use secrecy::SecretString;
use std::process::Command;

use crate::core::token::{BackendType, TokenMeta};

/// Scan for GitHub CLI tokens
pub fn scan() -> Result<Vec<ImportEntry>> {
    let output = Command::new("gh")
        .args(["auth", "token"])
        .output()
        .context("failed to run `gh auth token` — is gh installed?")?;

    if !output.status.success() {
        return Ok(Vec::new());
    }

    let token = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if token.is_empty() {
        return Ok(Vec::new());
    }

    // Get username
    let username = Command::new("gh")
        .args(["api", "user", "--jq", ".login"])
        .output()
        .ok()
        .and_then(|o| {
            if o.status.success() {
                Some(String::from_utf8_lossy(&o.stdout).trim().to_string())
            } else {
                None
            }
        });

    let mut meta = TokenMeta::new("github", "token", BackendType::EncryptedFile);
    meta.label = Some("GitHub Personal Access Token".into());
    meta.url = Some("https://github.com".into());
    meta.username = username;
    meta.tags = vec!["vcs".into()];

    Ok(vec![ImportEntry {
        meta,
        value: SecretString::from(token),
    }])
}

pub struct ImportEntry {
    pub meta: TokenMeta,
    pub value: SecretString,
}
