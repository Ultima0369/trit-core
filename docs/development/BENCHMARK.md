# BENCHMARK — 性能基准

Trit-Core 使用 Criterion 进行统计性能基准测试。

## 运行基准测试

```bash
cargo bench
```

首次运行会花费较长时间（Criterion 需要多次采样以达到统计显著性）。后续运行会与历史基线对比。

## 基准测试内容

基准测试文件：`benches/trit_bench.rs`，包含 5 个 criterion 组。

| 基准 | 测量内容 |
|---|---|
| `tand_same_frame` | 同帧 TAND（热路径，无 MetaInterrupt） |
| `tand_cross_frame` | 跨帧 TAND（冷路径，含 MetaInterrupt 分配） |
| `tor_same_frame` | 同帧 TOR |
| `tor_cross_frame` | 跨帧 TOR |
| `tnot` | TNOT 操作 |
| `tand_hot_path` | `t_and_hot` 热路径（跳过帧检查） |
| `tor_hot_path` | `t_or_hot` 热路径 |
| `precheck_same_frame` | 帧一致性预检查 |
| `tand_cascade_10` | 10 个 TritWord 的 TAND 级联（跨帧混合） |
| `tand_cascade_10_hot` | 10 个同帧 TritWord 热路径级联 |
| `tand_cascade_100_hot` | 100 个同帧 TritWord 热路径级联 |
| `cross_domain_tand_100pairs` | 100 对跨域 TAND（Science↔Individual↔Consensus 轮转） |
| `hot_path_same_frame_pair` | 热路径 vs 冷路径对比：同帧对 |
| `cold_path_cross_frame_pair` | 热路径 vs 冷路径对比：跨帧对 |
| `phase_quantize_near_*` | Phase 量化精度（近中性/近零/近一/无需量化） |

Criterion 基准组：`core_ops`, `hot_path`, `cascades`, `cross_domain`, `phase_precision`。

## 当前性能数据

测量环境：标准开发机器，release build（`opt-level=3`，`lto=true`）。

| 操作 | 延迟 | 说明 |
|---|---|---|
| TAND 热路径 | ~3 ns | 同帧，分支无关 LUT + 一次浮点运算 |
| TAND 冷路径 | ~95 ns | 跨帧，含 String 分配 + 时间戳 |
| TOR 热路径 | ~3 ns | 同帧 |
| TOR 冷路径 | ~95 ns | 跨帧 |
| TNOT | ~2 ns | 单次 LUT 查找 + 浮点减法 |
| Phase 量化 | ~2 ns | 吸附到锚点（0.0、0.5、1.0） |
| 10 元素级联（热路径） | ~30 ns | 全部同帧 |
| 100 元素级联（热路径） | ~300 ns | 全部同帧 |
| 100 对跨域级联 | ~9.5 μs | 每对触发冷路径 |

## 性能目标

目标吞吐量：**10,000 TPS**（每秒 TritWord 操作）。

以当前热路径 ~3ns/op 计算，单核理论吞吐量为 ~333M ops/s，远超目标。瓶颈在冷路径（跨帧操作）和序列化/反序列化（JSON I/O）。

## 性能注意事项

### 热路径优化

- TritValue 的 `negate()` 和 `to_i8()` 使用编译时 LUT，LLVM 优化为单次寄存器加载
- `FrameMask` 使用 u8 位掩码 + `popcount` 指令
- `Phase::mean` 内联到调用点

### 冷路径成本

- `MetaInterrupt::with_frames()` 使用预分配 String（48 字节容量），避免 `format!()` 的多次分配
- `chrono::Utc::now()` 是冷路径的主要成本来源（系统调用）

### 已知瓶颈

- JSON 序列化/反序列化（`serde_json`）占端到端延迟的 90% 以上
- 消息日志的 VecDeque 在达到 `MAX_MESSAGE_LOG` 后会触发 pop_front（O(1) 摊销，但涉及内存移动）

## HTML 报告

```bash
cargo bench -- --output-format html
```

报告生成在 `target/criterion/` 目录。打开 `target/criterion/report/index.html` 查看可视化对比。
