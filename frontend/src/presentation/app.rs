use std::rc::Rc;
use std::time::Duration;

use dioxus::prelude::*;
use gloo_timers::future::sleep;

use crate::application::DataSource;
use crate::infrastructure::{ConfigStorage, RepositoryFactory};
use crate::presentation::components::{ChartView, Dashboard, Settings, Sidebar};
use crate::presentation::view_models::{ChartType, ViewMode};
use crate::domain::errors::DomainError;
use crate::domain::entities::Stock;

#[component]
pub fn App() -> Element {
    // ── 状态 ─────────────────────────────────────────────────────────────────
    let mut view_mode      = use_signal(|| ViewMode::Dashboard);
    let mut selected_stock = use_signal(|| None::<String>);
    let mut chart_type     = use_signal(|| ChartType::Candlestick);
    let mut data_source    = use_signal(ConfigStorage::load_data_source);
    let mut refresh        = use_signal(|| 0_u32);

    // ── 依赖注入：DataSource 变化 → 重建 service ─────────────────────────────
    let service = use_memo(move || {
        Rc::new(RepositoryFactory::create_service(data_source()))
    });

    // ── 全球指数服务（始终使用 Yahoo Finance，固定指数代码）─────────────────────
    let index_service = use_memo(|| Rc::new(RepositoryFactory::create_index_service()));

    // ── 异步用例：获取股票列表（refresh 或 service 变化时重新请求）─────────────
    let stocks_res = use_resource(move || {
        let _tick = refresh();
        let svc   = service();
        async move { svc.get_all_stocks().await }
    });

    // ── 异步用例：获取全球指数（refresh 触发，服务固定）─────────────────────────
    let indices_res = use_resource(move || {
        let _tick = refresh();
        let svc   = index_service();
        async move { svc.get_all_stocks().await }
    });

    // ── 每 5 秒触发刷新 ───────────────────────────────────────────────────────
    use_future(move || async move {
        loop {
            sleep(Duration::from_secs(5)).await;
            refresh += 1;
        }
    });

    rsx! {
        style { {GLOBAL_CSS} }

        // ── 全屏左右布局 ───────────────────────────────────────────────────────
        div {
            style: "display: flex; min-height: 100vh;",

            // ── 左侧导航栏 ─────────────────────────────────────────────────────
            Sidebar {
                view_mode:        view_mode(),
                on_view_change:   move |v| view_mode.set(v),
                data_source:      data_source(),
                on_source_change: move |src: DataSource| {
                    ConfigStorage::save_data_source(&src);
                    data_source.set(src);
                    view_mode.set(ViewMode::Dashboard);
                    selected_stock.set(None);
                },
                on_settings: move |_| view_mode.set(ViewMode::Settings),
            }

            // ── 右侧内容区 ─────────────────────────────────────────────────────
            div {
                style: "flex: 1; background: #f1f5f9; min-height: 100vh; overflow-y: auto;",

                // 页面标题栏
                div {
                    style: "background: white; padding: 1.25rem 2rem; \
                           border-bottom: 1px solid #e2e8f0; \
                           display: flex; align-items: center; justify-content: space-between;",
                    div {
                        match view_mode() {
                            ViewMode::Dashboard => rsx! {
                                div {
                                    style: "font-size: 1.5rem; font-weight: 700; color: #1e293b; line-height: 1.2;",
                                    "市场全景"
                                }
                                div {
                                    style: "font-size: 0.8rem; color: #94a3b8; margin-top: 0.2rem;",
                                    "宏观在上，趋势在下"
                                }
                            },
                            ViewMode::Chart => rsx! {
                                div {
                                    style: "font-size: 1.5rem; font-weight: 700; color: #1e293b;",
                                    "图表分析"
                                }
                            },
                            ViewMode::Settings => rsx! {
                                div {
                                    style: "font-size: 1.5rem; font-weight: 700; color: #1e293b;",
                                    "设置"
                                }
                            },
                        }
                    }
                    div {
                        style: "font-size: 0.8125rem; color: #94a3b8;",
                        "数据源: {data_source().display_name()}"
                    }
                }

                // 内容主体
                div {
                    style: "padding: 2rem;",
                    match view_mode() {
                        ViewMode::Dashboard => rsx! {
                            DashboardPage {
                                stocks_res,
                                indices_res,
                                source_name: data_source().display_name(),
                                on_stock_select: move |symbol: String| {
                                    selected_stock.set(Some(symbol));
                                    view_mode.set(ViewMode::Chart);
                                },
                            }
                        },
                        ViewMode::Chart => rsx! {
                            ChartView {
                                service:              service(),
                                stock_symbol:         selected_stock().unwrap_or_default(),
                                chart_type:           chart_type(),
                                on_chart_type_change: move |t: ChartType| chart_type.set(t),
                                on_back:              move |_| view_mode.set(ViewMode::Dashboard),
                            }
                        },
                        ViewMode::Settings => rsx! {
                            Settings {
                                on_back: move |_| view_mode.set(ViewMode::Dashboard),
                            }
                        },
                    }
                }
            }
        }
    }
}

