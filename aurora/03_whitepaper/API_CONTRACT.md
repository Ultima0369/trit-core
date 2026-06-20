# Aurora API 契约

**版本**: 0.1.0
**日期**: 2026-06-20
**状态**: 活跃
**分类**: 03_whitepaper — 技术白皮书

---

## 一、API 版本策略

- **语义化版本**：MAJOR.MINOR.PATCH
- **MAJOR 变更**：破坏向后兼容（如删除 API、改变参数类型）
- **MINOR 变更**：新增功能，向后兼容
- **PATCH 变更**：Bug 修复，向后兼容
- **当前版本**：0.1.0

---

## 二、公共 API 接口

### 2.1 数据采集 API

```rust
// 注册数据源
pub fn register_data_source(&mut self, source: Box<dyn DataSource>) -> Result<DataSourceId, DataError>;

// 获取原始信号
pub fn fetch_raw_signals(
    &self,
    source_id: DataSourceId,
    start: DateTime<Utc>,
    end: DateTime<Utc>,
) -> Result<Vec<RawSignal>, DataError>;

// 获取所有已注册数据源
pub fn list_data_sources(&self) -> Vec<DataSourceInfo>;
```

### 2.2 小波分析 API

```rust
// 分析信号
pub fn analyze_signal(&self, signal: &[f64]) -> Result<WaveletResult, WaveletError>;

// 提取特征
pub fn extract_features(&self, result: &WaveletResult) -> Result<Vec<WaveletFeature>, WaveletError>;

// 获取基频
pub fn get_fundamental_frequency(&self, result: &WaveletResult) -> Result<f64, WaveletError>;

// 获取相位漂移
pub fn get_phase_drift(&self, result: &WaveletResult) -> Result<Vec<PhaseDrift>, WaveletError>;
```

### 2.3 Trit-Core 决策 API

```rust
// 创建 TritWord
pub fn create_trit_word(
    value: TritValue,
    phase: Phase,
    frame: AuroraFrame,
) -> Result<TritWord, WordError>;

// 执行 TAND
pub fn t_and(a: &TritWord, b: &TritWord) -> (TritWord, Option<MetaInterrupt>);

// 执行 TOR
pub fn t_or(a: &TritWord, b: &TritWord) -> (TritWord, Option<MetaInterrupt>);

// 域仲裁
pub fn arbitrate(
    domain: AuroraDomain,
    inputs: &[TritWord],
    policy: &ResolutionPolicy,
) -> Result<ArbitrationResult, PolicyError>;

// 安全降级
pub fn safe_fallback(
    domain: AuroraDomain,
    result: &ArbitrationResult,
    interrupts: &[MetaInterrupt],
) -> TritValue;
```

### 2.4 环境冲击 API

```rust
// 计算环境相位冲击
pub fn calculate_shock(
    &self,
    old_env: &Environment,
    new_env: &Environment,
    user_weights: &FrameWeights,
) -> Result<ShockResult, ShockError>;

// 获取恢复状态
pub fn get_recovery_state(&self, user_id: Uuid) -> Option<RecoveryState>;

// 检查恢复完成
pub fn is_recovery_complete(&self, user_id: Uuid) -> bool;
```

### 2.5 角色边界 API

```rust
// 计算角色边界指标
pub fn calculate_role_boundary(
    &self,
    role_weight: Phase,
    self_weight: Phase,
) -> RoleBoundaryMetrics;

// 获取污染预警
pub fn get_contamination_alert(&self, metrics: &RoleBoundaryMetrics) -> Option<Alert>;
```

### 2.6 注意力向量 API

```rust
// 计算注意力向量
pub fn calculate_attention_vector(
    &self,
    features: &[WaveletFeature],
    weights: &FrameWeights,
) -> AttentionVector;

// 获取注意力图谱
pub fn get_attention_map(&self, user_id: Uuid) -> Result<AttentionMap, AttentionError>;

// 获取冲突面板
pub fn get_conflict_dashboard(&self, user_id: Uuid) -> Result<ConflictDashboard, AttentionError>;
```

### 2.7 审计日志 API

```rust
// 获取决策审计日志
pub fn get_decision_audit(
    &self,
    user_id: Uuid,
    decision_id: Uuid,
) -> Result<AuditLog, AuditError>;

// 获取所有决策日志
pub fn list_decisions(
    &self,
    user_id: Uuid,
    start: DateTime<Utc>,
    end: DateTime<Utc>,
) -> Result<Vec<DecisionSummary>, AuditError>;
```

---

## 三、错误类型

```rust
pub enum AuroraError {
    DataError(DataError),
    WaveletError(WaveletError),
    TritError(TritError),
    ShockError(ShockError),
    PolicyError(PolicyError),
    AuditError(AuditError),
    ValidationError(String),
    SecurityError(String),
    InternalError(String),
}

impl std::error::Error for AuroraError { ... }
```

---

## 四、序列化格式

### 4.1 JSON 输出格式

所有 API 返回的序列化格式：

```json
{
  "version": "0.1.0",
  "timestamp": "2026-06-20T12:00:00Z",
  "status": "success" | "error",
  "data": { ... },
  "error": null | { "code": "...", "message": "..." }
}
```

### 4.2 二进制格式（内部）

- 使用 bincode 或 MessagePack 进行高效序列化
- 仅在内部模块间使用，不暴露给用户

---

## 五、向后兼容性

### 5.1 保证

- MINOR 版本增加：新增字段有默认值，旧客户端可忽略
- PATCH 版本：无 API 变更
- MAJOR 版本：明确标注破坏变更，提供迁移指南

### 5.2 废弃策略

- 废弃 API 保留 2 个 MAJOR 版本
- 废弃前标记 `#[deprecated]`，附迁移建议
- 废弃后返回 `AuroraError::ValidationError`

---

*本文档为 Aurora 的公共 API 契约。所有接口在此定义，后续变更需经过 ADR 流程。不是指教，是提醒。*
