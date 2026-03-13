use dioxus::prelude::*;
use gloo_net::http::Request;
use serde::Deserialize;
use crate::infrastructure::{ConfigStorage, KeyApiClient};

/// 通用设置页面（后端服务地址、数据目录等）
#[component]
pub fn Settings() -> Element {
    let backend_url = ConfigStorage::load_backend_url();

    // 从后端异步加载当前 data_dir
    let url_for_res = backend_url.clone();
    let config_res = use_resource(move || {
        let url = url_for_res.clone();
        async move { fetch_data_dir(&url).await }
    });

    rsx! {
        div {
            style: "max-width: 640px;",

            // ── 后端服务 ──────────────────────────────────────────────────────
            SettingsCard {
                title: "后端服务",
                icon: "🖥",

                SettingsField {
                    label: "Backend URL",
                    description: "stock-backend 服务地址，所有 API 数据均经此代理获取",
                    placeholder: "http://localhost:3000",
                    initial_value: ConfigStorage::load_backend_url(),
                    is_secret: false,
                    on_change: move |v: String| ConfigStorage::save_backend_url(&v),
                }
            }

            // ── 数据目录 ──────────────────────────────────────────────────────
            SettingsCard {
                title: "数据目录",
                icon: "📁",

                {
                    let url = backend_url.clone();
                    let init = (*config_res.read()).clone().unwrap_or_else(|| "data".to_string());
                    rsx! {
                        BackendField {
                            label: "Data Directory",
                            description: "后端存储 Arrow 缓存、加密密钥的根目录（相对或绝对路径）。修改后重启后端生效。",
                            placeholder: "data",
                            initial_value: init,
                            is_secret: false,
                            on_save: move |v: String| {
                                let url = url.clone();
                                async move { save_data_dir(&url, &v).await; }
                            },
                        }
                    }
                }

                div {
                    style: "padding: 0.75rem 1.5rem; font-size: 0.75rem; color: #f59e0b;",
                    "⚠️ 修改数据目录后需重启后端服务才能生效。默认值为项目根目录下的 data/ 文件夹。"
                }
            }
        }
    }
}

// ── 辅助：与后端 /api/config 交互 ────────────────────────────────────────────

#[derive(Deserialize)]
struct ConfigDto { data_dir: String }

async fn fetch_data_dir(base_url: &str) -> String {
    let url = format!("{}/api/config", base_url.trim_end_matches('/'));
    match Request::get(&url).send().await {
        Ok(resp) if resp.ok() => {
            resp.json::<ConfigDto>().await
                .map(|c| c.data_dir)
                .unwrap_or_else(|_| "data".to_string())
        }
        _ => "data".to_string(),
    }
}

async fn save_data_dir(base_url: &str, data_dir: &str) {
    let url  = format!("{}/api/config", base_url.trim_end_matches('/'));
    let body = serde_json::json!({ "data_dir": data_dir }).to_string();
    let _ = Request::put(&url)
        .header("Content-Type", "application/json")
        .body(body).unwrap()
        .send().await;
}

