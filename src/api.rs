use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::Json,
    routing::get,
    Router,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use crate::db::Database;
use crate::models::{ProxyResponse, ProxyStats};

/// Shared application state
pub struct AppState {
    pub db: Database,
}

#[derive(Debug, Deserialize)]
pub struct PaginationParams {
    pub page: Option<i64>,
    pub per_page: Option<i64>,
}

#[derive(Debug, Deserialize)]
pub struct TopParams {
    pub limit: Option<i64>,
}

#[derive(Debug, Serialize)]
pub struct ApiResponse<T: Serialize> {
    pub success: bool,
    pub data: T,
}

#[derive(Debug, Serialize)]
pub struct ErrorResponse {
    pub success: bool,
    pub error: String,
}

#[derive(Debug, Serialize)]
pub struct ProxyListResponse {
    pub proxies: Vec<ProxyResponse>,
    pub count: usize,
}

/// Build the API router
pub fn api_router() -> Router<Arc<AppState>> {
    Router::new()
        .route("/api/v1/proxy/random", get(get_random_proxy))
        .route("/api/v1/proxy/top", get(get_top_proxies))
        .route("/api/v1/proxy/country/{country}", get(get_proxies_by_country))
        .route("/api/v1/proxy/all", get(get_all_proxies))
        .route("/api/v1/proxy/stats", get(get_stats))
        .route("/api/v1/health", get(health_check))
}

/// GET /api/v1/proxy/random
async fn get_random_proxy(
    State(state): State<Arc<AppState>>,
) -> Result<Json<ApiResponse<Option<ProxyResponse>>>, (StatusCode, Json<ErrorResponse>)> {
    match state.db.get_random_alive_proxy().await {
        Ok(proxy) => Ok(Json(ApiResponse {
            success: true,
            data: proxy.map(ProxyResponse::from),
        })),
        Err(e) => Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                success: false,
                error: e.to_string(),
            }),
        )),
    }
}

/// GET /api/v1/proxy/top?limit=10
async fn get_top_proxies(
    State(state): State<Arc<AppState>>,
    Query(params): Query<TopParams>,
) -> Result<Json<ApiResponse<ProxyListResponse>>, (StatusCode, Json<ErrorResponse>)> {
    let limit = params.limit.unwrap_or(10).min(100);

    match state.db.get_top_proxies(limit).await {
        Ok(proxies) => {
            let count = proxies.len();
            let proxies: Vec<ProxyResponse> = proxies.into_iter().map(ProxyResponse::from).collect();
            Ok(Json(ApiResponse {
                success: true,
                data: ProxyListResponse { proxies, count },
            }))
        }
        Err(e) => Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                success: false,
                error: e.to_string(),
            }),
        )),
    }
}

/// GET /api/v1/proxy/country/:country
async fn get_proxies_by_country(
    State(state): State<Arc<AppState>>,
    Path(country): Path<String>,
    Query(params): Query<TopParams>,
) -> Result<Json<ApiResponse<ProxyListResponse>>, (StatusCode, Json<ErrorResponse>)> {
    let limit = params.limit.unwrap_or(20).min(100);

    match state.db.get_proxies_by_country(&country, limit).await {
        Ok(proxies) => {
            let count = proxies.len();
            let proxies: Vec<ProxyResponse> = proxies.into_iter().map(ProxyResponse::from).collect();
            Ok(Json(ApiResponse {
                success: true,
                data: ProxyListResponse { proxies, count },
            }))
        }
        Err(e) => Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                success: false,
                error: e.to_string(),
            }),
        )),
    }
}

/// GET /api/v1/proxy/all?page=1&per_page=20
async fn get_all_proxies(
    State(state): State<Arc<AppState>>,
    Query(params): Query<PaginationParams>,
) -> Result<Json<ApiResponse<ProxyListResponse>>, (StatusCode, Json<ErrorResponse>)> {
    let page = params.page.unwrap_or(1).max(1);
    let per_page = params.per_page.unwrap_or(20).min(100);

    match state.db.get_all_proxies(page, per_page).await {
        Ok(proxies) => {
            let count = proxies.len();
            let proxies: Vec<ProxyResponse> = proxies.into_iter().map(ProxyResponse::from).collect();
            Ok(Json(ApiResponse {
                success: true,
                data: ProxyListResponse { proxies, count },
            }))
        }
        Err(e) => Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                success: false,
                error: e.to_string(),
            }),
        )),
    }
}

/// GET /api/v1/proxy/stats
async fn get_stats(
    State(state): State<Arc<AppState>>,
) -> Result<Json<ApiResponse<ProxyStats>>, (StatusCode, Json<ErrorResponse>)> {
    match state.db.get_stats().await {
        Ok(stats) => Ok(Json(ApiResponse {
            success: true,
            data: stats,
        })),
        Err(e) => Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                success: false,
                error: e.to_string(),
            }),
        )),
    }
}

/// GET /api/v1/health
async fn health_check() -> Json<ApiResponse<String>> {
    Json(ApiResponse {
        success: true,
        data: "Proxy Pulse is running".to_string(),
    })
}
