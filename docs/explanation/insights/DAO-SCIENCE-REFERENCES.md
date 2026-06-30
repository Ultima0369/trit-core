# dao-science 引用参考 — 为 Trit-Core 提供认知科学支撑

**Version**: 0.3.0
**Status**: Active

---

## 项目简介

`dao-science`（`C:/dao-science`，MIT 许可证）是一部开源双语研究手册，定位为“关于如何优化观察者本身的心智操作系统手册”。它将东方第一人称心性实践与预测编码、主动推理、神经科学、复杂系统对接，使用形式化方程和可运行仿真表达核心命题：

> 道是预期自由能上的梯度流：`Dao ≡ −∇G(π)`。

本项目与 `trit-core` 的关系不是竞争，而是互补：
- `trit-core` 偏向**数学支撑的 AI 决策模块设计**，提供可运行的三值逻辑引擎。
- `dao-science` 偏向**认知风格与态度**，提供关于觉知、注意力、停止标准、个体实情和偏离代价的理论语言。

本文档筛选 `dao-science` 中对 `trit-core` 具有直接引用价值的内容，标注其证据等级、适用位置和注意事项。

---

## 高价值引用内容

### 1. L0–L7 认知频谱框架

**来源**：`dao-science/0_motivation/L0_L7_spectrum.md`

**核心内容**

L0–L7 将“事实与关系”区分为八个层级：

| 层级 | 名称 | 含义 |
|---|---|---|
| L0 | 绝对事实 / 觉知本身 | 物自体、意识起源、觉悟性洞见；不可言说，只可体认 |
| L1 | 物理规律 / 自然法则 | 可重复验证的普适规律 |
| L2 | 个体实情 / 主观事实 | 感受、记忆、痛苦、身体感知 |
| L3 | 群体共识 / 文化传承 | 叙事、仪式、传统、社会期望 |
| L4 | 理性合作 / 契约精神 | 逻辑、法律、工程协议、项目管理 |
| L5 | 互不干涉 / 关系断裂 | 冷漠、回避、边界固化 |
| L6 | 纯粹妄想 / 虚无主义 | 概念空转、脱离现实检验的信念闭环 |
| L7 | 陷入战争 / 自取灭亡 | 系统的毁灭性崩溃 |

**与 Trit-Core 的映射**

| dao-science 层级 | Trit-Core 对应 |
|---|---|
| L0 | `Frame::Absolute` — 不可知/不可观测，永远 `Hold` |
| L1 | `Frame::Science` + `Physical`/`Engineering` Domain — 普适规律优先 |
| L2 | `Frame::Individual` + `MedicalEthics` 优先级 — 个体实情保护 |
| L3 | `Frame::Consensus` — 统计/群体偏好，不被升格为真理 |
| L4 | `General` Domain / 契约式协商 — 可形式化协作 |
| L5–L7 | 高冲突/高风险状态 — 触发 `Hold` + `MetaInterrupt` 或 `SafeFallback` |

**为何引用**

L0–L7 为 Trit-Core 的 `Frame` 系统提供了认知科学背景：不同层级的事实需要不同的处理方式，跨层级操作不能直接取平均。这直接支撑了 `Frame` 和 `Domain` 的设计哲学。

**建议融入位置**
- `docs/explanation/CONCEPTS.md` §3（Frame 哲学）
- `HUMANITIES-INDEX.md` 的“参考系/Frame”词条
- 新增场景：L2 个体实情 vs L3 群体共识的冲突

---

### 2. 知止不殆与 AI 停止标准

**来源**：`dao-science/4_applications/ai_governance.md`

**核心内容**

“知止”（knowing when to stop）被形式化为 AI 安全的核心原则。当以下两个条件任一满足时，系统应进入停止/悬置状态：

- 风险超过阈值：`D_KL[Q(o|π) || P(o|C_safe)] > τ_risk`
- 歧义超过阈值：`H[P(o|s)] > τ_ambiguity`

其中 `D_KL` 是预期观察分布与安全先验之间的 KL 散度，`H` 是观察的条件熵。

**与 Trit-Core 的映射**

| dao-science 概念 | Trit-Core 对应 |
|---|---|
| 知止 / 停止标准 | `TritValue::Hold` — 主动悬置判断 |
| 风险超过阈值 | 危险域（`Physical`/`Engineering`）触发 `SafeFallback` |
| 歧义超过阈值 | 跨 Frame 冲突触发 `MetaInterrupt` + `Hold` |
| 收放自如（焦点/全局切换） | `SandboxPipeline` 的验证 → TAND → 仲裁 → SafeFallback 多阶段切换 |

**为何引用**

Trit-Core 的 `Hold` 状态不只是“未解析的中间态”，而是“知道何时该停”的主动策略。`dao-science` 从主动推理和自由能原理出发，为这种策略提供了数学化表达。

