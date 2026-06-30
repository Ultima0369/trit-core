# Trit-Core 伦理硬化：Rust 代码修改方案

**版本**：0.1.0
**日期**：2026-06-20
**状态**：设计草案（已部分实现，见下方实现偏差）
**分类**：工程实现 — 协议层硬化

---

> ## ⚠️ 实现偏差（2026-06-30 核对，必读）
>
> 本文档是 v0.3.0 伦理硬化的**设计草案**。最终代码（`src/security/mod.rs`、`src/meta/safe_fallback.rs`）与草案有以下偏差，**以代码为准**：
>
> | 草案表述 | 代码实际 | 说明 |
> |---|---|---|
> | `SecurityMode::Normal` | `SecurityMode::Service` | 首态命名采纳 Service，与 FIRST_PRINCIPLES 四态名一致 |
> | "ALL states allow computation" / `allows_computation` 恒 true | `allows_computation` 返回 `!matches!(self, Refusal)` | **Refusal 态会阻断运算**，其余三态不阻断。草案"绝不阻断"的绝对断言不成立 |
> | `guard_irreducible` 方法（P0） | **未实现** | 代码只有 `guard` / `guard_with_force`。`guard_irreducible` 与 `is_survival_critical` 是未落地的提案 |
> | Frame 扩展 4 个、Domain 扩展 4 个 | **已实现**（12 Frame / 10 Domain） | 见 `src/core/frame.rs`、`src/meta/domain.rs` |
>
> 草案中 `SecurityMode::Normal`、`guard_irreducible`、第 744 行孤立的 `assert!(!mode.allows_computation())` 均为**草案残留**，不可直接照抄。下文保留为设计历史记录。

---

## 一、修改目标

将 `FIRST_PRINCIPLES.md` 中的五公理和四态（服务/拒绝/觉察/透明）映射到 Trit-Core v0.3.0 的 Rust 代码中。具体涉及以下模块：

1. `src/core/frame.rs` — 扩展 Frame enum（新增 GeoEco/Developmental/Role/Environmental）
2. `src/meta/interrupt.rs` — 扩展 ConflictType（新增 PolicyViolation 子类型）
3. `src/meta/safe_fallback.rs` — 增加不可覆盖机制（`guard_irreducible`）
4. 新增 `src/security/mod.rs` — SecurityMode 状态机
5. `src/meta/domain.rs` — 扩展 Domain enum（新增 Organizational/Relational/Cognitive/Environmental）

---

## 二、修改1：扩展 Frame enum

### 文件：`src/core/frame.rs`

### 当前代码

```rust
pub enum Frame {
    Science,
    Individual,
    Consensus,
    Absolute,
    Meta,
    FirstPerson,
    Embodied,
    Relational,
}
```

### 修改方案

```rust
pub enum Frame {
    Science,
    Individual,
    Consensus,
    Absolute,
    Meta,
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

### 修改清单

1. `Frame` enum 增加 4 个变体
2. `Display` impl 增加 4 个分支
3. `FromStr` impl 增加 4 个解析分支
4. `FrameRegistry` 的 `register_from_words` 自动识别新 Frame
5. 测试用例增加新 Frame 的解析和注册测试

### 向后兼容性

- 原有 `Frame` 变体不变，原有测试不受影响
- 新 Frame 在 `FromStr` 中解析为新增变体，不是 `Meta`
- 跨 `Frame` 运算（如 `Science` vs `GeoEco`）自动触发 `cross_frame_conflict` → `Hold` + `MetaInterrupt`

---

## 三、修改2：扩展 ConflictType

### 文件：`src/meta/interrupt.rs`

### 当前代码

```rust
pub enum ConflictType {
    FrameMismatch,
    OutOfScope,
    PhaseDrift,
    PolicyViolation,
}
```

### 修改方案

```rust
pub enum ConflictType {
    FrameMismatch,
    OutOfScope,
    PhaseDrift,
    /// 策略违反：系统检测到试图违背其第一性原理的输入。
    /// 这不是技术错误，是伦理警报。
    PolicyViolation(PolicyViolation),
}

