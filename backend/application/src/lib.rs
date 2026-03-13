//! stock-application：应用服务层
//!
//! 应用服务是用例的编排者：依赖领域端口（接口），不依赖任何基础设施实现。
//! 具体仓储由调用方（gateway 接口层）在运行时注入。

use std::sync::Arc;

use stock_domain::{
    DomainError, EtfBasic, EtfDaily, EtfDividend, EtfIndex, EtfPortfolio, EtfShare,
    EtfRepository, FundNav, OHLCBar, Stock, StockRepository,
};

// ── 股票应用服务（A股 / 美股 / 港股）────────────────────────────────────────

pub struct StockAppService {
    repo: Arc<dyn StockRepository>,
}

impl StockAppService {
    pub fn new(repo: Arc<dyn StockRepository>) -> Self {
        Self { repo }
    }

    pub async fn get_quotes(&self, symbols: &[String]) -> Result<Vec<Stock>, DomainError> {
        self.repo.get_quotes(symbols).await
    }

    pub async fn get_ohlc(&self, symbol: &str, days: usize) -> Result<Vec<OHLCBar>, DomainError> {
        self.repo.get_ohlc(symbol, days).await
    }
}

// ── ETF 应用服务 ──────────────────────────────────────────────────────────────

pub struct EtfAppService {
    repo: Arc<dyn EtfRepository>,
}

impl EtfAppService {
    pub fn new(repo: Arc<dyn EtfRepository>) -> Self {
        Self { repo }
    }

    pub async fn get_quotes(&self, symbols: &[String]) -> Result<Vec<Stock>, DomainError> {
        self.repo.get_quotes(symbols).await
    }

    pub async fn get_basic(&self, symbols: &[String]) -> Result<Vec<EtfBasic>, DomainError> {
        self.repo.get_basic(symbols).await
    }

    pub async fn get_daily(&self, symbol: &str, days: usize) -> Result<Vec<EtfDaily>, DomainError> {
        self.repo.get_daily(symbol, days).await
    }

    pub async fn get_history(
        &self,
        symbol: &str,
        days: usize,
    ) -> Result<Vec<OHLCBar>, DomainError> {
        self.repo.get_history(symbol, days).await
    }

    pub async fn get_nav(&self, symbol: &str, days: usize) -> Result<Vec<FundNav>, DomainError> {
        self.repo.get_nav(symbol, days).await
    }

    pub async fn get_portfolio(
        &self,
        symbol: &str,
        period: Option<String>,
    ) -> Result<Vec<EtfPortfolio>, DomainError> {
        self.repo.get_portfolio(symbol, period).await
    }

    pub async fn get_share(
        &self,
        symbol: &str,
        days: usize,
    ) -> Result<Vec<EtfShare>, DomainError> {
        self.repo.get_share(symbol, days).await
    }

    pub async fn get_dividend(&self, symbol: &str) -> Result<Vec<EtfDividend>, DomainError> {
        self.repo.get_dividend(symbol).await
    }

    pub async fn get_index(
        &self,
        index_code: &str,
        days: usize,
    ) -> Result<Vec<EtfIndex>, DomainError> {
        self.repo.get_index(index_code, days).await
    }
}
