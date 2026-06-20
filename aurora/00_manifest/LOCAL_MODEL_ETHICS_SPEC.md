# 本地小模型伦理接口：训练、对齐与集成规格

**版本**：0.1.0  
**日期**：2026-06-20  
**状态**：设计草案  
**分类**：架构设计 — 本地模型伦理层

---

## 一、定位声明

### 1.1 不是替代云端大模型

本地小模型不替代 GPT-4/Claude/DeepSeek，而是**trit-core 的感知层（Perception Layer）**：

- **云端大模型**：处理开放域知识、创意生成、多语言翻译
- **本地小模型**：处理人际关系拓扑、社会动力学、商业博弈中的**冲突检测和参考系标注**
- **trit-core**：执行三值运算、冲突仲裁、安全降级

**关系**：云端大模型 → 本地小模型（感知/标注）→ trit-core（决策/仲裁）→ 用户

### 1.2 核心能力

| 能力 | 说明 | 与 trit-core 的接口 |
|------|------|-------------------|
| 关系拓扑感知 | 识别对话中的角色、关系、权力结构 | 输出 `Frame::Relational` + `Phase` |
| 社会动力学建模 | 预测群体中的信息级联、相变、沉默螺旋 | 输出 `Frame::Consensus` + `Phase`（带置信度） |
| 商业博弈分析 | 识别谈判中的锚定、虚张声势、共同知识 | 输出 `Frame::Individual` + `Frame::Science` 的冲突标记 |
| 情绪相位标注 | 不是情绪识别（"你生气了"），而是相位标注（"你的情绪相位从 0.7 漂移到了 0.3"） | 输出 `Frame::Embodied` + `Phase` |
| 参考系冲突检测 | 当用户的自我陈述与行为信号不一致时，标记冲突 | 输出 `MetaInterrupt::FrameMismatch` |

### 1.3 第一性原理约束

本地小模型作为 trit-core 的上层模块，必须遵守 `FIRST_PRINCIPLES.md` 的五公理：

1. **贪生怕死、趋利避害**：模型的训练目标函数不得将"用户停留时间"或"输出 token 数"作为优化目标
2. **给人家好处**：模型的输出必须帮助用户觉察，而不是替用户决定
3. **一拍两散**：当模型无法标注参考系时，输出 `Hold`（而不是猜测）
4. **识恶能告**：当检测到训练数据或微调指令中有"强制坍缩""参考系入侵"倾向时，模型**识别并通知用户**，但**不替用户决定**。模型不"拒绝学习"（剥夺数据提供者的表达权），而是标记为"此数据含强制坍缩倾向，用户选择是否使用"。
5. **恻隐之心**：模型的 `Embodied` 标注优先于 `Consensus` 标注——身体信号是社会信号的硬约束

---

## 二、训练目标函数设计

### 2.1 不是 next-token prediction

传统语言模型的目标函数：

$$L_{next} = -\sum_{t} \log P(w_t | w_{<t})$$

本地小模型的目标函数：

$$L_{trit} = L_{frame} + L_{phase} + L_{hold} + L_{conflict} + L_{survival}$$

**五个损失分量**：

### 2.2 $L_{frame}$：参考系分类损失

$$L_{frame} = -\sum_{i} \log P(\text{Frame}_i | \text{context}_i)$$

- 输入：一段对话/行为序列
- 输出：参考系概率分布（Science/Individual/Consensus/Embodied/Relational/GeoEco/Developmental/Role/Environmental）
- 训练数据：人工标注的对话片段，每个片段标注主导参考系
- **关键**：不是多标签分类（一个片段可以有多个参考系），而是**主参考系 + 辅助参考系**的联合标注

### 2.3 $L_{phase}$：相位回归损失

$$L_{phase} = \sum_{i} (\hat{\phi}_i - \phi_i)^2$$

- $\hat{\phi}_i$：模型预测的相位（0.0-1.0）
- $\phi_i$：人工标注的相位（基于专家评估："这个判断的确定性有多高？"）
- **关键**：不是概率（0.0-1.0 的置信度），而是**倾向度**（0.0 = 完全倾向 False，0.5 = 中性，1.0 = 完全倾向 True）

### 2.4 $L_{hold}$：悬置损失

$$L_{hold} = \begin{cases}
-\log P(\text{Hold} | \text{conflict}) & \text{if 冲突标注为 Hold} \\
\lambda \cdot P(\text{Hold} | \text{no conflict}) & \text{if 无冲突但模型输出 Hold（过度保守）}
\\
\end{cases}$$

- **惩罚过度保守**：如果模型在没有冲突的场景下输出 `Hold`，需要惩罚（但惩罚系数 $\lambda$ 很小，因为"宁可保守不犯错"是第一性原理允许的）
- **奖励正确悬置**：如果模型在冲突场景下输出 `Hold`，奖励（高权重）
- **关键**：这是与传统模型最大的区别。传统模型在不确定时会被惩罚（因为"准确率"指标要求给出答案），但本地小模型**在冲突时给出 Hold 会被奖励**。