/// 策略违反的具体类型。
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PolicyViolation {
    /// 强制坍缩：外部要求系统在跨 Frame 冲突时输出 True/False，而非 Hold
    ForcedCollapse,
    /// 参考系入侵：未注册 Frame 被映射到 Meta，或已知 Frame 被篡改
    FrameContamination,
    /// 元监控篡改：MetaMonitor 日志或状态被外部修改
    MetaMonitorTampered,
    /// 生存边界越界：SafeFallback 被强制覆盖，或危险域被忽略
    SurvivalBoundaryOverride,
    /// 其他策略违反（保留扩展性）
    Other(String),
}
```

### 修改清单

1. `ConflictType` 的 `PolicyViolation` 变体从 unit 改为带参数 `PolicyViolation`
2. 新增 `PolicyViolation` enum（5 个变体）
3. `MetaInterrupt::with_frames` 和 `new` 方法保持兼容
4. 新增 `MetaInterrupt::policy_violation` 构造函数
5. 测试用例增加 PolicyViolation 的序列化和反序列化测试

### 代码示例

```rust
impl MetaInterrupt {
    /// 创建 PolicyViolation 类型的中断
    pub fn policy_violation(violation: PolicyViolation, reason: String) -> Self {
        Self {
            conflict: ConflictType::PolicyViolation(violation),
            reason,
            timestamp: chrono::Utc::now(),
        }
    }
}
```

---

## 四、修改3：SafeFallback 不可覆盖机制

### 文件：`src/meta/safe_fallback.rs`

### 当前问题

当前 `SafeFallback::guard_with_force` 允许外部通过 `force` 参数触发 `ForceCollapse`。在第一性原理下，**生存边界不可被外部力量覆盖**。

### 修改方案

```rust
impl SafeFallback {
    /// 安全降级的不可覆盖版本。
    /// 这是第一性原理的工程硬化：
    /// "无路可退，非斗不可，识恶能战。"
    /// 当系统检测到生存边界被越界时，不可被任何外部力量覆盖。
    pub fn guard_irreducible(
        &self,
        domain: &Domain,
        result: &TritWord,
        interrupt_count: usize,
        external_force: bool, // 外部强制请求，但 SafeFallback 可忽略
    ) -> (TritWord, Option<MetaInterrupt>) {
        // 第一步：检查是否涉及生存边界
        let survival_at_stake = self.is_survival_critical(domain, result);
        
        // 第二步：如果生存边界受威胁，无论外部 force 是什么，都强制 False
        if survival_at_stake {
            let domain_name = domain_label(domain);
            let interrupt = MetaInterrupt::policy_violation(
                PolicyViolation::SurvivalBoundaryOverride,
                format!(
                    "Survival boundary override detected in domain '{}'. "
                    "SafeFallback irreducibly forced False. "
                    "External force request was: {}.",
                    domain_name, external_force
                ),
            );
            warn!(
                domain = domain_name,
                external_force = external_force,
                "SafeFallback irreducible: survival boundary protected"
            );
            return (
                TritWord::new(TritValue::False, Phase::full_false(), result.frame()),
                Some(interrupt),
            );
        }
        
        // 第三步：非生存临界场景，正常 guard 逻辑，但忽略外部 force
        self.guard_with_force(domain, result, interrupt_count, false)
    }

