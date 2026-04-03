use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};

use crate::core::token::TokenMeta;

/// Registry wrapper for TOML serialization
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, Default)]
struct RegistryFile {
    #[serde(default)]
    entries: Vec<TokenMeta>,
}

/// Manages the plaintext registry that maps services to backends + metadata.
/// The registry contains NO secrets — only metadata.
pub struct Registry {
    path: PathBuf,
    data: RegistryFile,
}

impl Registry {
    /// Load or create registry from the given path
    pub fn load(path: &Path) -> Result<Self> {
        let data = if path.exists() {
            let content = fs::read_to_string(path)
                .with_context(|| format!("failed to read registry: {}", path.display()))?;
            toml::from_str(&content)
                .with_context(|| format!("failed to parse registry: {}", path.display()))?
        } else {
            RegistryFile::default()
        };

        Ok(Self {
            path: path.to_path_buf(),
            data,
        })
    }

    /// Save registry to disk
    pub fn save(&self) -> Result<()> {
        if let Some(parent) = self.path.parent() {
            fs::create_dir_all(parent)?;
        }
        let content = toml::to_string_pretty(&self.data)?;
        fs::write(&self.path, content)?;
        Ok(())
    }

    /// Get metadata for a service (first matching entry)
    pub fn get(&self, service: &str) -> Option<&TokenMeta> {
        self.data
            .entries
            .iter()
            .find(|e| e.service == service)
    }

    /// Get metadata for a specific service:key pair
    pub fn get_exact(&self, service: &str, key: &str) -> Option<&TokenMeta> {
        self.data
            .entries
            .iter()
            .find(|e| e.service == service && e.key == key)
    }

    /// List all entries
    pub fn list(&self) -> &[TokenMeta] {
        &self.data.entries
    }

    /// Add or update an entry
    pub fn upsert(&mut self, meta: TokenMeta) {
        if let Some(existing) = self
            .data
            .entries
            .iter_mut()
            .find(|e| e.service == meta.service && e.key == meta.key)
        {
            *existing = meta;
        } else {
            self.data.entries.push(meta);
        }
    }

    /// Remove an entry
    pub fn remove(&mut self, service: &str, key: &str) -> bool {
        let before = self.data.entries.len();
        self.data
            .entries
            .retain(|e| !(e.service == service && e.key == key));
        self.data.entries.len() < before
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::token::BackendType;
    use tempfile::NamedTempFile;

    fn make_meta(service: &str, key: &str) -> TokenMeta {
        TokenMeta::new(service, key, BackendType::EncryptedFile)
    }

    #[test]
    fn test_registry_crud() {
        let tmp = NamedTempFile::new().unwrap();
        let path = tmp.path();

        let mut reg = Registry::load(path).unwrap();
        assert!(reg.list().is_empty());

        // Add
        reg.upsert(make_meta("github", "token"));
        assert_eq!(reg.list().len(), 1);
        assert!(reg.get("github").is_some());

        // Update
        let mut updated = make_meta("github", "token");
        updated.label = Some("Updated".into());
        reg.upsert(updated);
        assert_eq!(reg.list().len(), 1);
        assert_eq!(reg.get("github").unwrap().label.as_deref(), Some("Updated"));

        // Remove
        assert!(reg.remove("github", "token"));
        assert!(reg.list().is_empty());
    }

    #[test]
    fn test_registry_save_load_roundtrip() {
        let tmp = NamedTempFile::new().unwrap();
        let path = tmp.path().to_path_buf();

        {
            let mut reg = Registry::load(&path).unwrap();
            reg.upsert(make_meta("github", "token"));
            reg.upsert(make_meta("docker", "password"));
            reg.save().unwrap();
        }

        {
            let reg = Registry::load(&path).unwrap();
            assert_eq!(reg.list().len(), 2);
            assert!(reg.get("github").is_some());
            assert!(reg.get("docker").is_some());
        }
    }
}
