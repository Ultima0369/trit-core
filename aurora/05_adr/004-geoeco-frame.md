# ADR-004：地理生态参考系的引入（GeoEco Frame）

**状态**：已接受
**日期**：2026-06-20
**分类**：05_adr — 架构决策记录

---

## 背景

Aurora 的核心假设是人的认知被环境（地理、气候、生态）塑造。但 Trit-Core 原有 Frame 系统没有环境相关的参考系。需要决定是否引入新的 Frame，以及引入多少个。

## 决策

**引入 `GeoEco` Frame（地理生态参考系），同时引入 `Developmental`（成长轨迹）、`Role`（角色）、`Environmental`（环境状态）三个扩展 Frame。**

## 考虑的选项

### 选项A：不引入新 Frame，用现有 Frame 映射

**映射方案**：
- `GeoEco` → `Science`（地理生态是"科学事实"）
- `Developmental` → `Individual`（成长轨迹是"个人历史"）
- `Role` → `Consensus`（角色是"社会共识"）
- `Environmental` → `Embodied`（环境状态是"身体感知"）

**优势**：
- 不修改 Trit-Core，保持向后兼容
- 概念简单，用户容易理解

**劣势**：
- 语义不准确：地理生态不是"科学"，它是**认知框架的约束条件**
- 冲突检测失效：当 `Science`（数据）和 `GeoEco`（环境约束）冲突时，系统无法区分"数据错误"和"环境效应"
- 仲裁规则混乱：`Physical` Domain 的 `Science` 优先规则会覆盖 `GeoEco` 的约束

**结论**：不可接受。映射破坏了参考系的语义独立性。

### 选项B：引入一个通用 `Environment` Frame

**方案**：
- 一个 `Environment` Frame 涵盖所有环境相关：地理、气候、生态、社交密度、发展阶段

**优势**：
- 简单，只有一个新 Frame
- 向后兼容容易

**劣势**：
- 粒度太粗：地理生态（慢变）和环境状态（快变）是不同时间尺度的信号
- 冲突检测不精确：当 `Environment` 和 `Embodied` 冲突时，无法区分是"地理适应"还是"当前环境应激"
- 仲裁规则单一：无法对不同环境子类型设置不同优先级

**结论**：不可接受。粒度不足以支持精细的冲突检测。

### 选项C：引入四个扩展 Frame（GeoEco / Developmental / Role / Environmental）✅

**方案**：
- `GeoEco`：地理生态参考系（慢变，月/年）
- `Developmental`：成长轨迹参考系（极慢变，年）
- `Role`：角色参考系（快变，周/月）
- `Environmental`：环境状态参考系（快变，实时）

**优势**：
- 语义精确：每个 Frame 有明确的时间尺度和数据来源
- 冲突检测精细：可以区分"地理适应"vs"当前应激"vs"角色冲突"vs"成长烙印"
- 仲裁规则灵活：不同 Domain 可以对不同 Frame 设置不同优先级
- 可扩展：未来可以增加更多 Frame（如 `Microbiome`、`Epigenetic`）

**劣势**：
- 复杂度增加：四个新 Frame 需要四个权重分配、四个冲突检测、四个仲裁规则
- 向后兼容：需要扩展 Trit-Core 的 Frame enum，可能影响现有测试
- 用户理解：用户需要学习四个新 Frame 的含义

**缓解劣势**：
- 复杂度：模块化设计，每个 Frame 独立实现，互不影响
- 向后兼容：扩展 enum 而非修改，原有 Frame 和测试不受影响
- 用户理解：UI 层抽象，高级用户可见，普通用户默认配置

**结论**：可接受。劣势可通过工程设计和 UI 抽象缓解，优势不可妥协。

## 决策矩阵

| 维度 | 不引入新 Frame | 一个通用 Frame | 四个扩展 Frame |
|------|---------------|---------------|---------------|
| 语义精确性 | ❌ | ⚠️ | ✅ |
| 冲突检测粒度 | ❌ | ⚠️ | ✅ |
| 仲裁规则灵活性 | ❌ | ⚠️ | ✅ |
| 向后兼容 | ✅ | ✅ | ⚠️ |
| 实现复杂度 | ✅ | ✅ | ⚠️ |
| 用户理解成本 | ✅ | ✅ | ⚠️ |
| 可扩展性 | ❌ | ⚠️ | ✅ |
| 与 Aurora 价值一致 | ❌ | ⚠️ | ✅ |

## 影响

- **Trit-Core 扩展**：`src/core/frame.rs` 增加 `GeoEco`、`Developmental`、`Role`、`Environmental`
- **Trit-Core 扩展**：`src/meta/domain.rs` 增加 `Organizational`、`Relational`、`Cognitive`、`Environmental`
- **权重系统**：需要 `FrameWeights` 结构，每个 Environment 有默认权重
- **冲突检测**：跨四个新 Frame 的冲突检测逻辑
- **仲裁规则**：四个新 Domain 的仲裁规则
- **UI**：需要显示新 Frame 的注意力图谱

## 验证

- 现有 Trit-Core 测试全部通过（向后兼容）
- 新 Frame 的冲突检测测试通过
- 新 Domain 的仲裁规则测试通过

## 相关 ADR

- ADR-003：三值协议 vs 二值概率（支持精细冲突检测）
- 02_math/FIELD_EQUATIONS.md：环境相位冲击的数学基础
- 03_whitepaper/PROTOCOL_SPEC.md：扩展 Frame/Domain 的规格

## 状态

**已接受**。作为 Aurora 对 Trit-Core 的核心扩展，不可回退，除非重新评估整个参考系设计。

---

*本 ADR 记录 Aurora 扩展 Trit-Core Frame 系统的决策过程。不是指教，是提醒。*