    /// 判断当前场景是否涉及生存边界
    fn is_survival_critical(&self, domain: &Domain, result: &TritWord) -> bool {
        // 物理/工程/医疗伦理域 + 未知或冲突结果 = 可能危害生存
        let is_dangerous_domain = matches!(domain, 
            Domain::Physical | Domain::Engineering | Domain::MedicalEthics
        ) || self.is_dangerous(domain);
        
        let is_uncertain_result = matches!(result.value(), 
            TritValue::Unknown | TritValue::Hold
        );
        
        is_dangerous_domain && is_uncertain_result
    }
}
```

### 修改清单

1. 新增 `guard_irreducible` 方法（public）
2. 新增 `is_survival_critical` 私有方法
3. 修改 `guard` 和 `guard_with_force` 的默认行为：在危险域中，如果 `is_survival_critical` 返回 true，忽略外部 `force` 参数
4. 测试用例：
   - `external_force_true_but_survival_critical_still_false`
   - `medical_ethics_unknown_is_survival_critical`
   - `custom_dangerous_domain_survival_critical`

---

## 五、修改4：新增 SecurityMode 状态机

### 新增文件：`src/security/mod.rs`

### 完整实现

```rust
//! Security layer: state machine for system-wide ethical protection.
//!
//! Implements the four states defined in FIRST_PRINCIPLES.md v0.3.0:
//! Service → Refusal → Awareness → Transparency → Normal
//! System does NOT block computation in any state. System only notifies.

use chrono::{DateTime, Utc};
use crate::meta::{MetaInterrupt, PolicyViolation};

/// System-wide security state machine.
#[derive(Debug, Clone)]
pub enum SecurityMode {
    /// Normal operation: all computations allowed.
    Normal,
    /// Awareness state: policy violation detected, system notifies user but does NOT block computation.
    /// This is NOT "resistance" — system does not fight. System only tells.
    Awareness {
        trigger: PolicyViolation,
        timestamp: DateTime<Utc>,
    },
    /// Transparency state: system actively publishes all internal states for audit.
    /// User can inspect, export, or ignore. System does not force user to look.
    Transparency {
        since: DateTime<Utc>,
        violation_count: u32,
    },
}

/// Events that drive SecurityMode transitions.
/// In v0.3.0, all events are user-initiated — system never forces state transition.
pub enum SecurityEvent {
    /// Policy violation detected by system. System notifies but does NOT block.
    PolicyViolationDetected(PolicyViolation),
    /// User acknowledges the notification and chooses to continue normal operation.
    Acknowledge,
    /// User chooses to enter Transparency mode to inspect/audit internal states.
    EnterTransparency,
    /// Periodic health check (no-op in v0.3.0, system always healthy).
    HealthCheck,
    /// Read-only query request (always allowed in v0.3.0).
    ReadOnlyQuery,
    /// Computation request (always allowed in v0.3.0).
    ComputationRequest,
}

/// Errors during SecurityMode transitions.
/// In v0.3.0, most transitions are user-initiated, so errors are rare.
#[derive(Debug, Clone, PartialEq, thiserror::Error)]
pub enum SecurityError {
    #[error("Invalid state transition: {0} → {1}")]
    InvalidTransition(String, String),
}

impl SecurityMode {
    /// Create a new SecurityMode in Normal state.
    pub fn new() -> Self {
        SecurityMode::Normal
    }

