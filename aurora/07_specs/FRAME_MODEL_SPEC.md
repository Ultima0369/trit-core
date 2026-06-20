# 参考系建模模块规格

**版本**: 0.1.0
**日期**: 2026-06-20
**状态**: 活跃
**分类**: 07_specs — 详细模块规格

---

## 一、模块职责

管理用户的参考系模型（地理生态、成长轨迹、当前角色、环境状态），计算 Frame 权重，检测环境冲击和角色边界污染。

## 二、接口定义

```rust
pub struct FrameModel {
    pub geo_eco: GeoEcoProfile,
    pub developmental: DevelopmentalProfile,
    pub environmental: EnvironmentalState,
    pub role: RoleState,
}

impl FrameModel {
    pub fn weights(&self) -> FrameWeights;
    pub fn shock_level(&self, old_env: &Environment, new_env: &Environment) -> ShockResult;
    pub fn role_boundary(&self) -> RoleBoundaryMetrics;
    pub fn update_environment(&mut self, new_env: EnvironmentalState);
    pub fn update_role(&mut self, new_role: RoleState);
}

pub struct ShockResult {
    pub delta_phi: f64,
    pub level: ShockLevel,  // Micro / Moderate / Strong / Catastrophic
    pub recovery_eta: Duration,
}

pub struct RoleBoundaryMetrics {
    pub role_weight: Phase,
    pub self_weight: Phase,
    pub contamination_ratio: f64,
    pub dissociation_index: f64,
    pub boundary_permeability: f64,
}
```

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
    let self_weight = self.developmental.self_weight();
    let contamination_ratio = role_weight.value() / (self_weight.value() + 1e-6);
    
    RoleBoundaryMetrics {
        role_weight,
        self_weight,
        contamination_ratio,
        dissociation_index: self.calculate_dissociation(),
        boundary_permeability: self.calculate_permeability(),
    }
}
```

## 五、持久化

- FrameModel 存储在 SQLite 的 `users` 表中（JSON 字段）
- 更新时：读取 → 修改 → 写回
- 版本控制：FrameModel 结构变更时，自动迁移

---

*不是指教，是提醒。*
