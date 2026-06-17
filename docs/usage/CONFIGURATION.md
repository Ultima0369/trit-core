# CONFIGURATION — 配置与日志

Trit-Core 通过环境变量控制日志行为。没有配置文件——所有配置通过环境变量完成。

## 环境变量

| 变量 | 值 | 默认 | 说明 |
|---|---|---|---|
| `TRIT_LOG` | `trace`, `debug`, `info`, `warn`, `error` | `info` | 日志级别 |
| `TRIT_LOG_JSON` | `1` 或未设置 | 未设置 | 启用 JSON 格式日志输出 |

## 日志级别

```
trace  → 所有内部操作（TAND/TOR 入口、Phase 计算细节）
debug  → 仲裁步骤、帧检测结果
info   → 策略创建、仲裁完成、SafeFallback 触发（默认）
warn   → 跨帧冲突、Phase 钳制、NaN/Inf 检测
error  → 不可恢复的错误
```

## 使用示例

### 开发调试

```bash
TRIT_LOG=trace cargo run --bin trit-sandbox -- --scenario scenarios/medical_conflict_01.json
```

输出示例：
```
2026-06-17T10:30:00.000Z TRACE t_and{op="t_and" a_frame=Science b_frame=Individual}: entering TAND
2026-06-17T10:30:00.001Z WARN t_and{op="t_and"}: cross-frame conflict detected op="TAND" a=Science b=Individual
2026-06-17T10:30:00.001Z INFO arbitrate{domain=MedicalEthics}: arbitration completed result=Preserve(...)
```

### 生产环境（JSON 格式）

```bash
TRIT_LOG=info TRIT_LOG_JSON=1 cargo run --release --bin trit-sandbox -- --scenario scenarios/medical_conflict_01.json
```

输出为 JSON 行，可被日志聚合器（如 Loki、Elasticsearch）直接摄取：

```json
{"timestamp":"2026-06-17T10:30:00.001Z","level":"INFO","fields":{"message":"arbitration completed","result":"Preserve(...)"}}
```

### 静默模式

```bash
TRIT_LOG=error cargo run --release --bin trit-sandbox -- --scenario scenarios/medical_conflict_01.json
```

只输出错误级别的日志。

## 内部机制

Trit-Core 使用 `tracing` 框架。`tracing-subscriber` 在 `src/tracing_init.rs` 中初始化，在 `src/bin/sandbox.rs` 和 `src/bin/node.rs` 的 `main()` 函数中调用。

初始化逻辑：
1. 读取 `TRIT_LOG` 环境变量（默认 `info`）
2. 如果 `TRIT_LOG_JSON=1`，使用 `tracing_subscriber::fmt().json()`
3. 否则使用人类可读的格式
4. 应用 `EnvFilter` 进行级别过滤
