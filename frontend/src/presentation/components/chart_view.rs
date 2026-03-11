use dioxus::prelude::*;
use std::rc::Rc;
use crate::application::StockApplicationService;
use crate::presentation::view_models::ChartType;
use crate::presentation::charts;

#[component]
pub fn ChartView(
    service: Rc<StockApplicationService>,
    stock_symbol: String,
    chart_type: ChartType,
    on_chart_type_change: EventHandler<ChartType>,
    on_back: EventHandler<()>,
) -> Element {
    // 异步获取历史数据，symbol 或 chart_type 变化时自动重新请求
    let symbol = stock_symbol.clone();
    let chart_html = use_resource(move || {
        let svc = service.clone();
        let sym = symbol.clone();
        async move {
            let data = svc.get_stock_history(&sym, 60).await?;
            let html = match chart_type {
                ChartType::Candlestick => charts::render_candlestick(&sym, &data),
                ChartType::Line        => charts::render_line(&sym, &data),
                ChartType::Volume      => charts::render_volume(&sym, &data),
            };
            Ok::<String, crate::domain::errors::DomainError>(html)
        }
    });

    rsx! {
        div {
            style: "background: white; border-radius: 16px; padding: 2rem; box-shadow: 0 4px 20px rgba(0,0,0,0.1);",

            // 头部
            div {
                style: "display: flex; justify-content: space-between; align-items: center; margin-bottom: 2rem;",
                div {
                    style: "display: flex; align-items: center; gap: 1rem;",
                    button {
                        onclick: move |_| on_back.call(()),
                        style: "padding: 0.5rem 1rem; background: #e2e8f0; border: none; \
                               border-radius: 8px; cursor: pointer; font-weight: 600; color: #4a5568; font-size: 1rem;",
                        "← 返回"
                    }
                    h2 { style: "margin: 0; font-size: 1.75rem; font-weight: 700; color: #2d3748;",
                         "{stock_symbol} - 图表分析" }
                }
                div {
                    style: "display: flex; gap: 0.5rem;",
                    ChartTypeButton { chart_type: ChartType::Candlestick, current_type: chart_type, color: "#3b82f6",
                        on_click: move |_| on_chart_type_change.call(ChartType::Candlestick) }
                    ChartTypeButton { chart_type: ChartType::Line,        current_type: chart_type, color: "#10b981",
                        on_click: move |_| on_chart_type_change.call(ChartType::Line) }
                    ChartTypeButton { chart_type: ChartType::Volume,      current_type: chart_type, color: "#f59e0b",
                        on_click: move |_| on_chart_type_change.call(ChartType::Volume) }
                }
            }

            // 图表区域
            match &*chart_html.read() {
                None => rsx! {
                    div {
                        style: "height: 650px; display: flex; align-items: center; justify-content: center; \
                               background: #f7fafc; border-radius: 12px;",
                        div { style: "color: #718096; font-size: 1rem;", "⏳ 加载中..." }
                    }
                },
                Some(Err(e)) => rsx! {
                    div {
                        style: "height: 200px; display: flex; flex-direction: column; align-items: center; \
                               justify-content: center; background: #fff5f5; border-radius: 12px; gap: 0.5rem;",
                        div { style: "font-size: 1.5rem;", "⚠️" }
                        div { style: "color: #c53030; font-weight: 600;", "{e}" }
                    }
                },
                Some(Ok(html)) => rsx! {
                    iframe {
                        srcdoc: "{html}",
                        style: "width: 100%; height: 650px; border: none; border-radius: 12px; \
                               background: #fafafa; display: block;",
                    }
                },
            }

            // 说明
            div {
                style: "margin-top: 2rem; background: #f7fafc; border-radius: 12px; padding: 1.5rem;",
                h3 { style: "margin: 0 0 1rem 0; color: #2d3748; font-size: 1.1rem; font-weight: 600;",
                     "图表功能说明" }
                ul {
                    style: "margin: 0; padding-left: 1.5rem; color: #4a5568; line-height: 1.8;",
                    li { strong { "日K线：" } "显示每日开盘、收盘、最高、最低价格及成交量" }
                    li { strong { "趋势图：" } "价格线 + MA5 / MA10 / MA20 移动平均线" }
                    li { strong { "成交量：" } "每日成交量柱状图" }
                    li { strong { "交互：" }   "支持缩放、拖拽查看任意时间段" }
                }
            }
        }
    }
}

#[component]
fn ChartTypeButton(
    chart_type: ChartType,
    current_type: ChartType,
    color: String,
    on_click: EventHandler<()>,
) -> Element {
    let is_active = chart_type == current_type;
    let style = if is_active {
        format!("padding: 0.5rem 1.25rem; border: none; border-radius: 8px; font-weight: 600; \
                 cursor: pointer; font-size: 1rem; background: {color}; color: white; \
                 box-shadow: 0 4px 12px {color}40;")
    } else {
        "padding: 0.5rem 1.25rem; border: none; border-radius: 8px; font-weight: 600; \
         cursor: pointer; font-size: 1rem; background: #e2e8f0; color: #4a5568;".to_string()
    };
    rsx! {
        button { onclick: move |_| on_click.call(()), style: "{style}", "{chart_type}" }
    }
}
