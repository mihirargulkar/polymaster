use rsa::{RsaPrivateKey};
use rsa::pkcs8::DecodePrivateKey;
use rsa::pkcs1::DecodeRsaPrivateKey;
use base64::{Engine as _, engine::general_purpose};
use sha2::{Digest, Sha256};
use std::time::{SystemTime, UNIX_EPOCH};

/// Generate the Kalshi V2 authentication headers
pub fn generate_auth_headers(
    api_key_id: &str,
    private_key_pem: &str,
) -> Result<Vec<(String, String)>, Box<dyn std::error::Error + Send + Sync>> {
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)?
        .as_millis()
        .to_string();

    let method = "GET";
    let path = "/trade-api/ws/v2";
    let message = format!("{}{}{}", timestamp, method, path);

    // Parse the private key
    let priv_key = RsaPrivateKey::from_pkcs8_pem(private_key_pem)
        .or_else(|_| RsaPrivateKey::from_pkcs1_pem(private_key_pem))
        .map_err(|e| format!("Failed to parse private key: {}", e))?;

    // Hash the message
    let mut hasher = Sha256::new();
    hasher.update(message.as_bytes());
    let hashed = hasher.finalize();

    // Sign the hash using RSA-PSS SHA256
    // Note: Kalshi V2 uses RSA-PSS
    let mut rng = rand::thread_rng(); // rand 0.8 uses thread_rng()
    let pss = rsa::Pss::new::<Sha256>();
    let signature = priv_key.sign_with_rng(&mut rng, pss, &hashed)?;

    let signature_b64 = general_purpose::STANDARD.encode(signature);

    Ok(vec![
        ("KALSHI-ACCESS-KEY".to_string(), api_key_id.to_string()),
        ("KALSHI-ACCESS-TIMESTAMP".to_string(), timestamp),
        ("KALSHI-ACCESS-SIGNATURE".to_string(), signature_b64),
    ])
}
