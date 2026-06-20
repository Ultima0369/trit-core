# ⚡ SESSION_START — 开机导航

> **目的**：每次新会话开始时，AI 或人类协作者读此文件，30 秒内定位工作进度。
> **维护规则**：每次完成一个阶段或做出重大决策后，更新"当前进度"和"上次决策"两节。
> **版本**：1.0.0
> **最后更新**：2026-06-20

---

## 1. 项目是什么（一句话）

**Trit-Core**：三元决策引擎（Rust 库），用 True/Hold/False 替代二元逻辑。
**Aurora**：基于 Trit-Core 的注意力主权训练系统（桌面应用），帮助用户梳理人际关系、提升决策质量。

---

## 2. 当前进度

| 维度 | 状态 |
|------|------|
| **Trit-Core 版本** | v0.3.0 — 单机决策引擎，5 层架构完整（Core→Meta→Hook→Adapter→Feedback） |
| **Aurora 阶段** | M0 概念验证 — 合成数据→小波→三值→CLI/HTML 输出已完成；伦理门禁 10 个测试通过；数据采集抽象层/注意力调度闭环/注意力图谱 HTML 渲染已完成 |
| **架构设计** | 已完成 — 见 `docs/superpowers/specs/2026-06-20-aurora-architecture-design.md` |
| **M0 剩余工作** | 无代码任务。P1 待办：作者自我验证 + 2 外部用户试用（创始人侧） |
| **M1 入口** | Tauri 桌面应用（见 `aurora/06_roadmap/CTO_ROADMAP.md` §M1） |

---

## 3. 上次决策（最近 3 个）

| 日期 | 决策 | 文档 |
|------|------|------|
| 2026-06-20 | M0 全部 P0 代码任务完成 — 数据采集抽象层/注意力调度闭环/ASI 仪表渲染/端到端测试全部通过（600+ tests），clippy clean | `docs/superpowers/plans/2026-06-20-m0-remaining-tasks.md` |
| 2026-06-20 | 架构设计文档完成 — 6 个 BC，模块化单体 M0-M1，分布式 trait 预留 | `docs/superpowers/specs/2026-06-20-aurora-architecture-design.md` |
| 2026-06-20 | Aurora 架构风格：模块化单体（M0-M1），分布式接口 trait 预留，本期不实现 | `docs/superpowers/specs/2026-06-20-aurora-architecture-design.md` |

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
2. `CLAUDE.md` — 项目技术约束（构建命令、架构、设计规则）
3. `map/06_code.md` — 代码→文档的交叉引用
4. `docs/superpowers/specs/2026-06-20-aurora-architecture-design.md` — 最新架构设计

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
