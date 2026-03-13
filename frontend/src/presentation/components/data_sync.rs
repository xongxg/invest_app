//! 数据同步页面
//!
//! 支持通过 Tushare Pro / Yahoo Finance 将指定时间段的数据同步到后端缓存。
//! 可选择多个 API 接口，批量拉取并写入 ArrowStore。

use dioxus::prelude::*;
use gloo_net::http::Request;
use serde::Serialize;

use crate::infrastructure::ConfigStorage;

// ── 可选 API 接口（按数据源分组）────────────────────────────────────────────

#[derive(Clone, PartialEq)]
struct ApiOption {
    id:    &'static str,
    label: &'static str,
    desc:  &'static str,
}

const TUSHARE_APIS: &[ApiOption] = &[
    ApiOption { id: "stocks",        label: "A股行情",       desc: "stock_basic + daily + daily_basic" },
    ApiOption { id: "history",       label: "A股历史K线",    desc: "daily OHLCV" },
    ApiOption { id: "etf_list",      label: "ETF行情列表",   desc: "fund_basic + fund_daily" },
    ApiOption { id: "etf_basic",     label: "ETF基本信息",   desc: "fund_basic" },
    ApiOption { id: "etf_daily",     label: "ETF日线详情",   desc: "fund_daily" },
    ApiOption { id: "etf_history",   label: "ETF历史K线",    desc: "fund_daily OHLCV" },
    ApiOption { id: "etf_nav",       label: "ETF净值",       desc: "fund_nav" },
    ApiOption { id: "etf_portfolio", label: "ETF持仓",       desc: "fund_portfolio" },
    ApiOption { id: "etf_trade",     label: "ETF申赎份额",   desc: "fund_share" },
    ApiOption { id: "etf_dividend",  label: "ETF分红",       desc: "fund_div" },
    ApiOption { id: "etf_index",     label: "跟踪指数日线",  desc: "index_daily" },
];

const YAHOO_APIS: &[ApiOption] = &[
    ApiOption { id: "stocks",  label: "股票行情",  desc: "chart (1d / 5d)" },
    ApiOption { id: "history", label: "历史K线",   desc: "chart (1d / range)" },
];

// ── 后端同步请求体 ────────────────────────────────────────────────────────────

#[derive(Serialize)]
struct SyncRequest {
    source:     String,
    symbols:    Vec<String>,
    start_date: String,
    end_date:   String,
    apis:       Vec<String>,
}

// ── 日志条目 ──────────────────────────────────────────────────────────────────

#[derive(Clone)]
struct LogEntry {
    ok:  bool,
    msg: String,
}

// ── 组件 ──────────────────────────────────────────────────────────────────────

