# Trit-Core 集成规格

**版本**: 0.1.0
**日期**: 2026-06-20
**状态**: 活跃
**分类**: 07_specs — 详细模块规格

---

## 一、模块职责

将 Aurora 的扩展 Frame/Domain 集成到 Trit-Core 中，保持向后兼容。

## 二、集成点

### 2.1 Frame 扩展

```rust
// 扩展 Trit-Core 的 Frame
pub enum AuroraFrame {
    Core(Frame),  // Trit-Core 原有 Frame
    GeoEco,
    Developmental,
    Role,
    Environmental,
}

// 实现与 Trit-Core Frame 的转换
impl From<AuroraFrame> for Frame {
    fn from(af: AuroraFrame) -> Self {
        match af {
            AuroraFrame::Core(f) => f,
            // 扩展 Frame 映射到 Meta（在 Trit-Core 内部表示）
            _ => Frame::Meta,
        }
    }
}
```

### 2.2 Domain 扩展

```rust
pub enum AuroraDomain {
    Core(Domain),
    Organizational,
    Relational,
    Cognitive,
    Environmental,
}

impl AuroraDomain {
    pub fn arbitrate(&self, inputs: &[TritWord]) -> Result<ArbitrationResult, PolicyError> {
        match self {
            AuroraDomain::Core(d) => ResolutionPolicy::arbitrate(d, inputs),
            AuroraDomain::Organizational => self.arbitrate_organizational(inputs),
            AuroraDomain::Relational => self.arbitrate_relational(inputs),
            AuroraDomain::Cognitive => self.arbitrate_cognitive(inputs),
            AuroraDomain::Environmental => self.arbitrate_environmental(inputs),
        }
    }
}
```

### 2.3 仲裁规则

```rust
impl AuroraDomain {
    fn arbitrate_organizational(&self, inputs: &[TritWord]) -> Result<ArbitrationResult, PolicyError> {
        // 组织决策：Meta 统合，跨 Frame Negotiate
        if has_cross_frame(inputs) {
            Ok(ArbitrationResult::Negotiate)
        } else {
            Ok(ArbitrationResult::Commit(inputs[0].clone(), Frame::Meta))
        }
    }
    
    fn arbitrate_relational(&self, inputs: &[TritWord]) -> Result<ArbitrationResult, PolicyError> {
        // 关系决策：Relational 优先 Preserve
        if let Some(relational) = find_frame(inputs, Frame::Relational) {
            Ok(ArbitrationResult::Preserve(relational.clone(), Frame::Relational))
        } else {
            Ok(ArbitrationResult::Negotiate)
        }
    }
    
    fn arbitrate_cognitive(&self, inputs: &[TritWord]) -> Result<ArbitrationResult, PolicyError> {
        // 认知决策：Embodied 优先 Preserve
        if let Some(embodied) = find_frame(inputs, Frame::Embodied) {
            Ok(ArbitrationResult::Preserve(embodied.clone(), Frame::Embodied))
        } else {
            Ok(ArbitrationResult::Negotiate)
        }
    }
    
    fn arbitrate_environmental(&self, inputs: &[TritWord]) -> Result<ArbitrationResult, PolicyError> {
        // 环境决策：GeoEco 优先，但 Negotiate
        if let Some(geo_eco) = find_frame(inputs, Frame::GeoEco) {
            Ok(ArbitrationResult::Preserve(geo_eco.clone(), Frame::GeoEco))
        } else {
            Ok(ArbitrationResult::Negotiate)
        }
    }
}
```

## 三、测试要求

- 所有原有 Trit-Core 测试继续通过（向后兼容）
- 新 Frame 的冲突检测测试通过
- 新 Domain 的仲裁规则测试通过
- 扩展 Frame 与原有 Frame 的跨域冲突测试通过

---

*不是指教，是提醒。*
