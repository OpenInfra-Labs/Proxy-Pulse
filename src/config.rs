use serde::Deserialize;
use std::path::Path;

#[derive(Debug, Clone, Deserialize)]
pub struct AppConfig {
    pub server: ServerConfig,
    pub database: DatabaseConfig,
    pub sources: SourcesConfig,
    pub checker: CheckerConfig,
    pub scoring: ScoringConfig,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ServerConfig {
    pub host: String,
    pub port: u16,
}

#[derive(Debug, Clone, Deserialize)]
pub struct DatabaseConfig {
    pub url: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct SourcesConfig {
    pub sync_interval_secs: u64,
    pub providers: Vec<ProviderConfig>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ProviderConfig {
    #[serde(rename = "type")]
    pub provider_type: String,
    pub path: Option<String>,
    pub url: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct CheckerConfig {
    pub interval_secs: u64,
    pub timeout_secs: u64,
    pub max_concurrent: usize,
    pub targets: Vec<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ScoringConfig {
    #[allow(dead_code)]
    pub min_score: f64,
    pub weight_success_rate: f64,
    pub weight_latency: f64,
    pub weight_stability: f64,
}

impl AppConfig {
    pub fn load<P: AsRef<Path>>(path: P) -> anyhow::Result<Self> {
        let content = std::fs::read_to_string(path)?;
        let config: AppConfig = serde_yaml::from_str(&content)?;
        Ok(config)
    }
}
