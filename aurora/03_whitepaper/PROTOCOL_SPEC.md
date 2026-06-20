# Aurora 技术白皮书：协议规格

**版本**：0.2.0
**日期**：2026-06-20
**状态**：活跃
**分类**：03_whitepaper — 技术白皮书

---

## 一、摘要

Aurora 是一个**本地优先的注意力量化与三值决策协议系统**。它通过小波变换将离散行为流转化为连续时间-频率表示，通过地理生态参考系标注认知框架，通过 Trit-Core 三值协议在跨域冲突时输出 `Hold` + 完整审计记录。

核心主张：
> 在注意力资本主义时代，**允许悬置的三值协议比二值概率输出更能保留冲突信息、保护认知主权、避免系统级默化。**

---

## 二、协议架构

### 2.1 五层架构

```
应用层（Presentation）
  ├── 注意力图谱
  ├── 冲突面板
  ├── 节奏报告
  ├── 环境冲击预警
  └── 决策审计

Trit-Core 决策层（Protocol）
  ├── 信号标注（Frame）
  ├── 跨域冲突检测（TAND/TOR）
  ├── 域仲裁（ResolutionPolicy）
  ├── 注意力量化（Phase 场）
  ├── 安全回退（SafeFallback）
  ├── 元监控（MetaMonitor）
  └── 觉察通知（Awareness/Transparency）

小波分析层（Wavelet Analytics）
  ├── 多尺度分解
  ├── 基频识别
  ├── 谐波检测
  ├── 相位漂移检测
  ├── 频谱重构检测
  └── 跨信号同步

参考系建模层（Frame Modeling）
  ├── 地理生态框架（GeoEco）
  ├── 成长轨迹框架（Developmental）
  ├── 环境相位冲击（Environmental Shock）
  └── 角色边界监控（Role Boundary）

数据采集层（Data Ingestion）
  ├── 数字信号（通信、日程、社交）
  ├── 生理信号（HRV、睡眠、可穿戴）
  ├── 环境信号（气象、地理）
  ├── 成长档案（手动输入）
  └── 公开情报（定向抓取）
```

### 2.2 数据流

```
原始数据 → 数据清洗 → 小波变换 → 特征提取 → Phase 归一化 → Frame 标注
                                                                    ↓
                                                                     ├─→ Trit-Core 核心运算（TAND/TOR/仲裁）
                                                                     │
                                                                     └─→ 环境冲击检测（ΔΦ 计算）
                                                                                ↓
                                                                     元监控（MetaMonitor 观察）
                                                                                ↓
                                                                     应用层输出（图谱/冲突/节奏/审计）
```

---

## 三、扩展的 Frame 系统

### 3.1 Trit-Core 原有 Frame

```rust
pub enum Frame {
    Science,      // 经验/证据驱动
    Individual,   // 个人上下文/个人事实
    Consensus,    // 统计/群体偏好
    Absolute,     // 不可知/不可观测（永远 Hold）
    Meta,         // 冲突仲裁/策略输出（系统内部使用，不可作为外部输入）
    FirstPerson,  // 第一人称主观报告
    Embodied,     // 身体/生理状态参考系
    Relational,   // 关系/社会互动参考系
}
```

### 3.2 Aurora 扩展 Frame

**扩展方式**：直接在 `Frame` enum 中新增变体，不使用 wrapper + `From` 映射。

```rust
pub enum Frame {
    // 原有 Frame ...
    
    // Aurora 扩展 — 直接作为独立变体
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
- 外部输入不可映射到 `Meta` 帧（检测到则返回 `PolicyViolation::FrameContamination`）

**语义定义**：

| Frame | 定义 | 数据来源 | 更新频率 |
|-------|------|----------|----------|
| `GeoEco` | 地理生态对认知模式的约束 | 出生地、迁徙路径、气候、土壤 | 月/年（慢变） |
| `Developmental` | 早期经历对神经回路的 imprint | 依附模式、关键事件、成长轨迹 | 年（极慢变） |
| `Role` | 当前社会角色的认知框架 | 职业、职位、社会身份 | 周/月（快变） |
| `Environmental` | 当前物理环境的实时状态 | 位置、气候、社交密度 | 实时（快变） |

### 3.3 Frame 的权重分配

每个 Frame 在特定 Environment 中有默认权重：

```rust
pub struct FrameWeights {
    pub science: f64,
    pub individual: f64,
    pub consensus: f64,
    pub first_person: f64,
    pub embodied: f64,
    pub relational: f64,
    pub geo_eco: f64,
    pub developmental: f64,
    pub role: f64,
    pub environmental: f64,
}

