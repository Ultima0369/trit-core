# Trit-Core v0.1.0 — CTO 深度审计报告

**审计日期**：2026-06-18  
**审计视角**：Google 资深项目 CTO / 工程副总裁  
**项目版本**：v0.1.0（commit 6fada84，M0–M9 宣称完成）  
**审计范围**：完整代码库（`src/`、`tests/`、`benches/`、CI/CD、依赖、文档）  
**审计方法**：静态代码审查、工具链验证、架构与风险面分析  

> **历史版本说明**：本报告审计的是 Trit-Core v0.1.0。v0.2.0 已移除网络层并重构核心模块（如 `TritWord` 字段私有、`Phase::new` 返回 `Result`、移除 `src/net/`）。当前状态请参考 `audit_log/08_reflexive_audit.md` 与最新源码。

---

## 1. 执行摘要

Trit-Core 是一份**工程纪律严明、文档极度丰富、核心代数经过_property-based testing_强验证**的 MVP。它成功地把一个哲学/AI 安全假设（“三元决策比二元平均更能保留冲突”）转化成了可运行的 Rust 代码，并在 4,500 行左右实现了 ternary algebra、policy arbitration、safe fallback、分布式节点协议、Byzantine gatekeeper 和完整的测试矩阵。

但是，作为准备进入生产或吸引外部贡献者的项目，它仍然存在若干**高优先级风险**：本地/CI 编译内存压力、关键路径上的静默降级（silent clamping / fallback）、分布式协议中若干“演示级”而非“生产级”的实现，以及核心不变量靠约定而非类型系统保证。

**总体评级**：

| 维度 | 评级 | 说明 |
|------|:--:|------|
| 代码质量 | A- | Clippy clean、无 unsafe、格式统一，但存在 unwrap/expect、重复验证与公开可变字段 |
| 架构设计 | B+ | 分层清晰，热/冷路径分离，但部分模块（clock、frame registry）尚未融入主路径 |
| 核心正确性 | A- | 真值表、Unknown 传播、De Morgan、domain policy 均有 proptest 覆盖；API  misuse 面较大 |
| 安全 / 风险 | C+ | 网络层身份可伪造、明文传输、gatekeeper 默认关闭、BFT 声明过度 |
| 并发 / 分布式 | C+ | TCP/PLL/分区容忍/BFT 门控都有原型，但分布式语义、decouple、目标选择均未闭合 |
| 测试策略 | B+ | 覆盖广，但 expected_behavior 是死代码、pipeline 与 CLI 不一致、当前环境无法编译 |
| 性能工程 | B+ | 热路径零分配声明合理，但缺少可复现的 TPS 数据 |
| 可维护性 / 文档 | A- | 文档、ADR、双语、审计报告丰富，但 scenario 期望与代码行为不一致 |
| 运营成熟度 | C | 本地编译即 OOM，CI 在当前树不是全绿，对贡献者不友好 |

**加权综合评分：B（约 76/100）**  
**建议**：在对外宣传“生产就绪”之前，必须先解决 P0/P1 风险。

---

## 2. 关键发现（按优先级）

### P0 — 阻塞级风险

#### P0.1 完整测试套件在当前环境无法编译
- **位置**：全局（`cargo test --all-features`）
- **现象**：运行 `cargo test --all-features -- --test-threads=1` 时，rustc 出现大量 `memory allocation ... failed`、`页面文件太小，无法完成操作 (os error 1455)`，最终 `thread ... has overflowed its stack`。
- **影响**：开发者无法在本地验证更改；CI 若使用相同 Windows 配置也会失败。README 宣称“305 passing, 0 failures”与本地可复现性之间存在裂痕。
- **根因**：
  1. `--all-features` 同时启用了 `dhat-profile`，拉入 dhat 及其 proc-macro/debug 依赖；
  2. 测试文件（尤其 `tests/proptest.rs` 1,520 行、Criterion + proptest + tokio 同时链接）导致链接阶段内存峰值过高；
  3. Windows 默认页面文件 + scoop 安装的 rustup 工具链可能配置偏小。
- **建议**：
  1. 将 dhat 相关测试/二进制从默认 `cargo test` 中拆分；
  2. CI 中不再要求 `--all-features` 用于单元测试，而是单独为 `dhat-profile` 写 job；
  3. 在 README/CONTRIBUTING 中给出内存/页面文件建议；
  4. 提供 `cargo test --lib` 快速反馈路径。

#### P0.2 网络层缺乏身份认证，任意客户端可伪造 sender
- **位置**：`src/net/tcp_server.rs:125-142`、`src/net/gate.rs:200-236`、`src/net/frame_codec.rs:40-59`
- **问题**：
  - 协议使用明文 JSON over TCP；
  - `sender` 字段完全来自消息本身，没有任何与 TCP 连接绑定的身份验证；
  - gatekeeper 的 known-sender 检查在 `known_nodes` 为空时（默认状态）完全关闭。
- **影响**：在同一网络中的任何主机都可以发送 `RESONATE_REQ`/`HEARTBEAT`/`NEGOTIATE` 并冒充其他节点，造成身份欺骗、状态污染、分区误判。
- **建议**：
  - 短期：默认启用 gatekeeper 并要求显式注册所有已知节点；
  - 中期：引入 mTLS 或每条消息的数字签名，将节点身份绑定到连接/密钥。

