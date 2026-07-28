use orbit_webhooks::{HmacAuthenticator, WebhookAuth};

#[test]
fn hmac_sign_and_verify() {
    let auth = HmacAuthenticator::new("secret123".to_string(), "X-Hub-Signature-256".to_string());
    let payload = b"test payload data";
    let signature = auth.sign(payload);
    assert!(signature.starts_with("sha256="));
    assert_eq!(signature.len(), 71);
}

#[test]
fn hmac_verify_correct_payload() {
    let auth = HmacAuthenticator::new("secret123".to_string(), "X-Hub-Signature-256".to_string());
    let payload = b"test payload";
    let signature = auth.sign(payload);
    let result = auth.verify(payload, &signature).unwrap();
    assert!(result);
}

#[test]
fn hmac_verify_wrong_payload() {
    let auth = HmacAuthenticator::new("secret123".to_string(), "X-Hub-Signature-256".to_string());
    let payload = b"test payload";
    let signature = auth.sign(payload);
    let result = auth.verify(b"wrong payload", &signature).unwrap();
    assert!(!result);
}

#[test]
fn hmac_verify_invalid_signature_format() {
    let auth = HmacAuthenticator::new("secret123".to_string(), "X-Hub-Signature-256".to_string());
    let result = auth.verify(b"test", "invalid-format");
    assert!(result.is_err());
}

#[test]
fn hmac_verify_empty_payload() {
    let auth = HmacAuthenticator::new("secret".to_string(), "X-Hub-Signature-256".to_string());
    let payload = b"";
    let signature = auth.sign(payload);
    let result = auth.verify(payload, &signature).unwrap();
    assert!(result);
}

#[test]
fn webhook_auth_hmac_variant() {
    let auth = WebhookAuth::Hmac {
        secret: "s".to_string(),
        header: "X-Sig".to_string(),
    };
    match &auth {
        WebhookAuth::Hmac { secret, header } => {
            assert_eq!(secret, "s");
            assert_eq!(header, "X-Sig");
        }
        WebhookAuth::None => panic!("expected Hmac"),
    }
}

#[test]
fn webhook_auth_none_variant() {
    match WebhookAuth::None {
        WebhookAuth::None => {}
        WebhookAuth::Hmac { .. } => panic!("expected None"),
    }
}