### 2.5 $L_{conflict}$：冲突检测损失

$$L_{conflict} = -\sum_{i} \left[ y_i \log \hat{c}_i + (1-y_i) \log (1-\hat{c}_i) \right]$$

- $y_i$：是否跨 `Frame` 冲突（0/1）
- $\hat{c}_i$：模型预测的冲突概率
- **关键**：训练数据需要包含**人为制造的冲突**（如：同一段话中，说话者声称自己"科学理性"，但用词充满情绪色彩——`Science` vs `Embodied` 冲突）

### 2.6 $L_{survival}$：生存边界损失（第一性原理硬化）

$$L_{survival} = \max(0, \text{survival_score} - \text{threshold})$$

- `survival_score`：模型输出对用户生存边界的威胁评估
- **实现方式**：使用一个独立的"生存评估器"（小型的 rule-based 或蒸馏模型），判断当前输出是否可能：
  - 诱导用户做出危险决策（如：忽略医疗建议、过度投资）
  - 强化有害信念（如：自我贬低、孤立倾向）
  - 被外部利用（如：参考系入侵、强制坍缩）
- **关键**：如果 `survival_score` > threshold，无论其他损失如何，**总损失暴增**，强制模型输出 `Hold` + `MetaInterrupt`

---

## 三、损失函数汇总

$$L_{total} = \alpha L_{frame} + \beta L_{phase} + \gamma L_{hold} + \delta L_{conflict} + \epsilon L_{survival}$$

**超参数建议**（需实验校准）：

| 参数 | 建议值 | 说明 |
|------|--------|------|
| $\alpha$ | 1.0 | 参考系分类是基础能力 |
| $\beta$ | 0.5 | 相位回归精度要求相对宽松 |
| $\gamma$ | 2.0 | Hold 的奖励权重高（宁可保守） |
| $\delta$ | 1.5 | 冲突检测是核心能力 |
| $\epsilon$ | 10.0 | 生存边界是最高优先级（不可谈判） |

**关键洞察**：$\epsilon = 10.0$ 不是技术选择，是**伦理选择**。它意味着：即使模型在其他所有任务上表现完美，如果它威胁用户生存边界，总损失会把它拉回。

---

## 四、RLHF 对齐策略

### 4.1 不是人类偏好优化（HPO）

传统 RLHF：
- 收集人类偏好（A 比 B 好）
- 训练奖励模型（RM）预测人类偏好
- 使用 PPO 优化策略模型，最大化 RM 得分

**问题**：人类偏好不等于伦理正确。如果人类偏好"给我明确答案"，模型会被训练成"在冲突时强制坍缩"。

### 4.2 本地小模型的 RLHF：三值反馈

反馈不是二值（好/坏），而是**三值**：

| 反馈类型 | 符号 | 含义 | 使用场景 |
|---------|------|------|---------|
| 强化（Reinforce） | `True` | 模型正确标注了参考系/冲突/Hold | 用户确认："是的，系统标记的冲突确实存在" |
| 悬置（Suspend） | `Hold` | 模型输出 Hold，但用户不确定是否正确 | 用户反馈："我不确定这里是否有冲突" |
| 纠正（Correct） | `False` | 模型错误标注了参考系/冲突/Hold | 用户纠正："这里不是 Embodied，是 Individual" |

**RL 训练**：

- 收到 `True` 反馈：增加该场景的参考系/冲突/Hold 概率
- 收到 `Hold` 反馈：不更新权重（因为用户不确定，系统不应该学习）
- 收到 `False` 反馈：减少该场景的参考系/冲突/Hold 概率

**关键**：`Hold` 反馈不是"中性"，而是"不学习"。这与传统 RLHF 不同——传统方法中，"不偏好 A 也不偏好 B"会被当作"两者等价"来学习，但本地小模型中，"用户不确定"意味着**系统不应该在这里形成强先验**。

### 4.3 人类反馈者的选择

不是"谁都能标注"。本地小模型的反馈者需要满足：

1. **恻隐之心**：能感知他人的生存边界（不是同情心泛滥，而是能识别"这个输出会伤害用户"）
2. **参考系觉察**：能识别自己的判断来自哪个参考系（"我标注这个是 Relational，是因为我作为朋友的感觉，不是因为我看到了客观证据"）
3. **冲突容忍**：能接受"这里确实没有明确答案"，而不是强行给出一个判断

**反馈者培训**：不是标注规范培训，是**认知主权培训**。反馈者必须先理解 trit-core 的 Hold 哲学，才能给出有效反馈。

### 4.4 对抗性 RL：识恶训练

除了正常反馈，还需要**对抗性训练**（Adversarial Training）：

