use orbit_webhooks::{WebhookAuth, WebhookConfig, WebhookReceiver};

#[test]
fn webhook_config_default() {
    let config = WebhookConfig::default();
    assert_eq!(config.port, 8555);
    assert!(matches!(config.auth, WebhookAuth::None));
    assert_eq!(config.max_events, 1000);
}

#[test]
fn webhook_config_custom_port() {
    let config = WebhookConfig {
        port: 9000,
        auth: WebhookAuth::None,
        max_events: 500,
    };
    assert_eq!(config.port, 9000);
    assert_eq!(config.max_events, 500);
}

#[test]
fn webhook_config_with_hmac_auth() {
    let config = WebhookConfig {
        port: 8555,
        auth: WebhookAuth::Hmac {
            secret: "my-secret".to_string(),
            header: "X-Signature".to_string(),
        },
        max_events: 1000,
    };
    match &config.auth {
        WebhookAuth::Hmac { secret, header } => {
            assert_eq!(secret, "my-secret");
            assert_eq!(header, "X-Signature");
        }
        WebhookAuth::None => panic!("expected Hmac"),
    }
}

#[test]
fn webhook_receiver_construction() {
    let config = WebhookConfig::default();
    let receiver = WebhookReceiver::new(config);
    let processor = receiver.event_processor();
    assert!(processor.try_read().is_ok());
}
