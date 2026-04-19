use aes_gcm::{
    aead::{Aead, AeadCore, KeyInit, OsRng},
    Aes256Gcm, Nonce,
};
use base64::{engine::general_purpose::STANDARD as BASE64, Engine};

use crate::error::{AppError, AppResult};

const SERVICE_NAME: &str = "codex-manager";
const MASTER_KEY_ACCOUNT: &str = "master-encryption-key";
const MASTER_KEY_ENV_VAR: &str = "CODEX_MANAGER_MASTER_KEY";

struct ResolvedMasterKey {
    value: Vec<u8>,
    source_label: &'static str,
}

fn get_master_key() -> AppResult<ResolvedMasterKey> {
    if let Some(value) = read_master_key_from_env()? {
        return Ok(ResolvedMasterKey {
            value,
            source_label: "CODEX_MANAGER_MASTER_KEY",
        });
    }

    Ok(ResolvedMasterKey {
        value: get_or_create_system_master_key()?,
        source_label: "系统凭据库",
    })
}

fn get_or_create_system_master_key() -> AppResult<Vec<u8>> {
    if let Some(key) = read_master_key_from_system_store()? {
        return Ok(key);
    }

    // 只有系统凭据库明确没有主密钥时才创建，避免读取失败时生成无法解密历史数据的新密钥。
    let key: Vec<u8> = (0..32).map(|_| rand::random::<u8>()).collect();
    let entry = keyring::Entry::new(SERVICE_NAME, MASTER_KEY_ACCOUNT)
        .map_err(|error| AppError::Security(format!("打开系统凭据库失败: {error}")))?;
    entry
        .set_password(&BASE64.encode(&key))
        .map_err(|error| AppError::Security(format!("写入系统凭据库失败: {error}")))?;

    Ok(key)
}

fn read_master_key_from_system_store() -> AppResult<Option<Vec<u8>>> {
    let entry = keyring::Entry::new(SERVICE_NAME, MASTER_KEY_ACCOUNT)
        .map_err(|error| AppError::Security(format!("打开系统凭据库失败: {error}")))?;

    match entry.get_password() {
        Ok(encoded_key) => {
            let key = BASE64
                .decode(encoded_key)
                .map_err(|_| AppError::Security("系统凭据库中的主密钥格式无效".to_string()))?;
            if key.len() != 32 {
                return Err(AppError::Security(
                    "系统凭据库中的主密钥长度无效".to_string(),
                ));
            }
            Ok(Some(key))
        }
        Err(keyring::Error::NoEntry) => Ok(None),
        Err(error) => Err(AppError::Security(format!("读取系统凭据库失败: {error}"))),
    }
}

fn read_master_key_from_env() -> AppResult<Option<Vec<u8>>> {
    let Ok(raw_value) = std::env::var(MASTER_KEY_ENV_VAR) else {
        return Ok(None);
    };

    let value = raw_value.trim();
    if value.is_empty() {
        return Ok(None);
    }

    let key = parse_env_master_key(value)?;
    if key.len() != 32 {
        return Err(AppError::Security(format!(
            "{MASTER_KEY_ENV_VAR} 必须解析为 32 字节主密钥"
        )));
    }

    Ok(Some(key))
}

fn parse_env_master_key(value: &str) -> AppResult<Vec<u8>> {
    if let Ok(decoded) = BASE64.decode(value) {
        if decoded.len() == 32 {
            return Ok(decoded);
        }
    }

    if let Some(decoded) = decode_hex_key(value) {
        return Ok(decoded);
    }

    if value.as_bytes().len() == 32 {
        return Ok(value.as_bytes().to_vec());
    }

    Err(AppError::Security(format!(
        "{MASTER_KEY_ENV_VAR} 必须是 32 字节原文、64 位十六进制或 base64 编码的 32 字节密钥"
    )))
}

fn decode_hex_key(value: &str) -> Option<Vec<u8>> {
    if value.len() != 64 || !value.chars().all(|character| character.is_ascii_hexdigit()) {
        return None;
    }

    let mut key = Vec::with_capacity(32);
    for index in (0..value.len()).step_by(2) {
        let byte = u8::from_str_radix(&value[index..index + 2], 16).ok()?;
        key.push(byte);
    }

    Some(key)
}

pub fn encrypt(plaintext: &str) -> AppResult<String> {
    let master_key = get_master_key()?;
    let cipher = Aes256Gcm::new_from_slice(&master_key.value)
        .map_err(|e| AppError::Security(e.to_string()))?;

    let nonce = Aes256Gcm::generate_nonce(&mut OsRng);
    let ciphertext = cipher
        .encrypt(&nonce, plaintext.as_bytes())
        .map_err(|e| AppError::Security(e.to_string()))?;

    // 密文携带随机 nonce，便于后续只依赖主密钥完成解密。
    let mut combined = nonce.to_vec();
    combined.extend_from_slice(&ciphertext);
    Ok(BASE64.encode(combined))
}

pub fn decrypt(ciphertext_b64: &str) -> AppResult<String> {
    let combined = BASE64
        .decode(ciphertext_b64)
        .map_err(|e| AppError::Security(e.to_string()))?;

    if combined.len() < 12 {
        return Err(AppError::Security("Invalid ciphertext".to_string()));
    }

    let master_key = get_master_key()?;
    let plaintext = decrypt_payload(&master_key.value, &combined).map_err(|_| {
        AppError::Security(format!(
            "本地凭证无法用当前主密钥来源（{}）解密，请确认未切换主密钥，或重新导入账号",
            master_key.source_label
        ))
    })?;

    String::from_utf8(plaintext).map_err(|e| AppError::Security(e.to_string()))
}

fn decrypt_payload(master_key: &[u8], combined: &[u8]) -> AppResult<Vec<u8>> {
    let cipher =
        Aes256Gcm::new_from_slice(master_key).map_err(|e| AppError::Security(e.to_string()))?;
    let (nonce_bytes, ciphertext) = combined.split_at(12);
    let nonce = Nonce::from_slice(nonce_bytes);

    cipher
        .decrypt(nonce, ciphertext)
        .map_err(|_| AppError::Security("主密钥无法解密该凭证".to_string()))
}
