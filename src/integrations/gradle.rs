use anyhow::Result;
use secrecy::SecretString;

use crate::core::token::{BackendType, TokenMeta};
use super::gh::ImportEntry;

/// Scan for Gradle/Maven credentials in ~/.gradle/gradle.properties
pub fn scan() -> Result<Vec<ImportEntry>> {
    let props_path = dirs::home_dir()
        .map(|h| h.join(".gradle/gradle.properties"));

    let props_path = match props_path {
        Some(p) if p.exists() => p,
        _ => return Ok(Vec::new()),
    };

    let content = std::fs::read_to_string(&props_path)?;
    let mut entries = Vec::new();

    // Look for common credential patterns
    let credential_keys = [
        ("nexusUsername", "nexusPassword", "nexus-maven"),
        ("NEXUS_USERNAME", "NEXUS_PASSWORD", "nexus-maven"),
        ("mavenUser", "mavenPassword", "maven"),
        ("gpr.user", "gpr.key", "github-packages"),
        ("artifactory_user", "artifactory_password", "artifactory"),
    ];

    for (user_key, pass_key, service_name) in &credential_keys {
        let username = find_property(&content, user_key);
        let password = find_property(&content, pass_key);

        if let Some(password) = password {
            let mut meta = TokenMeta::new(service_name, "password", BackendType::EncryptedFile);
            meta.label = Some(format!("Gradle: {service_name}"));
            meta.username = username.clone();
            meta.tags = vec!["build".into(), "maven".into()];

            entries.push(ImportEntry {
                meta,
                value: SecretString::from(password),
            });

            // Also import username if present
            if let Some(username) = username {
                let mut user_meta = TokenMeta::new(service_name, "username", BackendType::EncryptedFile);
                user_meta.label = Some(format!("Gradle: {service_name} (username)"));
                user_meta.tags = vec!["build".into(), "maven".into()];

                entries.push(ImportEntry {
                    meta: user_meta,
                    value: SecretString::from(username),
                });
            }
        }
    }

    Ok(entries)
}

fn find_property(content: &str, key: &str) -> Option<String> {
    content
        .lines()
        .filter(|line| !line.trim_start().starts_with('#'))
        .find_map(|line| {
            let line = line.trim();
            if let Some(rest) = line.strip_prefix(key) {
                let rest = rest.trim_start();
                if let Some(value) = rest.strip_prefix('=') {
                    let value = value.trim().to_string();
                    if !value.is_empty() {
                        return Some(value);
                    }
                }
            }
            None
        })
}