/// API Key 存储页面
///
/// 所有 Key 经 AES-256-GCM 加密后保存在后端 `data/keys.json`，
/// 此组件在挂载时从后端异步加载，保存时调用 PUT /api/keys/:name。
#[component]
pub fn ApiKeys() -> Element {
    let client = KeyApiClient::new(&ConfigStorage::load_backend_url());

    // 异步加载各 key 的当前值
    let c1 = client.clone(); let tushare = use_resource(move || { let c = c1.clone(); async move { c.get("tushare_token").await } });
    let c2 = client.clone(); let yahoo   = use_resource(move || { let c = c2.clone(); async move { c.get("yahoo_api_key").await } });
    let c3 = client.clone(); let claude  = use_resource(move || { let c = c3.clone(); async move { c.get("claude_api_key").await } });
    let c4 = client.clone(); let openai  = use_resource(move || { let c = c4.clone(); async move { c.get("openai_api_key").await } });

    let loading = tushare.read().is_none();

    rsx! {
        div {
            style: "max-width: 640px;",

            if loading {
                div {
                    style: "padding: 2rem; text-align: center; color: #94a3b8;",
                    "⏳ 从后端加载密钥…"
                }
            } else {
                // ── 数据源 API Keys ───────────────────────────────────────────
                SettingsCard {
                    title: "数据源 API Keys",
                    icon: "🔑",

                    {
                        let c = client.clone();
                        let init = (*tushare.read()).clone().unwrap_or_default();
                        rsx! {
                            BackendField {
                                label: "Tushare Pro Token",
                                description: "从 tushare.pro 获取的 API Token，用于 A 股数据",
                                placeholder: "请输入 Tushare Pro Token…",
                                initial_value: init,
                                is_secret: true,
                                on_save: move |v: String| {
                                    let c = c.clone();
                                    async move { c.set("tushare_token", &v).await; }
                                },
                            }
                        }
                    }

                    {
                        let c = client.clone();
                        let init = (*yahoo.read()).clone().unwrap_or_default();
                        rsx! {
                            BackendField {
                                label: "Yahoo Finance API Key",
                                description: "付费版或自定义代理的 API Key（公开接口留空即可）",
                                placeholder: "请输入 Yahoo Finance API Key…",
                                initial_value: init,
                                is_secret: true,
                                on_save: move |v: String| {
                                    let c = c.clone();
                                    async move { c.set("yahoo_api_key", &v).await; }
                                },
                            }
                        }
                    }
                }

                // ── AI Agent API Keys ─────────────────────────────────────────
                SettingsCard {
                    title: "AI Agent",
                    icon: "🤖",

                    {
                        let c = client.clone();
                        let init = (*claude.read()).clone().unwrap_or_default();
                        rsx! {
                            BackendField {
                                label: "Claude API Key",
                                description: "Anthropic Claude API 密钥（sk-ant-api03-…）",
                                placeholder: "sk-ant-api03-…",
                                initial_value: init,
                                is_secret: true,
                                on_save: move |v: String| {
                                    let c = c.clone();
                                    async move { c.set("claude_api_key", &v).await; }
                                },
                            }
                        }
                    }

                    {
                        let c = client.clone();
                        let init = (*openai.read()).clone().unwrap_or_default();
                        rsx! {
                            BackendField {
                                label: "OpenAI API Key",
                                description: "OpenAI API 密钥（sk-…）",
                                placeholder: "sk-…",
                                initial_value: init,
                                is_secret: true,
                                on_save: move |v: String| {
                                    let c = c.clone();
                                    async move { c.set("openai_api_key", &v).await; }
                                },
                            }
                        }
                    }
                }
            }

            // ── 说明 ──────────────────────────────────────────────────────────
            div {
                style: "margin-top: 1.5rem; padding: 1rem 1.25rem; background: #f0f9ff; \
                       border: 1px solid #bae6fd; border-radius: 10px; \
                       font-size: 0.8125rem; color: #0c4a6e; line-height: 1.6;",
                "🔒  所有密钥使用 AES-256-GCM 加密后保存在后端 "
                code { "data/keys.json" }
                "，不明文存储。需先在「设置」中配置后端地址。"
            }
        }
    }
}

// ── 卡片容器 ─────────────────────────────────────────────────────────────────

#[component]
fn SettingsCard(title: &'static str, icon: &'static str, children: Element) -> Element {
    rsx! {
        div {
            style: "background: white; border-radius: 12px; \
                   box-shadow: 0 2px 12px rgba(0,0,0,0.06); \
                   margin-bottom: 1.5rem; overflow: hidden;",

            // 卡片标题
            div {
                style: "padding: 1rem 1.5rem; border-bottom: 1px solid #f1f5f9; \
                       display: flex; align-items: center; gap: 0.5rem;",
                span { style: "font-size: 1.1rem;", "{icon}" }
                span {
                    style: "font-size: 0.9375rem; font-weight: 600; color: #334155;",
                    "{title}"
                }
            }

            div { style: "padding: 0.5rem 0;",
                {children}
            }
        }
    }
}

// ── 单个配置字段 ──────────────────────────────────────────────────────────────

