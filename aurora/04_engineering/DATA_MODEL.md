# Aurora 数据模型

**版本**: 0.1.0
**日期**: 2026-06-20
**状态**: 活跃
**分类**: 04_engineering — 工程规格

---

## 一、实体关系图

```
User ||--o{ RawSignal : generates
User ||--o{ WaveletFeature : produces
User ||--o{ TritDecision : makes
User ||--o{ Alert : receives
User ||--o{ AuditLog : owns
User ||--|| FrameModel : has
User ||--|| AlertSettings : configures
RawSignal }o--|| DataSource : comes_from
WaveletFeature }o--|| Frame : annotated_with
TritDecision }o--|| Domain : in
TritDecision }o--|| MetaInterrupt : contains
Alert }o--|| AlertType : is
```

---

## 二、核心实体

### 2.1 User

```rust
pub struct User {
    pub id: Uuid,                           // 主键
    pub created_at: DateTime<Utc>,         // 创建时间
    pub updated_at: DateTime<Utc>,         // 更新时间
    pub frame_model: FrameModel,           // 参考系模型（JSON）
    pub alert_settings: AlertSettings,     // 告警设置（JSON）
    pub encryption_key_hash: String,       // 加密密钥哈希（不存储密钥本身）
}
```

### 2.2 RawSignal

```rust
pub struct RawSignal {
    pub id: Uuid,                           // 主键
    pub user_id: Uuid,                     // 外键 → User
    pub timestamp: DateTime<Utc>,          // 信号时间
    pub source_type: SourceType,           // 信号来源类型
    pub metadata: HashMap<String, String>, // 元数据（JSON 序列化）
    pub content_hash: Option<String>,      // 内容哈希（不存储内容）
    pub created_at: DateTime<Utc>,         // 入库时间
}
```

### 2.3 WaveletFeature

```rust
pub struct WaveletFeature {
    pub id: Uuid,                           // 主键
    pub user_id: Uuid,                     // 外键 → User
    pub timestamp: DateTime<Utc>,          // 特征时间
    pub feature_type: FeatureType,         // 特征类型
    pub value: f64,                         // 特征值
    pub confidence: f64,                   // 信噪比
    pub phase: f64,                        // 归一化 Phase [0,1]
    pub frame: String,                     // Frame 名称
    pub created_at: DateTime<Utc>,         // 入库时间
}
```

### 2.4 TritDecision

```rust
pub struct TritDecision {
    pub id: Uuid,                           // 主键
    pub user_id: Uuid,                     // 外键 → User
    pub timestamp: DateTime<Utc>,          // 决策时间
    pub domain: String,                    // Domain 名称
    pub result: String,                   // TritValue (True/Hold/False/Unknown)
    pub phase: f64,                        // 结果 Phase
    pub interrupts: String,               // MetaInterrupt[] (JSON)
    pub audit_log: String,                // 审计日志文本
    pub created_at: DateTime<Utc>,         // 入库时间
}
```

### 2.5 Alert

```rust
pub struct Alert {
    pub id: Uuid,                           // 主键
    pub user_id: Uuid,                     // 外键 → User
    pub timestamp: DateTime<Utc>,          // 告警时间
    pub alert_type: String,               // 告警类型
    pub severity: String,                  // 严重程度 (Low/Medium/High/Critical)
    pub message: String,                   // 告警消息
    pub meta_interrupt: Option<String>,  // 关联的 MetaInterrupt (JSON)
    pub resolved: bool,                   // 是否已解决
    pub resolved_at: Option<DateTime<Utc>>, // 解决时间
    pub created_at: DateTime<Utc>,         // 入库时间
}
```

### 2.6 AuditLog

```rust
pub struct AuditLog {
    pub id: i64,                            // 自增主键
    pub timestamp: DateTime<Utc>,          // 日志时间
    pub operation: String,                 // 操作类型
    pub details: String,                   // 详情
    pub prev_hash: String,                // 前一行哈希
    pub hash: String,                      // 当前行哈希
}
```

---

## 三、SQLite Schema

