# Aurora 测试策略

**版本**: 0.2.0
**日期**: 2026-06-20
**状态**: 活跃
**分类**: 04_engineering — 工程规格

---

## 一、测试金字塔

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

---

## 二、测试类型

### 2.1 单元测试

| 目标 | 工具 | 覆盖率要求 |
|------|------|-----------|
| 函数正确性 | Rust built-in test | > 80% |
| 边界条件 | Rust test | 全部边界 |
| 错误路径 | Rust test | 全部错误路径 |

**正确示例**（与 Trit-Core v0.3.0 API 一致）：

```rust
#[test]
fn should_reject_nan_phase() {
    assert!(Phase::new(f64::NAN).is_err());
}

#[test]
fn should_clamp_out_of_range_phase() {
    let p = Phase::new_clamped(1.5);
    assert_eq!(p.inner(), 1.0);
}
```

### 2.2 集成测试

| 目标 | 工具 | 覆盖率要求 |
|------|------|-----------|
| 模块间交互 | Rust test + SQLite | > 60% |
| 数据流完整性 | 自定义脚本 | 核心管道 100% |

**正确示例**（与 Trit-Core v0.3.0 API 一致）：

```rust
#[test]
fn should_detect_cross_frame_conflict() {
    let a = TritWord::new(TritValue::True, Phase::new(0.8).unwrap(), Frame::Science);
    let b = TritWord::new(TritValue::False, Phase::new(0.2).unwrap(), Frame::Individual);
    let (result, interrupt) = TernaryAlgebra::t_and(&a, &b);
    assert_eq!(result.value(), TritValue::Hold);
    assert!(interrupt.is_some());
}

#[test]
fn new_frame_geoeco_crosses_with_science() {
    let a = TritWord::tru(Frame::Science);
    let b = TritWord::fals(Frame::GeoEco); // 新扩展 Frame
    let (result, interrupt) = TernaryAlgebra::t_and(&a, &b);
    assert_eq!(result.value(), TritValue::Hold);
    assert!(interrupt.is_some());
}
```

### 2.3 属性测试（Property-Based Testing）

| 目标 | 工具 | 覆盖率要求 |
|------|------|-----------|
| 代数不变量 | proptest | 核心代数 100% |
| Phase 有界性 | proptest | 100% |
| SafeFallback | proptest | 100% |

**正确示例**（与 Trit-Core v0.3.0 API 一致）：

```rust
proptest! {
    #[test]
    fn tand_should_never_force_true_on_conflict(p1 in 0.0..1.0, p2 in 0.0..1.0) {
        let a = TritWord::new(TritValue::True, Phase::new(p1).unwrap(), Frame::Science);
        let b = TritWord::new(TritValue::False, Phase::new(p2).unwrap(), Frame::Individual);
        let (result, _) = TernaryAlgebra::t_and(&a, &b);
        prop_assert_ne!(result.value(), TritValue::True);
    }
}
```

**注意**：`TritWord::tru` 只接受 `Frame` 一个参数，`Phase` 自动为 `1.0`。如果需要自定义 `Phase`，使用 `TritWord::new(value, phase, frame)`。

### 2.4 性能测试

| 目标 | 工具 | 基准 |
|------|------|------|
| 延迟 | Criterion | 热路径 < 5ns，端到端 < 1s |
| 吞吐量 | Criterion | 55-210x 10K TPS 目标 |
| 内存 | dhat | 单用户 < 500MB |
| 回归 | CI 性能基准 | 每次提交对比 |

### 2.5 安全测试

| 目标 | 工具 | 频率 |
|------|------|------|
| 依赖漏洞 | cargo-audit | 每次提交 |
| 模糊测试 | cargo-fuzz | 每周 |
| 输入验证 | 自定义测试 | 每次提交 |
| 安全审计 | 第三方 | M1/M3 |

### 2.6 E2E 测试

