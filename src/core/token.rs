use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum BackendType {
    #[serde(rename = "keychain")]
    Keychain,
    #[serde(rename = "encrypted_file")]
    EncryptedFile,
}

impl std::fmt::Display for BackendType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BackendType::Keychain => write!(f, "keychain"),
            BackendType::EncryptedFile => write!(f, "file"),
        }
    }
}

/// Metadata stored in the plaintext registry (no secrets)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenMeta {
    pub service: String,
    pub key: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub username: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tags: Vec<String>,
    pub backend: BackendType,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expires_at: Option<DateTime<Utc>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub notes: Option<String>,
}

impl TokenMeta {
    pub fn new(service: &str, key: &str, backend: BackendType) -> Self {
        let now = Utc::now();
        Self {
            service: service.to_string(),
            key: key.to_string(),
            label: None,
            username: None,
            url: None,
            tags: Vec::new(),
            backend,
            created_at: now,
            updated_at: now,
            expires_at: None,
            notes: None,
        }
    }

    pub fn is_expired(&self) -> bool {
        self.expires_at
            .map(|exp| exp < Utc::now())
            .unwrap_or(false)
    }

    /// Unique identifier for this token entry
    pub fn id(&self) -> String {
        format!("{}:{}", self.service, self.key)
    }
}

/// Secret stored in the encrypted vault (for EncryptedFile backend)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VaultEntry {
    pub service: String,
    pub key: String,
    pub value: String, // the actual secret value
}

/// The entire vault content (serialized/deserialized as a whole)
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct VaultData {
    pub entries: Vec<VaultEntry>,
}

impl VaultData {
    pub fn get(&self, service: &str, key: &str) -> Option<&str> {
        self.entries
            .iter()
            .find(|e| e.service == service && e.key == key)
            .map(|e| e.value.as_str())
    }

    pub fn set(&mut self, service: &str, key: &str, value: &str) {
        if let Some(entry) = self
            .entries
            .iter_mut()
            .find(|e| e.service == service && e.key == key)
        {
            entry.value = value.to_string();
        } else {
            self.entries.push(VaultEntry {
                service: service.to_string(),
                key: key.to_string(),
                value: value.to_string(),
            });
        }
    }

    pub fn delete(&mut self, service: &str, key: &str) -> bool {
        let before = self.entries.len();
        self.entries
            .retain(|e| !(e.service == service && e.key == key));
        self.entries.len() < before
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_token_meta_id() {
        let meta = TokenMeta::new("github", "token", BackendType::Keychain);
        assert_eq!(meta.id(), "github:token");
    }

    #[test]
    fn test_token_meta_not_expired() {
        let meta = TokenMeta::new("github", "token", BackendType::Keychain);
        assert!(!meta.is_expired());
    }

    #[test]
    fn test_vault_data_crud() {
        let mut vault = VaultData::default();

        // Set
        vault.set("github", "token", "ghp_abc123");
        assert_eq!(vault.get("github", "token"), Some("ghp_abc123"));

        // Update
        vault.set("github", "token", "ghp_new456");
        assert_eq!(vault.get("github", "token"), Some("ghp_new456"));
        assert_eq!(vault.entries.len(), 1);

        // Delete
        assert!(vault.delete("github", "token"));
        assert_eq!(vault.get("github", "token"), None);

        // Delete non-existent
        assert!(!vault.delete("github", "token"));
    }
}