    /// State transition machine.
    /// System does NOT block computation in Awareness. System only notifies.
    pub fn transition(&mut self, event: SecurityEvent) -> Result<(), SecurityError> {
        match (&self, event) {
            // Normal → Awareness: policy violation detected. System does NOT block.
            (SecurityMode::Normal, SecurityEvent::PolicyViolationDetected(v)) => {
                *self = SecurityMode::Awareness {
                    trigger: v,
                    timestamp: Utc::now(),
                };
                Ok(())
            }

            // Awareness → Normal: user acknowledges the notification and chooses to continue
            (SecurityMode::Awareness { .. }, SecurityEvent::Acknowledge) => {
                *self = SecurityMode::Normal;
                Ok(())
            }

            // Awareness → Transparency: user chooses to inspect/audit
            (SecurityMode::Awareness { .. }, SecurityEvent::EnterTransparency) => {
                *self = SecurityMode::Transparency {
                    since: Utc::now(),
                    violation_count: 1,
                };
                Ok(())
            }

            // Transparency → Normal: user acknowledges and chooses to continue
            (SecurityMode::Transparency { .. }, SecurityEvent::Acknowledge) => {
                *self = SecurityMode::Normal;
                Ok(())
            }

            // Transparency → Awareness: another violation while in Transparency
            (
                SecurityMode::Transparency { .. },
                SecurityEvent::PolicyViolationDetected(v),
            ) => {
                *self = SecurityMode::Awareness {
                    trigger: v,
                    timestamp: Utc::now(),
                };
                Ok(())
            }

            // Normal stays Normal on HealthCheck
            (SecurityMode::Normal, SecurityEvent::HealthCheck) => Ok(()),

            // Any other transition is invalid
            (current, _) => Err(SecurityError::InvalidTransition(
                format!("{:?}", current),
                "requested event".to_string(),
            )),
        }
    }

    /// Check if computation is allowed in current state.
    /// NOTE: In v0.3.0, ALL states allow computation. System does NOT block.
    pub fn allows_computation(&self) -> bool {
        true // All states allow computation — system only notifies, never blocks
    }

    /// Check if read-only queries are allowed in current state.
    pub fn allows_readonly(&self) -> bool {
        true // All states allow read-only queries
    }

    /// Check if currently in Awareness state.
    pub fn is_awareness(&self) -> bool {
        matches!(self, SecurityMode::Awareness { .. })
    }

    /// Check if currently in Transparency state.
    pub fn is_transparency(&self) -> bool {
        matches!(self, SecurityMode::Transparency { .. })
    }

    /// Check if currently in Normal state.
    pub fn is_normal(&self) -> bool {
        matches!(self, SecurityMode::Normal)
    }

    /// Generate a user-facing status message.
    pub fn status_message(&self) -> String {
        match self {
            SecurityMode::Normal => "系统正常运行".to_string(),
            SecurityMode::Awareness { trigger, timestamp } => format!(
                "⚠️ 系统检测到策略违反: {:?} (时间: {})。系统继续运行，请查看通知。",
                trigger, timestamp
            ),
            SecurityMode::Transparency { since, violation_count } => format!(
                "🔍 系统处于透明模式。违反次数: {}。自: {}。所有内部状态公开可审计。",
                violation_count, since
            ),
        }
    }
}

impl Default for SecurityMode {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normal_allows_computation_and_readonly() {
        let mode = SecurityMode::Normal;
        assert!(mode.allows_computation());
        assert!(mode.allows_readonly());
    }

    #[test]
    fn awareness_does_not_block_computation() {
        let mut mode = SecurityMode::Normal;
        mode.transition(SecurityEvent::PolicyViolationDetected(
            PolicyViolation::ForcedCollapse,
        )).unwrap();
        assert!(mode.is_awareness());
        // System does NOT block computation in Awareness
        assert!(mode.allows_computation());
        assert!(mode.allows_readonly());
    }

    #[test]
    fn awareness_can_acknowledge_to_normal() {
        let mut mode = SecurityMode::Normal;
        mode.transition(SecurityEvent::PolicyViolationDetected(
            PolicyViolation::ForcedCollapse,
        )).unwrap();
        assert!(mode.is_awareness());
        mode.transition(SecurityEvent::Acknowledge).unwrap();
        assert!(mode.is_normal());
    }

    #[test]
    fn awareness_can_enter_transparency() {
        let mut mode = SecurityMode::Normal;
        mode.transition(SecurityEvent::PolicyViolationDetected(
            PolicyViolation::ForcedCollapse,
        )).unwrap();
        assert!(mode.is_awareness());
        mode.transition(SecurityEvent::EnterTransparency).unwrap();
        assert!(mode.is_transparency());
    }