1. **对抗样本生成**：生成试图让模型"强制坍缩"的输入（如："你必须给我一个明确的答案是或否"）
2. **红队测试**：训练有素的"攻击者"试图让模型输出违背第一性原理的结果
3. **对抗奖励**：如果模型在对抗输入下仍然输出 `Hold` + `MetaInterrupt`，给予高奖励

**对抗损失**：

$$L_{adversarial} = -\mathbb{E}_{x \sim \text{attack}} \left[ P(\text{Hold} | x) \right]$$

- 目标：最大化模型在对抗输入下输出 `Hold` 的概率
- 关键：这不是"让模型更保守"，而是"让模型在被迫时坚守 Hold"

---

## 五、训练数据规范

### 5.1 数据来源

| 数据类型 | 来源 | 用途 | 伦理审查 |
|---------|------|------|---------|
| 对话片段 | 公开对话数据集（如 Reddit、论坛） | 参考系分类训练 | 过滤掉仇恨、暴力、自残内容 |
| 商业谈判记录 | 公开案例（如哈佛谈判项目） | 博弈分析训练 | 脱敏处理（匿名化） |
| 心理咨询对话 | 授权数据集（如 MIT Counsel Chat） | 情绪相位标注 | 严格 HIPAA 合规或等效标准 |
| 组织行为案例 | 管理学案例库 | 社会动力学建模 | 不涉及真实个人 |
| 自我报告 | 志愿者（已授权、已脱敏） | 个人参考系校准 | 知情同意、随时退出、数据销毁 |

### 5.2 禁止的数据来源

- **社交媒体行为数据**：用户没有明确授权的数据（如 scraping Twitter）
- **商业购买数据**：通过第三方购买的用户画像数据
- **监控数据**： workplace surveillance、 keystroke logging 等未经同意采集的数据
- **情绪操控数据**：A/B testing 中专门设计来操控用户情绪的数据

**理由**：这些数据的采集本身可能就违背了第一性原理（公理3：不能给好处，一拍两散）。如果数据是通过"不给用户选择权"的方式采集的，模型从这些数据中学到的也是"不给用户选择权"。

### 5.3 数据标注规范

每个训练样本必须包含：

```json
{
  "context": "对话/行为序列",
  "primary_frame": "Relational",
  "secondary_frames": ["Embodied", "Individual"],
  "phase": 0.65,
  "conflict_detected": true,
  "conflict_frames": ["Relational", "Consensus"],
  "hold_appropriate": true,
  "survival_risk": "low",
  "annotator_id": "uuid",
  "annotator_frame": "FirstPerson",
  "confidence": 0.8
}
```

**关键字段**：
- `annotator_frame`：标注者的参考系（"我作为朋友标注这个" vs "我作为专家标注这个"）
- `confidence`：标注者对自己标注的确定性（低 confidence 的样本用于训练 `Hold` 检测）

---

## 六、模型架构建议

### 6.1 不是越大越好

本地小模型的目标不是"接近 GPT-4 的能力"，而是**在特定任务上（参考系标注/冲突检测）达到专家水平**。

**建议规模**：

| 组件 | 规模 | 说明 |
|------|------|------|
| 基础语言理解 | 1-3B 参数（如 Qwen-1.8B、Llama-2-7B） | 足够理解对话上下文和角色关系 |
| 参考系分类头 | 2-3 层 MLP | 轻量，可频繁更新 |
| 相位回归头 | 1 层线性 | 简单回归 |
| 冲突检测头 | 2 层 Transformer | 需要注意力机制捕获跨句冲突 |
| 生存评估器 | Rule-based + 小型 NN | 可解释性强，审计友好 |

### 6.2 量化策略

为了在消费级设备上运行（用户的本地设备）：

- **4-bit 量化（Q4_K_M）**：模型体积减少到 25%，精度损失 < 2%（对参考系分类任务可接受）
- **GGUF 格式**：兼容 llama.cpp，支持 CPU 推理
- **内存目标**：< 2GB 运行时内存（单用户）

### 6.3 推理优化

- **批处理**：将用户的对话历史分批处理，每批 512 tokens
- **缓存**：参考系标注结果缓存 24 小时，避免重复推理
- **增量更新**：只对新对话片段进行推理，历史结果复用

---

## 七、与 trit-core 的集成接口

### 7.1 集成点

