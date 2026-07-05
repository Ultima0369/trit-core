# ⚡ SESSION_START — 开机导航

> **目的**：每次新会话开始时，AI 或人类协作者读此文件，30 秒内定位工作进度。
> **维护规则**：每次完成一个阶段或做出重大决策后，更新"当前进度"和"上次决策"两节。
> **版本**：1.1.0
> **最后更新**：2026-07-05（CTO 审计 L8 修复 + cost_factor 集成）

---

## 1. 项目是什么（一句话）

**Trit-Core**：三元决策引擎（Rust 库），用 True/Hold/False 替代二元逻辑。Hold 不是"不确定"，是"此刻不该塌缩，因为参考系还撑不起"。
**Aurora**：基于 Trit-Core 的**长见识输入源 + 注意力主权训练系统**（开源桌面应用）。持续喂入地理/生态/文化这些被主流决策 AI 无视的真实世界维度，撑开被默化收窄的参考系，直到用户能自己运算。

> **叙事基准**：所有文档的动机口径以 `docs/NARRATIVE_CHARTER.md` 为准。产品成功标准 = 用户毕业（不再需要系统），不是留存或收入。开源免费、不争注意力、靠自我筛选触达已觉醒用户。

---

## 2. 当前进度

| 维度 | 状态 |
|------|------|
| **Trit-Core 版本** | v0.3.0 — 单机决策引擎，5 层架构完整（Core→Meta→Hook→Adapter→Feedback） |
| **Aurora 阶段** | M1 — Sandcastle 3-column UI 完成（Monaco Editor + CesiumJS Globe + Analysis Panel）。Cosmos 视觉预设（MeshStandardMaterial + 双辉光壳 + 星空）已集成。纹理切换（Blue Marble / Topographic）通过 TopBar 按钮支持。决策结果抽屉完成（顶栏 decision 标签可点击 → 抽屉展示 phase/asi/signals/conflicts；Esc 关闭）。**Tauri 桌面打包验证通过**（NSIS `aurora_0.1.0_x64-setup.exe` 13M + MSI 15M，M1 Exit Criteria"可打包安装"达成）。41 UI 测试通过，TypeScript 干净。共享类型和事件常量已提取到 types.ts。 |
| **架构设计** | 已完成 — 见 `docs/superpowers/specs/2026-06-20-aurora-architecture-design.md` |
| **M0 剩余工作** | 无代码任务。P1 待办：作者自我验证 + 2 外部用户试用（创始人侧） |
| **M1 入口** | 桌面打包已验证 ✅。下一步：真实数据源接入（邮件/日历，M1 P0，目前仅 JSON fallback + 公开气候数据）、注意力训练闭环测试、作者自验证 |
| **地图 + 公开数据链路 (2026-06-30)** | 2D 矢量地图 (MapLibre + PMTiles) + 左侧栏目 (图层/anchor 摘要/冲突摘要/数据导出)。公开数据采集：Open-Meteo 温度异常 + NOAA GML Mauna Loa CO2 + UCDP 地缘冲突事件，本地 L2 缓存 + 后台定时刷新。anchor 接真实气候读数。图层效果借鉴 worldmonitor：分色+半径缩放+区域填充。DB 持久化到 `aurora.db` + `export_user_data` 命令 (M1"数据导出"验收 + CHARTER"不剥夺"底线)。 |

---

## 3. 上次决策（最近 3 个）