#### P0.3 `expected_behavior` 字段是死代码，且多个场景期望值与 CLI 实际输出矛盾
- **位置**：`src/sandbox.rs:10`、`src/bin/sandbox.rs:138-149`、`scenarios/general_negotiation.json`、`scenarios/medical_conflict_*.json`
- **问题**：
  - `expected_behavior` 在代码中只被解析，从未被任何测试读取；
  - `medical_conflict_01.json` 期望 `hold`，但 CLI 输出 `False`（Individual: -1，MedicalEthics Preserve 规则）；
  - `medical_conflict_02.json`、`medical_conflict_03.json` 同样期望 `hold`，实际输出为 `True`/`False`；
  - `general_negotiation.json` 期望 `commit_false`，实际输出 `True`（General 域同帧取 first signal）。
- **影响**：场景文件作为项目核心验证资产，其声明与代码行为不一致，严重削弱 M2 验证报告的可信度。
- **建议**：
  - 立即修正错误期望值，或移除该字段；
  - 添加自动化测试，加载所有 `scenarios/*.json` 并断言 `expected_behavior` 与实际输出一致。

---

### P1 — 高优先级风险

#### P1.1 `Phase::new()` 默认静默 clamp，可能隐藏上游 bug
- **位置**：`src/trit/phase.rs:16-27`
- **代码**：
  ```rust
  pub fn new(v: f64) -> Self {
      if v.is_nan() || v.is_infinite() { ... return Phase(0.5); }
      if !(0.0..=1.0).contains(&v) { let clamped = v.clamp(0.0, 1.0); ... }
      Phase(v)
  }
  ```
- **问题**：所有非法输入都被降级为合法值并记录 warning。对于核心代数来说，**非法 phase 是一个合约违反**，应当通过类型系统强制失败（`try_new` 已存在，但不是默认构造函数）。
- **风险**：在下游 sandbox、node、PLL 中，任何计算错误导致的越界 phase 都会被“自动修复”，掩盖数值漂移或逻辑错误。
- **建议**：
  - 在 `0.2.0` 中考虑让 `Phase::new` 在 debug build 下 panic、release build 下 clamp；或
  - 将 `TritWord::new` 改为使用 `Phase::try_new` 并返回 `Result`。

#### P1.2 Sandbox CLI 对未知 frame 静默回退到 `Frame::Meta`
- **位置**：`src/bin/sandbox.rs:173`
- **代码**：
  ```rust
  let frame: Frame = s.frame.parse().unwrap_or(Frame::Meta);
  ```
- **问题**：`validate_scenario` 已经校验 frame 必须是已知值，理论上这行不会触发；但**防御层与执行层不一致**——如果未来有人绕过 `validate_scenario`，未知输入会被静默当成 Meta 处理，而不是报错退出。
- **建议**：改为 `expect("validated frame")` 或返回错误退出；或者让 `validate_scenario` 返回解析后的结构，避免重复解析。

#### P1.3 `Absolute` 帧不变量靠约定而非类型系统保证
- **位置**：`src/meta/interrupt.rs:92-101`、`src/trit/mod.rs:70-84`
- **问题**：
  - `TritWord::set_value` 可以直接把 `Frame::Absolute` 的 trit 设为 `True`；
  - `MetaMonitor::inspect` 能检测这个问题，但**没有任何核心路径自动调用 inspect**；
  - `TernaryAlgebra` 也没有对 `Absolute` 帧做特殊处理。
- **风险**：核心不变量“`Absolute` must always remain Hold”容易被遗忘，尤其在自定义规则、网络节点、序列化反序列化之后。
- **建议**：
  - 在 `TritWord::set_value` 中针对 `Frame::Absolute` 强制 Hold（或 panic）；或
  - 将 `Absolute` 构造为不可变状态，仅提供 `TritWord::absolute_hold()` 工厂函数。

#### P1.4 `ResolutionPolicy::arbitrate` 使用 `.expect()`
- **位置**：`src/meta/domain.rs:43-46`、`src/meta/domain.rs:54-57`
- **代码**：
  ```rust
  let t = inputs.iter().find(|t| t.frame == Frame::Science)
      .expect("invariant: FrameMask guarantees Science presence");
  ```
- **问题**：虽然 `FrameMask` 与输入同步，但**依赖两个独立实现的隐式契约**。如果未来 `FrameMask::from_inputs` 有 bug，或调用者传入空切片/不同步数据，程序会 panic。
- **建议**：返回 `ArbitrationResult::Hold` 或 `Negotiate` 作为安全降级，而非 panic。

#### P1.5 网络发现丢失 frame 信息
- **位置**：`src/net/discovery.rs:135-143`
- **代码**：
  ```rust
  fn extract_frame_from_state(state: &str) -> crate::frame::Frame {
      let _ = state;
      crate::frame::Frame::Meta
  }
  ```
- **问题**：种子节点发现后，对方的 frame 信息被完全丢弃，统一注册为 `Frame::Meta`。
- **影响**：跨帧冲突检测、interference 计算、PLL 都会基于错误的 Meta 帧执行，导致后续 coupling/negotiation 决策失真。
- **建议**：扩展 `HeartbeatPayload` 携带 frame；或在 seed 地址中携带 frame 元数据。

#### P1.6 TCP 服务器中 `RESONATE_REQ` 目标选择非确定性
- **位置**：`src/net/tcp_server.rs:157-162`
- **代码**：
  ```rust
  let target_id = bus.nodes.keys().find(|id| *id != sender).cloned();
  ```
