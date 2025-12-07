//! Configuration loader and typed settings for the application.
//!
//! Loads configuration from a user config file (platform-appropriate
//! location) and environment variables prefixed with `PD__`.
use anyhow::Result;
use config::{Config, Environment, File};
use directories::ProjectDirs;
use serde::Deserialize;

/// Application configuration settings loaded from a file.
///
/// # Fields
/// * `threads` - Default number of concurrent download threads.
/// * `rate_limit` - Default rate limit in bytes per second.
/// * `default_dir` - Default directory to save downloaded files.
/// * `concurrent_files` - Default number of files to download at a time.
#[derive(Debug, Deserialize, Default)]
pub struct Settings {
    pub threads: Option<u8>,
    pub rate_limit: Option<u32>,
    pub default_dir: Option<String>,
    pub concurrent_files: Option<usize>,
}

impl Settings {
    /// Loads the configuration settings from a file.
    ///
    /// # Returns
    /// Returns a `Settings` instance with the loaded configuration.
    pub fn load() -> Result<Self> {
        let mut s = Config::builder();

        // Linux:   ~/.config/pd/config.toml
        // Windows: C:\Users\Name\AppData\Roaming\pd\config.toml
        // Mac:     ~/Library/Application Support/pd/config.toml
        if let Some(proj_dirs) = ProjectDirs::from("com", "parallel_downloader", "pd") {
            let config_dir = proj_dirs.config_dir();
            let config_path = config_dir.join("config.toml");

            if config_path.exists() {
                s = s.add_source(File::from(config_path));
            }
        };

        s = s.add_source(Environment::with_prefix("PD").separator("__"));

        match s.build() {
            Ok(config) => Ok(config.try_deserialize().unwrap_or_default()),
            Err(_) => Ok(Self::default()),
        }
    }
}
