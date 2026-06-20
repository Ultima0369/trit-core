# Trit-Core 深度技术审计报告

> **历史版本说明**：本报告审计的是 Trit-Core v0.1.0（commit 未标注，约 6,463 行 Rust）。v0.2.0 已移除网络层、重构核心模块（`TritWord` 字段私有、`Phase::new` 返回 `Result`）。v0.3.0 进一步引入结构化可观测性、沙盒诊断和 `t_and_n` 批量运算。当前状态请参考 `audit_log/08_reflexive_audit.md`、aurora/08_reports/ 与最新源码。**本报告中部分问题已在后续版本中修正。**

**审计视角**: Google 资深项目 CTO  
**审计日期**: 2026-06-18  
**项目版本**: 0.1.0  
**代码规模**: ~6,463 行 Rust (31 源文件 + 3 二进制入口)  
**审计范围**: 全部源代码、测试、配置、文档、架构

---

## 执行摘要

Trit-Core 是一个异常高质量的研究级 Rust 项目。代码库展现了扎实的软件工程纪律：零 unsafe、零 clippy 警告、305 项测试（304 通过，1 项预存 proptest 基础设施问题）、零 TODO/FIXME、完整的 CI/CD 流水线。核心三值代数设计优雅，热/冷路径分离是真正的性能创新。

然而，作为从研究原型向生产级库过渡的项目，存在若干需要关注的问题。本报告识别出 **42 项发现**，按严重程度分为：**严重 (Critical) 5 项**、**高 (High) 11 项**、**中 (Medium) 16 项**、**低 (Low) 10 项**。

**总体评级: B+** — 研究级代码质量优异，但生产化需要处理 API 安全性、错误处理一致性和分布式协议健壮性问题。

---

## 评级体系

| 维度 | 评级 | 说明 |
|------|------|------|
| 核心代数正确性 | **A** | LUT 驱动、分支无关、语义一致。仅 1 项 TOR Unknown 传播不对称 |
| API 安全性 | **C+** | `TritWord` 公开字段允许构造非法状态；`#![deny(warnings)]` 对库不友好 |
| 错误处理 | **C** | 52 处 `unwrap()`、2 处 `expect()`、多处 `String` 错误类型 |
| 分布式协议 | **B-** | 架构合理但存在静默失败、随机对等节点选择、脑裂检测 panic 风险 |
| 性能工程 | **A-** | 热路径 ~3ns，端到端 658K-1.02M TPS。缺少 `codegen-units=1` |
| 测试覆盖 | **A-** | 305 测试、88 proptest、29 criterion benchmarks。`FrameMask` 无直接单测 |
| 文档质量 | **A** | 37+ 文档文件、中英双语、ADR 完整。最近审计已修复 31 项不一致 |
| 安全态势 | **A-** | `#![forbid(unsafe_code)]`、IEC 61508 SafeFallback、Byzantine 门卫。`MetaMonitor.log` 无界增长 |
| 依赖管理 | **B** | `dhat` 误放在 `[dependencies]`、`tokio` "full" features 过度拉取 |

---

## 严重发现 (Critical) — 必须修复

### C1. `TritWord` 公开字段允许构造非法状态

**文件**: `src/trit/mod.rs:22-24`  
**严重程度**: Critical  
**类别**: API 安全性 / 不变量违反

```rust
pub struct TritWord {
    pub value: TritValue,  // 可任意修改
    pub phase: Phase,      // 可绕过 Phase::new() 钳制
    pub frame: Frame,      // 可设置为 Absolute + True (违反核心不变量)
}
```

三个字段全部 `pub`。任何代码都可以构造 `TritWord { value: True, phase: Phase(2.0), frame: Absolute }`，绕过：
- `Phase::new()` 的 NaN/Inf/范围钳制
- `MetaMonitor::inspect()` 的 `Absolute → Hold` 不变量
- 所有构造函数提供的语义保证

