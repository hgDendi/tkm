use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{bail, Context, Result};
use secrecy::{ExposeSecret, SecretString};
use zeroize::Zeroize;

use super::StorageBackend;
use crate::crypto::{aead, kdf};
use crate::core::token::VaultData;

/// Storage backend using an AES-256-GCM encrypted local file.
pub struct EncryptedFileBackend {
    vault_path: PathBuf,
    salt_path: PathBuf,
    vault: Option<VaultData>,
    key: Option<[u8; 32]>,
}

impl EncryptedFileBackend {
    pub fn new(tkm_dir: &Path) -> Self {
        Self {
            vault_path: tkm_dir.join("vault.enc"),
            salt_path: tkm_dir.join("salt"),
            vault: None,
            key: None,
        }
    }

    /// Initialize a new vault with a master password
    pub fn init(&mut self, password: &SecretString) -> Result<()> {
        if self.vault_path.exists() {
            bail!("vault already exists at {}", self.vault_path.display());
        }

        if let Some(parent) = self.vault_path.parent() {
            fs::create_dir_all(parent)?;
        }

        let salt = kdf::generate_salt();
        fs::write(&self.salt_path, salt)?;

        let key = kdf::derive_key(password, &salt)
            .context("failed to derive key")?;

        let vault = VaultData::default();
        self.save_vault(&key, &salt, &vault)?;

        self.vault = Some(vault);
        self.key = Some(key);

        Ok(())
    }

    pub fn vault_exists(&self) -> bool {
        self.vault_path.exists()
    }

    fn load_salt(&self) -> Result<[u8; 32]> {
        let bytes = fs::read(&self.salt_path)
            .with_context(|| format!("failed to read salt from {}", self.salt_path.display()))?;
        if bytes.len() != 32 {
            bail!("invalid salt file: expected 32 bytes, got {}", bytes.len());
        }
        let mut salt = [0u8; 32];
        salt.copy_from_slice(&bytes);
        Ok(salt)
    }

    fn save_vault(&self, key: &[u8; 32], salt: &[u8; 32], vault: &VaultData) -> Result<()> {
        let plaintext = serde_json::to_vec(vault)?;
        let vault_file = aead::encrypt(key, salt, &plaintext)?;
        fs::write(&self.vault_path, vault_file.to_bytes())?;
        Ok(())
    }
}

impl StorageBackend for EncryptedFileBackend {
    fn get(&self, service: &str, key: &str) -> Result<SecretString> {
        let vault = self.vault.as_ref()
            .context("vault is locked — call unlock first")?;
        vault.get(service, key)
            .map(|v| SecretString::from(v.to_string()))
            .context(format!("no token found for {service}:{key} in vault"))
    }

    fn set(&mut self, service: &str, key: &str, value: &SecretString) -> Result<()> {
        let vault = self.vault.as_mut()
            .context("vault is locked — call unlock first")?;
        vault.set(service, key, value.expose_secret());

        let k = self.key.as_ref().expect("key must be available");
        let salt = self.load_salt()?;
        self.save_vault(k, &salt, self.vault.as_ref().unwrap())?;
        Ok(())
    }

    fn delete(&mut self, service: &str, key: &str) -> Result<()> {
        let vault = self.vault.as_mut()
            .context("vault is locked — call unlock first")?;
        if !vault.delete(service, key) {
            bail!("no token found for {service}:{key} in vault");
        }

        let k = self.key.as_ref().expect("key must be available");
        let salt = self.load_salt()?;
        self.save_vault(k, &salt, self.vault.as_ref().unwrap())?;
        Ok(())
    }

    fn needs_unlock(&self) -> bool {
        self.vault.is_none()
    }

    fn unlock(&mut self, password: &SecretString) -> Result<()> {
        let salt = self.load_salt()?;
        let key = kdf::derive_key(password, &salt).context("failed to derive key")?;

        let vault_bytes = fs::read(&self.vault_path)
            .with_context(|| format!("failed to read vault: {}", self.vault_path.display()))?;

        let vault_file = aead::VaultFile::from_bytes(&vault_bytes)?;
        let plaintext = aead::decrypt(&key, &vault_file)?;

        let vault: VaultData = serde_json::from_slice(&plaintext)
            .context("failed to parse vault data")?;

        self.vault = Some(vault);
        self.key = Some(key);
        Ok(())
    }

    fn lock(&mut self) {
        self.vault = None;
        if let Some(ref mut key) = self.key {
            key.zeroize();
        }
        self.key = None;
    }

    fn name(&self) -> &str {
        "encrypted_file"
    }
}

impl Drop for EncryptedFileBackend {
    fn drop(&mut self) {
        self.lock();
    }
}
