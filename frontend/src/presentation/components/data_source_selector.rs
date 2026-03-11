use dioxus::prelude::*;
use crate::application::{DataSource, data_source::{TushareConfig, YahooConfig}};

#[component]
pub fn DataSourceSelector(
    current: DataSource,
    on_change: EventHandler<DataSource>,
) -> Element {
    let mut show_panel    = use_signal(|| false);
    let mut tushare_token = use_signal(|| String::new());
    let mut yahoo_symbols = use_signal(|| "AAPL,GOOGL,MSFT,TSLA,AMZN".to_string());

    let source_label = current.display_name();
    let panel_arrow  = if show_panel() { "▲" } else { "▼" };

    // 预计算边框颜色，避免在 rsx! 内写复杂表达式
    let tushare_border = if matches!(current, DataSource::TusharePro(_)) { "#667eea" } else { "#e2e8f0" };
    let yahoo_border   = if matches!(current, DataSource::YahooFinance(_)) { "#667eea" } else { "#e2e8f0" };
    let apply_tushare_opacity = if tushare_token().is_empty() { "0.5" } else { "1" };

    rsx! {
        div { style: "position: relative;",

            // 当前数据源按钮
            button {
                onclick: move |_| show_panel.set(!show_panel()),
                style: "display: flex; align-items: center; gap: 0.5rem; \
                       padding: 0.5rem 1rem; background: white; border: 2px solid #667eea; \
                       border-radius: 8px; cursor: pointer; font-size: 0.875rem; font-weight: 600; color: #667eea;",
                span { "🔌" }
                span { "{source_label}" }
                span { "{panel_arrow}" }
            }

            // 弹出面板
            if show_panel() {
                div {
                    style: "position: absolute; right: 0; top: calc(100% + 8px); \
                           background: white; border-radius: 12px; \
                           box-shadow: 0 8px 32px rgba(0,0,0,0.15); \
                           padding: 1.5rem; min-width: 320px; z-index: 100; border: 1px solid #e2e8f0;",

                    h3 { style: "margin: 0 0 1rem 0; font-size: 1rem; font-weight: 700; color: #2d3748;",
                         "选择数据源" }

                    // 模拟数据
                    SourceOption {
                        label: "📦 模拟数据",
                        desc:  "本地随机生成，无需网络和 API key",
                        active: matches!(current, DataSource::Mock),
                        on_select: move |_| {
                            on_change.call(DataSource::Mock);
                            show_panel.set(false);
                        }
                    }

                    // Tushare Pro
                    div {
                        style: "margin-top: 0.75rem; padding: 1rem; border-radius: 8px; \
                               border: 2px solid {tushare_border};",

                        div { style: "font-weight: 600; color: #2d3748; margin-bottom: 4px;",
                              "🇨🇳 Tushare Pro" }
                        div { style: "font-size: 0.75rem; color: #718096; margin-bottom: 0.75rem;",
                              "A 股数据，需要 Token（需后端代理解决 CORS）" }

                        input {
                            r#type: "text",
                            placeholder: "请输入 Tushare Token",
                            value: "{tushare_token}",
                            oninput: move |e| tushare_token.set(e.value()),
                            style: "width: 100%; padding: 0.5rem; border: 1px solid #e2e8f0; \
                                   border-radius: 6px; font-size: 0.875rem; \
                                   box-sizing: border-box; margin-bottom: 0.5rem;",
                        }

                        button {
                            disabled: tushare_token().is_empty(),
                            onclick: move |_| {
                                let token = tushare_token();
                                if !token.is_empty() {
                                    on_change.call(DataSource::TusharePro(TushareConfig::new(token)));
                                    show_panel.set(false);
                                }
                            },
                            style: "width: 100%; padding: 0.4rem; border: none; border-radius: 6px; \
                                   font-size: 0.875rem; font-weight: 600; cursor: pointer; \
                                   background: #667eea; color: white; opacity: {apply_tushare_opacity};",
                            "应用 Tushare Pro"
                        }
                    }

                    // Yahoo Finance
                    div {
                        style: "margin-top: 0.75rem; padding: 1rem; border-radius: 8px; \
                               border: 2px solid {yahoo_border};",

                        div { style: "font-weight: 600; color: #2d3748; margin-bottom: 4px;",
                              "🌐 Yahoo Finance" }
                        div { style: "font-size: 0.75rem; color: #718096; margin-bottom: 0.75rem;",
                              "美股/港股/ETF，需后端代理解决 CORS" }

                        input {
                            r#type: "text",
                            placeholder: "股票代码，逗号分隔（如 AAPL,TSLA）",
                            value: "{yahoo_symbols}",
                            oninput: move |e| yahoo_symbols.set(e.value()),
                            style: "width: 100%; padding: 0.5rem; border: 1px solid #e2e8f0; \
                                   border-radius: 6px; font-size: 0.875rem; \
                                   box-sizing: border-box; margin-bottom: 0.5rem;",
                        }

                        button {
                            onclick: move |_| {
                                let symbols: Vec<String> = yahoo_symbols()
                                    .split(',')
                                    .map(|s| s.trim().to_uppercase())
                                    .filter(|s| !s.is_empty())
                                    .collect();
                                on_change.call(DataSource::YahooFinance(YahooConfig::new(symbols, crate::infrastructure::ConfigStorage::load_yahoo_api_key())));
                                show_panel.set(false);
                            },
                            style: "width: 100%; padding: 0.4rem; border: none; border-radius: 6px; \
                                   font-size: 0.875rem; font-weight: 600; cursor: pointer; \
                                   background: #667eea; color: white;",
                            "应用 Yahoo Finance"
                        }
                    }

                    // CORS 提示
                    div {
                        style: "margin-top: 1rem; padding: 0.75rem; background: #fffbeb; \
                               border-radius: 6px; border-left: 3px solid #f59e0b; \
                               font-size: 0.75rem; color: #92400e;",
                        "💡 Tushare / Yahoo Finance 直接从浏览器调用受 CORS 限制，"
                        "生产环境需通过同域后端代理转发 API 请求。"
                    }
                }
            }
        }
    }
}

#[component]
fn SourceOption(
    label: &'static str,
    desc: &'static str,
    active: bool,
    on_select: EventHandler<()>,
) -> Element {
    let border = if active { "#667eea" } else { "#e2e8f0" };
    rsx! {
        div {
            onclick: move |_| on_select.call(()),
            style: "padding: 0.75rem 1rem; border-radius: 8px; \
                   border: 2px solid {border}; cursor: pointer; transition: border-color 0.2s;",
            div { style: "font-weight: 600; color: #2d3748;", "{label}" }
            div { style: "font-size: 0.75rem; color: #718096; margin-top: 2px;", "{desc}" }
        }
    }
}