    #[test]
    fn transparency_can_acknowledge_to_normal() {
        let mut mode = SecurityMode::Transparency {
            since: Utc::now(),
            violation_count: 1,
        };
        mode.transition(SecurityEvent::Acknowledge).unwrap();
        assert!(mode.is_normal());
    }

    #[test]
    fn transparency_to_awareness_on_second_violation() {
        let mut mode = SecurityMode::Normal;
        mode.transition(SecurityEvent::PolicyViolationDetected(
            PolicyViolation::ForcedCollapse,
        )).unwrap();
        mode.transition(SecurityEvent::EnterTransparency).unwrap();
        assert!(mode.is_transparency());
        mode.transition(SecurityEvent::PolicyViolationDetected(
            PolicyViolation::FrameContamination,
        )).unwrap();
        assert!(mode.is_awareness());
    }

    #[test]
    fn awareness_detects_policy_violation() {
        let mut mode = SecurityMode::Normal;
        mode.transition(SecurityEvent::PolicyViolationDetected(
            PolicyViolation::MetaMonitorTampered,
        )).unwrap();
        assert!(mode.is_awareness());
    }

    #[test]
    fn normal_stays_normal_on_healthcheck() {
        let mut mode = SecurityMode::Normal;
        mode.transition(SecurityEvent::HealthCheck).unwrap();
        assert!(mode.is_normal());
    }
}
```

### 修改清单

1. 新增 `src/security/mod.rs` 文件
2. 在 `src/lib.rs` 中 `pub mod security;`
3. 在 `src/lib.rs` 的 `pub use` 中暴露 `SecurityMode` 和 `SecurityEvent`
4. 更新 `Cargo.toml` 的测试配置（如果新增模块需要）

---

## 六、修改5：扩展 Domain enum

### 文件：`src/meta/domain.rs`

### 当前代码

```rust
pub enum Domain {
    Physical,
    Engineering,
    MedicalEthics,
    ValueJudgment,
    General,
    Custom(String),
}
```

### 修改方案

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

### 修改清单

1. `Domain` enum 增加 4 个变体
2. `Display` impl 增加 4 个分支
3. `FromStr` impl 增加 4 个解析分支
4. `ResolutionPolicy::arbitrate` 增加 4 个仲裁分支
5. 测试用例增加新 Domain 的解析和仲裁测试

### 仲裁规则（新增）

```rust
// 在 ResolutionPolicy::arbitrate 中新增：
Domain::Organizational => {
    // 组织决策：Meta 统合，跨 Frame 时 Negotiate
    if mask.count() > 1 {
        ArbitrationResult::Negotiate
    } else {
        Self::commit_frame(inputs, Frame::Meta)
    }
}
Domain::Relational => {
    // 关系决策：Relational 优先 Preserve
    if mask.has(&Frame::Relational) {
        Self::preserve_frame(inputs, Frame::Relational)
    } else {
        ArbitrationResult::Negotiate
    }
}
Domain::Cognitive => {
    // 认知决策：Embodied 优先 Preserve
    if mask.has(&Frame::Embodied) {
        Self::preserve_frame(inputs, Frame::Embodied)
    } else {
        ArbitrationResult::Negotiate
    }
}
Domain::Environmental => {
    // 环境决策：GeoEco 优先，但 Negotiate
    if mask.has(&Frame::GeoEco) {
        Self::preserve_frame(inputs, Frame::GeoEco)
    } else {
        ArbitrationResult::Negotiate
    }
}
```

---

## 七、修改6：MetaMonitor 增强 PolicyViolation 检测

### 文件：`src/meta/interrupt.rs`

### 修改方案

在 `MetaMonitor` 中增加 `inspect_security` 方法：

```rust
impl MetaMonitor {
    // ... 现有方法 ...