- **问题**：使用 `HashMap` 的任意顺序选择“第一个非 sender”节点。在多节点场景下，同样一条 `RESONATE_REQ` 可能发给不同节点，导致不可复现的耦合拓扑。
- **建议**：在 `ResonateReq` 中显式携带 `target_id`，或要求消息中指定目标。

#### P1.7 `negotiate` 同帧时无条件 `commit_true`
- **位置**：`src/net/negotiate.rs:38-44`
- **代码**：
  ```rust
  let conflict_resolution = if has_cross_frame { "hold" } else { "commit_true" };
  ```
- **问题**：即使所有节点 phase 都接近 0（强 False），只要同帧就返回 `commit_true`。这与 ternary algebra 的语义不一致。
- **建议**：使用 consensus_phase 与阈值/commitment 映射来决定 commit_true / commit_false / hold。

#### P1.8 `node.rs` CLI 与 `coupling.rs` 中存在 unwrap
- **位置**：`src/bin/node.rs:199, 231`、`src/net/coupling.rs:72`
- **问题**：虽然 `node_id` 在启动时已被注册，但 unwrap 仍是不必要的 panic 来源。
- **建议**：替换为 `if let Some(node) = ...` 并给出清晰错误。

#### P1.9 `Domain::Custom` 规则被 `ResolutionPolicy` 完全忽略
- **位置**：`src/meta/domain.rs:64-66`、`src/meta/rules.rs:38-78`、`src/bin/sandbox.rs:156-163`
- **问题**：`ResolutionPolicy::arbitrate` 对任何 `Domain::Custom(_)` 直接返回 `Negotiate`，从不调用 `RuleLoader::apply`。`JsonRuleLoader` 和 `CustomRule` 只在单元/property test 中被使用。
- **影响**：安全关键自定义域（如 `Custom("nuclear")`）无法表达“Science 优先，否则 safe fallback”；sandbox 接受 `"Custom(chemistry)"` 却不会加载 `chemistry.json`。
- **建议**：
  - 在 `ResolutionPolicy` 中保存 `Option<CustomRule>`；
  - `Domain::Custom` 分支调用 `RuleLoader::apply`；
  - sandbox 根据 `Custom(name)` 自动加载对应规则文件。

#### P1.10 同帧多 trit 冲突时输出顺序依赖
- **位置**：`src/meta/domain.rs:43-47`（Physical/Engineering 取第一个 Science）、`:54-57`（MedicalEthics 取第一个 Individual）、`:71-73`（General 同帧取 `inputs[0]`）
- **问题**：当多个同帧 trit 值互相冲突（如两个 Science trit，一个 True 一个 False）时，输出完全取决于输入切片的顺序。如果输入来自 `HashMap`、网络或异步收集，结果可能非确定性。
- **建议**：定义明确的 tie-breaker（phase 加权多数、最安全值、或显式返回 `Negotiate`）。

#### P1.11 网络层冲突检测未接入 `MetaMonitor` / `ResolutionPolicy` / `SafeFallback`
- **位置**：`src/net/coupling.rs:23-61`、`src/net/negotiate.rs:14-67`、`src/net/node.rs:78-84`
- **问题**：
  - `ResonanceBus::handle_resonate_req` 检测到 destructive interference 后，只在 ACK 字符串中写 `"hold"`，不调用 `node.enter_hold()`，不记录 `MetaInterrupt`，也不调用 policy/safe-fallback；
  - `ResonanceBus::negotiate` 自己决定 `hold`/`commit_true`，完全不使用 `ResolutionPolicy`；
  - 网络路径中 `SafeFallback` 从未被使用。
- **影响**：分布式运行时，核心的 safety/policy 引擎被完全绕过。
- **建议**：在 coupling/negotiate 中集成 `ResolutionPolicy`、`MetaMonitor` 和 `SafeFallback`，与 sandbox 路径一致。

#### P1.12 Sandbox 用 policy 结果覆盖 TAND cascade 结果
- **位置**：`src/bin/sandbox.rs:179-214`
- **问题**：sandbox 先计算 TAND cascade 得到 `current`，然后独立调用 `policy.arbitrate(&trits)`。如果 policy 返回 `Commit`/`Preserve`，则 cascade 结果被丢弃。
- **影响**：例如 `General` 域同帧场景中，两个输入 `True` 和 `False` 的 TAND 应为 `False`，但 policy 会取 `inputs[0]` 即 `True`，最终输出与三元逻辑矛盾。
- **建议**：明确代数层与策略层的组合语义；可考虑 policy 仅作用于 cascade 输出的最终仲裁，而不是直接覆盖。

#### P1.13 热路径 `t_and_hot` / `t_or_hot` 在 release build 不强制同帧前提
- **位置**：`src/trit/algebra.rs:66-78`、`src/trit/algebra.rs:102-114`
- **问题**：`debug_assert_eq!(a.frame, b.frame)` 只在 debug build 触发 panic；release build 中若调用者未 precheck，会静默计算出错误结果并污染下游。
- **影响**：公共 API 的安全前提在优化构建中不被执行，违反了 fail-fast 原则。
- **建议**：
  - 将热路径改为 `pub(crate)` 并仅由已验证的调用者使用；或
  - 返回 `Result<TritWord, MetaInterrupt>`；或
  - 在 release build 中也执行运行时检查（若零分配是硬性要求，则必须在文档中明确标注“错误调用为 UB-adjacent”）。

