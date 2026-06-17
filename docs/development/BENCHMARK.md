# BENCHMARK — 性能基准

Trit-Core 使用 Criterion 进行统计性能基准测试。

## 运行基准测试

```bash
cargo bench
```

首次运行会花费较长时间（Criterion 需要多次采样以达到统计显著性）。后续运行会与历史基线对比。

## 基准测试内容

基准测试文件：`benches/trit_bench.rs`

| 基准 | 测量内容 |
|---|---|
| `tand_hot` | 热路径 TAND（同帧，无 MetaInterrupt 分配） |
| `tand_cold` | 冷路径 TAND（跨帧，含 MetaInterrupt 分配） |
| `tor_hot` | 热路径 TOR |
| `tor_cold` | 冷路径 TOR |
| `tnot` | TNOT 操作 |
| `phase_mean` | Phase 均值计算 |
| `phase_complement` | Phase 互补计算 |
| `arbitrate_physical` | Physical 域仲裁 |
| `arbitrate_medical` | MedicalEthics 域仲裁 |
| `safe_fallback_guard` | SafeFallback 守卫检查 |
| `cascade_10` | 10 个 TritWord 的 TAND 级联 |
| `cascade_100` | 100 个 TritWord 的 TAND 级联 |

## 当前性能数据

测量环境：标准开发机器，release build（`opt-level=3`，`lto=true`）。

| 操作 | 延迟 | 说明 |
|---|---|---|
| TAND 热路径 | ~3 ns | 同帧，分支无关 LUT + 一次浮点运算 |
| TAND 冷路径 | ~95 ns | 跨帧，含 String 分配 + 时间戳 |
| TOR 热路径 | ~3 ns | 同帧 |
| TOR 冷路径 | ~95 ns | 跨帧 |
| TNOT | ~2 ns | 单次 LUT 查找 + 浮点减法 |
| Phase::mean | ~2 ns | 一次加法 + 一次除法 + 量化 |
| Phase::complement | ~2 ns | 一次减法 + 量化 |
| arbitrate (Physical) | ~15 ns | FrameMask O(1) 查找 |
| SafeFallback::guard | ~10 ns | 域检查 + 值检查（无分配时） |
| 10 元素级联 | ~30 ns | 全部热路径 |
| 100 元素级联 | ~300 ns | 全部热路径 |

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
