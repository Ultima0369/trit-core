# Consistency Deep Audit — Doc System Update Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Fix all factual errors, add missing content, and update stale references across 7 documentation files to match the current codebase (305 tests, M0-M9 complete).

**Architecture:** Documentation-only edits. Each task modifies one file independently. No code changes. Source of truth is the actual Rust source code in `src/`. Edits use exact string replacement via the Edit tool.

**Tech Stack:** Markdown, git

---

### Task 1: Fix README.md

**Files:**
- Modify: `README.md`

- [ ] **Step 1: Update test count from 298 to 305**

Old: `**Tests**: 298 passing, 0 failures`
New: `**Tests**: 305 passing, 0 failures`

- [ ] **Step 2: Expand docs/ description in project structure table**

Old:
```
| `docs/` | Architecture Decision Records (ADRs), whitepaper, preprint |
```
New:
```
| `docs/` | Full documentation system (37+ files): ADRs, concepts, usage guides, dev docs, insights, audits, Chinese translations — see [docs/INDEX.md](docs/INDEX.md) |
```

- [ ] **Step 3: Update net/ description to include M8 gatekeeper**

Old:
```
| `src/net/` | Distributed node protocol (M4-M8: TCP, PLL, seed discovery, partition tolerance, Byzantine fault tolerance) |
```
(Already correct — verify)

- [ ] **Step 4: Add dhat-profile to build section**

Add after the distributed node line:
```
# Heap profiling
cargo run --release --bin dhat-profile
```

- [ ] **Step 5: Add M9 to status line**

Old: `**Status**: v0.1.0 — M0–M8 complete, all code milestones delivered`
New: `**Status**: v0.1.0 — M0–M9 complete, all code milestones delivered`

- [ ] **Step 6: Add M9 to milestones table**

Add row after M8:
```
| M9: Concurrency Stress Testing | ✅ Complete |
```

- [ ] **Step 7: Commit**

```bash
git add README.md
git commit -m "docs: update README for M9, 305 tests, dhat-profile binary

Co-Authored-By: Claude <noreply@anthropic.com>"
```

---

### Task 2: Fix CHANGELOG.md

**Files:**
- Modify: `CHANGELOG.md`

- [ ] **Step 1: Fix M8 test count from 298 to 305**

Old: `- Total: 298 tests, 0 failures, 0 warnings, 0 clippy issues.`
New: `- Total: 305 tests, 0 failures, 0 warnings, 0 clippy issues.`

- [ ] **Step 2: Add M9 entry after M8 section**

Insert after the M8 `### Added` block and before `### Changed`:

```markdown
### Added (M9)
- Multi-threaded concurrency stress testing: concurrent bus operations under load.
- Thread-safe ResonanceBus access patterns validated.
- Concurrency test suite (6 tests) covering race conditions and deadlock prevention.
```

- [ ] **Step 3: Shorten verbose performance note in Known Limitations**

Old:
```
- Performance target (10,000 TPS) validated at both micro-benchmark and end-to-end level; 29 criterion benchmarks across 9 groups; 10,000 TPS target exceeded by 65-101x (see docs/performance-validation.md).
```
New:
```
- Performance validated: 29 criterion benchmarks across 9 groups; 10,000 TPS target exceeded by 65-101x (see docs/performance-validation.md).
```

- [ ] **Step 4: Commit**

```bash
git add CHANGELOG.md
git commit -m "docs: update CHANGELOG for M9, 305 tests

Co-Authored-By: Claude <noreply@anthropic.com>"
```

---

### Task 3: Fix CLAUDE.md

**Files:**
- Modify: `CLAUDE.md`

- [ ] **Step 1: Fix TritValue description — three states → four states**

Old:
```
- **`TritValue`**: Three discrete states: `True` (+1), `Hold` (0), `False` (-1).
```
New:
```
- **`TritValue`**: Four discrete states: `True` (+1), `Hold` (0), `False` (-1), `Unknown` (⊥ — out-of-distribution, propagates through TAND/TOR).
```

- [ ] **Step 2: Fix Phase behavior description**

