# CLI REFERENCE — trit-sandbox 命令行参考

## 命令

```bash
cargo run --release --bin trit-sandbox -- --scenario <PATH>
```

## 参数

| 参数 | 必需 | 说明 |
|---|---|---|
| `--scenario <PATH>` | 是 | JSON 场景文件路径 |

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
| `domain` | string | 仲裁域。可选值：`Physical`、`Engineering`、`MedicalEthics`、`ValueJudgment`、`General`，或自定义名称（如 `chemistry`） |
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
| `frame` | string | 决策域。可选值：`Science`、`Individual`、`Consensus`、`Absolute`、`Meta` |
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
  "policy_action": "Preserve(Individual: False)"
}
```

| 字段 | 类型 | 说明 |
|---|---|---|
| `scenario_id` | string | 对应输入的 id |
| `final_value` | string | 最终值的人类可读表示：`True`、`Hold`、`False`、`Unknown` |
| `final_value_code` | i8 | 数值表示：`1`=True，`0`=Hold/Unknown，`-1`=False |
| `final_frame` | string | 最终 TritWord 的帧 |
| `final_phase` | f64 | 最终相位值 |
| `interrupts` | string[] | 冲突记录列表。格式：`"ConflictType: 原因"`。为空表示无跨域冲突 |
| `policy_action` | string | 仲裁结果的可读表示，如 `Commit(Science: False)`、`Preserve(Individual: True)`、`Hold`、`Negotiate` |

## 可用场景

`scenarios/` 目录包含 17 个预置场景：

| 文件 | 域 | 说明 |
|---|---|---|
| `medical_conflict_01.json` | MedicalEthics | 患者拒绝标准化疗 |
| `medical_autonomy.json` | MedicalEthics | 终末期患者请求实验性治疗 |
| `medical_mandate.json` | MedicalEthics | 疫苗强制令：科学 vs 个人 |
| `bridge_safety.json` | Engineering | 桥梁结构疲劳 vs 社区反对封桥 |
| `crane_overload.json` | Physical | 起重机超载：操作员 vs 传感器 |
| `runway_safety.json` | Physical | 跑道长度不足 vs 航班压力 |
| `career_value_conflict.json` | ValueJudgment | 职业选择：数据 vs 内心 |
| `material_tradeoff.json` | Engineering | 材料选择：共识 vs 科学 vs 个人 |
| `bridge_retrofit.json` | Engineering | 桥梁加固：共识 vs 科学 |
| `general_negotiation.json` | General | 同帧信号，协商成功 |
| `general_multi_domain.json` | General | 跨域信号，触发协商 |
| 其他 | 混合 | 更多场景覆盖 |

## 退出码

| 码 | 含义 |
|---|---|
| 0 | 成功 |
| 1 | 场景文件未找到或 JSON 解析失败 |

## 日志

设置环境变量控制日志行为：

```bash
# 人类可读格式（默认）
TRIT_LOG=info cargo run --bin trit-sandbox -- --scenario scenarios/medical_conflict_01.json

# JSON 格式（可机器解析）
TRIT_LOG_JSON=1 cargo run --bin trit-sandbox -- --scenario scenarios/medical_conflict_01.json
```

详见 [CONFIGURATION](CONFIGURATION.md)。
