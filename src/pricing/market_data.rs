use crate::{config::settings::PricingSettings, metadata::MetadataIndex};
use eyre::Result;
use reqwest::header::{HeaderMap, HeaderValue};
use std::{collections::HashMap, sync::Arc};

/// Fetches USD prices for all known assets and overlays any configured statics.
pub async fn fetch_prices(
    settings: &PricingSettings,
    metadata: Arc<MetadataIndex>,
) -> Result<HashMap<String, f64>> {
    let mut prices = settings.static_prices.clone();

    // Only request IDs that the metadata says can be priced through CoinGecko.
    let ids = metadata
        .assets
        .iter()
        .filter_map(|asset| asset.coingecko_id.clone())
        .collect::<Vec<_>>();

    if ids.is_empty() {
        return Ok(prices);
    }

    let mut headers = HeaderMap::new();
    if let Some(api_key) = settings.coingecko_api_key.as_ref() {
        headers.insert("X-CG-API-KEY", HeaderValue::from_str(api_key)?);
    }

    let client = reqwest::Client::builder()
        .default_headers(headers)
        .build()?;
    let response = client
        .get(&settings.coingecko_api_url)
        .query(&[("ids", ids.join(",")), ("vs_currencies", "usd".to_string())])
        .send()
        .await?;

    let payload: HashMap<String, HashMap<String, f64>> = response.json().await?;
    for asset in &metadata.assets {
        // Re-key upstream prices by local asset ID so the quote engine stays chain-aware.
        if let Some(id) = asset.coingecko_id.as_ref() {
            if let Some(usd) = payload.get(id).and_then(|entry| entry.get("usd")) {
                prices.insert(asset.asset.id.to_string(), *usd);
            }
        }
    }

    Ok(prices)
}
