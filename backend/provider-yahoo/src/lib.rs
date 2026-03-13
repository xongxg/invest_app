//! Yahoo Finance API 数据提供层（美股、港股、ETF）
//!
//! 基础设施层：实现领域端口 [`StockRepository`]。

use std::sync::Arc;
use std::time::Duration;

use anyhow::{anyhow, Result};
use async_trait::async_trait;
use reqwest::Client;
use serde::Deserialize;

use stock_domain::{DomainError, OHLCBar, Stock, StockRepository};
use stock_storage::ArrowStore;

const STOCK_TTL:   Duration = Duration::from_secs(60 * 5);
const HISTORY_TTL: Duration = Duration::from_secs(60 * 60 * 24);

// ── Yahoo Finance JSON 结构 ──────────────────────────────────────────────────

#[derive(Deserialize)]
struct YahooChartResponse {
    chart: YahooChart,
}

#[derive(Deserialize)]
struct YahooChart {
    result: Option<Vec<YahooResult>>,
    error:  Option<serde_json::Value>,
}

#[derive(Deserialize)]
struct YahooResult {
    meta:       YahooMeta,
    timestamp:  Option<Vec<i64>>,
    indicators: YahooIndicators,
}

#[derive(Deserialize, Default)]
#[serde(rename_all = "camelCase")]
struct YahooMeta {
    regular_market_price:  f64,
    previous_close:        Option<f64>,
    regular_market_volume: Option<u64>,
    market_cap:            Option<f64>,
    long_name:             Option<String>,
}

#[derive(Deserialize)]
struct YahooIndicators {
    quote: Vec<YahooQuote>,
}

#[derive(Deserialize, Default)]
struct YahooQuote {
    open:   Vec<Option<f64>>,
    high:   Vec<Option<f64>>,
    low:    Vec<Option<f64>>,
    close:  Vec<Option<f64>>,
    volume: Vec<Option<u64>>,
}

// ── 符号映射：展示代码 → Yahoo Finance 实际符号 ───────────────────────────────

fn to_yahoo_symbol(symbol: &str) -> &str {
    match symbol {
        "000001.SH" => "000001.SS",  // 上证指数
        "HSI"       => "^HSI",       // 恒生指数
        "HSTECH"    => "^HSTECH",    // 恒生科技
        "NDX"       => "^NDX",       // 纳斯达克100
        other       => other,
    }
}

// ── 工具函数 ─────────────────────────────────────────────────────────────────

fn fmt_market_cap(cap: f64) -> String {
    if cap >= 1e12      { format!("${:.1}T", cap / 1e12) }
    else if cap >= 1e9  { format!("${:.1}B", cap / 1e9) }
    else                { format!("${:.1}M", cap / 1e6) }
}

fn ts_to_date(ts: i64) -> String {
    let days  = ts / 86400;
    let year  = 1970 + days / 365;
    let month = (days % 365) / 30 + 1;
    let day   = (days % 365) % 30 + 1;
    format!("{year}-{month:02}-{day:02}")
}

fn build_request(client: &Client, url: &str, api_key: &str) -> reqwest::RequestBuilder {
    let req = client.get(url);
    if api_key.is_empty() { req } else { req.header("Authorization", format!("Bearer {api_key}")) }
}

async fn fetch_chart(
    client: &Client,
    symbol: &str,
    interval: &str,
    range: &str,
    api_key: &str,
) -> Result<YahooResult> {
    let url = format!(
        "https://query1.finance.yahoo.com/v8/finance/chart/\
         {symbol}?interval={interval}&range={range}"
    );
    let text = build_request(client, &url, api_key)
        .send().await?.text().await?;
    let chart: YahooChartResponse = serde_json::from_str(&text)
        .map_err(|e| anyhow!("JSON parse: {e}"))?;
    if let Some(err) = chart.chart.error {
        return Err(anyhow!("Yahoo error: {err}"));
    }
    chart.chart.result
        .into_iter().flatten().next()
        .ok_or_else(|| anyhow!("no result for {symbol}"))
}

// ── 公开接口函数 ──────────────────────────────────────────────────────────────

