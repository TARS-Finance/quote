use crate::{
    AppState, error::AppError, orders::types::CreateOrderRequest, quote::types::QuoteRequest,
    registry::pairs::derive_pairs, server::response::success,
};
use axum::{
    Json,
    extract::{Path, Query, State},
};
use std::sync::Arc;
use tars::orderbook::primitives::OrderQueryFilters;

/// Lightweight readiness endpoint used by deployments and local smoke tests.
pub async fn health() -> Json<crate::server::response::ApiResponse<serde_json::Value>> {
    success(serde_json::json!({ "status": "ok" }))
}

/// Returns route candidates for a requested swap.
pub async fn quote(
    State(state): State<Arc<AppState>>,
    Query(request): Query<QuoteRequest>,
) -> Result<Json<crate::server::response::ApiResponse<crate::quote::types::QuoteResponse>>, AppError>
{
    let response = state.quote_service.quote(request).await?;
    Ok(success(response))
}

/// Prices and persists a matched order in one request.
pub async fn create_order(
    State(state): State<Arc<AppState>>,
    Json(request): Json<CreateOrderRequest>,
) -> Result<
    Json<crate::server::response::ApiResponse<tars::orderbook::primitives::MatchedOrderVerbose>>,
    AppError,
> {
    let response = state.order_service.create_order(request).await?;
    Ok(success(response))
}

/// Fetches a single persisted order by create ID or swap ID.
pub async fn get_order(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<
    Json<crate::server::response::ApiResponse<tars::orderbook::primitives::MatchedOrderVerbose>>,
    AppError,
> {
    let order = state
        .read_api
        .get_order(&id)
        .await?
        .ok_or_else(|| AppError::not_found(format!("order not found: {id}")))?;
    Ok(success(order))
}

/// Lists orders from the shared orderbook using the existing Garden filters.
pub async fn list_orders(
    State(state): State<Arc<AppState>>,
    Query(filters): Query<OrderQueryFilters>,
) -> Result<
    Json<
        crate::server::response::ApiResponse<
            tars::orderbook::primitives::PaginatedData<
                tars::orderbook::primitives::MatchedOrderVerbose,
            >,
        >,
    >,
    AppError,
> {
    let orders = state.read_api.list_orders(filters).await?;
    Ok(success(orders))
}

/// Exposes the current in-memory solver liquidity snapshot.
pub async fn liquidity(
    State(state): State<Arc<AppState>>,
) -> Result<
    Json<crate::server::response::ApiResponse<crate::liquidity::primitives::SolverLiquidity>>,
    AppError,
> {
    Ok(success(state.liquidity.all().await))
}

/// Returns the loaded strategy registry keyed by strategy ID.
pub async fn strategies(
    State(state): State<Arc<AppState>>,
) -> Result<
    Json<
        crate::server::response::ApiResponse<
            std::collections::HashMap<String, crate::registry::Strategy>,
        >,
    >,
    AppError,
> {
    Ok(success(state.registry.all_strategies().clone()))
}

/// Returns the supported order pairs derived from loaded strategies.
pub async fn pairs(
    State(state): State<Arc<AppState>>,
) -> Result<
    Json<crate::server::response::ApiResponse<Vec<crate::registry::pairs::PairDescriptor>>>,
    AppError,
> {
    Ok(success(derive_pairs(&state.registry)))
}