**建议**: 将 `value` 和 `phase` 设为私有，提供 `set_value()` / `set_phase()` 访问器方法强制执行不变量。`frame` 可保留公开（无内部不变量）。

---

### C2. `commitment()` 与 `quantize()` 使用不一致的 epsilon

**文件**: `src/trit/phase.rs:64,74-82`  
**严重程度**: Critical  
**类别**: 正确性 / 数值一致性

```rust
// quantize 使用调用者提供的 epsilon (典型值 1e-6)
pub fn quantize(self, epsilon: f64) -> Phase { ... }

// commitment 使用硬编码的 f64::EPSILON (~2.2e-16)
pub fn commitment(self) -> Commitment {
    if self.0 > 0.5 + f64::EPSILON { ... }  // 10 个数量级的差异!
}
```

`quantize(1e-6)` 认为 `0.5000001` 是中性（不吸附），但 `commitment()` 将其分类为 `TowardTrue`。这意味着先量化再提交的代码会产生不一致的结果。

**建议**: 让 `commitment()` 接受可选的 epsilon 参数，默认值与 `quantize` 一致（1e-6），或添加文档说明这种不对称性。

---

### C3. `Instant::duration_since` panic 风险（时钟偏移）

**文件**: `src/net/bus.rs:113,148-157`  
**严重程度**: Critical  
**类别**: 正确性 / 运行时 panic

```rust
// stale_peers (line 113)
now.duration_since(*t).as_secs() > HEARTBEAT_TIMEOUT_SECS

// detect_split_brain (line 148-157)
now.duration_since(*t).as_secs() > SPLIT_BRAIN_TIMEOUT_SECS
```

`Instant::duration_since` 在 `*t > now` 时 panic。这在系统时钟调整、虚拟机挂起恢复或 NTP 校正时可能发生。测试代码（line 316）正确使用了 `checked_sub`，但生产代码未使用。

**建议**: 统一使用 `now.checked_duration_since(*t).map(|d| d.as_secs()).unwrap_or(0)`。

---

### C4. `Message::negotiate` 除零产生 NaN

**文件**: `src/net/message.rs:172`  
**严重程度**: Critical  
**类别**: 正确性 / 数值安全

```rust
let consensus_phase = phases.iter().sum::<f64>() / phases.len() as f64;
```

当 `phases` 为空时，计算 `0.0 / 0.0 = NaN`。虽然门卫会捕获 NaN，但构造函数本身不应产生无效消息。

**建议**: 添加 `assert!(!phases.is_empty(), "negotiate requires at least one phase")` 或返回 `Result`。

---

### C5. `parse_flag` 数组越界 panic

**文件**: `src/bin/node.rs:284,292,299`  
**严重程度**: Critical  
**类别**: 正确性 / CLI 健壮性

```rust
fn parse_flag(args: &[String], flag: &str, default: &str) -> String {
    args.iter()
        .position(|a| a == flag)
        .map(|i| args[i + 1].clone())  // 如果 flag 是最后一个元素则 panic!
        .unwrap_or_else(|| default.to_string())
}
```

当用户输入 `trit-node --frame`（忘记提供值）时，程序 panic 而非给出友好的错误消息。`parse_flag_f64` 和 `parse_flag_u16` 存在相同问题。

**建议**: 使用 `args.get(i + 1).cloned().unwrap_or_else(|| default.to_string())`。

---

## 高优先级发现 (High)

### H1. `#![deny(warnings)]` 对库 crate 不友好

**文件**: `src/lib.rs:1`  
**类别**: 下游兼容性

`deny(warnings)` 意味着下游用户使用较新 Rust 版本编译时，若新版本引入了新警告，`trit-core` 将导致编译失败。这是库 crate 的反模式。

**建议**: 从 `lib.rs` 移除，仅在 CI 中通过 `RUSTFLAGS="-D warnings"` 强制执行。

---

### H2. `From<i8>` 静默将越界值映射为 `Hold`

**文件**: `src/trit/value.rs:68-78`  
**类别**: API 设计 / 数据丢失

