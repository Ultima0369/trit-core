# Trit-Core 深度审计综合报告

**日期**: 2026-07-06  
**分支**: `ocr-empty-base` (clean)  
**审计范围**: trit-core v0.3.0 + aurora v0.1.0 + dataforge v0.1.0 + aurora-desktop v0.1.0  
**索引节点数**: 3,611 符号, 8,528 边 (CodeGraph)

---

## 执行摘要

Trit-Core 是一个**架构纪律良好、测试覆盖率扎实**的 Rust workspace。核心三元逻辑正确执行，不变量由构造函数强制，测试全面（477 个 lib test 全部通过）。主要风险集中在：

1. **Aurora 测试 ICE** — rustc 1.96 编译器 bug 导致 8 个集成测试无法编译
2. **`unwrap()` 调用量偏高** — 非测试代码中 295 个 `.unwrap()`，多数可消除
3. **未使用的 `allow(dead_code)` 字段** — 7 处标注了未使用但保留了字段
4. **Aurora-desktop 依赖链偏重** — 引入 Tauri + actix-web + moka 用于桌面 shell

---

## 一、构建与测试状态

| 项目 | 状态 |
|------|------|
| `cargo fmt --check` | ✅ 通过 |
| `cargo clippy --lib` | ✅ 通过（非 test target） |
| `cargo test --lib` (所有 lib) | ✅ **477 passed, 0 failed** |
| `cargo clippy --all-targets` | ❌ `dataforge` 1 个 clippy error |
| `cargo test --all-targets` | ❌ `aurora` 8 个集成测试 ICE |

### 集成测试 ICE 详情

8 个 aurora 集成测试在 rustc 1.96.0 上触发 `Res::Err but no error emitted` ICE：

| 测试文件 | 触发原因 |
|----------|----------|
| `cloud_llm_tests.rs` | `truncore::TritValue` 别名解析 + 类型推断冲突 |
| `percept_chain_tests.rs` | 同上，`use aurora::percept` 解析失败 |
| `ethics_gate_tests.rs` | 级联失败（依赖 aurora crate） |
| `local_llm_tests.rs` | 级联失败 |
| `wavelet_detect.rs` | 级联失败 |
| `fft_provider_tests.rs` | 级联失败 |
| `attention_session.rs` | 级联失败 + `idna`/`icu_normalizer` rlib 缺失 |

**根因**: `aurora/Cargo.toml` 使用 `truncore = { path = "..", package = "trit-core" }` 别名。rustc 1.96 在特定代码路径下（多个 extern crate 共存时）对别名的解析存在 ICE。这是**编译器 bug**，非项目代码问题。

**修复方案**:
1. **短期**: 升级到 rustc 1.97+ (预计修复)
2. **变通**: 将 `truncore` 别名改为标准 `trit-core` 依赖名，或添加 `#[cfg(test)] extern crate trit_core;` 桥接
3. **最低成本**: 添加 `#![allow(internal_features)]` 或在受影响的测试文件中显式使用 `trit_core::` 路径

---

## 二、安全性审计

### 2.1 Unsafe 代码

**状态**: `#![forbid(unsafe_code)]` 在 trit-core 和 dataforge 中严格执行。唯一例外是 `aurora/src/config/dpapi.rs`，使用 `#![allow(unsafe_code)]` 隔离 Windows DPAPI FFI 调用。

```
文件: aurora/src/config/dpapi.rs
函数: encrypt() / decrypt()
unsafe 块数: 10 (全部是 Windows CryptProtectData/CryptUnprotectData FFI)
```

**评估**: 可接受。DPAPI 是 Windows 平台原生加密，FFI 边界隔离在一个文件中。两个函数都有明确的文档说明和错误处理。

### 2.2 `.unwrap()` 密度

| Crate | 非测试代码 `.unwrap()` 数 | 风险 |
|-------|--------------------------|------|
| trit-core | 107 | 低（主要在 bin/工具代码） |
| aurora | 178 | **中高** |
| dataforge | 10 | 低 |

**高风险区域**:
- `aurora/src/app.rs:416-466` — 测试代码中有 15+ 个连续 `.unwrap()`，但标记为 `#[cfg(test)]`
- `aurora/src/db/mod.rs:91,97` — **两个 `panic!()`**：`"failed to clone database connection"`
- `src-tauri/src/lib.rs:120` — **`panic!("日志初始化失败: {e}")`** — 日志失败不应该是 fatal