**建议融入位置**
- `docs/explanation/PHILOSOPHY.md` §4（心智是三值运算的）
- `HUMANITIES-INDEX.md` 的“悬置/Hold”词条
- `docs/how-to/CLI_REFERENCE.md` 中 `--validate-only` 和 `--dry-run` 的设计理念说明

---

### 3. 第一人称认识论

**来源**：`dao-science/1_first_principles/05_first_person_epistemology.md`

**核心内容**

- 科学方法的主流形态是第三人称的，在 L1 层面极为成功，但在 L2/L0 层面有根本盲区。
- 个体差异不是噪声，独特体验不是离群值，第一人称报告不是“merely subjective”。
- L0/L2 是不可还原的数据层。真正的科学成熟不在于用 L1/L4 吞并 L0/L2，而在于建立二者之间的严谨翻译协议。
- 提出“结构化现象学报告”的操作化标准。

**与 Trit-Core 的映射**

| dao-science 概念 | Trit-Core 对应 |
|---|---|
| L2 个体实情 | `Frame::Individual` |
| 群体平均不能替代个体 | `MedicalEthics` 中 Individual 帧优先于 Science/Consensus |
| 第三人称偏见 | 把 `Frame::Consensus` 误当成 `Frame::Science` 的风险 |
| 认识论谦逊 | `Hold` 和 `Unknown` 的诚实表达 |

**为何引用**

Trit-Core 对 `Individual` 帧的保护（尤其在 `MedicalEthics` 域）需要认识论层面的辩护。`dao-science` 提供了系统的论证：个体实情是独立的数据层，不应被统计共识否决。

**建议融入位置**
- `docs/explanation/PHILOSOPHY.md` §7.2（个体实情的权重）
- `HUMANITIES-INDEX.md` 的“个体实情”和“第一人称检验”词条
- `SECURITY.md` 中关于输入验证和个体数据保护的部分

---

### 4. 偏离代价函数

**来源**：`dao-science/1_first_principles/07_cost_of_deviation.md`

**核心内容**

偏离道的代价函数定义为：

```
Cost(π_dev) = G(π_actual) - G(π_optimal)
```

其中 `G(π)` 是预期自由能。偏离代价可分解为：
- 认知代价：信息增益减少，系统不再有效探索
- 实用代价：偏好满足减少，行动不再有效达成目标

导致偏离的三种精度错配：
1. 先验精度过高（坚持错误信念）
2. 似然精度过低（忽视感官证据）
3. 时间视野过短（短期优化，长期代价高）

**与 Trit-Core 的映射**

| dao-science 概念 | Trit-Core 对应 |
|---|---|
| 偏离代价 | `Hold` 的机会成本 + 错误 Commit 的后悔成本 |
| 先验精度过高 | 对某一 `Frame` 或 `Domain` 规则的僵化坚持 |
| 似然精度过低 | 忽略 `MetaInterrupt` 或 `Unknown` 信号 |
| 时间视野过短 | 为短期性能而关闭 `SafeFallback` 或跳过验证 |

**为何引用**

Trit-Core 目前主要关注“是否触发 Hold”，但尚未对“Hold 的代价”和“错误决策的代价”进行量化。`dao-science` 提供了可能的理论包装和度量方向。

**建议融入位置**
- `FUTURE.md` 作为未来研究方向
- `docs/explanation/PHILOSOPHY.md` §9（定论与断言的工程伦理）
- 未来 `SandboxDiagnostics` 扩展：增加“决策后悔”或“偏离代价”估计字段

---

### 5. 尺度与道德沉默

**来源**：`dao-science/1_first_principles/11_scale_and_moral_silence.md`

**核心内容**

- 同一件事在不同尺度（个人/组织/行星/宇宙）下，道德语言会失效。
- 提出“可行动的一米”原则：只在你能负责、能感知、能改变的尺度上行动。
- 避免用大尺度取消小尺度痛苦。

**与 Trit-Core 的映射**

| dao-science 概念 | Trit-Core 对应 |
|---|---|
| 尺度错配 | 跨 Frame 冲突被错误地放在同一 Domain 中仲裁 |
| 可行动的一米 | `Domain` 的选择必须匹配决策者的责任和影响范围 |
| 大尺度取消小尺度 | 用 `Consensus` 或 `Science` 平均覆盖 `Individual` 实情 |

**为何引用**

Trit-Core 的 `Domain` 系统本质上是在做“决策尺度”的划分。`dao-science` 提供了哲学语言，说明为什么同一事实在不同尺度下需要不同处理。

**建议融入位置**
- `CONFLICT_CATALOG.md` 的冲突模式说明
- `docs/explanation/CONCEPTS.md` §4（Domain 设计理由）
- 新增场景：个人尺度的 `Individual` 与组织尺度的 `Consensus` 冲突

