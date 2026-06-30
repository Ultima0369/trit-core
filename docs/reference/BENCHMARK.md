# BENCHMARK — 性能基准

Trit-Core 使用 Criterion 进行统计性能基准测试。

> **版本说明**：本文档已针对 v0.2.0 重构后的 bench 结构进行更新。v0.1.x 中基于 TCP 帧往返和 `ResonanceBus` 的基准已随网络层移除而删除。下表中的延迟数字为 **v0.2.0 实测**（2026-06-18，Windows 开发工作站，release build）。

## 运行基准测试

```bash
cargo bench
```

首次运行会花费较长时间（Criterion 需要多次采样以达到统计显著性）。后续运行会与历史基线对比。

## 基准测试内容

基准测试文件：`benches/trit_bench.rs`，包含 **16 个基准函数**，分为 **6 个 Criterion 组**。

### 微基准组（4 组）

| 组 | 基准测试 | 测量内容 |
|---|---|---|
| `core_ops` | `tand_same_frame` | 同帧 TAND（热路径，无 MetaInterrupt） |
| | `tand_cross_frame` | 跨帧 TAND（冷路径，含 MetaInterrupt 分配） |
| | `tor_same_frame` | 同帧 TOR |
| | `tnot` | TNOT 操作 |
| `hot_path` | `tand_hot_path` | `t_and_hot` 热路径（跳过帧检查） |
| | `precheck_same_frame` | 帧一致性预检查 |
| `cascades` | `tand_cascade_10` | 10 个 TritWord 的 TAND 级联（跨帧混合） |
| | `tand_cascade_10_hot` | 10 个同帧 TritWord 热路径级联 |
| `phase_precision` | `phase_quantize_near_*` | Phase 量化精度（近中性/近零/近一/无需量化） |

### 端到端组（2 组）

| 组 | 基准测试 | 测量内容 |
|---|---|---|
| `pipeline` | `full_pipeline_medical_ethics` | MedicalEthics 完整管道（JSON 解析 → TAND 级联 → 仲裁 → SafeFallback → JSON 输出） |
| | `full_pipeline_physical` | Physical 完整管道（同上，不同域） |
| `json_serde` | `json_deser_scenario` | ScenarioInput 反序列化（2 信号） |
| | `json_ser_output` | SandboxOutput 序列化 |

## 当前性能数据

测量环境：标准开发工作站，release build（`opt-level=3`，`lto="thin"`，`codegen-units=16`）。

> 与 v0.1.x（`lto=true`、`codegen-units=1`）相比，thin LTO 下的绝对延迟有所上升，但所有层级仍远高于 10,000 TPS 目标。

### 微基准

| 操作 | 延迟 | 说明 |
|---|---|---|
| TAND 同帧 | ~7.3 ns | 热路径，分支无关 LUT + 一次浮点运算 |
| TAND 跨帧 | ~104 ns | 冷路径，含 MetaInterrupt 分配 + 时间戳 |
| TOR 同帧 | ~6.5 ns | 热路径 |
| TNOT | ~5.8 ns | 单次 LUT 查找 + 浮点减法 |
| Phase 量化（近中性） | ~0.8 ns | 吸附到锚点（0.5） |
| Phase 量化（近零） | ~0.9 ns | 吸附到锚点（0.0） |
| Phase 量化（近一 / 无需量化） | ~1.4 ns | 吸附到锚点（1.0）或无需量化 |
| 10 元素级联（跨帧混合） | ~1.0 μs | 每对触发冷路径 |
| 10 元素级联（热路径） | ~35 ns | 全部同帧 |

### 端到端

| 管道 | 延迟 | 吞吐量 |
|---|---|---|
| MedicalEthics 管道（2 信号） | ~3.32 μs | **~602K signals/s**（按约 7 逻辑 op/pipe ≈ **~2.1M ops/s**） |
| Physical 管道（2 信号） | ~3.59 μs | **~558K signals/s**（按约 7 逻辑 op/pipe ≈ **~1.95M ops/s**） |
| JSON 反序列化（2 信号） | ~830 ns | ~1.2M deser/s |
| JSON 序列化（output） | ~326 ns | ~3.1M ser/s |

## 性能目标

目标吞吐量：**10,000 TPS**（每秒 TritWord 操作）。

**状态：✅ 已验证。** 端到端管道在 v0.2.0 仍达到数百万 ops/s 量级，即使在最保守的 signals/s 口径下也远超 10,000 TPS 目标。

## 性能注意事项

### 热路径优化

- `TritValue` 的 `negate()` 和 `to_i8()` 使用编译时 LUT，LLVM 优化为单次寄存器加载
- `FrameMask` 使用 u8 位掩码 + `popcount` 指令
- `Phase::mean` 内联到调用点

### 冷路径成本

- `MetaInterrupt::with_frames()` 使用预分配 String（48 字节容量），避免 `format!()` 的多次分配
- `chrono::Utc::now()` 是冷路径的主要成本来源（系统调用）

### 已知瓶颈

- JSON 序列化/反序列化（`serde_json`）占端到端延迟的 25-49%，是最大的可优化单项开销
- 当前 v0.1.x 性能已 65-101 倍于目标，进一步优化不是 MVP 必需项

## HTML 报告

```bash
cargo bench -- --output-format html
```

报告生成在 `target/criterion/` 目录。打开 `target/criterion/report/index.html` 查看可视化对比。
