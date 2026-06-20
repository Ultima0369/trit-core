# Aurora 管道设计：从数据到决策

**版本**: 0.1.0
**日期**: 2026-06-20
**状态**: 活跃
**分类**: 04_engineering — 工程规格

---

## 一、管道总览

```
原始数据采集 → 数据清洗 → 特征工程 → 小波分析 → Frame 标注
                                                    ↓
                                    Trit-Core 运算（TAND/TOR/仲裁）
                                                    ↓
                                    环境冲击检测 → 角色边界检测 → 元监控
                                                    ↓
                                    告警生成 → 注意力图谱更新 → 审计日志
```

---

## 二、管道阶段详解

### 2.1 阶段 1：原始数据采集（Raw Data Ingestion）

**输入**: 外部数据源（邮件客户端、日历、可穿戴设备）
**输出**: `RawSignal[]`
**延迟要求**: 实时（事件驱动）
**错误处理**: 跳过失败源，记录错误，继续其他源

**实现**: 
```rust
pub trait DataSource {
    fn connect(&mut self) -> Result<(), DataError>;
    fn fetch(&self, start: DateTime<Utc>, end: DateTime<Utc>) -> Result<Vec<RawSignal>, DataError>;
    fn disconnect(&mut self);
}
```

### 2.2 阶段 2：数据清洗（Data Sanitization）

**输入**: `RawSignal[]`
**输出**: `CleanSignal[]`
**延迟要求**: < 100ms
**错误处理**: 丢弃无效数据，标记异常

**清洗规则**:
- 时间戳验证（不能是未来时间，不能是 1970 年前）
- 元数据验证（必填字段存在，类型正确）
- PII 脱敏（姓名、地址、电话替换为哈希 ID）
- 内容不读取（只保留元数据，删除内容字段）

### 2.3 阶段 3：特征工程（Feature Engineering）

**输入**: `CleanSignal[]`
**输出**: `FeatureVector[]`
**延迟要求**: < 500ms
**错误处理**: 降级到简单统计特征

**特征提取**:
- 通信频率（消息数/小时）
- 响应延迟（平均回复时间）
- 社交亲密度（加权互动次数）
- 消费金额（元/天）
- HRV（ms/小时）
- 睡眠效率（%/夜）

### 2.4 阶段 4：小波分析（Wavelet Analysis）

**输入**: `FeatureVector[]`（时间序列）
**输出**: `WaveletResult`（CWT 输出）
**延迟要求**: < 1s（日数据）
**错误处理**: 降级到频谱分析

**算法**:
- CWT：连续小波变换，Morlet 母小波
- 输出：小波功率谱 $P(a,b)$
- 特征提取：基频、谐波、相位漂移、频谱重构

### 2.5 阶段 5：Frame 标注（Frame Annotation）

**输入**: `WaveletResult` + `FrameModel`
**输出**: `TritWord[]`（每个特征一个 TritWord）
**延迟要求**: < 100ms
**错误处理**: 使用默认 Frame

**标注规则**:
- 生理信号 → `Embodied`
- 社交信号 → `Relational`
- 个人信号 → `Individual`
- 环境信号 → `Environmental`
- 群体信号 → `Consensus`
- 数据信号 → `Science`

### 2.6 阶段 6：Trit-Core 运算（Ternary Computation）

**输入**: `TritWord[]`
**输出**: `TritDecision`（决策结果）
**延迟要求**: < 10ms
**错误处理**: SafeFallback 触发

**运算流程**:
1. `t_and_n`：批量 TAND 级联
2. `arbitrate`：域策略仲裁
3. `reflexive_guard`：自反审计（可选）
4. `safe_fallback`：危险域安全降级
5. `build_output`：构造输出

### 2.7 阶段 7：环境冲击检测（Environmental Shock Detection）

**输入**: `TritDecision` + `EnvironmentState`
**输出**: `ShockAlert`（如果检测到冲击）
**延迟要求**: < 100ms
**错误处理**: 保守估计（高冲击）

**检测逻辑**:
- 比较新旧环境的 Frame 权重差异
- 计算 ΔΦ
- 如果 ΔΦ > 0.5：强制 Hold，进入恢复模式

### 2.8 阶段 8：角色边界检测（Role Boundary Detection）

**输入**: `TritDecision` + `RoleState`
**输出**: `RoleAlert`（如果检测到污染）
**延迟要求**: < 100ms
**错误处理**: 保守估计（高污染）

**检测逻辑**:
- 计算角色权重 vs 自我权重
- 计算污染比、解离指数、边界渗透性
- 如果超过阈值：生成预警

### 2.9 阶段 9：元监控（MetaMonitor）

**输入**: 所有中间结果
**输出**: `MetaLog`（追加写入）
**延迟要求**: 追加写入，不可失败
**错误处理**: 不可失败（只读观察）

**监控内容**:
- 所有 Frame 的 Phase 历史
- 跨域冲突的频率和模式
- 环境冲击的恢复进度
- 角色边界的演化轨迹

### 2.10 阶段 10：告警生成（Alert Generation）

**输入**: 所有检测结果
**输出**: `Alert[]`
**延迟要求**: < 100ms
**错误处理**: 批量生成，降级显示

**告警类型**:
- 环境冲击预警
- 角色边界预警
- 跨域冲突提醒
- 恢复进度提醒
- 系统状态提醒

### 2.11 阶段 11：可视化更新（Visualization Update）

**输入**: `Alert[]` + `TritDecision[]`
**输出**: UI 更新
**延迟要求**: < 200ms
**错误处理**: 异步更新

**更新内容**:
- 注意力图谱
- 冲突面板
- 节奏报告
- 环境预警
- 决策审计

---

## 三、管道配置

### 3.1 可配置参数

```rust
pub struct PipelineConfig {
    pub wavelet_mother: MotherWavelet,      // 母小波类型
    pub wavelet_scales: Vec<f64>,          // 分析尺度
    pub shock_threshold: f64,               // 冲击阈值 (0.5)
    pub role_contamination_threshold: f64,  // 角色污染阈值 (0.7)
    pub alert_batch_size: usize,           // 告警批量大小
    pub audit_log_retention: Duration,     // 审计日志保留期
}
```

### 3.2 管道模式

| 模式 | 数据频率 | 分析深度 | 用途 |
|------|----------|----------|------|
| 实时模式 | 每秒 | 轻量 | 实时监控 |
| 日常模式 | 每小时 | 完整 | 日常分析 |
| 深度模式 | 每天 | 全面 | 深度报告 |
| 手动模式 | 按需 | 用户选择 | 特定查询 |

---

## 四、性能优化

### 4.1 热路径优化

- 小波分析：Rust 实现，SIMD 优化
- Trit-Core 运算：热路径零分配，~4ns
- 数据库查询：索引优化，预编译语句

### 4.2 缓存策略

- Frame 权重：缓存（月级更新）
- 环境数据：缓存（小时级更新）
- 小波结果：缓存（日级更新）
- 注意力向量：缓存（实时计算，但可复用）

### 4.3 降级策略

| 资源不足 | 降级策略 |
|----------|----------|
| CPU 高 | 减少小波尺度数量 |
| 内存高 | 减少数据保留期 |
| 磁盘高 | 压缩审计日志 |
| 电池低 | 暂停非关键分析 |

---

*本文档为 Aurora 的数据流管道设计。完整模块规格见 07_specs/ 目录。不是指教，是提醒。*
