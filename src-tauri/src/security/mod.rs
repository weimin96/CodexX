use aes_gcm::{
    aead::{Aead, AeadCore, KeyInit, OsRng},
    Aes256Gcm, Nonce,
};
use base64::{engine::general_purpose::STANDARD as BASE64, Engine};
use pbkdf2::pbkdf2_hmac;
use sha2::Sha256;

use crate::error::{AppError, AppResult};

const SERVICE_NAME: &str = "codex-manager";
const MASTER_KEY_ACCOUNT: &str = "master-encryption-key";
const PBKDF2_ITERATIONS: u32 = 100_000;
const SALT_LEN: usize = 16;

/// Get or create the master encryption key from the system keyring
fn get_master_key() -> AppResult<Vec<u8>> {
    let entry = keyring::Entry::new(SERVICE_NAME, MASTER_KEY_ACCOUNT)
        .map_err(|e| AppError::Security(e.to_string()))?;

    match entry.get_password() {
        Ok(key_b64) => BASE64
            .decode(key_b64)
            .map_err(|e| AppError::Security(e.to_string())),
        Err(_) => {
            // Generate a new master key
            let key: Vec<u8> = (0..32).map(|_| rand::random::<u8>()).collect();
            let key_b64 = BASE64.encode(&key);
            entry
                .set_password(&key_b64)
                .map_err(|e| AppError::Security(e.to_string()))?;
            Ok(key)
        }
    }
}

/// Encrypt a plaintext value using AES-256-GCM
pub fn encrypt(plaintext: &str) -> AppResult<String> {
    let master_key = get_master_key()?;
    let cipher = Aes256Gcm::new_from_slice(&master_key)
        .map_err(|e| AppError::Security(e.to_string()))?;

    let nonce = Aes256Gcm::generate_nonce(&mut OsRng);
    let ciphertext = cipher
        .encrypt(&nonce, plaintext.as_bytes())
        .map_err(|e| AppError::Security(e.to_string()))?;

    // Format: base64(nonce + ciphertext)
    let mut combined = nonce.to_vec();
    combined.extend_from_slice(&ciphertext);
    Ok(BASE64.encode(combined))
}

/// Decrypt a ciphertext value
pub fn decrypt(ciphertext_b64: &str) -> AppResult<String> {
    let master_key = get_master_key()?;
    let cipher = Aes256Gcm::new_from_slice(&master_key)
        .map_err(|e| AppError::Security(e.to_string()))?;

    let combined = BASE64
        .decode(ciphertext_b64)
        .map_err(|e| AppError::Security(e.to_string()))?;

    if combined.len() < 12 {
        return Err(AppError::Security("Invalid ciphertext".to_string()));
    }

    let (nonce_bytes, ciphertext) = combined.split_at(12);
    let nonce = Nonce::from_slice(nonce_bytes);
    let plaintext = cipher
        .decrypt(nonce, ciphertext)
        .map_err(|e| AppError::Security(e.to_string()))?;

    String::from_utf8(plaintext).map_err(|e| AppError::Security(e.to_string()))
}

/// Derive a key from a password for export encryption
pub fn derive_key_from_password(password: &str, salt: &[u8]) -> Vec<u8> {
    let mut key = vec![0u8; 32];
    pbkdf2_hmac::<Sha256>(password.as_bytes(), salt, PBKDF2_ITERATIONS, &mut key);
    key
}

/// Encrypt data for export with a user-provided password
pub fn encrypt_export(data: &str, password: &str) -> AppResult<String> {
    let salt: Vec<u8> = (0..SALT_LEN).map(|_| rand::random::<u8>()).collect();
    let key = derive_key_from_password(password, &salt);

    let cipher = Aes256Gcm::new_from_slice(&key)
        .map_err(|e| AppError::Security(e.to_string()))?;
    let nonce = Aes256Gcm::generate_nonce(&mut OsRng);

    let ciphertext = cipher
        .encrypt(&nonce, data.as_bytes())
        .map_err(|e| AppError::Security(e.to_string()))?;

    // Format: base64(salt + nonce + ciphertext)
    let mut combined = salt;
    combined.extend_from_slice(&nonce);
    combined.extend_from_slice(&ciphertext);

    Ok(BASE64.encode(combined))
}

/// Decrypt exported data with user password
pub fn decrypt_export(encrypted_b64: &str, password: &str) -> AppResult<String> {
    let combined = BASE64
        .decode(encrypted_b64)
        .map_err(|e| AppError::Security(e.to_string()))?;

    if combined.len() < SALT_LEN + 12 {
        return Err(AppError::Security("Invalid export data".to_string()));
    }

    let (salt, rest) = combined.split_at(SALT_LEN);
    let (nonce_bytes, ciphertext) = rest.split_at(12);

    let key = derive_key_from_password(password, salt);
    let cipher = Aes256Gcm::new_from_slice(&key)
        .map_err(|e| AppError::Security(e.to_string()))?;

    let nonce = Nonce::from_slice(nonce_bytes);
    let plaintext = cipher
        .decrypt(nonce, ciphertext)
        .map_err(|e| AppError::Security(e.to_string()))?;

    String::from_utf8(plaintext).map_err(|e| AppError::Security(e.to_string()))
}
