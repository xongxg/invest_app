//! stock-storage: Apache Arrow 列式持久化存储层
//!
//! 对外暴露：
//! - [`ArrowStore`]：内存 + 磁盘双层缓存，Arrow IPC File 格式
//! - [`StockDto`]、[`OHLCDto`]、[`HealthDto`]：与前端 JSON 对齐的数据传输对象

mod store;
mod types;

pub use store::ArrowStore;
pub use types::{HealthDto, OHLCDto, StockDto};
