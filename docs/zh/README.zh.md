# Trit-Core 三值决策引擎

**版本**：0.1.0-alpha  
**协议**：MIT License

---

## 项目概述

Trit-Core 是一个**三值决策引擎**，用于处理人类中心场景中的认知冲突与伦理对齐。与当前基于二值逻辑（真/假）或概率平滑的大语言模型对齐方案不同，Trit-Core 引入了一个独立的**悬置态（Hold）**，代表"不判定、不强制、保留开放"。

**核心目标**：
> 在医疗伦理、价值判断、情感困境等人类咨询场景中，证明"三值协议（允许悬置）比二值概率输出更能匹配用户的真实满意与人格主权"。

---

## 架构总览

```
输入层（多源信号）
    ├── 科学域（Science）—— 实证数据、临床试验
    ├── 个体域（Individual）—— 用户情境、过敏史、信仰
    ├── 共识域（Consensus）—— 统计均值、群体偏好
    └── 绝对域（Absolute）—— 不可知、不可观测
         │
         ▼
    三值算术逻辑单元（HTA）
         ├── 谐波与（TAND）/ 谐波或（TOR）/ 谐波非（TNOT）
         ├── 相位算术（0.0 ~ 1.0）
         └── 跨域冲突检测
         │
         ▼
    元监控策略引擎（Meta-Monitor）
         ├── 冲突中断（MetaInterrupt）
         ├── 域规则（Domain Rules）
         └── 仲裁决议（Arbitration）
         │
         ▼
    输出层
         ├── 确定态：真（+1）/ 假（-1）
         ├── 悬置态：未判定（0）+ 原因
         └── 决策日志（JSONL，可审计）
```

---

## 项目结构

| 路径 | 说明 |
|------|------|
| `src/trit/` | 三值代数核心：TritValue、Phase、TAND/TOR/TNOT |
| `src/frame/` | 参考系注册：科学、个体、共识、绝对、元 |
| `src/meta/` | 元监控与策略引擎：域规则、仲裁、冲突日志 |
| `src/clock/` | 谐波时钟（MVP 占位） |
| `src/sandbox/` | 沙盒数据结构 |
| `src/bin/sandbox.rs` | 命令行工具（CLI） |
| `docs/zh/` | **中文技术文档**（ADR、白皮书、路线图、API） |
| `scenarios/` | 测试场景（JSON） |
| `tests/` | 单元测试与集成测试 |

---

## 技术栈

- **语言**：Rust 2021 Edition
- **序列化**：serde + serde_json（决策日志）
- **错误处理**：thiserror
- **时间戳**：chrono + uuid

---

## 构建与运行

```bash
cargo build --release
cargo test
cargo run --bin trit-sandbox -- --scenario scenarios/medical_conflict_01.json
```

---

## 双语文档索引

| 文档 | 英文版 | 中文版 |
|------|--------|--------|
| 项目说明 | `README.md` | `docs/zh/README.zh.md`（本文） |
| 架构决策记录 | `docs/adr/` | `docs/zh/adr/` |
| 技术白皮书 | `docs/whitepaper.md` | `docs/zh/whitepaper.zh.md` |
| 路线图与验收 | `docs/roadmap.md` | `docs/zh/roadmap.zh.md` |
| API 契约 | `docs/api.md` | `docs/zh/api.zh.md` |

---

## 核心概念速查（中文对照）

| 中文概念 | 代码对应 | 说明 |
|---------|---------|------|
| 三态 | `TritValue` | 真（+1）、悬置（0）、假（-1） |
| 相位 | `Phase` | 倾向度 0.0~1.0，0.5 为中立 |
| 参考系 | `Frame` | 科学、个体、共识、绝对、元 |
| 元监控 | `MetaMonitor` | 跨域冲突检测与策略仲裁 |
| 悬置 | `Hold` | 不强制输出，保持开放端口 |
| 坍缩 | `Commit` / `ForceCollapse` | 被迫或主动给出确定结论 |
| 谐波运算 | `TAND` / `TOR` | 同域叠加，异域触发冲突 |

---

## 许可

MIT License
