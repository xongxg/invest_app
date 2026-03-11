use dioxus::prelude::*;
use crate::infrastructure::ConfigStorage;

/// 独立的 API 设置页面
///
/// 包含：
///   • Tushare Pro API Token
///   • Yahoo Finance API Key（付费版 / 自定义代理）
///   • AI Agent：Claude API Key、OpenAI API Key
#[component]
pub fn Settings(on_back: EventHandler<()>) -> Element {
    rsx! {
        div {
            style: "max-width: 640px;",

            // ── 页头 ─────────────────────────────────────────────────────────
            div {
                style: "display: flex; align-items: center; gap: 1rem; margin-bottom: 2rem;",
                button {
                    style: "padding: 0.5rem 1rem; border-radius: 8px; border: 1px solid #e2e8f0; \
                           background: white; cursor: pointer; font-size: 0.875rem; color: #64748b;",
                    onclick: move |_| on_back.call(()),
                    "← 返回"
                }
                h2 {
                    style: "margin: 0; font-size: 1.5rem; font-weight: 700; color: #1e293b;",
                    "设置"
                }
            }

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

            // ── 数据源 API Keys ───────────────────────────────────────────────
            SettingsCard {
                title: "数据源 API Keys",
                icon: "🔑",

                SettingsField {
                    label: "Tushare Pro Token",
                    description: "从 tushare.pro 获取的 API Token，用于 A 股数据",
                    placeholder: "请输入 Tushare Pro Token…",
                    initial_value: ConfigStorage::load_tushare_token(),
                    is_secret: true,
                    on_change: move |v: String| ConfigStorage::save_tushare_token(&v),
                }

                SettingsField {
                    label: "Yahoo Finance API Key",
                    description: "付费版或自定义代理的 API Key（公开接口留空即可）",
                    placeholder: "请输入 Yahoo Finance API Key…",
                    initial_value: ConfigStorage::load_yahoo_api_key(),
                    is_secret: true,
                    on_change: move |v: String| ConfigStorage::save_yahoo_api_key(&v),
                }
            }

            // ── AI Agent API Keys ─────────────────────────────────────────────
            SettingsCard {
                title: "AI Agent",
                icon: "🤖",

                SettingsField {
                    label: "Claude API Key",
                    description: "Anthropic Claude API 密钥（sk-ant-api03-…）",
                    placeholder: "sk-ant-api03-…",
                    initial_value: ConfigStorage::load_api_key("claude"),
                    is_secret: true,
                    on_change: move |v: String| ConfigStorage::save_api_key("claude", &v),
                }

                SettingsField {
                    label: "OpenAI API Key",
                    description: "OpenAI API 密钥（sk-…）",
                    placeholder: "sk-…",
                    initial_value: ConfigStorage::load_api_key("openai"),
                    is_secret: true,
                    on_change: move |v: String| ConfigStorage::save_api_key("openai", &v),
                }
            }

            // ── 说明 ──────────────────────────────────────────────────────────
            div {
                style: "margin-top: 1.5rem; padding: 1rem 1.25rem; background: #f0f9ff; \
                       border: 1px solid #bae6fd; border-radius: 10px; \
                       font-size: 0.8125rem; color: #0c4a6e; line-height: 1.6;",
                "ℹ️  所有密钥仅保存在本地浏览器 localStorage 中，不会上传至任何服务器。"
                br {}
                "若浏览器直接访问 Tushare / Yahoo Finance API 遇到 CORS 限制，请通过后端代理转发请求。"
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
