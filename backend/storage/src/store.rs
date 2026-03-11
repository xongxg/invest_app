//! Apache Arrow 列式持久化数据库
//!
//! 存储后端：Arrow IPC File 格式（`.arrow` 文件），每个 cache key 一个文件。
//! 内存中保留热数据（RwLock<HashMap>），重启后从磁盘恢复。
//!
//! 读取优先级
//! ──────────
//!   1. 内存缓存（最快，重启后丢失）
//!   2. 磁盘 Arrow 文件（持久，以文件 mtime 判断新鲜度）
//!   3. 外部 API（由调用方在 store miss 时主动触发）
//!
//! Schema
//! ──────
//! stocks : symbol Utf8 | name Utf8 | price f64 | change f64 |
//!          change_percent f64 | volume u64 | market_cap Utf8(nullable)
//!
//! ohlc   : date Utf8 | open f64 | high f64 | low f64 |
//!          close f64 | volume u64
//!
//! 文件名规则
//! ──────────
//! `{s|o}_{safe_key}.arrow`
//! 原始 cache key 存于 Arrow schema metadata["cache_key"]。

use std::collections::HashMap;
use std::fs::{self, File};
use std::path::{Path, PathBuf};
use std::sync::{Arc, RwLock};
use std::time::{Duration, SystemTime};

use arrow::array::{ArrayRef, Float64Array, StringArray, UInt64Array};
use arrow::datatypes::{DataType, Field, Schema};
use arrow::ipc::reader::FileReader;
use arrow::ipc::writer::FileWriter;
use arrow::record_batch::RecordBatch;

use crate::types::{OHLCDto, StockDto};

// ── Schema 定义 ───────────────────────────────────────────────────────────────

fn stock_schema() -> Arc<Schema> {
    Arc::new(Schema::new(vec![
        Field::new("symbol",         DataType::Utf8,    false),
        Field::new("name",           DataType::Utf8,    false),
        Field::new("price",          DataType::Float64, false),
        Field::new("change",         DataType::Float64, false),
        Field::new("change_percent", DataType::Float64, false),
        Field::new("volume",         DataType::UInt64,  false),
        Field::new("market_cap",     DataType::Utf8,    true),
    ]))
}

fn ohlc_schema() -> Arc<Schema> {
    Arc::new(Schema::new(vec![
        Field::new("date",   DataType::Utf8,    false),
        Field::new("open",   DataType::Float64, false),
        Field::new("high",   DataType::Float64, false),
        Field::new("low",    DataType::Float64, false),
        Field::new("close",  DataType::Float64, false),
        Field::new("volume", DataType::UInt64,  false),
    ]))
}

const META_KEY: &str = "cache_key";

// ── 内存 Entry ────────────────────────────────────────────────────────────────

struct Entry {
    batch:      RecordBatch,
    written_at: SystemTime,
}

impl Entry {
    fn is_fresh(&self, ttl: Duration) -> bool {
        self.written_at.elapsed().map(|e| e < ttl).unwrap_or(false)
    }
}

// ── ArrowStore ────────────────────────────────────────────────────────────────

pub struct ArrowStore {
    data_dir: PathBuf,
    stocks:   RwLock<HashMap<String, Entry>>,
    ohlc:     RwLock<HashMap<String, Entry>>,
}

impl ArrowStore {
    /// 创建 ArrowStore，并从磁盘加载已有数据到内存。
    pub fn new(data_dir: PathBuf) -> anyhow::Result<Self> {
        fs::create_dir_all(&data_dir)?;
        let store = Self {
            data_dir,
            stocks: RwLock::new(HashMap::new()),
            ohlc:   RwLock::new(HashMap::new()),
        };
        store.load_all_from_disk();
        Ok(store)
    }

    /// 已缓存的键数（内存层）
    pub fn cached_key_count(&self) -> usize {
        self.stocks.read().unwrap().len() + self.ohlc.read().unwrap().len()
    }

    // ── Stocks ────────────────────────────────────────────────────────────────

    /// 写入股票行情：同时更新内存缓存和磁盘 `.arrow` 文件。
    pub fn put_stocks(&self, key: &str, stocks: &[StockDto]) -> anyhow::Result<()> {
        let batch = stocks_to_batch(stocks, key)?;
        let path  = self.stocks_path(key);
        write_ipc(&path, &batch)?;
        self.stocks.write().unwrap().insert(key.to_string(), Entry {
            batch,
            written_at: SystemTime::now(),
        });
        tracing::debug!("put_stocks key={key} rows={} path={}", stocks.len(), path.display());
        Ok(())
    }

    /// 读取股票行情（内存优先 → 磁盘 → None）。
    pub fn get_stocks(&self, key: &str, ttl: Duration) -> Option<Vec<StockDto>> {
        {
            let cache = self.stocks.read().unwrap();
            if let Some(e) = cache.get(key) {
                if e.is_fresh(ttl) {
                    tracing::debug!("get_stocks mem-hit key={key}");
                    return Some(batch_to_stocks(&e.batch));
                }
            }
        }
        let path = self.stocks_path(key);
        if let Some((batch, written_at)) = load_ipc_if_fresh(&path, ttl) {
            tracing::debug!("get_stocks disk-hit key={key}");
            let data = batch_to_stocks(&batch);
            self.stocks.write().unwrap().insert(key.to_string(), Entry { batch, written_at });
            return Some(data);
        }
        None
    }

