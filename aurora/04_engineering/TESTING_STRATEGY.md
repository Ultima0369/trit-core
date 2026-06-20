# Aurora 测试策略

**版本**: 0.1.0
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

**示例**:
```rust
#[test]
fn should_reject_nan_phase() {
    assert!(Phase::new(f64::NAN).is_err());
}

#[test]
fn should_normalize_phase_out_of_range() {
    let p = Phase::new_clamped(1.5);
    assert_eq!(p.value(), 1.0);
}
```

### 2.2 集成测试

| 目标 | 工具 | 覆盖率要求 |
|------|------|-----------|
| 模块间交互 | Rust test + SQLite | > 60% |
| 数据流完整性 | 自定义脚本 | 核心管道 100% |

**示例**:
```rust
#[test]
fn should_detect_cross_frame_conflict() {
    let a = TritWord::tru(Frame::Science, Phase(0.8));
    let b = TritWord::fals(Frame::Individual, Phase(0.2));
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

**示例**:
```rust
proptest! {
    #[test]
    fn tand_should_never_force_true_on_conflict(p1 in 0.0..1.0, p2 in 0.0..1.0) {
        let a = TritWord::tru(Frame::Science, Phase(p1));
        let b = TritWord::fals(Frame::Individual, Phase(p2));
        let (result, _) = TernaryAlgebra::t_and(&a, &b);
        prop_assert_ne!(result.value(), TritValue::True);
    }
}
```

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

---

## 三、CI/CD 测试流程

```
提交代码
  ├── cargo fmt --check
  ├── cargo clippy --all-targets --all-features
  ├── cargo test --lib（单元测试）
  ├── cargo test --integration（集成测试）
  ├── cargo test --proptest（属性测试）
  ├── cargo bench（性能基准）
  ├── cargo audit（安全审计）
  └── cargo build --release（构建验证）
```

---

## 四、测试数据

### 4.1 合成数据

- 正弦波 + 噪声：验证基频检测
- 多频叠加：验证谐波检测
- 频率漂移：验证相位漂移检测
- 频率跳变：验证频谱重构检测

### 4.2 模拟数据

- 邮件通信模拟：随机生成通信频率
- 生理信号模拟：模拟 HRV、睡眠数据
- 环境切换模拟：模拟地理位置变化

### 4.3 真实数据（匿名化）

- 作者自身数据（已脱敏）
- 志愿者数据（已授权、已脱敏）

---

## 五、测试环境

| 环境 | 用途 | 配置 |
|------|------|------|
| 开发 | 本地开发测试 | 开发者机器 |
| CI | 自动化测试 | GitHub Actions / 自建 CI |
|  staging | 预发布测试 | 与生产一致 |
| 生产 | 用户实际使用 | 用户设备 |

---

*本文档为 Aurora 的测试策略。完整测试用例见源码 tests/ 目录。不是指教，是提醒。*
