use serde_json::json;
use crate::domain::entities::OHLCData;
use crate::domain::services::ChartDomainService;
use crate::presentation::view_models::KlinePeriod;

/// 将 ECharts option JSON 包装为自包含 HTML（通过 iframe srcdoc 嵌入）
/// extra_js 会在 setOption 之前插入，用于注入无法 JSON 序列化的 JS 函数（如 formatter）
fn make_chart_html(option_json: &str, extra_js: &str) -> String {
    format!(
        r#"<!DOCTYPE html>
<html>
<head>
  <meta charset="utf-8">
  <meta name="viewport" content="width=device-width, initial-scale=1">
  <script src="https://cdn.jsdelivr.net/npm/echarts@5/dist/echarts.min.js"></script>
  <style>
    * {{ margin: 0; padding: 0; box-sizing: border-box; }}
    body {{ background: #fafafa; }}
    #chart {{ width: 100%; height: 100vh; }}
  </style>
</head>
<body>
  <div id="chart"></div>
  <script>
    var chart = echarts.init(document.getElementById('chart'));
    var option = {option_json};
    {extra_js}
    chart.setOption(option);
    window.addEventListener('resize', function() {{ chart.resize(); }});
  </script>
</body>
</html>"#
    )
}

// ── 周期聚合 ──────────────────────────────────────────────────────────────────

/// 将日线数据按周期聚合为 OHLC bar 列表
pub fn aggregate(data: &[OHLCData], period: KlinePeriod) -> Vec<OHLCData> {
    if matches!(period, KlinePeriod::Daily) {
        return data.to_vec();
    }

    // 按周期 key 分组（保持原始顺序，data 应按日期升序排列）
    let mut groups: Vec<(String, Vec<&OHLCData>)> = Vec::new();

    for bar in data {
        let key = period_key(&bar.date, period);
        if let Some(last) = groups.last_mut() {
            if last.0 == key {
                last.1.push(bar);
                continue;
            }
        }
        groups.push((key, vec![bar]));
    }

    groups
        .into_iter()
        .map(|(key, bars)| OHLCData {
            date:   key,
            open:   bars.first().unwrap().open,
            close:  bars.last().unwrap().close,
            high:   bars.iter().map(|b| b.high).fold(f64::NEG_INFINITY, f64::max),
            low:    bars.iter().map(|b| b.low).fold(f64::INFINITY, f64::min),
            volume: bars.iter().map(|b| b.volume).sum(),
        })
        .collect()
}

/// "YYYY-MM-DD" → 周期标签
fn period_key(date: &str, period: KlinePeriod) -> String {
    // date 格式固定为 YYYY-MM-DD，直接按字符切分
    let year  = &date[..4];
    let month: u32 = date[5..7].parse().unwrap_or(1);

    match period {
        KlinePeriod::Daily     => date.to_string(),
        KlinePeriod::Monthly   => format!("{}-{:02}", year, month),
        KlinePeriod::Quarterly => {
            let q = (month - 1) / 3 + 1;
            format!("{}-Q{}", year, q)
        }
        KlinePeriod::Yearly    => year.to_string(),
    }
}

// ── 图表渲染 ──────────────────────────────────────────────────────────────────

/// 生成 K 线图 + 成交量复合图
pub fn render_candlestick(symbol: &str, data: &[OHLCData], period: KlinePeriod) -> String {
    let data = aggregate(data, period);
    let dates:   Vec<&str>    = data.iter().map(|d| d.date.as_str()).collect();
    let candles: Vec<[f64;4]> = data.iter().map(|d| [d.open, d.close, d.low, d.high]).collect();
    let volumes: Vec<f64>     = data.iter().map(|d| d.volume as f64).collect();
    let title = format!("{} - {}线", symbol, period.label());

    let option = json!({
        "title":   { "text": title, "left": "center" },
        "tooltip": {
            "trigger": "axis",
            "axisPointer": { "type": "cross" }
        },
        "legend":  { "data": [period.label(), "成交量"], "top": "30" },
        "grid": [
            { "left": "10%", "right": "10%", "height": "50%" },
            { "left": "10%", "right": "10%", "top": "70%", "height": "15%" }
        ],
        "xAxis": [
            { "type": "category", "data": dates, "scale": true, "boundaryGap": false, "gridIndex": 0 },
            { "type": "category", "data": dates, "scale": true, "gridIndex": 1 }
        ],
        "yAxis": [
            { "scale": true, "gridIndex": 0 },
            { "scale": true, "gridIndex": 1, "splitNumber": 2 }
        ],
        "dataZoom": [
            { "type": "inside", "xAxisIndex": [0, 1], "start": 60, "end": 100 },
            { "show": true,     "xAxisIndex": [0, 1], "type": "slider", "top": "90%", "start": 60, "end": 100 }
        ],
        "series": [
            {
                "name": period.label(), "type": "candlestick", "data": candles,
                "xAxisIndex": 0, "yAxisIndex": 0,
                "itemStyle": {
                    "color": "#ef5350", "color0": "#26a69a",
                    "borderColor": "#ef5350", "borderColor0": "#26a69a"
                }
            },
            {
                "name": "成交量", "type": "bar", "data": volumes,
                "xAxisIndex": 1, "yAxisIndex": 1
            }
        ]
    });

    let extra_js = r#"option.tooltip.formatter = function(params) {
        var p = params[0];
        if (!p) return '';
        var d = Array.isArray(p.data) ? p.data : [];
        return p.name
            + '<br/>开盘: ' + d[0]
            + '<br/>收盘: ' + d[1]
            + '<br/>最低: ' + d[2]
            + '<br/>最高: ' + d[3];
    };"#;

    make_chart_html(&option.to_string(), extra_js)
}

/// 生成趋势折线图（含 MA5/MA10/MA20）
pub fn render_line(symbol: &str, data: &[OHLCData], period: KlinePeriod) -> String {
    let data   = aggregate(data, period);
    let dates:  Vec<&str>        = data.iter().map(|d| d.date.as_str()).collect();
    let prices: Vec<f64>         = data.iter().map(|d| d.close).collect();
    let ma5:    Vec<Option<f64>> = ChartDomainService::calculate_ma(&prices, 5);
    let ma10:   Vec<Option<f64>> = ChartDomainService::calculate_ma(&prices, 10);
    let ma20:   Vec<Option<f64>> = ChartDomainService::calculate_ma(&prices, 20);
    let title = format!("{} - {}价格趋势", symbol, period.label());

    let option = json!({
        "title":   { "text": title, "left": "center" },
        "tooltip": { "trigger": "axis", "axisPointer": { "type": "cross" } },
        "legend":  { "data": ["价格", "MA5", "MA10", "MA20"], "top": "30" },
        "grid":    { "left": "3%", "right": "4%", "bottom": "15%", "containLabel": true },
        "xAxis":   { "type": "category", "boundaryGap": false, "data": dates },
        "yAxis":   { "type": "value", "scale": true },
        "dataZoom": [
            { "type": "inside", "start": 0, "end": 100 },
            { "show": true, "type": "slider", "start": 0, "end": 100 }
        ],
        "series": [
            { "name": "价格",  "type": "line", "data": prices, "smooth": 0.3, "symbolSize": 0,
              "lineStyle": { "width": 2, "color": "#1976d2" }, "areaStyle": { "opacity": 0.1 } },
            { "name": "MA5",   "type": "line", "data": ma5,    "smooth": 0.3, "symbolSize": 0,
              "lineStyle": { "width": 1.5, "color": "#ff9800" } },
            { "name": "MA10",  "type": "line", "data": ma10,   "smooth": 0.3, "symbolSize": 0,
              "lineStyle": { "width": 1.5, "color": "#4caf50" } },
            { "name": "MA20",  "type": "line", "data": ma20,   "smooth": 0.3, "symbolSize": 0,
              "lineStyle": { "width": 1.5, "color": "#9c27b0" } }
        ]
    });

    make_chart_html(&option.to_string(), "")
}

/// 生成成交量柱状图
pub fn render_volume(symbol: &str, data: &[OHLCData], period: KlinePeriod) -> String {
    let data   = aggregate(data, period);
    let dates:   Vec<&str> = data.iter().map(|d| d.date.as_str()).collect();
    let volumes: Vec<f64>  = data.iter().map(|d| d.volume as f64).collect();
    let title = format!("{} - {}成交量", symbol, period.label());

    let option = json!({
        "title":   { "text": title, "left": "center" },
        "tooltip": { "trigger": "axis" },
        "grid":    { "left": "3%", "right": "4%", "bottom": "15%", "containLabel": true },
        "xAxis":   { "type": "category", "data": dates },
        "yAxis":   { "type": "value" },
        "dataZoom": [
            { "type": "inside", "start": 0, "end": 100 },
            { "show": true, "type": "slider", "start": 0, "end": 100 }
        ],
        "series": [
            { "name": "成交量", "type": "bar", "data": volumes,
              "itemStyle": { "color": "#667eea" } }
        ]
    });

    make_chart_html(&option.to_string(), "")
}
