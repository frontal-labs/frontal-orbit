use orbit_server::ServerConfig;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let config = ServerConfig::from_env()?;
    orbit_server::serve(config).await
}
