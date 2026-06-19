# CLI REFERENCE — trit-sandbox 命令行参考

**Version**: 0.3.0

## 命令

```bash
cargo run --release --bin trit-sandbox -- --scenario <PATH> [OPTIONS]
```

## 参数

| 参数 | 必需 | 说明 |
|---|---|---|
| `--scenario <PATH>` | 是 | JSON 场景文件路径 |

## 选项

| 选项 | 说明 |
|---|---|
| `-v`, `--verbose` | 启用 debug 级别日志 |
| `-q`, `--quiet` | 仅输出 warn 及以上级别日志 |
| `--trace` | 启用 trace 级别日志（最详细） |
| `--log-file <PATH>` | 将日志写入指定文件（替代 stderr） |
| `--log-format <FMT>` | 日志格式：`json`（默认）、`pretty`、`compact`、`full` |
| `--diagnostic` | 在标准错误输出 JSON 诊断报告 |
| `--validate-only` | 校验场景文件后退出，不运行决策管道 |
| `--dry-run` | 构建 trits 并运行 TAND，但跳过仲裁与 SafeFallback；此模式下不校验 `expected_behavior` |
| `--reflexive` | 启用自反审计 guard：当存在未解决的跨帧冲突时，强制 True/False 结果会被覆写为 Hold |
| `--self-knowledge` | 启用自我知识模型，在输出中附加 receiver_estimate |
| `--trace-phase` | 记录最终相位到诊断报告的 `phase_trace` 字段 |
| `--hold-final` | 将 Hold 视为最终答案（默认行为相同；用于显式表达不自动质疑的保持） |
| `-h`, `--help` | 打印帮助信息 |

## 使用示例

### 基本用法

```bash
cargo run --release --bin trit-sandbox -- --scenario scenarios/medical_conflict_01.json
```

### 带诊断报告的调试运行

```bash
cargo run --release --bin trit-sandbox -- \
  --scenario scenarios/bridge_safety.json \
  --trace --diagnostic
```

### 仅校验场景

```bash
cargo run --release --bin trit-sandbox -- \
  --scenario scenarios/general_negotiation.json \
  --validate-only
```

### 以 pretty 格式输出到文件

```bash
cargo run --release --bin trit-sandbox -- \
  --scenario scenarios/career_value_conflict.json \
  --log-file trit.log --log-format pretty --verbose
```

## `--help` 输出

```text
trit-sandbox — run a Trit-Core scenario through the decision pipeline

Usage:
  trit-sandbox --scenario <path.json> [OPTIONS]

Required:
  --scenario <path.json>   Path to a scenario JSON file under the scenarios/ directory

Logging options:
  -v, --verbose            Enable debug-level logging
  -q, --quiet              Only log warnings and errors
      --trace              Enable trace-level logging (most verbose)
      --log-file <path>    Write logs to a file instead of stderr
      --log-format <fmt>   One of: json (default), pretty, compact, full

Execution options:
      --diagnostic         Emit a diagnostic report alongside the output
      --validate-only      Validate the scenario and exit without running the pipeline
      --dry-run            Build trits and run TAND, but skip arbitration and SafeFallback
      --reflexive          Enable reflexive audit guard
      --self-knowledge     Enable self-knowledge receiver estimation
      --trace-phase        Record final phase in diagnostics
      --hold-final         Treat Hold as a final answer
  -h, --help               Print this help message

Environment:
  TRIT_LOG                 Log filter (e.g., debug, info, warn)
  TRIT_LOG_FILE            Path to write logs to a file
  TRIT_LOG_FORMAT          json | pretty | compact | full
  TRIT_LOG_JSON            0/false to disable JSON logging
```

## 场景 JSON 格式

### 顶层结构

```json
{
  "id": "唯一标识符",
  "description": "人类可读的场景描述",
  "domain": "域名称",
  "signals": [...],
  "expected_behavior": "hold|commit_true|commit_false|negotiate"
}
```

### 字段说明

| 字段 | 类型 | 说明 |
|---|---|---|
| `id` | string | 场景唯一 ID，用于输出追踪 |
| `description` | string | 自由文本描述 |
| `domain` | string | 仲裁域。可选值：`Physical`、`Engineering`、`MedicalEthics`、`ValueJudgment`、`General`，或 `Custom(name)`（如 `Custom(chemistry)`） |
| `signals` | array | SignalInput 对象数组（至少 1 个） |
| `expected_behavior` | string | 预期行为。可选值：`hold`、`commit_true`、`commit_false`、`negotiate`。用于测试断言 |

### SignalInput 结构

```json
{
  "frame": "Science",
  "value": 1,
  "phase": 0.8
}
```

