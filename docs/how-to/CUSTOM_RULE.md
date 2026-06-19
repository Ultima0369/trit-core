# CUSTOM RULE — 自定义仲裁域

Trit-Core 支持通过 JSON 文件定义自定义域的仲裁规则。这使你可以为特定领域（如基因编辑、核安全、金融风控）定义专属的仲裁逻辑，而不需要修改核心代码。

## 1. RuleLoader 特质

```rust
pub trait RuleLoader {
    type Error: std::fmt::Display;

    fn load<P: AsRef<Path>>(path: P) -> Result<CustomRule, Self::Error>;
    fn load_json(json: &str) -> Result<CustomRule, Self::Error>;
    fn apply(rule: &CustomRule, inputs: &[TritWord]) -> ArbitrationResult;
}
```

Trit-Core 提供了默认实现 `JsonRuleLoader`。

## 2. CustomRule 结构

```json
{
  "name": "chemistry_safety",
  "priority_frame": "Science",
  "allow_forced_collapse": true,
  "fallback": "safe_fallback"
}
```

| 字段 | 类型 | 说明 |
|---|---|---|
| `name` | string | 规则名称，与 `Domain::Custom("chemistry_safety")` 对应 |
| `priority_frame` | string\|null | 优先帧（`"Science"`、`"Individual"`、`"Consensus"`、`null`） |
| `allow_forced_collapse` | bool | 是否允许强制坍缩。若为 true，优先帧匹配时调用 `Commit`；若为 false，调用 `Preserve` |
| `fallback` | string | 当优先帧不匹配时的行为：`"hold"`、`"negotiate"`、`"commit_first"`、`"safe_fallback"` |

## 3. 仲裁逻辑

```
优先帧匹配？
  ├── 是 → allow_forced_collapse?
  │         ├── true  → Commit(匹配的 TritWord)
  │         └── false → Preserve(匹配的 TritWord)
  └── 否 → fallback
            ├── "hold"          → Hold
            ├── "commit_first"  → Commit(第一个信号)
            ├── "safe_fallback"  → ForceCollapse（交由 SafeFallback 处理）
            └── 其他             → Negotiate
```

## 4. 使用示例

### 4.1 定义规则文件

创建 `rules/chemistry_safety.json`：

```json
{
  "name": "chemistry_safety",
  "priority_frame": "Science",
  "allow_forced_collapse": true,
  "fallback": "safe_fallback"
}
```

### 4.2 在代码中加载和应用

```rust
use trit_core::meta::{CustomRule, Domain, JsonRuleLoader, ResolutionPolicy, RuleLoader};
use trit_core::trit::TritWord;
use trit_core::Frame;

// 加载规则
let rule = JsonRuleLoader::load("rules/chemistry_safety.json").unwrap();

// 创建该域的 ResolutionPolicy
let policy = ResolutionPolicy::new(Domain::Custom("chemistry_safety".into()));

// 应用规则到输入
let inputs = vec![
    TritWord::tru(Frame::Science),
    TritWord::fals(Frame::Individual),
];
let result = JsonRuleLoader::apply(&rule, &inputs);
// 结果：ArbitrationResult::Commit(Science TritWord)
// 因为 Science 帧匹配优先帧，且 allow_forced_collapse = true
```

### 4.3 场景文件中使用自定义域

```json
{
  "id": "chemistry_experiment_01",
  "description": "化学实验安全评估",
  "domain": "chemistry_safety",
  "signals": [
    { "frame": "Science", "value": -1, "phase": 0.9 },
    { "frame": "Consensus", "value": 1, "phase": 0.6 }
  ],
  "expected_behavior": "commit_false"
}
```

## 5. 预置危险域

以下自定义域名在 `SafeFallback` 中预注册为危险的（即使没有外部规则文件）：

- `chemistry`
- `genetics`
- `structural`
- `nuclear`
- `pharmaceutical`

这意味着当 `Domain::Custom("chemistry")` 产生 Hold/Unknown 且存在中断时，SafeFallback 会自动强制 False。

### 注册新危险域

```rust
use trit_core::meta::SafeFallback;

let mut sf = SafeFallback::new();
sf.register_dangerous("biohacking");
// 现在 Domain::Custom("biohacking") 也会触发 SafeFallback
```

## 6. 规则加载错误处理

```rust
match JsonRuleLoader::load("rules/nonexistent.json") {
    Ok(rule) => { /* 使用规则 */ }
    Err(e) => eprintln!("规则加载失败: {}", e),
    // 输出: "规则加载失败: Failed to read rule file 'rules/nonexistent.json': ..."
}
```

## 7. 设计约束

- 规则文件是**不可信的输入**——加载失败不应该 panic
- `Domain::Custom(name)` 和 `CustomRule.name` 通过字符串匹配关联——确保名称一致
- 自定义域的 `ResolutionPolicy::arbitrate()` 始终返回 `Negotiate`——实际的仲裁逻辑在 `RuleLoader::apply()` 中
- 这是有意的分离：`ResolutionPolicy` 处理内置域，`RuleLoader` 处理外部域，两者通过 `SafeFallback` 统一衔接
