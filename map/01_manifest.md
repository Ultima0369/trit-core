# MOC — 宪章、原则与认知架构

> **Scope**: 项目存在的根本理由、不可谈判的底线、以及心智架构的层级模型。
> 
> #trit-core #aurora #manifest #charter #principles #cognitive-architecture

---

## 核心宪章

| 文件 | 位置 | 语言 | 关键内容 |
|---|---|---|---|
| [[NARRATIVE_CHARTER]] | `docs/NARRATIVE_CHARTER.md` | 中文 | 叙事基准：长见识输入源 / 反注意力 / 开源免费 / 毕业即成功。统一全部文档动机口径。 |
| [[CHARTER]] | `aurora/00_manifest/CHARTER.md` | 中文 | 四条底线：不剥夺、不自欺、不进化、公开可审查。定稿锁定。 |
| [[FIRST_PRINCIPLES]] | `aurora/00_manifest/FIRST_PRINCIPLES.md` | 中文 | 第一性原理：三值逻辑、熵减标准、认知主权、元监控可审查。 |
| [[COGNITIVE_ARCHITECTURE_LAYERS]] | `aurora/00_manifest/COGNITIVE_ARCHITECTURE_LAYERS.md` | 中文 | L1-L5 分层模型：锚定层 → 钩子层 → 适配层 → 核心层 → 元层。 |

---

## 项目定位声明

| 文件 | 位置 | 语言 | 关键内容 |
|---|---|---|---|
| [[AURORA_MANIFEST]] | `aurora/00_manifest/AURORA_MANIFEST.md` | 中文 | Aurora 的存在理由：长见识输入源 + 注意力主权训练系统。开源免费、不争注意力、自我筛选；本地优先、数据主权、工具而非代理。 |
| [[PHILOSOPHY]] | `docs/explanation/PHILOSOPHY.md` | 中文 | Trit-Core 的深层动机：热力学约束、LLM 架构缺陷、三值必要性。 |

---

## 伦理规范

| 文件 | 位置 | 语言 | 关键内容 |
|---|---|---|---|
| [[LOCAL_MODEL_ETHICS_SPEC]] | `aurora/00_manifest/LOCAL_MODEL_ETHICS_SPEC.md` | 中文 | 本地模型伦理约束：用户数据不上传、加密、可删除。 |
| [[SECURITY_MODEL]] | `aurora/03_whitepaper/SECURITY_MODEL.md` | 中文 | 安全模型：SQLCipher（AES-256-CBC + HMAC-SHA256，M1 启用）+ 本地优先 + 四态 SecurityMode。 |
| [[009-ethics-hardening]] | `aurora/05_adr/009-ethics-hardening.md` | 中文 | 伦理硬化：从安全模式→透明模式，系统不阻断运算。 |

---

## 认识论立场

| 文件 | 位置 | 语言 | 关键内容 |
|---|---|---|---|
| [[EPISTEMIC_HUMILITY]] | `docs/explanation/insights/EPISTEMIC-HUMILITY.md` | 中文 | 全部内容属于"提醒"而非"指教"。 |
| [[DIALOGUE_ORIGIN]] | `docs/explanation/insights/DIALOGUE-ORIGIN.md` | 中文 | 开悟.md 如何催生 Trit-Core：三次击碎→可操作认知技术。 |

---

## 跨链连接（代码 ↔ 文档）

| 概念 | 文档位置 | 代码位置 |
|---|---|---|
| 四条底线 | `aurora/00_manifest/CHARTER.md` | `src/meta/safe_fallback.rs`（可关闭 = 不剥夺） |
| L1-L5 架构 | `aurora/00_manifest/COGNITIVE_ARCHITECTURE_LAYERS.md` | `src/` 目录结构（L1=anchor, L2=hook, L3=adapters, L4=core+meta, L5=feedback） |
| 伦理模式 | `aurora/05_adr/009-ethics-hardening.md` | `src/security/mod.rs`（SecurityMode enum） |

---

**相关 MOC**: [[02_concepts]] · [[03_adr]] · [[07_insights]]

#map-of-content #manifest #charter #first-principles #ethics #epistemology