### 2.3 静默忽略的错误

```
dataforge/src/cache.rs:34  let _ = fs::create_dir_all(&base_dir);
dataforge/src/cache.rs:55  let _ = touch_file(&path_clone);
dataforge/src/cache.rs:78  let _ = fs::remove_file(&tmp);
dataforge/src/cache.rs:86  let _ = self.evict_lru();
dataforge/src/cache.rs:140 let _ = fs::remove_file(path);
dataforge/src/cache.rs:196 let _ = fs::remove_dir(&path);
```

缓存操作使用 `let _ =` 静默吞没所有错误。`create_dir_all` 失败应该被记录（至少 `tracing::warn!`），同理 `evict_lru` 失败。

### 2.4 SQL 注入

**状态**: ✅ 无风险。`aurora/src/db/` 中所有 SQL 使用 `rusqlite` 的参数化查询（`?` 占位符）。无字符串拼接 SQL。

### 2.5 路径遍历

**状态**: ✅ 无风险。`dataforge/src/cache.rs` 的路径操作限制在 `dirs::cache_dir()` 下。

### 2.6 密钥管理

**状态**: ✅ 无硬编码密钥。API keys 通过 `ConfigStore` (DPAPI 加密) 管理。

---

## 三、架构审计

### 3.1 模块内聚性

| 模块 | 行数 | 评估 |
|------|------|------|
| `src/sandbox/pipeline.rs` | 949 | ⚠️ 最大单文件，可拆分 |
| `src/meta/domain.rs` | 713 | ⚠️ 仲裁逻辑与领域定义混合 |
| `src/adapters/self_knowledge.rs` | 707 | ⚠️ 超过 700 行 |
| `src/core/algebra.rs` | 618 | ✅ 纯代数，合理 |
| `src/adapters/bandwidth_scheduler.rs` | 491 | ✅ 边界清晰 |

### 3.2 耦合度

```
TritWord 影响面: 472 个符号（最大热区）
Phase 影响面: 108 个符号
```

`TritWord` 是核心类型，影响面广是正常的。但值得关注的是：
- `TritWord` 直接依赖 6 个文件（word, algebra, decision_engine, safe_fallback, interrupt, rules）
- 所有 adapter 都依赖 `TritWord`（符合设计）
- `aurora` 通过 `truncore` 别名消费 `TritWord`（正常）

### 3.3 架构合规性

对照 CLAUDE.md 五层架构检查：

| 层级 | 合规 | 备注 |
|------|------|------|
| Layer 1 (Anchor) | ✅ | 5 个约束全部实现，veto 逻辑正确 |
| Layer 2 (Hook) | ✅ | ScenarioRecognizer + MountArbiter + HookContext |
| Layer 3 (Adapters) | ✅ | 10 个模块，均通过 HookContext 通信 |
| Layer 4 (Core + Meta) | ✅ | TritValue/Phase/Frame 不变量由构造函数强制 |
| Layer 5 (Feedback) | ✅ | PracticeTest + ProxyEnvironment |

**关键规则验证**:
- `#![forbid(unsafe_code)]` ✅
- `Frame` 和 `TritWord` 是 `Copy` ✅
- 跨帧操作产生 `Hold + MetaInterrupt` ✅
- `Absolute` 帧必须保持 `Hold + neutral` ✅
- `Phase::new` 返回 `Result` ✅
- 策略代码无 panic ✅
- 模块不互相调用 ✅

### 3.4 依赖审计

| Crate | 直接依赖数 | 评估 |
|-------|-----------|------|
| trit-core | 8 | ✅ 轻量 |
| dataforge | 8 | ✅ 合理（HTTP + JSON） |
| aurora | 18 | ⚠️ 偏重 |
| aurora-desktop | 19 | ⚠️ 偏重（Tauri + actix-web + moka + flate2 + tar） |

**aurora-desktop 引入 actix-web 但未见明显的 HTTP server 使用**。`moka` 缓存库对桌面应用可能是过度设计。

---

## 四、Ponytail 复杂度审计

### 4.1 Dead Code (`#[allow(dead_code)]`)