```rust
fn from(v: i8) -> Self {
    match v { 1 => True, -1 => False, _ => Hold }
}
```

`TritValue::from(127)` 返回 `Hold`，无任何错误指示。与 `Phase::try_new()` 的严格验证模式不一致。

**建议**: 添加 `TryFrom<i8>` 返回 `Result`，保留 `From<i8>` 仅用于 `{-1, 0, 1}` 子集。

---

### H3. `Unknown.to_i8()` 返回 0，与 `Hold` 不可区分

**文件**: `src/trit/value.rs:46`  
**类别**: 语义精度丢失

```rust
const TO_I8_LUT: [i8; 4] = [1, 0, -1, 0];  // Hold 和 Unknown 都映射到 0
```

任何使用 `to_i8()` 的下游代码都无法区分"故意悬置判断"和"超出分布无法计算"。`sandbox.rs:28` 的 `final_value_code` 明确承认了这种合并。

**建议**: 添加 `fn discriminant(self) -> u8` 返回 0/1/2/3 以保留完整 4 态区分。

---

### H4. `MetaMonitor.log` 无界增长

**文件**: `src/meta/interrupt.rs:58`  
**类别**: 内存安全

```rust
log: Vec<MetaInterrupt>,  // 无上限
```

在长时间运行的节点中，`log` 会无限增长。`ResonanceBus` 有 `MAX_MESSAGE_LOG=10,000` 环形缓冲区，但 `MetaMonitor` 没有。

**建议**: 改为 `VecDeque` 并设置可配置的容量上限，或添加 `drain()` 方法。

---

### H5. `Domain::Custom` 从未调用 `RuleLoader`

**文件**: `src/meta/domain.rs:64-67`  
**类别**: 设计 / 死代码

```rust
Domain::Custom(ref name) => {
    info!("Custom domain '{}': defaulting to Negotiate", name);
    ArbitrationResult::Negotiate
}
```

`Custom` 域总是返回 `Negotiate`，从不查询已加载的 `CustomRule`。`RuleLoader` 基础设施（`rules.rs`）和 `Domain::Custom` 变体是为彼此设计的，但从未连接。

**建议**: 将 `ResolutionPolicy` 与可选的 `CustomRule` 关联，或在文档中明确说明 `Custom` 域需要外部调用 `RuleLoader::apply`。

---

### H6. `SafeFallback::guard` 在 `interrupt_count == 0` 时绕过安全保护

**文件**: `src/meta/safe_fallback.rs:81`  
**类别**: 正确性 / 安全

```rust
if interrupt_count > 0 { /* 强制 False */ }
```

如果 `Unknown` 出现在危险域（如 Physical）但中断计数为 0，结果会原样通过。这与"IEC 61508：不知道就必须不做"的安全原则相矛盾——`Unknown` 本身就意味着"不知道"。

**建议**: 将条件改为 `result.value == TritValue::Unknown || (result.value == TritValue::Hold && interrupt_count > 0)`。

---

### H7. `push_log` 对 ACK 消息被绕过

**文件**: `src/net/coupling.rs:81`  
**类别**: 正确性 / 数据完整性

```rust
self.message_log.push_back(ack.clone());  // 绕过 push_log!
```

`handle_resonate_ack` 直接推送到 `message_log`，绕过了 `push_log` 的门卫追踪和 `MAX_MESSAGE_LOG` 上限强制执行。门卫的每对等节点日志计数将不准确。

**建议**: 改用 `self.push_log(ack.clone())`。

---

### H8. 所有已发现对等节点获得 `Frame::Meta`

**文件**: `src/net/discovery.rs:135-143`  
**类别**: 正确性 / 协议

```rust
fn extract_frame_from_state(_state: &str) -> Frame {
    Frame::Meta  // 总是 Meta，忽略输入!
}
```

所有通过种子发现的节点都被注册为 `Frame::Meta`。这意味着跨帧检查会将它们视为同帧（Meta==Meta），掩盖真实的帧冲突。