#[component]
pub fn DataSync() -> Element {
    // ── 表单状态 ───────────────────────────────────────────────────────────────
    let mut source        = use_signal(|| "tushare".to_string());
    let mut symbols_input = use_signal(|| "510300.SH,510500.SH".to_string());
    let mut start_date    = use_signal(|| "2024-01-01".to_string());
    let mut end_date      = use_signal(|| chrono_today());
    let mut selected_apis = use_signal(|| vec!["etf_daily".to_string(), "etf_nav".to_string()]);
    let mut is_syncing    = use_signal(|| false);
    let mut log           = use_signal(|| Vec::<LogEntry>::new());

    let apis = if source() == "tushare" { TUSHARE_APIS } else { YAHOO_APIS };

    // ── 全选 / 全不选 ─────────────────────────────────────────────────────────
    let select_all = {
        let apis = apis;
        move |_| {
            let all: Vec<String> = apis.iter().map(|a| a.id.to_string()).collect();
            selected_apis.set(all);
        }
    };
    let clear_all = move |_| selected_apis.set(vec![]);

    // ── 点击同步 ───────────────────────────────────────────────────────────────
    let on_sync = {
        move |_| {
            if is_syncing() { return; }

            let syms: Vec<String> = symbols_input()
                .split(',')
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .collect();

            if syms.is_empty() {
                log.with_mut(|v| v.push(LogEntry { ok: false, msg: "请输入至少一个代码".into() }));
                return;
            }
            if selected_apis().is_empty() {
                log.with_mut(|v| v.push(LogEntry { ok: false, msg: "请至少选择一个接口".into() }));
                return;
            }

            let backend   = ConfigStorage::load_backend_url();
            let token     = ConfigStorage::load_tushare_token();
            let req_body  = SyncRequest {
                source:     source(),
                symbols:    syms,
                start_date: start_date(),
                end_date:   end_date(),
                apis:       selected_apis(),
            };

            is_syncing.set(true);
            log.with_mut(|v| {
                v.clear();
                v.push(LogEntry { ok: true, msg: format!(
                    "▶ 开始同步 | 来源: {} | 接口: {} | {} → {}",
                    req_body.source,
                    req_body.apis.join(", "),
                    req_body.start_date,
                    req_body.end_date,
                )});
            });

            spawn(async move {
                let url = format!("{}/api/sync", backend);
                let mut builder = Request::post(&url)
                    .header("Content-Type", "application/json");
                if !token.is_empty() {
                    builder = builder.header("x-tushare-token", &token);
                }
                match builder.json(&req_body) {
                    Err(e) => {
                        log.with_mut(|v| v.push(LogEntry { ok: false, msg: format!("序列化错误: {e}") }));
                    }
                    Ok(req) => match req.send().await {
                        Err(e) => {
                            log.with_mut(|v| v.push(LogEntry {
                                ok:  false,
                                msg: format!("网络错误: {e}"),
                            }));
                        }
                        Ok(resp) => {
                            let status = resp.status();
                            let body   = resp.text().await.unwrap_or_default();
                            if status == 200 {
                                log.with_mut(|v| v.push(LogEntry {
                                    ok:  true,
                                    msg: format!("✓ 同步成功: {body}"),
                                }));
                            } else {
                                log.with_mut(|v| v.push(LogEntry {
                                    ok:  false,
                                    msg: format!("✗ 服务端错误 {status}: {body}"),
                                }));
                            }
                        }
                    }
                }
                is_syncing.set(false);
            });
        }
    };

    // ── 渲染 ───────────────────────────────────────────────────────────────────
    rsx! {
        div {
            style: "display: flex; flex-direction: column; gap: 1.5rem;",

            // ── 顶部两列布局 ─────────────────────────────────────────────────
            div {
                style: "display: grid; grid-template-columns: 320px 1fr; gap: 1.5rem; \
                       align-items: start;",

                // ── 左列：配置卡片 ───────────────────────────────────────────
                div {
                    style: CARD_STYLE,

                    div { style: CARD_TITLE_STYLE, "⚙️  同步配置" }

                    // 数据源选择
                    FieldLabel { "数据源" }
                    div { style: "display: flex; gap: 0.75rem; margin-bottom: 1.25rem;",
                        SourceTab {
                            label: "Tushare Pro",
                            active: source() == "tushare",
                            on_click: move |_| {
                                source.set("tushare".to_string());
                                selected_apis.set(vec!["etf_daily".to_string()]);
                            },
                        }
                        SourceTab {
                            label: "Yahoo Finance",
                            active: source() == "yahoo",
                            on_click: move |_| {
                                source.set("yahoo".to_string());
                                selected_apis.set(vec!["stocks".to_string()]);
                            },
                        }
                    }

                    // 标的代码
                    FieldLabel { "标的代码（逗号分隔）" }
                    input {
                        style: INPUT_STYLE,
                        r#type: "text",
                        placeholder: "510300.SH, 510500.SH, ...",
                        value: "{symbols_input}",
                        oninput: move |e| symbols_input.set(e.value()),
                    }

                    // 开始日期
                    FieldLabel { "开始日期" }
                    input {
                        style: INPUT_STYLE,
                        r#type: "date",
                        value: "{start_date}",
                        oninput: move |e| start_date.set(e.value()),
                    }

                    // 结束日期
                    FieldLabel { "结束日期" }
                    input {
                        style: INPUT_STYLE,
                        r#type: "date",
                        value: "{end_date}",
                        oninput: move |e| end_date.set(e.value()),
                    }
                }

                // ── 右列：接口选择卡片 ───────────────────────────────────────
                div {
                    style: CARD_STYLE,

                    div {
                        style: "display: flex; align-items: center; justify-content: space-between; \
                               margin-bottom: 1.25rem;",
                        div { style: CARD_TITLE_STYLE, "🔌  同步接口" }
                        div { style: "display: flex; gap: 0.5rem;",
                            button {
                                style: MINI_BTN_STYLE,
                                onclick: select_all,
                                "全选"
                            }
                            button {
                                style: MINI_BTN_STYLE,
                                onclick: clear_all,
                                "清空"
                            }
                        }
                    }

                    div {
                        style: "display: grid; grid-template-columns: 1fr 1fr; gap: 0.625rem;",
                        for api in apis.iter() {
                            {
                                let api_id     = api.id;
                                let is_checked = selected_apis().contains(&api_id.to_string());
                                rsx! {
                                    ApiCheckbox {
                                        key: "{api_id}",
                                        id: api_id,
                                        label: api.label,
                                        desc: api.desc,
                                        checked: is_checked,
                                        on_toggle: move |_| {
                                            selected_apis.with_mut(|v| {
                                                if let Some(pos) = v.iter().position(|s| s == api_id) {
                                                    v.remove(pos);
                                                } else {
                                                    v.push(api_id.to_string());
                                                }
                                            });
                                        },
                                    }
                                }
                            }
                        }
                    }

                    // 已选摘要
                    div {
                        style: "margin-top: 1rem; padding: 0.75rem; background: #f8fafc; \
                               border-radius: 8px; font-size: 0.8rem; color: #64748b;",
                        "已选 {selected_apis().len()} / {apis.len()} 个接口：{selected_apis().join(\", \")}"
                    }
                }
            }

            // ── 同步按钮 ─────────────────────────────────────────────────────
            div { style: "display: flex; align-items: center; gap: 1rem;",
                button {
                    style: sync_btn_style(is_syncing()),
                    disabled: is_syncing(),
                    onclick: on_sync,
                    if is_syncing() { "⏳  正在同步..." } else { "🔄  开始同步" }
                }
                if is_syncing() {
                    div {
                        style: "font-size: 0.85rem; color: #64748b; \
                               animation: pulse 1.5s ease-in-out infinite;",
                        "请稍候，正在拉取数据并写入缓存..."
                    }
                }
            }

            // ── 同步日志 ─────────────────────────────────────────────────────
            if !log().is_empty() {
                div { style: CARD_STYLE,
                    div {
                        style: "display: flex; align-items: center; justify-content: space-between; \
                               margin-bottom: 1rem;",
                        div { style: CARD_TITLE_STYLE, "📋  同步日志" }
                        button {
                            style: MINI_BTN_STYLE,
                            onclick: move |_| log.set(vec![]),
                            "清空"
                        }
                    }
                    div {
                        style: "font-family: 'Menlo', 'Monaco', monospace; font-size: 0.82rem; \
                               background: #0f172a; border-radius: 8px; padding: 1rem; \
                               max-height: 320px; overflow-y: auto; display: flex; \
                               flex-direction: column; gap: 0.35rem;",
                        for entry in log().iter() {
                            {
                                let color = if entry.ok { "#4ade80" } else { "#f87171" };
                                let msg   = entry.msg.clone();
                                rsx! {
                                    div {
                                        style: "color: {color}; line-height: 1.5;",
                                        "{msg}"
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

// ── 子组件 ────────────────────────────────────────────────────────────────────

#[component]
fn SourceTab(label: &'static str, active: bool, on_click: EventHandler<()>) -> Element {
    let style = if active {
        "flex: 1; padding: 0.6rem 0; border-radius: 8px; border: 2px solid #3b82f6; \
         background: #eff6ff; color: #1d4ed8; font-weight: 600; font-size: 0.875rem; \
         cursor: pointer; transition: all 0.15s;"
    } else {
        "flex: 1; padding: 0.6rem 0; border-radius: 8px; border: 2px solid #e2e8f0; \
         background: white; color: #64748b; font-weight: 500; font-size: 0.875rem; \
         cursor: pointer; transition: all 0.15s;"
    };
    rsx! {
        button { style: "{style}", onclick: move |_| on_click.call(()), "{label}" }
    }
}

#[component]
fn ApiCheckbox(
    id:        &'static str,
    label:     &'static str,
    desc:      &'static str,
    checked:   bool,
    on_toggle: EventHandler<()>,
) -> Element {
    let border    = if checked { "border-color: #3b82f6; background: #eff6ff;" }
                    else       { "border-color: #e2e8f0; background: white;" };
    let box_style = if checked { "width: 16px; height: 16px; border-radius: 4px; flex-shrink: 0; \
                                  margin-top: 1px; display: flex; align-items: center; \
                                  justify-content: center; font-size: 0.7rem; \
                                  background: #3b82f6; color: white;" }
                    else       { "width: 16px; height: 16px; border-radius: 4px; flex-shrink: 0; \
                                  margin-top: 1px; display: flex; align-items: center; \
                                  justify-content: center; font-size: 0.7rem; \
                                  border: 1.5px solid #cbd5e1;" };
    rsx! {
        div {
            style: "display: flex; gap: 0.625rem; padding: 0.75rem; border-radius: 8px; \
                   border: 1.5px solid; cursor: pointer; transition: all 0.15s; {border}",
            onclick: move |_| on_toggle.call(()),

            // 复选框
            div {
                style: "{box_style}",
                if checked { "✓" }
            }
            div {
                div { style: "font-size: 0.85rem; font-weight: 600; color: #1e293b; line-height: 1.3;",
                    "{label}" }
                div { style: "font-size: 0.75rem; color: #94a3b8; margin-top: 0.1rem;",
                    "{desc}" }
            }
        }
    }
}

#[component]
fn FieldLabel(children: Element) -> Element {
    rsx! {
        div {
            style: "font-size: 0.8125rem; font-weight: 600; color: #475569; \
                   margin-bottom: 0.4rem; margin-top: 0;",
            {children}
        }
    }
}

// ── 辅助 ──────────────────────────────────────────────────────────────────────

fn chrono_today() -> String {
    // 在 WASM 中通过 js_sys 获取当前日期
    #[cfg(target_arch = "wasm32")]
    {
        use js_sys::Date;
        let d = Date::new_0();
        format!("{:04}-{:02}-{:02}", d.get_full_year(), d.get_month() + 1, d.get_date())
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        "2025-12-31".to_string()
    }
}

fn sync_btn_style(disabled: bool) -> &'static str {
    if disabled {
        "padding: 0.75rem 2rem; border-radius: 10px; border: none; \
         background: #94a3b8; color: white; font-size: 1rem; font-weight: 600; \
         cursor: not-allowed; opacity: 0.7;"
    } else {
        "padding: 0.75rem 2rem; border-radius: 10px; border: none; \
         background: linear-gradient(135deg, #3b82f6, #6366f1); \
         color: white; font-size: 1rem; font-weight: 600; \
         cursor: pointer; box-shadow: 0 4px 14px rgba(59,130,246,0.4); \
         transition: all 0.2s;"
    }
}

// ── 静态样式常量 ──────────────────────────────────────────────────────────────

const CARD_STYLE: &str =
    "background: white; border-radius: 16px; padding: 1.5rem; \
     box-shadow: 0 4px 20px rgba(0,0,0,0.06);";

const CARD_TITLE_STYLE: &str =
    "font-size: 1rem; font-weight: 700; color: #1e293b; margin-bottom: 1.25rem;";

const INPUT_STYLE: &str =
    "width: 100%; padding: 0.6rem 0.875rem; border: 1.5px solid #e2e8f0; \
     border-radius: 8px; font-size: 0.875rem; color: #1e293b; \
     background: white; outline: none; box-sizing: border-box; margin-bottom: 1rem;";

const MINI_BTN_STYLE: &str =
    "padding: 0.3rem 0.75rem; border-radius: 6px; border: 1px solid #e2e8f0; \
     background: white; color: #64748b; font-size: 0.78rem; cursor: pointer;";
