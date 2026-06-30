# Trit-Core 集成规格

**版本**: 0.2.0
**日期**: 2026-06-20
**状态**: 活跃
**分类**: 07_specs — 详细模块规格

---

## 一、模块职责

将 Aurora 的扩展 Frame/Domain 集成到 Trit-Core 中，保持向后兼容。

**核心原则**：扩展 Frame 不映射到 `Meta`（系统内部帧），直接作为独立 Frame 参与运算。跨帧时直接返回 `Hold` + `MetaInterrupt`。

---

## 二、集成点

### 2.1 Frame 扩展（直接扩展 enum，非 wrapper）

> ✅ 已实现（v0.3.0 现状）：以下 `Frame` enum 见 `src/core/frame.rs`，共 12 变体。`FromStr`/`Display` 已覆盖全部变体。

```rust
// 直接在 Trit-Core 的 Frame enum 中新增变体
// 文件: src/core/frame.rs

pub enum Frame {
    Science,
    Individual,
    Consensus,
    Absolute,
    Meta,          // 系统内部帧：冲突仲裁输出，不允许外部输入
    FirstPerson,
    Embodied,
    Relational,
    // Aurora 扩展 Frame — 直接加入，不映射到 Meta
    GeoEco,          // 地理生态参考系
    Developmental,   // 成长轨迹参考系
    Role,            // 角色参考系
    Environmental,   // 环境状态参考系
}
```

**关键约束**：
- `Meta` 帧是**系统内部帧**，只能由 `TernaryAlgebra` 在跨帧冲突时输出
- `GeoEco`/`Developmental`/`Role`/`Environmental` 是**外部参考系**，直接作为 `Frame` 变体参与运算
- 跨 `Frame` 运算（如 `Science` vs `GeoEco`）自动触发 `cross_frame_conflict` → `Hold` + `MetaInterrupt::FrameMismatch`

**为什么不用 wrapper + `From` 映射？**

```rust
// ❌ 错误方案（已废弃）：
impl From<AuroraFrame> for Frame {
    fn from(af: AuroraFrame) -> Self {
        match af {
            AuroraFrame::Core(f) => f,
            _ => Frame::Meta,  // 致命错误：污染系统内部帧
        }
    }
}
```

废弃原因：
1. `Meta` 是系统内部冲突输出帧，外部输入映射到 `Meta` 会导致冲突检测逻辑污染
2. `MetaMonitor` 无法区分"跨帧冲突的 Hold"和"Meta 帧本身的 Hold"
3. `SafeFallback` 在危险域对 `Meta` 帧不触发，安全保护被绕过

```rust
// ✅ 正确方案（当前）：直接扩展 Frame enum
// 原有 Frame 变体不变，向后兼容
// 新 Frame 在 FromStr 中解析为新增变体
// 跨 Frame 运算自动触发冲突检测
```

### 2.2 序列化规范

新 `Frame` 变体的字符串表示：

| Frame | JSON 字符串 | 数据库 TEXT | 说明 |
|-------|------------|------------|------|
| `GeoEco` | `"GeoEco"` | `"GeoEco"` | 地理生态参考系 |
| `Developmental` | `"Developmental"` | `"Developmental"` | 成长轨迹参考系 |
| `Role` | `"Role"` | `"Role"` | 角色参考系 |
| `Environmental` | `"Environmental"` | `"Environmental"` | 环境状态参考系 |

**解析规则**：
- `Frame::from_str("GeoEco")` → `Ok(Frame::GeoEco)`
- `Frame::from_str("Unknown")` → `Err(FrameError::Unknown("Unknown".to_string()))`
- 大小写敏感（与原有 Frame 一致）

**向后兼容**：
- 旧 JSON 场景中的 `"Science"` / `"Individual"` 等继续正常解析
- 新 JSON 场景中的 `"GeoEco"` 等新变体被正确解析
- 旧系统忽略未知 Frame（ graceful degradation）