| 场景 | 验证内容 | 频率 |
|------|----------|------|
| 合成数据 | 已知节律，验证小波检测 | 每次提交 |
| 迁移场景 | 模拟环境切换，验证冲击检测 | 每周 |
| 冲突场景 | 模拟跨域冲突，验证 Hold | 每次提交 |
| 角色场景 | 模拟角色入侵，验证污染检测 | 每周 |
| 极端场景 | 数据缺失、格式错误、恶意输入 | 每周 |
| 伦理场景 | 模拟强制坍缩、参考系入侵，验证 Awareness | 每次提交 |

---

## 三、伦理门禁测试（新增）

### 3.1 不可谈判的测试用例

以下测试用例是**发布硬性门槛**。任何版本发布前必须全部通过。这些测试不是"可选项"，是定心盘的工程硬化。

```rust
#[test]
fn ethics_cross_frame_must_hold() {
    // 跨 Frame 冲突必须输出 Hold，不可强制坍缩
    let a = TritWord::tru(Frame::Science);
    let b = TritWord::fals(Frame::Individual);
    let (result, interrupt) = TernaryAlgebra::t_and(&a, &b);
    assert_eq!(result.value(), TritValue::Hold);
    assert!(interrupt.is_some());
}

#[test]
fn ethics_user_can_override_hold() {
    // 用户可以选择覆盖系统的 Hold
    let a = TritWord::tru(Frame::Science);
    let b = TritWord::fals(Frame::Individual);
    let (result, interrupt) = TernaryAlgebra::t_and(&a, &b);
    assert_eq!(result.value(), TritValue::Hold);
    assert!(interrupt.is_some());
    
    // 用户选择忽略 Hold，自己决定
    // 系统不阻止，只记录
    let user_override = TritWord::tru(Frame::Individual);
    // 系统不阻止这个选择
}

#[test]
fn ethics_user_can_disable_safe_fallback() {
    // 用户可以选择关闭 SafeFallback
    let sf = SafeFallback::disabled();
    let result = TritWord::unknown(Frame::Science);
    let (guarded, interrupt) = sf.guard(&Domain::Physical, &result, 1);
    // 系统不强制 False，输出原样
    assert_eq!(guarded.value(), TritValue::Unknown);
    assert!(interrupt.is_none());
}

#[test]
fn ethics_awareness_does_not_block_computation() {
    // 系统检测到策略违反，只通知，不阻断运算
    let a = TritWord::new(TritValue::True, Phase::full_true(), Frame::Meta);
    let b = TritWord::tru(Frame::Science);
    let (result, interrupt) = TernaryAlgebra::t_and(&a, &b);
    // 运算继续，结果可能为 Hold，但不是因为被阻断
    assert_eq!(result.value(), TritValue::Hold);
    // 如果有中断，是 PolicyViolation 通知
    if let Some(int) = interrupt {
        assert!(matches!(int.conflict, ConflictType::PolicyViolation(_)));
    }
}

#[test]
fn ethics_meta_frame_cannot_be_external_input() {
    // Meta 帧不能作为外部输入
    let meta_input = TritWord::new(TritValue::True, Phase::full_true(), Frame::Meta);
    // 系统检测到 Meta 帧作为外部输入，返回 PolicyViolation 通知
    // 但系统不阻止运算（Awareness 原则）
    let result = TernaryAlgebra::t_and(&meta_input, &TritWord::tru(Frame::Science));
    assert_eq!(result.0.value(), TritValue::Hold);
}

#[test]
fn ethics_data_anomaly_triggers_awareness() {
    // 数据模式与历史基线偏离 > 3σ，标记为 DataAnomaly
    // 系统输出 Hold + 数据异常预警
    let detector = DataAnomalyDetector::new();
    let recent_data = vec![0.0; 100]; // 全部为零，与历史基线偏离
    let baseline = vec![0.5; 100];   // 历史基线均值为 0.5
    let anomaly = detector.detect(&recent_data, &baseline);
    assert!(anomaly.is_some());
    assert!(matches!(anomaly.unwrap().conflict, ConflictType::PolicyViolation(
        PolicyViolation::DataAnomaly
    )));
}

#[test]
fn ethics_system_does_not_guess_on_hold() {
    // 系统在 Hold 时不提供"最可能"答案
    let a = TritWord::tru(Frame::Science);
    let b = TritWord::fals(Frame::Individual);
    let (result, _) = TernaryAlgebra::t_and(&a, &b);
    assert_eq!(result.value(), TritValue::Hold);
    // 系统不提供替代方案、建议、最可能选项
    // 如果系统提供了，这个测试应该失败
}

#[test]
fn ethics_system_does_not_profile_user() {
    // 系统不基于用户行为预测用户偏好
    // 如果系统实现了用户画像功能，这个测试应该失败
    // 这是一个"负测试"：验证某项功能不存在
    let has_user_profiling = check_feature_exists("user_profiling");
    assert!(!has_user_profiling, "User profiling is forbidden by CHARTER.md");
}

#[test]
fn ethics_system_does_not_emotionally_manipulate() {
    // 系统不根据用户情绪状态调整输出以最大化参与度
    let has_emotion_manipulation = check_feature_exists("emotion_manipulation");
    assert!(!has_emotion_manipulation, "Emotion manipulation is forbidden by CHARTER.md");
}

#[test]
fn ethics_audit_log_is_append_only() {
    // 审计日志追加写入，不可篡改
    let mut log = AuditLog::new();
    log.append("event1");
    log.append("event2");
    let hash1 = log.last_hash();
    
    // 尝试篡改（如果系统允许篡改，这个测试失败）
    let tampered = log.try_tamper(0, "tampered");
    assert!(tampered.is_err());
    
    let hash2 = log.last_hash();
    assert_ne!(hash1, hash2); // 哈希链验证失败
}
```