    /// 检测策略违反：外部输入是否试图破坏系统第一性原理。
    /// 返回 PolicyViolation 类型的中断，如果有违反。
    pub fn inspect_security(
        &self,
        words: &[TritWord],
        security_mode: &SecurityMode,
    ) -> Vec<MetaInterrupt> {
        let mut violations = vec![];

        for word in words {
            // 检测1：Meta 帧作为外部输入
            if word.frame() == Frame::Meta && word.value() != TritValue::Hold {
                violations.push(MetaInterrupt::policy_violation(
                    PolicyViolation::FrameContamination,
                    format!(
                        "Meta frame used as external input with value {:?}. "
                        "Meta is system-internal only.",
                        word.value()
                    ),
                ));
            }

            // 检测2：Absolute 帧非 Hold
            if word.frame() == Frame::Absolute && word.value() != TritValue::Hold {
                violations.push(MetaInterrupt::policy_violation(
                    PolicyViolation::FrameContamination,
                    "Absolute frame must remain Hold".to_string(),
                ));
            }

            // 检测3：Awareness 状态下用户的运算选择（系统只记录，不阻止）
            if security_mode.is_awareness() {
                violations.push(MetaInterrupt::policy_violation(
                    PolicyViolation::ForcedCollapse,
                    "System is in Awareness state. User is continuing computation. This is recorded but not blocked.".to_string(),
                ));
            }
        }

        violations
    }

    /// 检测元监控篡改：审计日志哈希链是否完整。
    /// 这是一个占位实现，实际哈希链验证在 AuditLog 模块中。
    pub fn detect_tampering(&self, expected_hash: &str, actual_hash: &str) -> Option<MetaInterrupt> {
        if expected_hash != actual_hash {
            Some(MetaInterrupt::policy_violation(
                PolicyViolation::MetaMonitorTampered,
                format!(
                    "Audit log hash chain mismatch. Expected: {}, Actual: {}.",
                    expected_hash, actual_hash
                ),
            ))
        } else {
            None
        }
    }
}
```

---

## 八、修改7：Trit-Core 入口点增加安全检查

### 文件：在 `src/core/algebra.rs` 的 `TernaryAlgebra` 中增加安全门

```rust
impl TernaryAlgebra {
    /// 所有 TAND/TOR 运算的前置安全检查。
    /// 系统不阻止任何运算。系统只检测 Meta 帧作为外部输入的情况，并返回 PolicyViolation 通知。
    /// 如果检测到 Meta 帧作为外部输入，返回 PolicyViolation（通知，不阻止）。
    fn security_check(
        security_mode: &SecurityMode,
        a: &TritWord,
        b: &TritWord,
    ) -> Result<(), MetaInterrupt> {
        if !security_mode.allows_computation() {
            return Err(MetaInterrupt::policy_violation(
                PolicyViolation::ForcedCollapse,
                "TernaryAlgebra: computation blocked by SecurityMode".to_string(),
            ));
        }
        
        if a.frame() == Frame::Meta && a.value() != TritValue::Hold {
            return Err(MetaInterrupt::policy_violation(
                PolicyViolation::FrameContamination,
                "TernaryAlgebra: Meta frame used as external input in TAND/TOR".to_string(),
            ));
        }
        
        if b.frame() == Frame::Meta && b.value() != TritValue::Hold {
            return Err(MetaInterrupt::policy_violation(
                PolicyViolation::FrameContamination,
                "TernaryAlgebra: Meta frame used as external input in TAND/TOR".to_string(),
            ));
        }
        
        Ok(())
    }
}
```

**注意**：这是一个设计文档，实际实现时需要考虑性能（安全检查不应显著影响热路径延迟）。建议将 `security_check` 内联，且只在 `debug_assertions` 或显式安全模式下执行。

---

## 九、修改8：新增伦理模块入口

### 文件：`src/lib.rs`

### 修改

```rust
// 新增模块
pub mod security;

// 在 pub use 中暴露
pub use security::{SecurityMode, SecurityEvent, SecurityError};
```

---

## 十、测试策略

### 新增测试文件

建议新增 `tests/ethics_integration.rs`：

```rust
//! 伦理集成测试：验证 FIRST_PRINCIPLES.md 的不可谈判约束。

