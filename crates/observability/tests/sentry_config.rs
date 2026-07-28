use orbit_observability::SentryConfig;

#[test]
fn disabled_by_default() {
    let config = SentryConfig::disabled();
    assert!(!config.enabled);
    assert!(config.dsn.is_none());
    assert!(!config.is_enabled());
}

#[test]
fn enabled_requires_dsn() {
    let config = SentryConfig::enabled("https://key@o0.ingest.sentry.io/1");
    assert!(config.enabled);
    assert_eq!(
        config.dsn.as_deref(),
        Some("https://key@o0.ingest.sentry.io/1")
    );
    assert!(config.is_enabled());
}

#[test]
fn enabled_without_dsn_returns_false() {
    let mut config = SentryConfig::disabled();
    config.enabled = true;
    assert!(!config.is_enabled());
}

#[test]
fn with_environment() {
    let config = SentryConfig::disabled().with_environment("staging");
    assert_eq!(config.environment.as_deref(), Some("staging"));
}

#[test]
fn with_release() {
    let config = SentryConfig::disabled().with_release("1.0.0");
    assert_eq!(config.release.as_deref(), Some("1.0.0"));
}

#[test]
fn with_server_name() {
    let config = SentryConfig::disabled().with_server_name("host-1");
    assert_eq!(config.server_name.as_deref(), Some("host-1"));
}

#[test]
fn with_traces_sample_rate() {
    let config = SentryConfig::disabled().with_traces_sample_rate(0.75);
    assert_eq!(config.traces_sample_rate, Some(0.75));
}

#[test]
fn with_profiles_sample_rate() {
    let config = SentryConfig::disabled().with_profiles_sample_rate(0.25);
    assert_eq!(config.profiles_sample_rate, Some(0.25));
}

#[test]
fn enabled_with_all_fields() {
    let config = SentryConfig::enabled("https://key@o0.ingest.sentry.io/1")
        .with_environment("production")
        .with_release("2.0.0")
        .with_server_name("prod-1")
        .with_traces_sample_rate(1.0)
        .with_profiles_sample_rate(0.5);
    assert!(config.is_enabled());
    assert_eq!(config.environment.as_deref(), Some("production"));
    assert_eq!(config.release.as_deref(), Some("2.0.0"));
    assert_eq!(config.server_name.as_deref(), Some("prod-1"));
    assert_eq!(config.traces_sample_rate, Some(1.0));
    assert_eq!(config.profiles_sample_rate, Some(0.5));
}

#[test]
fn default_is_disabled() {
    let config = SentryConfig::default();
    assert!(!config.enabled);
    assert!(!config.is_enabled());
}

#[test]
fn clone_and_debug() {
    let config =
        SentryConfig::enabled("https://key@o0.ingest.sentry.io/1").with_environment("test");
    let cloned = config.clone();
    assert_eq!(config, cloned);
    let debug = format!("{config:?}");
    assert!(!debug.is_empty());
}

#[test]
fn serde_roundtrip() {
    let config =
        SentryConfig::enabled("https://key@o0.ingest.sentry.io/1").with_environment("prod");
    let json = serde_json::to_string(&config).unwrap();
    let deserialized: SentryConfig = serde_json::from_str(&json).unwrap();
    assert_eq!(config, deserialized);
}
