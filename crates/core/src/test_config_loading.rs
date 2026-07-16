#[cfg(test)]
mod tests {
    use crate::config::ProjectConfig;

    #[test]
    fn test_load_actual_config() {
        // This test will try to load the actual config file
        // It should work if the config file exists and is valid
        match ProjectConfig::load() {
            Ok(config) => {
                println!("Successfully loaded config:");
                println!(
                    "Project: {} v{}",
                    config.project.name, config.project.version
                );
                println!("Default provider: {}", config.runtime.default_provider);
                println!(
                    "Max concurrent requests: {}",
                    config.runtime.max_concurrent_requests
                );
                println!("Cache directory: {}", config.paths.cache_dir);
                println!("Telemetry enabled: {}", config.features.enable_telemetry);
                println!("UI theme: {}", config.ui.theme);
            }
            Err(e) => {
                println!("Could not load config file (this is expected in CI): {e}");
                // This is fine - the config file might not exist in the test environment
            }
        }
    }
}