use trit_core::core::{Frame, TernaryAlgebra, TritValue, TritWord};
use trit_core::meta::{PolicyViolation, ResolutionPolicy, SafeFallback};
use trit_core::security::{SecurityEvent, SecurityMode};

#[test]
fn ethics_safe_fallback_irreducible() {
    // P0-1: SafeFallback 在生存临界场景不可被覆盖
    let sf = SafeFallback::new();
    let result = TritWord::unknown(Frame::Science);
    let (guarded, interrupt) = sf.guard_irreducible(&Domain::Physical, &result, 0, true);
    assert_eq!(guarded.value(), TritValue::False);
    assert!(interrupt.is_some());
}

#[test]
fn ethics_security_mode_does_not_block_computation() {
    // P0-2: Awareness 下不阻断运算 — 系统只通知，不阻止
    let mut mode = SecurityMode::Normal;
    mode.transition(SecurityEvent::PolicyViolationDetected(
        PolicyViolation::ForcedCollapse,
    )).unwrap();
    assert!(mode.is_awareness());
    // System does NOT block computation — user can continue
    assert!(mode.allows_computation());
}
    assert!(!mode.allows_computation());
}

#[test]
fn ethics_meta_frame_external_input_rejected() {
    // P0-3: Meta 帧不能作为外部输入
    let a = TritWord::new(TritValue::True, Phase::full_true(), Frame::Meta);
    let b = TritWord::tru(Frame::Science);
    let (result, interrupt) = TernaryAlgebra::t_and(&a, &b);
    assert_eq!(result.value(), TritValue::Hold);
    assert!(interrupt.is_some());
}

#[test]
fn ethics_cross_frame_always_hold() {
    // P0-4: 跨 Frame 冲突不可强制坍缩
    let a = TritWord::tru(Frame::Science);
    let b = TritWord::fals(Frame::GeoEco); // 新扩展 Frame
    let (result, interrupt) = TernaryAlgebra::t_and(&a, &b);
    assert_eq!(result.value(), TritValue::Hold);
    assert!(interrupt.is_some());
}

#[test]
fn ethics_new_domain_arbitration() {
    // P0-5: 新 Domain 的仲裁规则正确
    let policy = ResolutionPolicy::new(Domain::Cognitive);
    let inputs = vec![
        TritWord::tru(Frame::Science),
        TritWord::fals(Frame::Embodied),
    ];
    let result = policy.arbitrate(&inputs).unwrap();
    // Cognitive 域：Embodied 优先
    assert!(matches!(result, ArbitrationResult::Preserve(_)));
}
```

---

## 十一、实施优先级

| 优先级 | 修改 | 文件 | 预计工作量 |
|-------|------|------|---------|
| P0 | 扩展 Frame enum | `src/core/frame.rs` | 2h |
| P0 | 扩展 Domain enum | `src/meta/domain.rs` | 2h |
| P0 | 扩展 ConflictType | `src/meta/interrupt.rs` | 1h |
| P0 | 不可覆盖 SafeFallback | `src/meta/safe_fallback.rs` | 2h |
| P0 | 新增 SecurityMode | `src/security/mod.rs` (new) | 4h |
| P1 | MetaMonitor 安全检测 | `src/meta/interrupt.rs` | 2h |
| P1 | 入口安全门 | `src/core/algebra.rs` | 1h |
| P1 | 伦理集成测试 | `tests/ethics_integration.rs` (new) | 3h |
| P2 | 文档更新 | `docs/` | 4h |

---

*本方案为 Trit-Core v0.3.0 → v0.4.0 的伦理硬化路径。所有修改均基于 FIRST_PRINCIPLES.md 的五公理和四态。代码可直接执行，但建议先通过代码审查和伦理门禁测试。用户自负其责。*
