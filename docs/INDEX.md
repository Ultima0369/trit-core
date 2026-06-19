# Trit-Core 文档导航

**Current version**: 0.3.0

欢迎。本文档系统按 [Diátaxis](https://diataxis.fr/) 框架组织为四类：

- **Tutorials** — 手把手教程，面向学习者
- **How-to Guides** — 任务导向指南，面向使用者
- **Explanation** — 概念解释，面向理解者
- **Reference** — 精确信息，面向查阅者

此外设有 `reports/`（验证与审计报告）、`adr/`（架构决策记录）、`archive/`（历史归档）和 `zh/`（中文翻译）。

---

## 推荐阅读路径

| 你的身份 | 推荐路径 |
|---|---|
| 第一次听说这个项目 | [WHAT_IS_TRIT](tutorials/WHAT_IS_TRIT.md) → [PHILOSOPHY](explanation/PHILOSOPHY.md) → [README](/README.md) |
| 想快速跑起来看看 | [QUICKSTART](tutorials/QUICKSTART.md) → [CLI_REFERENCE](how-to/CLI_REFERENCE.md) |
| 想理解架构和数学 | [CONCEPTS](explanation/CONCEPTS.md) → [ARCHITECTURE](explanation/ARCHITECTURE.md) → [ADR 系列](adr/) |
| 想集成到自己的项目 | [API 参考](reference/api.md) → [MODULES](reference/MODULES.md) → [CUSTOM_RULE](how-to/CUSTOM_RULE.md) |
| 想贡献代码 | [CONTRIBUTING](how-to/CONTRIBUTING.md) → [MODULES](reference/MODULES.md) → [BENCHMARK](reference/BENCHMARK.md) |
| 想评审或审计项目 | [REVIEWER_GUIDE](how-to/REVIEWER_GUIDE.md) → [validation-report](reports/validation-report.md) → [security-audit](reports/security-audit.md) |
| 想理解深层动机和未来 | [PHILOSOPHY](explanation/PHILOSOPHY.md) → [DIALOGUE_ORIGIN](explanation/insights/DIALOGUE-ORIGIN.md) → [FUTURE](explanation/insights/FUTURE.md) → [CONFLICT_CATALOG](explanation/insights/CONFLICT_CATALOG.md) |

---

## 文档地图

### 🌱 Tutorials — 入门教程

| 文档 | 内容 |
|---|---|
| [WHAT_IS_TRIT](tutorials/WHAT_IS_TRIT.md) | 三个故事解释“为什么需要三值决策” |
| [QUICKSTART](tutorials/QUICKSTART.md) | 3 分钟：克隆→编译→运行第一个场景 |

### 🛠️ How-to Guides — 使用指南

| 文档 | 内容 |
|---|---|
| [CLI_REFERENCE](how-to/CLI_REFERENCE.md) | `trit-sandbox` 命令、参数、JSON 场景格式规范 |
| [CONFIGURATION](how-to/CONFIGURATION.md) | 环境变量与日志行为控制 |
| [CUSTOM_RULE](how-to/CUSTOM_RULE.md) | `RuleLoader` 特质：如何定义自定义仲裁域 |
| [CONTRIBUTING](how-to/CONTRIBUTING.md) | 代码风格、CI 门禁、测试策略、如何添加新 Frame/Domain |
| [REVIEWER_GUIDE](how-to/REVIEWER_GUIDE.md) | 评审者指引（核心声明验证步骤） |

### 📖 Explanation — 概念解释

| 文档 | 内容 |
|---|---|
| [技术白皮书](technical-whitepaper.md) | v0.3.0 综合技术总览与审核索引（本文档替代 archive/technical-whitepaper.md） |
| [CONCEPTS](explanation/CONCEPTS.md) | `TritValue`、`Phase`、`Frame`、`Domain`、`TritWord` 的完整定义与设计理由 |
| [ARCHITECTURE](explanation/ARCHITECTURE.md) | 分层架构、热/冷路径、SafeFallback 的 IEC 61508 依据 |
| [PHILOSOPHY](explanation/PHILOSOPHY.md) | 热力学约束、群体认知髓鞘化、AI 对齐的认知生态视角 |
| [ROADMAP](explanation/roadmap.md) | 当前版本路线图与里程碑 |

#### 深度洞察（explanation/insights/）

| 文档 | 内容 |
|---|---|
| [EPISTEMIC_HUMILITY](explanation/insights/EPISTEMIC-HUMILITY.md) | 认识论谦逊声明：提醒而非指教，邀请实践检验与交叉验证 |
| [DAO_SCIENCE_REFERENCES](explanation/insights/DAO-SCIENCE-REFERENCES.md) | dao-science 项目引用参考：认知频谱、知止、第一人称认识论、偏离代价 |
| [DIALOGUE_ORIGIN](explanation/insights/DIALOGUE-ORIGIN.md) | 开悟.md 对话与 Trit-Core 的思想源流关系 |
| [HUMANITIES_INDEX](explanation/insights/HUMANITIES-INDEX.md) | 人文关键词科学化定义索引：科学范式、个体实情、身心关系、意识起源等 |
| [CONFLICT_CATALOG](explanation/insights/CONFLICT_CATALOG.md) | 跨域冲突模式分类与记录 |
| [FUTURE](explanation/insights/FUTURE.md) | 已知局限与可能的解决路径 |
| [GLOSSARY](explanation/insights/GLOSSARY.md) | 术语表：本项目发明的术语及其跨学科对应 |

### 🔍 Reference — 参考文档

| 文档 | 内容 |
|---|---|
| [api.md](reference/api.md) | 公共 API 契约：类型、方法、错误分类 |
| [MODULES](reference/MODULES.md) | `src/` 下每个子模块的职责、关键函数、设计约束 |
| [BENCHMARK](reference/BENCHMARK.md) | Criterion 基准测试运行方法与当前性能数据 |

### 📊 Reports — 报告与审计

| 文档 | 内容 |
|---|---|
| [validation-report](reports/validation-report.md) | M2/M3 三元 vs 二元对比验证（0.3.0） |
| [performance-validation](reports/performance-validation.md) | 端到端性能验证（TPS 对比、瓶颈分析） |
| [security-audit](reports/security-audit.md) | 安全审计报告（v0.1.0） |
| [code-quality-audit](reports/code-quality-audit.md) | 代码质量审计报告（v0.1.0） |
| [cto-audit-report](reports/cto-audit-report.md) | CTO 审计报告（v0.1.0） |
| [deep-audit-cto](reports/deep-audit-cto-2026-06-18.md) | CTO 深度技术审计（v0.1.0，42 项发现） |
| [reflexive-audit](../audit_log/08_reflexive_audit.md) | v0.2.0 重构自反性审计与迭代跟进 |

### 🗃️ ADR — 架构决策记录

| 文档 | 内容 |
|---|---|
| [adr/001-ternary-logic.md](adr/001-ternary-logic.md) | 采用三值逻辑的决策 |
| [adr/002-phase-arithmetic.md](adr/002-phase-arithmetic.md) | Phase 算术设计 |
| [adr/003-domain-conflict.md](adr/003-domain-conflict.md) | 域冲突检测策略 |
| [adr/004-distributed-protocol.md](adr/004-distributed-protocol.md) | 分布式节点协议（已归档，v0.1.x） |

---

## 历史文档（v0.1.x / v0.2.0）

以下文档描述早期版本，已归档。完整归档说明见 [archive/README.md](archive/README.md)。

| 文档 | 内容 |
|---|---|
| [archive/technical-whitepaper.md](archive/technical-whitepaper.md) | 技术白皮书（中文，v0.1.x） |
| [archive/preprint.md](archive/preprint.md) | 预印本（英文，v0.1.x） |
| [archive/roadmap-v0.1.0.md](archive/roadmap-v0.1.0.md) | v0.1.0 路线图快照 |
| [zh/README.zh.md](zh/README.zh.md) | 中文文档入口（v0.1.x 历史文档，按 Diátaxis 镜像组织） |
| [zh/archive/whitepaper.zh.md](zh/archive/whitepaper.zh.md) | 技术白皮书（中文，v0.1.x） |
| [zh/archive/preprint.zh.md](zh/archive/preprint.zh.md) | 学术预印本（中文，v0.1.x） |
| [zh/explanation/roadmap.zh.md](zh/explanation/roadmap.zh.md) | 路线图草案（中文，v0.1.x） |
| [zh/explanation/architecture-audit.zh.md](zh/explanation/architecture-audit.zh.md) | 架构审计（中文，v0.1.x） |
| [zh/reference/api.zh.md](zh/reference/api.zh.md) | 公共 API 契约（中文，v0.1.x） |

当前版本顶层文件：[CHANGELOG.md](../CHANGELOG.md)、[SECURITY.md](../SECURITY.md)。

---

## 关于本文档系统

本导航文件 (`INDEX.md`) 是文档系统的入口。所有活跃文档按 Diátaxis 四类 + Reports/ADR/Archive 组织，便于按读者目的查找和维护。旧文档保留在 `archive/` 或 `zh/` 中，确保外部引用不失效。