| 日期 | 决策 | 文档 |
|------|------|------|
| 2026-07-05 | 三个杠杆全部推进：L1 cost_factor 完整 CLI 集成，L2 RetrospectiveProvider + SSP1-5 + run_retrospective()，L3 MirrorFetcher + get_mirror_snapshot。L8 审计修复。clippy zero warnings。11 commits, 822 tests。 | 本轮对话 |
| 2026-07-01 | Tauri 桌面打包验证通过 — `cargo tauri build` 产出 NSIS `aurora_0.1.0_x64-setup.exe` (13M) + MSI (15M) + 裸 exe (27M)，release 编译 2m20s 零错误。19 个 Tauri 命令注册齐全（run_analysis_pipeline/get_anchor_status/get_geo_events/export_user_data 等），应用非空壳。M1 Exit Criteria "桌面应用可打包安装" 硬指标达成。 | 本轮对话 |
| 2026-07-01 | 决策结果抽屉完成 — 顶栏 decision 标签可点击 → 抽屉展示 phase/asi/signals/conflicts。Esc 键优先关设置抽屉，其次关决策抽屉，最后退出应用。冲突项布局借鉴 worldmonitor renderSignal（左色条+badge）。8 提交，41 UI 测试，终审 Ready to merge。 | 本轮对话 |
| 2026-06-30 | M1 数据持久化 + 导出完成 — 对齐 M1 Exit Criteria "数据导出"硬指标 + CHARTER "不剥夺"底线。AuroraApp 加 `export_data_json()` (5 表通用反射导出, 2 单测)。桌面应用 DB 从 in-memory 改为落盘 `aurora_data_dir/aurora.db` (持久化失败回落 in-memory)。新增 `export_user_data` Tauri 命令，前端 Blob+a[download] 下载 (零新插件依赖)。Sidebar 加"导出我的数据"按钮。 | 本轮对话 |
| 2026-06-30 | 文档系统动机校准 — 创立 `docs/NARRATIVE_CHARTER.md` 作为统一叙事基准（长见识输入源 / 反注意力 / 毕业即成功 / 地理生态文化补全决策地基）。ADR-008 从"订阅制"重写为"开源免费"。AURORA_MANIFEST 移除付费层级、提升长见识为顶层北极星。CTO_ROADMAP M1-M4 Exit Criteria 移除全部商业化指标（付费用户数/年收入/留存率/NPS），替换为反指标。全量文档扫描校准中。 | `docs/NARRATIVE_CHARTER.md` |
| 2026-06-28 | Cosmos 视觉预设 + 纹理切换完成 — Earth.tsx 集成 Three.js Cosmos 预设（MeshStandardMaterial 升级、双辉光壳、600 星星空、动画循环）。TopBar 纹理切换按钮（Blue Marble ↔ Topographic）。5 个 bug 修复（reset 卡 loading、Sandcastle 错误损坏 Cosmos、cesiumContainer 缺失 ID、重复清理 effect、隐式 Cosmos 清理）。10 项审计发现修复（共享类型/事件常量提取、resetCounter 替代 serverReady 切换、死代码移除）。55 UI 测试通过，TypeScript 干净。 | 审计报告见本轮对话 |
| 2026-06-23 | Contacts 数据接入 Pipeline 完成 — `RelationshipAnnotation` BC 的 contacts 现已参与 analysis + attention 两条链路。新增 `ContactInput`/`FrameAnnotationInput` 反序列化类型、`ContactAuditRecord` 审计记录、`contacts_to_tritwords()` 转换函数。`run_analysis()` 和 `run_attention()` 均接受 contacts 参数。`SqliteAuditLog` 支持 `contact_participation` 往返序列化。端到端集成测试通过。96+33 tests, clippy + fmt clean。 | `docs/superpowers/specs/2026-06-23-aurora-contacts-pipeline-design.md` |
| 2026-06-23 | BC 架构硬化完成 — 删除旧 M0 模块（attention/decision/render），两条独立 BC 链路（analysis + attention）替代旧单管道。`pipeline/analysis.rs`（SignalAnalysis→TernaryDecision）+ `pipeline/attention.rs`（AttentionGuidance→AuditTrail→SQLite）。114 tests 0 failures，clippy + fmt clean。 | `docs/superpowers/specs/2026-06-23-aurora-bc-hardening-design.md` |
| 2026-06-23 | 深度技术审计完成 — 702 tests 0 failures，clippy clean，5层架构评估 B+。修复 3 个 `unimplemented!()` 调用（AnnotationStore/AuditPort trait 改为 owned 方法默认实现），TECH_REVIEW_CHECKLIST 不一致修正。WAL 模式已启用。 | 审计报告见本轮对话 |
| 2026-06-22 | M1 SQLite 数据层完成 — rusqlite 集成，`aurora/src/db/` 含 schema 迁移、AnnotationStore/AuditPort SQLite 实现、`~/.aurora/` 目录管理，22 个新测试通过 | `aurora/src/db/` |
| 2026-06-22 | M1 BC 模块骨架完成 — 6 个限界上下文（SignalAnalysis/RelationshipAnnotation/TernaryDecision/AttentionGuidance/AuditTrail/Presentation）在 `aurora/src/bc/` 搭建，47 个测试通过，trait 边界清晰，旧模块向后兼容 | `docs/superpowers/specs/2026-06-20-aurora-architecture-design.md` §3 |
| 2026-06-22 | M0 Exit Criteria 全部达成 — TECH_REVIEW_CHECKLIST 81 项勾选，性能基准建立（热路径 4.2ns，管道 10µs），`#![forbid(unsafe_code)]` 添加至 Aurora crate，文档同步完成 | `aurora/04_engineering/TECH_REVIEW_CHECKLIST.md` |
| 2026-06-20 | M0 全部 P0 代码任务完成 — 数据采集抽象层/注意力调度闭环/ASI 仪表渲染/端到端测试全部通过（600+ tests），clippy clean | `docs/superpowers/plans/2026-06-20-m0-remaining-tasks.md` |

---

## 4. 文档导航（按阅读顺序）

### 新协作者（第一次加入）

1. **本文件**（你在读的）— 30 秒了解进度
2. `aurora/MASTER_PLAN.md` — 唯一执行入口
3. `aurora/06_roadmap/CTO_ROADMAP.md` — CTO 级战略规划
4. `aurora/00_manifest/CHARTER.md` — 不可谈判的 4 条底线
5. `map/00_START_HERE.md` — 双螺旋知识库入口

