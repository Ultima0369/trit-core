# MOC — 数学模型

> **Scope**: 所有数学/形式化文档，包括三值代数、信息论、场方程、注意力动力学、小波分析。
>
> #trit-core #aurora #math #formal #equations #information-theory

---

## 三值代数与相位运算

| 文件 | 位置 | 语言 | 关键内容 |
|---|---|---|---|
| [[PHASE_ARITHMETIC]] | `aurora/02_math/PHASE_ARITHMETIC.md` | 中文 | 相位定义、运算规则、连续性证明。 |
| [[002-phase-arithmetic]] | `docs/adr/002-phase-arithmetic.md` | 英文 | 相位算术的 ADR 决策记录。 |

**代码位置**: `src/core/phase.rs`

---

## 信息论

| 文件 | 位置 | 语言 | 关键内容 |
|---|---|---|---|
| [[INFORMATION_THEORY]] | `aurora/02_math/INFORMATION_THEORY.md` | 中文 | 三值信息论：F_trinary 启发性类比、KL 散度路径、决策信息熵。 |

**注意**: `INFORMATION_THEORY` 明确标注为**启发性类比**，不是严格数学定理。阅读时请注意文档中的“类比”与“严格”区分。

---

## 场方程（类比模型）

| 文件 | 位置 | 语言 | 关键内容 |
|---|---|---|---|
| [[FIELD_EQUATIONS]] | `aurora/02_math/FIELD_EQUATIONS.md` | 中文 | 认知场方程：E_input → CRU 的映射关系。 |

**注意**: 标注为**类比模型**，非严格物理场论。阅读时请注意 `CRU`（Cognitive Resource Unit）是项目自定义单位。

---

## 注意力动力学

| 文件 | 位置 | 语言 | 关键内容 |
|---|---|---|---|
| [[ATTENTION_DYNAMICS]] | `aurora/02_math/ATTENTION_DYNAMICS.md` | 中文 | α 注意力动力学：注意力资本、α-ROI、注意力信贷。 |
| [[ATTENTION_CAPITALISM]] | `aurora/01_insights/ATTENTION_CAPITALISM.md` | 中文 | 注意力资本主义的批判与替代。 |

**来源**: 从 `dao-science` 项目引入。核心洞察：注意力不是资源，而是**可投资的资本**。

---

## 小波分析

| 文件 | 位置 | 语言 | 关键内容 |
|---|---|---|---|
| [[WAVELET_ANALYSIS]] | `aurora/02_math/WAVELET_ANALYSIS.md` | 中文 | 小波变换：时间-频率联合分析、多分辨率分解。 |
| [[WAVELET_ENGINE_SPEC]] | `aurora/07_specs/WAVELET_ENGINE_SPEC.md` | 中文 | 小波引擎规格：实现细节、接口定义。 |
| [[002-wavelet-over-fft]] | `aurora/05_adr/002-wavelet-over-fft.md` | 中文 | ADR：为什么选小波而非 FFT。 |

**代码位置**: `src/wavelet/`（Aurora 分析引擎）

---

## 跨链连接（数学 ↔ 代码）

| 数学概念 | 文档 | 代码 |
|---|---|---|
| Phase [0.0, 1.0] | `aurora/02_math/PHASE_ARITHMETIC.md` | `src/core/phase.rs` |
| 三值真值表 | `docs/explanation/CONCEPTS.md` §1.4 | `src/core/algebra.rs` |
| 注意力动力学 | `aurora/02_math/ATTENTION_DYNAMICS.md` | `aurora/src/pipeline/attention.rs` |
| 小波变换 | `aurora/02_math/WAVELET_ANALYSIS.md` | `aurora/src/wavelet/` |
| 信息熵类比 | `aurora/02_math/INFORMATION_THEORY.md` | `src/adapters/cognitive_deconstruction.rs`（熵计算） |

---

## 风险提示

以下文档包含**启发性类比**，阅读时请注意：

- `INFORMATION_THEORY.md` — $F_{trinary}$ 是启发性定义，非严格信息论
- `FIELD_EQUATIONS.md` — 认知场是类比模型，$CRU$ 是自定义单位
- `ATTENTION_DYNAMICS.md` — α-ROI 是描述性框架，非金融投资公式

这些类比的价值在于**提供直觉**和**建立跨学科对话**，而非作为定理使用。如需严格形式化，请参考 `docs/adr/` 和 `src/core/` 中的类型约束。

---

**相关 MOC**: [[02_concepts]] · [[03_adr]] · [[05_engineering]] · [[07_insights]]

#map-of-content #math #formal #phase #information-theory #wavelet #attention #analogy
