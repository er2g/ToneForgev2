//! Secure Storage for ToneForge
//!
//! Provides encrypted storage for sensitive data like API keys.
//! Uses simple XOR encryption with a machine-specific key.

use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

const CONFIG_FILE: &str = "toneforge_config.enc";
const MAGIC_HEADER: &[u8] = b"TFCFG1";

/// Encrypted configuration
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SecureConfig {
    pub api_key: Option<String>,
    pub provider: Option<String>,
    pub model: Option<String>,
    pub custom_instructions: Option<String>,
}

/// Get machine-specific encryption key
fn get_machine_key() -> Vec<u8> {
    // Use a combination of factors for the key
    let mut key = Vec::new();

    // Add some entropy from the hostname
    if let Ok(hostname) = std::env::var("COMPUTERNAME")
        .or_else(|_| std::env::var("HOSTNAME"))
        .or_else(|_| std::env::var("USER"))
    {
        key.extend(hostname.as_bytes());
    }

    // Add a fixed salt
    key.extend(b"ToneForge_v2_Salt_2024!");

    // Ensure minimum key length
    while key.len() < 32 {
        key.push(0x42);
    }

    // Hash the key to fixed length
    let mut hash = [0u8; 32];
    for (i, &byte) in key.iter().enumerate() {
        hash[i % 32] ^= byte;
        hash[(i + 1) % 32] = hash[(i + 1) % 32].wrapping_add(byte);
    }

    hash.to_vec()
}

/// Simple XOR encryption/decryption
fn xor_crypt(data: &[u8], key: &[u8]) -> Vec<u8> {
    data.iter()
        .enumerate()
        .map(|(i, &byte)| byte ^ key[i % key.len()])
        .collect()
}

/// Get the config file path
fn get_config_path() -> PathBuf {
    let config_dir = dirs::config_dir()
        .or_else(dirs::home_dir)
        .unwrap_or_else(|| PathBuf::from("."));

    config_dir.join("ToneForge").join(CONFIG_FILE)
}

/// Save encrypted config to disk
pub fn save_config(config: &SecureConfig) -> Result<(), String> {
    let config_path = get_config_path();

    // Create directory if it doesn't exist
    if let Some(parent) = config_path.parent() {
        fs::create_dir_all(parent).map_err(|e| format!("Failed to create config dir: {}", e))?;
    }

    // Serialize to JSON
    let json = serde_json::to_string(config).map_err(|e| format!("Failed to serialize: {}", e))?;

    // Encrypt
    let key = get_machine_key();
    let encrypted = xor_crypt(json.as_bytes(), &key);

    // Add magic header and write
    let mut data = MAGIC_HEADER.to_vec();
    data.extend(&encrypted);

    fs::write(&config_path, &data).map_err(|e| format!("Failed to write config: {}", e))?;

    println!(
        "[SECURE] Config saved to: {}",
        config_path.to_string_lossy()
    );

    Ok(())
}

/// Load encrypted config from disk
pub fn load_config() -> Result<SecureConfig, String> {
    let config_path = get_config_path();

    if !config_path.exists() {
        return Ok(SecureConfig::default());
    }

    // Read file
    let data = fs::read(&config_path).map_err(|e| format!("Failed to read config: {}", e))?;

    // Verify magic header
    if data.len() < MAGIC_HEADER.len() || &data[..MAGIC_HEADER.len()] != MAGIC_HEADER {
        return Err("Invalid config file format".to_string());
    }

    // Decrypt
    let encrypted = &data[MAGIC_HEADER.len()..];
    let key = get_machine_key();
    let decrypted = xor_crypt(encrypted, &key);

    // Parse JSON
    let json =
        String::from_utf8(decrypted).map_err(|e| format!("Failed to decode config: {}", e))?;

    serde_json::from_str(&json).map_err(|e| format!("Failed to parse config: {}", e))
}

/// Delete the config file
pub fn delete_config() -> Result<(), String> {
    let config_path = get_config_path();

    if config_path.exists() {
        fs::remove_file(&config_path).map_err(|e| format!("Failed to delete config: {}", e))?;
        println!("[SECURE] Config deleted");
    }

    Ok(())
}

/// Check if config exists
pub fn config_exists() -> bool {
    get_config_path().exists()
}

/// Mask an API key for display
pub fn mask_api_key(key: &str) -> String {
    if key.len() <= 8 {
        return "*".repeat(key.len());
    }

    let prefix = &key[..4];
    let suffix = &key[key.len() - 4..];
    format!("{}...{}", prefix, suffix)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_xor_crypt_roundtrip() {
        let data = b"Hello, World! This is a test.";
        let key = get_machine_key();

        let encrypted = xor_crypt(data, &key);
        let decrypted = xor_crypt(&encrypted, &key);

        assert_eq!(data.to_vec(), decrypted);
    }

    #[test]
    fn test_mask_api_key() {
        assert_eq!(mask_api_key("abcd1234efgh5678"), "abcd...5678");
        assert_eq!(mask_api_key("short"), "*****");
    }

    #[test]
    fn test_config_serialization() {
        let config = SecureConfig {
            api_key: Some("test-api-key-123".to_string()),
            provider: Some("xai".to_string()),
            model: Some("grok-2-latest".to_string()),
            custom_instructions: None,
        };

        let json = serde_json::to_string(&config).unwrap();
        let parsed: SecureConfig = serde_json::from_str(&json).unwrap();

        assert_eq!(parsed.api_key, config.api_key);
        assert_eq!(parsed.provider, config.provider);
    }
}
