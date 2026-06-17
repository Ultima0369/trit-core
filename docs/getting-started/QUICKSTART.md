# QUICKSTART — 3 分钟上手

从零到运行第一个三值决策场景。

## 前置条件

- Rust 工具链（[rustup.rs](https://rustup.rs)）
- Git

## 步骤

### 1. 克隆并编译

```bash
git clone https://github.com/trit-core/trit-core.git
cd trit-core
cargo build --release
```

编译时间：约 30 秒（首次，含依赖下载）。

### 2. 运行第一个场景

```bash
cargo run --release --bin trit-sandbox -- --scenario scenarios/medical_conflict_01.json
```

### 3. 预期输出

```json
{
  "scenario_id": "medical_conflict_01",
  "final_value": "False",
  "final_value_code": -1,
  "final_frame": "Individual",
  "final_phase": 0.2,
  "interrupts": [
    "FrameMismatch: TAND conflict: Science vs Individual"
  ],
  "policy_action": "Preserve(Individual: False)"
}
```

### 4. 发生了什么？

1. 场景文件包含两个信号：Science 帧（临床指南推荐治疗）和 Individual 帧（患者拒绝）
2. TAND 运算检测到跨帧冲突 → 产生 Hold + MetaInterrupt
3. MedicalEthics 域仲裁 → 优先 Individual 帧（患者自主权）
4. 输出：False（尊重患者拒绝），附带完整的冲突记录

### 5. 尝试其他场景

```bash
# 物理安全场景（桥梁评估）
cargo run --release --bin trit-sandbox -- --scenario scenarios/bridge_safety.json

# 价值判断场景（职业选择）
cargo run --release --bin trit-sandbox -- --scenario scenarios/career_value_conflict.json

# 工程场景（起重机过载）
cargo run --release --bin trit-sandbox -- --scenario scenarios/crane_overload.json
```

### 6. 运行测试

```bash
cargo test --all-features
```

预期：227 个测试全部通过。

### 7. 运行基准测试（可选）

```bash
cargo bench
```

29 个 Criterion 基准测试，分为 9 个组（微基准 5 组 + 端到端 4 组）。端到端性能远超 10,000 TPS 目标。

### 8. 运行分布式节点（可选）

```bash
# 启动一个 Science 帧节点（独立模式）
cargo run --release --bin trit-node -- --frame Science --phase 0.75 --id my-node

# 带种子发现的 3 节点网格
cargo run --release --bin trit-node -- --frame Science --phase 0.75 --id node-a --port 9000 --peers "127.0.0.1:9001,127.0.0.1:9002"
```

---

## 下一步

- [WHAT_IS_TRIT](WHAT_IS_TRIT.md) — 理解三值决策的核心思想
- [CLI_REFERENCE](../usage/CLI_REFERENCE.md) — CLI 工具的完整参考
- [CONCEPTS](../concepts/CONCEPTS.md) — 所有核心类型的定义