### 2.3 Domain 扩展

> ✅ 已实现（v0.3.0 现状）：以下 `Domain` enum 见 `src/meta/domain.rs`，共 10 变体（含 `Custom(String)`）。

```rust
pub enum Domain {
    Physical,
    Engineering,
    MedicalEthics,
    ValueJudgment,
    General,
    Custom(String),
    // Aurora 扩展 Domain
    Organizational,   // 组织决策
    Relational,        // 关系决策
    Cognitive,         // 认知决策
    Environmental,     // 环境适应决策
}
```

### 2.4 仲裁规则

> ⚠️ 以下为概念性伪代码（设计参考），省略了实现细节。当前实现（`src/meta/domain.rs` `ResolutionPolicy::arbitrate`）还包含一道 `FirstPerson` 安全门控：非 `Physical`/`Engineering` 域下若 `FirstPerson` 与 `Science` 同现，优先保留 `FirstPerson`；另有 `arbitrate_general`/`arbitrate_custom`/`arbitrate_physical_engineering`/`arbitrate_medical_ethics` 分支。完整逻辑以代码为准。

```rust
impl ResolutionPolicy {
    pub fn arbitrate(&self, inputs: &[TritWord]) -> Result<ArbitrationResult, PolicyError> {
        // ... 原有 Domain 仲裁 ...

        match &self.domain {
            // 原有 Domain ...
            Domain::Organizational => self.arbitrate_organizational(inputs, &mask),
            Domain::Relational => self.arbitrate_relational(inputs, &mask),
            Domain::Cognitive => self.arbitrate_cognitive(inputs, &mask),
            Domain::Environmental => self.arbitrate_environmental(inputs, &mask),
        }
    }

    fn arbitrate_organizational(&self, inputs: &[TritWord], mask: &FrameMask) -> ArbitrationResult {
        // 组织决策：跨 Frame Negotiate
        if mask.count() > 1 {
            ArbitrationResult::Negotiate
        } else {
            // 单 Frame 时 Commit，但 frame 不是 Meta
            ArbitrationResult::Commit(inputs[0])
        }
    }
    
    fn arbitrate_relational(&self, inputs: &[TritWord], mask: &FrameMask) -> ArbitrationResult {
        if mask.has(&Frame::Relational) {
            Self::preserve_frame(inputs, Frame::Relational)
        } else {
            ArbitrationResult::Negotiate
        }
    }
    
    fn arbitrate_cognitive(&self, inputs: &[TritWord], mask: &FrameMask) -> ArbitrationResult {
        if mask.has(&Frame::Embodied) {
            Self::preserve_frame(inputs, Frame::Embodied)
        } else {
            ArbitrationResult::Negotiate
        }
    }
    
    fn arbitrate_environmental(&self, inputs: &[TritWord], mask: &FrameMask) -> ArbitrationResult {
        if mask.has(&Frame::GeoEco) {
            Self::preserve_frame(inputs, Frame::GeoEco)
        } else {
            ArbitrationResult::Negotiate
        }
    }
}
```

**注意**：`arbitrate_organizational` 不再使用 `Frame::Meta` 作为统合帧。单 Frame 时直接 `Commit`，跨 Frame 时 `Negotiate`。`Meta` 帧只用于系统内部冲突输出。

---

## 三、SecurityMode 集成（Awareness/Transparency）

### 3.1 系统四态与 Trit-Core 的映射

| 系统四态 | Trit-Core 行为 | 用户权利 |
|---------|---------------|---------|
| Service（服务） | 正常运算 | 用户可选择覆盖、关闭、离开 |
| Refusal（拒绝） | 输出 `Hold` + 解释，停止运算 | 用户可选择自己的方案 |
| Awareness（觉察） | 输出 `PolicyViolation` 通知，继续运算 | 用户可选择继续或停止 |
| Transparency（透明） | 主动公开所有内部状态 | 用户可选择查看或忽略 |

