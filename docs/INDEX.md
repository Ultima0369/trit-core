# Trit-Core 文档导航

欢迎。这份索引帮助你根据目的找到正确的文档。

## 推荐阅读路径

| 你的身份 | 推荐路径 |
|---|---|
| 第一次听说这个项目 | [WHAT_IS_TRIT](getting-started/WHAT_IS_TRIT.md) → [PHILOSOPHY](concepts/PHILOSOPHY.md) → [README](/README.md) |
| 想快速跑起来看看 | [QUICKSTART](getting-started/QUICKSTART.md) → [CLI_REFERENCE](usage/CLI_REFERENCE.md) |
| 想理解架构和数学 | [CONCEPTS](concepts/CONCEPTS.md) → [ARCHITECTURE](concepts/ARCHITECTURE.md) → [ADR 系列](adr/) |
| 想集成到自己的项目 | [API 参考](api.md) → [MODULES](development/MODULES.md) → [CUSTOM_RULE](usage/CUSTOM_RULE.md) |
| 想贡献代码 | [CONTRIBUTING](development/CONTRIBUTING.md) → [MODULES](development/MODULES.md) → [BENCHMARK](development/BENCHMARK.md) |
| 想理解深层动机和未来 | [PHILOSOPHY](concepts/PHILOSOPHY.md) → [FUTURE](insights/FUTURE.md) → [CONFLICT_CATALOG](insights/CONFLICT_CATALOG.md) |

---

## 文档地图

### 🌱 第一层：入门与快速上手

| 文档 | 内容 |
|---|---|
| [WHAT_IS_TRIT](getting-started/WHAT_IS_TRIT.md) | 三个故事解释"为什么需要三值决策" |
| [QUICKSTART](getting-started/QUICKSTART.md) | 3 分钟：克隆→编译→运行第一个场景 |
| [README](/README.md) | 项目概述、架构速览、里程碑状态 |

### 📖 第二层：核心概念与原理

| 文档 | 内容 |
|---|---|
| [CONCEPTS](concepts/CONCEPTS.md) | TritValue、Phase、Frame、Domain、TritWord 的完整定义与设计理由 |
| [ARCHITECTURE](concepts/ARCHITECTURE.md) | 分层架构、热/冷路径、SafeFallback 的 IEC 61508 依据 |
| [PHILOSOPHY](concepts/PHILOSOPHY.md) | 热力学约束、群体认知髓鞘化、AI 对齐的认知生态视角 |

### 🛠️ 第三层：使用指南

| 文档 | 内容 |
|---|---|
| [CLI_REFERENCE](usage/CLI_REFERENCE.md) | trit-sandbox 命令、参数、JSON 场景格式规范 |
| [CONFIGURATION](usage/CONFIGURATION.md) | 环境变量与日志行为控制 |
| [CUSTOM_RULE](usage/CUSTOM_RULE.md) | RuleLoader 特质：如何定义自定义仲裁域 |

### 🔍 第四层：开发者文档

| 文档 | 内容 |
|---|---|
| [CONTRIBUTING](development/CONTRIBUTING.md) | 代码风格、CI 门禁、测试策略、如何添加新 Frame/Domain |
| [MODULES](development/MODULES.md) | src/ 下每个子模块的职责、关键函数、设计约束 |
| [BENCHMARK](development/BENCHMARK.md) | Criterion 基准测试运行方法与当前性能数据 |

### 🧠 第五层：深度洞察

| 文档 | 内容 |
|---|---|
| [FUTURE](insights/FUTURE.md) | 已知局限与可能的解决路径 |
| [CONFLICT_CATALOG](insights/CONFLICT_CATALOG.md) | 跨域冲突模式分类与记录 |
| [GLOSSARY](insights/GLOSSARY.md) | 术语表：本项目发明的术语及其跨学科对应 |

---

## 历史文档（保留）

以下文档在开发过程中产生，保持原位以供追溯：

| 文档 | 内容 |
|---|---|
| [whitepaper.md](whitepaper.md) | 技术规格书（v0.1.0-alpha） |
| [technical-whitepaper.md](technical-whitepaper.md) | 扩展技术白皮书 |
| [preprint.md](preprint.md) | 预印本（英文，10+ 页） |
| [api.md](api.md) | 公共 API 合约 |
| [roadmap.md](roadmap.md) | 里程碑计划 |
| [validation-report.md](validation-report.md) | M2 三元 vs 二元对比验证 |
| [security-audit.md](security-audit.md) | 应用安全审计 |
| [code-quality-audit.md](code-quality-audit.md) | 代码质量审计 |
| [performance-audit.md](performance-audit.md) | 性能审计 |
| [adr/](adr/) | 架构决策记录（4 篇） |
| [zh/](zh/) | 中文翻译（预印本、白皮书、ADR、路线图、API） |

---

## 关于本文档系统

本导航文件 (`INDEX.md`) 是文档系统的入口。所有新文档位于语义命名的子目录中，旧文档保留原位以确保外部引用不失效。随着项目演进，旧文档中的内容将逐步迁移到新的分层结构中。