impl FrameWeights {
    pub fn for_environment(env: Environment) -> Self {
        match env {
            Environment::SubtropicalMonsoon => Self {
                embodied: 0.3, individual: 0.4, consensus: 0.5,
                relational: 0.2, science: 0.6, geo_eco: 0.3,
                developmental: 0.2, role: 0.1, environmental: 0.1,
                first_person: 0.3,
            },
            Environment::TropicalSavanna => Self {
                embodied: 0.8, individual: 0.2, consensus: 0.1,
                relational: 0.9, science: 0.3, geo_eco: 0.8,
                developmental: 0.2, role: 0.1, environmental: 0.8,
                first_person: 0.3,
            },
            // ... 其他环境
        }
    }
}
```

---

## 四、扩展的 Domain 系统

### 4.1 Trit-Core 原有 Domain

```rust
pub enum Domain {
    Physical,        // 硬科学约束
    Engineering,     // 应用约束
    MedicalEthics,   // 软约束
    ValueJudgment,   // 不可通约
    General,         // 默认协商
    Custom(String),  // 外部规则
}
```

### 4.2 Aurora 扩展 Domain

```rust
pub enum Domain {
    // 原有 Domain ...
    
    // Aurora 扩展
    Organizational,   // 组织决策（人员、战略、资源）
    Relational,       // 关系决策（信任、合作、边界）
    Cognitive,        // 认知决策（注意力分配、学习、创作）
    Environmental,    // 环境适应决策（迁移、旅行、定居）
}
```

**仲裁规则**：

| Domain | 优先 Frame | 冲突处理 | 逻辑 |
|--------|-----------|----------|------|
| `Organizational` | 无（Negotiate） | `Negotiate` | 组织决策需要统合多方参考系，不预设统合帧 |
| `Relational` | `Relational` | `Preserve` | 关系决策中，关系历史优先 |
| `Cognitive` | `Embodied` | `Preserve` | 认知决策中，生理状态是硬约束 |
| `Environmental` | `GeoEco` | `Negotiate` | 环境决策中，地理生态是长期约束 |

**注意**：`Organizational` 不再使用 `Meta` 作为统合帧。单 Frame 时直接 `Commit`，跨 Frame 时 `Negotiate`。`Meta` 帧只用于系统内部冲突输出。

---

## 五、环境相位冲击协议

### 5.1 冲击检测

当检测到环境切换（地理位置显著变化、气候带变化、社交密度剧变）：

```rust
pub struct EnvironmentalShockDetector {
    pub old_environment: Environment,
    pub new_environment: Environment,
    pub delta_phi: f64,              // 相位冲击量
    pub shock_level: ShockLevel,     // 微震/中震/强震/毁灭级
    pub recovery_eta: Duration,      // 预计恢复时间
}
```

### 5.2 系统响应

| 冲击等级 | ΔΦ | 系统响应 |
|----------|-----|----------|
| 微震 | 0.0-0.2 | 无干预，自动校准 |
| 中震 | 0.2-0.5 | 恢复建议：增加睡眠、营养、社交 |
| 强震 | 0.5-0.8 | 输出 Hold：所有决策建议暂停，仅监控恢复。用户可覆盖。 |
| 毁灭级 | 0.8-1.0 | 系统进入 Awareness：通知用户当前冲击等级，用户自负其责。系统不阻止任何操作。 |

**关键变化**：
- 强震时系统输出 `Hold`，但**用户可覆盖**（不剥夺原则）
- 毁灭级时系统进入 `Awareness`（觉察通知），**不进入安全模式**（不阻断原则）
- 系统只通知用户发生了什么，用户自行决定下一步

### 5.3 恢复曲线监控

```rust
pub struct RecoveryCurve {
    pub start_time: DateTime<Utc>,
    pub target_phase: Phase,         // 新稳态的 Phase
    pub current_phase: Phase,        // 当前 Phase
    pub variance_7d: f64,           // 最近7天方差
    pub slope: f64,                  // 趋势斜率
    pub is_stable: bool,             // 是否稳定（方差 < 0.01，斜率 ≈ 0）
}
```

---

## 六、角色边界监控协议

### 6.1 角色入侵检测

```rust
pub struct RoleBoundaryMetrics {
    pub role_frame_weight: Phase,       // 角色帧权重
    pub self_frame_weight: Phase,       // 自我帧权重（Frame::Individual）
    pub contamination_ratio: f64,       // 角色→自我的污染比
    pub dissociation_index: f64,        // 解离指数
    pub boundary_permeability: f64,     // 边界渗透性
}
```

### 6.2 预警阈值

| 指标 | 黄色预警 | 橙色预警 | 红色预警 |
|------|---------|---------|---------|
| `contamination_ratio` | > 0.7 | > 0.85 | > 0.95 |
| `dissociation_index` | > 0.4 | > 0.5 | > 0.6 |
| `boundary_permeability` | > 0.6 | > 0.75 | > 0.9 |

### 6.3 系统响应

- 黄色：建议角色后恢复仪式
- 橙色：建议专业支持（心理学/督导）
- 红色：系统输出 `Hold` 所有角色相关决策。**用户可覆盖。** 系统不阻止用户继续操作，只记录用户选择覆盖。

---

## 七、注意力量化协议

### 7.1 注意力向量场

$$A_{total} = \int_{Frames} \int_{Time} P(Frame, t) \cdot w(Frame) \, dt \, dFrame$$

**分解维度**：

| 维度 | 公式 | 意义 |
|------|------|------|
| 强度 | $I = \max_{Frame} P(Frame)$ | 当前最强关注焦点 |
| 分散度 | $D = H(P) / \log N$ | 注意力是聚焦还是分散 |
| 方向 | $\vec{v} = \sum P(Frame) \cdot \hat{e}_{Frame}$ | 注意力向量的合成方向 |
| 变化率 | $d\vec{v}/dt$ | 注意力转移的速度和方向 |
| 冲突度 | $C = \sum_{i \neq j} |P_i - P_j| \cdot \delta_{ij}$ | 不同参考系之间的张力 |

### 7.2 输出格式

```rust
pub struct AttentionVector {
    pub intensity: Phase,           // 强度
    pub dispersion: Phase,          // 分散度
    pub direction: Vec<FramePhase>, // 方向（各 Frame 的 Phase）
    pub change_rate: Phase,         // 变化率
    pub conflict_level: Phase,      // 冲突度
}
```

---

## 八、安全模型

### 8.1 威胁分析

| 威胁 | 可能性 | 影响 | 缓解 |
|------|--------|------|------|
| 数据泄露（本地设备被盗） | 中 | 高 | SQLCipher 加密（AES-256-CBC + HMAC-SHA256），密钥用户持有[^1] |
| 恶意输入（伪造数据） | 低 | 中 | 输入验证、来源校验、异常检测 |
| 元监控劫持 | 低 | 极高 | MetaMonitor 只读，不可被外部修改 |
| 算法偏见 | 中 | 高 | 多参考系冲突检测，不依赖单一模型 |
| 用户误读（Hold 被当作失败） | 高 | 中 | 产品教育、UI 明确标注、onboarding 引导 |

[^1]: SQLite 原生不支持加密。使用 SQLCipher（`rusqlite` 的 `bundled-sqlcipher` 特性），加密方案为 AES-256-CBC + HMAC-SHA256。不是 AES-256-GCM。

### 8.2 数据主权保证

- 本地运行：所有计算在本地完成
- 内容不读取：只提取元数据，不读取内容
- 用户控制：随时导出、删除、断网
- 加密：SQLCipher（AES-256-CBC + HMAC-SHA256），密钥用户持有[^1]
- 审计：所有操作追加日志，不可篡改（链式哈希）

### 8.3 SecurityMode 集成

| 状态 | 行为 | 用户权利 |
|------|------|---------|
| Normal | 正常运算 | 用户可关闭、可覆盖、可离开 |
| Awareness | 检测到策略违反，返回通知，运算继续 | 用户可选择继续或停止 |
| Transparency | 主动公开所有内部状态 | 用户可选择查看或忽略 |

**关键**：系统在任何状态下**不阻止用户操作**。`Awareness` 不是 `SafeMode`。系统只通知，不决定。

---

## 九、性能规格

| 指标 | 目标 | 验证方式 |
|------|------|----------|
| 小波分析延迟 | < 1秒（单用户，日数据） | Criterion 基准测试 |
| Trit-Core 运算 | < 10ms（单次决策） | 微基准测试 |
| 内存占用 | < 500MB（单用户） | dhat 堆分析 |
| 存储占用 | < 1GB（单用户，90天数据） | 磁盘使用监控 |
| 启动时间 | < 3秒 | 手动计时 |
| 离线可用 | 100% 功能可用 | 断网测试 |

---

## 十、结论

Aurora 协议的核心创新：
1. **小波注意力分析**：把离散行为变成连续时间-频率表示，检测节奏变化而非阈值偏离
2. **地理生态参考系**：把环境作为认知框架的约束条件，而非背景
3. **三值决策协议**：在跨域冲突时保留 Hold，不强制输出
4. **环境相位冲击检测**：识别参考系重构，在恢复完成前输出 Hold（用户可覆盖）
5. **角色边界监控**：检测角色入侵，预警解离风险
6. **元监控节点**：不随环境改变的观察层，对应觉悟功夫
7. **觉察通知协议**：系统不阻止用户，只通知用户发生了什么

---

*本白皮书为 Aurora 协议的技术规格。v0.2.0 修正了 Frame 扩展方式（直接扩展 enum，非 wrapper）、毁灭级冲击响应（Awareness 替代 SafeMode）、加密方案（SQLCipher 替代 AES-256-GCM）、角色边界红色预警（用户可覆盖）。所有扩展的 Frame/Domain 在 Trit-Core v0.3.0 基础上进行向后兼容的扩展。不是指教，是提醒。*
