use serde::{Deserialize, Serialize};

/// 股票 / ETF 行情快照
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Stock {
    pub symbol:         String,
    pub name:           String,
    pub price:          f64,
    pub change:         f64,
    pub change_percent: f64,
    pub volume:         u64,
    pub market_cap:     Option<String>,
}

/// OHLCV K 线
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OHLCBar {
    pub date:   String,
    pub open:   f64,
    pub high:   f64,
    pub low:    f64,
    pub close:  f64,
    pub volume: u64,
}

/// ETF 基本信息（fund_basic）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EtfBasic {
    pub ts_code:      String,
    pub name:         String,
    pub management:   String,
    pub trustee:      String,
    pub fund_type:    String,
    pub found_date:   String,
    pub list_date:    String,
    pub issue_date:   String,
    pub delist_date:  String,
    pub issue_amount: f64,
    pub benchmark:    String,
    pub status:       String,
    pub invest_type:  String,
    pub market:       String,
}

/// ETF 详细日线（fund_daily，含 pre_close / pct_chg / amount）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EtfDaily {
    pub trade_date: String,
    pub open:       f64,
    pub high:       f64,
    pub low:        f64,
    pub close:      f64,
    pub pre_close:  f64,
    pub change:     f64,
    pub pct_chg:    f64,
    pub vol:        f64,
    pub amount:     f64,
}

/// ETF 持仓（fund_portfolio）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EtfPortfolio {
    pub ann_date:        String,
    pub end_date:        String,
    pub symbol:          String,
    pub mkv:             f64,
    pub amount:          f64,
    pub stk_mkv_ratio:   f64,
    pub stk_float_ratio: f64,
}

/// ETF 份额申赎（fund_share）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EtfShare {
    pub trade_date:   String,
    pub fd_share:     f64,
    pub fd_net_share: f64,
}

/// ETF 分红（fund_div）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EtfDividend {
    pub ann_date:    String,
    pub imp_anndate: String,
    pub base_date:   String,
    pub div_proc:    String,
    pub base_unit:   f64,
    pub cash_div:    f64,
    pub ex_date:     String,
    pub pay_date:    String,
}

/// ETF 跟踪指数日线（index_daily）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EtfIndex {
    pub trade_date: String,
    pub open:       f64,
    pub high:       f64,
    pub low:        f64,
    pub close:      f64,
    pub pre_close:  f64,
    pub change:     f64,
    pub pct_chg:    f64,
    pub vol:        f64,
    pub amount:     f64,
}

/// 基金净值（fund_nav）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FundNav {
    pub nav_date:  String,
    pub unit_nav:  f64,
    pub accum_nav: f64,
    pub adj_nav:   f64,
}
