# Aurora Tauri Desktop Shell — Design Spec

**日期**: 2026-06-24
**状态**: 已确认
**版本**: 1.0.0
**Tauri 版本**: v2（最新稳定版）

---

## 1. 目标

将 Aurora 从纯 CLI 工具升级为 Tauri 桌面应用，保留 CLI 模式，新增 React + TypeScript 图形界面。

**北极星**：用户双击图标 → 看到注意力仪表盘 → 输入信号 → 看到 ASI + 冲突 → 可导航所有页面。

---

## 2. 架构决策

### 2.1 三层分离

```
┌─────────────────────────────────────────────────────────┐
│  React UI (ui/)          ← 纯前端，只通过 IPC 通信      │
├─────────────────────────────────────────────────────────┤
│  Tauri IPC (src-tauri/)   ← #[tauri::command] 薄壳       │
├─────────────────────────────────────────────────────────┤
│  Aurora Core (aurora/)    ← 所有业务逻辑，不改 BC/pipeline│
└─────────────────────────────────────────────────────────┘
```

### 2.2 核心 Facade：`AuroraApp`

新增 `aurora/src/app.rs`，将当前 `main.rs` 内联逻辑抽成可复用结构体：

```rust
pub struct AuroraApp {
    db: Database,
    contacts: Vec<ContactProfile>,
}

pub struct AnalysisInput {
    pub spec: SignalSpec,
    pub frequency_threshold: f64,
    pub user_feels_normal: bool,
}

pub struct AppOutput {
    pub analysis_report: AnalysisReport,
    pub attention_outcome: AttentionOutcome,
    pub html: String,
    pub json: String,
}

impl AuroraApp {
    pub fn new(db_path: Option<&Path>) -> Result<Self>
    pub fn load_contacts(&mut self, data_source: &Path) -> Result<usize>
    pub fn run_pipeline(&self, input: AnalysisInput) -> Result<AppOutput>
}
```

**约束**：
- `run_pipeline` 消费 `Database` 所有权（因为 `run_attention` 需要）。每次调用后需重建 `AuroraApp`。
- Tauri 端用 `Mutex<Option<PathBuf>>` 管理 db_path，每次 Command 调用创建临时 `AuroraApp`。
- 跨调用状态由 SQLite 持久化承载（`~/.aurora/data/aurora.db`）。

### 2.3 现有代码改动

| 文件 | 改动 |
|------|------|
| `aurora/src/app.rs` | **新增** — AuroraApp facade |
| `aurora/src/main.rs` | **重写** — 缩减为 ~15 行 CLI thin shell，调用 AuroraApp |
| `aurora/src/lib.rs` | **微调** — 添加 `pub mod app;` |
| `aurora/src/pipeline/` | **不改** |
| `aurora/src/bc/` | **不改** |
| `aurora/src/db/` | **不改** |
| `aurora/src/wavelet/` | **不改** |
| `aurora/src/ingest/` | **不改** |
| `aurora/src/cli.rs` | **不改** |

---

## 3. Tauri IPC 设计

### 3.1 Commands（请求-响应）

| Command | 参数 | 返回值 | 说明 |
|---------|------|--------|------|
| `run_analysis` | `AnalysisInput` | `AppOutput` | 完整 pipeline 执行 |
| `load_contacts` | `String` (path) | `usize` | 加载联系人，返回数量 |
| `get_asi_history` | `Option<u32>` (days) | `Vec<ASIDataPoint>` | 从 SQLite 查询 ASI 历史 |
| `get_audit_log` | `AuditFilter` | `Vec<AuditEntry>` | 审计日志查询 |
| `get_settings` | — | `AppSettings` | 读取当前设置 |
| `update_settings` | `AppSettings` | `()` | 保存设置 |
| `export_data` | `String` (format) | `String` (path) | 导出 JSON/SQLite |

### 3.2 Events（服务端推送）

| Event | Payload | 触发时机 |
|-------|---------|----------|
| `analysis_progress` | `{ step, pct }` | FFT 分析进度 |
| `attention_reminder` | `{ cmd_type, reason }` | 注意力提醒触发 |
| `conflict_detected` | `{ conflict_type, reason }` | 跨域冲突检测到 |

### 3.3 Tauri State

```rust
struct AuroraState {
    db_path: Mutex<Option<PathBuf>>,
}
```

### 3.4 前端调用封装

```typescript
// ui/src/hooks/useAurora.ts
export function useAurora() {
  const runAnalysis = async (input: AnalysisInput): Promise<AppOutput> =>
    invoke('run_analysis', { input });

  const onProgress = (cb: (p: ProgressPayload) => void) =>
    listen('analysis_progress', (e) => cb(e.payload));

  // ...
}
```

---

## 4. 前端设计

### 4.1 技术栈

| 层 | 选择 |
|----|------|
| 框架 | React 18 + TypeScript |
| 构建 | Vite 5 |
| 路由 | react-router-dom (HashRouter) |
| 图表 | 雷达图/折线图用 Canvas API 手写（零依赖，符合本地优先） |
| 样式 | CSS Modules（零运行时，与 CSP 兼容） |
| 状态 | React Context + useReducer |

### 4.2 路由

| 路径 | 组件 | 说明 |
|------|------|------|
| `/` | `Dashboard` | 默认首页 |
| `/conflicts` | `ConflictPanel` | 冲突列表 |
| `/analyze` | `SignalAnalyzer` | 信号输入 + 分析 |
| `/audit` | `AuditLog` | 审计日志（骨架） |
| `/settings` | `SettingsPanel` | 设置（骨架） |

### 4.3 组件树