### AI 协作者（每次新会话）

1. **本文件** — 看"当前进度"和"上次决策"
2. `docs/NARRATIVE_CHARTER.md` — 叙事基准（产品动机统一口径，必读）
3. `CLAUDE.md` — 项目技术约束（构建命令、架构、设计规则）
4. `map/06_code.md` — 代码→文档的交叉引用
5. `docs/superpowers/specs/2026-06-20-aurora-architecture-design.md` — 最新架构设计

### 按主题深入

| 你想了解... | 入口 |
|------------|------|
| 为什么用三值逻辑 | `docs/adr/001-ternary-logic.md` |
| Frame 系统怎么工作 | `docs/explanation/CONCEPTS.md` §2 |
| 安全模型 | `aurora/03_whitepaper/SECURITY_MODEL.md` |
| 伦理约束 | `aurora/00_manifest/CHARTER.md` |
| 认知架构 5 层 | `aurora/00_manifest/COGNITIVE_ARCHITECTURE_LAYERS.md` |
| 所有架构决策 | `map/03_adr.md` |
| API 契约 | `docs/reference/api.md` |

---

## 5. 关键文件清单（防迷路）

```
项目根目录/
├── SESSION_START.md          ← ⚡ 你在这里
├── CLAUDE.md                 ← AI 协作者必读（技术约束）
├── README.md                 ← 项目主页
├── CHANGELOG.md              ← 版本变更记录
├── SECURITY.md               ← 安全策略
│
├── src/                      ← Trit-Core 源码（Rust 库）
│   ├── core/                 ← 三值代数核心
│   ├── meta/                 ← 策略引擎
│   ├── adapters/             ← 认知模块池（10 个模块）
│   ├── anchor/               ← 锚点约束层
│   ├── feedback/             ← 反馈循环层
│   ├── hook/                 ← Hook 管理层
│   ├── security/             ← 安全门禁
│   ├── sandbox/              ← 场景管道
│   ├── budget/               ← 计算预算
│   ├── calibration/          ← 校准日志
│   └── clock.rs              ← 相位振荡器
│
├── aurora/                   ← Aurora 应用（Rust 二进制）
│   ├── MASTER_PLAN.md        ← 唯一执行入口
│   ├── 00_manifest/          ← 宪章、原则、认知架构
│   ├── 01_insights/          ← 认知科学洞察
│   ├── 02_math/              ← 数学模型
│   ├── 03_whitepaper/        ← 技术白皮书
│   ├── 04_engineering/       ← 工程实现
│   ├── 05_adr/               ← 架构决策记录（9 个）
│   ├── 06_roadmap/           ← 路线图 + CTO_ROADMAP
│   ├── 07_specs/             ← 详细规格
│   ├── 08_reports/           ← 报告模板
│   └── src/                  ← Aurora 源码
│
├── docs/                     ← Trit-Core 技术文档
│   ├── adr/                  ← 架构决策记录（4 个）
│   ├── explanation/          ← 架构、概念、哲学
│   ├── how-to/               ← CLI 参考、配置、贡献
│   ├── reference/            ← API、模块、基准
│   ├── reports/              ← 验证与审计报告
│   ├── tutorials/            ← 快速上手
│   └── superpowers/specs/    ← 架构设计文档
│
├── map/                      ← Obsidian 风格知识库 MOC
│   ├── 00_START_HERE.md      ← 双螺旋入口
│   ├── 01_manifest.md        ← 宣言 MOC
│   ├── 02_concepts.md        ← 概念 MOC
│   ├── 03_adr.md             ← ADR MOC
│   ├── 04_math.md            ← 数学 MOC
│   ├── 05_engineering.md     ← 工程 MOC
│   ├── 06_code.md            ← 代码导航 MOC
│   ├── 07_insights.md        ← 洞察 MOC
│   └── 99_tag_index.md       ← 标签索引
│
├── 圆桌会议.md               ← 哲学对话记录
├── 开悟.md                   ← 长篇哲学论述
├── 审计2023.6.19.md          ← 审计记录
└── 自审计.md                 ← 自我审计
```

---

## 6. 快速命令

```bash
# 构建
cargo build --release

# 全部测试
cargo test --workspace --all-features

# 伦理门禁（不可跳过）
cargo test ethics_

# 代码质量
cargo fmt --check && cargo clippy --workspace --all-targets --all-features -- -D warnings

# 运行 Aurora
cargo run --bin aurora -- --input synthetic_2hz.json --output report.html

# 运行 Trit-Core 沙盒
cargo run --bin trit-sandbox -- --scenario scenarios/medical_conflict_01.json
```

---

*此文件为项目开机导航。每次重大决策或阶段完成后更新"当前进度"和"上次决策"。不是指教，是提醒。*