Old:
```
- **`Phase`**: Continuous tendency 0.0–1.0 (0.5 = neutral). Wraps `f64` with bounds-checking.
```
New:
```
- **`Phase`**: Continuous tendency 0.0–1.0 (0.5 = neutral). Wraps `f64` with bounds-clamping and NaN/Inf protection (logs warning via tracing). Use `try_new()` for strict validation that returns `Err`.
```

- [ ] **Step 3: Add gate.rs to net/ module listing**

Old:
```
- **`tcp_server/`** — TCP node server dispatching messages to ResonanceBus (M5)
```
New:
```
- **`tcp_server/`** — TCP node server dispatching messages to ResonanceBus (M5)
- **`gate/`** — Byzantine fault tolerance gatekeeper with 7 safety checks (M8)
```

- [ ] **Step 4: Add dhat_profile.rs to binary listing**

Add after the sandbox binary mention:
```
- `src/bin/dhat_profile.rs`: dhat heap profiling binary for allocation analysis
```

- [ ] **Step 5: Fix Known Limitations — remove stale benchmark status**

Old:
```
- Performance target (10,000 TPS) validated at micro-benchmark and end-to-end level (~3ns/op hot path, ~333M ops/s theoretical, 658K-1.02M end-to-end TPS); see `docs/performance-validation.md`.
```
(Already updated — verify)

- [ ] **Step 6: Commit**

```bash
git add CLAUDE.md
git commit -m "docs: fix CLAUDE.md — 4-state TritValue, Phase clamping, gate/dhat entries

Co-Authored-By: Claude <noreply@anthropic.com>"
```

---

### Task 4: Fix docs/api.md

**Files:**
- Modify: `docs/api.md`

- [ ] **Step 1: Fix Phase::new() description**

Old (line 35):
```
- `fn new(v: f64) -> Phase` (panics if out of [0.0, 1.0])
```
New:
```
- `fn new(v: f64) -> Phase` — clamps out-of-range values with tracing warning; NaN/Inf → NEUTRAL (0.5)
- `fn try_new(v: f64) -> Result<Phase, String>` — strict constructor, returns Err for invalid values
```

- [ ] **Step 2: Add Commitment enum and TritWord::unknown()**

After Phase methods, add:
```
### `Commitment` (enum)
```rust
pub enum Commitment {
    TowardTrue,
    TowardFalse,
    Neutral,
}
```
```

In TritWord constructors, add:
```
- `fn unknown(frame: Frame) -> TritWord` — out-of-distribution trit (⊥ state)
```

- [ ] **Step 3: Add hot path API to TernaryAlgebra**

After `t_sense`, add:
```
- `fn precheck_same_frame(a: &TritWord, b: &TritWord) -> bool` — O(1) frame equality check
- `fn t_and_hot(a: &TritWord, b: &TritWord) -> TritWord` — hot-path TAND (requires same frame, debug_assert)
- `fn t_or_hot(a: &TritWord, b: &TritWord) -> TritWord` — hot-path TOR (requires same frame, debug_assert)
```

- [ ] **Step 4: Fix SafeFallback API**

Old:
```
Methods:
- `fn new() -> SafeFallback`
- `fn register_dangerous(&mut self, domain: &str)` — register a domain as dangerous
- `fn is_dangerous(&self, domain: &str) -> bool`
- `fn is_enabled(&self) -> bool`
- `fn set_enabled(&mut self, enabled: bool)`
- `fn guard(&self, result: &TritWord, interrupts: &[MetaInterrupt], domain: &Domain) -> TritWord`
```
New:
```
Fields:
- `pub dangerous_custom_domains: Vec<String>` — custom domains requiring safe fallback
- `pub enabled: bool` — whether SafeFallback is active (default true)

Methods:
- `fn new() -> SafeFallback` — pre-registers chemistry, genetics, structural, nuclear, pharmaceutical
- `fn register_dangerous(&mut self, domain: &str)` — register a custom domain as dangerous
- `fn is_dangerous(&self, domain: &Domain) -> bool` — Physical/Engineering always dangerous; MedicalEthics/ValueJudgment/General never
- `fn guard(&self, domain: &Domain, result: &TritWord, interrupt_count: usize) -> (TritWord, Option<MetaInterrupt>)` — forces False when domain is dangerous, result is Hold/Unknown, and interrupt_count > 0
```

