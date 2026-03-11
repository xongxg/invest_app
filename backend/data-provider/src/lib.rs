//! stock-data-provider: 外部数据抓取层
//!
//! 负责从外部数据源（Yahoo Finance、Tushare Pro）拉取行情数据，
//! 返回 `stock-storage` 定义的 DTO 类型，交由调用方写入 ArrowStore。
//!
//! 当前支持的 provider：
//! - [`providers::yahoo`]  – Yahoo Finance（美股、港股、ETF）
//! - [`providers::tushare`] – Tushare Pro（A 股）

pub mod providers;
