use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::time::Duration;

use orbit_server::{LaneTransportKind, ServerConfig};

#[test]
fn default_server_config() {
    let config = ServerConfig::default();
    assert_eq!(
        config.bind_addr,
        SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), 8788)
    );
    assert_eq!(config.event_replay_limit, 100);
    assert_eq!(config.lane_transport_kind, LaneTransportKind::InMemory);
    assert!(config.api_key.is_none());
    assert_eq!(config.reconcile_interval, Some(Duration::from_secs(15)));
}

#[test]
fn lane_transport_kind_default() {
    assert_eq!(LaneTransportKind::InMemory, LaneTransportKind::InMemory);
    assert_eq!(
        LaneTransportKind::LocalDocker,
        LaneTransportKind::LocalDocker
    );
    assert_eq!(LaneTransportKind::ToolsAgent, LaneTransportKind::ToolsAgent);
}

#[test]
fn lane_transport_kind_debug() {
    let in_memory = LaneTransportKind::InMemory;
    assert_eq!(format!("{in_memory:?}"), "InMemory");
    assert_eq!(
        format!("{:?}", LaneTransportKind::LocalDocker),
        "LocalDocker"
    );
    assert_eq!(format!("{:?}", LaneTransportKind::ToolsAgent), "ToolsAgent");
}

#[test]
fn lane_transport_kind_clone_and_eq() {
    let a = LaneTransportKind::InMemory;
    let b = a;
    assert_eq!(a, b);

    let kinds = [
        LaneTransportKind::InMemory,
        LaneTransportKind::LocalDocker,
        LaneTransportKind::ToolsAgent,
    ];
    for i in 0..kinds.len() {
        for j in 0..kinds.len() {
            if i == j {
                assert_eq!(kinds[i], kinds[j]);
            } else {
                assert_ne!(kinds[i], kinds[j]);
            }
        }
    }
}

#[test]
fn server_config_from_env_empty() {
    let config = ServerConfig::from_env().expect("from_env should succeed with no env vars set");
    let default = ServerConfig::default();
    assert_eq!(config.bind_addr, default.bind_addr);
    assert_eq!(config.event_replay_limit, default.event_replay_limit);
    assert_eq!(config.lane_transport_kind, default.lane_transport_kind);
    assert_eq!(config.api_key, default.api_key);
    assert_eq!(config.reconcile_interval, default.reconcile_interval);
}