```
App
├── Sidebar
│   ├── NavItem("仪表盘", "/")
│   ├── NavItem("冲突面板", "/conflicts")
│   ├── NavItem("信号分析", "/analyze")
│   ├── NavItem("审计日志", "/audit")
│   └── NavItem("设置", "/settings")
│
├── Dashboard
│   ├── ASIGauge              ← 圆环仪表：ASI 值 + 趋势
│   ├── FrameRadar            ← 雷达图：Frame 权重分布
│   ├── ReminderTimeline      ← 时间线：最近提醒
│   └── QuickStats            ← 卡片：分析次数/Hold次数/联系人
│
├── ConflictPanel
│   ├── ConflictCard[]        ← 卡片列表
│   └── ConflictDetail        ← 展开详情
│
├── SignalAnalyzer
│   ├── SignalForm            ← freq/sample_rate/duration/noise
│   ├── FrequencyChart        ← FFT 频谱折线图
│   └── DecisionResult        ← True/Hold/False + Phase
│
├── AuditLog                  ← 骨架（M1 后续）
│
└── SettingsPanel             ← 骨架（M1 后续）
    ├── DataSourceToggle
    ├── ThresholdSlider
    └── ExportButton
```

### 4.4 全局状态

```typescript
interface AuroraState {
  currentOutput: AppOutput | null;
  contacts: ContactProfile[];
  settings: AppSettings;
  isLoading: boolean;
  progress: ProgressPayload | null;
}
```

---

## 5. 文件结构

```
aurora/
├── src/
│   ├── app.rs                    ← NEW: AuroraApp facade
│   ├── main.rs                   ← REWRITE: thin CLI shell
│   ├── lib.rs                    ← TWEAK: add `pub mod app;`
│   ├── cli.rs                    ← UNCHANGED
│   ├── pipeline/                 ← UNCHANGED
│   ├── bc/                       ← UNCHANGED
│   ├── db/                       ← UNCHANGED
│   ├── wavelet/                  ← UNCHANGED
│   └── ingest/                   ← UNCHANGED
│
├── src-tauri/                    ← NEW: Tauri Rust 后端
│   ├── Cargo.toml
│   ├── tauri.conf.json
│   ├── build.rs
│   ├── src/
│   │   ├── main.rs               ← Tauri 入口 + commands
│   │   └── commands/
│   │       ├── mod.rs
│   │       ├── analysis.rs
│   │       ├── audit.rs
│   │       └── settings.rs
│   └── icons/
│
├── ui/                           ← NEW: React + TypeScript 前端
│   ├── package.json
│   ├── tsconfig.json
│   ├── vite.config.ts
│   ├── index.html
│   └── src/
│       ├── main.tsx
│       ├── App.tsx
│       ├── components/
│       │   ├── layout/
│       │   │   ├── Sidebar.tsx
│       │   │   └── AppShell.tsx
│       │   ├── dashboard/
│       │   │   ├── Dashboard.tsx
│       │   │   ├── ASIGauge.tsx
│       │   │   ├── FrameRadar.tsx
│       │   │   └── ReminderTimeline.tsx
│       │   ├── conflicts/
│       │   │   ├── ConflictPanel.tsx
│       │   │   └── ConflictCard.tsx
│       │   ├── analyzer/
│       │   │   ├── SignalAnalyzer.tsx
│       │   │   ├── SignalForm.tsx
│       │   │   └── DecisionResult.tsx
│       │   ├── audit/
│       │   │   ├── AuditLog.tsx
│       │   │   └── AuditTable.tsx
│       │   └── settings/
│       │       └── SettingsPanel.tsx
│       ├── hooks/
│       │   ├── useAurora.ts
│       │   └── useEvents.ts
│       ├── context/
│       │   └── AuroraContext.tsx
│       └── types/
│           └── aurora.ts
```

---

## 6. 安全模型

### 6.1 CSP

```
default-src 'self'; style-src 'self' 'unsafe-inline'
```

- 禁止外部 CDN
- 禁止 `connect-src` 外部域名
- 符合 CHARTER 第 5 条：无网络依赖

### 6.2 文件系统权限

限定在 `$HOME/.aurora/` 范围内。Tauri fs allowlist 只开放此路径。

### 6.3 不变约束

- `#![forbid(unsafe_code)]` — Tauri crate 同样强制
- 不上云、不联网、不收集数据
- 所有数据本地 SQLite 存储

---

## 7. 分步交付

| Step | 内容 | 验证 |
|------|------|------|
| 1 | `AuroraApp` facade 抽离 + CLI main.rs 重写 | `cargo test --workspace` + CLI 仍正常 |
| 2 | Tauri 项目骨架 + React 脚手架 | `cargo build` + `npm run dev` |
| 3 | IPC 层：Commands + hooks | 前端 invoke 能拿到 AppOutput |
| 4 | Dashboard 页面（ASI + 雷达 + 时间线） | 合成信号 → Dashboard 显示数据 |
| 5 | 其余页面（Conflicts/Analyzer/Settings 骨架） | 所有页面可导航 |
| 6 | 集成测试 + 文档更新 | `cargo test --workspace` + clippy + fmt clean |

---

## 8. 不在本次范围

- 真实邮件/日历数据源（DataSource trait 已就绪，M1 后续）
- 实时注意力提醒推送（Event 系统预留，M1 后续）
- 审计日志查询 UI 完整实现（骨架已建，M1 后续）
- 数据导出功能完整实现（骨架已建，M1 后续）
- 节奏报告 PDF 导出（P1，M1 后续）
- 应用图标/打包配置（M1 发布前）
- Tauri 窗口自定义/系统托盘（M1 发布前）

---

*此文档为 Aurora Tauri 桌面应用骨架的架构设计。所有技术决策以 CHARTER.md 为最高判据。*