- [ ] **Step 5: Fix CustomRule struct**

Old:
```
pub struct CustomRule {
    pub name: String,
    pub domain: String,
    pub priority_frame: String,
    pub fallback_policy: String, // "hold" | "safe_fallback" | "negotiate"
}
```
New:
```
pub struct CustomRule {
    pub name: String,
    pub priority_frame: Option<String>,
    pub allow_forced_collapse: bool,
    pub fallback: String, // "hold" | "negotiate" | "commit_first" | "safe_fallback"
}
```

- [ ] **Step 6: Fix RuleLoader trait**

Old:
```
pub trait RuleLoader {
    fn load(&self, source: &str) -> Result<Vec<CustomRule>, String>;
}
```
New:
```
pub trait RuleLoader {
    type Error: std::fmt::Display;
    fn load<P: AsRef<Path>>(path: P) -> Result<CustomRule, Self::Error>;
    fn load_json(json: &str) -> Result<CustomRule, Self::Error>;
    fn apply(rule: &CustomRule, inputs: &[TritWord]) -> ArbitrationResult;
}
```

- [ ] **Step 7: Add Frame::from_str()**

In §2 Frame section, add after the enum definition:
```
Implements `Display`, `FromStr`, `Clone`, `Debug`, `PartialEq`, `Eq`, `Hash`.
```

- [ ] **Step 8: Update §5 title and add M7/M8 APIs**

Old title: `## 5. `trit_core::net` — Distributed Protocol (M4-M6)`
New title: `## 5. `trit_core::net` — Distributed Protocol (M4-M8)`

Add after frame_codec module:

```
### `ByzantineGatekeeper` (struct, M8)
Validates incoming messages with 7 safety checks before bus dispatch.
Methods:
- `fn new(max_messages_per_window, rate_window_secs, max_per_peer_log) -> ByzantineGatekeeper`
- `fn register_node(&mut self, node_id: &str)`
- `fn unregister_node(&mut self, node_id: &str)`
- `fn validate(&mut self, msg: &Message) -> Result<(), GateRejection>`

### `GateRejection` (enum, M8)
```rust
pub enum GateRejection {
    InvalidSender(String),
    PhaseOutOfRange { field: String, value: f64 },
    InvalidFrame(String),
    PayloadInconsistent(String),
    RateLimited { peer: String, count: usize },
    PerPeerLogFull { peer: String, count: usize },
    UnknownSender(String),
}
```

### `ResonanceBus` constants (M7)
- `HEARTBEAT_TIMEOUT_SECS: u64` — stale peer detection timeout (30s)
- `SPLIT_BRAIN_TIMEOUT_SECS: u64` — split-brain detection timeout (60s)

### `ResonanceBus` partition tolerance methods (M7)
- `fn stale_peers(&self) -> Vec<String>` — peers with no heartbeat within timeout
- `fn purge_stale_peers(&mut self)` — remove stale peers
- `fn detect_split_brain(&self) -> bool` — true when partition suspected
```

- [ ] **Step 9: Commit**

```bash
git add docs/api.md
git commit -m "docs: fix api.md — Phase/SafeFallback/CustomRule APIs, add M7/M8 docs

Co-Authored-By: Claude <noreply@anthropic.com>"
```

---

### Task 5: Fix docs/concepts/ARCHITECTURE.md

**Files:**
- Modify: `docs/concepts/ARCHITECTURE.md`

- [ ] **Step 1: Update §7 title**

Old: `## 7. 分布式协议（M4–M6）`
New: `## 7. 分布式协议（M4–M8）`

- [ ] **Step 2: Update evolution table header**

Old:
```
| 维度 | M4（内存总线） | M5（TCP 传输层） | M6（种子发现） |
```
New:
```
| 维度 | M4（内存总线） | M5（TCP 传输层） | M6（种子发现） | M7（分区容错） | M8（拜占庭容错） |
```

