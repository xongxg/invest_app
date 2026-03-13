//! 后端 /api/keys 的异步 HTTP 客户端

use gloo_net::http::Request;
use serde::{Deserialize, Serialize};

#[derive(Clone)]
pub struct KeyApiClient {
    base_url: String,
}

#[derive(Deserialize)]
struct KeyValue {
    #[allow(dead_code)]
    name:  String,
    value: String,
}

#[derive(Serialize)]
struct SetKeyBody<'a> {
    value: &'a str,
}

impl KeyApiClient {
    pub fn new(base_url: &str) -> Self {
        Self { base_url: base_url.trim_end_matches('/').to_string() }
    }

    /// 读取单个 key 的解密值；失败或不存在时返回空字符串。
    pub async fn get(&self, name: &str) -> String {
        let url = format!("{}/api/keys/{}", self.base_url, name);
        match Request::get(&url).send().await {
            Ok(resp) if resp.ok() => {
                resp.json::<KeyValue>().await.map(|kv| kv.value).unwrap_or_default()
            }
            _ => String::new(),
        }
    }

    /// 加密并存储 key；返回是否成功。
    pub async fn set(&self, name: &str, value: &str) -> bool {
        let url  = format!("{}/api/keys/{}", self.base_url, name);
        let body = serde_json::to_string(&SetKeyBody { value }).unwrap_or_default();
        Request::put(&url)
            .header("Content-Type", "application/json")
            .body(body)
            .unwrap()
            .send()
            .await
            .map(|r| r.ok())
            .unwrap_or(false)
    }
}
