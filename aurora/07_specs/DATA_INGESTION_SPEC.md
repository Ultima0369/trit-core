# 数据采集模块规格

**版本**: 0.1.0
**日期**: 2026-06-20
**状态**: 活跃
**分类**: 07_specs — 详细模块规格

---

## 一、模块职责

负责从外部数据源采集原始信号，提取元数据，存入本地数据库。

## 二、接口定义

```rust
pub trait DataSource {
    fn name(&self) -> &str;
    fn connect(&mut self) -> Result<(), DataError>;
    fn fetch(&self, start: DateTime<Utc>, end: DateTime<Utc>) -> Result<Vec<RawSignal>, DataError>;
    fn disconnect(&mut self);
    fn is_available(&self) -> bool;
}

pub struct RawSignal {
    pub timestamp: DateTime<Utc>,
    pub source_type: SourceType,
    pub metadata: HashMap<String, String>,
    pub content_hash: Option<String>,
}

pub enum SourceType {
    Email,
    Calendar,
    Location,
    Physiological,
    Environmental,
    Social,
    Transaction,
}
```

## 三、数据源实现

| 数据源 | 实现方式 | 可用性 |
|--------|----------|--------|
| Apple Mail | 读取 ~/Library/Mail/V10/ 下的 .emlx 文件 | macOS 可用 |
| Microsoft Outlook | 读取 OST/PST 文件 | Windows 可用 |
| Thunderbird | 读取 SQLite 数据库 | 跨平台 |
| Google Calendar | 本地日历缓存（已授权） | 需授权 |
| Apple Calendar | 读取本地 .ics 缓存 | macOS 可用 |
| Apple Health | 读取 HealthKit 导出 | iOS/macOS 可用 |
| Garmin | 读取 Connect 导出 | 需导出 |
| Oura | API 读取（用户授权） | 需授权 |
| 位置 | 读取系统位置日志 | 需授权 |
| 天气 | 公开 API | 自动 |

## 四、隐私保护

- 只读取元数据（时间、频率、方向），不读取内容
- 内容哈希用于去重，不存储内容
- PII 在采集时脱敏
- 用户可随时断开数据源

## 五、错误处理

| 错误 | 处理 |
|------|------|
| 数据源不可用 | 标记为不可用，跳过 |
| 格式错误 | 记录错误，跳过该条 |
| 权限不足 | 提示用户重新授权 |
| 网络失败 | 重试 3 次，失败后跳过 |

