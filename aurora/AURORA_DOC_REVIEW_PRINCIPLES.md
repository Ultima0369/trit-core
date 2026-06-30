# Aurora 文档审查：基于第一性原理的重新对齐

**版本**：0.1.0  
**日期**：2026-06-20  
**审查依据**：`FIRST_PRINCIPLES.md` v0.2.0  
**范围**：`trit-core/aurora/` 全部 47 份文档

---

## 一、审查方法论

### 审查标准

每份文档回答以下问题：

1. **价值对齐**：是否支持"给人家好处，短期物质，长期开智"？
2. **冲突保留**：是否支持"不能给好处，一拍两散"？是否保留 `Hold` 的合法性？
3. **生存优先**：是否将贪生怕死、趋利避害作为默认合法性？
4. **识恶能战**：是否定义了系统对抗邪恶输入的能力？
5. **恻隐之心**：是否尊重身体/个体信号优先于群体统计？

### 处置分类

| 标记 | 含义 | 行动 |
|------|------|------|
| **保留** | 完全对齐，无需修改 | 维持现状 |
| **重写** | 核心逻辑需要对齐 | 重写关键章节 |
| **删减** | 与第一性原理冲突或冗余 | 删除或合并 |
| **新增** | 缺少必要内容 | 补充新文档 |
| **合并** | 两份文档可以合并 | 合并后保留一份 |

---

## 二、逐层审查

### 00_manifest — 项目宣言（2份）

| 文档 | 状态 | 说明 | 行动 |
|------|------|------|------|
| AURORA_MANIFEST.md | **重写** | 商业模式和订阅制定价与第一性原理无直接冲突，但缺少"阳谋"声明——动机、愿景、机制未明确邀请审查。技术原则（本地优先、数据主权）对齐，但"加密：AES-256-GCM"是错误（P2-004）。 | 重写第1节"我们为什么存在"，增加"阳谋声明"；修正加密声明；增加引用 `FIRST_PRINCIPLES.md` 的链接。 |
| FIRST_PRINCIPLES.md | **保留** | 本文档是审查基准，无需修改。 | 维持现状。 |

**新增需求**：`ETHICS_STATEMENT.md` — 独立文档，将第一性原理的"阳谋"性质明确化，面向外部审查者（投资人、合作者、监管机构）。

---

### 01_insights — 洞见层（5份）

| 文档 | 状态 | 说明 | 行动 |
|------|------|------|------|
| COGNITIVE_SOVEREIGNTY.md | **保留** | 核心概念与第一性原理完全对齐。"三层结构"（数据主权/参考系主权/元监控主权）直接映射到公理2（给人家好处）和公理5（恻隐）。 | 维持现状，增加对 `FIRST_PRINCIPLES.md` 的引用。 |
| ENVIRONMENTAL_SHOCK.md | **重写** | 核心洞见（环境相位冲击）对齐，但存在 `Frame::Self` 错误（P2-006）。第6.2节"冲击检测的 Trit-Core 协议"中 `delta_phi > 0.5` 强制 `Hold` 的逻辑需要与新的 `SecurityMode` 对齐。 | 修正 `Frame::Self` → `Frame::Individual`；增加对 `SecurityMode` 的引用（当冲击等级为"毁灭级"时，系统进入 SafeMode）。 |
| ORGANIZATIONAL_VORTEX.md | **保留** | 组织涡旋动力学与第一性原理的"识恶能战"有深层对应——组织失稳条件 $R/d_s > 1.5$ 是系统级邪恶检测。 | 增加对 `FIRST_PRINCIPLES.md` 公理4的引用。 |
| ATTENTION_CAPITALISM.md | **保留** | 对注意力资本主义的批判与第一性原理完全对齐。"收割的完成态"（用户用被植入的框架评判自己是否被收割）是公理3（一拍两散）的社会学对应。 | 维持现状。 |
| FRAME_EPISTEMOLOGY.md | **保留** | Frame 认识论是协议层的哲学基础，与第一性原理的公理5（恻隐）和公理2（给人家好处）对齐。 | 维持现状。 |

---

### 02_math — 数学支持（4份）

