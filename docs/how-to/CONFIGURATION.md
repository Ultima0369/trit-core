# CONFIGURATION — 配置与日志

**Version**: 0.3.0

Trit-Core 通过环境变量和命令行选项控制日志行为。没有独立的配置文件——所有配置通过环境变量或 CLI flags 完成。

## 环境变量

| 变量 | 值 | 默认 | 说明 |
|---|---|---|---|
| `TRIT_LOG` | `trace`, `debug`, `info`, `warn`, `error` | `info` | 日志级别 |

> **注意**：日志通过 `TRIT_LOG` 环境变量控制，无独立 CLI 标志。`tracing_init::init()` 在程序启动时读取 `TRIT_LOG`，回退到 `RUST_LOG`。

## 日志级别

| 级别 | 输出内容 |
|---|---|
| `trace` | 所有内部操作（TAND/TOR 入口、Phase 计算细节、SafeFallback 检查） |
| `debug` | 仲裁步骤、帧检测结果、场景校验通过 |
| `info` | 策略创建、仲裁完成、SafeFallback 触发、管道完成（默认） |
| `warn` | 跨帧冲突、Phase 钳制、NaN/Inf 检测 |
| `error` | 不可恢复的错误 |

## 使用示例

### 开发调试

```bash
TRIT_LOG=trace cargo run --bin trit-sandbox -- --scenario scenarios/medical_conflict_01.json
```

### 生产环境（JSON 格式）

默认即为 JSON，可直接被日志聚合器摄取：

```bash
TRIT_LOG=info cargo run --release --bin trit-sandbox -- --scenario scenarios/medical_conflict_01.json
```

示例输出：

```json
{"timestamp":"2026-06-19T06:28:07.413355Z","level":"INFO","fields":{"message":"pipeline started","scenario_id":"bridge_safety"},"target":"trit_core::sandbox::pipeline","filename":"src\\sandbox\\pipeline.rs","line_number":74,"span":{"domain":"Engineering","scenario_id":"bridge_safety","signal_count":2,"name":"run_with_diagnostics"},"spans":[{"domain":"Engineering","scenario_id":"bridge_safety","signal_count":2,"name":"run_with_diagnostics"}],"threadId":"ThreadId(1)"}
```

### 人类可读格式

```bash
TRIT_LOG=info cargo run --release --bin trit-sandbox -- --scenario scenarios/medical_conflict_01.json
```

tracing subscriber 默认使用人类可读格式。使用 `TRIT_LOG=debug` 可获取更多内部细节。

### 写入日志文件

使用 shell 重定向：

```bash
cargo run --release --bin trit-sandbox -- \
  --scenario scenarios/bridge_safety.json \
  > trit-sandbox.log 2>&1
```

文件以追加模式打开；若文件不存在则自动创建。

### 静默模式

```bash
cargo run --release --bin trit-sandbox -- --scenario scenarios/medical_conflict_01.json --quiet
```

或：

```bash
TRIT_LOG=error cargo run --release --bin trit-sandbox -- --scenario scenarios/medical_conflict_01.json
```

## 格式说明

| 格式 | 用途 |
|---|---|
| `json` | 结构化、机器可解析（默认） |
| `pretty` | 多行人类可读，带 ANSI 高亮 |
| `compact` | 单行人类可读，字段紧凑 |
| `full` | 单行人类可读，字段完整 |

## 内部机制

Trit-Core 使用 `tracing` 框架。`tracing-subscriber` 在 `src/tracing_init.rs` 中初始化，在 `src/bin/sandbox.rs` 和 `src/bin/dhat_profile.rs` 的 `main()` 函数中调用。

初始化逻辑：
1. 读取 `TRIT_LOG` 环境变量（默认 `info`），未设置时回退到 `RUST_LOG`
2. 读取 `TRIT_LOG_FORMAT`（默认 `json`）
3. 若设置了 `TRIT_LOG_FILE`，日志同时写入该文件
4. 应用 `EnvFilter` 进行级别过滤
5. 0.3.0 默认启用 span close 事件，便于性能诊断
