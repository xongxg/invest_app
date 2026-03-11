use dioxus::prelude::*;
use crate::domain::entities::Stock;

const INDEX_LIST: &[(&str, &str)] = &[
    ("000001.SH", "上证指数"),
    ("399001.SZ", "深证成指"),
    ("399006.SZ", "创业板指"),
    ("HSI",       "恒生指数"),
    ("HSTECH",    "恒生科技"),
    ("NDX",       "纳斯达克100"),
];

#[component]
pub fn Dashboard(stocks: Vec<Stock>, indices: Vec<Stock>, on_stock_select: EventHandler<String>) -> Element {
    // ── 计算市场概览统计 ─────────────────────────────────────────────────────
    let up_count   = stocks.iter().filter(|s| s.change_percent > 0.0).count();
    let down_count = stocks.iter().filter(|s| s.change_percent < 0.0).count();
    let avg_change = if stocks.is_empty() {
        0.0_f64
    } else {
        stocks.iter().map(|s| s.change_percent).sum::<f64>() / stocks.len() as f64
    };
    let best  = stocks.iter().max_by(|a, b| a.change_percent.partial_cmp(&b.change_percent).unwrap());
    let worst = stocks.iter().min_by(|a, b| a.change_percent.partial_cmp(&b.change_percent).unwrap());

    let best_label  = best.map(|s| format!("{} {:.2}%", s.symbol, s.change_percent)).unwrap_or_default();
    let worst_label = worst.map(|s| format!("{} {:.2}%", s.symbol, s.change_percent)).unwrap_or_default();
    let avg_positive = avg_change >= 0.0;
    let avg_label    = format!("{}{:.2}%", if avg_positive { "+" } else { "" }, avg_change);
    let stock_count  = stocks.len();

    let mut show_indices = use_signal(|| true);

    rsx! {
        div {
            // ── 全球指数（可折叠）────────────────────────────────────────────
            div {
                style: "margin-bottom: 1.5rem;",
                div {
                    style: "display: flex; align-items: center; justify-content: space-between; \
                           margin-bottom: 0.75rem;",
                    div {
                        style: "font-size: 0.75rem; font-weight: 600; color: #64748b; \
                               letter-spacing: 0.08em; text-transform: uppercase;",
                        "全球指数"
                    }
                    button {
                        style: "background: none; border: none; cursor: pointer; \
                               font-size: 0.75rem; color: #94a3b8; padding: 0 0.25rem;",
                        onclick: move |_| show_indices.set(!show_indices()),
                        if show_indices() { "▲ 收起" } else { "▼ 展开" }
                    }
                }
                if show_indices() {
                    div {
                        style: "display: grid; grid-template-columns: repeat(auto-fill, minmax(180px, 1fr)); gap: 1rem;",
                        {INDEX_LIST.iter().map(|(sym, cn_name)| {
                            let live = indices.iter().find(|s| s.symbol == *sym);
                            let (price, chg, pct, arrow, color, bg_color) = match live {
                                Some(idx) => {
                                    let is_up    = idx.change_percent >= 0.0;
                                    let color    = if is_up { "#22c55e" } else { "#ef4444" };
                                    let bg_color = if is_up { "#f0fdf4" } else { "#fef2f2" };
                                    let arrow    = if is_up { "▲" } else { "▼" };
                                    let pct  = format!("{}{:.2}%", if is_up { "+" } else { "" }, idx.change_percent);
                                    let chg  = format!("{}{:.2}", if is_up { "+" } else { "" }, idx.change);
                                    let price = format!("{:.2}", idx.price);
                                    (price, chg, pct, arrow, color.to_string(), bg_color.to_string())
                                }
                                None => (
                                    "--".to_string(), "--".to_string(), "--.-- %".to_string(),
                                    "·", "#94a3b8".to_string(), "#f8fafc".to_string(),
                                ),
                            };
                            rsx! {
                                IndexCard {
                                    key: "{sym}",
                                    symbol: sym.to_string(),
                                    name: cn_name,
                                    price, chg, pct, arrow, color, bg_color,
                                }
                            }
                        })}
                    }
                }
            }

            // ── 市场概览 ──────────────────────────────────────────────────────
            div {
                style: "margin-bottom: 1.5rem;",
                div {
                    style: "font-size: 0.75rem; font-weight: 600; color: #64748b; \
                           letter-spacing: 0.08em; text-transform: uppercase; margin-bottom: 0.75rem;",
                    "市场概览"
                }
                div {
                    style: "display: grid; grid-template-columns: repeat(auto-fit, minmax(180px, 1fr)); gap: 1rem;",
                    OverviewCard {
                        label: "股票总数",
                        value: stock_count.to_string(),
                        sub: format!("↑{up_count} ↓{down_count}"),
                        accent: "#6366f1",
                    }
                    OverviewCard {
                        label: "平均涨跌幅",
                        value: avg_label,
                        sub: if avg_positive { "整体偏强".to_string() } else { "整体偏弱".to_string() },
                        accent: if avg_positive { "#22c55e".to_string() } else { "#ef4444".to_string() },
                    }
                    OverviewCard {
                        label: "最强个股",
                        value: best_label,
                        sub: "涨幅最大".to_string(),
                        accent: "#22c55e",
                    }
                    OverviewCard {
                        label: "最弱个股",
                        value: worst_label,
                        sub: "跌幅最大".to_string(),
                        accent: "#ef4444",
                    }
                }
            }

            // ── 个股行情 ──────────────────────────────────────────────────────
            div {
                style: "font-size: 0.75rem; font-weight: 600; color: #64748b; \
                       letter-spacing: 0.08em; text-transform: uppercase; margin-bottom: 0.75rem;",
                "个股行情"
            }
            div {
                style: "display: grid; grid-template-columns: repeat(auto-fill, minmax(320px, 1fr)); gap: 1.25rem;",
                {stocks.into_iter().map(|stock| {
                    let symbol  = stock.symbol.clone();
                    let handler = on_stock_select.clone();
                    rsx! {
                        StockCard {
                            key: "{symbol}",
                            stock,
                            on_click: move |_| handler.call(symbol.clone()),
                        }
                    }
                })}
            }
        }
    }
}