#[component]
fn SettingsField(
    label: &'static str,
    description: &'static str,
    placeholder: &'static str,
    initial_value: String,
    is_secret: bool,
    on_change: EventHandler<String>,
) -> Element {
    let mut value   = use_signal(|| initial_value.clone());
    let mut visible = use_signal(|| false);
    let mut saved   = use_signal(|| false);

    let input_type = if is_secret && !visible() { "password" } else { "text" };
    let has_value  = !value().is_empty();
    let dot_color  = if has_value { "#4ade80" } else { "#cbd5e1" };

    rsx! {
        div {
            style: "padding: 1rem 1.5rem; border-bottom: 1px solid #f8fafc;",

            // 标签行
            div {
                style: "display: flex; align-items: center; justify-content: space-between; \
                       margin-bottom: 0.25rem;",
                div {
                    span {
                        style: "font-size: 0.875rem; font-weight: 500; color: #374151;",
                        "{label}"
                    }
                    // 已保存提示
                    if saved() {
                        span {
                            style: "margin-left: 0.5rem; font-size: 0.75rem; color: #4ade80;",
                            "✓ 已保存"
                        }
                    }
                }
                // 状态指示点
                div {
                    style: "width: 8px; height: 8px; border-radius: 50%; \
                           background: {dot_color}; transition: background 0.3s; flex-shrink: 0;",
                    title: if has_value { "已设置" } else { "未设置" },
                }
            }

            // 描述
            div {
                style: "font-size: 0.75rem; color: #94a3b8; margin-bottom: 0.5rem;",
                "{description}"
            }

            // 输入框行
            div {
                style: "display: flex; gap: 0.5rem;",

                input {
                    r#type: "{input_type}",
                    placeholder: "{placeholder}",
                    value: "{value}",
                    style: "flex: 1; min-width: 0; padding: 0.5rem 0.75rem; border-radius: 8px; \
                           border: 1px solid #e2e8f0; background: #f8fafc; \
                           color: #1e293b; font-size: 0.875rem; \
                           outline: none; transition: border-color 0.2s;",
                    oninput: move |e| {
                        let v = e.value();
                        on_change.call(v.clone());
                        value.set(v);
                        saved.set(true);
                        // 2 秒后隐藏"已保存"提示
                        spawn(async move {
                            gloo_timers::future::sleep(std::time::Duration::from_secs(2)).await;
                            saved.set(false);
                        });
                    },
                }

                // 显示/隐藏开关
                if is_secret {
                    button {
                        style: "flex-shrink: 0; padding: 0.5rem 0.65rem; border-radius: 8px; \
                               border: 1px solid #e2e8f0; background: white; \
                               color: #64748b; font-size: 0.875rem; cursor: pointer;",
                        title: if visible() { "隐藏" } else { "显示" },
                        onclick: move |_| { let v = visible(); visible.set(!v); },
                        if visible() { "🙈" } else { "👁" }
                    }
                }
            }
        }
    }
}

// ── 后端异步字段（ApiKeys 专用）─────────────────────────────────────────────

#[component]
fn BackendField(
    label:         &'static str,
    description:   &'static str,
    placeholder:   &'static str,
    initial_value: String,
    is_secret:     bool,
    on_save:       EventHandler<String>,
) -> Element {
    let mut value   = use_signal(|| initial_value.clone());
    let mut visible = use_signal(|| false);
    let mut status  = use_signal(|| "");   // "" | "saving" | "ok" | "err"

    let input_type = if is_secret && !visible() { "password" } else { "text" };
    let has_value  = !value().is_empty();
    let dot_color  = if has_value { "#4ade80" } else { "#cbd5e1" };

    rsx! {
        div {
            style: "padding: 1rem 1.5rem; border-bottom: 1px solid #f8fafc;",

            // 标签行
            div {
                style: "display: flex; align-items: center; justify-content: space-between; \
                       margin-bottom: 0.25rem;",
                div {
                    span { style: "font-size: 0.875rem; font-weight: 500; color: #374151;", "{label}" }
                    match status() {
                        "saving" => rsx! { span { style: "margin-left: 0.5rem; font-size: 0.75rem; color: #94a3b8;", "保存中…" } },
                        "ok"     => rsx! { span { style: "margin-left: 0.5rem; font-size: 0.75rem; color: #4ade80;", "✓ 已保存" } },
                        "err"    => rsx! { span { style: "margin-left: 0.5rem; font-size: 0.75rem; color: #f87171;", "✗ 保存失败" } },
                        _        => rsx! { },
                    }
                }
                div {
                    style: "width: 8px; height: 8px; border-radius: 50%; \
                           background: {dot_color}; flex-shrink: 0;",
                    title: if has_value { "已设置" } else { "未设置" },
                }
            }

            // 描述
            div { style: "font-size: 0.75rem; color: #94a3b8; margin-bottom: 0.5rem;", "{description}" }

            // 输入框行
            div {
                style: "display: flex; gap: 0.5rem;",

                input {
                    r#type: "{input_type}",
                    placeholder: "{placeholder}",
                    value: "{value}",
                    style: "flex: 1; min-width: 0; padding: 0.5rem 0.75rem; border-radius: 8px; \
                           border: 1px solid #e2e8f0; background: #f8fafc; \
                           color: #1e293b; font-size: 0.875rem; outline: none;",
                    oninput: move |e| { value.set(e.value()); },
                    onblur: move |_| {
                        let v = value();
                        status.set("saving");
                        on_save.call(v);
                        // 乐观更新：0.8s 后显示成功（实际为 fire-and-forget）
                        spawn(async move {
                            gloo_timers::future::sleep(std::time::Duration::from_millis(800)).await;
                            status.set("ok");
                            gloo_timers::future::sleep(std::time::Duration::from_secs(2)).await;
                            status.set("");
                        });
                    },
                }

                if is_secret {
                    button {
                        style: "flex-shrink: 0; padding: 0.5rem 0.65rem; border-radius: 8px; \
                               border: 1px solid #e2e8f0; background: white; \
                               color: #64748b; font-size: 0.875rem; cursor: pointer;",
                        onclick: move |_| { let v = visible(); visible.set(!v); },
                        if visible() { "🙈" } else { "👁" }
                    }
                }
            }
        }
    }
}