**建议**: 从心跳响应中提取实际帧信息，或默认为 `Frame::Consensus`（等待协商）。

---

### H9. `negotiate` 在参与者全部缺失时静默返回 Hold

**文件**: `src/net/negotiate.rs:20-31`  
**类别**: 正确性 / 静默失败

如果所有参与者 ID 都不在 `self.nodes` 中，函数返回 `(TritWord::hold(Frame::Meta), false)`——一个无冲突检测的 Hold，语义上不正确。

**建议**: 当零个参与者被找到时返回错误或至少设置 `conflict_detected = true`。

---

### H10. `t_hold` 丢弃原始 phase 无文档说明

**文件**: `src/trit/algebra.rs:125-127`  
**类别**: API 设计

```rust
pub fn t_hold(a: &TritWord) -> TritWord {
    TritWord::new(TritValue::Hold, 0.5, a.frame.clone())  // phase 信息丢失
}
```

强制 Hold 时硬编码 phase 为 0.5。在某些上下文中，调用者可能希望保留原始 phase（表示悬置前的倾向）。

**建议**: 文档说明理由，或添加 `t_hold_with_phase(a: &TritWord) -> TritWord` 变体。

---

### H11. `dhat` 在 `[dependencies]` 而非 `[dev-dependencies]`

**文件**: `Cargo.toml:23`  
**类别**: 依赖管理 / 编译时间

每个库消费者都需编译 `dhat` 及其传递依赖（包括 `backtrace`），尽管只有 `dhat_profile.rs` 二进制使用它。

**建议**: 移至 `[dev-dependencies]`。

---

## 中优先级发现 (Medium)

### M1. TOR Unknown 传播与 TAND 不对称

**文件**: `src/trit/algebra.rs:89-93`  
**类别**: 正确性 / 语义一致性

TAND 中 `(Unknown, _)` 无条件产生 `Unknown`。TOR 中 `(Unknown, False)` 落入通配符 `_ => Hold`。TritValue 文档说"Unknown 通过 TAND/TOR 传播"，但 TOR 对 `(Unknown, False)` 不遵循此规则。

**建议**: 统一行为或明确记录不对称性及其理由。

---

### M2. `try_new` 返回 `String` 而非类型化错误

**文件**: `src/trit/phase.rs:31`  
**类别**: API 设计

项目已依赖 `thiserror`，但 `try_new` 返回 `Result<Self, String>`。调用者无法以编程方式区分 NaN 和越界。

**建议**: 定义 `PhaseError` 枚举，包含 `NotFinite(f64)` 和 `OutOfRange(f64)` 变体。

---

### M3. `FrameMask` 魔法数字 `0b11111`

**文件**: `src/meta/frame_mask.rs:24`  
**类别**: 可维护性

```rust
if mask == 0b11111 { break; }  // 5 帧全部出现时提前退出
```

如果添加第 6 个 Frame 变体，编译器不会捕获此硬编码常量。应使用 `(1 << NUM_FRAMES) - 1` 或从枚举派生。

---

### M4. `FrameMask` 零直接单元测试

**文件**: `src/meta/frame_mask.rs`  
**类别**: 测试覆盖

`FrameMask` 仅在 `ResolutionPolicy::arbitrate()` 测试中间接测试。空输入、全 5 帧、重复帧等边界情况无直接覆盖。

---

### M5. `commitment()` 零测试

**文件**: `src/trit/phase.rs:92-148`  
**类别**: 测试覆盖

测试模块覆盖了 `quantize`、`mean`、`complement`，但完全没有 `commitment()` 的测试。鉴于 C2 中发现的 epsilon 不一致，这是关键缺口。

---

### M6. `t_sense` 零测试

**文件**: `src/trit/algebra.rs:136-220`  
**类别**: 测试覆盖

`t_sense` 是外部输入的主要入口点，但没有测试覆盖 NaN phase、Inf phase 或越界 phase 输入。

---

### M7. `arbitrate()` 空输入 panic

