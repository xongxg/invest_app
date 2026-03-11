//! Tushare Pro API 客户端（服务端，使用 reqwest）
//!
//! 接口：stock_basic / daily / daily_basic / pro_bar

use std::collections::HashMap;
use anyhow::{anyhow, Result};
use chrono::Local;
use reqwest::Client;
use serde::{Deserialize, Serialize};

use stock_storage::{OHLCDto, StockDto};

const TUSHARE_API: &str = "https://api.tushare.pro";

// ── 请求 / 响应结构 ───────────────────────────────────────────────────────────

#[derive(Serialize)]
struct TushareReq<'a> {
    api_name: &'a str,
    token:    &'a str,
    params:   serde_json::Value,
    fields:   &'a str,
}

#[derive(Deserialize)]
struct TushareResp {
    code: i32,
    msg:  String,
    data: Option<TushareData>,
}

#[derive(Deserialize)]
struct TushareData {
    fields: Vec<String>,
    items:  Vec<Vec<serde_json::Value>>,
}

// ── 通用辅助 ─────────────────────────────────────────────────────────────────

fn idx_map(fields: &[String]) -> HashMap<&str, usize> {
    fields.iter().enumerate().map(|(i, f)| (f.as_str(), i)).collect()
}

fn get_f64(row: &[serde_json::Value], idx: Option<&usize>) -> f64 {
    idx.and_then(|&i| row.get(i)).and_then(|v| v.as_f64()).unwrap_or(0.0)
}

fn get_str(row: &[serde_json::Value], idx: Option<&usize>) -> String {
    idx.and_then(|&i| row.get(i))
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string()
}

fn today() -> String {
    Local::now().format("%Y%m%d").to_string()
}

fn days_ago(n: i64) -> String {
    (Local::now() - chrono::Duration::days(n))
        .format("%Y%m%d")
        .to_string()
}

async fn tushare_call(client: &Client, _token: &str, req: &TushareReq<'_>) -> Result<TushareData> {
    let resp: TushareResp = client
        .post(TUSHARE_API)
        .json(req)
        .send()
        .await?
        .json()
        .await?;

    if resp.code != 0 {
        return Err(anyhow!("[{}] {}", resp.code, resp.msg));
    }
    resp.data.ok_or_else(|| anyhow!("data 字段为空"))
}

fn fmt_market_cap(total_mv_wan: f64) -> String {
    let yi = total_mv_wan / 10_000.0;
    if yi >= 10_000.0 { format!("{:.2}万亿", yi / 10_000.0) }
    else if yi >= 1.0 { format!("{:.2}亿", yi) }
    else              { format!("{:.0}万", total_mv_wan) }
}

// ── stock_basic ───────────────────────────────────────────────────────────────

async fn fetch_stock_basic(
    client: &Client,
    token: &str,
    symbols: &[String],
) -> Result<HashMap<String, (String, String)>> {
    let data = tushare_call(client, token, &TushareReq {
        api_name: "stock_basic",
        token,
        params: serde_json::json!({
            "ts_code": symbols.join(","),
            "list_status": "L"
        }),
        fields: "ts_code,name,industry,market",
    }).await?;

    let idx = idx_map(&data.fields);
    Ok(data.items.iter().map(|row| {
        let ts_code  = get_str(row, idx.get("ts_code"));
        let name     = get_str(row, idx.get("name"));
        let industry = get_str(row, idx.get("industry"));
        (ts_code, (name, industry))
    }).collect())
}

// ── daily ─────────────────────────────────────────────────────────────────────

struct DailyRow {
    #[allow(dead_code)]
    ts_code: String, trade_date: String,
    close: f64, change: f64, pct_chg: f64, vol: f64,
}

async fn fetch_latest_daily(
    client: &Client,
    token: &str,
    symbols: &[String],
) -> Result<HashMap<String, DailyRow>> {
    let data = tushare_call(client, token, &TushareReq {
        api_name: "daily",
        token,
        params: serde_json::json!({
            "ts_code":    symbols.join(","),
            "start_date": days_ago(10),
            "end_date":   today(),
        }),
        fields: "ts_code,trade_date,open,high,low,close,pre_close,change,pct_chg,vol,amount",
    }).await?;

    let idx = idx_map(&data.fields);
    let mut map: HashMap<String, DailyRow> = HashMap::new();

    for row in &data.items {
        let ts_code    = get_str(row, idx.get("ts_code"));
        let trade_date = get_str(row, idx.get("trade_date"));
        let is_newer   = map.get(&ts_code)
            .map(|e: &DailyRow| trade_date > e.trade_date)
            .unwrap_or(true);
        if is_newer {
            map.insert(ts_code.clone(), DailyRow {
                ts_code, trade_date,
                close:   get_f64(row, idx.get("close")),
                change:  get_f64(row, idx.get("change")),
                pct_chg: get_f64(row, idx.get("pct_chg")),
                vol:     get_f64(row, idx.get("vol")),
            });
        }
    }
    Ok(map)
}

