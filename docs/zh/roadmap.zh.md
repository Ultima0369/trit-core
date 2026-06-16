# Trit-Core MVP 路线图

**版本**：0.1.0  
**状态**：草案  
**更新日期**：2026-06-17

---

## 里程碑

### M0：基础（第 0–1 周）
**目标**：项目骨架 + 核心代数 + 单元测试。

**交付物**：
- [x] `Cargo.toml` 与依赖配置
- [x] `src/lib.rs` 公开 API
- [x] `src/trit/` 模块（TritValue、Phase、TernaryAlgebra）
- [x] `src/frame/` 模块（参考系注册）
- [x] `src/meta/` 模块（MetaMonitor、ResolutionPolicy、5 域）
- [x] 单元测试：TAND、TOR、TNOT 全 9 种同域组合
- [x] 单元测试：异域冲突检测
- [x] 强制 `#![forbid(unsafe_code)]`

**验收标准**：
- `cargo test` 100% 通过。
- 零编译警告（`#[deny(warnings)]`）。
- `trit/` 与 `meta/` 代码覆盖率 > 80%。

---

### M1：沙盒 CLI（第 1–2 周）
**目标**：可运行的命令行工具，消费场景 JSON 并产出决策日志。

**交付物**：
- [ ] `src/bin/sandbox.rs` 命令行解析（`--scenario <path>`）
- [ ] JSON 输入模式验证（ScenarioInput、SignalInput）
- [ ] JSON 输出序列化（SandboxOutput）
- [ ] `src/sandbox/` 模块（流水线引擎）
- [ ] 5 个示例场景 JSON 文件
- [ ] 集成测试：跑通全部场景，断言预期行为

**验收标准**：
- `cargo run --bin trit-sandbox -- --scenario scenarios/example.json` 输出合法 JSON。
- 全部 5 个示例场景端到端通过。
- 跨域场景必须输出非空 `interrupts`。

---

### M2：场景验证套件（第 2–3 周）
**目标**：扩展至 10–20 个人类中心咨询案例，与二值基线对比。

**交付物**：
- [ ] 10–20 个场景 JSON，覆盖：
  - 医疗伦理（3 例）
  - 职业/价值冲突（3 例）
  - 物理安全（2 例）
  - 工程权衡（2 例）
  - 通用协商（2 例）
- [ ] 二值基线比较器（简单多数规则，无悬置态）
- [ ] 对比报告：标注二值基线失效而 Trit-Core 正确悬置的案例
- [ ] `docs/validation-report.md` 总结

**验收标准**：
- 至少 5 个案例证明：二值基线产生"和稀泥"或"越界"输出，而 Trit-Core 正确输出悬置。
- 报告可被非技术利益相关者阅读（每例附白话摘要）。

---

### M3：预印本与开源（第 3–4 周）
**目标**：代码、文档、验证报告打包，公开发布。

**交付物**：
- [ ] GitHub 仓库公开（`main` 分支）
- [ ] MIT LICENSE
- [ ] README.md（含架构图与构建说明）
- [ ] `docs/whitepaper.md` 定稿
- [ ] `docs/adr/` 3 篇 ADR 完成
- [ ] 预印本 Markdown（10–15 页）
- [ ] （可选）crates.io 发布

**验收标准**：
- `cargo build --release` 在稳定 Rust（1.70+）成功。
- 预印本包含：摘要、问题陈述、架构、验证结果、局限、参考。
- 至少一位外部人类审阅者阅读预印本并提供反馈。

---

### M4：分布式原型（MVP 后，可选）
**目标**：多节点谐波耦合（本地/网络）。

**交付物**：
- [ ] `src/net/` 模块（Node、Resonate、Decouple）
- [ ] 锁相环（PLL）模拟
- [ ] `trit-node` 可执行文件
- [ ] Docker Compose 三节点本地集群

**验收标准**：
- 三节点不同域耦合，输出协商后的悬置态。
- 节点解耦不导致全局共识崩溃。

---

## 风险登记

| 风险 | 可能性 | 影响 | 缓解 |
|------|--------|------|------|
| Rust 学习曲线延迟 M1 | 中 | 低 | 结对编程；MVP 接受"够用即可"的代码质量 |
| 场景设计主观化 | 高 | 中 | 使用匿名真实事件；引入二值基线对比 |
| 无学术审阅者 | 中 | 高 | 投稿 arXiv 与 Hacker News；寻求社区反馈 |
| 性能开销过高 | 低 | 中 | 尽早基准测试；若 >5× 则接受，M4 后优化 |

---

## MVP 完成定义

- [ ] 代码编译通过，测试通过，无 unsafe 块。
- [ ] 10–20 个场景含二值对比。
- [ ] 白皮书 + ADR + 预印本完整。
- [ ] GitHub 公开仓库上线。
- [ ] 至少一位人类审阅者确认："Trit-Core 在保留冲突方面优于二值 RLHF 代理。"
