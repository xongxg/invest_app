use async_trait::async_trait;
use rand::Rng;
use crate::domain::entities::{Stock, OHLCData};
use crate::domain::errors::DomainError;
use crate::domain::repositories::StockRepository;

/// 模拟数据仓储：生成本地随机数据，无需网络（用于开发/演示）
pub struct MockStockRepository {
    symbols: Vec<String>,
}

impl MockStockRepository {
    pub fn new(symbols: Vec<String>) -> Self {
        Self { symbols }
    }

    fn make_stock(symbol: &str) -> Stock {
        match symbol {
            "AAPL"  => Stock::new(symbol, "Apple Inc.",            175.43,  2.34, 52_341_234, "$2.7T"),
            "GOOGL" => Stock::new(symbol, "Alphabet Inc.",         142.67, -1.23, 28_456_123, "$1.8T"),
            "MSFT"  => Stock::new(symbol, "Microsoft Corporation", 378.91,  5.67, 21_234_567, "$2.8T"),
            "TSLA"  => Stock::new(symbol, "Tesla, Inc.",           242.84, -3.45, 98_765_432, "$770B"),
            "AMZN"  => Stock::new(symbol, "Amazon.com, Inc.",      151.23,  1.89, 45_678_901, "$1.6T"),
            _       => Stock::new(symbol, symbol,                  100.00,  0.00, 10_000_000, "N/A"),
        }
    }
}

#[async_trait(?Send)]
impl StockRepository for MockStockRepository {
    async fn get_all_stocks(&self) -> Result<Vec<Stock>, DomainError> {
        let mut rng = rand::rng();
        let stocks = self.symbols.iter().map(|sym| {
            let mut s = Self::make_stock(sym);
            // 每次刷新加入随机波动
            let delta: f64 = rng.random_range(-2.0..2.0);
            s.price          += delta;
            s.change         += delta;
            s.change_percent  = (s.change / (s.price - s.change)) * 100.0;
            s
        }).collect();
        Ok(stocks)
    }

    async fn get_stock(&self, symbol: &str) -> Result<Option<Stock>, DomainError> {
        Ok(Some(Self::make_stock(symbol)))
    }

    async fn get_history(&self, symbol: &str, days: usize) -> Result<Vec<OHLCData>, DomainError> {
        let base: f64 = match symbol {
            "AAPL"  => 175.0, "GOOGL" => 142.0, "MSFT" => 378.0,
            "TSLA"  => 242.0, "AMZN"  => 151.0, _      => 100.0,
        };

        let mut rng     = rand::rng();
        let mut current = base;
        let mut data    = Vec::with_capacity(days);

        for i in 0..days {
            let date:  String = format!("2024-{:02}-{:02}", (i / 30) % 12 + 1, i % 30 + 1);
            let open:  f64    = current + rng.random_range(-2.0..2.0);
            let close: f64    = open    + rng.random_range(-5.0..5.0);
            let high:  f64    = open.max(close) + rng.random_range(0.0..3.0);
            let low:   f64    = open.min(close) - rng.random_range(0.0..3.0);
            let volume: u64   = rng.random_range(10_000_000..100_000_000);
            data.push(OHLCData { date, open, high, low, close, volume });
            current = close;
        }

        Ok(data)
    }
}
