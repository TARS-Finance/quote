use serde::Serialize;

/// USD price snapshot for a single asset.
#[derive(Debug, Clone, Serialize)]
pub struct AssetPrice {
    pub asset_id: String,
    pub usd_price: f64,
}
