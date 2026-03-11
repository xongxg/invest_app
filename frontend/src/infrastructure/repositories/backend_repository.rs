//! 后端 API 仓储实现（前端 WASM 侧）
//!
//! 所有行情 / 历史数据都通过 `stock-backend` 服务获取。
//! API Key 通过请求头传递给后端，由后端再转发给 Tushare / Yahoo Finance。

use async_trait::async_trait;

use crate::domain::entities::{OHLCData, Stock};
use crate::domain::errors::DomainError;
use crate::domain::repositories::StockRepository;

pub struct BackendApiRepository {
    base_url:      String,
    source:        String,   // "tushare" | "yahoo"
    symbols:       Vec<String>,
    tushare_token: String,
    yahoo_api_key: String,
}

impl BackendApiRepository {
    pub fn new_tushare(
        base_url: String,
        symbols: Vec<String>,
        tushare_token: String,
    ) -> Self {
        Self {
            base_url,
            source: "tushare".into(),
            symbols,
            tushare_token,
            yahoo_api_key: String::new(),
        }
    }

    pub fn new_yahoo(
        base_url: String,
        symbols: Vec<String>,
        yahoo_api_key: String,
    ) -> Self {
        Self {
            base_url,
            source: "yahoo".into(),
            symbols,
            tushare_token: String::new(),
            yahoo_api_key,
        }
    }

    /// 构建带凭证 Header 的 GET 请求
    fn get(&self, path: &str) -> gloo_net::http::RequestBuilder {
        let req = gloo_net::http::Request::get(&format!("{}{}", self.base_url, path));
        let req = if self.tushare_token.is_empty() {
            req
        } else {
            req.header("X-Tushare-Token", &self.tushare_token)
        };
        if self.yahoo_api_key.is_empty() {
            req
        } else {
            req.header("X-Yahoo-Api-Key", &self.yahoo_api_key)
        }
    }
}

#[async_trait(?Send)]
impl StockRepository for BackendApiRepository {
    async fn get_all_stocks(&self) -> Result<Vec<Stock>, DomainError> {
        let symbols = self.symbols.join(",");
        let url     = format!("/api/stocks?source={}&symbols={}", self.source, symbols);

        let resp = self.get(&url)
            .send()
            .await
            .map_err(|e| DomainError::NetworkError(format!("backend: {e}")))?;

        if !resp.ok() {
            let body = resp.text().await.unwrap_or_default();
            return Err(DomainError::ApiError(format!("backend HTTP {}: {body}", resp.status())));
        }

        resp.json::<Vec<Stock>>()
            .await
            .map_err(|e| DomainError::ParseError(e.to_string()))
    }

    async fn get_stock(&self, symbol: &str) -> Result<Option<Stock>, DomainError> {
        let all = self.get_all_stocks().await?;
        Ok(all.into_iter().find(|s| s.symbol == symbol))
    }

    async fn get_history(&self, symbol: &str, days: usize) -> Result<Vec<OHLCData>, DomainError> {
        let url = format!(
            "/api/history?source={}&symbol={}&days={}",
            self.source, symbol, days
        );

        let resp = self.get(&url)
            .send()
            .await
            .map_err(|e| DomainError::NetworkError(format!("backend: {e}")))?;

        if !resp.ok() {
            let body = resp.text().await.unwrap_or_default();
            return Err(DomainError::ApiError(format!("backend HTTP {}: {body}", resp.status())));
        }

        resp.json::<Vec<OHLCData>>()
            .await
            .map_err(|e| DomainError::ParseError(e.to_string()))
    }
}
