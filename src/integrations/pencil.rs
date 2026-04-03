use anyhow::Result;
use secrecy::SecretString;

use crate::core::token::{BackendType, TokenMeta};
use super::gh::ImportEntry;

/// Scan for Pencil license token
pub fn scan() -> Result<Vec<ImportEntry>> {
    let token_path = dirs::home_dir()
        .map(|h| h.join(".pencil/license-token.json"));

    let token_path = match token_path {
        Some(p) if p.exists() => p,
        _ => return Ok(Vec::new()),
    };

    let content = std::fs::read_to_string(&token_path)?;
    let json: serde_json::Value = serde_json::from_str(&content)?;

    let token = json
        .get("token")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());

    let email = json
        .get("email")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());

    match token {
        Some(token) if !token.is_empty() => {
            let mut meta = TokenMeta::new("pencil", "license", BackendType::EncryptedFile);
            meta.label = Some("Pencil Design License".into());
            meta.username = email;
            meta.tags = vec!["design".into(), "license".into()];

            Ok(vec![ImportEntry {
                meta,
                value: SecretString::from(token),
            }])
        }
        _ => Ok(Vec::new()),
    }
}
