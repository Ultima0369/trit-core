# 告警引擎模块规格

**版本**: 0.1.0
**日期**: 2026-06-20
**状态**: 活跃
**分类**: 07_specs — 详细模块规格

---

## 一、模块职责

根据检测结果生成告警，控制告警频率和打扰程度，管理告警生命周期。

## 二、接口定义

```rust
pub struct AlertEngine {
    pub settings: AlertSettings,
    pub alert_history: Vec<Alert>,
}

impl AlertEngine {
    pub fn process(&mut self, detections: Vec<Detection>) -> Vec<Alert>;
    pub fn should_alert(&self, alert: &Alert) -> bool;
    pub fn suppress(&mut self, alert_type: AlertType, duration: Duration);
    pub fn resolve(&mut self, alert_id: Uuid);
}

pub struct AlertSettings {
    pub do_not_disturb: bool,
    pub quiet_hours: (Time, Time),  // 夜间不打扰
    pub max_alerts_per_hour: usize,
    pub min_severity: Severity,     // 只显示 >= 该严重级别的告警
    pub suppressed_types: Vec<AlertType>,
}

pub enum AlertType {
    EnvironmentalShock,
    RoleContamination,
    CrossFrameConflict,
    PhaseDrift,
    RecoveryProgress,
    SystemStatus,
}

pub enum Severity {
    Low,
    Medium,
    High,
    Critical,
}
```

## 三、告警策略

| 告警类型 | 触发条件 | 默认严重级别 | 默认频率 |
|----------|----------|-------------|----------|
| 环境冲击 | ΔΦ > 0.2 | 中震=Medium, 强震=High | 每次冲击一次 |
| 角色污染 | contamination_ratio > 0.7 | Medium | 每天最多一次 |
| 跨域冲突 | 跨 Frame Hold 产生 | Low | 每小时最多一次 |
| 相位漂移 | 漂移速度 > 阈值 | Low | 每天最多一次 |
| 恢复进度 | 恢复完成 50%/100% | Low | 每个里程碑一次 |
| 系统状态 | 数据源断开/内存不足 | Medium | 每次一次 |

## 四、防打扰机制

- 静默时段：用户可设置"不要打扰"时段
- 频率限制：同一类型告警每小时最多一次
- 批量处理：短时间内多个告警合并为摘要
- 严重级别过滤：用户可只接收 >= Medium 的告警

## 五、生命周期

```
生成 → 评估（是否提醒）→ 展示（UI/通知）→ 用户查看 → 解决/忽略 → 归档
```

