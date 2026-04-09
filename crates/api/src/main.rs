use orbit_api::service::ApiServiceConfig;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let config = ApiServiceConfig::from_env()?;
    orbit_api::service::serve(config).await
}
