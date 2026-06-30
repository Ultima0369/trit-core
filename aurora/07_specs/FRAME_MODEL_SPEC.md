# 参考系建模模块规格

**版本**: 0.2.0
**日期**: 2026-06-20
**状态**: 活跃
**分类**: 07_specs — 详细模块规格

---

## 一、模块职责

管理用户的参考系模型（地理生态、成长轨迹、当前角色、环境状态），计算 Frame 权重，检测环境冲击和角色边界污染。

**核心原则**：所有参考系模型数据本地存储，不上云。用户可导出、可删除、可修改。

---

## 二、接口定义

```rust
pub struct FrameModel {
    pub geo_eco: GeoEcoProfile,
    pub developmental: DevelopmentalProfile,
    pub environmental: EnvironmentalState,
    pub role: RoleState,
}

impl FrameModel {
    /// 计算当前环境下的 Frame 权重
    pub fn weights(&self) -> FrameWeights;
    
    /// 计算环境冲击等级
    pub fn shock_level(&self, old_env: &Environment, new_env: &Environment) -> ShockResult;
    
    /// 计算角色边界指标
    pub fn role_boundary(&self) -> RoleBoundaryMetrics;
    
    /// 更新环境状态
    pub fn update_environment(&mut self, new_env: EnvironmentalState);
    
    /// 更新角色状态
    pub fn update_role(&mut self, new_role: RoleState);
}

pub struct ShockResult {
    pub delta_phi: f64,
    pub level: ShockLevel,  // Micro / Moderate / Strong / Catastrophic
    pub recovery_eta: Duration,
}

pub struct RoleBoundaryMetrics {
    pub role_weight: Phase,
    pub individual_weight: Phase,  // 与 Frame::Individual 对齐，非 Frame::Self
    pub contamination_ratio: f64,
    pub dissociation_index: f64,
    pub boundary_permeability: f64,
}
```

**关键修正**：`self_weight` 改为 `individual_weight`，与 Trit-Core v0.3.0 的 `Frame::Individual` 对齐。Trit-Core 中没有 `Frame::Self` 变体。

---

## 三、数据模型

### GeoEcoProfile

```rust
pub struct GeoEcoProfile {
    pub birthplace: Location,
    pub migration_path: Vec<Location>,
    pub current_location: Location,
    pub climate_zone: ClimateZone,
    pub soil_type: SoilType,
    pub altitude: f64,
    pub water_access: WaterAccess,
    pub social_density: SocialDensity,
}
```

### DevelopmentalProfile

```rust
pub struct DevelopmentalProfile {
    pub attachment_style: AttachmentStyle,
    pub key_imprints: Vec<ImprintEvent>,
    pub economic_trauma: bool,
    pub authority_trauma: bool,
    pub identity_trauma: bool,
    pub migration_age: Option<u8>,
    pub language_count: u8,
    pub awakening_level: Option<u8>,
}
```

---

## 四、计算逻辑

### 环境冲击计算

```rust
pub fn calculate_shock(&self, old_env: &Environment, new_env: &Environment) -> ShockResult {
    let old_weights = self.weights_for(old_env);
    let new_weights = self.weights_for(new_env);
    let delta_phi = self.frame_weights.iter()
        .map(|(frame, w_old)| {
            let w_new = new_weights.get(frame);
            let phi_old = self.typical_phase(frame, old_env);
            let phi_new = self.typical_phase(frame, new_env);
            w_old * (phi_old - phi_new).abs()
        })
        .sum();
    
    ShockResult {
        delta_phi,
        level: ShockLevel::from_delta(delta_phi),
        recovery_eta: self.estimate_recovery(delta_phi),
    }
}
```

### 角色边界计算

```rust
pub fn calculate_role_boundary(&self) -> RoleBoundaryMetrics {
    let role_weight = self.role.weight();
    // 使用 Frame::Individual 而非 Frame::Self
    let individual_weight = self.developmental.individual_weight();
    let contamination_ratio = role_weight.value() / (individual_weight.value() + 1e-6);
    
    RoleBoundaryMetrics {
        role_weight,
        individual_weight,  // 与 Frame::Individual 对齐
        contamination_ratio,
        dissociation_index: self.calculate_dissociation(),
        boundary_permeability: self.calculate_permeability(),
    }
}
```

### 角色边界预警的系统响应

```rust
impl RoleBoundaryMetrics {
    /// 系统响应：只通知，不阻止
    pub fn system_response(&self) -> (TritValue, Option<MetaInterrupt>) {
        if self.contamination_ratio > 0.95 {
            // 红色预警：系统输出 Hold + 通知
            let hold = TritWord::hold(Frame::Individual);
            let interrupt = MetaInterrupt::policy_violation(
                PolicyViolation::FrameContamination,
                "Role contamination ratio > 0.95".to_string(),
            );
            // 用户可覆盖此 Hold
            (hold.value(), Some(interrupt))
        } else if self.contamination_ratio > 0.85 {
            // 橙色预警：通知用户，建议专业支持
            let unknown = TritWord::unknown(Frame::Individual);
            let interrupt = MetaInterrupt::new(
                ConflictType::OutOfScope,
                "Role contamination ratio > 0.85".to_string(),
            );
            (unknown.value(), Some(interrupt))
        } else if self.contamination_ratio > 0.7 {
            // 黄色预警：建议恢复仪式
            (TritValue::Unknown, None)
        } else {
            // 正常
            (TritValue::Unknown, None)
        }
    }
}
```

---

## 五、持久化

- FrameModel 存储在 SQLite 的 `users` 表中（JSON 字段）
- 更新时：读取 → 修改 → 写回
- 版本控制：FrameModel 结构变更时，自动迁移
- 加密：FrameModel 存储在 SQLCipher 加密的数据库中

---

## 六、与 Trit-Core 的集成

### 6.1 Frame 扩展（直接扩展 enum）

```rust
// 在 Trit-Core 的 Frame enum 中新增变体
// 文件: trit-core/src/core/frame.rs

pub enum Frame {
    // 原有变体...
    GeoEco,
    Developmental,
    Role,
    Environmental,
}
```

**不使用 wrapper**：

```rust
// ❌ 已废弃的 wrapper 方案
// pub enum AuroraFrame {
//     Core(Frame),  // 致命问题：Core 变体中的 Meta 帧可被外部输入污染
//     GeoEco,
//     // ...
// }
//
// impl From<AuroraFrame> for Frame {
//     fn from(af: AuroraFrame) -> Self {
//         match af {
//             AuroraFrame::Core(f) => f,
//             _ => Frame::Meta,  // 致命错误：外部输入映射到 Meta
//         }
//     }
// }

// ✅ 正确方案：直接扩展 Frame enum
// 原有 Frame 变体不变，新增变体直接参与运算
// 跨 Frame 运算自动触发冲突检测
```

### 6.2 Domain 扩展（直接扩展 enum）

```rust
// 在 Trit-Core 的 Domain enum 中新增变体

pub enum Domain {
    // 原有变体...
    Organizational,
    Relational,
    Cognitive,
    Environmental,
}
```

---

*本文档为参考系建模模块的详细规格。v0.2.0 修正了 `self_weight` 为 `individual_weight`（与 Trit-Core v0.3.0 的 `Frame::Individual` 对齐），移除了 wrapper 定义（改为直接扩展 enum），增加了角色边界预警的系统响应（Awareness 原则：只通知，不阻止）。*
