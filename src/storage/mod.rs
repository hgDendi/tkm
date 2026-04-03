pub mod encrypted_file;
pub mod keychain;

use anyhow::Result;
use secrecy::SecretString;

/// Abstraction over different secret storage mechanisms.
pub trait StorageBackend: Send + Sync {
    /// Retrieve a secret value by service and key
    fn get(&self, service: &str, key: &str) -> Result<SecretString>;

    /// Store a secret value
    fn set(&mut self, service: &str, key: &str, value: &SecretString) -> Result<()>;

    /// Delete a secret
    fn delete(&mut self, service: &str, key: &str) -> Result<()>;

    /// Whether this backend requires unlocking before use
    fn needs_unlock(&self) -> bool;

    /// Unlock the backend (e.g., decrypt vault with master password)
    fn unlock(&mut self, password: &SecretString) -> Result<()>;

    /// Lock the backend (zeroize decrypted material)
    fn lock(&mut self);

    /// Backend display name
    fn name(&self) -> &str;
}
