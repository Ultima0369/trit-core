# CONTRIBUTING — 贡献指南

欢迎贡献。本文档描述代码风格、CI 门禁、测试策略和扩展方法。

## 1. 开发环境

```bash
# 克隆
git clone https://github.com/trit-core/trit-core.git
cd trit-core

# 确保通过所有门禁
cargo fmt -- --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all-features
```

## 2. 代码风格

### 2.1 格式化

使用 `rustfmt` 默认配置。CI 强制检查：

```bash
cargo fmt -- --check   # CI 门禁
cargo fmt              # 自动修复
```

### 2.2 Lint

Clippy 以 `-D warnings` 运行——所有警告都是错误：

```bash
cargo clippy --all-targets --all-features -- -D warnings
```

### 2.3 命名约定

- 模块：`snake_case`（`frame_mask.rs`、`safe_fallback.rs`）
- 类型：`PascalCase`（`TritWord`、`ResolutionPolicy`）
- 函数/方法：`snake_case`（`t_and`、`arbitrate`、`is_dangerous`）
- 常量：`SCREAMING_SNAKE_CASE`（`MAX_MESSAGE_LOG`、`MAX_NODES`）

### 2.4 文档

- 所有 `pub` 项必须有文档注释（`///`）
- 模块级文档（`//!`）描述模块职责
- 内部注释使用 `//`，解释"为什么"而非"是什么"

## 3. 硬性约束

### 3.1 不可违反

- **`#![forbid(unsafe_code)]`** — 零 unsafe 代码，无例外
- **`#![deny(warnings)]`** — 警告即错误
- **核心代数冻结** — `src/trit/algebra.rs` 中 TAND/TOR/TNOT 的真值表在 0.1.x 中不可变。这是为了结果的可复现性

### 3.2 设计原则

- **跨帧操作不强制二元决策** — 始终返回 Hold + MetaInterrupt
- **Absolute 帧必须永远 Hold** — 由 MetaMonitor 强制执行
- **Phase 构造时钳制 NaN/Inf** — 防止浮点异常传播

## 4. 测试策略

### 4.1 测试类型

| 类型 | 位置 | 说明 |
|---|---|---|
| 单元测试 | `src/**/*.rs`（`#[cfg(test)]` 模块） | 每个模块的内部测试 |
| 集成测试 | `tests/integration_test.rs` | 跨模块场景测试 |
| 属性测试 | `tests/proptest.rs` | 随机化不变性验证 |
| 基准测试 | `benches/trit_bench.rs` | Criterion 性能基准 |

### 4.2 运行测试

```bash
cargo test --all-features          # 全部测试（170 个）
cargo test -- trit_tests           # 特定模块
cargo test -- proptest             # 仅属性测试
cargo bench                        # 基准测试
```

### 4.3 添加新测试

- 新功能必须有对应的单元测试
- 如果新功能涉及代数不变性，在 `tests/proptest.rs` 中添加 proptest
- 如果新功能涉及跨模块行为，在 `tests/integration_test.rs` 中添加场景测试

## 5. 如何添加新 Frame

1. 在 `src/frame/mod.rs` 的 `Frame` 枚举中添加变体
2. 在 `Display` 实现中添加对应的字符串
3. 在 `FromStr` 实现中添加解析逻辑
4. 在 `src/meta/frame_mask.rs` 中分配一个新的 bit（注意 `u8` 最多 8 位，当前使用 5 位）
5. 在 `FrameMask::from_inputs()` 和 `has()` 中添加对应的 match 分支
6. 在 `tests/proptest.rs` 的 `arb_frame()` 策略中添加新变体
7. 更新 `docs/concepts/CONCEPTS.md` 中的 Frame 表格

## 6. 如何添加新 Domain

1. 在 `src/meta/domain.rs` 的 `Domain` 枚举中添加变体
2. 在 `ResolutionPolicy::arbitrate()` 中添加该域的仲裁逻辑
3. 在 `src/meta/safe_fallback.rs` 的 `is_dangerous()` 中决定该域是否危险
4. 在 `domain_label()` 中添加该域的人类可读标签
5. 在 `tests/proptest.rs` 中添加对应的仲裁不变性测试
6. 更新 `docs/concepts/CONCEPTS.md` 中的 Domain 表格

## 7. 提交信息格式

```
<type>: <简短描述>

<详细说明（可选）>

Co-Authored-By: Claude <noreply@anthropic.com>
```

类型：`feat`、`fix`、`test`、`refactor`、`docs`、`chore`、`security`

## 8. 发布流程

1. 所有测试通过
2. Clippy 零警告
3. `cargo fmt -- --check` 通过
4. 更新 `Cargo.toml` 版本号
5. 更新 `README.md` 中的测试计数和状态
6. Git tag + `cargo publish`（未来）
