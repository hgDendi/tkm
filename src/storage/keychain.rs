use anyhow::{bail, Context, Result};
use secrecy::{ExposeSecret, SecretString};

use super::StorageBackend;

const SERVICE_PREFIX: &str = "tkm";

/// Storage backend using the OS keychain (macOS Keychain, Linux Secret Service, etc.)
pub struct KeychainBackend;

impl KeychainBackend {
    pub fn new() -> Self {
        Self
    }

    fn keyring_service(service: &str) -> String {
        format!("{SERVICE_PREFIX}:{service}")
    }
}

impl StorageBackend for KeychainBackend {
    fn get(&self, service: &str, key: &str) -> Result<SecretString> {
        let entry = keyring::Entry::new(&Self::keyring_service(service), key)
            .context("failed to create keyring entry")?;

        match entry.get_password() {
            Ok(password) => Ok(SecretString::from(password)),
            Err(keyring::Error::NoEntry) => {
                bail!("no token found for {service}:{key} in keychain")
            }
            Err(e) => Err(e).context(format!("failed to read {service}:{key} from keychain")),
        }
    }

    fn set(&mut self, service: &str, key: &str, value: &SecretString) -> Result<()> {
        let entry = keyring::Entry::new(&Self::keyring_service(service), key)
            .context("failed to create keyring entry")?;

        entry
            .set_password(value.expose_secret())
            .context(format!("failed to store {service}:{key} in keychain"))
    }

    fn delete(&mut self, service: &str, key: &str) -> Result<()> {
        let entry = keyring::Entry::new(&Self::keyring_service(service), key)
            .context("failed to create keyring entry")?;

        match entry.delete_credential() {
            Ok(()) => Ok(()),
            Err(keyring::Error::NoEntry) => Ok(()), // already gone
            Err(e) => Err(e).context(format!("failed to delete {service}:{key} from keychain")),
        }
    }

    fn needs_unlock(&self) -> bool {
        false // OS keychain handles auth
    }

    fn unlock(&mut self, _password: &SecretString) -> Result<()> {
        Ok(()) // no-op for keychain
    }

    fn lock(&mut self) {
        // no-op for keychain
    }

    fn name(&self) -> &str {
        "keychain"
    }
}
