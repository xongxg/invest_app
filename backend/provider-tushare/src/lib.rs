//! Tushare Pro API 数据提供层
//!
//! 支持接口：
//! - A股：stock_basic / daily / daily_basic
//! - ETF：fund_basic / fund_daily / fund_nav / fund_portfolio / fund_share / fund_div
//! - 指数：index_daily
//!
//! 基础设施层：实现领域端口 [`StockRepository`] 和 [`EtfRepository`]。

use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use std::time::Duration;

use anyhow::{anyhow, Result};
use async_trait::async_trait;
use chrono::{Datelike, Local};
use reqwest::Client;
use serde::{Deserialize, Serialize};

use stock_domain::{
    DomainError, EtfBasic, EtfDaily, EtfDividend, EtfIndex, EtfPortfolio, EtfRepository,
    EtfShare, FundNav, OHLCBar, Stock, StockRepository,
};
use stock_storage::ArrowStore;

const TUSHARE_API: &str = "https://api.tushare.pro";
const STOCK_TTL:   Duration = Duration::from_secs(60 * 5);
const HISTORY_TTL: Duration = Duration::from_secs(60 * 60 * 24);

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

/// 将 `YYYYMMDD` 格式转为 `YYYY-MM-DD`，其他格式原样返回
fn fmt_date(raw: String) -> String {
    if raw.len() == 8 {
        format!("{}-{}-{}", &raw[..4], &raw[4..6], &raw[6..])
    } else {
        raw
    }
}

/// 上一个季度末日期（fund_portfolio 默认 period）
fn last_quarter_end() -> String {
    let now = Local::now();
    let (y, m) = (now.year(), now.month());
    let (qy, qm) = match m {
        1..=3  => (y - 1, 12),
        4..=6  => (y,      3),
        7..=9  => (y,      6),
        _      => (y,      9),
    };
    let qd = if qm == 3 || qm == 9 { 30 } else { 31 };
    format!("{qy}{qm:02}{qd:02}")
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
    else if yi >= 1.0  { format!("{:.2}亿", yi) }
    else               { format!("{:.0}万", total_mv_wan) }
}

// ── A股内部辅助 ───────────────────────────────────────────────────────────────

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

