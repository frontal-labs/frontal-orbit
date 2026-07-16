#[cfg(test)]
mod tests {
    use crate::config::ProjectConfig;

    #[test]
    fn test_load_project_local_config() {
        // Test loading from the project-local config directory
        let project_root = std::env::current_dir().unwrap();
        let config_path = project_root.join("config").join("project.json");

        println!("Looking for config at: {config_path:?}");

        if config_path.exists() {
            match ProjectConfig::load_from_path(&config_path) {
                Ok(config) => {
                    println!("Successfully loaded project-local config:");
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
                    println!(
                        "Experimental features enabled: {}",
                        config.experimental.enable_new_features
                    );
                }
                Err(e) => {
                    println!("Failed to load config file: {e}");
                }
            }
        } else {
            println!("Config file does not exist at: {config_path:?}");
        }
    }
}