**文件**: `src/meta/domain.rs:71`  
**类别**: 正确性

```rust
ArbitrationResult::Commit(inputs[0].clone())  // 若 inputs 为空则 panic
```

`Domain::General` 路径在 `mask.count() == 1` 时访问 `inputs[0]`，但函数签名接受 `&[TritWord]`（可为空）。

---

### M8. `custom_rule` 的 `"commit_first"` 回退可能提交 `Unknown`

**文件**: `src/meta/rules.rs:56-60`  
**类别**: 正确性

如果第一个输入是 `Unknown`，`"commit_first"` 回退会提交它。提交 `Unknown`（"无法计算"）违背了安全架构的初衷。

---

### M9. 自定义规则帧解析错误被静默吞没

**文件**: `src/meta/rules.rs:41`  
**类别**: 正确性

如果 `priority_frame` 字符串不匹配任何已知帧（如 `"Science "` 有尾随空格），错误被吞没，规则静默降级为回退行为。

---

### M10. 字符串类型的回退行为

**文件**: `src/meta/rules.rs:53-64`  
**类别**: API 设计

`fallback` 字段是自由格式的 `String`，通过字符串比较匹配。`"Hold"`（大写 H）或 `"safe_falback"`（拼写错误）静默落入 `Negotiate`。

---

### M11. `ResolutionPolicy::new()` 在 `info!` 级别日志

**文件**: `src/meta/domain.rs:26`  
**类别**: 可观测性

每次构造策略对象都记录 `info!` 日志。在生产环境中会泛滥日志。

---

### M12. `MetaMonitor` 存储未使用的 `policy` 字段

**文件**: `src/meta/interrupt.rs:56`  
**类别**: 代码质量

```rust
#[allow(dead_code)]
policy: ResolutionPolicy,  // 存储但从未读取
```

---

### M13. `build_frame_mismatch_reason` 预分配容量可能不足

**文件**: `src/meta/interrupt.rs:32-42`  
**类别**: 正确性

硬编码 `String::with_capacity(48)`，但长操作名（如 `"RESONATE_NEGOTIATE"`）加上长帧名可能超过此容量，导致重新分配。

---

### M14. `SafeFallback` 危险域列表硬编码

**文件**: `src/meta/safe_fallback.rs:35-43`  
**类别**: 可配置性

危险自定义域（`"chemistry"`, `"genetics"`, `"structural"`, `"nuclear"`, `"pharmaceutical"`）硬编码在源码中。部署时无法通过配置文件或环境变量添加。

---

### M15. `ResonanceBus::register` 静默拒绝溢出

**文件**: `src/net/bus.rs:76-84`  
**类别**: API 设计

当达到 `MAX_NODES` 时，`register` 记录警告并静默返回。调用者无法知道注册失败。

---

### M16. `TcpNodeServer` 无优雅关闭机制

**文件**: `src/net/tcp_server.rs:72-94`  
**类别**: 运维

`serve` 运行无限循环，无 `CancellationToken`、关闭通道或 `serve_with_shutdown` 变体。唯一的停止方式是中止 tokio 任务。

---

## 低优先级发现 (Low)

### L1. `disc()` 应为 `const fn`

**文件**: `src/trit/value.rs:28`  
**类别**: 性能

此函数仅对枚举进行模式匹配。标记为 `const fn` 可让 LLVM 在编译时折叠 LUT 查找。

---

### L2. `quantize()` 近零检查不对称

**文件**: `src/trit/phase.rs:64`  
**类别**: 可维护性

`v < epsilon`（单侧）vs `(v - 0.5).abs() < epsilon`（对称）。在实践中安全（Phase 保证 ≥0），但代码令人困惑。

---

### L3. `Trit` 类型别名重复 `TritWord` 命名

**文件**: `src/trit/mod.rs:57`  
**类别**: API 清晰度

`pub type Trit = TritWord` 在 crate 根部重新导出，为同一类型提供两个名称。

---

### L4. TAND/TOR 真值表在 hot/cold 路径中重复