// ── 仪表盘页面（含 Loading / Error / 数据三态）────────────────────────────────

#[component]
fn DashboardPage(
    stocks_res: Resource<Result<Vec<Stock>, DomainError>>,
    indices_res: Resource<Result<Vec<Stock>, DomainError>>,
    source_name: &'static str,
    on_stock_select: EventHandler<String>,
) -> Element {
    let indices = match &*indices_res.read() {
        Some(Ok(v)) => v.clone(),
        _ => vec![],
    };
    match &*stocks_res.read() {
        None => rsx! { LoadingCard { message: "正在从 {source_name} 加载数据..." } },
        Some(Err(e)) => rsx! { ErrorCard { error: e.to_string() } },
        Some(Ok(stocks)) => rsx! {
            Dashboard {
                stocks: stocks.clone(),
                indices,
                on_stock_select,
            }
        },
    }
}

#[component]
fn LoadingCard(message: String) -> Element {
    rsx! {
        div {
            style: "display: flex; align-items: center; justify-content: center; \
                   height: 300px; background: white; border-radius: 16px; \
                   box-shadow: 0 4px 20px rgba(0,0,0,0.06);",
            div {
                style: "text-align: center; color: #94a3b8;",
                div { style: "font-size: 2rem; margin-bottom: 0.5rem;", "⏳" }
                div { "{message}" }
            }
        }
    }
}

#[component]
fn ErrorCard(error: String) -> Element {
    rsx! {
        div {
            style: "padding: 2rem; background: white; border-radius: 16px; \
                   box-shadow: 0 4px 20px rgba(0,0,0,0.06);",
            div { style: "color: #dc2626; font-size: 1.25rem; font-weight: 600; margin-bottom: 0.5rem;",
                  "⚠️ 数据加载失败" }
            div { style: "color: #64748b;", "{error}" }
            div {
                style: "margin-top: 1rem; padding: 1rem; background: #fefce8; \
                       border-radius: 8px; font-size: 0.875rem; color: #854d0e;",
                "提示：直接从浏览器访问 Tushare / Yahoo Finance API 会遇到 CORS 限制，"
                "请切换为「模拟数据」或通过后端代理访问实际 API。"
            }
        }
    }
}

// ── 全局样式 ──────────────────────────────────────────────────────────────────

const GLOBAL_CSS: &str = r#"
* { box-sizing: border-box; }
body {
    margin: 0; padding: 0;
    font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', sans-serif;
    -webkit-font-smoothing: antialiased;
}
@keyframes pulse { 0%, 100% { opacity: 1; } 50% { opacity: 0.5; } }
.stock-card:hover {
    transform: translateY(-4px);
    box-shadow: 0 8px 30px rgba(0,0,0,0.12) !important;
}
"#;
