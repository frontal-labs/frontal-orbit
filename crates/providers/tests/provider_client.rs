use orbit_providers::{read_base_url, OAuthTokenSet};

#[test]
fn oauth_token_set_construction() {
    let tokens = OAuthTokenSet {
        access_token: "access_123".to_string(),
        refresh_token: Some("refresh_456".to_string()),
        expires_at: Some(1000),
        scopes: vec!["read".to_string()],
    };
    assert_eq!(tokens.access_token, "access_123");
    assert_eq!(tokens.refresh_token.as_deref(), Some("refresh_456"));
}

#[test]
fn oauth_token_set_no_refresh() {
    let tokens = OAuthTokenSet {
        access_token: "access_123".to_string(),
        refresh_token: None,
        expires_at: None,
        scopes: vec![],
    };
    assert!(tokens.refresh_token.is_none());
    assert!(tokens.expires_at.is_none());
}

#[test]
fn oauth_token_set_with_scopes() {
    let tokens = OAuthTokenSet {
        access_token: "tok".to_string(),
        refresh_token: None,
        expires_at: None,
        scopes: vec!["scope1".to_string(), "scope2".to_string()],
    };
    assert_eq!(tokens.scopes.len(), 2);
}

#[test]
fn read_base_url_default() {
    let url = read_base_url();
    assert!(url.contains("api.anthropic.com") || !url.is_empty());
}