**文件**: `src/trit/algebra.rs:51-56,69-74,89-93,105-109`  
**类别**: 可维护性

相同的 match 逻辑出现 4 次。提取为 `const` LUT 数组可消除重复并实现无分支计算。

---

### L5. `Frame` 派生 `Hash` 但未使用

**文件**: `src/frame/mod.rs:4`  
**类别**: 死代码

`FrameRegistry` 使用 `Vec::contains`（需要 `PartialEq`，不需要 `Hash`）。

---

### L6. `Frame` 的 `Display` 和 `FromStr` 重复字符串字面量

**文件**: `src/frame/mod.rs:13-37`  
**类别**: 可维护性

每个变体名称在两处都以字符串字面量出现。`strum::Display` / `strum::EnumString` 可消除重复。

---

### L7. `tokio` "full" features 过度拉取

**文件**: `Cargo.toml:22`  
**类别**: 编译时间

`features = ["full"]` 包含 `rt-multi-thread`、`signal`、`process`、`fs`——库不需要这些。精简为 `["net", "sync", "rt", "macros", "time"]`。

---

### L8. 缺少 `codegen-units = 1`

**文件**: `Cargo.toml:51-53`  
**类别**: 性能

`[profile.release]` 有 `lto = true` 和 `opt-level = 3`，但缺少 `codegen-units = 1`。没有它，LLVM 无法在 thin-LTO 期间跨 CGU 内联。

---

### L9. `PllController` 字段全部 `pub`

**文件**: `src/net/pll.rs:9-15`  
**类别**: API 安全性

外部代码可以直接修改 `kp`、`deadband`、`max_correction`、`total_correction`，绕过所有不变量。

---

### L10. `BinaryBaseline` 是仅含静态方法的单元结构体

**文件**: `src/baseline/mod.rs:22`  
**类别**: 代码风格

所有方法都不接受 `&self`。一个包含自由函数的模块比单元结构体更符合 Rust 惯例。

---

## 指标汇总

| 指标 | 值 | 评级 |
|------|-----|------|
| 总测试数 | 305 | ✅ |
| 通过测试 | 304 | ✅ |
| 失败测试 | 1 (预存 proptest 基础设施) | ⚠️ |
| Clippy 警告 | 0 | ✅ |
| `unsafe` 代码 | 0 (`#![forbid(unsafe_code)]`) | ✅ |
| `unwrap()` 调用 | 52 (37 在测试中, 15 在生产代码中) | ⚠️ |
| `expect()` 调用 | 2 (均有文档说明的不变量) | ✅ |
| TODO/FIXME | 0 | ✅ |
| 依赖项数量 | 18 (含传递依赖 ~120) | ⚠️ |
| 文档文件 | 37+ | ✅ |
| ADR | 4 | ✅ |
| Criterion benchmarks | 29 | ✅ |
| 场景 JSON | 17 (含 5 中文变体) | ✅ |

---

## 架构评估

### 优势

1. **热/冷路径分离** — 这是真正的创新。同帧操作约 3ns，跨帧操作约 95ns，语义清晰分离。`debug_assert_eq!` 在 release 模式下零开销验证帧一致性。

2. **LUT 驱动的分支无关设计** — `NEGATE_LUT`、`TO_I8_LUT`、`TAND/TOR` 真值表使用查找表消除分支，对性能和安全（无时序侧信道）都有益。

3. **分层仲裁** — `FrameMask` (O(1) 位检测) → `ResolutionPolicy` (域特定规则) → `SafeFallback` (IEC 61508) 形成清晰的升级路径。

4. **Byzantine 门卫可选且零开销** — `Option<ByzantineGatekeeper>` 设计意味着禁用时无性能损失，启用时提供 7 重验证。

5. **中英双语文档** — 37+ 文档文件，完整的中文翻译，在开源项目中罕见且值得称赞。

### 关注点

