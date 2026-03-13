//! 接口层：Axum 路由处理

use std::sync::Arc;

use axum::{
    extract::{Path, Query, State},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Json, Response},
    routing::{delete, get, put},
    Router,
};
use serde::{Deserialize, Serialize};

use stock_application::{EtfAppService, StockAppService};
use stock_domain::DomainError;
use stock_storage::HealthDto;

use provider_tushare::{TushareEtfRepository, TushareStockRepository};
use provider_yahoo::YahooStockRepository;

use crate::server_config::{self, ServerConfig};
use crate::state::AppState;

// ── 错误包装 ──────────────────────────────────────────────────────────────────

struct ApiError(String);

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let msg = serde_json::json!({ "error": self.0 }).to_string();
        (StatusCode::INTERNAL_SERVER_ERROR, msg).into_response()
    }
}

impl From<DomainError> for ApiError {
    fn from(e: DomainError) -> Self { ApiError(e.to_string()) }
}

impl From<anyhow::Error> for ApiError {
    fn from(e: anyhow::Error) -> Self { ApiError(e.to_string()) }
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

#[derive(Deserialize)]
pub struct EtfsQuery {
    pub symbols: Option<String>,
}

#[derive(Deserialize)]
pub struct EtfHistoryQuery {
    pub symbol: String,
    #[serde(default = "default_days")]
    pub days:   usize,
}

#[derive(Deserialize)]
pub struct EtfNavQuery {
    pub symbol: String,
    #[serde(default = "default_days")]
    pub days:   usize,
}

#[derive(Deserialize)]
pub struct EtfBasicQuery {
    pub symbols: Option<String>,
}

#[derive(Deserialize)]
pub struct EtfDailyQuery {
    pub symbol: String,
    #[serde(default = "default_days")]
    pub days:   usize,
}

#[derive(Deserialize)]
pub struct EtfPortfolioQuery {
    pub symbol: String,
    pub period: Option<String>,
}

#[derive(Deserialize)]
pub struct EtfTradeQuery {
    pub symbol: String,
    #[serde(default = "default_days")]
    pub days:   usize,
}

#[derive(Deserialize)]
pub struct EtfDividendQuery {
    pub symbol: String,
}

#[derive(Deserialize)]
pub struct EtfIndexQuery {
    pub index_code: String,
    #[serde(default = "default_days")]
    pub days:       usize,
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

fn default_etf_symbols() -> Vec<String> {
    ["510300.SH", "510500.SH", "159915.SZ", "512010.SH", "159920.SZ"]
        .iter().map(|s| s.to_string()).collect()
}

fn parse_symbols(opt: Option<String>, default: Vec<String>) -> Vec<String> {
    opt.as_deref()
        .filter(|s| !s.is_empty())
        .map(|s| s.split(',').map(|sym| sym.trim().to_string()).filter(|s| !s.is_empty()).collect())
        .unwrap_or(default)
}

// ── /api/config ───────────────────────────────────────────────────────────────

/// GET /api/config → 当前服务器配置
async fn get_config(State(state): State<Arc<AppState>>) -> Json<ServerConfig> {
    Json(state.config.read().unwrap().clone())
}

#[derive(Deserialize)]
struct PutConfigBody {
    data_dir: String,
}

#[derive(Serialize)]
struct ConfigSaveResult {
    ok:      bool,
    message: &'static str,
}

/// PUT /api/config — 保存配置，重启后端后生效
async fn put_config(
    State(state): State<Arc<AppState>>,
    Json(body): Json<PutConfigBody>,
) -> Result<Json<ConfigSaveResult>, ApiError> {
    let new_cfg = ServerConfig { data_dir: body.data_dir };
    server_config::save(&new_cfg)?;
    *state.config.write().unwrap() = new_cfg;
    Ok(Json(ConfigSaveResult { ok: true, message: "已保存，重启后端后生效" }))
}

// ── /api/keys — CRUD ──────────────────────────────────────────────────────────

/// GET /api/keys → [{name, has_value}]
async fn list_keys(State(state): State<Arc<AppState>>) -> Json<Vec<crate::key_store::KeyMeta>> {
    Json(state.keys.list())
}

#[derive(Serialize)]
struct KeyValue { name: String, value: String }

/// GET /api/keys/:name → {name, value}（解密）
async fn get_key(
    State(state): State<Arc<AppState>>,
    Path(name): Path<String>,
) -> Result<Json<KeyValue>, ApiError> {
    let value = state.keys.get(&name)?.unwrap_or_default();
    Ok(Json(KeyValue { name, value }))
}

#[derive(Deserialize)]
struct SetKeyBody { value: String }

/// PUT /api/keys/:name  body: {"value":"..."}
async fn set_key(
    State(state): State<Arc<AppState>>,
    Path(name): Path<String>,
    Json(body): Json<SetKeyBody>,
) -> Result<StatusCode, ApiError> {
    state.keys.set(&name, &body.value)?;
    Ok(StatusCode::NO_CONTENT)
}

/// DELETE /api/keys/:name
async fn delete_key(
    State(state): State<Arc<AppState>>,
    Path(name): Path<String>,
) -> Result<StatusCode, ApiError> {
    state.keys.delete(&name)?;
    Ok(StatusCode::NO_CONTENT)
}

// ── GET /api/stocks ───────────────────────────────────────────────────────────

async fn get_stocks(
    State(state): State<Arc<AppState>>,
    Query(q): Query<StocksQuery>,
    headers: HeaderMap,
) -> Result<impl IntoResponse, ApiError> {
    let tushare_token = state.keys.resolve("tushare_token", header_str(&headers, "x-tushare-token"));
    let yahoo_api_key = state.keys.resolve("yahoo_api_key", header_str(&headers, "x-yahoo-api-key"));
    let symbols = parse_symbols(q.symbols.clone(),
        if q.source == "tushare" { default_tushare_symbols() } else { default_yahoo_symbols() });

    let data = match q.source.as_str() {
        "tushare" => {
            let repo = Arc::new(TushareStockRepository::new(
                state.ashare_store.clone(), state.client.clone(), tushare_token,
            ));
            StockAppService::new(repo).get_quotes(&symbols).await?
        }
        _ => {
            let repo = Arc::new(YahooStockRepository::new(
                state.store.clone(), state.client.clone(), yahoo_api_key,
            ));
            StockAppService::new(repo).get_quotes(&symbols).await?
        }
    };
    Ok(Json(data))
}

// ── GET /api/history ──────────────────────────────────────────────────────────

async fn get_history(
    State(state): State<Arc<AppState>>,
    Query(q): Query<HistoryQuery>,
    headers: HeaderMap,
) -> Result<impl IntoResponse, ApiError> {
    let tushare_token = state.keys.resolve("tushare_token", header_str(&headers, "x-tushare-token"));
    let yahoo_api_key = state.keys.resolve("yahoo_api_key", header_str(&headers, "x-yahoo-api-key"));

    let data = match q.source.as_str() {
        "tushare" => {
            let repo = Arc::new(TushareStockRepository::new(
                state.ashare_store.clone(), state.client.clone(), tushare_token,
            ));
            StockAppService::new(repo).get_ohlc(&q.symbol, q.days).await?
        }
        _ => {
            let repo = Arc::new(YahooStockRepository::new(
                state.store.clone(), state.client.clone(), yahoo_api_key,
            ));
            StockAppService::new(repo).get_ohlc(&q.symbol, q.days).await?
        }
    };
    Ok(Json(data))
}

// ── GET /api/etfs ─────────────────────────────────────────────────────────────

async fn get_etfs(
    State(state): State<Arc<AppState>>,
    Query(q): Query<EtfsQuery>,
    headers: HeaderMap,
) -> Result<impl IntoResponse, ApiError> {
    let token   = state.keys.resolve("tushare_token", header_str(&headers, "x-tushare-token"));
    let symbols = parse_symbols(q.symbols.clone(), default_etf_symbols());
    let repo    = Arc::new(TushareEtfRepository::new(
        state.etf_store.clone(), state.client.clone(), token,
    ));
    Ok(Json(EtfAppService::new(repo).get_quotes(&symbols).await?))
}

// ── GET /api/etf/history ──────────────────────────────────────────────────────

async fn get_etf_history(
    State(state): State<Arc<AppState>>,
    Query(q): Query<EtfHistoryQuery>,
    headers: HeaderMap,
) -> Result<impl IntoResponse, ApiError> {
    let token = state.keys.resolve("tushare_token", header_str(&headers, "x-tushare-token"));
    let repo  = Arc::new(TushareEtfRepository::new(
        state.etf_store.clone(), state.client.clone(), token,
    ));
    Ok(Json(EtfAppService::new(repo).get_history(&q.symbol, q.days).await?))
}

// ── GET /api/etf/nav ──────────────────────────────────────────────────────────

async fn get_etf_nav(
    State(state): State<Arc<AppState>>,
    Query(q): Query<EtfNavQuery>,
    headers: HeaderMap,
) -> Result<impl IntoResponse, ApiError> {
    let token = state.keys.resolve("tushare_token", header_str(&headers, "x-tushare-token"));
    let repo  = Arc::new(TushareEtfRepository::new(
        state.etf_store.clone(), state.client.clone(), token,
    ));
    Ok(Json(EtfAppService::new(repo).get_nav(&q.symbol, q.days).await?))
}

// ── GET /api/etf/basic ───────────────────────────────────────────────────────

async fn get_etf_basic(
    State(state): State<Arc<AppState>>,
    Query(q): Query<EtfBasicQuery>,
    headers: HeaderMap,
) -> Result<impl IntoResponse, ApiError> {
    let token   = state.keys.resolve("tushare_token", header_str(&headers, "x-tushare-token"));
    let symbols = parse_symbols(q.symbols, default_etf_symbols());
    let repo    = Arc::new(TushareEtfRepository::new(
        state.etf_store.clone(), state.client.clone(), token,
    ));
    Ok(Json(EtfAppService::new(repo).get_basic(&symbols).await?))
}

// ── GET /api/etf/daily ───────────────────────────────────────────────────────

async fn get_etf_daily(
    State(state): State<Arc<AppState>>,
    Query(q): Query<EtfDailyQuery>,
    headers: HeaderMap,
) -> Result<impl IntoResponse, ApiError> {
    let token = state.keys.resolve("tushare_token", header_str(&headers, "x-tushare-token"));
    let repo  = Arc::new(TushareEtfRepository::new(
        state.etf_store.clone(), state.client.clone(), token,
    ));
    Ok(Json(EtfAppService::new(repo).get_daily(&q.symbol, q.days).await?))
}

// ── GET /api/etf/portfolio ───────────────────────────────────────────────────

async fn get_etf_portfolio(
    State(state): State<Arc<AppState>>,
    Query(q): Query<EtfPortfolioQuery>,
    headers: HeaderMap,
) -> Result<impl IntoResponse, ApiError> {
    let token = state.keys.resolve("tushare_token", header_str(&headers, "x-tushare-token"));
    let repo  = Arc::new(TushareEtfRepository::new(
        state.etf_store.clone(), state.client.clone(), token,
    ));
    Ok(Json(EtfAppService::new(repo).get_portfolio(&q.symbol, q.period).await?))
}

// ── GET /api/etf/trade ───────────────────────────────────────────────────────

async fn get_etf_trade(
    State(state): State<Arc<AppState>>,
    Query(q): Query<EtfTradeQuery>,
    headers: HeaderMap,
) -> Result<impl IntoResponse, ApiError> {
    let token = state.keys.resolve("tushare_token", header_str(&headers, "x-tushare-token"));
    let repo  = Arc::new(TushareEtfRepository::new(
        state.etf_store.clone(), state.client.clone(), token,
    ));
    Ok(Json(EtfAppService::new(repo).get_share(&q.symbol, q.days).await?))
}

// ── GET /api/etf/dividend ────────────────────────────────────────────────────

async fn get_etf_dividend(
    State(state): State<Arc<AppState>>,
    Query(q): Query<EtfDividendQuery>,
    headers: HeaderMap,
) -> Result<impl IntoResponse, ApiError> {
    let token = state.keys.resolve("tushare_token", header_str(&headers, "x-tushare-token"));
    let repo  = Arc::new(TushareEtfRepository::new(
        state.etf_store.clone(), state.client.clone(), token,
    ));
    Ok(Json(EtfAppService::new(repo).get_dividend(&q.symbol).await?))
}

// ── GET /api/etf/index ───────────────────────────────────────────────────────

async fn get_etf_index(
    State(state): State<Arc<AppState>>,
    Query(q): Query<EtfIndexQuery>,
    headers: HeaderMap,
) -> Result<impl IntoResponse, ApiError> {
    let token = state.keys.resolve("tushare_token", header_str(&headers, "x-tushare-token"));
    let repo  = Arc::new(TushareEtfRepository::new(
        state.etf_store.clone(), state.client.clone(), token,
    ));
    Ok(Json(EtfAppService::new(repo).get_index(&q.index_code, q.days).await?))
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
        // 服务器配置
        .route("/config",      get(get_config).put(put_config))
        // Key management
        .route("/keys",        get(list_keys))
        .route("/keys/:name",  get(get_key).put(set_key).delete(delete_key))
        // Stock & history
        .route("/stocks",      get(get_stocks))
        .route("/history",     get(get_history))
        // ETF 接口
        .route("/etfs",        get(get_etfs))
        .route("/etf/basic",   get(get_etf_basic))
        .route("/etf/daily",   get(get_etf_daily))
        .route("/etf/history", get(get_etf_history))
        .route("/etf/nav",     get(get_etf_nav))
        .route("/etf/portfolio", get(get_etf_portfolio))
        .route("/etf/trade",   get(get_etf_trade))
        .route("/etf/dividend", get(get_etf_dividend))
        .route("/etf/index",   get(get_etf_index))
        .route("/health",      get(health))
}
