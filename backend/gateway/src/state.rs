use std::sync::{Arc, RwLock};
use stock_storage::ArrowStore;
use crate::key_store::KeyStore;
use crate::server_config::ServerConfig;

pub struct AppState {
    pub store:        Arc<ArrowStore>,   // Yahoo Finance / 通用
    pub ashare_store: Arc<ArrowStore>,   // Tushare A股专用 (data/a_stock/)
    pub etf_store:    Arc<ArrowStore>,   // Tushare ETF 专用 (data/etf/)
    pub client:       reqwest::Client,
    pub keys:         Arc<KeyStore>,     // 加密 API Key 文件存储
    pub config:       Arc<RwLock<ServerConfig>>, // 服务器配置（可热更新，重启后生效）
}
