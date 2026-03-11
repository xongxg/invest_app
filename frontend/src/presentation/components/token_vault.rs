use dioxus::prelude::*;
use crate::infrastructure::ConfigStorage;

// ── Public entry point ────────────────────────────────────────────────────────

/// Collapsible API-key vault rendered inside the sidebar.
///
/// Sections:
///   • 数据源 — Tushare Pro token, Yahoo Finance symbols
///   • AI Agent — Claude API Key, OpenAI API Key  (extensible)
#[component]
pub fn TokenVault() -> Element {
    let mut open = use_signal(|| false);

    rsx! {
        div {
            style: "margin-top: 1.5rem;",

            // ── Section header (toggle) ──────────────────────────────────────
            button {
                style: "width: 100%; display: flex; align-items: center; justify-content: space-between; \
                       padding: 0 0.75rem; border: none; background: transparent; \
                       cursor: pointer; color: #475569; transition: color 0.2s;",
                onclick: move |_| { let v = open(); open.set(!v); },

                span {
                    style: "font-size: 0.6875rem; font-weight: 600; letter-spacing: 0.1em; \
                           text-transform: uppercase;",
                    "🔐 API 密钥"
                }
                span {
                    style: "font-size: 0.7rem;",
                    if open() { "▲" } else { "▼" }
                }
            }

            // ── Vault body ───────────────────────────────────────────────────
            if open() {
                div {
                    style: "margin-top: 0.75rem; display: flex; flex-direction: column; gap: 0.125rem;",

                    VaultGroup { title: "数据源" }

                    TokenEntry {
                        label: "Tushare Pro",
                        placeholder: "Token…",
                        initial_value: ConfigStorage::load_tushare_token(),
                        is_secret: true,
                        on_change: move |v: String| ConfigStorage::save_tushare_token(&v),
                    }
                    TokenEntry {
                        label: "Yahoo 标的",
                        placeholder: "AAPL,MSFT,GOOG",
                        initial_value: ConfigStorage::load_yahoo_symbols_str(),
                        is_secret: false,
                        on_change: move |v: String| {
                            let syms: Vec<String> = v.split(',')
                                .map(|s| s.trim().to_string())
                                .filter(|s| !s.is_empty())
                                .collect();
                            ConfigStorage::save_yahoo_symbols(&syms);
                        },
                    }

                    VaultGroup { title: "AI Agent" }

                    TokenEntry {
                        label: "Claude API Key",
                        placeholder: "sk-ant-api03-…",
                        initial_value: ConfigStorage::load_api_key("claude"),
                        is_secret: true,
                        on_change: move |v: String| ConfigStorage::save_api_key("claude", &v),
                    }
                    TokenEntry {
                        label: "OpenAI API Key",
                        placeholder: "sk-…",
                        initial_value: ConfigStorage::load_api_key("openai"),
                        is_secret: true,
                        on_change: move |v: String| ConfigStorage::save_api_key("openai", &v),
                    }
                }
            }
        }
    }
}

// ── Sub-components ────────────────────────────────────────────────────────────

/// Small group label inside the vault.
#[component]
fn VaultGroup(title: &'static str) -> Element {
    rsx! {
        div {
            style: "font-size: 0.625rem; font-weight: 600; color: #334155; \
                   letter-spacing: 0.08em; text-transform: uppercase; \
                   padding: 0.5rem 0.75rem 0.25rem;",
            "{title}"
        }
    }
}

/// Single token row: label + masked input + show/hide toggle + saved dot.
#[component]
fn TokenEntry(
    label: &'static str,
    placeholder: &'static str,
    initial_value: String,
    is_secret: bool,
    on_change: EventHandler<String>,
) -> Element {
    let mut value   = use_signal(|| initial_value.clone());
    let mut visible = use_signal(|| false);

    let input_type = if is_secret && !visible() { "password" } else { "text" };
    let has_value  = !value().is_empty();

    // Saved indicator colour: green dot if non-empty, grey if empty
    let dot_color = if has_value { "#4ade80" } else { "#334155" };

    rsx! {
        div {
            style: "padding: 0.35rem 0.75rem;",

            // Label + saved dot
            div {
                style: "display: flex; align-items: center; justify-content: space-between; \
                       margin-bottom: 0.25rem;",
                span {
                    style: "font-size: 0.75rem; color: #64748b;",
                    "{label}"
                }
                div {
                    style: "width: 6px; height: 6px; border-radius: 50%; background: {dot_color}; \
                           transition: background 0.3s;",
                    title: if has_value { "已保存" } else { "未设置" },
                }
            }

            // Input row
            div {
                style: "display: flex; gap: 0.25rem;",

                input {
                    r#type: "{input_type}",
                    placeholder: "{placeholder}",
                    value: "{value}",
                    style: "flex: 1; min-width: 0; padding: 0.4rem 0.5rem; border-radius: 6px; \
                           border: 1px solid #1e293b; background: #0f172a; \
                           color: #94a3b8; font-size: 0.75rem; \
                           outline: none; transition: border-color 0.2s;",
                    oninput: move |e| {
                        let v = e.value();
                        on_change.call(v.clone());
                        value.set(v);
                    },
                }

                // Show / hide toggle (only for secret fields)
                if is_secret {
                    button {
                        style: "flex-shrink: 0; padding: 0 0.4rem; border-radius: 6px; \
                               border: 1px solid #1e293b; background: #1e293b; \
                               color: #64748b; font-size: 0.75rem; cursor: pointer;",
                        title: if visible() { "隐藏" } else { "显示" },
                        onclick: move |_| { let v = visible(); visible.set(!v); },
                        if visible() { "🙈" } else { "👁" }
                    }
                }
            }
        }
    }
}
