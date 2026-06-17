# BENCHMARK — 性能基准

Trit-Core 使用 Criterion 进行统计性能基准测试。

## 运行基准测试

```bash
cargo bench
```

首次运行会花费较长时间（Criterion 需要多次采样以达到统计显著性）。后续运行会与历史基线对比。

## 基准测试内容

基准测试文件：`benches/trit_bench.rs`，包含 29 个基准，分为 9 个 criterion 组。

### 微基准组（5 组）

| 组 | 基准测试 | 测量内容 |
|---|---|---|
| `core_ops` | `tand_same_frame` | 同帧 TAND（热路径，无 MetaInterrupt） |
| | `tand_cross_frame` | 跨帧 TAND（冷路径，含 MetaInterrupt 分配） |
| | `tor_same_frame` | 同帧 TOR |
| | `tor_cross_frame` | 跨帧 TOR |
| | `tnot` | TNOT 操作 |
| `hot_path` | `tand_hot_path` | `t_and_hot` 热路径（跳过帧检查） |
| | `tor_hot_path` | `t_or_hot` 热路径 |
| | `precheck_same_frame` | 帧一致性预检查 |
| `cascades` | `tand_cascade_10` | 10 个 TritWord 的 TAND 级联（跨帧混合） |
| | `tand_cascade_10_hot` | 10 个同帧 TritWord 热路径级联 |
| | `tand_cascade_100_hot` | 100 个同帧 TritWord 热路径级联 |
| `cross_domain` | `cross_domain_tand_100pairs` | 100 对跨域 TAND（Science↔Individual↔Consensus 轮转） |
| | `hot_path_same_frame_pair` | 热路径 vs 冷路径对比：同帧对 |
| | `cold_path_cross_frame_pair` | 热路径 vs 冷路径对比：跨帧对 |
| `phase_precision` | `phase_quantize_near_*` | Phase 量化精度（近中性/近零/近一/无需量化） |

### 端到端组（4 组）

| 组 | 基准测试 | 测量内容 |
|---|---|---|
| `pipeline` | `full_pipeline_medical_ethics` | MedicalEthics 完整管道（JSON 解析 → TAND 级联 → 仲裁 → SafeFallback → JSON 输出） |
| | `full_pipeline_physical` | Physical 完整管道（同上，不同域） |
| `tcp_roundtrip` | `tcp_frame_roundtrip` | TCP 帧完整往返（序列化 → 写帧 → 读帧 → 反序列化） |
| | `tcp_frame_serialize_only` | 仅序列化（Message → JSON + 帧头） |
| | `tcp_frame_deserialize_only` | 仅反序列化（JSON → Message） |
| `concurrent_bus` | `concurrent_bus_register_100_nodes` | 注册 100 个节点到 ResonanceBus |
| | `concurrent_bus_10_resonates` | 10 并发 resonate 操作 |
| `json_serde` | `json_deser_scenario` | ScenarioInput 反序列化（2 信号） |
| | `json_ser_output` | SandboxOutput 序列化 |
| | `json_message_roundtrip` | Message 往返（resonate_req） |
| | `json_message_negotiate_10` | Message 往返（negotiate，10 节点） |

## 当前性能数据

测量环境：标准开发机器，release build（`opt-level=3`，`lto=true`）。

### 微基准

| 操作 | 延迟 | 说明 |
|---|---|---|
| TAND 热路径 | ~1.5 ns | 同帧，分支无关 LUT + 一次浮点运算 |
| TAND 冷路径 | ~95 ns | 跨帧，含 String 分配 + 时间戳 |
| TOR 热路径 | ~1.5 ns | 同帧 |
| TOR 冷路径 | ~95 ns | 跨帧 |
| TNOT | ~1.2 ns | 单次 LUT 查找 + 浮点减法 |
| Phase 量化 | ~1.0 ns | 吸附到锚点（0.0、0.5、1.0） |
| 10 元素级联（热路径） | ~14 ns | 全部同帧 |
| 100 元素级联（热路径） | ~130 ns | 全部同帧 |
| 100 对跨域级联 | ~9.5 μs | 每对触发冷路径 |

### 端到端

| 管道 | 延迟 | 吞吐量 |
|---|---|---|
| MedicalEthics 管道（2 信号） | ~1.52 μs | **657,895 ops/s** |
| Physical 管道（2 信号） | ~985 ns | **1,015,228 ops/s** |
| TCP 帧往返 | ~1.48 μs | ~676,000 msg/s |
| 注册 100 个节点 | ~11.5 μs | ~8,696,000 注册/s |
| 10 并发 resonate | ~1.62 μs | ~617,000 resonate/s |

## 性能目标

目标吞吐量：**10,000 TPS**（每秒 TritWord 操作）。

**状态：✅ 已验证。** 端到端 MedicalEthics 管道达到 ~658K TPS（65.8× 目标），Physical 管道达到 ~1.02M TPS（101.5× 目标）。详见 `docs/performance-validation.md`。

## 性能注意事项

### 热路径优化

- TritValue 的 `negate()` 和 `to_i8()` 使用编译时 LUT，LLVM 优化为单次寄存器加载
- `FrameMask` 使用 u8 位掩码 + `popcount` 指令
- `Phase::mean` 内联到调用点

### 冷路径成本

- `MetaInterrupt::with_frames()` 使用预分配 String（48 字节容量），避免 `format!()` 的多次分配
- `chrono::Utc::now()` 是冷路径的主要成本来源（系统调用）

### 已知瓶颈

- JSON 序列化/反序列化（`serde_json`）占端到端延迟的 25-49%，是最大的可优化单项开销
- 消息日志的 VecDeque 在达到 `MAX_MESSAGE_LOG` 后会触发 pop_front（O(1) 摊销，但涉及内存移动）
- 当前性能已 65-101 倍于目标，进一步优化不是 MVP 必需项

## HTML 报告

```bash
cargo bench -- --output-format html
```

报告生成在 `target/criterion/` 目录。打开 `target/criterion/report/index.html` 查看可视化对比。
