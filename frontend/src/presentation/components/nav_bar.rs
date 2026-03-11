use dioxus::prelude::*;
use crate::application::DataSource;
use crate::presentation::view_models::ViewMode;
use crate::presentation::components::DataSourceSelector;

#[component]
pub fn NavBar(
    view_mode: ViewMode,
    chart_enabled: bool,
    on_view_change: EventHandler<ViewMode>,
    data_source: DataSource,
    on_source_change: EventHandler<DataSource>,
) -> Element {
    let nav_btn = |label: &'static str, target: ViewMode, active: bool, enabled: bool| {
        let active_style = "background: linear-gradient(135deg, #667eea 0%, #764ba2 100%); \
                            color: white; box-shadow: 0 4px 15px rgba(102,126,234,0.4);";
        let inactive_style = "background: #e2e8f0; color: #4a5568;";
        let disabled_style = "background: #e2e8f0; color: #4a5568; opacity: 0.6; cursor: not-allowed;";

        let style = format!(
            "padding: 0.5rem 1.5rem; border: none; border-radius: 8px; \
             font-weight: 600; font-size: 1rem; transition: all 0.3s; {}",
            if !enabled { disabled_style }
            else if active { active_style }
            else { inactive_style }
        );

        rsx! {
            button {
                disabled: !enabled,
                style: "{style}",
                onclick: move |_| { if enabled { on_view_change.call(target.clone()); } },
                "{label}"
            }
        }
    };

    rsx! {
        nav {
            style: "background: white; padding: 1.5rem 2rem; box-shadow: 0 4px 20px rgba(0,0,0,0.1);",

            div {
                style: "max-width: 1400px; margin: 0 auto; \
                       display: flex; justify-content: space-between; align-items: center;",

                // 左：Logo + 页面切换
                div {
                    style: "display: flex; align-items: center; gap: 2rem;",

                    h1 {
                        style: "margin: 0; font-size: 1.75rem; font-weight: 700; \
                               background: linear-gradient(135deg, #667eea 0%, #764ba2 100%); \
                               -webkit-background-clip: text; -webkit-text-fill-color: transparent; \
                               background-clip: text;",
                        "📊 股票投资系统"
                    }

                    div { style: "display: flex; gap: 0.5rem;",
                        {nav_btn("仪表盘", ViewMode::Dashboard, matches!(view_mode, ViewMode::Dashboard), true)}
                        {nav_btn("图表分析", ViewMode::Chart, matches!(view_mode, ViewMode::Chart), chart_enabled)}
                    }
                }

                // 右：数据源选择器 + 实时状态
                div {
                    style: "display: flex; align-items: center; gap: 1rem;",

                    DataSourceSelector {
                        current: data_source,
                        on_change: move |s| on_source_change.call(s),
                    }

                    div {
                        style: "display: flex; align-items: center; gap: 0.5rem; \
                               color: #718096; font-size: 0.875rem;",
                        span { "实时更新" }
                        div {
                            style: "width: 8px; height: 8px; border-radius: 50%; \
                                   background: #48bb78; animation: pulse 2s infinite;",
                        }
                    }
                }
            }
        }
    }
}