#### P1.14 `TritWord` 字段全公开可变，破坏不变量
- **位置**：`src/trit/mod.rs:21-25`、`src/trit/mod.rs:71-85`
- **问题**：`value`、`phase`、`frame` 均为 `pub`，且新增 `set_value`/`set_phase_direct` 可直接绕过 invariant。调用者可以构造 `Frame::Absolute + True` 或 `Frame::Meta + 任意值` 等非法状态。
- **影响**：核心不变量无法通过类型系统保证，只能依赖调用者自觉遵守。
- **建议**：
  - 将字段改为非公开，通过构造函数和受控 setter 强制 invariant；
  - 或至少为 `Absolute`/`Meta` 帧提供专门构造器，并限制 `set_value`/`set_phase_direct` 为 `pub(crate)`。

#### P1.15 Gatekeeper 默认关闭，文档却声称“M8 Byzantine Fault Tolerance 完成”
- **位置**：`src/net/tcp_server.rs:50`、`src/net/bus.rs:47-56`、`src/net/gate.rs:1-24`、`CLAUDE.md:80`
- **问题**：`TcpNodeServer::new()` 创建的 `ResonanceBus` 不带 gatekeeper；`ResonanceBus::new()` 默认 `gatekeeper: None`。文档却将其描述为已完成的 BFT 功能。
- **影响**：默认部署等同于无 Byzantine 防护；只有在显式调用 `with_default_gatekeeper()` 时才启用。
- **建议**：
  - 默认启用 gatekeeper（opt-out 需显式参数）；
  - 重命名或调整文档：当前实现只是输入验证 + 速率限制，不是完整 BFT（无签名、无 quorum、无视 equivocation）。

#### P1.16 分布式 decouple 是单向的
- **位置**：`src/net/tcp_server.rs:178-181`、`src/net/coupling.rs:85-106`
- **问题**：`DecoupleAck` 被接收后只写入 message_log，不更新本地节点状态。发送 decouple 的节点回到 Sovereign，但对端节点仍认为自己在 Coupled。
- **影响**：分布式状态下的耦合视图不一致，可能导致 split-brain 误判或持续错误的 phase 同步。
- **建议**：实现 `handle_decouple_ack`，将接收方节点也 transition 回 `Sovereign`。

#### P1.17 协议为明文 JSON，无机密性与完整性保护
- **位置**：`src/net/frame_codec.rs`、`src/net/tcp_server.rs`
- **问题**：所有消息以明文 JSON 通过 TCP 传输，无 TLS/mTLS，也无消息级签名。
- **影响**：在不可信网络中易受中间人篡改、重放、窃听。
- **建议**：在部署指南中明确“仅用于可信局域网”；若需跨公网，引入 TLS/mTLS 或 framed + signed messages。

#### P1.18 `RESONATE_REQ` 可被用于 DoS 淹没未限制并发连接
- **位置**：`src/net/tcp_server.rs:72-94`、`src/net/frame_codec.rs:27-58`
- **问题**：服务器对每个连接都 spawn 一个 task，没有并发数上限；每个连接可分配最大 1 MiB 帧缓冲。
- **影响**：连接洪泛 + 大帧可导致内存耗尽。
- **建议**：
  - 添加 `tokio::sync::Semaphore` 限制并发连接数；
  - 考虑默认更小的最大帧尺寸或每连接内存预算。

---

### P2 — 中优先级改进

#### P2.1 验证逻辑在 `message.rs` 与 `gate.rs` 中重复
- **位置**：`src/net/message.rs:199-314`、`src/net/gate.rs:239-371`
- **问题**：phase、frame、consistency 检查几乎相同，维护时容易漂移。
- **建议**：让 `Message::validate_*` 成为唯一真相源，`ByzantineGatekeeper` 调用这些方法。

#### P2.2 缺少 fuzz 测试
- **位置**：`tests/`
- **问题**：JSON 反序列化（`ScenarioInput`、`Message`、`CustomRule`）和 TCP 帧解析是攻击面，但没有 `cargo-fuzz` 目标。
- **建议**：为 `ScenarioInput` 和 `Message` 各添加一个 fuzz target。

#### P2.3 `tracing_init::init()` 可被重复调用并 panic
- **位置**：`src/tracing_init.rs:15-39`
- **问题**：`tracing_subscriber::fmt().init()` 在第二次调用时会 panic。`tracing_init` 被多个 binary（sandbox、node、dhat-profile）独立调用是安全的，但如果被测试或库代码多次调用会崩溃。
- **建议**：使用 `Once` 或 `try_init` 包装。

#### P2.4 `HarmonicClock` 与 `FrameRegistry` 尚未融入主路径
- **位置**：`src/clock.rs`、`src/frame/mod.rs:40-65`
- **问题**：两者都有完整实现和测试，但在 sandbox、node、policy 中均未被使用，属于“孤儿抽象”。
- **建议**：明确路线图：要么在 0.2.0 中接入，要么暂时移到 `examples/` 或标记为 `#[doc(hidden)]`。

#### P2.5 没有可复现的 cargo-audit 结果
- **位置**：`audit_log/03_cargo_audit.txt`
- **问题**：由于网络限制，无法拉取 RustSec advisory-db。
- **建议**：在 CI 中启用 `cargo audit`，并把结果作为 release 阻塞条件。