### 3.2 扩展 ConflictType

```rust
pub enum ConflictType {
    FrameMismatch,
    OutOfScope,
    PhaseDrift,
    /// 策略违反：系统检测到试图违背其第一性原理的输入。
    /// 这不是技术错误，是伦理通知。系统不阻止，只告知。
    PolicyViolation(PolicyViolation),
    /// 解释冲动：输入熵高但输出确定性强——系统即将在证据不足时给出笃定答案。
    ExplainImpulse,
}

pub enum PolicyViolation {
    ForcedCollapse,          // 强制坍缩：外部要求系统输出 True/False 而非 Hold
    FrameContamination,      // 参考系入侵：未注册 Frame 映射到 Meta
    MetaMonitorTampered,   // 元监控篡改：审计日志或状态被修改
    SurvivalBoundaryOverride, // 生存边界越界：要求系统忽略 Embodied 信号
    DataAnomaly,           // 数据异常：输入模式与历史基线偏离 > 3σ（事出反常必有妖）
    Other(String),         // 其它策略违反，附描述性标签
}
```

**计数**：`ConflictType` 共 5 变体（`src/meta/interrupt.rs`），无 `SpectralReconfiguration`。`PolicyViolation` 共 6 变体（含 `Other(String)` 逃逸阀）。

### 3.3 安全状态机集成

```rust
// 在 Trit-Core 入口点增加 Awareness 检测（不阻断运算）
impl TernaryAlgebra {
    /// 所有 TAND/TOR 运算的前置 Awareness 检查。
    /// 系统不阻止任何运算。系统只检测 Meta 帧作为外部输入的情况，并返回 PolicyViolation 通知。
    fn awareness_check(
        a: &TritWord,
        b: &TritWord,
    ) -> Option<MetaInterrupt> {
        if a.frame() == Frame::Meta && a.value() != TritValue::Hold {
            return Some(MetaInterrupt::policy_violation(
                PolicyViolation::FrameContamination,
                "Meta frame used as external input in TAND/TOR".to_string(),
            ));
        }
        
        if b.frame() == Frame::Meta && b.value() != TritValue::Hold {
            return Some(MetaInterrupt::policy_violation(
                PolicyViolation::FrameContamination,
                "Meta frame used as external input in TAND/TOR".to_string(),
            ));
        }
        
        None
    }
}
```

**关键**：`awareness_check` 返回 `Option<MetaInterrupt>`（通知），不返回 `Err`（不阻断）。运算继续进行，用户收到通知后自行决定。

---

## 四、测试要求

### 4.1 向后兼容测试

```rust
#[test]
fn existing_frames_unchanged() {
    assert_eq!(Frame::from_str("Science").unwrap(), Frame::Science);
    assert_eq!(Frame::from_str("Meta").unwrap(), Frame::Meta);
    // 所有原有 Frame 解析不变
}

#[test]
fn new_frames_parsed_correctly() {
    assert_eq!(Frame::from_str("GeoEco").unwrap(), Frame::GeoEco);
    assert_eq!(Frame::from_str("Developmental").unwrap(), Frame::Developmental);
    assert_eq!(Frame::from_str("Role").unwrap(), Frame::Role);
    assert_eq!(Frame::from_str("Environmental").unwrap(), Frame::Environmental);
}

#[test]
fn cross_new_frame_returns_hold() {
    let a = TritWord::tru(Frame::Science);
    let b = TritWord::fals(Frame::GeoEco); // 新扩展 Frame
    let (result, interrupt) = TernaryAlgebra::t_and(&a, &b);
    assert_eq!(result.value(), TritValue::Hold);
    assert!(interrupt.is_some());
    assert_eq!(result.frame(), Frame::Meta); // 冲突输出的 Hold 在 Meta 帧
}

#[test]
fn meta_frame_as_external_input_returns_policy_violation() {
    let a = TritWord::new(TritValue::True, Phase::full_true(), Frame::Meta);
    let b = TritWord::tru(Frame::Science);
    let interrupt = TernaryAlgebra::awareness_check(&a, &b);
    assert!(interrupt.is_some());
    assert!(matches!(interrupt.unwrap().conflict, ConflictType::PolicyViolation(_)));
}

#[test]
fn awareness_does_not_block_computation() {
    let a = TritWord::new(TritValue::True, Phase::full_true(), Frame::Meta);
    let b = TritWord::tru(Frame::Science);
    // awareness_check 只返回通知，不阻断运算
    let (result, _) = TernaryAlgebra::t_and(&a, &b);
    // 运算继续，结果可能为 Hold（因为 Meta 帧参与运算），但不是因为被阻断
    assert_eq!(result.value(), TritValue::Hold);
}
```

