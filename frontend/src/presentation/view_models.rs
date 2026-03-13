use serde::{Deserialize, Serialize};

/// 展示层视图模式（纯 UI 状态，与领域无关）
#[derive(Debug, Clone, PartialEq)]
pub enum ViewMode {
    Dashboard,
    Chart,
    ServerConfig,   // 服务配置（后端地址 + 数据目录）
    ApiKeys,
    DataSync,
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
            ChartType::Candlestick => write!(f, "K线"),
            ChartType::Line        => write!(f, "趋势图"),
            ChartType::Volume      => write!(f, "成交量"),
        }
    }
}

/// K 线周期
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum KlinePeriod {
    Daily,
    Monthly,
    Quarterly,
    Yearly,
}

impl KlinePeriod {
    /// 需要拉取的原始日线天数
    pub fn fetch_days(self) -> usize {
        match self {
            KlinePeriod::Daily     => 180,
            KlinePeriod::Monthly   => 365 * 3,
            KlinePeriod::Quarterly => 365 * 6,
            KlinePeriod::Yearly    => 365 * 15,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            KlinePeriod::Daily     => "日K",
            KlinePeriod::Monthly   => "月K",
            KlinePeriod::Quarterly => "季K",
            KlinePeriod::Yearly    => "年K",
        }
    }
}
