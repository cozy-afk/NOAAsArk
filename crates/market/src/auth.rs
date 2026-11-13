use anyhow::{Context, Result};
use base64::Engine as _;
use hmac::{Hmac, Mac};
use sha2::Sha256;

/// CLOB API credentials + request signing (HMAC-SHA256, L2 scheme).
///
/// Polymarket CLOB L2 auth (per their docs):
///   message   = timestamp + method + path + body
///   secret    = base64-decoded api_secret
///   signature = base64(HMAC-SHA256(secret, message))
/// Headers sent: POLY_ADDRESS, POLY_SIGNATURE, POLY_TIMESTAMP,
/// POLY_API_KEY, POLY_PASSPHRASE.
#[derive(Debug, Clone)]
pub struct ClobAuth {
    pub api_key: String,
    pub api_secret: String,
    pub api_passphrase: String,
    pub private_key: String,
    pub wallet_address: String,
}

impl ClobAuth {
    pub fn new(
        api_key: String,
        api_secret: String,
        api_passphrase: String,
        private_key: String,
        wallet_address: String,
    ) -> Self {
        Self {
            api_key,
            api_secret,
            api_passphrase,
            private_key,
            wallet_address,
        }
    }

    pub fn from_config(cfg: &noaas_ark_core::config::ClobConfig) -> Self {
        Self::new(
            cfg.api_key.clone(),
            cfg.api_secret.clone(),
            cfg.api_passphrase.clone(),
            cfg.private_key.clone(),
            cfg.wallet_address.clone(),
        )
    }

    /// L2 HMAC signature for a request.
    pub fn sign_l2(&self, method: &str, path: &str, body: &str, timestamp: i64) -> Result<String> {
        let secret_bytes = base64::engine::general_purpose::STANDARD
            .decode(self.api_secret.as_bytes())
            .or_else(|_| {
                base64::engine::general_purpose::URL_SAFE.decode(self.api_secret.as_bytes())
            })
            .context("api_secret is not valid base64")?;
        let message = format!("{}{}{}{}", timestamp, method.to_uppercase(), path, body);
        let mut mac = Hmac::<Sha256>::new_from_slice(&secret_bytes)
            .map_err(|e| anyhow::anyhow!("hmac key error: {e}"))?;
        mac.update(message.as_bytes());
        let sig = mac.finalize().into_bytes();
        Ok(base64::engine::general_purpose::STANDARD.encode(sig))
    }

    /// Legacy order signing entry point kept for the bot's place_order path.
    /// Builds the L2 signature over the serialized order body.
    pub fn sign_order(&self, order: &super::clob::OrderRequest) -> Result<String> {
        let body = serde_json::to_string(order)?;
        let ts = chrono::Utc::now().timestamp();
        self.sign_l2("POST", "/order", &body, ts)
    }

    pub fn is_configured(&self) -> bool {
        !self.api_key.is_empty() && !self.api_secret.is_empty() && !self.private_key.is_empty()
    }

    pub fn wallet_address(&self) -> &str {
        &self.wallet_address
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn auth() -> ClobAuth {
        // 32 bytes of 0x01 base64-encoded.
        let secret = base64::engine::general_purpose::STANDARD.encode(vec![1u8; 32]);
        ClobAuth::new(
            "key".into(),
            secret,
            "pass".into(),
            "0xabc".into(),
            "0xwallet".into(),
        )
    }

    #[test]
    fn sign_l2_is_deterministic() {
        let a = auth();
        let s1 = a
            .sign_l2("POST", "/order", "{\"price\":0.5}", 1_700_000_000)
            .unwrap();
        let s2 = a
            .sign_l2("POST", "/order", "{\"price\":0.5}", 1_700_000_000)
            .unwrap();
        assert_eq!(s1, s2);
        assert!(!s1.is_empty());
    }

    #[test]
    fn sign_l2_changes_with_timestamp() {
        let a = auth();
        let s1 = a.sign_l2("POST", "/order", "{}", 1).unwrap();
        let s2 = a.sign_l2("POST", "/order", "{}", 2).unwrap();
        assert_ne!(s1, s2);
    }

    #[test]
    fn sign_l2_changes_with_body() {
        let a = auth();
        let s1 = a.sign_l2("POST", "/order", "{\"a\":1}", 1).unwrap();
        let s2 = a.sign_l2("POST", "/order", "{\"a\":2}", 1).unwrap();
        assert_ne!(s1, s2);
    }

    #[test]
    fn sign_l2_rejects_bad_secret() {
        let mut a = auth();
        a.api_secret = "not-base64!!!".into();
        assert!(a.sign_l2("GET", "/book", "", 1).is_err());
    }
}