    /// 读取股票行情（忽略 TTL，有数据就返回——用于 API 失败时的旧数据兜底）
    pub fn get_stocks_stale(&self, key: &str) -> Option<Vec<StockDto>> {
        if let Some(e) = self.stocks.read().unwrap().get(key) {
            return Some(batch_to_stocks(&e.batch));
        }
        let path = self.stocks_path(key);
        let (batch, written_at) = load_ipc_raw(&path).ok()?;
        let data = batch_to_stocks(&batch);
        self.stocks.write().unwrap().insert(key.to_string(), Entry { batch, written_at });
        Some(data)
    }

    // ── OHLC ─────────────────────────────────────────────────────────────────

    pub fn put_ohlc(&self, key: &str, data: &[OHLCDto]) -> anyhow::Result<()> {
        let batch = ohlc_to_batch(data, key)?;
        let path  = self.ohlc_path(key);
        write_ipc(&path, &batch)?;
        self.ohlc.write().unwrap().insert(key.to_string(), Entry {
            batch,
            written_at: SystemTime::now(),
        });
        tracing::debug!("put_ohlc key={key} rows={} path={}", data.len(), path.display());
        Ok(())
    }

    pub fn get_ohlc(&self, key: &str, ttl: Duration) -> Option<Vec<OHLCDto>> {
        {
            let cache = self.ohlc.read().unwrap();
            if let Some(e) = cache.get(key) {
                if e.is_fresh(ttl) {
                    tracing::debug!("get_ohlc mem-hit key={key}");
                    return Some(batch_to_ohlc(&e.batch));
                }
            }
        }
        let path = self.ohlc_path(key);
        if let Some((batch, written_at)) = load_ipc_if_fresh(&path, ttl) {
            tracing::debug!("get_ohlc disk-hit key={key}");
            let data = batch_to_ohlc(&batch);
            self.ohlc.write().unwrap().insert(key.to_string(), Entry { batch, written_at });
            return Some(data);
        }
        None
    }

    // ── 启动时加载 ────────────────────────────────────────────────────────────

    fn load_all_from_disk(&self) {
        let Ok(dir) = fs::read_dir(&self.data_dir) else { return };

        for entry in dir.flatten() {
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) != Some("arrow") {
                continue;
            }

            match load_ipc_raw(&path) {
                Ok((batch, written_at)) => {
                    let key = batch.schema().metadata()
                        .get(META_KEY)
                        .cloned()
                        .unwrap_or_default();
                    if key.is_empty() { continue; }

                    let stem = path.file_stem().and_then(|s| s.to_str()).unwrap_or("");
                    if stem == "stock" {
                        self.stocks.write().unwrap()
                            .insert(key.clone(), Entry { batch, written_at });
                        tracing::info!("loaded stocks key={key} from disk");
                    } else if stem.starts_with("stock_") {
                        self.ohlc.write().unwrap()
                            .insert(key.clone(), Entry { batch, written_at });
                        tracing::info!("loaded ohlc key={key} from disk");
                    }
                }
                Err(e) => tracing::warn!("skip {}: {e}", path.display()),
            }
        }
    }

    /// Dashboard 聚合行情文件：固定名称 `stock.arrow`
    fn stocks_path(&self, _key: &str) -> PathBuf {
        self.data_dir.join("stock.arrow")
    }

    /// 单只股票 OHLC 文件：`stock_{symbol}.arrow`
    /// key 格式：`ohlc:{source}:{symbol}:{days}`
    fn ohlc_path(&self, key: &str) -> PathBuf {
        let symbol = key.splitn(4, ':').nth(2).unwrap_or(key);
        self.data_dir.join(format!("stock_{symbol}.arrow"))
    }
}

// ── IPC 读写 ──────────────────────────────────────────────────────────────────

fn write_ipc(path: &Path, batch: &RecordBatch) -> anyhow::Result<()> {
    let file   = File::create(path)?;
    let mut wr = FileWriter::try_new(file, batch.schema_ref())?;
    wr.write(batch)?;
    wr.finish()?;
    Ok(())
}

fn load_ipc_if_fresh(path: &Path, ttl: Duration) -> Option<(RecordBatch, SystemTime)> {
    let (batch, written_at) = load_ipc_raw(path).ok()?;
    let age = written_at.elapsed().ok()?;
    if age > ttl { return None; }
    Some((batch, written_at))
}

fn load_ipc_raw(path: &Path) -> anyhow::Result<(RecordBatch, SystemTime)> {
    let written_at = fs::metadata(path)?.modified()?;
    let file       = File::open(path)?;
    let reader     = FileReader::try_new(file, None)?;
    let batch      = reader
        .into_iter()
        .next()
        .ok_or_else(|| anyhow::anyhow!("empty arrow file: {}", path.display()))??;
    Ok((batch, written_at))
}