```sql
-- 用户表
CREATE TABLE users (
    id BLOB PRIMARY KEY,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    frame_model TEXT NOT NULL,
    alert_settings TEXT NOT NULL,
    encryption_key_hash TEXT
);

-- 原始信号表
CREATE TABLE raw_signals (
    id BLOB PRIMARY KEY,
    user_id BLOB NOT NULL REFERENCES users(id),
    timestamp TEXT NOT NULL,
    source_type TEXT NOT NULL,
    metadata TEXT NOT NULL,
    content_hash TEXT,
    created_at TEXT NOT NULL
);
CREATE INDEX idx_signals_user_time ON raw_signals(user_id, timestamp);
CREATE INDEX idx_signals_source ON raw_signals(user_id, source_type);

-- 小波特征表
CREATE TABLE wavelet_features (
    id BLOB PRIMARY KEY,
    user_id BLOB NOT NULL REFERENCES users(id),
    timestamp TEXT NOT NULL,
    feature_type TEXT NOT NULL,
    value REAL NOT NULL,
    confidence REAL NOT NULL,
    phase REAL NOT NULL,
    frame TEXT NOT NULL,
    created_at TEXT NOT NULL
);
CREATE INDEX idx_features_user_time ON wavelet_features(user_id, timestamp);
CREATE INDEX idx_features_type ON wavelet_features(user_id, feature_type);

-- Trit 决策表
CREATE TABLE trit_decisions (
    id BLOB PRIMARY KEY,
    user_id BLOB NOT NULL REFERENCES users(id),
    timestamp TEXT NOT NULL,
    domain TEXT NOT NULL,
    result TEXT NOT NULL,
    phase REAL NOT NULL,
    interrupts TEXT NOT NULL,
    audit_log TEXT NOT NULL,
    created_at TEXT NOT NULL
);
CREATE INDEX idx_decisions_user_time ON trit_decisions(user_id, timestamp);
CREATE INDEX idx_decisions_domain ON trit_decisions(user_id, domain);

-- 告警表
CREATE TABLE alerts (
    id BLOB PRIMARY KEY,
    user_id BLOB NOT NULL REFERENCES users(id),
    timestamp TEXT NOT NULL,
    alert_type TEXT NOT NULL,
    severity TEXT NOT NULL,
    message TEXT NOT NULL,
    meta_interrupt TEXT,
    resolved INTEGER NOT NULL DEFAULT 0,
    resolved_at TEXT,
    created_at TEXT NOT NULL
);
CREATE INDEX idx_alerts_user_resolved ON alerts(user_id, resolved);
CREATE INDEX idx_alerts_severity ON alerts(user_id, severity);

-- 审计日志表（追加写入，不可篡改）
CREATE TABLE audit_logs (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    timestamp TEXT NOT NULL,
    operation TEXT NOT NULL,
    details TEXT NOT NULL,
    prev_hash TEXT NOT NULL,
    hash TEXT NOT NULL
);

-- 元数据表（版本、配置）
CREATE TABLE metadata (
    key TEXT PRIMARY KEY,
    value TEXT NOT NULL,
    updated_at TEXT NOT NULL
);
```

---

## 四、数据迁移策略

### 4.1 版本控制

- 使用 `metadata` 表存储数据库版本
- 每次 schema 变更增加版本号
- 启动时检查版本，自动执行迁移脚本

### 4.2 迁移示例

```rust
pub fn migrate(conn: &Connection) -> Result<(), MigrationError> {
    let current_version = get_db_version(conn)?;
    match current_version {
        0 => migrate_v0_to_v1(conn)?,
        1 => migrate_v1_to_v2(conn)?,
        _ => {},
    }
    Ok(())
}
```

### 4.3 备份策略

- 自动备份：每次启动时创建 `.backup` 文件
- 手动导出：用户可随时导出 SQLite 文件
- 恢复：替换 `.db` 文件即可

---

*本文档为 Aurora 的数据模型定义。完整 SQLite Schema 在此，后续变更需经过迁移流程。不是指教，是提醒。*