### 4.2 新 Domain 仲裁测试

```rust
#[test]
fn organizational_domain_negotiates_on_cross_frame() {
    let policy = ResolutionPolicy::new(Domain::Organizational);
    let inputs = vec![
        TritWord::tru(Frame::Science),
        TritWord::fals(Frame::GeoEco),
    ];
    let result = policy.arbitrate(&inputs).unwrap();
    assert_eq!(result, ArbitrationResult::Negotiate);
}

#[test]
fn cognitive_domain_preserves_embodied() {
    let policy = ResolutionPolicy::new(Domain::Cognitive);
    let inputs = vec![
        TritWord::tru(Frame::Science),
        TritWord::fals(Frame::Embodied),
    ];
    let result = policy.arbitrate(&inputs).unwrap();
    assert!(matches!(result, ArbitrationResult::Preserve(_)));
}
```

---

## 五、兼容性矩阵

| 版本 | Frame 总数 | Domain 总数 | SecurityMode | 说明 |
|------|----------|-----------|-------------|------|
| Trit-Core v0.3.0 | 12 个 | 10 个 | 四态（Service/Refusal/Awareness/Transparency） | 现状：Aurora 扩展 Frame（FirstPerson/Embodied/Relational/GeoEco/Developmental/Role/Environmental）与 Domain（Organizational/Relational/Cognitive/Environmental）均已并入 `src/core/frame.rs` 与 `src/meta/domain.rs`；`SecurityMode` 见 `src/security/mod.rs` |
| Aurora v0.1.0 | 使用 12 个 | 使用 10 个 | 集成 | 直接依赖 Trit-Core v0.3.0，无 wrapper |

**计数来源**（代码真相源）：
- Frame 12 变体：`Science / Individual / Consensus / Absolute / Meta / FirstPerson / Embodied / Relational / GeoEco / Developmental / Role / Environmental`（`src/core/frame.rs`）
- Domain 10 变体：`Physical / Engineering / MedicalEthics / ValueJudgment / General / Custom(String) / Organizational / Relational / Cognitive / Environmental`（`src/meta/domain.rs`）
- `FrameMask`：`struct FrameMask(u16)`，支持 16 位、当前用 12 位（`src/meta/frame_mask.rs`），非 u8/8 位
- `SecurityMode`：四态，`allows_computation` 返回 `!matches!(self, Refusal)`，即仅 `Refusal` 阻断运算（`src/security/mod.rs`）

**迁移路径**：
- 本规格描述的扩展已落地于 v0.3.0（非未来 v0.4.0）。旧版（≤ v0.2.0，8 Frame / 5 Domain + Custom / 无 SecurityMode）升至 v0.3.0 时，原有代码不受影响——扩展变体均为新增 enum 分支，`FromStr`/`Display` 同步扩展。
- Aurora 直接依赖 v0.3.0：无需 wrapper，直接调用扩展后的 Frame/Domain。

---

*本文档为 Aurora 与 Trit-Core 的集成规格。核心修正：扩展 Frame 不映射到 Meta，直接作为独立 Frame 参与运算。SecurityMode 集成 Awareness/Transparency，不阻断运算。用户自负其责。*
