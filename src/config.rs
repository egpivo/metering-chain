use std::env;
use std::path::PathBuf;

/// Configuration for metering-chain CLI tool
///
/// This is a simple, single-threaded config suitable for the MVP.
/// For multi-node scenarios, consider adding node addresses and mining settings.
#[derive(Debug, Clone)]
pub struct Config {
    /// Data directory path (default: `.metering-chain/` in current directory)
    pub data_dir: PathBuf,

    /// Output format: "human" (default) or "json"
    pub output_format: String,

    /// Log level: "info", "debug", "warn", "error" (default: "info")
    pub log_level: String,

    /// PoW mining difficulty target (optional, for future use)
    /// Represented as a hex string of the target hash
    pub pow_target: Option<String>,
}

impl Config {
    /// Create a new config with defaults
    pub fn new() -> Self {
        let data_dir = env::current_dir()
            .unwrap_or_else(|_| PathBuf::from("."))
            .join(".metering-chain");

        Config {
            data_dir,
            output_format: "human".to_string(),
            log_level: "info".to_string(),
            pow_target: None,
        }
    }

    /// Create config with custom data directory
    pub fn with_data_dir(data_dir: PathBuf) -> Self {
        Config {
            data_dir,
            ..Config::new()
        }
    }

    /// Get the data directory path
    pub fn get_data_dir(&self) -> &PathBuf {
        &self.data_dir
    }

    /// Set data directory
    pub fn set_data_dir(&mut self, dir: PathBuf) {
        self.data_dir = dir;
    }

    /// Get output format
    pub fn get_output_format(&self) -> &str {
        &self.output_format
    }

    /// Set output format ("human" or "json")
    pub fn set_output_format(&mut self, format: String) {
        self.output_format = format;
    }

    /// Get log level
    pub fn get_log_level(&self) -> &str {
        &self.log_level
    }

    /// Set log level
    pub fn set_log_level(&mut self, level: String) {
        self.log_level = level;
    }

    /// Get transaction log path
    pub fn get_tx_log_path(&self) -> PathBuf {
        self.data_dir.join("tx.log")
    }

    /// Get state snapshot path
    pub fn get_state_path(&self) -> PathBuf {
        self.data_dir.join("state.bin")
    }

    /// Get state JSON path (for debugging)
    pub fn get_state_json_path(&self) -> PathBuf {
        self.data_dir.join("state.json")
    }

    /// Load config from environment variables
    ///
    /// Environment variables:
    /// - `METERING_CHAIN_DATA_DIR`: override data directory
    /// - `METERING_CHAIN_OUTPUT_FORMAT`: "human" or "json"
    /// - `METERING_CHAIN_LOG_LEVEL`: log level
    pub fn from_env() -> Self {
        let mut config = Config::new();

        if let Ok(dir) = env::var("METERING_CHAIN_DATA_DIR") {
            config.data_dir = PathBuf::from(dir);
        }

        if let Ok(format) = env::var("METERING_CHAIN_OUTPUT_FORMAT") {
            config.output_format = format;
        }

        if let Ok(level) = env::var("METERING_CHAIN_LOG_LEVEL") {
            config.log_level = level;
        }

        config
    }
}

impl Default for Config {
    fn default() -> Self {
        Config::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_defaults() {
        let config = Config::new();
        assert_eq!(config.output_format, "human");
        assert_eq!(config.log_level, "info");
        assert!(config.data_dir.ends_with(".metering-chain"));
    }

    #[test]
    fn test_config_paths() {
        let config = Config::new();
        assert!(config.get_tx_log_path().ends_with("tx.log"));
        assert!(config.get_state_path().ends_with("state.bin"));
    }

    #[test]
    fn test_config_setters() {
        let mut config = Config::new();
        config.set_output_format("json".to_string());
        assert_eq!(config.get_output_format(), "json");

        config.set_log_level("debug".to_string());
        assert_eq!(config.get_log_level(), "debug");
    }
}
