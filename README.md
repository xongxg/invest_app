# 股票投资系统

基于 Rust 全栈开发的个人股票投资分析平台。前端使用 Dioxus 0.7（WASM），后端使用 Axum，支持 A 股（Tushare Pro）和美股/港股（Yahoo Finance）数据源。

## 功能

- **市场全景**：全球指数 + 个股行情卡片，每 5 秒自动刷新
- **图表分析**：日 K 线 / 趋势线 / 成交量，基于 ECharts（通过 iframe 渲染）
- **数据同步**：批量拉取并缓存历史 K 线、ETF 净值、持仓等数据到 Apache Arrow 本地存储
- **设置**：管理 Tushare Token、Yahoo API Key、Claude/OpenAI Key 等（仅存于浏览器 localStorage）

## 项目结构

```
invest_app/
├── frontend/                  # Dioxus WASM 前端
│   └── src/
│       ├── domain/            # 实体 & 仓储 trait
│       ├── application/       # 应用服务 & 数据源枚举
│       ├── infrastructure/    # 仓储实现（Mock / BackendApi）& localStorage
│       └── presentation/      # Dioxus 组件 & 视图模型
└── backend/
    ├── domain/                # 领域实体 & 端口 trait
    ├── application/           # 用例服务（StockAppService / EtfAppService）
    ├── storage/               # Apache Arrow 列式本地缓存
    ├── provider-tushare/      # Tushare Pro API 适配器
    ├── provider-yahoo/        # Yahoo Finance API 适配器
    └── gateway/               # Axum HTTP 网关（入口）
```

## 快速开始

### 环境要求

| 工具 | 版本要求 | 安装 |
|------|----------|------|
| Rust | 1.75+ | `curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs \| sh` |
| wasm32 target | — | `rustup target add wasm32-unknown-unknown` |
| Dioxus CLI | 0.7+ | `cargo install dioxus-cli` |

### 模式一：纯前端（模拟数据，无需后端）

最简单的启动方式，数据为本地随机生成，无需任何 API Key 或后端服务：

```bash
# 克隆项目
git clone <repo-url>
cd invest_app

# 启动前端开发服务器（端口 8082）
make dev
# 或等价命令
dx serve --platform web --port 8082 --package stock-frontend
```

打开浏览器访问 `http://localhost:8082`，在左侧侧边栏默认已选「模拟数据」模式。

### 模式二：连接后端（真实数据）

需要 Tushare Pro Token 或 Yahoo Finance API Key。

**第一步：启动后端 Gateway**

```bash
# 在项目根目录
cargo run -p stock-gateway
```

后端默认监听 `http://localhost:3000`，启动后会输出服务地址。

**第二步：启动前端**

```bash
# 新开一个终端
make dev
```

**第三步：配置**

浏览器打开 `http://localhost:8082`，点击左侧「⚙ 设置」：

- **Backend URL**：`http://localhost:3000`（后端地址）
- **数据目录**（Data Directory）：后端存储缓存和密钥的根目录，默认 `data`（相对于项目根目录），修改后需重启后端生效

然后点击「🔑 API Key 存储」，填写：

- **Tushare Pro Token**：从 [tushare.pro](https://tushare.pro) 注册获取
- **Yahoo Finance API Key**：公开接口留空即可

API Key 通过 AES-256-GCM 加密后存储在后端 `data/keys.json`，主密钥保存在 `data/master.key`（也可通过环境变量 `STOCK_MASTER_KEY` 注入 64 位 hex 字符串）。

配置完成后，在左侧侧边栏切换数据源。

### 生产构建

```bash
# 前端：编译为优化后的 WASM + 静态资源
make build
# 等价命令
dx build --platform web --release --package stock-frontend

# 后端：编译为原生二进制
cargo build -p stock-gateway --release
./target/release/stock-gateway
```

## 后端 API

后端 Gateway 运行后提供以下接口（前缀 `/api`）：

| 方法 | 路由 | 说明 | 关键参数 |
|------|------|------|----------|
| GET | `/api/stocks` | 股票/指数行情列表 | `source=tushare\|yahoo`，`symbols=...` |
| GET | `/api/history` | 个股 OHLCV 历史 K 线 | `source`，`symbol`，`days` |
| GET | `/api/etfs` | ETF 行情列表 | `symbols=510300.SH,...` |
| GET | `/api/etf/history` | ETF 历史 K 线 | `symbol`，`days` |
| GET | `/api/etf/nav` | ETF 净值历史 | `symbol`，`days` |
| GET | `/api/etf/basic` | ETF 基本信息 | `symbols` |
| GET | `/api/etf/daily` | ETF 日线详情 | `symbol`，`days` |
| GET | `/api/etf/portfolio` | ETF 持仓明细 | `symbol`，`period` |
| GET | `/api/etf/trade` | ETF 申赎份额 | `symbol`，`days` |
| GET | `/api/etf/dividend` | ETF 分红记录 | `symbol` |
| GET | `/api/etf/index` | 跟踪指数日线 | `index_code`，`days` |
| GET | `/api/health` | 服务健康检查 | — |

Tushare 接口需在请求头携带 `x-tushare-token`；Yahoo 接口可选携带 `x-yahoo-api-key`。

## 数据源说明

| 数据源 | 覆盖范围 | 是否免费 | CORS 限制 |
|--------|----------|----------|-----------|
| 模拟数据 | 本地随机，仅演示 | 是 | 无 |
| Tushare Pro | A 股、ETF、指数 | 注册后有积分限额 | 需后端代理 |
| Yahoo Finance | 美股、港股、全球指数 | 公开接口免费 | 需后端代理 |

> 浏览器直接请求第三方 API 会遇到 CORS 限制，生产环境务必通过后端代理转发。

## 技术栈

| 层 | 技术 |
|----|------|
| 前端框架 | [Dioxus 0.7](https://dioxuslabs.com) (WASM) |
| 前端图表 | ECharts（通过 iframe + HTML 字符串渲染） |
| 后端框架 | [Axum 0.7](https://github.com/tokio-rs/axum) |
| 异步运行时 | Tokio |
| 本地缓存 | Apache Arrow（IPC 格式） |
| HTTP 客户端 | reqwest（后端）/ gloo-net（前端 WASM） |
