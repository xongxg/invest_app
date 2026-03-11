use serde::{Deserialize, Serialize};

/// 股票行情 DTO（与前端 domain::Stock 的 JSON 字段完全对齐）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StockDto {
    pub symbol:         String,
    pub name:           String,
    pub price:          f64,
    pub change:         f64,
    pub change_percent: f64,
    pub volume:         u64,
    pub market_cap:     Option<String>,
}

/// OHLCV K 线 DTO（与前端 domain::OHLCData 字段对齐）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OHLCDto {
    pub date:   String,
    pub open:   f64,
    pub high:   f64,
    pub low:    f64,
    pub close:  f64,
    pub volume: u64,
}

/// `/api/health` 响应
#[derive(Serialize)]
pub struct HealthDto {
    pub status:      &'static str,
    pub cached_keys: usize,
}