// ── 全球指数卡片 ──────────────────────────────────────────────────────────────

#[component]
fn IndexCard(
    symbol: String,
    name: &'static str,
    price: String,
    chg: String,
    pct: String,
    arrow: &'static str,
    color: String,
    bg_color: String,
) -> Element {
    rsx! {
        div {
            style: "background: white; border-radius: 12px; padding: 1.125rem 1.25rem; \
                   box-shadow: 0 1px 4px rgba(0,0,0,0.06); \
                   border-left: 3px solid {color};",

            // 代码（灰色小字）
            div {
                style: "font-size: 0.7rem; color: #94a3b8; font-family: monospace; \
                       margin-bottom: 0.2rem;",
                "{symbol}"
            }
            // 名称（加粗）
            div {
                style: "font-size: 0.9rem; font-weight: 700; color: #1e293b; \
                       margin-bottom: 0.625rem;",
                "{name}"
            }
            // 点位（大字）
            div {
                style: "font-size: 1.25rem; font-weight: 700; color: #1e293b; \
                       margin-bottom: 0.5rem;",
                "{price}"
            }
            // 涨跌额 + 涨跌幅
            div {
                style: "display: flex; align-items: center; gap: 0.5rem;",
                span {
                    style: "font-size: 0.8rem; font-weight: 600; color: {color};",
                    "{arrow} {chg}"
                }
                span {
                    style: "font-size: 0.75rem; font-weight: 600; color: {color}; \
                           background: {bg_color}; padding: 0.15rem 0.4rem; border-radius: 4px;",
                    "{pct}"
                }
            }
        }
    }
}

// ── 市场概览卡片 ──────────────────────────────────────────────────────────────

#[component]
fn OverviewCard(label: String, value: String, sub: String, accent: String) -> Element {
    rsx! {
        div {
            style: "background: white; border-radius: 12px; padding: 1.25rem; \
                   box-shadow: 0 1px 4px rgba(0,0,0,0.06); \
                   border-left: 3px solid {accent};",
            div {
                style: "font-size: 0.75rem; color: #94a3b8; margin-bottom: 0.375rem; font-weight: 500;",
                "{label}"
            }
            div {
                style: "font-size: 1.25rem; font-weight: 700; color: #1e293b; margin-bottom: 0.25rem;",
                "{value}"
            }
            div { style: "font-size: 0.75rem; color: #94a3b8;", "{sub}" }
        }
    }
}

// ── 股票卡片 ──────────────────────────────────────────────────────────────────

#[component]
pub fn StockCard(stock: Stock, on_click: EventHandler<()>) -> Element {
    let is_positive  = stock.is_positive();
    let change_color = if is_positive { "#22c55e" } else { "#ef4444" };
    let bg_color     = if is_positive { "#f0fdf4" } else { "#fef2f2" };
    let arrow        = if is_positive { "▲" } else { "▼" };

    rsx! {
        div {
            class: "stock-card",
            onclick: move |_| on_click.call(()),
            style: "background: white; border-radius: 14px; padding: 1.25rem; \
                   box-shadow: 0 1px 4px rgba(0,0,0,0.06); cursor: pointer; \
                   transition: all 0.25s ease; border-left: 4px solid {change_color};",

            // 股票代码 & 名称 & 价格
            div {
                style: "display: flex; justify-content: space-between; align-items: start; margin-bottom: 0.875rem;",
                div {
                    h3 { style: "margin: 0 0 0.2rem 0; font-size: 1.25rem; font-weight: 700; color: #1e293b;",
                         "{stock.symbol}" }
                    p  { style: "margin: 0; font-size: 0.8125rem; color: #64748b;",
                         { if stock.name.is_empty() { stock.symbol.clone() } else { stock.name.clone() } }
                    }
                }
                div {
                    style: "font-size: 1.625rem; font-weight: 700; color: #1e293b;",
                    "{stock.price:.2}"
                }
            }

            // 涨跌信息
            div {
                style: "background: {bg_color}; border-radius: 8px; padding: 0.75rem; \
                       display: flex; justify-content: space-between; align-items: center;",
                div { style: "color: {change_color}; font-weight: 700; font-size: 1rem;",
                      "{arrow} {stock.change:.2}" }
                div { style: "color: {change_color}; font-weight: 700; font-size: 1rem;",
                      "{stock.change_percent:.2}%" }
                div { style: "color: #94a3b8; font-size: 0.8125rem;",
                      "量: {stock.format_volume()}" }
            }

            // 市值 + 查看图表提示
            div {
                style: "margin-top: 0.875rem; padding-top: 0.875rem; border-top: 1px solid #f1f5f9; \
                       display: flex; justify-content: space-between; align-items: center;",
                if let Some(ref cap) = stock.market_cap {
                    span { style: "color: #64748b; font-size: 0.8125rem;", "市值: {cap}" }
                } else {
                    span {}
                }
                span { style: "color: #6366f1; font-size: 0.8125rem; font-weight: 500;",
                       "查看图表 →" }
            }
        }
    }
}