// ── daily_basic ───────────────────────────────────────────────────────────────

async fn fetch_daily_basic(
    client: &Client,
    token: &str,
    symbols: &[String],
    trade_date: &str,
) -> Result<HashMap<String, f64>> {
    let data = tushare_call(client, token, &TushareReq {
        api_name: "daily_basic",
        token,
        params: serde_json::json!({
            "ts_code":    symbols.join(","),
            "trade_date": trade_date,
        }),
        fields: "ts_code,total_mv",
    }).await?;

    let idx = idx_map(&data.fields);
    Ok(data.items.iter().map(|row| {
        let ts_code  = get_str(row, idx.get("ts_code"));
        let total_mv = get_f64(row, idx.get("total_mv"));
        (ts_code, total_mv)
    }).collect())
}

// ── Public API ────────────────────────────────────────────────────────────────

/// 获取所有关注股票的最新行情
pub async fn fetch_stocks(
    client: &Client,
    token: &str,
    symbols: &[String],
) -> Result<Vec<StockDto>> {
    let basic_map = fetch_stock_basic(client, token, symbols).await?;
    let daily_map = fetch_latest_daily(client, token, symbols).await?;

    if daily_map.is_empty() {
        return Err(anyhow!("最近 10 天无交易数据，请确认股票代码格式（如 000001.SZ）"));
    }

    let latest_date = daily_map.values()
        .map(|r| r.trade_date.as_str())
        .max()
        .unwrap_or("")
        .to_string();

    let mv_map = fetch_daily_basic(client, token, symbols, &latest_date)
        .await
        .unwrap_or_default();

    Ok(symbols.iter().filter_map(|sym| {
        let daily = daily_map.get(sym)?;
        let (raw_name, _) = basic_map.get(sym).cloned()
            .unwrap_or_else(|| (String::new(), String::new()));
        let name = if raw_name.is_empty() { sym.clone() } else { raw_name };
        let market_cap = mv_map.get(sym)
            .filter(|&&mv| mv > 0.0)
            .map(|&mv| fmt_market_cap(mv));
        Some(StockDto {
            symbol:         sym.clone(),
            name,
            price:          daily.close,
            change:         daily.change,
            change_percent: daily.pct_chg,
            volume:         (daily.vol * 100.0) as u64,
            market_cap,
        })
    }).collect())
}

/// 获取历史 K 线（via daily，基础积分即可访问）
pub async fn fetch_history(
    client: &Client,
    token: &str,
    symbol: &str,
    days: usize,
) -> Result<Vec<OHLCDto>> {
    let start = days_ago((days as i64) + 15);
    let end   = today();

    let data = tushare_call(client, token, &TushareReq {
        api_name: "daily",
        token,
        params: serde_json::json!({
            "ts_code":    symbol,
            "start_date": start,
            "end_date":   end,
        }),
        fields: "ts_code,trade_date,open,high,low,close,vol",
    }).await?;

    let idx = idx_map(&data.fields);
    let mut rows: Vec<OHLCDto> = data.items.iter().map(|row| {
        let raw  = get_str(row, idx.get("trade_date"));
        let date = if raw.len() == 8 {
            format!("{}-{}-{}", &raw[..4], &raw[4..6], &raw[6..])
        } else { raw };
        OHLCDto {
            date,
            open:   get_f64(row, idx.get("open")),
            high:   get_f64(row, idx.get("high")),
            low:    get_f64(row, idx.get("low")),
            close:  get_f64(row, idx.get("close")),
            volume: (get_f64(row, idx.get("vol")) * 100.0) as u64,
        }
    }).collect();

    // daily API 返回最新在前，reverse 后截取最近 days 条
    rows.sort_by(|a, b| a.date.cmp(&b.date));
    rows.truncate(days);
    Ok(rows)
}
