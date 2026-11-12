use anyhow::Result;

/// CLOB API credentials + request signing (HMAC).
#[derive(Debug, Clone)]
pub struct ClobAuth {
    pub api_key: String,
    pub api_secret: String,
    pub private_key: String,
}

impl ClobAuth {
    pub fn new(api_key: String, api_secret: String, private_key: String) -> Self {
        Self {
            api_key,
            api_secret,
            private_key,
        }
    }

    pub fn from_config(cfg: &noaas_ark_core::config::ClobConfig) -> Self {
        Self::new(
            cfg.api_key.clone(),
            cfg.api_secret.clone(),
            cfg.private_key.clone(),
        )
    }

    /// Sign an order payload. v0.1: placeholder HMAC over the JSON body.
    /// Replace with the exact Polymarket CLOB signature scheme before live.
    pub fn sign_order(
        &self,
        order: &super::clob::OrderRequest,
    ) -> Result<String> {
        let body = serde_json::to_string(order)?;
        Ok(format!("{}:{}", self.api_key, simple_hash(&body, &self.api_secret)))
    }

    pub fn is_configured(&self) -> bool {
        !self.api_key.is_empty() && !self.private_key.is_empty()
    }
}

fn simple_hash(body: &str, secret: &str) -> String {
    // NOT for production - swap in HMAC-SHA256 per CLOB docs.
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};
    let mut h = DefaultHasher::new();
    body.hash(&mut h);
    secret.hash(&mut h);
    format!("{:016x}", h.finish())
}
