use serde::{Deserialize, Serialize};

/// 展示层视图模式（纯 UI 状态，与领域无关）
#[derive(Debug, Clone, PartialEq)]
pub enum ViewMode {
    Dashboard,
    Chart,
    Settings,
}

/// 图表类型（展示层选项）
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum ChartType {
    Candlestick,
    Line,
    Volume,
}

impl std::fmt::Display for ChartType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ChartType::Candlestick => write!(f, "日K线"),
            ChartType::Line        => write!(f, "趋势图"),
            ChartType::Volume      => write!(f, "成交量"),
        }
    }
}
