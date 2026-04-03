use argon2::{Argon2, Algorithm, Params, Version};
use rand::RngCore;
use secrecy::{ExposeSecret, SecretString};
use zeroize::Zeroize;

const SALT_LEN: usize = 32;
const KEY_LEN: usize = 32;

/// Argon2id parameters: 64MB memory, 3 iterations, 4 parallelism
fn argon2_params() -> Params {
    Params::new(65536, 3, 4, Some(KEY_LEN)).expect("valid argon2 params")
}

/// Generate a random salt
pub fn generate_salt() -> [u8; SALT_LEN] {
    let mut salt = [0u8; SALT_LEN];
    rand::thread_rng().fill_bytes(&mut salt);
    salt
}

/// Derive a 256-bit key from password + salt using Argon2id
pub fn derive_key(password: &SecretString, salt: &[u8; SALT_LEN]) -> anyhow::Result<[u8; KEY_LEN]> {
    let argon2 = Argon2::new(Algorithm::Argon2id, Version::V0x13, argon2_params());
    let mut key = [0u8; KEY_LEN];
    argon2.hash_password_into(
        password.expose_secret().as_bytes(),
        salt,
        &mut key,
    ).map_err(|e| anyhow::anyhow!("argon2 KDF failed: {e}"))?;
    Ok(key)
}

/// Zeroize a key after use
pub fn zeroize_key(key: &mut [u8; KEY_LEN]) {
    key.zeroize();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_derive_key_deterministic() {
        let password = SecretString::from("test-password");
        let salt = [42u8; SALT_LEN];
        let key1 = derive_key(&password, &salt).unwrap();
        let key2 = derive_key(&password, &salt).unwrap();
        assert_eq!(key1, key2);
    }

    #[test]
    fn test_different_salt_different_key() {
        let password = SecretString::from("test-password");
        let salt1 = [1u8; SALT_LEN];
        let salt2 = [2u8; SALT_LEN];
        let key1 = derive_key(&password, &salt1).unwrap();
        let key2 = derive_key(&password, &salt2).unwrap();
        assert_ne!(key1, key2);
    }

    #[test]
    fn test_different_password_different_key() {
        let salt = [42u8; SALT_LEN];
        let key1 = derive_key(&SecretString::from("password1"), &salt).unwrap();
        let key2 = derive_key(&SecretString::from("password2"), &salt).unwrap();
        assert_ne!(key1, key2);
    }

    #[test]
    fn test_generate_salt_randomness() {
        let s1 = generate_salt();
        let s2 = generate_salt();
        assert_ne!(s1, s2);
    }
}
