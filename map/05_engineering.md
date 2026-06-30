# MOC — 工程实现

> **Scope**: 所有工程文档：API 契约、系统架构、数据模型、测试策略、部署指南、性能报告。
>
> #trit-core #aurora #engineering #implementation #api #testing #deployment

---

## API 契约与公共接口

| 文件 | 位置 | 语言 | 关键内容 |
|---|---|---|---|
| [[api]] | `docs/reference/api.md` | 英文 | Trit-Core crate 公共 API：TritValue, TritWord, Frame, Phase, TernaryAlgebra 的完整接口。 |
| [[API_CONTRACT]] | `aurora/03_whitepaper/API_CONTRACT.md` | 中文 | Aurora 与 Trit-Core 的集成契约：Frame 映射、数据流、错误处理。 |
| [[MODULES]] | `docs/reference/MODULES.md` | 英文 | 模块级文档：每个模块的职责和接口。 |

**代码位置**: `src/lib.rs`（公共导出）, `src/core/mod.rs`

---

## 系统架构

| 文件 | 位置 | 语言 | 关键内容 |
|---|---|---|---|
| [[ARCHITECTURE]] | `docs/explanation/ARCHITECTURE.md` | 英文 | Trit-Core 系统架构：模块分层、设计原则、数据流。 |
| [[ARCHITECTURE]] | `aurora/03_whitepaper/ARCHITECTURE.md` | 中文 | Aurora 五层架构：L1 锚定层 → L5 元层。 |
| [[SYSTEM_DESIGN]] | `aurora/04_engineering/SYSTEM_DESIGN.md` | 中文 | 系统架构设计：组件交互、数据流、接口契约。 |
| [[PIPELINE_DESIGN]] | `aurora/04_engineering/PIPELINE_DESIGN.md` | 中文 | 场景管道设计：输入 → 验证 → 运算 → 仲裁 → 输出。 |

**代码位置**: `src/sandbox/pipeline.rs`, `src/sandbox/input.rs`, `src/sandbox/output.rs`

---

## 数据模型

| 文件 | 位置 | 语言 | 关键内容 |
|---|---|---|---|
| [[DATA_MODEL]] | `aurora/04_engineering/DATA_MODEL.md` | 中文 | Aurora 数据模型：Schema、表结构、关系、约束。 |
| [[FRAME_MODEL_SPEC]] | `aurora/07_specs/FRAME_MODEL_SPEC.md` | 中文 | Frame 模型规格：枚举定义、语义映射、验证规则。 |
| [[TRIT_CORE_INTEGRATION_SPEC]] | `aurora/07_specs/TRIT_CORE_INTEGRATION_SPEC.md` | 中文 | Trit-Core 集成规格：AuroraFrame → Frame 映射、数据流。 |

**代码位置**: `src/core/frame.rs`, `src/db/`（Aurora 数据层）

---

## 测试策略

| 文件 | 位置 | 语言 | 关键内容 |
|---|---|---|---|
| [[TESTING_STRATEGY]] | `aurora/04_engineering/TESTING_STRATEGY.md` | 中文 | 测试策略：单元测试、集成测试、属性测试、伦理门禁测试。 |
| [[CONTRIBUTING]] | `docs/how-to/CONTRIBUTING.md` | 中文 | 贡献指南：CI 门禁、代码风格、测试要求。 |
| [[BENCHMARK]] | `docs/reference/BENCHMARK.md` | 英文 | 基准测试：性能数据、测试方法。 |
| [[TECH_REVIEW_CHECKLIST]] | `aurora/04_engineering/TECH_REVIEW_CHECKLIST.md` | 中文 | 发布前技术审查清单：8 大维度。 |

**代码位置**: `tests/` 目录, `benches/` 目录

**关键测试**: `ethics_` 前缀的 10 个伦理门禁测试（不可跳过）

---

## 部署与运维

| 文件 | 位置 | 语言 | 关键内容 |
|---|---|---|---|
| [[DEPLOYMENT_GUIDE]] | `aurora/04_engineering/DEPLOYMENT_GUIDE.md` | 中文 | 部署指南：本地构建、Docker、Tauri 打包。 |
| [[CONFIGURATION]] | `docs/how-to/CONFIGURATION.md` | 中文 | 环境变量与日志配置。 |
| [[CLI_REFERENCE]] | `docs/how-to/CLI_REFERENCE.md` | 中文 | `trit-sandbox` CLI 命令参考。 |
| [[QUICKSTART]] | `docs/tutorials/QUICKSTART.md` | 中文 | 3 分钟快速上手。 |

**代码位置**: `src/bin/sandbox.rs`, `Dockerfile`, `docker-compose.yml`

---

## 验证与审计报告

| 文件 | 位置 | 类型 | 说明 |
|---|---|---|---|
| [[validation-report]] | `docs/reports/validation-report.md` | 当前 | M2/M3 验证：三值 vs 二值基线（v0.3.0） |
| [[performance-validation]] | `docs/reports/performance-validation.md` | 当前 | 性能基准（v0.2.0） |
| [[security-audit]] | `docs/reports/security-audit.md` | 历史 | 安全审计（v0.1.0，部分已修正） |
| [[code-quality-audit]] | `docs/reports/code-quality-audit.md` | 历史 | 代码质量（v0.1.0，部分已修正） |
| [[cto-audit-report]] | `docs/reports/cto-audit-report.md` | 历史 | CTO 审计（v0.1.0，部分已修正） |
| [[deep-audit-cto-2026-06-18]] | `docs/reports/deep-audit-cto-2026-06-18.md` | 历史 | 深度技术审计（v0.1.0，部分已修正） |

**Aurora 报告模板**:
- `aurora/08_reports/VALIDATION_REPORT_TEMPLATE.md`
- `aurora/08_reports/SECURITY_AUDIT_TEMPLATE.md`
- `aurora/08_reports/PERFORMANCE_REPORT_TEMPLATE.md`
- `aurora/08_reports/RETROSPECTIVE_TEMPLATE.md`

---

## 跨链连接（工程 ↔ 代码）

| 工程主题 | 文档 | 代码 |
|---|---|---|
| API 契约 | `docs/reference/api.md` | `src/lib.rs` |
| 沙盒管道 | `aurora/04_engineering/PIPELINE_DESIGN.md` | `src/sandbox/pipeline.rs` |
| 场景验证 | `docs/how-to/CLI_REFERENCE.md` | `src/sandbox/validate.rs` |
| 性能基准 | `docs/reference/BENCHMARK.md` | `benches/` |
| 伦理测试 | `aurora/04_engineering/TESTING_STRATEGY.md` | `tests/ethics_*.rs` |
| 数据模型 | `aurora/04_engineering/DATA_MODEL.md` | `src/db/schema.rs` |

---

**相关 MOC**: [[02_concepts]] · [[03_adr]] · [[04_math]] · [[06_code]]

#map-of-content #engineering #api #testing #deployment #benchmark #audit #ci
