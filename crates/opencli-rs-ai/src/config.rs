//! Configuration file handling for opencli-rs.
//! Reads ~/.opencli-rs/config.json for LLM settings and other configuration.

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Config {
    #[serde(default)]
    pub llm: LlmConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct LlmConfig {
    /// API endpoint URL (e.g., "https://api.anthropic.com/v1/messages", "https://api.openai.com/v1/chat/completions")
    pub endpoint: Option<String>,
    /// API key
    pub apikey: Option<String>,
    /// Model name (e.g., "claude-sonnet-4-20250514", "gpt-4o")
    pub modelname: Option<String>,
}

impl LlmConfig {
    pub fn is_configured(&self) -> bool {
        self.endpoint.is_some() && self.apikey.is_some() && self.modelname.is_some()
    }
}

/// Get the config file path: ~/.opencli-rs/config.json
pub fn config_path() -> PathBuf {
    let home = std::env::var("HOME")
        .or_else(|_| std::env::var("USERPROFILE"))
        .unwrap_or_else(|_| ".".to_string());
    PathBuf::from(home).join(".opencli-rs").join("config.json")
}

/// Load config from ~/.opencli-rs/config.json
/// Returns default config if file doesn't exist or can't be parsed.
pub fn load_config() -> Config {
    let path = config_path();
    match std::fs::read_to_string(&path) {
        Ok(content) => serde_json::from_str(&content).unwrap_or_default(),
        Err(_) => Config::default(),
    }
}
