use async_trait::async_trait;
use crate::domain::entities::{Stock, OHLCData};
use crate::domain::errors::DomainError;

/// 股票仓储接口（异步，领域层定义，基础设施层实现）
///
/// `?Send` 使接口在 WASM 单线程环境下可用（无需 Send 约束）
#[async_trait(?Send)]
pub trait StockRepository {
    /// 获取所有关注股票的最新行情
    async fn get_all_stocks(&self) -> Result<Vec<Stock>, DomainError>;

    /// 获取单只股票最新行情
    async fn get_stock(&self, symbol: &str) -> Result<Option<Stock>, DomainError>;

    /// 获取历史 K 线数据
    async fn get_history(&self, symbol: &str, days: usize) -> Result<Vec<OHLCData>, DomainError>;
}
