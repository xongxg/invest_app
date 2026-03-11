use serde::{Deserialize, Serialize};

/// 股票实体（领域核心对象）
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Stock {
    pub symbol: String,
    pub name: String,
    pub price: f64,
    pub change: f64,
    pub change_percent: f64,
    pub volume: u64,
    pub market_cap: Option<String>,
}

impl Stock {
    pub fn new(
        symbol: impl Into<String>,
        name: impl Into<String>,
        price: f64,
        change: f64,
        volume: u64,
        market_cap: impl Into<String>,
    ) -> Self {
        let change_percent = (change / (price - change)) * 100.0;
        Self {
            symbol: symbol.into(),
            name: name.into(),
            price,
            change,
            change_percent,
            volume,
            market_cap: Some(market_cap.into()),
        }
    }

    pub fn is_positive(&self) -> bool {
        self.change >= 0.0
    }

    pub fn format_volume(&self) -> String {
        match self.volume {
            v if v >= 1_000_000_000 => format!("{:.1}B", v as f64 / 1_000_000_000.0),
            v if v >= 1_000_000 => format!("{:.1}M", v as f64 / 1_000_000.0),
            v if v >= 1_000 => format!("{:.1}K", v as f64 / 1_000.0),
            v => format!("{}", v),
        }
    }
}

/// OHLC（开高低收）K线数据值对象
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OHLCData {
    pub date: String,
    pub open: f64,
    pub high: f64,
    pub low: f64,
    pub close: f64,
    pub volume: u64,
}
