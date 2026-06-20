# Aurora 工程规格：系统设计

**版本**：0.1.0
**日期**：2026-06-20
**状态**：活跃
**分类**：04_engineering — 工程规格

---

## 一、设计目标

### 1.1 功能目标（Functional Goals）

| 目标 | 优先级 | 说明 |
|------|--------|------|
| 采集个人注意力数据 | P0 | 通信、日程、行为、生理、环境 |
| 分析注意力节律 | P0 | 小波变换、基频、谐波、相位漂移 |
| 检测跨域冲突 | P0 | Trit-Core 三值协议、MetaInterrupt |
| 输出注意力图谱 | P0 | 可视化、可交互、可审计 |
| 检测环境冲击 | P1 | 地理生态变化、冲击等级、恢复曲线 |
| 监控角色边界 | P1 | 角色入侵、解离预警、恢复仪式 |
| 支持团队/组织 | P2 | 多节点拓扑、级联风险、组织涡旋 |

### 1.2 非功能目标（Non-Functional Goals）

| 目标 | 指标 | 验证方式 |
|------|------|----------|
| 本地优先 | 100% 离线可用 | 断网测试 |
| 数据主权 | 用户完全控制数据 | 导出/删除测试 |
| 性能 | 单用户 < 1GB 内存 | 压力测试 |
| 安全 | 无已知高危漏洞 | 安全审计 |
| 可维护 | 模块化、可替换 | 架构评审 |
| 可扩展 | 新 Frame/Domain 可插拔 | 扩展测试 |

---

## 二、技术栈

### 2.1 核心栈

| 层级 | 技术 | 选择理由 |
|------|------|----------|
| 语言 | Rust | 性能、安全、零 unsafe（Trit-Core 已有） |
| 数据库 | SQLite | 本地嵌入、零配置、文件即数据库 |
| 桌面 UI | Tauri | Rust 后端 + Web 前端，本地优先 |
| 前端 | HTML/CSS/JS + Web Components | 轻量、跨平台、不依赖 heavy framework |
| 小波分析 | 自研 Rust + 可选 Python 桥接 | 性能 vs 开发速度的平衡 |
| 协议核心 | Trit-Core（Rust crate） | 已有实现，向后兼容扩展 |

### 2.2 可选栈

| 场景 | 技术 | 说明 |
|------|------|------|
| 小波快速原型 | Python + PyWavelets | 验证算法可行性，后续用 Rust 重写热路径 |
| 可视化 | D3.js / ECharts | 注意力图谱、小波尺度图渲染 |
| 同步 | 端到端加密 P2P | 多设备同步，非默认，用户可选 |
| 移动 | Tauri Mobile / React Native | 未来扩展，M2 后考虑 |

---

## 三、模块设计

### 3.1 模块清单

```
Aurora/
├── aurora-core/          # 核心库（Rust crate）
│   ├── src/
│   │   ├── data/         # 数据采集模块
│   │   ├── wavelet/      # 小波分析模块
│   │   ├── frame/        # 参考系建模模块
│   │   ├── trit/         # Trit-Core 扩展模块
│   │   ├── alert/        # 告警引擎模块
│   │   ├── audit/        # 审计日志模块
│   │   └── lib.rs        # 公共 API
│   └── tests/            # 单元测试、集成测试
├── aurora-desktop/       # 桌面应用（Tauri）
│   ├── src-tauri/        # Rust 后端
│   └── src/              # Web 前端
├── aurora-cli/           # 命令行工具
│   └── src/
├── aurora-scenarios/     # 场景套件（JSON）
└── docs/                 # 文档（指向 aurora/ 目录）
```

### 3.2 模块接口

#### 数据采集模块（`aurora_core::data`）

```rust
pub trait DataSource {
    fn name(&self) -> &str;
    fn connect(&mut self) -> Result<(), DataError>;
    fn fetch(&self, start: DateTime<Utc>, end: DateTime<Utc>) -> Result<Vec<RawSignal>, DataError>;
    fn disconnect(&mut self);
}

pub struct RawSignal {
    pub timestamp: DateTime<Utc>,
    pub source_type: SourceType,  // Communication / Calendar / Location / Physiological / Environmental
    pub metadata: HashMap<String, String>,  // 频率、时长、方向等元数据
    pub content_hash: Option<String>,  // 内容哈希（不存储内容本身）
}
```