| 文档 | 状态 | 说明 | 行动 |
|------|------|------|------|
| FIELD_EQUATIONS.md | **重写** | $d_s$ 公式量纲不一致（P2-001）。"类比模型"的标注缺失。但更大的问题是：场方程与第一性原理的映射不够直接——组织涡旋的"失稳"如何触发系统的 `PolicyViolation`？ | 增加"类比模型"标注；增加第3章"场方程与第一性原理的映射"：组织失稳 $R/d_s > 1.5$ → `MetaInterrupt::PolicyViolation`（Awareness 通知）；信息激波 $M > 1$ → `MetaInterrupt::PolicyViolation`（ForcedCollapse）。 |
| WAVELET_ANALYSIS.md | **重写** | CWT 复杂度标注不准确（P3-001）。缺少内存策略（P3-003）。与第一性原理的映射不够：小波分析的"基频漂移"如何触发 `Hold`？"频谱重构"如何对应 `SecurityMode`？ | 修正 CWT 复杂度；增加内存策略；增加第6章"小波特征与第一性原理的映射"：基频漂移 → `Phase` 变化 → `MetaInterrupt::PhaseDrift`；毁灭级频谱重构 → `MetaInterrupt::PolicyViolation`（Awareness 通知）。 |
| INFORMATION_THEORY.md | **重写** | $F_{trinary}$ 公式不严格（P2-002）。需要与第一性原理对齐：信息论中的"压缩"与"强制坍缩"是同一现象的数学表述。 | 重写第3.2节，使用 KL 散度框架；增加第6章"信息论与第一性原理：邪恶即信息损耗"——当系统被强制坍缩时，信息损耗率 $H_{loss}$ 达到最大值，对应 `PolicyViolation`。 |
| PHASE_ARITHMETIC.md | **保留** | Phase 算术的形式化与代码一致，但 `Phase::new_clamped` 文档与代码不一致（P3-002）。与第一性原理的映射在"Phase 与 TritValue 的转换"部分需要明确：转换不是自动的，是用户主权的选择。 | 修正 `new_clamped` 文档；增加第4.3节"Phase 转换与用户主权：系统不替用户决定转换时机"。 |

---

### 03_whitepaper — 技术白皮书（5份）

| 文档 | 状态 | 说明 | 行动 |
|------|------|------|------|
| EXECUTIVE_SUMMARY.md | **重写** | 货币符号混用（P2-003）。核心问题是：执行摘要没有体现第一性原理的"阳谋"性质。它读起来像一个普通 SaaS 的一页纸，而不是一个公开邀请审查的伦理系统。 | 重写"一句话"部分：从"Aurora 是认知时代的仪表盘"改为"Aurora 是认知主权的协议层：阳谋设计，公开审查，用户自负其责"。修正货币符号。 |
| PROTOCOL_SPEC.md | **重写** | 扩展 Frame 定义（第3.2节）需要与 `src/core/frame.rs` 的修改同步。扩展 Domain 定义（第4.2节）需要与 `src/meta/domain.rs` 的修改同步。缺少 `SecurityMode` 的描述。 | 重写第3.2节（移除 `From<AuroraFrame> for Frame`）；增加第11章"安全模式：SecurityMode 状态机"；修正所有与代码不一致的 API 签名。 |
| ARCHITECTURE.md | **保留** | 五层架构设计合理，但需要增加 `SecurityMode` 作为第6层或嵌入到 Trit-Core 层。 | 增加 `SecurityMode` 在架构图中的位置；增加"安全层"作为跨层监控。 |
| API_CONTRACT.md | **重写** | `MetaInterrupt` 的 `ConflictType` 字段未说明（P3-006）。缺少 `SecurityMode` 的 API。缺少扩展 Frame/Domain 的序列化规范。 | 增加 `ConflictType` 详细说明；增加 `SecurityMode` API；增加 `AuroraFrame` 的序列化规范。 |
| SECURITY_MODEL.md | **重写** | SQLite 加密声明错误（P2-004）。安全模型需要与第一性原理对齐：当前的威胁分析是传统的 CIA 模型（机密性/完整性/可用性），但第一性原理要求的是**生存边界保护**（不可覆盖的 SafeFallback、安全模式、元监控只读）。 | 重写：从 CIA 模型转向"生存边界模型"——威胁分析增加"强制坍缩""参考系入侵""元监控篡改""生存边界越界"四类；修正加密声明；增加 `SecurityMode` 的交互流程（Awareness 通知，不阻断）。 |

---

### 04_engineering — 工程规格（5份）

| 文档 | 状态 | 说明 | 行动 |
|------|------|------|------|
| SYSTEM_DESIGN.md | **重写** | `RawSignal` 重复定义（P2-007）。模块设计中缺少 `security` 模块。技术栈中缺少 `SecurityMode` 的实现层。 | 统一 `RawSignal` 定义；增加 `aurora_core::security` 模块；增加 `SecurityMode` 状态机的实现位置。 |
| DATA_MODEL.md | **保留** | 数据模型与第一性原理无直接冲突，但需要增加 `SecurityMode` 的持久化（JSON 字段在 `users` 表中）。 | 增加 `security_mode`（Awareness/Transparency/Normal）和 `security_since` 字段。 |
| PIPELINE_DESIGN.md | **保留** | 管道设计合理，但需要增加安全检查节点（在 Trit-Core 运算前增加 `security_check`）。 | 增加"安全检查节点"在管道中的位置（Awareness 通知，不阻断）。 |
| TESTING_STRATEGY.md | **重写** | 代码示例错误（P2-005）。测试策略缺少"伦理门禁测试"（`FIRST_PRINCIPLES.md` 第6.1节）。 | 修正代码示例；增加第3章"伦理门禁测试：不可谈判的测试用例"。 |
| DEPLOYMENT_GUIDE.md | **保留** | 部署架构与第一性原理无直接冲突，但企业部署需要增加"安全模式下的远程管理"（管理员如何远程确认安全模式）。 | 增加企业部署中的 Awareness 通知管理流程。 |

