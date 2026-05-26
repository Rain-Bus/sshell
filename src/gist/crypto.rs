use crate::config::CredentialStore;
use aes_gcm::aead::{Aead, AeadCore, KeyInit, OsRng, rand_core::RngCore};
use aes_gcm::{Aes256Gcm, Nonce};
use anyhow::{Context, Result, bail};
use argon2::{Algorithm, Argon2, Params, Version};
use base64::{Engine as _, engine::general_purpose::STANDARD};

const SALT_LEN: usize = 16;
const NONCE_LEN: usize = 12;

pub(super) fn encrypt_credentials(store: &CredentialStore, password: &str) -> Result<String> {
    let json = serde_json::to_string(store).context("failed to serialize credentials")?;

    let mut salt = [0u8; SALT_LEN];
    OsRng.fill_bytes(&mut salt);
    let key = derive_key(password, &salt)?;
    let cipher =
        Aes256Gcm::new_from_slice(&key).map_err(|e| anyhow::anyhow!("aes init failed: {e}"))?;
    let nonce = Aes256Gcm::generate_nonce(&mut OsRng);
    let ciphertext = cipher
        .encrypt(&nonce, json.as_bytes())
        .map_err(|e| anyhow::anyhow!("encryption failed: {e}"))?;

    let mut blob = Vec::with_capacity(SALT_LEN + NONCE_LEN + ciphertext.len());
    blob.extend_from_slice(&salt);
    blob.extend_from_slice(&nonce);
    blob.extend_from_slice(&ciphertext);

    Ok(STANDARD.encode(&blob))
}

pub(super) fn decrypt_credentials(encoded: &str, password: &str) -> Result<CredentialStore> {
    let blob = STANDARD
        .decode(encoded)
        .context("failed to decode encrypted credentials")?;
    if blob.len() < SALT_LEN + NONCE_LEN {
        bail!("encrypted credentials blob too short");
    }

    let salt = &blob[..SALT_LEN];
    let nonce_bytes = &blob[SALT_LEN..SALT_LEN + NONCE_LEN];
    let ciphertext = &blob[SALT_LEN + NONCE_LEN..];

    let key = derive_key(password, salt)?;
    let cipher =
        Aes256Gcm::new_from_slice(&key).map_err(|e| anyhow::anyhow!("aes init failed: {e}"))?;
    let nonce = Nonce::from_slice(nonce_bytes);
    let plaintext = cipher
        .decrypt(nonce, ciphertext)
        .map_err(|_| anyhow::anyhow!("decryption failed; wrong sync_password?"))?;

    serde_json::from_slice(&plaintext).context("failed to parse decrypted credentials")
}

fn derive_key(password: &str, salt: &[u8]) -> Result<[u8; 32]> {
    let params = Params::new(64 * 1024, 3, 4, Some(32))
        .map_err(|e| anyhow::anyhow!("argon2 params: {e}"))?;
    let argon2 = Argon2::new(Algorithm::Argon2id, Version::V0x13, params);
    let mut key = [0u8; 32];
    argon2
        .hash_password_into(password.as_bytes(), salt, &mut key)
        .map_err(|e| anyhow::anyhow!("key derivation: {e}"))?;
    Ok(key)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::CredentialEntry;

    #[test]
    fn credentials_encrypt_decrypt_round_trip() {
        let mut store = CredentialStore::default();
        store.entries.insert(
            "main".to_string(),
            CredentialEntry::password("secret".to_string()),
        );
        store.entries.insert(
            "key".to_string(),
            CredentialEntry::private_key("private-key".to_string()),
        );

        let encrypted = encrypt_credentials(&store, "sync-password").unwrap();
        let decrypted = decrypt_credentials(&encrypted, "sync-password").unwrap();

        assert_eq!(decrypted.entries.len(), 2);
        assert_eq!(
            decrypted.entries.get("main").map(CredentialEntry::value),
            Some("secret")
        );
        assert_eq!(
            decrypted.entries.get("key").map(CredentialEntry::value),
            Some("private-key")
        );
    }

    #[test]
    fn credentials_decrypt_rejects_wrong_password() {
        let mut store = CredentialStore::default();
        store.entries.insert(
            "main".to_string(),
            CredentialEntry::password("secret".to_string()),
        );

        let encrypted = encrypt_credentials(&store, "sync-password").unwrap();

        assert!(decrypt_credentials(&encrypted, "wrong-password").is_err());
    }
}
