use crate::domain::entities::{Stock, OHLCData};
use crate::domain::errors::DomainError;
use crate::domain::repositories::StockRepository;
use crate::application::DataSource;

/// 应用服务：编排领域对象与仓储，实现业务用例（异步版本）
pub struct StockApplicationService {
    repository: Box<dyn StockRepository>,
    pub source: DataSource,
}

impl StockApplicationService {
    /// 依赖注入入口：由 RepositoryFactory 创建对应仓储实现
    pub fn new(source: DataSource, repository: impl StockRepository + 'static) -> Self {
        Self {
            repository: Box::new(repository),
            source,
        }
    }

    /// 用例：获取所有关注股票列表
    pub async fn get_all_stocks(&self) -> Result<Vec<Stock>, DomainError> {
        self.repository.get_all_stocks().await
    }

    /// 用例：获取指定股票历史 K 线数据
    pub async fn get_stock_history(
        &self,
        symbol: &str,
        days: usize,
    ) -> Result<Vec<OHLCData>, DomainError> {
        self.repository.get_history(symbol, days).await
    }
}

/// Dioxus #[component] 要求 props 实现 PartialEq（服务对象语义上不变）
impl PartialEq for StockApplicationService {
    fn eq(&self, other: &Self) -> bool {
        self.source == other.source
    }
}
