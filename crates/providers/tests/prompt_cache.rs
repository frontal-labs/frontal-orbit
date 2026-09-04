use orbit_providers::{
    CacheBreakEvent, PromptCache, PromptCacheConfig, PromptCachePaths, PromptCacheRecord,
    PromptCacheStats,
};

#[test]
fn prompt_cache_config_default() {
    let config = PromptCacheConfig::default();
    assert_eq!(config.session_id, "default");
    assert!(config.completion_ttl.as_secs() > 0);
    assert!(config.prompt_ttl.as_secs() >= config.completion_ttl.as_secs());
}

#[test]
fn prompt_cache_config_custom_session() {
    let config = PromptCacheConfig::new("session-test");
    assert_eq!(config.session_id, "session-test");
}

#[test]
fn prompt_cache_paths_for_session() {
    let paths = PromptCachePaths::for_session("test-session");
    assert!(paths
        .session_dir
        .to_str()
        .unwrap_or("")
        .contains("test-session"));
}

#[test]
fn prompt_cache_stats_default() {
    let stats = PromptCacheStats::default();
    assert_eq!(stats.completion_cache_hits, 0);
    assert_eq!(stats.completion_cache_misses, 0);
}

#[test]
fn cache_break_event_construction() {
    let event = CacheBreakEvent {
        unexpected: false,
        reason: "new_tools".to_string(),
        previous_cache_read_input_tokens: 5000,
        current_cache_read_input_tokens: 200,
        token_drop: 4800,
    };
    assert!(event.token_drop > 0);
    assert_eq!(event.reason, "new_tools");
}

#[test]
fn prompt_cache_new() {
    let cache = PromptCache::new("test-session");
    assert!(cache
        .paths()
        .session_dir
        .to_str()
        .unwrap_or("")
        .contains("test-session"));
}

#[test]
fn prompt_cache_new_with_config() {
    let config = PromptCacheConfig::new("configured-session");
    let cache = PromptCache::with_config(config);
    assert!(cache
        .paths()
        .session_dir
        .to_str()
        .unwrap_or("")
        .contains("configured-session"));
}

#[test]
fn prompt_cache_stats_access() {
    let cache = PromptCache::new("stats-test");
    let stats = cache.stats();
    assert_eq!(stats.tracked_requests, 0);
}

#[test]
fn prompt_cache_record_contains_cache_break() {
    let event = CacheBreakEvent {
        unexpected: true,
        reason: "test".to_string(),
        previous_cache_read_input_tokens: 100,
        current_cache_read_input_tokens: 50,
        token_drop: 50,
    };
    let record = PromptCacheRecord {
        cache_break: Some(event),
        stats: PromptCacheStats::default(),
    };
    assert!(record.cache_break.is_some());
    assert!(record.stats.completion_cache_hits == 0);
}