- [ ] **Step 3: Add M7 row to evolution table**

Add after M6 row:
```
| 容错能力 | 无 | 连接超时 | 种子不可达时优雅降级 | 心跳超时 + 脑裂检测 | 拜占庭节点过滤 |
```

- [ ] **Step 4: Add M8 row to evolution table**

```
| 安全模型 | 无 | TCP 帧大小限制 | 已知节点集合 | 分区恢复 | 7 重验证 + 速率限制 |
```

- [ ] **Step 5: Add §7.7 M7 partition tolerance after §7.6**

```
### 7.7 网络分区容错（M7）

M7 引入了心跳监控和分区检测机制：

**心跳超时**：
- 每个节点定时发送 HEARTBEAT 消息
- 30 秒无心跳视为对等节点失联（`HEARTBEAT_TIMEOUT_SECS`）
- 60 秒无响应触发脑裂检测（`SPLIT_BRAIN_TIMEOUT_SECS`）

**关键函数**：
- `stale_peers()` → 返回超时未心跳的对等节点列表
- `purge_stale_peers()` → 移除失联节点
- `detect_split_brain()` → 检测网络分区（脑裂）

**TcpClient 增强**：
- 连接超时（5s）、读超时（30s）、写超时（10s）
- BufReader/BufWriter 重写，支持多消息会话
```

- [ ] **Step 6: Add §7.8 M8 Byzantine fault tolerance**

```
### 7.8 拜占庭容错（M8）

M8 在 TCP 反序列化与总线分发之间插入了 `ByzantineGatekeeper` 验证层：

**7 重安全检查**：
1. 发送者 ID 验证（非空、非空白、≤128 字符）
2. 已知节点检查（可选，按节点集合白名单）
3. 相位值范围验证（[0.0, 1.0]，有限，非 NaN）
4. 帧名称验证（必须匹配已知 Frame 变体）
5. 负载一致性验证（数组长度匹配、非空参与者）
6. 速率限制（每对等节点每窗口 100 条消息）
7. 每对等节点日志上限（1000 条）

**架构**：
```
TCP 反序列化 → ByzantineGatekeeper::validate() → ResonanceBus::dispatch()
                   ↓ 拒绝
              REJECTED 响应
```

**设计原则**：
- 门卫是可选的（`ResonanceBus` 持有 `Option<ByzantineGatekeeper>`）
- 禁用时零开销（保持向后兼容）
- 速率限制窗口过期后自动重置
```

- [ ] **Step 7: Commit**

```bash
git add docs/concepts/ARCHITECTURE.md
git commit -m "docs: update ARCHITECTURE.md — M4→M8, add M7/M8 subsections

Co-Authored-By: Claude <noreply@anthropic.com>"
```

---

### Task 6: Fix docs/development/MODULES.md

**Files:**
- Modify: `docs/development/MODULES.md`

- [ ] **Step 1: Update net/ section header**

Old: `## `net/` — 分布式协议（M4–M6）`
New: `## `net/` — 分布式协议（M4–M8）`

- [ ] **Step 2: Remove "NEW" markers from file table**

Old:
```
| `discovery.rs` | NEW | 种子节点发现（M6） |
| `frame_codec.rs` | NEW | TCP 长度前缀帧协议（M5） |
| `tcp_client.rs` | NEW | TCP 客户端连接器（M5） |
| `tcp_server.rs` | NEW | TCP 节点服务器（M5） |
```
New:
```
| `discovery.rs` | 94 | 种子节点发现（M6） |
| `frame_codec.rs` | 67 | TCP 长度前缀帧协议（M5） |
| `gate.rs` | 382 | 拜占庭门卫（M8） |
| `tcp_client.rs` | 158 | TCP 客户端连接器（M5） |
| `tcp_server.rs` | 154 | TCP 节点服务器（M5） |
```

- [ ] **Step 3: Remove line counts from all file tables**

In every file table (trit/, meta/, net/), remove the "行数" column since they rot quickly:

