# Trit-Core Roadmap

**Version**: 0.3.0  
**Status**: Active  
**Last Updated**: 2026-06-19

---

## 已完成的里程碑

### M0: 基础核心（v0.1.0-alpha）
- [x] 三值代数（TAND、TOR、TNOT）与相位算术
- [x] 5 个决策域：Physical、Engineering、MedicalEthics、ValueJudgment、General
- [x] 元监控与基于域的仲裁
- [x] 沙盒 CLI（`trit-sandbox`）

### M1: 网络层原型（v0.1.0，已移除）
- [x] TCP 传输、节点发现、分区容忍、Byzantine gatekeeper（v0.1.0 完成）
- [x] 在 v0.2.0 中移除，以便聚焦核心正确性；未来可能作为独立 crate 恢复

### M2: v0.2.0 架构重构
- [x] `src/core/` 集中 `TritValue`、`Phase`、`Frame`、`TritWord`、`TernaryAlgebra`
- [x] `TritWord` 字段私有，不变量由构造函数保证
- [x] `Phase::new` 返回 `Result`，新增 `Phase::new_clamped`
- [x] `ResolutionPolicy::arbitrate` 返回 `Result`
- [x] `sandbox/` 层：输入验证、管道执行、期望行为校验
- [x] 自动化场景验证：`tests/sandbox_test.rs` 覆盖所有 `scenarios/*.json`
- [x] 移除 `src/net/`、`trit-node`、`tokio`、`uuid`
- [x] 性能文档实测与历史文档归档

### M3: v0.3.0 可观测性与决策完整性
- [x] 结构化日志：`tracing_init` 支持文件日志、JSON / pretty / compact / full 格式、`TRIT_LOG_FILE` / `TRIT_LOG_FORMAT`
- [x] `t_and_n` 批处理 TAND，避免 3+ 信号级联偏差
- [x] `SandboxDiagnostics` 运行时诊断：阶段耗时、帧分布、中断计数、SafeFallback 追踪
- [x] CLI 增强：`--verbose`、`--quiet`、`--trace`、`--log-file`、`--log-format`、`--diagnostic`、`--validate-only`、`--dry-run`
- [x] `SandboxError` 分类与可操作帮助文本（`ErrorCategory`、`help()`、`report()`）
- [x] `CustomRule.fallback` 改为类型安全的 `FallbackBehavior` 枚举
- [x] `Frame` / `TritWord` 实现 `Copy`

---

## 进行中 / 近期（v0.3.1 – v0.3.x）

### 测试与质量
- [ ] 引入覆盖率报告（`cargo-tarpaulin` 或 `llvm-cov`）到 CI
- [ ] 增加模糊测试（`cargo-fuzz`）覆盖 JSON 输入和 `TritWord` 构造
- [ ] 跨平台性能基准：在 Linux CI 上采集 Criterion 数据

### 工程成熟度
- [ ] 引入 `cargo-deny` 进行依赖许可证/安全审计
- [ ] 引入 `cargo-machete` 检测未使用依赖
- [ ] 发布到 crates.io

### 文档
- [x] 刷新 `docs/reports/validation-report.md` 到 0.3.0 场景集合
- [ ] 重新测量并更新 `docs/reports/performance-validation.md` 与 `docs/reference/BENCHMARK.md`
- [ ] 添加更多使用示例和 cookbook

---

## 中期（v0.4.0）

- [ ] 自定义规则 DSL：更丰富的 JSON 规则表达（优先级、阈值、多帧组合）
- [ ] 异步/流式场景处理（仍保持零网络依赖）
- [ ] 与其他 Rust 生态的集成示例（如 axum/actix-web 中间件模式）
- [ ] 决策日志的持久化与回放

---

## 长期（v0.5.0+）

- [ ] 分布式节点协议作为独立 crate 重新设计（吸取 v0.1.x 经验）
- [ ] 形式化验证核心代数（如 Kani / Miri 深入检查）
- [ ] `no_std` 核心子集，支持嵌入式场景
- [ ] 与 LLM/RLHF 系统的对比评估框架

---

## 设计原则（贯穿所有版本）

1. **安全优先**：危险域必须能强制进入安全状态（`SafeFallback`）
2. **类型驱动不变量**：非法状态应无法构造
3. **零 unsafe**：`#![forbid(unsafe_code)]`
4. **可解释性**：每个决策都产生可审计的冲突日志
5. **可观测性**：每个阶段都可被日志、诊断和错误上下文追踪
6. **可测试性**：每个公共行为都有自动化测试覆盖

---

## 归档

- [v0.1.0 路线图](../archive/roadmap-v0.1.0.md)