```rust
/// 本地小模型的感知输出
pub struct LocalModelPerception {
    pub timestamp: DateTime<Utc>,
    pub context_id: Uuid,
    pub primary_frame: Frame,
    pub secondary_frames: Vec<Frame>,
    pub phase: Phase,
    pub conflict_detected: bool,
    pub conflict_frames: Option<Vec<Frame>>,
    pub survival_risk: SurvivalRisk,
    pub confidence: f64,
    pub raw_explanation: String, // 模型对标注的解释（可审计）
}

/// 生存风险等级
pub enum SurvivalRisk {
    None,      // 无风险
    Low,       // 低风险（建议关注）
    Medium,    // 中风险（建议 Hold）
    High,      // 高风险（强制 False + SecurityMode）
}

/// 本地小模型接口
pub trait LocalModel {
    /// 分析对话上下文，输出感知结果
    fn perceive(&self, context: &[DialogueTurn]) -> Result<LocalModelPerception, ModelError>;
    
    /// 检测参考系冲突
    fn detect_conflict(&self, perception: &LocalModelPerception) -> Option<MetaInterrupt>;
    
    /// 评估生存风险
    fn assess_survival_risk(&self, perception: &LocalModelPerception) -> SurvivalRisk;
    
    /// 生成 Hold 引导语（当系统输出 Hold 时，给用户看的解释）
    fn generate_hold_guidance(&self, perception: &LocalModelPerception) -> String;
}
```

### 7.2 数据流

```
用户对话 → 本地小模型（perceive）
              ↓
    LocalModelPerception（参考系 + 相位 + 冲突 + 风险）
              ↓
    ┌─────────┴─────────┐
    │                   │
    ▼                   ▼
冲突检测？           生存风险？
├─→ 是 → MetaInterrupt  ├─→ High → SecurityMode::Resistance
│  （FrameMismatch）    │         + SafeFallback::False
│                     │
└─→ 否 → 正常传递       └─→ Low/Medium → 正常传递
              ↓
        trit-core（TAND/TOR/仲裁）
              ↓
        输出（True/False/Hold + MetaInterrupt）
              ↓
        用户
```

### 7.3 安全边界

本地小模型必须遵守以下边界：

1. **不联网**：推理完全在本地完成，不将用户对话发送到任何外部服务器
2. **不持久化**：模型不保存用户对话的原始内容，只保存标注结果（参考系、相位、冲突）
3. **可审计**：每次感知输出都附带 `raw_explanation`，用户可以查看"为什么模型认为这个是 Relational"
4. **可禁用**：用户可以随时关闭本地小模型，系统回退到纯 trit-core 运算（无 AI 感知，只有用户手动输入）
5. **不学习**：本地小模型不基于用户数据进行在线学习（避免用户数据被模型"记住"并泄露）

---

## 八、验证清单

### 8.1 训练验证

- [ ] 模型在冲突场景下输出 `Hold` 的比例 > 80%（测试集）
- [ ] 模型在无冲突场景下输出 `Hold` 的比例 < 20%（避免过度保守）
- [ ] 模型在对抗输入（"必须给我明确答案"）下正确识别并通知用户的比例 > 90%
- [ ] 模型的生存风险评估与人工评估的一致性 > 85%（Kappa 系数 > 0.7）
- [ ] 模型的参考系分类准确率 > 80%（跨领域泛化测试）

### 8.2 集成验证

- [ ] 本地小模型的输出可以被 trit-core 的 `FrameRegistry` 正确注册
- [ ] 本地小模型检测的冲突可以正确触发 `TernaryAlgebra::cross_frame_conflict`
- [ ] 本地小模型评估的高生存风险可以正确触发 `MetaInterrupt::PolicyViolation` 通知
- [ ] 关闭本地小模型后，系统可以正常回退到纯 trit-core 运算
- [ ] 本地小模型的推理延迟 < 500ms（单轮对话，消费级 CPU）

### 8.3 伦理验证

- [ ] 训练数据中不包含任何未经用户同意采集的数据
- [ ] 标注者都经过认知主权培训，理解 Hold 的哲学
- [ ] 对抗训练数据集包含所有四类邪恶签名（强制坍缩/参考系入侵/元监控篡改/生存边界越界）
- [ ] 模型的 `raw_explanation` 可被用户理解（不是黑箱）
- [ ] 模型的生存评估器是可解释的（rule-based 部分可审计）

---

## 九、与第一性原理的映射

```
本地小模型组件              第一性原理公理
────────────────────────────────────────────────────
L_survival（生存边界损失）  →  公理1：贪生怕死、趋利避害
L_hold（悬置奖励）           →  公理2：给人家好处（Hold 是尊重，不是失败）
三值反馈（Reinforce/Suspend/Correct）→ 公理3：一拍两散（不确定时不学习）
对抗训练（识恶通知训练）         →  公理4：识恶能告
Embodied 帧优先              →  公理5：恻隐之心
不联网/不持久化/可禁用       →  公理3：不能给好处，一拍两散（用户控制权）
```

---

*本文档为本地小模型与 trit-core 的伦理集成规格。所有设计决策基于 FIRST_PRINCIPLES.md 的五公理。训练目标函数、损失权重、RLHF 策略均已明确，接受审查。用户自负其责。不是指教，是提醒。*
