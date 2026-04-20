use crate::config::solver::SolverSettings;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Controls how market prices are fetched and overridden.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct PricingSettings {
    pub refresh_interval_secs: u64,
    pub coingecko_api_url: String,
    #[serde(default)]
    pub coingecko_api_key: Option<String>,
    #[serde(default)]
    pub static_prices: std::collections::HashMap<String, f64>,
}

impl Default for PricingSettings {
    /// Provides practical defaults for local development.
    fn default() -> Self {
        Self {
            refresh_interval_secs: 30,
            coingecko_api_url: "https://api.coingecko.com/api/v3/simple/price".to_string(),
            coingecko_api_key: None,
            static_prices: Default::default(),
        }
    }
}

/// Configures quote generation and order signing behavior.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct QuoteSettings {
    #[serde(default = "default_deadline_minutes")]
    pub order_deadline_in_minutes: i64,
    #[serde(default)]
    pub quote_private_key: Option<String>,
    #[serde(default)]
    pub max_user_slippage_bps: u64,
    #[serde(default = "default_eta")]
    pub default_eta_seconds: i32,
}

impl Default for QuoteSettings {
    /// Mirrors the default quote behavior expected by the unified service.
    fn default() -> Self {
        Self {
            order_deadline_in_minutes: default_deadline_minutes(),
            quote_private_key: None,
            max_user_slippage_bps: 300,
            default_eta_seconds: default_eta(),
        }
    }
}

/// Default create-order deadline for non-Bitcoin flows.
fn default_deadline_minutes() -> i64 {
    60
}

/// Fallback ETA returned when a route does not provide a chain-specific estimate.
fn default_eta() -> i32 {
    20
}

/// Root service settings loaded from `Settings.toml`.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Settings {
    pub addr: String,
    pub db_url: String,
    pub chain_json_path: String,
    pub strategy_path: String,
    pub chain_ids: HashMap<String, u128>,
    pub solver: SolverSettings,
    #[serde(default)]
    pub pricing: PricingSettings,
    #[serde(default)]
    pub quote: QuoteSettings,
    #[serde(default)]
    pub discord_webhook_url: Option<String>,
}

impl Settings {
    /// Reads and deserializes the service settings file.
    pub fn from_toml(path: &str) -> eyre::Result<Self> {
        let config = config::Config::builder()
            .add_source(config::File::with_name(path))
            .build()?;
        Ok(config.try_deserialize()?)
    }
}