---

## 中价值引用内容

### 6. 主动推理 / 自由能原理

**来源**：`dao-science/1_first_principles/01_dao_as_process.md`

**核心内容**

- `Dao ≡ −∇G(π)`：道是预期自由能上的梯度流。
- 策略选择：`P(π) = σ(−γ G(π))`
- “无为”对应于沿自由能梯度自然流动的策略。

**引用价值**

可作为 Trit-Core 决策动力学的数学背景。但需注意：其证据等级为 **F**（形式化），尚未经验证。不应将其作为已证实的物理定律引用。

**建议融入位置**
- `HUMANITIES-INDEX.md` 的“真知/实践真知”词条，作为背景参考
- 不直接写入 `docs/explanation/CONCEPTS.md` 的核心定义

---

### 7. 注意力作为精度优化

**来源**：`dao-science/2_models/attention_model.md`、`1_first_principles/10_de_and_ming.md`

**核心内容**

- 元参数 `α` 控制焦点注意与全局觉知之间的平衡。
- “德”是生成模型精度矩阵 `Π`。
- 安全 AI 需要“收放自如”：既能聚焦优化，又能全局监测。

**引用价值**

可为 Trit-Core 的 `MetaMonitor` 和 `SandboxPipeline` 的多阶段设计提供注意力动力学解释。

**建议融入位置**
- `docs/explanation/ARCHITECTURE.md` 的模块层说明
- `HUMANITIES-INDEX.md` 的“群体认知髓鞘化”词条

---

### 8. 碳硅共生框架

**来源**：`dao-science/4_applications/carbon_silicon_symbiosis.md`

**核心内容**

- 碳基负责 L0/L2（觉知、个体实情），硅基负责 L1/L4（物理规律、理性协作），L3 共同协作。
- AI 不应替代人类做价值判断，而应扩展人类的 L1/L4 能力。

**引用价值**

与 Trit-Core 的 `ValueJudgment` 永远 `Hold`、人类保留最终决策权的设计理念一致。

**建议融入位置**
- `docs/explanation/PHILOSOPHY.md` §6（AI 对齐的诚实问题）
- `EPISTEMIC-HUMILITY.md`

---

## 注意事项

### 证据等级

`dao-science` 使用证据等级标注：
- **F**：形式化（Formalization）
- **N**：神经证据（Neuroscience）
- **B**：行为预测（Behavioral）
- **S**：系统/仿真（Systems/Simulation）
- **M**：元伦理/规范（Meta-ethical/Normative）

引用时必须标注等级。例如 `Dao ≡ −∇G(π)` 是 **F**，不应被表述为已证实的自然定律。

### 文化/宗教负载

`dao-science` 使用大量佛教、道家术语（道、德、无为、知止、二入四行等）。Trit-Core 引用时应剥离宗教色彩，保留计算/认知科学内核。

### 立场差异

`dao-science` 明确断言当前大模型无法触及 L0（觉知本身）和 L2（个体实情）。Trit-Core 可以保持中性：在工程上，AI 无法直接观测 L2；在设计上，系统应保留 L2 的输入接口和不可侵犯性。不必接受或反驳其本体论断言。

### 成熟度

`dao-science` 版本 0.1.0，README 说明“不是已大规模验证的理论”“没有稳定 Python API”。Trit-Core 应仅将其作为概念参考，不依赖其具体数值结论或 API。

---

## 建议的下一步行动

1. **短期**
   - 在 `docs/explanation/CONCEPTS.md` 的 Frame/Domain 章节添加 L0–L7 映射脚注。
   - 在 `HUMANITIES-INDEX.md` 的相关词条中引用 `dao-science`。
   - 在 `docs/explanation/PHILOSOPHY.md` 增加“跨项目映射：trit-core 与 dao-science”一节。

2. **中期**
   - 新增 2–3 个跨层级冲突场景（如 L2 vs L3、L4 vs L5）。
   - 在 `SandboxDiagnostics` 中增加“决策层级/尺度”字段，记录当前决策所处的认知层级。

3. **长期**
   - 探索将“偏离代价函数”作为 `SandboxPipeline` 的辅助度量。
   - 研究“知止”条件如何形式化为 `Hold` 触发的补充规则。

---

## 相关文档

- [`PHILOSOPHY.md`](../PHILOSOPHY.md) — Trit-Core 深层动机
- [`HUMANITIES-INDEX.md`](HUMANITIES-INDEX.md) — 人文关键词科学化定义
- [`CONCEPTS.md`](../CONCEPTS.md) — 核心类型定义
- [`CONFLICT_CATALOG.md`](CONFLICT_CATALOG.md) — 跨域冲突模式
- [`EPISTEMIC-HUMILITY.md`](EPISTEMIC-HUMILITY.md) — 认识论谦逊声明