// ── 序列化：Vec<Dto> → RecordBatch ───────────────────────────────────────────

fn stocks_to_batch(stocks: &[StockDto], key: &str) -> anyhow::Result<RecordBatch> {
    let mut meta = HashMap::new();
    meta.insert(META_KEY.to_string(), key.to_string());
    let schema = Arc::new(stock_schema().as_ref().clone().with_metadata(meta));

    let cols: Vec<ArrayRef> = vec![
        Arc::new(StringArray::from(stocks.iter().map(|s| s.symbol.as_str()).collect::<Vec<_>>())),
        Arc::new(StringArray::from(stocks.iter().map(|s| s.name.as_str()).collect::<Vec<_>>())),
        Arc::new(Float64Array::from(stocks.iter().map(|s| s.price).collect::<Vec<_>>())),
        Arc::new(Float64Array::from(stocks.iter().map(|s| s.change).collect::<Vec<_>>())),
        Arc::new(Float64Array::from(stocks.iter().map(|s| s.change_percent).collect::<Vec<_>>())),
        Arc::new(UInt64Array::from(stocks.iter().map(|s| s.volume).collect::<Vec<_>>())),
        Arc::new(StringArray::from(stocks.iter().map(|s| s.market_cap.as_deref()).collect::<Vec<_>>())),
    ];
    Ok(RecordBatch::try_new(schema, cols)?)
}

fn ohlc_to_batch(data: &[OHLCDto], key: &str) -> anyhow::Result<RecordBatch> {
    let mut meta = HashMap::new();
    meta.insert(META_KEY.to_string(), key.to_string());
    let schema = Arc::new(ohlc_schema().as_ref().clone().with_metadata(meta));

    let cols: Vec<ArrayRef> = vec![
        Arc::new(StringArray::from(data.iter().map(|d| d.date.as_str()).collect::<Vec<_>>())),
        Arc::new(Float64Array::from(data.iter().map(|d| d.open  ).collect::<Vec<_>>())),
        Arc::new(Float64Array::from(data.iter().map(|d| d.high  ).collect::<Vec<_>>())),
        Arc::new(Float64Array::from(data.iter().map(|d| d.low   ).collect::<Vec<_>>())),
        Arc::new(Float64Array::from(data.iter().map(|d| d.close ).collect::<Vec<_>>())),
        Arc::new(UInt64Array::from( data.iter().map(|d| d.volume).collect::<Vec<_>>())),
    ];
    Ok(RecordBatch::try_new(schema, cols)?)
}

// ── 反序列化：RecordBatch → Vec<Dto> ─────────────────────────────────────────

fn batch_to_stocks(batch: &RecordBatch) -> Vec<StockDto> {
    let symbols     = col_str(batch, 0);
    let names       = col_str(batch, 1);
    let prices      = col_f64(batch, 2);
    let changes     = col_f64(batch, 3);
    let change_pcts = col_f64(batch, 4);
    let volumes     = col_u64(batch, 5);
    let caps        = col_str_opt(batch, 6);

    (0..batch.num_rows()).map(|i| StockDto {
        symbol:         symbols[i].clone(),
        name:           names[i].clone(),
        price:          prices[i],
        change:         changes[i],
        change_percent: change_pcts[i],
        volume:         volumes[i],
        market_cap:     caps[i].clone(),
    }).collect()
}

fn batch_to_ohlc(batch: &RecordBatch) -> Vec<OHLCDto> {
    let dates   = col_str(batch, 0);
    let opens   = col_f64(batch, 1);
    let highs   = col_f64(batch, 2);
    let lows    = col_f64(batch, 3);
    let closes  = col_f64(batch, 4);
    let volumes = col_u64(batch, 5);

    (0..batch.num_rows()).map(|i| OHLCDto {
        date:   dates[i].clone(),
        open:   opens[i],
        high:   highs[i],
        low:    lows[i],
        close:  closes[i],
        volume: volumes[i],
    }).collect()
}

// ── Column helpers ────────────────────────────────────────────────────────────

fn col_str(b: &RecordBatch, i: usize) -> Vec<String> {
    b.column(i).as_any().downcast_ref::<StringArray>().unwrap()
        .iter().map(|v| v.unwrap_or("").to_string()).collect()
}

fn col_str_opt(b: &RecordBatch, i: usize) -> Vec<Option<String>> {
    b.column(i).as_any().downcast_ref::<StringArray>().unwrap()
        .iter().map(|v| v.map(str::to_string)).collect()
}

fn col_f64(b: &RecordBatch, i: usize) -> Vec<f64> {
    b.column(i).as_any().downcast_ref::<Float64Array>().unwrap().values().to_vec()
}

fn col_u64(b: &RecordBatch, i: usize) -> Vec<u64> {
    b.column(i).as_any().downcast_ref::<UInt64Array>().unwrap().values().to_vec()
}
