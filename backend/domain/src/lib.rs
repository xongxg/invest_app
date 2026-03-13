//! stock-domain：领域层
//!
//! 对外暴露：
//! - [`entities`]：业务实体（Stock, OHLCBar, EtfBasic, …）
//! - [`errors::DomainError`]：领域错误类型
//! - [`ports::StockRepository`]、[`ports::EtfRepository`]：仓储端口（接口）

pub mod entities;
pub mod errors;
pub mod ports;

pub use entities::{
    EtfBasic, EtfDaily, EtfDividend, EtfIndex, EtfPortfolio, EtfShare,
    FundNav, OHLCBar, Stock,
};
pub use errors::DomainError;
pub use ports::{EtfRepository, StockRepository};