| 位置 | 标记为 dead 的字段 | 建议 |
|------|-------------------|------|
| `src/anchor/cost_factor.rs:139` | `FactorFile.description` | 删除或使用 |
| `src/sandbox/diagnostic.rs:200` | `deserialize` 方法 | 保留（序列化对称性） |
| `aurora/src/percept/cloud.rs:39` | `config: Arc<ConfigStore>` | 存储但未读取 → 删除或实现 |
| `aurora/src/percept/fft.rs:14` | `spec: SignalSpec` | 存储但未读取 |
| `aurora/src/percept/local.rs:17` | `config: Arc<ConfigStore>` | 存储但未读取 |
| `aurora/src/ingest/json_fallback.rs:26` | `path: PathBuf` | 存储但未读取 |
| `dataforge/src/sources/gbif.rs:50` | `status: Option<String>` | 解析但未使用 |

**合计**: 7 个字段声明为 dead code → 可删除 5 个（保留 diagnostic 和 gbif 的 serde 字段）。

### 4.2 YAGNI

| 发现 | 标签 | 详情 |
|------|------|------|
| `ResponsePatternCache` | `yagni:` | `src/adapters/self_knowledge.rs` — 单消费者模式缓存 |
| `CostFactor` trait | `yagni:` | `src/anchor/cost_factor.rs` — 一个 trait，零个实现（仅测试 mock） |
| `DataSource<T>` trait | `yagni:` | `src/anchor/mod.rs:184` — 一个泛型 trait，零个具体实现 |
| `prism.rs` (611行) | `shrink:` | 感知降级逻辑可压缩 — 大量 `match` 分支 |
| `aurora/src/app.rs:321` `render_output` | `delete:` | 未被 pipeline 调用（代码路径从 `run_pipeline` 直接到 `Presentation` BC） |

### 4.3 Stdlib 替代

| 发现 | 标签 | 详情 |
|------|------|------|
| 自定义 `Phase` 包装 | `stdlib:` | `Phase::new(0.0..=1.0)` 可用 `OrderedFloat(f64)` + `clamp`。但语义包装有其价值，**保留** |
| `src/meta/frame_mask.rs` `FrameMask` | `shrink:` | 60 行的位掩码可用 `enumset::EnumSet<Frame>` 或直接 `HashSet<Frame>` |

---

## 五、测试质量审计

### 5.1 覆盖率概览

| 类别 | 数量 |
|------|------|
| 工作区测试文件 | 23 个 |
| 单元测试函数 | ~660 个 |
| 集成测试 | 23 个文件 |
| 基准测试 | 2 个文件 |
| 伦理门测试 | 10 个（声明）/ 多个文件 |

### 5.2 伦理门测试

在以下文件中找到：
- `tests/core_invariants_test.rs`
- `aurora/tests/ethics_gates.rs`
- `aurora/tests/ethics_gate_tests.rs`

**状态**: 所有 lib test 通过（477 passed）。伦理门覆盖 `cross_frame_must_hold`, `absolute_frame_remains_hold`, `meta_frame_cannot_be_external_input`, `value_conflict_produces_hold` 等关键不变量。

### 5.3 Proptest

`tests/proptest.rs` 包含基于属性的测试：
- `arb_trit_word` / `arb_computable_trit_word` / `arb_committable_trit_word`
- 验证三元运算的代数性质

### 5.4 缺失测试

| 未测试模块 | 原因 |
|-----------|------|
| `src/bin/adversarial_audit.rs` | CLI 工具，无自动化测试 |
| `src/bin/dhat_profile.rs` | 性能分析工具 |
| `aurora/src/config/dpapi.rs` | 需要 Windows + DPAPI，难以自动化 |
| `src-tauri/src/` | 桌面 shell，依赖 Tauri 运行时 |

### 5.5 测试隔离

✅ 所有测试使用 `Database::open_in_memory()` 或纯函数。无共享可变状态。

---

## 六、发现汇总

### 🔴 Critical

1. **Aurora 集成测试 ICE** — 8 个测试文件因 rustc 1.96 bug 无法编译。需升级编译器或变通修复 `truncore` 别名。

2. **`panic!` 在非测试代码中** — `aurora/src/db/mod.rs:91,97`（数据库克隆失败 → panic）和 `src-tauri/src/lib.rs:120`（日志初始化失败 → panic）。应返回 `Result`。