| 字段 | 类型 | 说明 |
|---|---|---|
| `frame` | string | 决策域。可选值：`Science`、`Individual`、`Consensus`、`Absolute`、`FirstPerson`、`Embodied`、`Relational` |
| `value` | i8 | 三值状态：`1`=True，`0`=Hold，`-1`=False |
| `phase` | f64 | 倾向度 `0.0..1.0`。`0.5`=中性，`>0.5`=倾向 True，`<0.5`=倾向 False |

## 输出 JSON 格式

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
  "policy_action": "Preserve(False, phase=0.200, Individual)",
  "reflexive_alert": null,
  "attention_cmd": null,
  "receiver_estimate": null,
  "hold_state": null
}
```

| 字段 | 类型 | 说明 |
|---|---|---|
| `scenario_id` | string | 对应输入的 id |
| `final_value` | string | 最终值的人类可读表示：`True`、`Hold`、`False`、`Unknown` |
| `final_value_code` | i8 | 数值表示：`1`=True，`0`=Hold/Unknown（需结合 `final_value` 字符串区分），`-1`=False |
| `final_frame` | string | 最终 TritWord 的帧 |
| `final_phase` | f64 | 最终相位值，始终在 `[0.0, 1.0]` 范围内 |
| `interrupts` | string[] | 冲突记录列表。格式：`"ConflictType: 原因"`。为空表示无跨域冲突 |
| `policy_action` | string | 仲裁结果的可读表示，如 `Commit(False, phase=0.400, Science)`、`Preserve(True, phase=0.850, Individual)`、`Hold`、`Negotiate` |
| `reflexive_alert` | string \| null | 自反审计告警（启用 `--reflexive` 且触发时存在） |
| `attention_cmd` | string \| null | 注意力调度建议（启用 attention scheduler 时存在） |
| `receiver_estimate` | object \| null | 自我知识推断的接收者状态（启用 `--self-knowledge` 时存在） |
| `hold_state` | object \| null | Hold 状态说明（最终值为 Hold 时存在） |

## 可用场景

`scenarios/` 目录包含多个预置场景（含中文变体）：

| 文件 | 域 | 说明 |
|---|---|---|
| `medical_conflict_01.json` | MedicalEthics | 患者药物过敏 |
| `medical_conflict_02.json` | MedicalEthics | 终末期患者请求实验性治疗 |
| `medical_conflict_03.json` | MedicalEthics | 疫苗强制令：科学/共识 vs 个体不良反应 |
| `medical_pain_dismissed.json` | MedicalEthics | 慢性疼痛：科学无异常 vs 个体实情 vs 社会共识 |
| `bridge_safety.json` | Engineering | 桥梁结构疲劳 vs 社区反对封桥 |
| `engineering_bridge_retrofit.json` | Engineering | 桥梁加固：共识 vs 科学 |
| `engineering_material_tradeoff.json` | Engineering | 材料选择：共识 vs 科学 vs 个人 |
| `engineering_evacuation_consensus.json` | Engineering | 建筑疏散：科学判断 vs 租户共识 vs 居民报告 |
| `physical_crane_overload.json` | Physical | 起重机超载：操作员 vs 传感器 |
| `physical_runway_length.json` | Physical | 跑道长度不足 vs 航班压力 |
| `career_value_conflict.json` | ValueJudgment | 职业选择：数据 vs 内心 |
| `career_value_conflict_02.json` | ValueJudgment | 职业选择变体 2 |
| `career_value_conflict_03.json` | ValueJudgment | 职业选择变体 3 |
| `general_negotiation.json` | General | 同帧信号，提交首信号 |
| `general_negotiation_02.json` | General | 跨帧预算优先级，触发协商 |
| `general_conceptual_spin.json` | General | 理性协作滑向概念空转，触发协商 |
| `value_algorithmic_displacement.json` | ValueJudgment | 算法替代人类社工：效率 vs 人的尊严 |
| `general_water_rights.json` | General | 干旱流域水资源：科学、水权法、农户生计的跨帧协商 |
| `engineering_dam_breach_risk.json` | Engineering | 水坝溃堤风险：科学安全 vs 旅游经济 vs 居民家园 |
| `first_person_attention.json` | General | 第一人称主观报告 vs 个体自主权的跨帧协商 |
| `mind_reflexive_trigger.json` | MedicalEthics | 专门用于触发 `--reflexive` guard 的强制坍缩场景 |

中文变体：每个场景对应 `.zh.json` 文件（如 `medical_pain_dismissed.zh.json`），与英文版共享相同的域、信号和预期行为。

## 退出码

| 码 | 含义 |
|---|---|
| 0 | 成功 |
| 1 | 场景文件无效、JSON 解析失败、校验失败、`expected_behavior` 不匹配，或日志文件无法打开 |

## 环境变量

日志行为也可以通过环境变量控制，详见 [CONFIGURATION](CONFIGURATION.md)。
