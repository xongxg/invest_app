use serde_json::json;
use crate::domain::entities::OHLCData;
use crate::domain::services::ChartDomainService;

/// 将 ECharts option JSON 包装为自包含 HTML（通过 iframe srcdoc 嵌入）
fn make_chart_html(option_json: &str) -> String {
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
    chart.setOption({option_json});
    window.addEventListener('resize', function() {{ chart.resize(); }});
  </script>
</body>
</html>"#
    )
}

/// 生成 K 线图 + 成交量复合图
pub fn render_candlestick(symbol: &str, data: &[OHLCData]) -> String {
    let dates: Vec<&str>       = data.iter().map(|d| d.date.as_str()).collect();
    let candles: Vec<[f64; 4]> = data.iter().map(|d| [d.open, d.close, d.low, d.high]).collect();
    let volumes: Vec<f64>      = data.iter().map(|d| d.volume as f64).collect();

    let option = json!({
        "title":   { "text": format!("{} - 日K线", symbol), "left": "center" },
        "tooltip": { "trigger": "axis", "axisPointer": { "type": "cross" } },
        "legend":  { "data": ["日K", "成交量"], "top": "30" },
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
                "name": "日K", "type": "candlestick", "data": candles,
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

    make_chart_html(&option.to_string())
}

/// 生成趋势折线图（含 MA5/MA10/MA20）
pub fn render_line(symbol: &str, data: &[OHLCData]) -> String {
    let dates:  Vec<&str>         = data.iter().map(|d| d.date.as_str()).collect();
    let prices: Vec<f64>          = data.iter().map(|d| d.close).collect();
    let ma5:    Vec<Option<f64>>  = ChartDomainService::calculate_ma(&prices, 5);
    let ma10:   Vec<Option<f64>>  = ChartDomainService::calculate_ma(&prices, 10);
    let ma20:   Vec<Option<f64>>  = ChartDomainService::calculate_ma(&prices, 20);

    let option = json!({
        "title":   { "text": format!("{} - 价格趋势", symbol), "left": "center" },
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

    make_chart_html(&option.to_string())
}

/// 生成成交量柱状图
pub fn render_volume(symbol: &str, data: &[OHLCData]) -> String {
    let dates:   Vec<&str> = data.iter().map(|d| d.date.as_str()).collect();
    let volumes: Vec<f64>  = data.iter().map(|d| d.volume as f64).collect();

    let option = json!({
        "title":   { "text": format!("{} - 成交量", symbol), "left": "center" },
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

    make_chart_html(&option.to_string())
}
