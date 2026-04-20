use axum::Json;
use serde::Serialize;

/// Consistent success envelope for all JSON endpoints.
#[derive(Debug, Serialize)]
pub struct ApiResponse<T> {
    pub ok: bool,
    pub data: T,
}

/// Wraps successful handler output in the standard response envelope.
pub fn success<T: Serialize>(data: T) -> Json<ApiResponse<T>> {
    Json(ApiResponse { ok: true, data })
}
