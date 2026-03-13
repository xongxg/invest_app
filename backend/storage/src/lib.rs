//! stock-storage: Apache Arrow 列式持久化存储层
//!
//! 对外暴露：
//! - [`ArrowStore`]：内存 + 磁盘双层缓存，Arrow IPC File 格式
//! - [`HealthDto`]：健康检查响应（基础设施层）
//!
//! 领域实体（Stock, OHLCBar, FundNav, …）由 `stock-domain` crate 提供。

mod store;
mod types;

pub use store::ArrowStore;
pub use types::HealthDto;
