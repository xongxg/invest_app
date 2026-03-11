/// 领域服务：图表计算（与任何框架无关的纯逻辑）
pub struct ChartDomainService;

impl ChartDomainService {
    /// 计算 N 日移动平均线；数据不足时返回 None（ECharts 会自动跳过）
    pub fn calculate_ma(prices: &[f64], period: usize) -> Vec<Option<f64>> {
        (0..prices.len())
            .map(|i| {
                if i < period - 1 {
                    None
                } else {
                    let sum: f64 = prices[i - period + 1..=i].iter().sum();
                    Some((sum / period as f64 * 100.0).round() / 100.0)
                }
            })
            .collect()
    }
}
