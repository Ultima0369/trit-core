# MOC — 统一标签索引

> **Scope**: 全部文档的统一标签系统。按标签浏览，可快速定位跨 MOC 的内容。
>
> #trit-core #aurora #tags #index #navigation

---

## 按主题标签

### #trit-core
Trit-Core crate 相关文档。代码在 `src/`，文档在 `docs/` 和 `map/` 中标记为 #trit-core 的条目。

- [[001-ternary-logic]]
- [[002-phase-arithmetic]]
- [[003-domain-conflict]]
- [[004-distributed-protocol]]
- [[CONCEPTS]]
- [[ARCHITECTURE]]
- [[api]]
- [[MODULES]]
- [[BENCHMARK]]
- [[validation-report]]
- [[performance-validation]]

### #aurora
Aurora 应用相关文档。代码在 `src-tauri/`、`src/wavelet/`、`src/db/`，文档在 `aurora/` 和 `map/` 中标记为 #aurora 的条目。

- [[AURORA_MANIFEST]]
- [[CHARTER]]
- [[FIRST_PRINCIPLES]]
- [[COGNITIVE_ARCHITECTURE_LAYERS]]
- [[LOCAL_MODEL_ETHICS_SPEC]]
- [[001-local-first]] 到 [[009-ethics-hardening]]（全部 9 个 Aurora ADR）
- [[API_CONTRACT]]
- [[SECURITY_MODEL]]
- [[DATA_MODEL]]
- [[SYSTEM_DESIGN]]
- [[PIPELINE_DESIGN]]
- [[DEPLOYMENT_GUIDE]]
- [[TESTING_STRATEGY]]
- [[TECH_REVIEW_CHECKLIST]]
- [[WAVELET_ENGINE_SPEC]]
- [[UI_SPEC]]
- [[ALERT_ENGINE_SPEC]]
- [[DATA_INGESTION_SPEC]]
- [[FRAME_MODEL_SPEC]]
- [[TRIT_CORE_INTEGRATION_SPEC]]

### #adr
所有架构决策记录。

- docs: [[001-ternary-logic]], [[002-phase-arithmetic]], [[003-domain-conflict]], [[004-distributed-protocol]]
- aurora: [[001-local-first]] 到 [[009-ethics-hardening]]

### #math
数学与形式化文档。

- [[PHASE_ARITHMETIC]]
- [[INFORMATION_THEORY]]
- [[FIELD_EQUATIONS]]
- [[ATTENTION_DYNAMICS]]
- [[WAVELET_ANALYSIS]]

### #engineering
工程实现文档。

- [[API_CONTRACT]], [[SECURITY_MODEL]], [[ARCHITECTURE]]
- [[DATA_MODEL]], [[SYSTEM_DESIGN]], [[PIPELINE_DESIGN]]
- [[DEPLOYMENT_GUIDE]], [[TESTING_STRATEGY]], [[TECH_REVIEW_CHECKLIST]]
- [[MODULES]], [[BENCHMARK]], [[api]]

### #insights
洞察与哲学文档。

- [[100MS_HIJACK_MODEL]], [[ATTENTION_CAPITALISM]], [[ATTENTION_DYNAMICS]]
- [[COGNITIVE_SOVEREIGNTY]], [[FRAME_EPISTEMOLOGY]], [[NEURAL_REWIRING_PROTOCOL]]
- [[ORGANIZATIONAL_VORTEX]], [[ENVIRONMENTAL_SHOCK]], [[N_OF_1_PROTOCOL]]
- [[PHILOSOPHY]], [[EPISTEMIC_HUMILITY]], [[DIALOGUE_ORIGIN]]
- [[FUTURE]], [[CONFLICT_CATALOG]], [[HUMANITIES_INDEX]], [[GLOSSARY]]
- [[DAO_SCIENCE_REFERENCES]], [[DAO_SCIENCE_IMPORT_ASSESSMENT]]

### #testing
测试相关文档。

- [[TESTING_STRATEGY]]
- [[validation-report]]
- [[performance-validation]]
- [[TECH_REVIEW_CHECKLIST]]
- [[CONTRIBUTING]]
- [[BENCHMARK]]

### #security
安全相关文档。

- [[SECURITY_MODEL]]
- [[security-audit]]
- [[cto-audit-report]]
- [[deep-audit-cto-2026-06-18]]
- [[LOCAL_MODEL_ETHICS_SPEC]]
- [[009-ethics-hardening]]

---

## 按状态标签

### #active
当前有效文档（最新版本）。

- 全部 `aurora/` 文档（v0.3.0）
- 全部 `map/` 文档
- `docs/` 中的 v0.3.0 文档：[[validation-report]], [[performance-validation]], [[api]], [[CONCEPTS]], [[ARCHITECTURE]], [[PHILOSOPHY]]

### #historical
历史版本文档，仅供参考。

- `docs/archive/` 全部
- `docs/reports/security-audit.md`（v0.1.0）
- `docs/reports/code-quality-audit.md`（v0.1.0）
- `docs/reports/cto-audit-report.md`（v0.1.0）
- `docs/reports/deep-audit-cto-2026-06-18.md`（v0.1.0）
- `docs/adr/004-distributed-protocol.md`（已移除）
- `docs/_archive/superpowers/` 全部

### #deprecated
已废弃的文档或功能。

- [[004-distributed-protocol]]（v0.2.0 移除网络层）

---

## 按语言标签

### #english
英文文档。

- `docs/adr/` 全部（4 个）
- `docs/explanation/ARCHITECTURE.md`
- `docs/reference/` 全部（3 个）
- `docs/reports/validation-report.md`
- `docs/archive/` 中部分

### #chinese
中文文档。

- `aurora/` 全部（60 个）
- `docs/explanation/` 中大部分（CONCEPTS, PHILOSOPHY, insights/）
- `docs/how-to/` 全部
- `docs/tutorials/` 全部
- `map/` 全部

---

## 按证据等级标签

### #formal
严格形式化内容（类型系统约束、测试验证、ADR 决策）。

- `src/core/` 类型约束
- `tests/` 全部测试
- `docs/adr/` 全部
- `aurora/05_adr/` 全部
- [[TECH_REVIEW_CHECKLIST]]

### #analogy
启发性类比（非严格数学，提供直觉）。

- [[INFORMATION_THEORY]]（$F_{trinary}$ 为启发性定义）
- [[FIELD_EQUATIONS]]（认知场为类比模型）
- [[ATTENTION_DYNAMICS]]（α-ROI 为描述性框架）

### #n-of-1
第一人称观察（N=1）。明确标注，不构成统计证据。

- [[100MS_HIJACK_MODEL]]
- [[N_OF_1_PROTOCOL]]
- [[NEURAL_REWIRING_PROTOCOL]]

---

## 使用 Obsidian 标签功能

在 Obsidian 中：

1. 打开 **Tag Pane**（左侧边栏 → 标签图标）
2. 点击任意标签，查看所有带该标签的文件
3. 标签支持嵌套：`#engineering/testing` 形式的层级（需手动维护）

在普通 Markdown 中：

- 用文本搜索 `#标签名` 定位相关内容
- 本文件作为手动标签索引使用

---

**相关 MOC**: 全部

#map-of-content #tags #index #navigation #taxonomy
