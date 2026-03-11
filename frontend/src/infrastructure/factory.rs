use crate::application::{DataSource, StockApplicationService, YahooConfig};
use crate::infrastructure::repositories::{
    backend_repository::BackendApiRepository,
    mock_repository::MockStockRepository,
};
use crate::infrastructure::ConfigStorage;

/// 仓储工厂：根据 DataSource 创建对应的 StockApplicationService
///
/// • Mock         → MockStockRepository（本地随机数据，无需网络）
/// • TusharePro   → BackendApiRepository（通过 stock-backend 代理 Tushare API）
/// • YahooFinance → BackendApiRepository（通过 stock-backend 代理 Yahoo Finance API）
pub struct RepositoryFactory;

impl RepositoryFactory {
    pub fn create_service(source: DataSource) -> StockApplicationService {
        let backend_url = ConfigStorage::load_backend_url();
        let symbols     = source.default_symbols();

        match &source {
            DataSource::Mock => {
                let repo = MockStockRepository::new(symbols);
                StockApplicationService::new(source, repo)
            }
            DataSource::TusharePro(cfg) => {
                let repo = BackendApiRepository::new_tushare(
                    backend_url,
                    symbols,
                    cfg.token.clone(),
                );
                StockApplicationService::new(source, repo)
            }
            DataSource::YahooFinance(cfg) => {
                let repo = BackendApiRepository::new_yahoo(
                    backend_url,
                    cfg.symbols.clone(),
                    cfg.api_key.clone(),
                );
                StockApplicationService::new(source, repo)
            }
        }
    }

    /// 全球指数服务：始终使用 Yahoo Finance，固定指数代码
    pub fn create_index_service() -> StockApplicationService {
        let backend_url = ConfigStorage::load_backend_url();
        let symbols = vec![
            "000001.SH".to_string(), // 上证指数
            "399001.SZ".to_string(), // 深证成指
            "399006.SZ".to_string(), // 创业板指
            "HSI".to_string(),       // 恒生指数
            "HSTECH".to_string(),    // 恒生科技
            "NDX".to_string(),       // 纳斯达克100
        ];
        let api_key = ConfigStorage::load_yahoo_api_key();
        let repo = BackendApiRepository::new_yahoo(backend_url, symbols.clone(), api_key.clone());
        let source = DataSource::YahooFinance(YahooConfig { symbols, api_key });
        StockApplicationService::new(source, repo)
    }
}
