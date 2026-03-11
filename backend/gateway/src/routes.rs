//! Axum 路由处理
//!
//! 数据获取优先级
//! ──────────────
//!   1. ArrowStore（内存热缓存 + 磁盘持久化）：TTL 内直接返回
//!   2. data-provider（Tushare / Yahoo Finance）：miss 或超过 TTL 时触发
//!      → 结果写回 ArrowStore（内存 + 磁盘）
//!
//! TTL
//! ───
//!   stocks  : 5 分钟
//!   ohlc    : 24 小时

use std::sync::Arc;
use std::time::Duration;

use axum::{
    extract::{Query, State},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Json, Response},
    routing::get,
    Router,
};
use serde::Deserialize;

use stock_data_provider::providers::{tushare, yahoo};
use stock_storage::HealthDto;

use crate::state::AppState;

const STOCK_TTL:   Duration = Duration::from_secs(60 * 5);
const HISTORY_TTL: Duration = Duration::from_secs(60 * 60 * 24);

// ── 错误包装 ──────────────────────────────────────────────────────────────────

struct ApiError(anyhow::Error);

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let msg = serde_json::json!({ "error": self.0.to_string() }).to_string();
        (StatusCode::INTERNAL_SERVER_ERROR, msg).into_response()
    }
}

impl<E: Into<anyhow::Error>> From<E> for ApiError {
    fn from(e: E) -> Self { ApiError(e.into()) }
}

// ── Query 参数 ────────────────────────────────────────────────────────────────

#[derive(Deserialize)]
pub struct StocksQuery {
    pub source:  String,
    pub symbols: Option<String>,
}

#[derive(Deserialize)]
pub struct HistoryQuery {
    pub source: String,
    pub symbol: String,
    #[serde(default = "default_days")]
    pub days:   usize,
}

fn default_days() -> usize { 90 }

// ── 辅助 ──────────────────────────────────────────────────────────────────────

fn header_str<'a>(headers: &'a HeaderMap, name: &str) -> &'a str {
    headers.get(name).and_then(|v| v.to_str().ok()).unwrap_or("")
}

fn default_tushare_symbols() -> Vec<String> {
    ["000001.SZ", "600519.SH", "601318.SH", "000858.SZ", "002594.SZ"]
        .iter().map(|s| s.to_string()).collect()
}

fn default_yahoo_symbols() -> Vec<String> {
    ["AAPL", "GOOGL", "MSFT", "TSLA", "AMZN"]
        .iter().map(|s| s.to_string()).collect()
}

fn parse_symbols(opt: Option<String>, default: Vec<String>) -> Vec<String> {
    opt.as_deref()
        .filter(|s| !s.is_empty())
        .map(|s| s.split(',').map(|sym| sym.trim().to_string()).filter(|s| !s.is_empty()).collect())
        .unwrap_or(default)
}

// ── GET /api/stocks ───────────────────────────────────────────────────────────

async fn get_stocks(
    State(state): State<Arc<AppState>>,
    Query(q): Query<StocksQuery>,
    headers: HeaderMap,
) -> Result<impl IntoResponse, ApiError> {
    let tushare_token = header_str(&headers, "x-tushare-token").to_string();
    let yahoo_api_key = header_str(&headers, "x-yahoo-api-key").to_string();

    let symbols   = parse_symbols(q.symbols.clone(),
        if q.source == "tushare" { default_tushare_symbols() } else { default_yahoo_symbols() });
    let cache_key = format!("stocks:{}:{}", q.source, symbols.join(","));

    let store = if q.source == "tushare" { &state.ashare_store } else { &state.store };

    if let Some(data) = store.get_stocks(&cache_key, STOCK_TTL) {
        tracing::info!("stocks db-hit  key={cache_key}");
        return Ok(Json(data));
    }

    tracing::info!("stocks api-fetch key={cache_key}");
    let fetch_result = match q.source.as_str() {
        "tushare" => tushare::fetch_stocks(&state.client, &tushare_token, &symbols).await,
        _         => yahoo::fetch_stocks(&state.client, &symbols, &yahoo_api_key).await,
    };

    match fetch_result {
        Ok(stocks) => {
            if let Err(e) = store.put_stocks(&cache_key, &stocks) {
                tracing::warn!("arrow put_stocks failed: {e}");
            }
            Ok(Json(stocks))
        }
        Err(e) => {
            tracing::warn!("stocks api error, trying stale cache: {e}");
            if let Some(stale) = store.get_stocks_stale(&cache_key) {
                tracing::info!("stocks stale-hit key={cache_key}");
                Ok(Json(stale))
            } else {
                Err(ApiError(e))
            }
        }
    }
}

// ── GET /api/history ──────────────────────────────────────────────────────────

async fn get_history(
    State(state): State<Arc<AppState>>,
    Query(q): Query<HistoryQuery>,
    headers: HeaderMap,
) -> Result<impl IntoResponse, ApiError> {
    let tushare_token = header_str(&headers, "x-tushare-token").to_string();
    let yahoo_api_key = header_str(&headers, "x-yahoo-api-key").to_string();

    let cache_key = format!("ohlc:{}:{}:{}", q.source, q.symbol, q.days);
    let store = if q.source == "tushare" { &state.ashare_store } else { &state.store };

    if let Some(data) = store.get_ohlc(&cache_key, HISTORY_TTL) {
        tracing::info!("ohlc db-hit  key={cache_key}");
        return Ok(Json(data));
    }

    tracing::info!("ohlc api-fetch key={cache_key}");
    let ohlc = match q.source.as_str() {
        "tushare" => tushare::fetch_history(&state.client, &tushare_token, &q.symbol, q.days).await?,
        _         => yahoo::fetch_history(&state.client, &q.symbol, q.days, &yahoo_api_key).await?,
    };

    if let Err(e) = store.put_ohlc(&cache_key, &ohlc) {
        tracing::warn!("arrow put_ohlc failed: {e}");
    }

    Ok(Json(ohlc))
}

// ── GET /api/health ───────────────────────────────────────────────────────────

async fn health(State(state): State<Arc<AppState>>) -> Json<HealthDto> {
    Json(HealthDto {
        status:      "ok",
        cached_keys: state.store.cached_key_count(),
    })
}

// ── Router ────────────────────────────────────────────────────────────────────

pub fn router() -> Router<Arc<AppState>> {
    Router::new()
        .route("/stocks",  get(get_stocks))
        .route("/history", get(get_history))
        .route("/health",  get(health))
}