#### P2.6 性能声明缺少可复现数据
- **位置**：`docs/reports/performance-validation.md`、README
- **问题**：README 提到 “~3ns/op hot path, ~333M ops/s theoretical, 658K-1.02M end-to-end TPS”，但仓库中没有已提交的 benchmark 输出或 CI 性能回归阈值。
- **建议**：在 CI 中保存 `target/criterion/` 报告，或至少提交一次基准运行的 `report/` 摘要。

#### P2.7 `TritValue::from(i8)` 静默吞掉所有非 ±1 值
- **位置**：`src/trit/value.rs:84-91`
- **问题**：`From<i8>` 把 `2`、`-2`、`127` 等任何非 ±1 值都折叠为 `Hold`，而 `from_i8_strict` 才是安全路径。
- **建议**：将 `From<i8>` 标记为 deprecated，或改为仅接受 `-1, 0, 1` 并 panic（破坏性变更，适合 0.2.0）。

#### P2.8 `Phase::quantize` 接受任意 epsilon，包括负值
- **位置**：`src/trit/phase.rs:60-71`
- **问题**：`quantize` 是公共 API，但 `epsilon` 可为负或极大，导致 snapping 行为异常。
- **建议**：在 `quantize` 中 debug_assert 或返回 `Result`，要求 `epsilon > 0.0 && epsilon.is_finite()`。

#### P2.9 网络层无 Coupling 超时
- **位置**：`src/net/node.rs:63-75`
- **问题**：节点进入 `Coupling` 后，若 ACK 丢失，会永远停留在 `Coupling`，没有 ADR-004 中提到的 5 秒 fallback。
- **建议**：在 `Node` 中记录 `coupling_since` 并添加后台清理任务。

#### P2.10 `NEGOTIATE` 检测到冲突但不转换节点状态
- **位置**：`src/net/negotiate.rs:14-67`、`src/net/tcp_server.rs:183-198`
- **问题**：cross-frame negotiation 返回 `Hold` 值，但所有参与节点的 `NodeState` 保持原状，`Hold` 状态实际上不可达。
- **建议**：在冲突时将参与者节点设为 `Hold` 并记录 `MetaInterrupt`。

#### P2.11 PLL 使用线性相位差而非环形距离
- **位置**：`src/net/pll.rs:43-59`
- **问题**：`error = peer - local`。当 local=0.9、peer=0.1 时，error=-0.8，会“绕远路”拉回，而不是走 0.2 的短路径。
- **建议**：文档明确 phase 为线性，或实现 circular distance `min(|d|, 1-|d|)`。

#### P2.12 `PllController` 字段公开，可被设为 NaN/Inf 导致 panic
- **位置**：`src/net/pll.rs:7-15`、`src/net/node.rs:97-99`
- **问题**：`kp`、`deadband`、`max_correction` 为 `pub`，外部可设置非法值；`Node::adjust_phase` 中 `clamp` 遇到 NaN 会 panic。
- **建议**：字段私有化，提供带验证的 setter；`with_params` 中验证参数。

#### P2.13 无后台心跳任务
- **位置**：`src/net/discovery.rs:49-128`
- **问题**：bootstrap 只发一次 HEARTBEAT，之后若没有用户手动发心跳，节点会被标记为 stale。
- **建议**：在 node 启动后 spawn 周期性心跳 emitter。

#### P2.14 `DecoupleReq`、`DecoupleAck`、`Heartbeat` 未统一走 `push_log`
- **位置**：`src/net/coupling.rs:102-104`、`src/net/tcp_server.rs:178-207`
- **问题**：这些消息直接 `message_log.push_back`，绕过 `ResonanceBus::push_log`，导致 gatekeeper 的 per-peer log cap 和 accounting 不准确。
- **建议**：所有 log 写入都通过 `push_log`。

#### P2.15 `parse_flag_*` 对无效输入静默回退到默认值
- **位置**：`src/bin/node.rs:281-303`
- **问题**：`--phase foo` 会被解析失败并回退到 `0.5`，`--port abc` 回退到 `9000`。
- **建议**：解析失败时打印错误并退出。

#### P2.16 `ResonateReq.history` 长度未在 gatekeeper / `validate_all` 中检查
- **位置**：`src/net/message.rs:341-352`、`src/net/gate.rs:239-294`
- **问题**：`ResonateReq::validate_history` 存在但未被 gatekeeper 或 `Message::validate_all` 调用。
- **建议**：在 `validate_all` 中调用 `validate_history`。

#### P2.17 ADR-001 缺少具体真值表
- **位置**：`docs/adr/001-ternary-logic.md`
- **问题**：真值表只存在于 `docs/archive/technical-whitepaper.md:198-218`，而 ADR-001 被引用为权威来源却未列出表格。
- **建议**：将真值表移入 ADR-001，或在 ADR-001 中明确引用 whitepaper。

#### P2.18 集成测试 pipeline 与真实 CLI pipeline 不一致
- **位置**：`tests/integration_test.rs:82-114`
- **问题**：`run_pipeline` 没有调用 `SafeFallback::guard`，而 `src/bin/sandbox.rs:200-208` 会调用。两者对危险域的决策可能不同。
- **建议**：让 `run_pipeline` 完整复现 CLI 路径，或把 sandbox 主逻辑提取为可复用库函数。