### 🟠 High

3. **295 个 `.unwrap()` 在非测试代码中** — aurora 178 个，trit-core 107 个。多数可通过 `?`、`unwrap_or_else` 或 error propagation 消除。

4. **`let _ =` 吞没错误** — `dataforge/src/cache.rs` 的 6 处静默忽略 I/O 错误。应至少 `tracing::warn!`。

5. **未连接的代码路径** — `aurora/src/app.rs:render_output` 未被 pipeline 调用。Presentation BC 直接渲染，跳过了 app 层的 `render_output`。

### 🟡 Medium

6. **7 个 `#[allow(dead_code)]` 字段** — 5 个可安全删除（cloud/local/fft/ingest 中未使用的 config/spec/path 字段）。

7. **两个零实现的 trait** — `CostFactor` trait 和 `DataSource<T>` trait 没有非测试实现。如果是为未来预留 → YAGNI，删除。如果是接口文档 → 文档注释即可。

8. **`aurora-desktop` 依赖过重** — actix-web + moka 的引入理由不明确。如果桌面 shell 不需要 HTTP server，应移除 actix 依赖。

9. **`self_knowledge.rs` (707 行) 超大** — `ResponsePatternCache` 的 lookup 逻辑与 `infer_receiver` 逻辑可拆分。

### 🟢 Low

10. **`FrameMask` 位掩码可简化** — 60 行手动位掩码可替换为 `HashSet<Frame>`（除非性能关键路径）。

11. **`src/sandbox/diagnostic.rs` 的 `deserialize` 方法** — 返回 `Ok(None)` 表示 "不可反序列化"，但调用方无感知。可改为 `#[serde(skip_deserializing)]`。

12. **Clipy: `needless_borrow`** — `dataforge/tests/integration_test.rs:122`，修复方式：`cache.put(cache_key, &header)` → `cache.put(cache_key, &header)`。

---

## 七、行动建议（优先级排序）

### 立即（本周）
1. **修复 rustc ICE** — 将 `truncore` 别名移除或添加兼容桥接，恢复 8 个集成测试
2. **消除 3 个 `panic!`** — 改为返回 `Result` 或 `tracing::error!` + graceful degradation

### 短期（本月）
3. **减少 `.unwrap()` 密度** — 目标：aurora 从 178 → <20，trit-core 从 107 → <30
4. **清理 `allow(dead_code)`** — 删除 5 个未使用字段
5. **修复 `let _ =`** — 在 `dataforge/cache.rs` 中添加 `tracing::warn!`

### 中期（下季度）
6. **删除零实现 trait** — `CostFactor` 和 `DataSource<T>`
7. **审计 aurora-desktop 依赖** — 移除不必要的 actix-web（如不需要 HTTP server）
8. **拆分超大文件** — `self_knowledge.rs` (707行) → 分离 `pattern_cache.rs` 和 `inference.rs`

### 长期
9. **考虑 `Phase` 改为 `#[repr(transparent)]`** — 与 `f64` ABI 兼容，允许 SIMD 优化
10. **增加 property-based testing 覆盖面** — 对 `TernaryAlgebra` 添加更多代数性质 proptest（结合律、分配律）

---

## 八、基准指标

| 指标 | 值 |
|------|-----|
| 总源码行数 (src) | ~24,000 |
| 总测试行数 | ~8,000 |
| 单元测试数 | 477 (all passing) |
| 集成测试数 | 23 文件 (8 ICE) |
| 公共 trait 数 | 6 |
| `unsafe` 块数 | 10 (全部隔离在 dpapi.rs) |
| `panic!` (非测试) | 3 |
| `.unwrap()` (非测试) | 295 |
| TODO/FIXME/HACK | 0 |
| 架构层级合规 | 100% |

---

**总体评级**: **B+** — 核心架构扎实，测试覆盖好，零 unsafe（除隔离 FFI）。主要扣分项：rustc ICE 导致 1/3 的集成测试不可运行，以及 `unwrap()` 文化偏宽松。

**最值得投资的一行改动**: 移除 `truncore` 别名 → 恢复 8 个集成测试 → 发现真正的 bug vs 所有测试通过。
