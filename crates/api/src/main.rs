use orbit_api::service::ApiServiceConfig;
use orbit_core::config::ProjectConfig;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // Load core configuration
    let core_config = ProjectConfig::load_or_default();
    println!("Starting Orbit API with core configuration:");
    println!(
        "  Project: {} v{}",
        core_config.project.name, core_config.project.version
    );
    println!(
        "  Default provider: {}",
        core_config.runtime.default_provider
    );
    println!(
        "  Max concurrent requests: {}",
        core_config.runtime.max_concurrent_requests
    );
    println!(
        "  Telemetry enabled: {}",
        core_config.features.enable_telemetry
    );

    let config = ApiServiceConfig::from_env()?;
    orbit_api::service::serve(config).await
}
