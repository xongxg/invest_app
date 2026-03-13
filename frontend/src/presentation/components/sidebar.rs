use dioxus::prelude::*;
use crate::application::DataSource;
use crate::presentation::view_models::ViewMode;

#[component]
pub fn Sidebar(
    view_mode: ViewMode,
    on_view_change: EventHandler<ViewMode>,
    data_source: DataSource,
    on_source_change: EventHandler<DataSource>,
) -> Element {
    let settings_in_scope = matches!(
        view_mode,
        ViewMode::ServerConfig | ViewMode::ApiKeys | ViewMode::DataSync
    );
    let mut settings_open = use_signal(|| settings_in_scope);

    use_effect(move || {
        if settings_in_scope {
            settings_open.set(true);
        }
    });

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

            // ── Nav body ─────────────────────────────────────────────────────
            nav {
                style: "padding: 1rem 0.75rem; flex: 1;",

                SectionLabel { title: "主功能" }

                NavItem {
                    icon: "⬛",
                    label: "市场全景",
                    active: matches!(view_mode, ViewMode::Dashboard),
                    enabled: true,
                    expand_arrow: None,
                    on_click: move |_| on_view_change.call(ViewMode::Dashboard),
                }

                // ── 系统 ─────────────────────────────────────────────────────
                div { style: "margin-top: 1.5rem;",
                    SectionLabel { title: "系统" }

                    // 设置：仅展开/折叠，不导航
                    NavItem {
                        icon: "⚙",
                        label: "设置",
                        active: false,
                        enabled: true,
                        expand_arrow: Some(settings_open()),
                        on_click: move |_| settings_open.set(!settings_open()),
                    }

                    if settings_open() {
                        SubNavItem {
                            icon: "🖥",
                            label: "服务配置",
                            active: matches!(view_mode, ViewMode::ServerConfig),
                            on_click: move |_| on_view_change.call(ViewMode::ServerConfig),
                        }
                        SubNavItem {
                            icon: "🔑",
                            label: "API Key 存储",
                            active: matches!(view_mode, ViewMode::ApiKeys),
                            on_click: move |_| on_view_change.call(ViewMode::ApiKeys),
                        }
                        SubNavItem {
                            icon: "🔄",
                            label: "数据同步",
                            active: matches!(view_mode, ViewMode::DataSync),
                            on_click: move |_| on_view_change.call(ViewMode::DataSync),
                        }
                    }
                }
            }

            // ── Status ───────────────────────────────────────────────────────
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

// ── Sub-components ────────────────────────────────────────────────────────────

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
    expand_arrow: Option<bool>,
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
            span { style: "flex: 1;", "{label}" }
            if let Some(open) = expand_arrow {
                {
                    let rotate = if open { "rotate(90deg)" } else { "rotate(0deg)" };
                    rsx! {
                        span {
                            style: "font-size: 0.7rem; color: #475569; transition: transform 0.2s; \
                                    display: inline-block; transform: {rotate};",
                            "▶"
                        }
                    }
                }
            }
        }
    }
}

#[component]
fn SubNavItem(
    icon: &'static str,
    label: &'static str,
    active: bool,
    on_click: EventHandler<()>,
) -> Element {
    let bg = if active { "background: #1e3a8a; color: #bfdbfe;" }
             else      { "background: transparent; color: #64748b;" };
    let style = format!(
        "width: 100%; display: flex; align-items: center; gap: 0.625rem; \
         padding: 0.5rem 0.75rem 0.5rem 2.25rem; border-radius: 6px; border: none; \
         text-align: left; font-size: 0.85rem; font-weight: 400; \
         cursor: pointer; transition: background 0.2s; margin-bottom: 0.125rem; {bg}"
    );
    rsx! {
        button {
            style: "{style}",
            onclick: move |_| on_click.call(()),
            span { style: "font-size: 0.8rem;", "{icon}" }
            span { "{label}" }
        }
    }
}