#### 小波分析模块（`aurora_core::wavelet`）

```rust
pub struct WaveletEngine {
    pub mother_wavelet: MotherWavelet,  // Morlet / Daubechies-4
    pub scales: Vec<f64>,               // 分析尺度
}

impl WaveletEngine {
    pub fn analyze(&self, signal: &[f64]) -> Result<WaveletResult, WaveletError>;
    pub fn extract_features(&self, result: &WaveletResult) -> Result<Vec<WaveletFeature>, WaveletError>;
}

pub struct WaveletFeature {
    pub feature_type: FeatureType,  // FundamentalFreq / Harmonic / PhaseDrift / SpectralReconfig / CrossSync
    pub value: f64,
    pub confidence: f64,            // 信噪比
    pub timestamp: DateTime<Utc>,
}
```

#### 参考系建模模块（`aurora_core::frame`）

```rust
pub struct FrameModel {
    pub geo_eco: GeoEcoProfile,
    pub developmental: DevelopmentalProfile,
    pub environmental: EnvironmentalState,
    pub role: RoleState,
}

impl FrameModel {
    pub fn weights(&self) -> FrameWeights;
    pub fn shock_level(&self, old_env: &Environment, new_env: &Environment) -> ShockLevel;
    pub fn role_boundary(&self) -> RoleBoundaryMetrics;
}
```

#### Trit-Core 扩展模块（`aurora_core::trit`）

```rust
// 扩展 Trit-Core 的 Frame
pub enum AuroraFrame {
    Core(Frame),          // Trit-Core 原有 Frame
    GeoEco,
    Developmental,
    Role,
    Environmental,
}

// 扩展 Trit-Core 的 Domain
pub enum AuroraDomain {
    Core(Domain),         // Trit-Core 原有 Domain
    Organizational,
    Relational,
    Cognitive,
    Environmental,
}

// 扩展 MetaInterrupt
pub enum AuroraInterruptType {
    Core(ConflictType),   // Trit-Core 原有中断类型
    EnvironmentalPhaseShock,
    RoleContamination,
    SpectralReconfiguration,
}
```

---

## 四、数据模型

### 4.1 核心实体

```rust
// 用户
pub struct User {
    pub id: Uuid,
    pub created_at: DateTime<Utc>,
    pub frame_model: FrameModel,      // 参考系模型
    pub alert_settings: AlertSettings, // 告警设置
}

// 原始信号
pub struct RawSignal {
    pub id: Uuid,
    pub user_id: Uuid,
    pub timestamp: DateTime<Utc>,
    pub source_type: SourceType,
    pub metadata: HashMap<String, String>,
}

// 小波特征
pub struct WaveletFeature {
    pub id: Uuid,
    pub user_id: Uuid,
    pub timestamp: DateTime<Utc>,
    pub feature_type: FeatureType,
    pub value: f64,
    pub confidence: f64,
    pub phase: Phase,                 // 归一化到 [0,1]
    pub frame: AuroraFrame,
}

// Trit 决策
pub struct TritDecision {
    pub id: Uuid,
    pub user_id: Uuid,
    pub timestamp: DateTime<Utc>,
    pub domain: AuroraDomain,
    pub result: TritValue,              // True / Hold / False / Unknown
    pub phase: Phase,
    pub interrupts: Vec<MetaInterrupt>,
    pub audit_log: String,              // 不可篡改的审计日志
}

// 告警
pub struct Alert {
    pub id: Uuid,
    pub user_id: Uuid,
    pub timestamp: DateTime<Utc>,
    pub alert_type: AlertType,
    pub severity: Severity,
    pub message: String,
    pub meta_interrupt: Option<MetaInterrupt>,
    pub resolved: bool,
}
```

### 4.2 SQLite Schema

