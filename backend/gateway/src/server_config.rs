//! 后端服务配置文件管理
//!
//! 配置文件路径优先级：
//!   1. 环境变量 `STOCK_CONFIG`（绝对或相对路径）
//!   2. 当前工作目录下的 `stock_config.json`
//!
//! `data_dir` 优先级：
//!   1. 配置文件中的 `data_dir` 字段
//!   2. 环境变量 `STOCK_DATA_DIR`
//!   3. 默认值 `"data"`

use std::path::PathBuf;

use anyhow::Result;
use serde::{Deserialize, Serialize};

// ── 配置结构 ──────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerConfig {
    pub data_dir: String,
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            data_dir: std::env::var("STOCK_DATA_DIR").unwrap_or_else(|_| "data".to_string()),
        }
    }
}

// ── 路径 ──────────────────────────────────────────────────────────────────────

pub fn config_path() -> PathBuf {
    std::env::var("STOCK_CONFIG")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("stock_config.json"))
}

// ── 读写 ──────────────────────────────────────────────────────────────────────

/// 从文件加载配置；文件不存在或解析失败时返回默认值。
pub fn load() -> ServerConfig {
    let path = config_path();
    if path.exists() {
        if let Ok(content) = std::fs::read_to_string(&path) {
            if let Ok(cfg) = serde_json::from_str::<ServerConfig>(&content) {
                return cfg;
            }
        }
    }
    ServerConfig::default()
}

/// 将配置写入文件。
pub fn save(cfg: &ServerConfig) -> Result<()> {
    let path = config_path();
    let json = serde_json::to_string_pretty(cfg)?;
    std::fs::write(&path, json)?;
    Ok(())
}
