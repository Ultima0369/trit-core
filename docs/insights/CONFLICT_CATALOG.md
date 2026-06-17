# CONFLICT CATALOG — 跨域冲突模式分类

本文档系统性地记录 Trit-Core 中常见的跨域冲突模式。每个模式包含：冲突双方、仲裁逻辑、真实世界案例。

这既是文档，也是未来仲裁策略学习的训练数据雏形。

---

## 分类体系

冲突按参与帧分类：

```
模式编码: <Frame1>-<Frame2>
严重程度: L1 (可协商) | L2 (需仲裁) | L3 (不可通约)
```

---

## 模式 1: Science vs Individual

**编码**: SCI-IND
**严重程度**: L2（需仲裁）

### 冲突本质

统计事实 vs 个人存在性事实。科学提供的是"对 100 个人的平均效果"，个人提供的是"对这个特定的人意味着什么"。

### 仲裁规则

| Domain | 结果 |
|---|---|
| MedicalEthics | Preserve(Individual) — 患者自主权优先 |
| Physical | Commit(Science) — 物理定律不谈判 |
| Engineering | Commit(Science) — 安全系数不妥协 |
| ValueJudgment | Hold — 不可通约 |
| General | Negotiate |

### 真实案例

- 患者拒绝标准化疗（临床指南 vs 个人意愿）
- 疫苗强制令（群体免疫数据 vs 个人身体自主权）
- 基因检测（统计风险 vs 知情权/不知情权）

---

## 模式 2: Science vs Consensus

**编码**: SCI-CON
**严重程度**: L2（需仲裁）

### 冲突本质

经验证据 vs 群体信念。当科学共识与公众舆论不一致时。

### 仲裁规则

| Domain | 结果 |
|---|---|
| Physical | Commit(Science) — 物理定律不因投票改变 |
| Engineering | Commit(Science) — 安全系数不因舆论妥协 |
| MedicalEthics | Negotiate — 需要考虑公共卫生沟通 |
| ValueJudgment | Hold |
| General | Negotiate |

### 真实案例

- 桥梁安全：材料疲劳数据 vs 社区反对封桥
- 气候变化：物理数据 vs 经济利益驱动的舆论
- 疫苗安全性：临床试验数据 vs 公众怀疑

---

## 模式 3: Individual vs Consensus

**编码**: IND-CON
**严重程度**: L2（需仲裁）

### 冲突本质

个人选择 vs 社会期望。少数派权利 vs 多数派规范。

### 仲裁规则

| Domain | 结果 |
|---|---|
| MedicalEthics | Preserve(Individual) |
| Physical | 取决于 Science 是否存在 |
| Engineering | 取决于 Science 是否存在 |
| ValueJudgment | Hold |
| General | Negotiate |

### 真实案例

- 职业选择：个人艺术追求 vs 社会对"体面工作"的期望
- 生活方式选择：个人价值观 vs 文化规范
- 终末期医疗决策：患者意愿 vs 家属意见

---

## 模式 4: Science vs Absolute

**编码**: SCI-ABS
**严重程度**: L3（不可通约）

### 冲突本质

可知 vs 不可知。当科学方法触及认知边界时。

### 仲裁规则

所有 Domain → Absolute 帧必须 Hold（MetaMonitor 强制执行）。

### 真实案例

- 宇宙起源：大爆炸理论 vs "之前是什么"（不可观测）
- 意识本质：神经科学 vs "什么是主观体验"（困难问题）
- AGI 意识：功能测试 vs "它真的有意识吗"（不可知）

---

## 模式 5: 三方冲突

**编码**: SCI-IND-CON
**严重程度**: L3（不可通约）

### 冲突本质

三种不同参考系同时冲突。这是最复杂的决策场景。

### 仲裁规则

| Domain | 结果 |
|---|---|
| MedicalEthics | Preserve(Individual) |
| Physical | Commit(Science) |
| ValueJudgment | Hold |
| General | Negotiate |

### 真实案例

- 终末期患者请求实验性治疗：科学证据（低成功率）、个人意愿（想尝试一切）、社会共识（资源分配公平性）
- 疫苗强制令三方冲突：科学（有效）、个人（拒绝）、共识（多数人支持强制）

---

## 模式 6: Unknown 传播

**编码**: UNK-ANY
**严重程度**: L1（可检测，但不可计算）

### 冲突本质

任何帧的信号与 Unknown 信号相遇。Unknown 代表超出系统认知范围的输入。

### 行为

- TAND: Unknown 传染（一个未知因素污染整个合取链）
- TOR: True 覆盖 Unknown（已知的肯定可以覆盖未知）
- SafeFallback: 危险域中 Unknown + 中断 → 强制 False

### 真实案例

- 传感器故障：一个传感器返回"无法识别"的数据
- 新化学物质：没有先例的化合物安全性评估
- 外星信号：无法分类的输入模式

---

## 统计摘要

| 模式 | 严重程度 | 仲裁策略 |
|---|---|---|
| SCI-IND | L2 | 域相关（MedicalEthics→Individual，Physical→Science） |
| SCI-CON | L2 | 域相关（Physical/Engineering→Science） |
| IND-CON | L2 | 域相关（MedicalEthics→Individual） |
| SCI-ABS | L3 | 无条件 Hold |
| SCI-IND-CON | L3 | 域相关 |
| UNK-ANY | L1 | TAND 传染 / TOR True 覆盖 / SafeFallback |

---

## 贡献新冲突模式

如果你发现新的冲突模式，请提交 PR 添加到此文件。格式：

```markdown
## 模式 N: FrameA vs FrameB

**编码**: FRMA-FRMB
**严重程度**: L1|L2|L3

### 冲突本质
简要描述

### 仲裁规则
| Domain | 结果 |
|---|---|

### 真实案例
- 案例 1
- 案例 2
```