```sql
-- 用户表
CREATE TABLE users (
    id BLOB PRIMARY KEY,  -- UUID
    created_at TEXT NOT NULL,  -- ISO 8601
    frame_model TEXT NOT NULL,  -- JSON
    alert_settings TEXT NOT NULL  -- JSON
);

-- 原始信号表
CREATE TABLE raw_signals (
    id BLOB PRIMARY KEY,
    user_id BLOB NOT NULL REFERENCES users(id),
    timestamp TEXT NOT NULL,
    source_type TEXT NOT NULL,
    metadata TEXT NOT NULL,  -- JSON
    FOREIGN KEY (user_id) REFERENCES users(id)
);
CREATE INDEX idx_signals_user_time ON raw_signals(user_id, timestamp);

-- 小波特征表
CREATE TABLE wavelet_features (
    id BLOB PRIMARY KEY,
    user_id BLOB NOT NULL,
    timestamp TEXT NOT NULL,
    feature_type TEXT NOT NULL,
    value REAL NOT NULL,
    confidence REAL NOT NULL,
    phase REAL NOT NULL,
    frame TEXT NOT NULL,
    FOREIGN KEY (user_id) REFERENCES users(id)
);
CREATE INDEX idx_features_user_time ON wavelet_features(user_id, timestamp);

-- Trit 决策表
CREATE TABLE trit_decisions (
    id BLOB PRIMARY KEY,
    user_id BLOB NOT NULL,
    timestamp TEXT NOT NULL,
    domain TEXT NOT NULL,
    result TEXT NOT NULL,
    phase REAL NOT NULL,
    interrupts TEXT NOT NULL,  -- JSON
    audit_log TEXT NOT NULL,
    FOREIGN KEY (user_id) REFERENCES users(id)
);
CREATE INDEX idx_decisions_user_time ON trit_decisions(user_id, timestamp);

-- 告警表
CREATE TABLE alerts (
    id BLOB PRIMARY KEY,
    user_id BLOB NOT NULL,
    timestamp TEXT NOT NULL,
    alert_type TEXT NOT NULL,
    severity TEXT NOT NULL,
    message TEXT NOT NULL,
    meta_interrupt TEXT,  -- JSON
    resolved INTEGER NOT NULL DEFAULT 0,
    FOREIGN KEY (user_id) REFERENCES users(id)
);
CREATE INDEX idx_alerts_user_resolved ON alerts(user_id, resolved);

-- 审计日志表（追加写入，不可篡改）
CREATE TABLE audit_logs (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    timestamp TEXT NOT NULL,
    operation TEXT NOT NULL,
    details TEXT NOT NULL,
    hash TEXT NOT NULL  -- 前一行的 hash + 当前行内容的 SHA-256
);
```

---

## 五、管道设计

### 5.1 数据流管道

```
原始信号采集 → 数据清洗 → 特征工程 → 小波分析 → Frame 标注
                                                        ↓
                                        Trit-Core 运算（TAND/TOR/仲裁）
                                                        ↓
                                        环境冲击检测 → 角色边界检测 → 元监控
                                                        ↓
                                        告警生成 → 注意力图谱更新 → 审计日志
```

### 5.2 管道阶段详解

| 阶段 | 输入 | 输出 | 延迟要求 | 错误处理 |
|------|------|------|----------|----------|
| 数据采集 | 外部数据源 | RawSignal[] | 实时 | 跳过失败源，记录错误 |
| 数据清洗 | RawSignal[] | CleanSignal[] | < 100ms | 丢弃无效数据，标记异常 |
| 特征工程 | CleanSignal[] | FeatureVector[] | < 500ms | 降级到简单统计特征 |
| 小波分析 | FeatureVector[] | WaveletResult | < 1s | 降级到频谱分析 |
| Frame 标注 | WaveletResult | TritWord[] | < 100ms | 使用默认 Frame |
| Trit-Core | TritWord[] | TritDecision | < 10ms | SafeFallback 触发 |
| 冲击检测 | TritDecision + Environment | ShockAlert | < 100ms | 保守估计（高冲击） |
| 角色检测 | TritDecision + RoleState | RoleAlert | < 100ms | 保守估计（高污染） |
| 元监控 | 所有中间结果 | MetaLog | 追加写入 | 不可失败（只读） |
| 告警生成 | 所有检测结果 | Alert[] | < 100ms | 批量生成，降级显示 |
| 可视化 | Alert[] + TritDecision[] | UI 更新 | < 200ms | 异步更新 |

