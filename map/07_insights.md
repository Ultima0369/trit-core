# MOC — 洞察与哲学

> **Scope**: 所有超越技术实现的深层思考：认知科学、哲学、人文、社会批判、跨学科引用。
>
> #trit-core #aurora #insights #philosophy #cognitive-science #humanities #dao-science

---

## 认知科学

| 文件 | 位置 | 语言 | 关键内容 |
|---|---|---|---|
| [[100MS_HIJACK_MODEL]] | `aurora/01_insights/100MS_HIJACK_MODEL.md` | 中文 | 100ms vs 200ms：神经回路的自动化反应 vs 思维意识的后设叙事。 |
| [[ATTENTION_DYNAMICS]] | `aurora/02_math/ATTENTION_DYNAMICS.md` | 中文 | α 注意力动力学：注意力资本、α-ROI。 |
| [[ATTENTION_CAPITALISM]] | `aurora/01_insights/ATTENTION_CAPITALISM.md` | 中文 | 注意力资本主义的批判：平台如何劫持注意力。 |
| [[NEURAL_REWIRING_PROTOCOL]] | `aurora/01_insights/NEURAL_REWIRING_PROTOCOL.md` | 中文 | 神经重塑协议：长周期认知重构的方法论。 |
| [[N_OF_1_PROTOCOL]] | `aurora/03_methodology/N_OF_1_PROTOCOL.md` | 中文 | N-of-1 协议：个体作为样本量为1的实验。 |
| [[ENVIRONMENTAL_SHOCK]] | `aurora/01_insights/ENVIRONMENTAL_SHOCK.md` | 中文 | 环境冲击：物理约束作为非谈判边界。 |

**来源**: 大部分从 `dao-science` 项目引入。证据等级和适用位置见 [[DAO_SCIENCE_REFERENCES]]。

---

## 认识论与哲学

| 文件 | 位置 | 语言 | 关键内容 |
|---|---|---|---|
| [[PHILOSOPHY]] | `docs/explanation/PHILOSOPHY.md` | 中文 | 热力学约束、LLM 架构缺陷、三值必要性。 |
| [[EPISTEMIC_HUMILITY]] | `docs/explanation/insights/EPISTEMIC-HUMILITY.md` | 中文 | 全部内容属于"提醒"而非"指教"。 |
| [[DIALOGUE_ORIGIN]] | `docs/explanation/insights/DIALOGUE-ORIGIN.md` | 中文 | 开悟.md 如何催生 Trit-Core。 |
| [[FRAME_EPISTEMOLOGY]] | `aurora/01_insights/FRAME_EPISTEMOLOGY.md` | 中文 | 帧认识论：不同参考系不可通约。 |
| [[COGNITIVE_SOVEREIGNTY]] | `aurora/01_insights/COGNITIVE_SOVEREIGNTY.md` | 中文 | 认知主权：谁有权决定你的注意力投向。 |

---

## 社会与组织

| 文件 | 位置 | 语言 | 关键内容 |
|---|---|---|---|
| [[ORGANIZATIONAL_VORTEX]] | `aurora/01_insights/ORGANIZATIONAL_VORTEX.md` | 中文 | 组织涡旋：科层制如何消解个体智慧。 |
| [[ATTENTION_CAPITALISM]] | `aurora/01_insights/ATTENTION_CAPITALISM.md` | 中文 | 注意力资本主义：平台经济对认知的殖民。 |

---

## 人文索引与术语

| 文件 | 位置 | 语言 | 关键内容 |
|---|---|---|---|
| [[HUMANITIES_INDEX]] | `docs/explanation/insights/HUMANITIES-INDEX.md` | 中文 | 人文关键词科学化定义：可观察、可区分、可验证。 |
| [[GLOSSARY]] | `docs/explanation/insights/GLOSSARY.md` | 中文 | Trit-Core 术语表：全部自定义术语的定义。 |
| [[DAO_SCIENCE_REFERENCES]] | `docs/explanation/insights/DAO-SCIENCE-REFERENCES.md` | 中文 | dao-science 引用参考：证据等级、适用位置、注意事项。 |

---

## 未来与局限

| 文件 | 位置 | 语言 | 关键内容 |
|---|---|---|---|
| [[FUTURE]] | `docs/explanation/insights/FUTURE.md` | 中文 | 已知局限：无形式化验证、无定理证明器、无模型检查。 |
| [[CONFLICT_CATALOG]] | `docs/explanation/insights/CONFLICT_CATALOG.md` | 中文 | 跨域冲突模式分类：未来仲裁策略设计的参考案例（非学习数据，遵循"不进化"）。 |
| [[高杠杆]] | `map/高杠杆.md` | 中文 | 三个杠杆机制：真实成本显影、未来回望模拟、注意力主权训练——齿轮、支点、受力面与卡死点。 |

---

## 跨学科引用（dao-science）

| 引入内容 | 来源 | 当前位置 | 状态 |
|---|---|---|---|
| 100ms 劫持模型 | `dao-science` | `aurora/01_insights/100MS_HIJACK_MODEL.md` | 已整合 |
| α 注意力动力学 | `dao-science` | `aurora/02_math/ATTENTION_DYNAMICS.md` | 已整合 |
| N-of-1 协议 | `dao-science` | `aurora/03_methodology/N_OF_1_PROTOCOL.md` | 已整合 |
| 创造力模型 | `dao-science` | 待整合 | 规划中 |
| 证据等级系统 | `dao-science` | 待整合 | 规划中 |

完整评估见 [[DAO_SCIENCE_IMPORT_ASSESSMENT]]（`aurora/08_reports/DAO_SCIENCE_IMPORT_ASSESSMENT.md`）。

---

## 跨链连接（洞察 ↔ 代码）

| 洞察主题 | 文档 | 代码体现 |
|---|---|---|
| 100ms vs 200ms | `aurora/01_insights/100MS_HIJACK_MODEL.md` | 沙盒延迟控制、输入门控 |
| 注意力资本 | `aurora/02_math/ATTENTION_DYNAMICS.md` | 注意力引擎（预留） |
| 认知主权 | `aurora/01_insights/COGNITIVE_SOVEREIGNTY.md` | `SafeFallback::disabled()` |
| 帧认识论 | `aurora/01_insights/FRAME_EPISTEMOLOGY.md` | `Frame` enum 的 13 变体设计 |
| 环境冲击 | `aurora/01_insights/ENVIRONMENTAL_SHOCK.md` | `Physical` 域的安全回退 |
| N-of-1 | `aurora/03_methodology/N_OF_1_PROTOCOL.md` | `Individual` 帧的优先仲裁 |

---

## 阅读建议

- **想理解为什么这个项目存在**：[[PHILOSOPHY]] → [[DIALOGUE_ORIGIN]] → [[EPISTEMIC_HUMILITY]]
- **想理解认知科学基础**：[[100MS_HIJACK_MODEL]] → [[ATTENTION_DYNAMICS]] → [[N_OF_1_PROTOCOL]]
- **想理解社会批判维度**：[[ATTENTION_CAPITALISM]] → [[ORGANIZATIONAL_VORTEX]] → [[COGNITIVE_SOVEREIGNTY]]
- **想查术语**：[[GLOSSARY]] → [[HUMANITIES_INDEX]]
- **想了解 dao-science 引入**：[[DAO_SCIENCE_REFERENCES]] → [[DAO_SCIENCE_IMPORT_ASSESSMENT]]

---

**相关 MOC**: [[01_manifest]] · [[02_concepts]] · [[04_math]]

#map-of-content #insights #philosophy #cognitive-science #humanities #dao-science #attention #n-of-1
