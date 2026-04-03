use aes_gcm::{
    aead::{Aead, KeyInit},
    Aes256Gcm, Nonce,
};
use rand::RngCore;

const NONCE_LEN: usize = 12;
const VERSION: u32 = 1;

/// Encrypted vault file format:
/// [4B version LE][32B salt][12B nonce][N bytes ciphertext + 16B GCM tag]
pub struct VaultFile {
    pub version: u32,
    pub salt: [u8; 32],
    pub nonce: [u8; NONCE_LEN],
    pub ciphertext: Vec<u8>, // includes GCM tag
}

impl VaultFile {
    /// Serialize to bytes
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut buf = Vec::with_capacity(4 + 32 + NONCE_LEN + self.ciphertext.len());
        buf.extend_from_slice(&self.version.to_le_bytes());
        buf.extend_from_slice(&self.salt);
        buf.extend_from_slice(&self.nonce);
        buf.extend_from_slice(&self.ciphertext);
        buf
    }

    /// Deserialize from bytes
    pub fn from_bytes(data: &[u8]) -> Result<Self, AeadError> {
        let min_len = 4 + 32 + NONCE_LEN + 16; // version + salt + nonce + min GCM tag
        if data.len() < min_len {
            return Err(AeadError::InvalidFormat("vault file too short".into()));
        }

        let version = u32::from_le_bytes(data[0..4].try_into().unwrap());
        if version != VERSION {
            return Err(AeadError::InvalidFormat(format!(
                "unsupported vault version: {version}"
            )));
        }

        let mut salt = [0u8; 32];
        salt.copy_from_slice(&data[4..36]);

        let mut nonce = [0u8; NONCE_LEN];
        nonce.copy_from_slice(&data[36..48]);

        let ciphertext = data[48..].to_vec();

        Ok(Self {
            version,
            salt,
            nonce,
            ciphertext,
        })
    }
}

#[derive(Debug, thiserror::Error)]
pub enum AeadError {
    #[error("encryption failed: {0}")]
    EncryptionFailed(String),
    #[error("decryption failed: wrong password or corrupted vault")]
    DecryptionFailed,
    #[error("invalid format: {0}")]
    InvalidFormat(String),
}

/// Encrypt plaintext with AES-256-GCM
pub fn encrypt(key: &[u8; 32], salt: &[u8; 32], plaintext: &[u8]) -> Result<VaultFile, AeadError> {
    let cipher = Aes256Gcm::new_from_slice(key)
        .map_err(|e| AeadError::EncryptionFailed(e.to_string()))?;

    let mut nonce_bytes = [0u8; NONCE_LEN];
    rand::thread_rng().fill_bytes(&mut nonce_bytes);
    let nonce = Nonce::from_slice(&nonce_bytes);

    let ciphertext = cipher
        .encrypt(nonce, plaintext)
        .map_err(|e| AeadError::EncryptionFailed(e.to_string()))?;

    Ok(VaultFile {
        version: VERSION,
        salt: *salt,
        nonce: nonce_bytes,
        ciphertext,
    })
}

/// Decrypt ciphertext with AES-256-GCM
pub fn decrypt(key: &[u8; 32], vault: &VaultFile) -> Result<Vec<u8>, AeadError> {
    let cipher = Aes256Gcm::new_from_slice(key)
        .map_err(|e| AeadError::EncryptionFailed(e.to_string()))?;

    let nonce = Nonce::from_slice(&vault.nonce);

    cipher
        .decrypt(nonce, vault.ciphertext.as_ref())
        .map_err(|_| AeadError::DecryptionFailed)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encrypt_decrypt_roundtrip() {
        let key = [42u8; 32];
        let salt = [1u8; 32];
        let plaintext = b"hello world, this is a secret token";

        let vault = encrypt(&key, &salt, plaintext).unwrap();
        let decrypted = decrypt(&key, &vault).unwrap();

        assert_eq!(decrypted, plaintext);
    }

    #[test]
    fn test_wrong_key_fails() {
        let key = [42u8; 32];
        let wrong_key = [99u8; 32];
        let salt = [1u8; 32];
        let plaintext = b"secret";

        let vault = encrypt(&key, &salt, plaintext).unwrap();
        let result = decrypt(&wrong_key, &vault);

        assert!(result.is_err());
    }

    #[test]
    fn test_vault_file_serialization_roundtrip() {
        let key = [42u8; 32];
        let salt = [1u8; 32];
        let plaintext = b"roundtrip test";

        let vault = encrypt(&key, &salt, plaintext).unwrap();
        let bytes = vault.to_bytes();
        let restored = VaultFile::from_bytes(&bytes).unwrap();

        assert_eq!(restored.version, vault.version);
        assert_eq!(restored.salt, vault.salt);
        assert_eq!(restored.nonce, vault.nonce);
        assert_eq!(restored.ciphertext, vault.ciphertext);

        let decrypted = decrypt(&key, &restored).unwrap();
        assert_eq!(decrypted, plaintext);
    }

    #[test]
    fn test_invalid_vault_too_short() {
        let result = VaultFile::from_bytes(&[0u8; 10]);
        assert!(result.is_err());
    }
}