---

### 05_adr — 架构决策记录（8+1份）

| 文档 | 状态 | 说明 | 行动 |
|------|------|------|------|
| 001-008 | **保留** | 现有 ADR 与第一性原理对齐。 | 维持现状。 |
| **009-ethics-hardening.md** | **新增** | 需要新增 ADR-009：记录伦理硬化的决策过程——为什么增加 `SecurityMode`、为什么 `SafeFallback` 不可覆盖、为什么扩展 `Frame` enum 而非 wrapper。 | 新建。内容已输出到 `trit-core/aurora/05_adr/009-ethics-hardening.md`。 |

---

### 06_roadmap — 路线图与验收标准（6份）

| 文档 | 状态 | 说明 | 行动 |
|------|------|------|------|
| MILESTONES.md | **重写** | 里程碑定义缺少"伦理门禁"作为验收条件。M0 的验收标准缺少 `SecurityMode` 的验证。核心数据未说明来源（P4-004）。 | 在每个里程碑的 Exit Criteria 中增加"伦理门禁测试通过"；增加 `SecurityMode` 的阶段性目标（M0 实现基础 Awareness 通知，M1 实现完整 Transparency，M2 实现不可覆盖 SafeFallback）。 |
| M0-M4_EXIT_CRITERIA.md | **保留** | 各阶段验收标准框架合理，但需要增加"伦理门禁"作为硬性门槛。 | 每个 Exit Criteria 中增加：`[ ] 伦理门禁测试全部通过`。 |

---

### 07_specs — 详细模块规格（6份）

| 文档 | 状态 | 说明 | 行动 |
|------|------|------|------|
| DATA_INGESTION_SPEC.md | **保留** | 数据采集与第一性原理无直接冲突。 | 维持现状。 |
| WAVELET_ENGINE_SPEC.md | **保留** | 小波引擎规格与第一性原理无直接冲突，但内存策略（P3-003）需要补充。 | 补充内存策略。 |
| FRAME_MODEL_SPEC.md | **重写** | `contamination_ratio` 公式可能超出阈值范围（P3-004）。角色边界监控与第一性原理的"恻隐"（公理5）需要更深层映射——角色入侵是对个体生存动机的威胁。 | 修正 `contamination_ratio` 公式；增加"角色入侵与生存边界"章节。 |
| TRIT_CORE_INTEGRATION_SPEC.md | **重写** | P1 阻塞错误：`From<AuroraFrame> for Frame` 映射到 `Meta`（P1-001）。缺少 `SecurityMode` 的集成。缺少扩展 `ConflictType` 的规格。 | 重写第2.1节（移除 `From` 实现，改为直接扩展 `Frame`）；增加第2.4节"扩展 ConflictType"；增加第3章"SecurityMode 集成"。 |
| UI_SPEC.md | **保留** | UI 规格与第一性原理无直接冲突，但需要增加安全模式下的用户界面（显示状态、恢复选项）。 | 增加"安全模式 UI"章节。 |
| ALERT_ENGINE_SPEC.md | **保留** | 告警引擎与第一性原理无直接冲突，但需要增加 `PolicyViolation` 告警（最高优先级）。 | 增加 `PolicyViolation` 告警类型和响应流程。 |

---

### 08_reports — 报告模板（4份）

| 文档 | 状态 | 说明 | 行动 |
|------|------|------|------|
| VALIDATION_REPORT_TEMPLATE.md | **保留** | 验证报告模板与第一性原理无直接冲突。 | 增加"伦理验证"章节。 |
| SECURITY_AUDIT_TEMPLATE.md | **保留** | 安全审计模板与第一性原理无直接冲突。 | 增加"生存边界审计"章节（检查 SafeFallback 是否被覆盖、SecurityMode 是否正常工作）。 |
| PERFORMANCE_REPORT_TEMPLATE.md | **保留** | 性能报告模板与第一性原理无直接冲突。 | 增加"伦理门禁测试性能"（不可因性能优化而跳过安全检查）。 |
| RETROSPECTIVE_TEMPLATE.md | **保留** | 复盘模板与第一性原理无直接冲突。 | 增加"伦理对齐回顾"章节。 |

---

