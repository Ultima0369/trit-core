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
  "policy_action": "Preserve(False, phase=0.200, Individual)"
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
cargo run --release --bin trit-sandbox -- --scenario scenarios/physical_crane_overload.json
```

### 6. 运行测试

```bash
cargo test --all-features -- --test-threads=2
```

预期：全部测试通过（300+，持续增加）。

### 7. 运行基准测试（可选）

```bash
cargo bench
```

核心运算、热路径、级联、Phase 量化、端到端 pipeline、JSON serde 等 Criterion 基准测试组。

---

## 下一步

- [WHAT_IS_TRIT](WHAT_IS_TRIT.md) — 理解三值决策的核心思想
- [CLI_REFERENCE](../how-to/CLI_REFERENCE.md) — CLI 工具的完整参考
- [CONCEPTS](../explanation/CONCEPTS.md) — 所有核心类型的定义