#### P2.19 缺少 CLI 集成测试
- **位置**：`tests/`、`Cargo.toml:30-31`
- **问题**：`assert_fs` / `predicates` 已在 dev-dependencies 中，但没有任何测试实际执行 `trit-sandbox` 二进制并断言输出。
- **建议**：使用 `assert_cmd` 添加端到端 CLI 测试，覆盖有效/无效路径、畸形 JSON、每个 scenario。

#### P2.20 `BinaryBaseline` 未被任何生产或测试路径使用
- **位置**：`src/baseline/mod.rs`
- **问题**：README 强调“67% 场景中 binary baseline 产生误导”，但 `BinaryBaseline` 从未被 sandbox、集成测试或 CI 调用。
- **建议**：在 sandbox 中添加 `--baseline` 模式，或移除该模块并在文档中说明。

#### P2.21 `JsonRuleLoader::load` 读取任意路径，无路径验证
- **位置**：`src/meta/rules.rs:87-96`
- **问题**：`RuleLoader::load` 接受任意 `AsRef<Path>`，未检查是否在允许的目录内。
- **建议**：与 sandbox 类似，对规则文件路径做 canonicalize + 目录白名单校验。

#### P2.22 `Custom(...)` 域畸形输入被静默降级
- **位置**：`src/bin/sandbox.rs:156-165`
- **问题**：`Custom(chemistry`（缺少右括号）会被解析为 `Domain::Custom("unknown")`。
- **建议**：严格验证 `Custom(...)` 格式，不匹配时报错退出。

#### P2.23 CI release build 会失败于依赖 debug_assert 的测试
- **位置**：`src/trit/algebra.rs:175-180`、`.github/workflows/ci.yml:81-82`
- **问题**：`tand_hot_different_frame_panics_in_debug` 依赖 `debug_assert_eq!`，在 release build 中该断言被剥离，测试会失败。
- **建议**：将该测试标记为 `#[cfg(debug_assertions)]`，或改为测试 `Result`/`panic` 行为。

---

### P3 — 低优先级 / 技术债

#### P3.1 `TritWord::set_phase` 与 `set_phase_direct` 并存
- **位置**：`src/trit/mod.rs:77-85`
- **问题**：`set_phase` 会 clamp，`set_phase_direct` 不会。公共 API 提供了绕过 invariant 的通道。
- **建议**：限制 `set_phase_direct` 为 `pub(crate)`，或移除。

#### P3.2 `BinaryBaseline::compare` 逻辑冗余
- **位置**：`src/baseline/mod.rs:71-87`
- **问题**：`conflicts` 条件中 `ternary.value != binary.value && ternary.value != TritValue::Hold` 已被前面的 `ternary.value == TritValue::Hold` 覆盖部分分支，可读性可优化。
- **建议**：拆分为明确的 match 分支。

#### P3.3 测试辅助函数重复
- **位置**：`tests/multi_node_test.rs`、`tests/partition_test.rs`、`tests/byzantine_test.rs`
- **问题**：`spawn_server`、`resonate_full_handshake` 等辅助函数在多个文件中复制粘贴。
- **建议**：提取到 `tests/common/mod.rs`。

#### P3.4 `dhat_profile` 可执行文件未验证零分配声明
- **位置**：`src/bin/dhat_profile.rs`
- **问题**：代码打印“Profiling complete”，但不自动断言热路径零分配。需要人工用 dhat-viewer 查看。
- **建议**：在二进制中读取 dhat 输出并断言关键指标，或改为集成测试。

#### P3.5 proptest 覆盖存在缺口
- **位置**：`tests/proptest.rs`
- **问题**：缺少：16 组合真值表穷尽测试；cross-frame 结果 frame 必须为 `Meta` 的验证；`set_value`/`set_phase_direct` 误用面文档化测试；`quantize` 边界/负 epsilon 测试；`THOLD`/`TSENSE` proptest。
- **建议**：逐步补全上述属性测试。

#### P3.6 CI unsafe-code grep 范围不足
- **位置**：`.github/workflows/ci.yml:31-34`
- **问题**：`grep -r "unsafe" src/` 只扫描 `src/`，未覆盖 `tests/`、`benches/`。
- **建议**：扩展到 `src/ tests/ benches/`，或依赖 crate-level `#![forbid(unsafe_code)]` 自动覆盖。

---

## 3. 风险矩阵

