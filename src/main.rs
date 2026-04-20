use eyre::Result;
use std::sync::Arc;
use tars::orderbook::OrderbookProvider;
use tars_orderbook::{
    app_state::AppState, config::settings::Settings, liquidity::watcher::LiquidityWatcher,
    metadata::MetadataIndex, orders::service::OrderService, pricing::service::PricingService,
    quote::service::QuoteService, read_api::service::ReadApiService, registry::StrategyRegistry,
    server,
};
use tracing::Level;

/// Boots the unified service and wires together shared background workers.
#[tokio::main]
async fn main() -> Result<()> {
    let settings = Settings::from_toml("Settings.toml")?;

    // Initialize tracing before any long-lived services start emitting logs.
    match settings.discord_webhook_url.as_ref() {
        Some(webhook) => tars::utils::setup_tracing_with_webhook(
            webhook,
            "Tars Unified Orderbook",
            Level::ERROR,
            None,
        )?,
        None => {
            let _ = tracing_subscriber::fmt().pretty().try_init();
        }
    }

    // Build the shared registries and providers used by every request path.
    let metadata = Arc::new(MetadataIndex::load(&settings.chain_json_path)?);
    let registry = Arc::new(StrategyRegistry::load(
        &settings.strategy_path,
        metadata.as_ref(),
    )?);
    let orderbook = Arc::new(OrderbookProvider::from_db_url(&settings.db_url).await?);

    let pricing = Arc::new(PricingService::new(
        settings.pricing.clone(),
        metadata.clone(),
    ));
    // Refresh pricing in the background so quote requests stay read-only.
    pricing.start();

    let liquidity = Arc::new(
        LiquidityWatcher::new(settings.solver.clone(), metadata.clone(), orderbook.clone()).await?,
    );
    // Keep solver balances fresh independently of request traffic.
    liquidity.start();

    let quote_service = Arc::new(QuoteService::new(
        settings.quote.clone(),
        metadata.clone(),
        registry.clone(),
        pricing.clone(),
        liquidity.clone(),
    ));

    let order_service = Arc::new(OrderService::new(
        orderbook.clone(),
        quote_service.clone(),
        settings.quote.clone(),
        settings.chain_ids.clone(),
    ));

    let read_api = Arc::new(ReadApiService::new(orderbook.clone()));

    // Hand off the fully wired application state to the HTTP server.
    let state = Arc::new(AppState::new(
        settings,
        metadata,
        registry,
        pricing,
        liquidity,
        quote_service,
        order_service,
        read_api,
        orderbook,
    ));

    server::serve(state).await
}