1. **API 边界不安全** — `TritWord` 的公开字段意味着核心不变量（`Absolute → Hold`、Phase 范围）无法由类型系统强制执行。这是从研究原型向库演进时最关键的架构债务。

2. **错误处理不一致** — 代码库混合使用 `String` 错误、`&'static str` 错误、`unwrap()` panic 和类型化枚举（`GateRejection`）。缺乏统一的错误处理策略。

3. **分布式协议不完整** — `DecoupleAck` 未被处理、`Custom` 域与 `RuleLoader` 未连接、`to_trit()` 始终返回 Hold、种子发现将所有对等节点标记为 Meta。这些都是未完成功能的信号。

4. **无功能开关** — 核心 `trit/` 代数可能是一个轻量级 `no_std` crate，但 `tokio`、`chrono`、`serde` 等重量级依赖被无条件编译。

---

## 建议路线图

### 0.1.1 补丁 (1-2 周) — 关键修复

1. **C1**: 将 `TritWord.value` 和 `TritWord.phase` 设为私有，添加访问器
2. **C2**: 统一 `commitment()` 和 `quantize()` 的 epsilon
3. **C3**: 在 `stale_peers` 和 `detect_split_brain` 中使用 `checked_duration_since`
4. **C4**: 在 `Message::negotiate` 中添加 `phases` 非空断言
5. **C5**: 修复 `parse_flag` 数组越界
6. **H1**: 从 `lib.rs` 移除 `#![deny(warnings)]`

### 0.1.2 补丁 (2-4 周) — 高优先级改进

7. **H2/H3**: 添加 `TryFrom<i8>` 和 `TritValue::discriminant()`
8. **H4**: 为 `MetaMonitor.log` 添加上限
9. **H6**: 修复 `SafeFallback` 的 `interrupt_count == 0` 绕过
10. **H7**: 修复 `handle_resonate_ack` 中 `push_log` 被绕过的问题
11. **H11**: 将 `dhat` 移至 `[dev-dependencies]`
12. **M1-M10**: 解决中优先级问题

### 0.2.0 (1-3 月) — 架构改进

13. 引入功能开关（`full`、`net`、`sandbox`）
14. 统一错误处理（类型化错误枚举）
15. 连接 `Domain::Custom` 与 `RuleLoader`
16. 完成分布式协议（DecoupleAck 处理、帧发现、优雅关闭）
17. 考虑 `trit/` 的 `no_std` 兼容性

---

## 与行业标准对比

| 标准 | Trit-Core 状态 | 差距 |
|------|---------------|------|
| OWASP 安全编码 | ✅ `#![forbid(unsafe_code)]`、IEC 61508 SafeFallback、Byzantine 门卫 | — |
| Semantic Versioning | ✅ 0.1.0，核心代数冻结 | — |
| Keep a Changelog | ✅ CHANGELOG.md 格式正确 | — |
| Rust API 指南 | ⚠️ 部分遵循 | 公开字段、`String` 错误、缺少 `TryFrom` |
| 12-Factor App | ⚠️ 部分遵循 | 硬编码危险域列表、无优雅关闭 |
| 安全供应链 | ⚠️ 18 直接依赖 | 无 `cargo-deny`、无 SBOM、无依赖审计 CI |

---

## 结论

Trit-Core 是一个令人印象深刻的研究项目，具有真正的创新（三值决策、热/冷路径分离、域感知仲裁）。代码质量在 Rust 生态系统中处于前 20%。核心代数设计优雅，测试覆盖全面。

主要弱点在于**API 安全性**（公开字段允许非法状态）和**分布式协议成熟度**（若干未完成功能）。这些是研究原型向生产级库过渡的典型问题，而非根本性设计缺陷。

**最关键的单一改进**: 封装 `TritWord` 的字段。这将使类型系统能够强制执行核心不变量，并将整个 API 表面的安全性提升一个档次。

---

*审计由 Claude Code 以 Google 资深项目 CTO 视角执行。基于对全部 31 个源文件、305 项测试、29 项基准测试和 37+ 份文档文件的第一手检查。*
