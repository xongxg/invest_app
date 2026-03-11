/// 支持的数据源枚举（应用层配置概念）
#[derive(Debug, Clone, PartialEq)]
pub enum DataSource {
    /// 内置模拟数据，无需网络
    Mock,

    /// 聚宽 Tushare Pro（需要 token，支持 A 股）
    TusharePro(TushareConfig),

    /// Yahoo Finance（公开接口，支持美股、港股、ETF）
    YahooFinance(YahooConfig),
}

impl DataSource {
    pub fn display_name(&self) -> &'static str {
        match self {
            DataSource::Mock          => "模拟数据",
            DataSource::TusharePro(_) => "Tushare Pro",
            DataSource::YahooFinance(_) => "Yahoo Finance",
        }
    }

    /// 默认关注的股票代码列表
    pub fn default_symbols(&self) -> Vec<String> {
        match self {
            DataSource::Mock => {
                vec!["AAPL", "GOOGL", "MSFT", "TSLA", "AMZN"]
                    .into_iter().map(String::from).collect()
            }
            DataSource::TusharePro(_) => {
                // A 股代码格式：000001.SZ、600519.SH
                vec!["000001.SZ", "600519.SH", "601318.SH", "000858.SZ", "002594.SZ"]
                    .into_iter().map(String::from).collect()
            }
            DataSource::YahooFinance(cfg) => cfg.symbols.clone(),
        }
    }
}

/// Tushare Pro 配置
#[derive(Debug, Clone, PartialEq)]
pub struct TushareConfig {
    pub token: String,
}

impl TushareConfig {
    pub fn new(token: impl Into<String>) -> Self {
        Self { token: token.into() }
    }
}

/// Yahoo Finance 配置
#[derive(Debug, Clone, PartialEq)]
pub struct YahooConfig {
    /// 关注的股票代码（Yahoo 格式：AAPL、9988.HK 等）
    pub symbols: Vec<String>,
    /// API Key（用于付费版 Yahoo Finance API 或自定义代理，留空则使用公开接口）
    pub api_key: String,
}

impl YahooConfig {
    pub fn new(symbols: Vec<String>, api_key: String) -> Self {
        Self { symbols, api_key }
    }

    pub fn default() -> Self {
        Self {
            symbols: vec!["AAPL", "GOOGL", "MSFT", "TSLA", "AMZN"]
                .into_iter().map(String::from).collect(),
            api_key: String::new(),
        }
    }
}
