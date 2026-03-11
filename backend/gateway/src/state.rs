use std::sync::Arc;
use stock_storage::ArrowStore;

pub struct AppState {
    pub store:        Arc<ArrowStore>,   // Yahoo Finance / 通用
    pub ashare_store: Arc<ArrowStore>,   // Tushare A股专用 (data/a_stock/)
    pub client:       reqwest::Client,
}