---

## 六、测试策略

### 6.1 测试金字塔

```
        /\
       /  \      E2E 测试（5%）— 完整用户场景
      /    \     — 用真实数据验证端到端管道
     /------\    
    /        \   集成测试（15%）— 模块间交互
   /          \  — 数据采集+小波+Trit-Core 的联合测试
  /------------\ 
 /              \ 单元测试（80%）— 单个模块
/                \ — 每个函数、每个边界条件、每个错误路径
```

### 6.2 测试类型

| 类型 | 目标 | 工具 | 覆盖率要求 |
|------|------|------|-----------|
| 单元测试 | 函数正确性 | Rust built-in test | > 80% |
| 集成测试 | 模块交互 | Rust test + SQLite | > 60% |
| 属性测试 | 代数不变量 | proptest | 核心代数 100% |
| 性能测试 | 延迟/吞吐量 | Criterion | 基准 + 回归 |
| 安全测试 | 漏洞检测 | cargo-audit + fuzz | 无已知高危 |
| E2E 测试 | 用户场景 | 自定义脚本 | 核心场景覆盖 |

### 6.3 关键测试场景

- 合成数据：已知节律（正弦波 + 噪声），验证小波检测准确
- 迁移场景：模拟环境切换，验证冲击检测和恢复曲线
- 冲突场景：模拟跨域冲突，验证 Trit-Core 输出 Hold
- 角色场景：模拟角色入侵，验证污染检测和预警
- 极端场景：数据缺失、格式错误、恶意输入、资源耗尽

---

## 七、部署架构

### 7.1 本地部署（个人版）

```
用户设备（macOS/Windows/Linux）
  ├── Aurora Desktop（Tauri）
  │   ├── Rust 后端（aurora-core）
  │   │   ├── SQLite（本地数据库）
  │   │   ├── Trit-Core（决策协议）
  │   │   └── 小波引擎（分析）
  │   └── Web 前端（UI）
  └── 数据源（本地邮件、日历、文件）
```

### 7.2 团队部署（团队版）

```
本地服务器（可选）
  ├── Aurora Server（Rust）
  │   ├── aurora-core（多用户）
  │   ├── SQLite（多用户数据库）
  │   └── 团队管理模块
  └── 客户端（Tauri Desktop）
      ├── 个人数据（本地）
      └── 团队数据（从服务器同步）
```

### 7.3 企业部署（企业版）

```
企业内网
  ├── Aurora Enterprise Server
  │   ├── aurora-core（多用户、多团队）
  │   ├── PostgreSQL（可选，企业级数据库）
  │   ├── LDAP/SSO 集成
  │   └── 审计日志（集中存储）
  └── 客户端（Tauri Desktop）
      ├── 个人数据（本地加密）
      └── 企业数据（同步）
```

---

## 八、验收标准

### 8.1 功能验收

| 功能 | 验收标准 | 验证方式 |
|------|----------|----------|
| 数据采集 | 至少支持邮件元数据、日历、位置（可选） | 手动测试 + 自动化测试 |
| 小波分析 | 能检测基频、谐波、相位漂移 | 合成数据验证 |
| 跨域冲突 | 跨 Frame 运算输出 Hold + MetaInterrupt | 场景测试 |
| 注意力图谱 | 可视化当前注意力分布 | 手动测试 |
| 环境冲击 | 检测环境切换并分级 | 模拟测试 |
| 角色边界 | 检测角色入侵并预警 | 模拟测试 |

### 8.2 非功能验收

| 指标 | 目标 | 验证方式 |
|------|------|----------|
| 离线可用 | 100% 功能可用 | 断网测试 |
| 内存 | < 500MB | 压力测试 |
| 启动 | < 3秒 | 手动计时 |
| 小波延迟 | < 1秒（日数据） | Criterion |
| 安全 | 无已知高危漏洞 | cargo-audit |

---

*本工程规格为 Aurora 的系统设计文档。所有模块接口、数据模型、管道设计在此定义。后续实现需严格遵循。不是指教，是提醒。*
