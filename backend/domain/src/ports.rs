use async_trait::async_trait;

use crate::entities::{
    EtfBasic, EtfDaily, EtfDividend, EtfIndex, EtfPortfolio, EtfShare,
    FundNav, OHLCBar, Stock,
};
use crate::errors::DomainError;

/// 股票数据仓储端口（A股 / 美股 / 港股）
#[async_trait]
pub trait StockRepository: Send + Sync {
    /// 获取多只股票的最新行情快照
    async fn get_quotes(&self, symbols: &[String]) -> Result<Vec<Stock>, DomainError>;

    /// 获取单只股票的历史 K 线
    async fn get_ohlc(&self, symbol: &str, days: usize) -> Result<Vec<OHLCBar>, DomainError>;
}

/// ETF 数据仓储端口（Tushare A股场内 ETF）
#[async_trait]
pub trait EtfRepository: Send + Sync {
    /// ETF 最新行情列表
    async fn get_quotes(&self, symbols: &[String]) -> Result<Vec<Stock>, DomainError>;

    /// ETF 基本信息
    async fn get_basic(&self, symbols: &[String]) -> Result<Vec<EtfBasic>, DomainError>;

    /// ETF 详细日线（含 pre_close / pct_chg / amount）
    async fn get_daily(&self, symbol: &str, days: usize) -> Result<Vec<EtfDaily>, DomainError>;

    /// ETF 历史 K 线（OHLCV 格式）
    async fn get_history(&self, symbol: &str, days: usize) -> Result<Vec<OHLCBar>, DomainError>;

    /// ETF 净值历史
    async fn get_nav(&self, symbol: &str, days: usize) -> Result<Vec<FundNav>, DomainError>;

    /// ETF 持仓明细（按报告期）
    async fn get_portfolio(
        &self,
        symbol: &str,
        period: Option<String>,
    ) -> Result<Vec<EtfPortfolio>, DomainError>;

    /// ETF 份额申赎记录
    async fn get_share(&self, symbol: &str, days: usize) -> Result<Vec<EtfShare>, DomainError>;

    /// ETF 分红历史
    async fn get_dividend(&self, symbol: &str) -> Result<Vec<EtfDividend>, DomainError>;

    /// ETF 跟踪指数日线
    async fn get_index(
        &self,
        index_code: &str,
        days: usize,
    ) -> Result<Vec<EtfIndex>, DomainError>;
}
