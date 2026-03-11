use gloo_storage::{LocalStorage, Storage};

use crate::application::{DataSource, TushareConfig, YahooConfig};

// ── Storage key constants ─────────────────────────────────────────────────────

const KEY_TUSHARE_TOKEN: &str = "stock_app_tushare_token";
const KEY_YAHOO_SYMBOLS: &str = "stock_app_yahoo_symbols";
const KEY_YAHOO_API_KEY: &str = "stock_app_yahoo_api_key";
const KEY_LAST_SOURCE:   &str = "stock_app_last_source";
const KEY_BACKEND_URL:   &str = "stock_app_backend_url";
const DEFAULT_BACKEND:   &str = "http://localhost:3000";

/// Prefix for generic API keys: `stock_app_key_{name}`.
fn api_key_key(name: &str) -> String {
    format!("stock_app_key_{}", name)
}

// ── ConfigStorage ─────────────────────────────────────────────────────────────

/// Thin wrapper around `localStorage` for persisting API tokens and config.
pub struct ConfigStorage;

impl ConfigStorage {
    // ── Tushare ───────────────────────────────────────────────────────────────

    pub fn save_tushare_token(token: &str) {
        let _ = LocalStorage::set(KEY_TUSHARE_TOKEN, token);
    }

    pub fn load_tushare_token() -> String {
        LocalStorage::get::<String>(KEY_TUSHARE_TOKEN).unwrap_or_default()
    }

    // ── Yahoo Finance ─────────────────────────────────────────────────────────

    /// `symbols` is stored as comma-separated, e.g. `"AAPL,MSFT,GOOG"`.
    pub fn save_yahoo_symbols(symbols: &[String]) {
        let _ = LocalStorage::set(KEY_YAHOO_SYMBOLS, symbols.join(","));
    }

    /// Returns a raw comma-separated string for populating the text input.
    pub fn load_yahoo_symbols_str() -> String {
        LocalStorage::get::<String>(KEY_YAHOO_SYMBOLS).unwrap_or_default()
    }

    pub fn save_yahoo_api_key(key: &str) {
        let _ = LocalStorage::set(KEY_YAHOO_API_KEY, key);
    }

    pub fn load_yahoo_api_key() -> String {
        LocalStorage::get::<String>(KEY_YAHOO_API_KEY).unwrap_or_default()
    }

    // ── Backend URL ───────────────────────────────────────────────────────────

    pub fn save_backend_url(url: &str) {
        let _ = LocalStorage::set(KEY_BACKEND_URL, url);
    }

    pub fn load_backend_url() -> String {
        LocalStorage::get::<String>(KEY_BACKEND_URL)
            .unwrap_or_else(|_| DEFAULT_BACKEND.to_string())
    }

    // ── Generic API key vault ─────────────────────────────────────────────────
    //
    // Used for any provider not modelled in DataSource (e.g. LLM services).
    // `name` should be a stable lowercase identifier such as "claude", "openai".

    pub fn save_api_key(name: &str, key: &str) {
        let _ = LocalStorage::set(api_key_key(name), key);
    }

    pub fn load_api_key(name: &str) -> String {
        LocalStorage::get::<String>(api_key_key(name)).unwrap_or_default()
    }

    // ── Last active DataSource ────────────────────────────────────────────────

    fn save_source_key(key: &str) {
        let _ = LocalStorage::set(KEY_LAST_SOURCE, key);
    }

    fn load_source_key() -> String {
        LocalStorage::get::<String>(KEY_LAST_SOURCE).unwrap_or_else(|_| "mock".to_string())
    }

    // ── High-level DataSource helpers ─────────────────────────────────────────

    /// Persist the active `DataSource` (including its credentials).
    pub fn save_data_source(source: &DataSource) {
        match source {
            DataSource::Mock => {
                Self::save_source_key("mock");
            }
            DataSource::TusharePro(cfg) => {
                Self::save_source_key("tushare");
                Self::save_tushare_token(&cfg.token);
            }
            DataSource::YahooFinance(cfg) => {
                Self::save_source_key("yahoo");
                Self::save_yahoo_symbols(&cfg.symbols);
                Self::save_yahoo_api_key(&cfg.api_key);
            }
        }
    }

    /// Reconstruct the last-used `DataSource` from `localStorage`.
    pub fn load_data_source() -> DataSource {
        match Self::load_source_key().as_str() {
            "tushare" => DataSource::TusharePro(TushareConfig {
                token: Self::load_tushare_token(),
            }),
            "yahoo" => {
                let symbols = Self::load_yahoo_symbols_str()
                    .split(',')
                    .map(|s| s.trim().to_string())
                    .filter(|s| !s.is_empty())
                    .collect();
                DataSource::YahooFinance(YahooConfig {
                    symbols,
                    api_key: Self::load_yahoo_api_key(),
                })
            }
            _ => DataSource::Mock,
        }
    }
}