struct DailyRow {
    #[allow(dead_code)]
    ts_code:    String,
    trade_date: String,
    close:      f64,
    change:     f64,
    pct_chg:    f64,
    vol:        f64,
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

// ── ETF 内部辅助 ──────────────────────────────────────────────────────────────

async fn fetch_fund_basic_map(
    client: &Client,
    token: &str,
    symbols: &[String],
) -> Result<HashMap<String, String>> {
    let data = tushare_call(client, token, &TushareReq {
        api_name: "fund_basic",
        token,
        params: serde_json::json!({ "market": "E", "status": "L" }),
        fields: "ts_code,name,fund_type",
    }).await?;

    let idx = idx_map(&data.fields);
    let wanted: HashSet<&str> = symbols.iter().map(|s| s.as_str()).collect();
    Ok(data.items.iter().filter_map(|row| {
        let ts_code = get_str(row, idx.get("ts_code"));
        if !wanted.contains(ts_code.as_str()) { return None; }
        let name = get_str(row, idx.get("name"));
        Some((ts_code, name))
    }).collect())
}

async fn fetch_fund_daily_latest(
    client: &Client,
    token: &str,
    symbols: &[String],
) -> Result<HashMap<String, DailyRow>> {
    let data = tushare_call(client, token, &TushareReq {
        api_name: "fund_daily",
        token,
        params: serde_json::json!({
            "ts_code":    symbols.join(","),
            "start_date": days_ago(10),
            "end_date":   today(),
        }),
        fields: "ts_code,trade_date,close,change,pct_chg,vol",
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

// ── A股公开接口函数 ────────────────────────────────────────────────────────────

/// 获取 A 股最新行情（stock_basic + daily + daily_basic）
pub async fn fetch_stocks(
    client: &Client,
    token: &str,
    symbols: &[String],
) -> Result<Vec<Stock>> {
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
        Some(Stock {
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

/// 获取 A 股历史 K 线（daily）
pub async fn fetch_history(
    client: &Client,
    token: &str,
    symbol: &str,
    days: usize,
) -> Result<Vec<OHLCBar>> {
    let data = tushare_call(client, token, &TushareReq {
        api_name: "daily",
        token,
        params: serde_json::json!({
            "ts_code":    symbol,
            "start_date": days_ago((days as i64) + 15),
            "end_date":   today(),
        }),
        fields: "ts_code,trade_date,open,high,low,close,vol",
    }).await?;

    let idx = idx_map(&data.fields);
    let mut rows: Vec<OHLCBar> = data.items.iter().map(|row| OHLCBar {
        date:   fmt_date(get_str(row, idx.get("trade_date"))),
        open:   get_f64(row, idx.get("open")),
        high:   get_f64(row, idx.get("high")),
        low:    get_f64(row, idx.get("low")),
        close:  get_f64(row, idx.get("close")),
        volume: (get_f64(row, idx.get("vol")) * 100.0) as u64,
    }).collect();

    rows.sort_by(|a, b| a.date.cmp(&b.date));
    rows.truncate(days);
    Ok(rows)
}

// ── ETF 公开接口函数 ───────────────────────────────────────────────────────────

/// ETF 最新行情列表（fund_basic + fund_daily）
pub async fn fetch_etfs(
    client: &Client,
    token: &str,
    symbols: &[String],
) -> Result<Vec<Stock>> {
    let basic_map: HashMap<String, String> =
        fetch_fund_basic_map(client, token, symbols).await.unwrap_or_default();
    let daily_map: HashMap<String, DailyRow> =
        fetch_fund_daily_latest(client, token, symbols).await?;

    if daily_map.is_empty() {
        return Err(anyhow!("最近 10 天无 ETF 交易数据，请确认代码格式（如 510300.SH）"));
    }

    Ok(symbols.iter().filter_map(|sym| {
        let daily = daily_map.get(sym)?;
        let name = basic_map.get(sym)
            .filter(|n| !n.is_empty())
            .cloned()
            .unwrap_or_else(|| sym.clone());
        Some(Stock {
            symbol:         sym.clone(),
            name,
            price:          daily.close,
            change:         daily.change,
            change_percent: daily.pct_chg,
            volume:         (daily.vol * 100.0) as u64,
            market_cap:     None,
        })
    }).collect())
}

/// ETF 基本信息（fund_basic）
pub async fn fetch_etf_basic(
    client: &Client,
    token: &str,
    symbols: &[String],
) -> Result<Vec<EtfBasic>> {
    let data = tushare_call(client, token, &TushareReq {
        api_name: "fund_basic",
        token,
        params: serde_json::json!({ "market": "E", "status": "L" }),
        fields: "ts_code,name,management,trustee,fund_type,found_date,\
                 list_date,issue_date,delist_date,issue_amount,\
                 benchmark,status,invest_type,market",
    }).await?;

    let idx = idx_map(&data.fields);
    let wanted: HashSet<&str> = symbols.iter().map(|s| s.as_str()).collect();

    Ok(data.items.iter().filter_map(|row| {
        let ts_code = get_str(row, idx.get("ts_code"));
        if !symbols.is_empty() && !wanted.contains(ts_code.as_str()) {
            return None;
        }
        Some(EtfBasic {
            ts_code,
            name:         get_str(row, idx.get("name")),
            management:   get_str(row, idx.get("management")),
            trustee:      get_str(row, idx.get("trustee")),
            fund_type:    get_str(row, idx.get("fund_type")),
            found_date:   fmt_date(get_str(row, idx.get("found_date"))),
            list_date:    fmt_date(get_str(row, idx.get("list_date"))),
            issue_date:   fmt_date(get_str(row, idx.get("issue_date"))),
            delist_date:  fmt_date(get_str(row, idx.get("delist_date"))),
            issue_amount: get_f64(row, idx.get("issue_amount")),
            benchmark:    get_str(row, idx.get("benchmark")),
            status:       get_str(row, idx.get("status")),
            invest_type:  get_str(row, idx.get("invest_type")),
            market:       get_str(row, idx.get("market")),
        })
    }).collect())
}

/// ETF 详细日线（fund_daily，含 pre_close/pct_chg/amount）
pub async fn fetch_etf_daily_detail(
    client: &Client,
    token: &str,
    symbol: &str,
    days: usize,
) -> Result<Vec<EtfDaily>> {
    let data = tushare_call(client, token, &TushareReq {
        api_name: "fund_daily",
        token,
        params: serde_json::json!({
            "ts_code":    symbol,
            "start_date": days_ago((days as i64) + 15),
            "end_date":   today(),
        }),
        fields: "ts_code,trade_date,open,high,low,close,pre_close,change,pct_chg,vol,amount",
    }).await?;

    let idx = idx_map(&data.fields);
    let mut rows: Vec<EtfDaily> = data.items.iter().map(|row| EtfDaily {
        trade_date: fmt_date(get_str(row, idx.get("trade_date"))),
        open:       get_f64(row, idx.get("open")),
        high:       get_f64(row, idx.get("high")),
        low:        get_f64(row, idx.get("low")),
        close:      get_f64(row, idx.get("close")),
        pre_close:  get_f64(row, idx.get("pre_close")),
        change:     get_f64(row, idx.get("change")),
        pct_chg:    get_f64(row, idx.get("pct_chg")),
        vol:        get_f64(row, idx.get("vol")),
        amount:     get_f64(row, idx.get("amount")),
    }).collect();

    rows.sort_by(|a, b| a.trade_date.cmp(&b.trade_date));
    rows.truncate(days);
    Ok(rows)
}

/// ETF 历史 K 线（fund_daily → OHLCBar）
pub async fn fetch_etf_history(
    client: &Client,
    token: &str,
    symbol: &str,
    days: usize,
) -> Result<Vec<OHLCBar>> {
    let data = tushare_call(client, token, &TushareReq {
        api_name: "fund_daily",
        token,
        params: serde_json::json!({
            "ts_code":    symbol,
            "start_date": days_ago((days as i64) + 15),
            "end_date":   today(),
        }),
        fields: "ts_code,trade_date,open,high,low,close,vol",
    }).await?;

    let idx = idx_map(&data.fields);
    let mut rows: Vec<OHLCBar> = data.items.iter().map(|row| OHLCBar {
        date:   fmt_date(get_str(row, idx.get("trade_date"))),
        open:   get_f64(row, idx.get("open")),
        high:   get_f64(row, idx.get("high")),
        low:    get_f64(row, idx.get("low")),
        close:  get_f64(row, idx.get("close")),
        volume: (get_f64(row, idx.get("vol")) * 100.0) as u64,
    }).collect();

    rows.sort_by(|a, b| a.date.cmp(&b.date));
    rows.truncate(days);
    Ok(rows)
}

/// ETF 净值（fund_nav）
pub async fn fetch_fund_nav(
    client: &Client,
    token: &str,
    symbol: &str,
    days: usize,
) -> Result<Vec<FundNav>> {
    let data = tushare_call(client, token, &TushareReq {
        api_name: "fund_nav",
        token,
        params: serde_json::json!({
            "ts_code":    symbol,
            "start_date": days_ago((days as i64) + 15),
            "end_date":   today(),
        }),
        fields: "ts_code,nav_date,unit_nav,accum_nav,adj_nav",
    }).await?;

    let idx = idx_map(&data.fields);
    let mut rows: Vec<FundNav> = data.items.iter().filter_map(|row| {
        let raw = get_str(row, idx.get("nav_date"));
        if raw.is_empty() { return None; }
        Some(FundNav {
            nav_date:  fmt_date(raw),
            unit_nav:  get_f64(row, idx.get("unit_nav")),
            accum_nav: get_f64(row, idx.get("accum_nav")),
            adj_nav:   get_f64(row, idx.get("adj_nav")),
        })
    }).collect();

    rows.sort_by(|a, b| a.nav_date.cmp(&b.nav_date));
    rows.truncate(days);
    Ok(rows)
}

/// ETF 持仓明细（fund_portfolio）
pub async fn fetch_etf_portfolio(
    client: &Client,
    token: &str,
    symbol: &str,
    period: Option<&str>,
) -> Result<Vec<EtfPortfolio>> {
    let period = period.map(|s| s.to_string()).unwrap_or_else(last_quarter_end);

    let data = tushare_call(client, token, &TushareReq {
        api_name: "fund_portfolio",
        token,
        params: serde_json::json!({ "ts_code": symbol, "period": period }),
        fields: "ts_code,ann_date,end_date,symbol,mkv,amount,stk_mkv_ratio,stk_float_ratio",
    }).await?;

    let idx = idx_map(&data.fields);
    Ok(data.items.iter().map(|row| EtfPortfolio {
        ann_date:        fmt_date(get_str(row, idx.get("ann_date"))),
        end_date:        fmt_date(get_str(row, idx.get("end_date"))),
        symbol:          get_str(row, idx.get("symbol")),
        mkv:             get_f64(row, idx.get("mkv")),
        amount:          get_f64(row, idx.get("amount")),
        stk_mkv_ratio:   get_f64(row, idx.get("stk_mkv_ratio")),
        stk_float_ratio: get_f64(row, idx.get("stk_float_ratio")),
    }).collect())
}

/// ETF 份额申赎（fund_share）
pub async fn fetch_etf_share(
    client: &Client,
    token: &str,
    symbol: &str,
    days: usize,
) -> Result<Vec<EtfShare>> {
    let data = tushare_call(client, token, &TushareReq {
        api_name: "fund_share",
        token,
        params: serde_json::json!({
            "ts_code":    symbol,
            "start_date": days_ago((days as i64) + 15),
            "end_date":   today(),
        }),
        fields: "ts_code,trade_date,fd_share,fd_net_share",
    }).await?;

    let idx = idx_map(&data.fields);
    let mut rows: Vec<EtfShare> = data.items.iter().map(|row| EtfShare {
        trade_date:   fmt_date(get_str(row, idx.get("trade_date"))),
        fd_share:     get_f64(row, idx.get("fd_share")),
        fd_net_share: get_f64(row, idx.get("fd_net_share")),
    }).collect();

    rows.sort_by(|a, b| a.trade_date.cmp(&b.trade_date));
    rows.truncate(days);
    Ok(rows)
}

/// ETF 分红（fund_div）
pub async fn fetch_etf_dividend(
    client: &Client,
    token: &str,
    symbol: &str,
) -> Result<Vec<EtfDividend>> {
    let data = tushare_call(client, token, &TushareReq {
        api_name: "fund_div",
        token,
        params: serde_json::json!({ "ts_code": symbol }),
        fields: "ts_code,ann_date,imp_anndate,base_date,div_proc,base_unit,cash_div,ex_date,pay_date",
    }).await?;

    let idx = idx_map(&data.fields);
    let mut rows: Vec<EtfDividend> = data.items.iter().map(|row| EtfDividend {
        ann_date:    fmt_date(get_str(row, idx.get("ann_date"))),
        imp_anndate: fmt_date(get_str(row, idx.get("imp_anndate"))),
        base_date:   fmt_date(get_str(row, idx.get("base_date"))),
        div_proc:    get_str(row, idx.get("div_proc")),
        base_unit:   get_f64(row, idx.get("base_unit")),
        cash_div:    get_f64(row, idx.get("cash_div")),
        ex_date:     fmt_date(get_str(row, idx.get("ex_date"))),
        pay_date:    fmt_date(get_str(row, idx.get("pay_date"))),
    }).collect();

    rows.sort_by(|a, b| b.ann_date.cmp(&a.ann_date));
    Ok(rows)
}

/// ETF 跟踪指数日线（index_daily）
pub async fn fetch_etf_index(
    client: &Client,
    token: &str,
    index_code: &str,
    days: usize,
) -> Result<Vec<EtfIndex>> {
    let data = tushare_call(client, token, &TushareReq {
        api_name: "index_daily",
        token,
        params: serde_json::json!({
            "ts_code":    index_code,
            "start_date": days_ago((days as i64) + 15),
            "end_date":   today(),
        }),
        fields: "ts_code,trade_date,open,high,low,close,pre_close,change,pct_chg,vol,amount",
    }).await?;

    let idx = idx_map(&data.fields);
    let mut rows: Vec<EtfIndex> = data.items.iter().map(|row| EtfIndex {
        trade_date: fmt_date(get_str(row, idx.get("trade_date"))),
        open:       get_f64(row, idx.get("open")),
        high:       get_f64(row, idx.get("high")),
        low:        get_f64(row, idx.get("low")),
        close:      get_f64(row, idx.get("close")),
        pre_close:  get_f64(row, idx.get("pre_close")),
        change:     get_f64(row, idx.get("change")),
        pct_chg:    get_f64(row, idx.get("pct_chg")),
        vol:        get_f64(row, idx.get("vol")),
        amount:     get_f64(row, idx.get("amount")),
    }).collect();

    rows.sort_by(|a, b| a.trade_date.cmp(&b.trade_date));
    rows.truncate(days);
    Ok(rows)
}

// ── 基础设施层：仓储实现 ──────────────────────────────────────────────────────

/// A 股仓储（Tushare）：缓存优先 + API 回源
pub struct TushareStockRepository {
    store:  Arc<ArrowStore>,
    client: Client,
    token:  String,
}

impl TushareStockRepository {
    pub fn new(store: Arc<ArrowStore>, client: Client, token: String) -> Self {
        Self { store, client, token }
    }
}

#[async_trait]
impl StockRepository for TushareStockRepository {
    async fn get_quotes(&self, symbols: &[String]) -> Result<Vec<Stock>, DomainError> {
        let cache_key = format!("stocks:tushare:{}", symbols.join(","));

        if let Some(data) = self.store.get_stocks(&cache_key, STOCK_TTL) {
            return Ok(data);
        }

        match fetch_stocks(&self.client, &self.token, symbols).await {
            Ok(stocks) => {
                let _ = self.store.put_stocks(&cache_key, &stocks);
                Ok(stocks)
            }
            Err(e) => {
                if let Some(stale) = self.store.get_stocks_stale(&cache_key) {
                    tracing::warn!("tushare stocks api error, using stale cache: {e}");
                    Ok(stale)
                } else {
                    Err(DomainError::External(e.to_string()))
                }
            }
        }
    }

    async fn get_ohlc(&self, symbol: &str, days: usize) -> Result<Vec<OHLCBar>, DomainError> {
        let cache_key = format!("ohlc:tushare:{}:{}", symbol, days);

        if let Some(data) = self.store.get_ohlc(&cache_key, HISTORY_TTL) {
            return Ok(data);
        }

        let ohlc = fetch_history(&self.client, &self.token, symbol, days)
            .await
            .map_err(|e| DomainError::External(e.to_string()))?;
        let _ = self.store.put_ohlc(&cache_key, &ohlc);
        Ok(ohlc)
    }
}

/// ETF 仓储（Tushare）：缓存优先 + API 回源
pub struct TushareEtfRepository {
    store:  Arc<ArrowStore>,
    client: Client,
    token:  String,
}

impl TushareEtfRepository {
    pub fn new(store: Arc<ArrowStore>, client: Client, token: String) -> Self {
        Self { store, client, token }
    }
}

#[async_trait]
impl EtfRepository for TushareEtfRepository {
    async fn get_quotes(&self, symbols: &[String]) -> Result<Vec<Stock>, DomainError> {
        let cache_key = format!("stocks:etf:{}", symbols.join(","));

        if let Some(data) = self.store.get_stocks(&cache_key, STOCK_TTL) {
            return Ok(data);
        }

        match fetch_etfs(&self.client, &self.token, symbols).await {
            Ok(etfs) => {
                let _ = self.store.put_stocks(&cache_key, &etfs);
                Ok(etfs)
            }
            Err(e) => {
                if let Some(stale) = self.store.get_stocks_stale(&cache_key) {
                    tracing::warn!("tushare etfs api error, using stale cache: {e}");
                    Ok(stale)
                } else {
                    Err(DomainError::External(e.to_string()))
                }
            }
        }
    }

    async fn get_basic(&self, symbols: &[String]) -> Result<Vec<EtfBasic>, DomainError> {
        let cache_key = format!("etf:basic:{}", symbols.join(","));

        if let Some(data) = self.store.get_extra(&cache_key, HISTORY_TTL) {
            return Ok(data);
        }

        let data = fetch_etf_basic(&self.client, &self.token, symbols)
            .await
            .map_err(|e| DomainError::External(e.to_string()))?;
        let _ = self.store.put_extra(&cache_key, &data);
        Ok(data)
    }

    async fn get_daily(&self, symbol: &str, days: usize) -> Result<Vec<EtfDaily>, DomainError> {
        let cache_key = format!("etf:daily:{}:{}", symbol, days);

        if let Some(data) = self.store.get_extra(&cache_key, HISTORY_TTL) {
            return Ok(data);
        }

        let data = fetch_etf_daily_detail(&self.client, &self.token, symbol, days)
            .await
            .map_err(|e| DomainError::External(e.to_string()))?;
        let _ = self.store.put_extra(&cache_key, &data);
        Ok(data)
    }

    async fn get_history(&self, symbol: &str, days: usize) -> Result<Vec<OHLCBar>, DomainError> {
        let cache_key = format!("ohlc:etf:{}:{}", symbol, days);

        if let Some(data) = self.store.get_ohlc(&cache_key, HISTORY_TTL) {
            return Ok(data);
        }

        let ohlc = fetch_etf_history(&self.client, &self.token, symbol, days)
            .await
            .map_err(|e| DomainError::External(e.to_string()))?;
        let _ = self.store.put_ohlc(&cache_key, &ohlc);
        Ok(ohlc)
    }

    async fn get_nav(&self, symbol: &str, days: usize) -> Result<Vec<FundNav>, DomainError> {
        let cache_key = format!("nav:{}:{}", symbol, days);

        if let Some(data) = self.store.get_fund_nav(&cache_key, HISTORY_TTL) {
            return Ok(data);
        }

        let data = fetch_fund_nav(&self.client, &self.token, symbol, days)
            .await
            .map_err(|e| DomainError::External(e.to_string()))?;
        let _ = self.store.put_fund_nav(&cache_key, &data);
        Ok(data)
    }

    async fn get_portfolio(
        &self,
        symbol: &str,
        period: Option<String>,
    ) -> Result<Vec<EtfPortfolio>, DomainError> {
        let period_str = period.as_deref().unwrap_or("latest");
        let cache_key  = format!("etf:portfolio:{}:{}", symbol, period_str);

        if let Some(data) = self.store.get_extra(&cache_key, HISTORY_TTL) {
            return Ok(data);
        }

        let data = fetch_etf_portfolio(&self.client, &self.token, symbol, period.as_deref())
            .await
            .map_err(|e| DomainError::External(e.to_string()))?;
        let _ = self.store.put_extra(&cache_key, &data);
        Ok(data)
    }

    async fn get_share(&self, symbol: &str, days: usize) -> Result<Vec<EtfShare>, DomainError> {
        let cache_key = format!("etf:trade:{}:{}", symbol, days);

        if let Some(data) = self.store.get_extra(&cache_key, STOCK_TTL) {
            return Ok(data);
        }

        let data = fetch_etf_share(&self.client, &self.token, symbol, days)
            .await
            .map_err(|e| DomainError::External(e.to_string()))?;
        let _ = self.store.put_extra(&cache_key, &data);
        Ok(data)
    }

    async fn get_dividend(&self, symbol: &str) -> Result<Vec<EtfDividend>, DomainError> {
        let cache_key = format!("etf:dividend:{}", symbol);

        if let Some(data) = self.store.get_extra(&cache_key, HISTORY_TTL) {
            return Ok(data);
        }

        let data = fetch_etf_dividend(&self.client, &self.token, symbol)
            .await
            .map_err(|e| DomainError::External(e.to_string()))?;
        let _ = self.store.put_extra(&cache_key, &data);
        Ok(data)
    }

    async fn get_index(
        &self,
        index_code: &str,
        days: usize,
    ) -> Result<Vec<EtfIndex>, DomainError> {
        let cache_key = format!("etf:index:{}:{}", index_code, days);

        if let Some(data) = self.store.get_extra(&cache_key, HISTORY_TTL) {
            return Ok(data);
        }

        let data = fetch_etf_index(&self.client, &self.token, index_code, days)
            .await
            .map_err(|e| DomainError::External(e.to_string()))?;
        let _ = self.store.put_extra(&cache_key, &data);
        Ok(data)
    }
}