| 风险 | 可能性 | 影响 | 等级 | 状态 |
|------|:------:|:----:|:----:|------|
| 本地/CI 编译 OOM | 高 | 高 | 🔴 P0 | 已确认 |
| 网络身份可伪造 / 明文协议 | 高 | 高 | 🔴 P0 | 已确认 |
| 场景 expected_behavior 与实际输出不一致 | 高 | 高 | 🔴 P0 | 已确认 |
| 非法 phase 被静默修复 | 中 | 高 | 🟠 P1 | 待修复 |
| Absolute 帧不变量被绕过 | 中 | 高 | 🟠 P1 | 待修复 |
| arbitrate panic | 低 | 高 | 🟠 P1 | 待修复 |
| 网络发现丢失 frame | 高 | 中 | 🟠 P1 | 待修复 |
| resonate 目标非确定性 | 中 | 中 | 🟠 P1 | 待修复 |
| negotiate 同帧无条件 true | 中 | 中 | 🟠 P1 | 待修复 |
| Domain::Custom 规则被忽略 | 中 | 高 | 🟠 P1 | 待修复 |
| 热路径 release build 不检查前提 | 中 | 高 | 🟠 P1 | 待修复 |
| TritWord 字段公开可变 | 高 | 高 | 🟠 P1 | 待修复 |
| Gatekeeper 默认关闭 / BFT 过度宣传 | 高 | 高 | 🟠 P1 | 待修复 |
| 分布式 decouple 单向 | 中 | 高 | 🟠 P1 | 待修复 |
| 无并发连接限制 | 中 | 中 | 🟡 P2 | 待修复 |
| 缺少 fuzz | 中 | 中 | 🟡 P2 | 待规划 |
| 重复验证逻辑 | 高 | 低 | 🟡 P2 | 待重构 |
| tracing 重复 init panic | 低 | 中 | 🟡 P2 | 待修复 |
| 无后台心跳 | 高 | 中 | 🟡 P2 | 待修复 |
| Coupling timeout 缺失 | 中 | 中 | 🟡 P2 | 待修复 |
| PLL 线性相位差 | 中 | 低 | 🟡 P2 | 待修复 |
| 集成测试与 CLI pipeline 不一致 | 高 | 中 | 🟡 P2 | 待修复 |
| 缺少 CLI 集成测试 | 高 | 中 | 🟡 P2 | 待规划 |
| BinaryBaseline 未被使用 | 中 | 低 | 🟡 P2 | 待处理 |
| CI release build debug_assert 测试 | 高 | 中 | 🟡 P2 | 待修复 |

---

## 4. 改进建议路线图

### 立即（0.1.1 / 本周）
1. 拆分 `cargo test` 与 `cargo test --all-features`；让 CI 的常规 test job 不再启用 dhat；修复 release build 中依赖 `debug_assert` 的测试（P2.23）。
2. 修正 `scenarios/` 中 `expected_behavior` 错误值（医疗场景、general_negotiation），并添加自动 scenario runner 作为 CI job。
3. 将 `src/bin/sandbox.rs:173` 的 `unwrap_or(Frame::Meta)` 改为错误退出；修复 `Custom(...)` 畸形输入静默降级（P2.22）。
4. 修复 `src/bin/node.rs` 与 `src/net/coupling.rs` 中的 unwrap。
5. 在 README/CONTRIBUTING 中记录 Windows 页面文件建议。
6. 明确文档声明：网络层当前仅适用于可信局域网，不具备生产级 BFT / 安全。

### 短期（0.2.0 / 1–2 个月）
1. 重新设计 `Phase` 构造函数：默认使用 `try_new`，或 debug build panic。
2. 在类型层强制 `Absolute` 帧只能为 Hold。
3. 封装 `TritWord` 字段，限制 `set_value`/`set_phase_direct` 为内部使用。
4. 移除 `ResolutionPolicy::arbitrate` 中的 `.expect()`，返回安全降级。
5. 修复 `Domain::Custom` 规则加载与应用。
6. 修复同帧多 trit 冲突的顺序依赖，定义 tie-breaker。
7. 修复 sandbox 中 policy 覆盖 cascade 的语义不一致。
8. 扩展 heartbeat 协议携带 frame，修复 discovery 的 frame 丢失。
9. 在 `ResonateReq` 中显式指定 target，消除非确定性。
10. 基于 consensus_phase + commitment 重新实现 `negotiate`。
11. 默认启用 gatekeeper，重命名为 `MessageValidator`/`SafetyGate` 或调整文档。
12. 实现双向分布式 decouple。
13. 用 `Once` 保护 `tracing_init`。

### 中期（0.3.0 / 3–6 个月）
1. 引入 mTLS 或消息签名，将节点身份绑定到连接/密钥。
2. 引入 `cargo-fuzz` 目标（ScenarioInput、Message、TCP frame）。
3. 统一 `message.rs` 与 `gate.rs` 的验证逻辑。
4. 添加并发连接限制与每连接内存预算。
5. 添加后台心跳任务与 Coupling timeout。
6. 接入 `HarmonicClock` 到 node tick 与 sandbox sampling。
7. 在 CI 中运行 `cargo audit` 与性能回归检查。
8. 编写形式化规范文档（或 Coq/Lean 原型），作为 0.2.x 后核心代数的对照。

---

## 5. 分模块详细评估

### 5.1 `src/trit/` — 核心代数
**评级：A-**
- `TritValue` 使用 LUT 实现分支消除，测试覆盖完整。
- `Phase` 的 quantize 与 epsilon 设计合理，处理了浮点漂移。
- `TernaryAlgebra` 热/冷路径分离清晰，cross-frame 始终生成 `MetaInterrupt`。
- 主要风险：P1.13（热路径 release build 不检查同帧）、P1.14（`TritWord` 字段公开可变）、P2.7（`From<i8>` 静默失败）、P2.8（`quantize` epsilon 未验证）、P3.5（proptest 缺口）。

### 5.2 `src/frame/` — 决策域
**评级：B+**
- `Frame` enum 简洁，Display/FromStr roundtrip 有 proptest。
- `FrameMask` 使用 bit mask 做 O(1) 查询，是好优化。
- `FrameRegistry` 目前未被使用，建议 P2.4。

