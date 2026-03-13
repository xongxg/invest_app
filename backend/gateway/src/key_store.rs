//! AES-256-GCM 加密的文件型 API Key 存储
//!
//! 数据保存在 `{data_dir}/keys.json`，格式：
//! ```json
//! { "tushare_token": { "ct": "<base64>", "nonce": "<base64>" }, ... }
//! ```
//! 每次写入时整体刷新文件。
//!
//! 主密钥：优先读取 `STOCK_MASTER_KEY` 环境变量（64 位十六进制，32 字节），
//! 否则自动生成并保存到 `{data_dir}/master.key`（原始字节）。

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::RwLock;

use aes_gcm::{
    aead::{Aead, AeadCore, KeyInit, OsRng},
    Aes256Gcm, Key, Nonce,
};
use anyhow::{Context, Result};
use base64::{engine::general_purpose::STANDARD as B64, Engine};
use serde::{Deserialize, Serialize};

// ── 文件内存储格式 ────────────────────────────────────────────────────────────

#[derive(Serialize, Deserialize, Default)]
struct KeyFile(HashMap<String, EncEntry>);

#[derive(Serialize, Deserialize, Clone)]
struct EncEntry {
    ct:    String, // base64 ciphertext
    nonce: String, // base64 nonce (12 bytes)
}

// ── KeyStore ──────────────────────────────────────────────────────────────────

pub struct KeyStore {
    path:   PathBuf,
    cipher: Aes256Gcm,
    data:   RwLock<KeyFile>,
}

unsafe impl Send for KeyStore {}
unsafe impl Sync for KeyStore {}

impl KeyStore {
    /// 打开（或新建）key 文件，加载主密钥。
    pub fn open(data_dir: &Path) -> Result<Self> {
        let master_key = load_or_create_master_key(data_dir)?;
        let key    = Key::<Aes256Gcm>::from_slice(&master_key);
        let cipher = Aes256Gcm::new(key);

        let path = data_dir.join("keys.json");
        let data = if path.exists() {
            let raw = std::fs::read_to_string(&path)
                .with_context(|| format!("read {}", path.display()))?;
            serde_json::from_str(&raw).unwrap_or_default()
        } else {
            KeyFile::default()
        };

        Ok(Self { path, cipher, data: RwLock::new(data) })
    }

    /// 加密并存储（upsert），立即写盘。
    pub fn set(&self, name: &str, value: &str) -> Result<()> {
        let nonce = Aes256Gcm::generate_nonce(&mut OsRng);
        let ct    = self.cipher
            .encrypt(&nonce, value.as_bytes())
            .map_err(|e| anyhow::anyhow!("encrypt: {e}"))?;

        let entry = EncEntry { ct: B64.encode(&ct), nonce: B64.encode(nonce) };
        {
            let mut data = self.data.write().unwrap();
            data.0.insert(name.to_string(), entry);
        }
        self.flush()
    }

    /// 解密并返回值；key 不存在时返回 `None`。
    pub fn get(&self, name: &str) -> Result<Option<String>> {
        let data  = self.data.read().unwrap();
        let entry = match data.0.get(name) {
            Some(e) => e.clone(),
            None    => return Ok(None),
        };
        drop(data);

        let ct    = B64.decode(&entry.ct).context("decode ct")?;
        let nonce = B64.decode(&entry.nonce).context("decode nonce")?;
        let nonce = Nonce::from_slice(&nonce);
        let plain = self.cipher
            .decrypt(nonce, ct.as_ref())
            .map_err(|e| anyhow::anyhow!("decrypt: {e}"))?;
        Ok(Some(String::from_utf8(plain)?))
    }

    /// 列出所有 key 名。
    pub fn list(&self) -> Vec<KeyMeta> {
        let data = self.data.read().unwrap();
        let mut names: Vec<_> = data.0.keys()
            .map(|n| KeyMeta { name: n.clone(), has_value: true })
            .collect();
        names.sort_by(|a, b| a.name.cmp(&b.name));
        names
    }

    /// 删除（不存在时静默忽略）。
    pub fn delete(&self, name: &str) -> Result<()> {
        {
            let mut data = self.data.write().unwrap();
            data.0.remove(name);
        }
        self.flush()
    }

    /// 若 header_val 非空则直接返回，否则尝试从文件读取。
    pub fn resolve(&self, name: &str, header_val: &str) -> String {
        if !header_val.is_empty() {
            return header_val.to_string();
        }
        self.get(name).ok().flatten().unwrap_or_default()
    }

    // ── 内部 ────────────────────────────────────────────────────────────────

    fn flush(&self) -> Result<()> {
        let data = self.data.read().unwrap();
        let json = serde_json::to_string_pretty(&*data)?;
        std::fs::write(&self.path, json)
            .with_context(|| format!("write {}", self.path.display()))
    }
}

// ── 主密钥 ────────────────────────────────────────────────────────────────────

fn load_or_create_master_key(data_dir: &Path) -> Result<[u8; 32]> {
    // 环境变量：64 个十六进制字符
    if let Ok(hex_str) = std::env::var("STOCK_MASTER_KEY") {
        let bytes = (0..hex_str.len())
            .step_by(2)
            .map(|i| u8::from_str_radix(&hex_str[i..i + 2], 16))
            .collect::<std::result::Result<Vec<u8>, _>>()
            .context("STOCK_MASTER_KEY: invalid hex")?;
        return bytes.try_into()
            .map_err(|_| anyhow::anyhow!("STOCK_MASTER_KEY must be 32 bytes (64 hex chars)"));
    }

    let key_path = data_dir.join("master.key");
    if key_path.exists() {
        let bytes = std::fs::read(&key_path).context("read master.key")?;
        return bytes.try_into()
            .map_err(|_| anyhow::anyhow!("master.key must be exactly 32 bytes"));
    }

    // 首次启动：生成随机密钥
    use rand::RngCore;
    let mut key = [0u8; 32];
    rand::thread_rng().fill_bytes(&mut key);
    std::fs::create_dir_all(data_dir)?;
    std::fs::write(&key_path, &key).context("write master.key")?;
    tracing::info!("Generated new AES-256 master key at {}", key_path.display());
    Ok(key)
}

// ── DTO ───────────────────────────────────────────────────────────────────────

#[derive(serde::Serialize)]
pub struct KeyMeta {
    pub name:      String,
    pub has_value: bool,
}