For trit/ table, change:
```
| 文件 | 行数 | 职责 |
```
to:
```
| 文件 | 职责 |
```
And remove the number from each row.

Do the same for meta/ and net/ tables.

- [ ] **Step 4: Add M7/M8 key functions to net/ function table**

Add these rows:
```
| `ByzantineGatekeeper::validate` | `fn validate(&mut self, msg: &Message) -> Result<(), GateRejection>` | 7 重拜占庭验证 |
| `ResonanceBus::stale_peers` | `fn stale_peers(&self) -> Vec<String>` | 检测超时未心跳的对等节点 |
| `ResonanceBus::purge_stale_peers` | `fn purge_stale_peers(&mut self)` | 移除失联节点 |
| `ResonanceBus::detect_split_brain` | `fn detect_split_brain(&self) -> bool` | 脑裂检测 |
```

- [ ] **Step 5: Add design constraint for M8**

Add to net/ design constraints:
```
- ByzantineGatekeeper 是可选的（`Option<ByzantineGatekeeper>`），禁用时零开销
- 门卫在 `ResonanceBus::register()` 时自动同步已知节点集合
```

- [ ] **Step 6: Commit**

```bash
git add docs/development/MODULES.md
git commit -m "docs: update MODULES.md — M4→M8, remove line counts, add gate/M7/M8 entries

Co-Authored-By: Claude <noreply@anthropic.com>"
```

---

### Task 7: Fix docs/REVIEWER_GUIDE.md

**Files:**
- Modify: `docs/REVIEWER_GUIDE.md`

- [ ] **Step 1: Fix test count**

Old: `cargo test --all-features -- --test-threads=1`
Context: "运行所有测试（227 个）"
New: `# 运行所有测试（305 个）`

Actually, the count is in the comment above the command. Fix:

Old:
```
# 运行所有测试（227 个）
cargo test --all-features -- --test-threads=1
```
New:
```
# 运行所有测试（305 个）
cargo test --all-features
```

(Also remove `--test-threads=1` since tests are now thread-safe after M9)

- [ ] **Step 2: Update M4-M7 reference**

Old: `| 分布式协议 | `docs/concepts/ARCHITECTURE.md` §7（M4–M7） |`
New: `| 分布式协议 | `docs/concepts/ARCHITECTURE.md` §7（M4–M8） |`

- [ ] **Step 3: Add Byzantine validation to verification section**

Add after the security section:
```
### 声明 4：拜占庭容错有效阻止恶意消息

运行 `cargo test byzantine` 验证门卫的 7 重安全检查（无效发送者、相位越界、无效帧名、负载不一致、速率限制等）。
```

- [ ] **Step 4: Commit**

```bash
git add docs/REVIEWER_GUIDE.md
git commit -m "docs: update REVIEWER_GUIDE — 305 tests, M4→M8

Co-Authored-By: Claude <noreply@anthropic.com>"
```

---

### Task 8: Final verification

- [ ] **Step 1: Run full test suite**

```bash
cargo test --all-features
```
Expected: 305 tests passed, 0 failed

- [ ] **Step 2: Run clippy**

```bash
cargo clippy --all-targets --all-features -- -D warnings
```
Expected: 0 warnings

- [ ] **Step 3: Run fmt check**

```bash
cargo fmt -- --check
```
Expected: clean (no changes needed)

- [ ] **Step 4: Verify no stale test counts remain**

```bash
grep -rn "298 tests\|298 passing" --include="*.md" README.md CHANGELOG.md docs/ CLAUDE.md
```
Expected: ZERO matches

```bash
grep -rn "227 个\|227 tests" --include="*.md" README.md docs/ CLAUDE.md
```
Expected: ZERO matches (227 is only valid in CHANGELOG alpha section)

- [ ] **Step 5: Build release**

```bash
cargo build --release
```
Expected: success

- [ ] **Step 6: Commit verification**

```bash
git add -A
git commit -m "chore: final verification after consistency audit — 305 tests, 0 warnings

Co-Authored-By: Claude <noreply@anthropic.com>"
```