/// 获取多只股票的最新行情
pub async fn fetch_stocks(
    client: &Client,
    symbols: &[String],
    api_key: &str,
) -> Result<Vec<Stock>> {
    let mut out = Vec::new();
    for symbol in symbols {
        let yahoo_sym = to_yahoo_symbol(symbol);
        let result = match fetch_chart(client, yahoo_sym, "1d", "5d", api_key).await {
            Ok(r)  => r,
            Err(e) => { tracing::warn!("[yahoo] skip {symbol} ({yahoo_sym}): {e}"); continue; }
        };
        let meta       = result.meta;
        let price      = meta.regular_market_price;
        let prev_close = meta.previous_close.unwrap_or(price);
        let change     = price - prev_close;
        let change_pct = if prev_close != 0.0 { (change / prev_close) * 100.0 } else { 0.0 };

        out.push(Stock {
            symbol:         symbol.clone(),
            name:           meta.long_name.unwrap_or_else(|| symbol.clone()),
            price,
            change,
            change_percent: change_pct,
            volume:         meta.regular_market_volume.unwrap_or(0),
            market_cap:     meta.market_cap.map(fmt_market_cap),
        });
    }
    Ok(out)
}

/// 获取历史 K 线
pub async fn fetch_history(
    client: &Client,
    symbol: &str,
    days: usize,
    api_key: &str,
) -> Result<Vec<OHLCBar>> {
    let range  = if days <= 30 { "1mo" } else if days <= 90 { "3mo" } else { "6mo" };
    let result = fetch_chart(client, symbol, "1d", range, api_key).await?;

    let timestamps = result.timestamp.unwrap_or_default();
    let quotes = result.indicators.quote.into_iter().next()
        .ok_or_else(|| anyhow!("no quote data"))?;

    Ok(timestamps.iter().enumerate().filter_map(|(i, &ts)| {
        let open   = quotes.open  .get(i).copied().flatten()?;
        let high   = quotes.high  .get(i).copied().flatten()?;
        let low    = quotes.low   .get(i).copied().flatten()?;
        let close  = quotes.close .get(i).copied().flatten()?;
        let volume = quotes.volume.get(i).copied().flatten().unwrap_or(0);
        Some(OHLCBar { date: ts_to_date(ts), open, high, low, close, volume })
    }).take(days).collect())
}

// ── 基础设施层：仓储实现 ──────────────────────────────────────────────────────

/// 美股/港股仓储（Yahoo Finance）：缓存优先 + API 回源
pub struct YahooStockRepository {
    store:   Arc<ArrowStore>,
    client:  Client,
    api_key: String,
}

impl YahooStockRepository {
    pub fn new(store: Arc<ArrowStore>, client: Client, api_key: String) -> Self {
        Self { store, client, api_key }
    }
}

#[async_trait]
impl StockRepository for YahooStockRepository {
    async fn get_quotes(&self, symbols: &[String]) -> Result<Vec<Stock>, DomainError> {
        let cache_key = format!("stocks:yahoo:{}", symbols.join(","));

        if let Some(data) = self.store.get_stocks(&cache_key, STOCK_TTL) {
            return Ok(data);
        }

        match fetch_stocks(&self.client, symbols, &self.api_key).await {
            Ok(stocks) => {
                let _ = self.store.put_stocks(&cache_key, &stocks);
                Ok(stocks)
            }
            Err(e) => {
                if let Some(stale) = self.store.get_stocks_stale(&cache_key) {
                    tracing::warn!("yahoo stocks api error, using stale cache: {e}");
                    Ok(stale)
                } else {
                    Err(DomainError::External(e.to_string()))
                }
            }
        }
    }

    async fn get_ohlc(&self, symbol: &str, days: usize) -> Result<Vec<OHLCBar>, DomainError> {
        let cache_key = format!("ohlc:yahoo:{}:{}", symbol, days);

        if let Some(data) = self.store.get_ohlc(&cache_key, HISTORY_TTL) {
            return Ok(data);
        }

        let ohlc = fetch_history(&self.client, symbol, days, &self.api_key)
            .await
            .map_err(|e| DomainError::External(e.to_string()))?;
        let _ = self.store.put_ohlc(&cache_key, &ohlc);
        Ok(ohlc)
    }
}