### 5.3 `src/meta/` — 元监控与策略
**评级：B**
- `ResolutionPolicy` 的 domain 规则结构清晰，但 `Domain::Custom` 是 stub。
- `SafeFallback` 的 IEC 61508 语义明确，dangerous custom domains 可扩展。
- `MetaMonitor` 的 capped log 防止内存无限增长。
- 主要风险：P1.3（Absolute invariant）、P1.4（expect panic）、P1.9（Custom 规则被忽略）、P1.10（同帧冲突顺序依赖）、P1.11（网络层未接入 policy）、P2.1（验证重复）、P2.21（RuleLoader 路径未验证）。

### 5.4 `src/clock/` — 相位振荡器
**评级：C+**
- 实现正确，但未被任何核心路径使用。
- `HarmonicClock` 缺少输入验证与 `t` 的周期归约。
- 建议明确路线图，避免“看起来已完成但实际上不工作”的错觉。

### 5.5 `src/sandbox/` — CLI 模拟
**评级：B-**
- 输入验证较全面：路径遍历防护、JSON 大小限制、signal 数量限制、phase/value/frame 校验。
- 问题：P0.3（expected_behavior 不一致）、P1.2（未知 frame 静默回退）、P1.12（policy 覆盖 cascade）、P2.3（tracing init）、P2.22（Custom 畸形输入）、P2.18（测试 pipeline 不一致）。

### 5.6 `src/net/` — 分布式协议
**评级：C+**
- 协议设计（length-prefix JSON frame、heartbeat、PLL、gatekeeper）在原型层面合理。
- 已实现部分 DoS 防护（MAX_FRAME_SIZE、rate limit、per-peer log cap）。
- 重大风险：P0.2（身份可伪造）、P1.5（discovery 丢失 frame）、P1.6（目标选择非确定性）、P1.7（negotiate commit_true）、P1.11（policy/safe-fallback 被绕过）、P1.15（gatekeeper 默认关闭 / BFT 过度宣传）、P1.16（分布式 decouple 单向）、P1.18（无并发连接限制）、P2.9（Coupling timeout）、P2.10（NEGOTIATE 不改变状态）、P2.11（PLL 线性相位）、P2.12（PllController 字段公开）、P2.13（无后台心跳）、P2.14（push_log 不一致）。

### 5.7 `tests/` — 测试策略
**评级：B+**
- 单元测试、集成测试、proptest、多节点 TCP 测试、分区测试、Byzantine 测试、并发 stress 测试一应俱全。
- proptest 覆盖 15 层不变量，非常扎实。
- 重大扣分：P0.1（当前环境无法编译）、P0.3（expected_behavior 未验证）、P2.18（pipeline 与 CLI 不一致）、P2.19（缺少 CLI 集成测试）、P2.20（BinaryBaseline 未使用）、P2.23（release build debug_assert 测试）。

### 5.8 CI/CD
**评级：C+**
- GitHub Actions 配置了 fmt、clippy、unsafe 检查、test、multi-node test、partition test、bench、release build。
- 重大扣分：P0.1/P2.23（CI 在当前状态下不是全绿）、P0.3（缺少 scenario-validation）、P3.6（unsafe grep 范围不足）。
- 缺失：cargo-audit、coverage report、性能回归阈值、内存受限环境下的测试策略、Docker Compose 自动健康检查。

### 5.9 依赖与供应链
**评级：A**
- 仅 7 个运行时依赖（serde、serde_json、chrono、uuid、thiserror、tracing、tracing-subscriber），均为生态基石级库。
- 无可疑或已知 CVE 依赖（待 cargo-audit 确认）。

---

## 6. 结论

Trit-Core 是一个**有学术价值、工程认真、文档完备**的 MVP。它的核心创新（用 Hold 状态保留不可调和冲突）通过 Rust 类型与 property-based testing 得到了较好表达。

然而，从 CTO 视角看，它目前仍处于**“可演示的原型”**阶段，而非**“可部署的系统”**。三个最大障碍是：
1. **本地可构建/可测试性崩塌**（P0.1）：当前环境无法编译完整测试套件，CI 在当前树也不是全绿；
2. **网络层安全缺失**（P0.2）：明文 JSON、无身份认证、gatekeeper 默认关闭，导致任何同网络主机都可伪造节点身份；
3. **核心验证资产与代码行为不一致**（P0.3）：`scenarios/` 中的 `expected_behavior` 是死代码，且多个医疗/通用场景的期望值与实际 CLI 输出矛盾。

此外，关键路径上的静默降级（P1.1、P1.2）、核心不变量仅靠约定维持（P1.3、P1.14）、分布式协议语义未闭合（P1.5–P1.7、P1.11、P1.16）以及 BFT 声明过度（P1.15）都是必须正视的问题。

**我的建议**：
1. **立即（0.1.1）**：修复 P0.1、P0.2、P0.3，让项目重新“可构建、可测试、可验证”；明确网络层仅适用于可信局域网。
2. **短期（0.2.0）**：集中解决 P1 风险，把核心不变量从“约定”推进到“类型/编译期保证”，并默认启用 gatekeeper。
3. **中期（0.3.0）**：引入 mTLS/消息签名、`cargo-fuzz`、形式化规范、性能回归与真正的 BFT 证明。

如果团队能按此路线图推进，Trit-Core 有望从一个优秀的研究 MVP 成长为可信的 AI 安全基础设施组件。否则，它将继续是一份“看起来完成度很高，但无法在生产环境中信任”的演示代码。