## 三、审查统计

| 处置 | 数量 | 占比 | 文档 |
|------|------|------|------|
| **保留** | 14 | 30% | COGNITIVE_SOVEREIGNTY, ORGANIZATIONAL_VORTEX, ATTENTION_CAPITALISM, FRAME_EPISTEMOLOGY, PHASE_ARITHMETIC, DATA_MODEL, PIPELINE_DESIGN, DEPLOYMENT_GUIDE, 001-008, M0-M4_EXIT_CRITERIA, DATA_INGESTION_SPEC, WAVELET_ENGINE_SPEC, UI_SPEC, ALERT_ENGINE_SPEC, 4 reports |
| **重写** | 12 | 26% | AURORA_MANIFEST, ENVIRONMENTAL_SHOCK, FIELD_EQUATIONS, WAVELET_ANALYSIS, INFORMATION_THEORY, EXECUTIVE_SUMMARY, PROTOCOL_SPEC, SECURITY_MODEL, SYSTEM_DESIGN, TESTING_STRATEGY, MILESTONES, FRAME_MODEL_SPEC, TRIT_CORE_INTEGRATION_SPEC |
| **删减** | 0 | 0% | — |
| **新增** | 2 | 4% | FIRST_PRINCIPLES.md（已存在）, ETHICS_STATEMENT.md（新增）, ADR-009（已输出） |
| **合并** | 0 | 0% | — |
| **待评估** | 1 | 2% | AURORA_TECHNICAL_AUDIT_v1.md（本报告本身，完成使命后归档） |

**总计**：47 份文档中，14 份保留、12 份重写、2 份新增，其余为状态变化或已归档。

---

## 四、优先级排序

### 立即执行（本周）

1. **TRIT_CORE_INTEGRATION_SPEC.md** — P1 阻塞错误，必须先修复
2. **TESTING_STRATEGY.md** — 代码示例错误，影响开发者上手
3. **ENVIRONMENTAL_SHOCK.md** — `Frame::Self` 错误

### 短期（M0 之前）

4. **AURORA_MANIFEST.md** — 增加阳谋声明，修正加密声明
5. **EXECUTIVE_SUMMARY.md** — 重写核心定位，修正货币符号
6. **PROTOCOL_SPEC.md** — 增加 SecurityMode 章节，修正 Frame 扩展
7. **SECURITY_MODEL.md** — 重写为生存边界模型
8. **FIELD_EQUATIONS.md** — 增加类比标注，增加与第一性原理的映射
9. **INFORMATION_THEORY.md** — 重写保真度公式
10. **FRAME_MODEL_SPEC.md** — 修正 contamination_ratio，增加生存边界映射
11. **SYSTEM_DESIGN.md** — 统一 RawSignal，增加 security 模块
12. **MILESTONES.md** — 增加伦理门禁验收条件

### 中期（M1 之前）

13. **WAVELET_ANALYSIS.md** — 修正复杂度，增加内存策略，增加第一性原理映射
14. **API_CONTRACT.md** — 增加 ConflictType 说明、SecurityMode API、序列化规范
15. **ARCHITECTURE.md** — 增加 SecurityMode 层
16. **DATA_MODEL.md** — 增加 security_mode 字段
17. **UI_SPEC.md** — 增加安全模式 UI
18. **ALERT_ENGINE_SPEC.md** — 增加 PolicyViolation 告警
19. **各报告模板** — 增加伦理验证/审计/回顾章节
20. **ETHICS_STATEMENT.md** — 新建阳谋声明文档

---

## 五、审查结论

Aurora 文档系统在第一性原理的框架下，**约 30% 可直接保留，约 26% 需要重写**。这不是文档质量差，而是**第一性原理的引入重新定义了系统的边界**。

关键变化：
1. **从 CIA 安全模型到生存边界模型**：传统的机密性/完整性/可用性不足以保护认知主权。需要增加"强制坍缩""参考系入侵""元监控篡改""生存边界越界"四类威胁。
2. **从 Hold 到 Resistance**：`Hold` 不再是"我不知道"，而是"我拒绝替你决定"。当外部试图强制时，系统进入 `Resistance` → `SafeMode` → `Recovery`。
3. **从可选功能到不可谈判约束**：`SafeFallback` 不可覆盖、`SecurityMode` 不可自动恢复、`MetaMonitor` 只读——这些是架构的硬化，不是用户可选配置。

这些变化会让 Aurora 文档读起来**不像一个产品文档，而像一个协议宪章**。这正是第一性原理的要求：动机、愿景、机制全部公开，邀请全方位审查。如果是谋略，那就是阳谋。

---

*本审查基于 FIRST_PRINCIPLES.md v0.2.0 的五公理和四态。所有文档已逐份评估，给出具体修改指令。用户自负其责。*
