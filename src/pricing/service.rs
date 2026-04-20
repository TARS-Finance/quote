use crate::{config::settings::PricingSettings, metadata::MetadataIndex};
use moka::future::Cache;
use std::{collections::HashMap, sync::Arc, time::Duration};

/// Periodically refreshes USD prices and caches them for quote requests.
#[derive(Clone)]
pub struct PricingService {
    settings: PricingSettings,
    metadata: Arc<MetadataIndex>,
    prices: Cache<String, f64>,
}

impl PricingService {
    /// Creates the pricing service with an in-memory TTL cache.
    pub fn new(settings: PricingSettings, metadata: Arc<MetadataIndex>) -> Self {
        Self {
            settings,
            metadata,
            prices: Cache::builder()
                .time_to_live(Duration::from_secs(300))
                .build(),
        }
    }

    /// Starts the background refresh loop used by the quote service.
    pub fn start(self: &Arc<Self>) {
        let service = self.clone();
        tokio::spawn(async move {
            if let Err(error) = service.refresh().await {
                tracing::warn!(?error, "initial price refresh failed");
            }

            let mut interval =
                tokio::time::interval(Duration::from_secs(service.settings.refresh_interval_secs));
            loop {
                interval.tick().await;
                if let Err(error) = service.refresh().await {
                    tracing::warn!(?error, "price refresh failed");
                }
            }
        });
    }

    /// Returns the current cached USD price for an asset.
    pub async fn price_for(&self, asset_id: &str) -> Option<f64> {
        self.prices.get(&asset_id.to_lowercase()).await
    }

    /// Refreshes the cache from the configured market-data source.
    pub async fn refresh(&self) -> eyre::Result<()> {
        let prices =
            super::market_data::fetch_prices(&self.settings, self.metadata.clone()).await?;
        self.store_prices(prices).await;
        Ok(())
    }

    /// Stores a full snapshot of fetched prices in the cache.
    async fn store_prices(&self, prices: HashMap<String, f64>) {
        for (asset_id, price) in prices {
            self.prices.insert(asset_id.to_lowercase(), price).await;
        }
    }
}
