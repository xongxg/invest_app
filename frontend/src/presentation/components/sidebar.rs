use dioxus::prelude::*;
use crate::application::{DataSource, TushareConfig, YahooConfig};
use crate::infrastructure::ConfigStorage;
use crate::presentation::view_models::ViewMode;

#[component]
pub fn Sidebar(
    view_mode: ViewMode,
    on_view_change: EventHandler<ViewMode>,
    data_source: DataSource,
    on_source_change: EventHandler<DataSource>,
    on_settings: EventHandler<()>,
) -> Element {
    let mut source_open = use_signal(|| false);

    rsx! {
        aside {
            style: "width: 260px; min-width: 260px; background: #0f172a; \
                   min-height: 100vh; display: flex; flex-direction: column; \
                   color: #cbd5e1; font-family: inherit; overflow-y: auto;",

            // ── Brand ────────────────────────────────────────────────────────
            div {
                style: "padding: 1.75rem 1.5rem 1.25rem; border-bottom: 1px solid #1e293b; \
                       flex-shrink: 0;",
                div {
                    style: "font-size: 1.25rem; font-weight: 700; \
                           background: linear-gradient(135deg, #818cf8 0%, #a78bfa 100%); \
                           -webkit-background-clip: text; -webkit-text-fill-color: transparent; \
                           background-clip: text;",
                    "📊 股票投资系统"
                }
                div {
                    style: "margin-top: 0.25rem; font-size: 0.75rem; color: #475569;",
                    "Stock Analysis Platform"
                }
            }

            // ── Scrollable body ───────────────────────────────────────────────
            nav {
                style: "padding: 1rem 0.75rem; flex: 1;",

                // ── Navigation ───────────────────────────────────────────────
                SectionLabel { title: "主功能" }

                NavItem {
                    icon: "⬛",
                    label: "市场全景",
                    active: matches!(view_mode, ViewMode::Dashboard),
                    enabled: true,
                    on_click: move |_| on_view_change.call(ViewMode::Dashboard),
                }

                // ── Settings ─────────────────────────────────────────────────
                div { style: "margin-top: 1.5rem;",
                    SectionLabel { title: "系统" }
                    NavItem {
                        icon: "⚙",
                        label: "设置",
                        active: matches!(view_mode, ViewMode::Settings),
                        enabled: true,
                        on_click: move |_| on_settings.call(()),
                    }
                }
            }

            // ── Status indicator ──────────────────────────────────────────────
            div {
                style: "padding: 1rem 1.5rem; border-top: 1px solid #1e293b; \
                       display: flex; align-items: center; gap: 0.5rem; flex-shrink: 0;",
                div {
                    style: "width: 8px; height: 8px; border-radius: 50%; \
                           background: #4ade80; animation: pulse 2s infinite; flex-shrink: 0;",
                }
                span { style: "font-size: 0.75rem; color: #475569;", "实时数据更新中" }
            }
        }
    }
}

// ── Sub-components ─────────────────────────────────────────────────────────────

#[component]
fn SectionLabel(title: &'static str) -> Element {
    rsx! {
        div {
            style: "font-size: 0.6875rem; font-weight: 600; color: #475569; \
                   letter-spacing: 0.1em; text-transform: uppercase; \
                   padding: 0 0.75rem; margin-bottom: 0.5rem;",
            "{title}"
        }
    }
}

#[component]
fn NavItem(
    icon: &'static str,
    label: &'static str,
    active: bool,
    enabled: bool,
    on_click: EventHandler<()>,
) -> Element {
    let bg    = if active   { "background: #1e40af; color: #e0e7ff;" }
                else        { "background: transparent; color: #94a3b8;" };
    let extra = if !enabled { "opacity: 0.4; cursor: not-allowed;" } else { "cursor: pointer;" };
    let style = format!(
        "width: 100%; display: flex; align-items: center; gap: 0.75rem; \
         padding: 0.65rem 0.75rem; border-radius: 8px; border: none; \
         text-align: left; font-size: 0.9rem; font-weight: 500; \
         transition: background 0.2s; margin-bottom: 0.125rem; {bg} {extra}"
    );
    rsx! {
        button {
            style: "{style}",
            disabled: !enabled,
            onclick: move |_| { if enabled { on_click.call(()); } },
            span { "{icon}" }
            span { "{label}" }
        }
    }
}

#[component]
fn SourceOption(label: &'static str, active: bool, on_click: EventHandler<()>) -> Element {
    let bg = if active { "#334155" } else { "transparent" };
    rsx! {
        button {
            style: "width: 100%; text-align: left; padding: 0.6rem 1rem; \
                   border: none; background: {bg}; color: #cbd5e1; \
                   cursor: pointer; font-size: 0.8125rem; transition: background 0.2s;",
            onclick: move |_| on_click.call(()),
            "{label}"
        }
    }
}