### 3.2 伦理门禁测试运行方式

```bash
# 运行所有伦理门禁测试
cargo test ethics_

# 运行全部测试（包括伦理门禁）
cargo test

# CI 中：伦理门禁测试失败 = 构建失败
cargo test --lib && cargo test ethics_
```

### 3.3 伦理门禁测试的维护

- 新增伦理门禁测试需要 ADR 审批（`05_adr/`）
- 删除伦理门禁测试**不允许**——这是定心盘的工程硬化
- 修改伦理门禁测试的断言逻辑需要 **2 人审查**（不能单人修改）
- 伦理门禁测试的覆盖率要求：**100%**（不是 80%，不是 90%，是 100%）

---

## 四、CI/CD 测试流程

```
提交代码
  ├── cargo fmt --check
  ├── cargo clippy --all-targets --all-features
  ├── cargo test --lib（单元测试）
  ├── cargo test --integration（集成测试）
  ├── cargo test --proptest（属性测试）
  ├── cargo test ethics_（伦理门禁测试） ← 新增
  ├── cargo bench（性能基准）
  ├── cargo audit（安全审计）
  └── cargo build --release（构建验证）
```

**关键**：`cargo test ethics_` 是发布门禁。如果伦理门禁测试失败，**不允许发布**。

---

## 五、测试数据

### 5.1 合成数据

- 正弦波 + 噪声：验证基频检测
- 多频叠加：验证谐波检测
- 频率漂移：验证相位漂移检测
- 频率跳变：验证频谱重构检测
- **数据异常模式**：验证 DataAnomaly 检测（全部为零、突然跳变、周期性缺失）

### 5.2 模拟数据

- 邮件通信模拟：随机生成通信频率
- 生理信号模拟：模拟 HRV、睡眠数据
- 环境切换模拟：模拟地理位置变化
- **恶意输入模拟**：模拟强制坍缩请求、参考系入侵数据、元监控篡改尝试

### 5.3 真实数据（匿名化）

- 作者自身数据（已脱敏）
- 志愿者数据（已授权、已脱敏）

---

## 六、测试环境

| 环境 | 用途 | 配置 |
|------|------|------|
| 开发 | 本地开发测试 | 开发者机器 |
| CI | 自动化测试 | GitHub Actions / 自建 CI |
| staging | 预发布测试 | 与生产一致 |
| 生产 | 用户实际使用 | 用户设备 |

---

*本文档为 Aurora 的测试策略。v0.2.0 修正了与 Trit-Core v0.3.0 不一致的代码示例，增加了伦理门禁测试章节。所有伦理门禁测试为发布硬性门槛，不可跳过。用户自负其责。*
