# REVIEWER GUIDE — Trit-Core 评审者指引

感谢您抽出时间审阅 Trit-Core 项目。本文档旨在帮助您快速理解项目并验证核心声明。

---

## 这是什么？

Trit-Core 是一个**三元决策引擎**，专为冲突感知的 AI 对齐而设计。与传统二值逻辑（真/假）不同，它引入了第三种状态 **Hold（悬置）**——当检测到不可通约的价值冲突时，故意暂停判断。

**核心假设**：在使用人类反馈的强化学习（RLHF）中，直接基于群体偏好平均来对齐，会删除少数立场；而保留冲突的三元协议比二值系统产生更真实的对齐结果。

---

## 如何运行

### 环境要求
- Rust 1.70+（`rustup` 安装）
- 无其他依赖

### 快速开始
```bash
git clone https://github.com/trit-core/trit-core.git
cd trit-core

# 构建（release 模式含 LTO）
cargo build --release

# 运行所有测试（305 个）
cargo test --all-features

# 运行一个示例场景
cargo run --release --bin trit-sandbox -- --scenario scenarios/medical_conflict_01.json

# 运行基准测试
cargo bench
```

### 关键场景

| 场景 | 命令 | 预期行为 |
|------|------|---------|
| 医疗伦理冲突 | `--scenario scenarios/medical_conflict_01.json` | Hold（保留患者自主权） |
| 桥梁安全 | `--scenario scenarios/bridge_safety_01.json` | commit_false（科学优先） |
| 职业价值冲突 | `--scenario scenarios/career_value_conflict.json` | Hold（不可通约） |

---

## 需要验证的核心声明

### 声明 1：Trit-Core 在跨域冲突时正确悬置

**如何验证**：运行 `medical_conflict_01.json`（化疗 vs 姑息治疗：Science 说化疗有效，Individual 说患者拒绝）。检查输出中 `final_value` 为 `"Hold"` 且 `interrupts` 非空。

**对比**：二元多数投票（`baseline/` 模块）在此场景中会强制返回 True 或 False，丢失冲突信息。

### 声明 2：仲裁策略因域而异

- **Physical/Engineering**：Science 优先，允许强制坍缩
- **MedicalEthics**：Individual 优先，禁止强制坍缩
- **ValueJudgment**：始终 Hold（不可通约）

运行对应的场景 JSON，验证 `policy_action` 字段。

### 声明 3：端到端性能超过 10,000 TPS

**已验证**：完整管道（JSON I/O + 三值运算 + 仲裁 + 安全回退）在 MedicalEthics 场景中达到 ~658,000 TPS，在 Physical 场景中达到 ~1,015,000 TPS。见 `docs/performance-validation.md`。

---

## 文档导航

| 你想了解... | 阅读... |
|-----------|-------|
| 项目是什么、为什么重要 | `docs/getting-started/WHAT_IS_TRIT.md` |
| 3 分钟试用 | `docs/getting-started/QUICKSTART.md` |
| 哲学基础 | `docs/concepts/PHILOSOPHY.md` |
| 技术架构 | `docs/concepts/ARCHITECTURE.md` |
| 核心概念定义 | `docs/concepts/CONCEPTS.md` |
| API 参考 | `docs/api.md` |
| 所有场景列表 | `docs/usage/CLI_REFERENCE.md` |
| 验证数据（vs 二值基线） | `docs/validation-report.md` |
| 性能数据 | `docs/performance-validation.md` |
| 堆分配分析 | `docs/performance-validation.md` §7（dhat 验证） |
| 已知局限 | `docs/insights/FUTURE.md` |
| 冲突模式目录 | `docs/insights/CONFLICT_CATALOG.md` |
| 完整术语表 | `docs/insights/GLOSSARY.md` |
| 分布式协议 | `docs/concepts/ARCHITECTURE.md` §7（M4–M8） |
| 安全审计 | `docs/security-audit.md` |
| 安全策略 | `SECURITY.md` |

---

## 安全说明

- `#![forbid(unsafe_code)]` — 整个项目中无 unsafe Rust
- SafeFallback：对危险域（Physical、Engineering）实施 IEC 61508 故障安全语义
- 安全审计报告：`docs/security-audit.md`

---

## 反馈

请将反馈发送至项目仓库的 Issues 页面，或直接联系 Trit-Core 团队。

我们特别希望听到关于以下方面的意见：
1. 仲裁逻辑在您专业领域中的合理性
2. Hold 状态作为一个决策输出的实际效用
3. 文档和技术写作的可读性
4. 形式化验证建议（Coq/Lean/TLA+）
