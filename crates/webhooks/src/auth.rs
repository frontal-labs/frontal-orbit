//! Webhook authentication and verification.

use hmac::{Hmac, Mac};
use sha2::Sha256;
use std::collections::BTreeMap;

type HmacSha256 = Hmac<Sha256>;

/// Webhook authentication methods
#[derive(Debug, Clone)]
pub enum WebhookAuth {
    /// HMAC signature verification (GitHub-style)
    Hmac { secret: String, header: String },
    /// No authentication (for development)
    None,
}

/// HMAC authenticator for webhook signature verification
#[derive(Debug, Clone)]
pub struct HmacAuthenticator {
    secret: String,
}

impl HmacAuthenticator {
    #[must_use] 
    pub fn new(secret: String, _header_name: String) -> Self {
        Self { secret }
    }

    /// Verify webhook signature
    pub fn verify(&self, payload: &[u8], signature: &str) -> Result<bool, String> {
        // Parse signature (format: sha256=hex)
        let hex_sig = signature
            .strip_prefix("sha256=")
            .ok_or("Invalid signature format")?;

        // Decode hex signature
        let expected_bytes =
            hex::decode(hex_sig).map_err(|e| format!("Failed to decode signature: {e}"))?;

        // Compute HMAC
        let mut mac = HmacSha256::new_from_slice(self.secret.as_bytes())
            .map_err(|e| format!("Failed to create HMAC: {e}"))?;
        mac.update(payload);

        // Verify in constant time.
        Ok(mac.verify_slice(&expected_bytes).is_ok())
    }

    /// Generate signature for payload
    #[must_use] 
    pub fn sign(&self, payload: &[u8]) -> String {
        let mut mac = HmacSha256::new_from_slice(self.secret.as_bytes())
            .expect("HMAC can be created with valid key");
        mac.update(payload);
        let result = mac.finalize();
        let bytes = result.into_bytes();
        format!("sha256={encoded}", encoded = hex::encode(bytes))
    }
}

/// Extract webhook signature from headers
#[must_use] 
pub fn extract_signature(headers: &BTreeMap<String, String>, header_name: &str) -> Option<String> {
    headers.get(header_name).cloned()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hmac_signature_verification() {
        let secret = "test_secret".to_string();
        let authenticator = HmacAuthenticator::new(secret, "X-Hub-Signature-256".to_string());

        let payload = b"test payload";
        let signature = authenticator.sign(payload);

        assert!(authenticator.verify(payload, &signature).unwrap());

        // Test with wrong payload
        assert!(!authenticator.verify(b"wrong payload", &signature).unwrap());
    }

    #[test]
    fn test_signature_format() {
        let secret = "test_secret".to_string();
        let authenticator = HmacAuthenticator::new(secret, "X-Hub-Signature-256".to_string());

        let signature = authenticator.sign(b"test");
        assert!(signature.starts_with("sha256="));
        assert_eq!(signature.len(), 71); // "sha256=" + 64 hex chars
    }
}
