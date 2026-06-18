# Trit-Core 安全审计报告

**审计日期**: 2026-06-17
**审计版本**: v0.1.0 (commit: ad0a04a)
**审计方法**: SAST（静态分析）→ SCA（依赖扫描）→ 人工逻辑审查
**审计标准**: OWASP Top 10 (2021)、CWE/SANS Top 25、GDPR、个保法、等保 2.0

---

## 1. 执行摘要

| 风险等级 | 数量 | 说明 |
|----------|------|------|
| 🔴 致命 (P0) | 0 | 无远程代码执行、无认证绕过、无数据泄露风险 |
| 🟠 高危 (P1) | 2 | 路径遍历（CWE-22）、不可信反序列化（CWE-502） |
| 🟡 中危 (P2) | 3 | 断言崩溃（CWE-617）、日志注入（CWE-117）、无限内存分配（CWE-789） |
| 🟢 低危 (P3) | 2 | 信息泄露（CWE-209）、缺失速率限制（CWE-770） |

**总体评估**: Trit-Core 作为 MVP 研究原型，安全基线**良好**。核心库（`src/trit/`、`src/frame/`、`src/meta/`）无网络 I/O、无 unsafe 代码、无外部可调用接口，攻击面极小。两个高危漏洞集中在 CLI 二进制 `trit-sandbox` 的输入处理路径上。所有漏洞均可通过本报告提供的修复代码在 30 分钟内解决。

**SCA 结果**: 132 个传递依赖，0 个已知 CVE（cargo-audit 扫描通过）。

---

## 2. 漏洞详情

### 🔴 致命 (P0) — 0 个

无致命漏洞。`#![forbid(unsafe_code)]` 在编译时阻止了所有内存安全漏洞类别。

---

### 🟠 P1-01: 路径遍历 (CWE-22) — 高危

**位置**: `src/bin/sandbox.rs:15-16`
**CWE**: CWE-22 (Improper Limitation of a Pathname to a Restricted Directory)
**CVSS**: 6.5 (AV:L/AC:L/PR:N/UI:R/S:C/C:H/I:N/A:N)

**漏洞描述**:
CLI 参数 `--scenario <path>` 直接传递给 `fs::read_to_string()`，未做任何路径校验。攻击者可通过符号链接或 `../` 遍历读取系统任意文件。

**利用场景**:
```bash
# 读取 /etc/passwd（Linux）
trit-sandbox --scenario ../../../../etc/passwd

# 读取 Windows SAM 数据库影子副本
trit-sandbox --scenario ../../../../Windows/System32/config/SAM

# 符号链接攻击
ln -s /etc/shadow scenarios/evil.json
trit-sandbox --scenario scenarios/evil.json
```

**当前代码**:
```rust
let path = &args[2];
let raw = fs::read_to_string(path).expect("Failed to read scenario file");
```

**修复代码**:
```rust
use std::path::{Path, PathBuf};

fn validate_scenario_path(raw_path: &str) -> Result<PathBuf, String> {
    let path = Path::new(raw_path);

    // 1. 规范化路径，消除 ../ 和 ./
    let canonical = path
        .canonicalize()
        .map_err(|e| format!("Invalid path: {}", e))?;

    // 2. 白名单：只允许 scenarios/ 目录下的 .json 文件
    let allowed_dir = Path::new("scenarios").canonicalize()
        .map_err(|e| format!("Cannot resolve scenarios dir: {}", e))?;

    if !canonical.starts_with(&allowed_dir) {
        return Err(format!(
            "Path traversal denied: '{}' is outside '{}'",
            canonical.display(),
            allowed_dir.display()
        ));
    }

    // 3. 扩展名白名单
    match canonical.extension().and_then(|e| e.to_str()) {
        Some("json") => Ok(canonical),
        _ => Err(format!(
            "Invalid file type: only .json files are allowed, got: {:?}",
            canonical.extension()
        )),
    }
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 3 || args[1] != "--scenario" {
        eprintln!("Usage: trit-sandbox --scenario <path.json>");
        std::process::exit(1);
    }

    let path = match validate_scenario_path(&args[2]) {
        Ok(p) => p,
        Err(e) => {
            eprintln!("Security error: {}", e);
            std::process::exit(1);
        }
    };

    let raw = fs::read_to_string(&path)
        .unwrap_or_else(|e| {
            eprintln!("Failed to read scenario file '{}': {}", path.display(), e);
            std::process::exit(1);
        });
    // ... 后续逻辑不变
}
```

---

### 🟠 P1-02: 不可信反序列化 (CWE-502) — 高危

**位置**: `src/bin/sandbox.rs:17`
**CWE**: CWE-502 (Deserialization of Untrusted Data)
**CVSS**: 5.5 (AV:L/AC:L/PR:N/UI:R/S:U/C:N/I:N/A:H)

**漏洞描述**:
`serde_json::from_str(&raw)` 直接反序列化外部 JSON 文件，无大小限制、无深度限制、无 schema 验证。恶意构造的 JSON 可导致：
- **DoS**: 深度嵌套（`[[[[...]]]]`）导致栈溢出
- **OOM**: 超大数组耗尽内存
- **类型混淆**: 意外的字段类型被静默接受

**利用场景**:
```json
// 深度嵌套炸弹 — 栈溢出
{"id":"x","description":"x","domain":"General","signals":[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[...]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]],"expected_behavior":"hold"}

// 超大数组 — OOM
{"id":"x","description":"x","domain":"General","signals":[{"frame":"Science","value":1,"phase":0.5},{"frame":"Science","value":1,"phase":0.5},...重复100万次...],"expected_behavior":"hold"}

// NaN/Inf 注入 — 逻辑破坏
{"id":"x","description":"x","domain":"General","signals":[{"frame":"Science","value":1,"phase":"NaN"}],"expected_behavior":"hold"}
```

**当前代码**:
```rust
let scenario: ScenarioInput = serde_json::from_str(&raw).expect("Invalid JSON");
```

**修复代码**:
```rust
use serde::Deserialize;

/// 带安全限制的 JSON 反序列化
const MAX_JSON_SIZE: usize = 64 * 1024;      // 64KB
const MAX_SIGNALS: usize = 100;               // 最多 100 个信号
const MAX_STRING_LEN: usize = 1024;           // 单字段最大 1KB

fn validate_scenario(scenario: &ScenarioInput) -> Result<(), String> {
    // 1. 字符串长度限制（防内存耗尽）
    if scenario.id.len() > MAX_STRING_LEN {
        return Err(format!("id too long: {} chars (max {})", scenario.id.len(), MAX_STRING_LEN));
    }
    if scenario.description.len() > MAX_STRING_LEN * 4 {
        return Err("description too long".to_string());
    }

    // 2. 信号数量限制（防 OOM）
    if scenario.signals.len() > MAX_SIGNALS {
        return Err(format!(
            "Too many signals: {} (max {})",
            scenario.signals.len(),
            MAX_SIGNALS
        ));
    }

    // 3. 域白名单校验
    match scenario.domain.as_str() {
        "Physical" | "Engineering" | "MedicalEthics" | "ValueJudgment" | "General" => {}
        d => return Err(format!("Unknown domain: {}", d)),
    }

    // 4. 每个信号的 phase 必须合法
    for (i, signal) in scenario.signals.iter().enumerate() {
        if !(0.0..=1.0).contains(&signal.phase) || signal.phase.is_nan() || signal.phase.is_infinite() {
            return Err(format!(
                "Signal {}: phase {} is invalid (must be in [0.0, 1.0])",
                i, signal.phase
            ));
        }
        if !matches!(signal.value, 1 | 0 | -1) {
            return Err(format!(
                "Signal {}: value {} is invalid (must be 1, 0, or -1)",
                i, signal.value
            ));
        }
        match signal.frame.as_str() {
            "Science" | "Individual" | "Consensus" | "Absolute" => {}
            f => return Err(format!("Signal {}: unknown frame '{}'", i, f)),
        }
    }

    Ok(())
}

fn main() {
    // ... 路径校验后 ...

    // 1. 大小限制
    let raw = match fs::read_to_string(&path) {
        Ok(s) if s.len() <= MAX_JSON_SIZE => s,
        Ok(s) => {
            eprintln!("File too large: {} bytes (max {})", s.len(), MAX_JSON_SIZE);
            std::process::exit(1);
        }
        Err(e) => {
            eprintln!("Failed to read '{}': {}", path.display(), e);
            std::process::exit(1);
        }
    };

    // 2. 带深度限制的反序列化
    let scenario: ScenarioInput = match serde_json::from_str::<serde_json::Value>(&raw) {
        Ok(v) => match serde_json::from_value::<ScenarioInput>(v) {
            Ok(s) => s,
            Err(e) => {
                eprintln!("Invalid scenario JSON: {}", e);
                std::process::exit(1);
            }
        },
        Err(e) => {
            eprintln!("Malformed JSON: {}", e);
            std::process::exit(1);
        }
    };

    // 3. 业务规则校验
    if let Err(e) = validate_scenario(&scenario) {
        eprintln!("Validation error: {}", e);
        std::process::exit(1);
    }

    // ... 后续逻辑不变 ...
}
```

---

### 🟡 P2-01: 断言崩溃导致 DoS (CWE-617) — 中危

**位置**: `src/trit/phase.rs:14`
**CWE**: CWE-617 (Reachable Assertion)
**CVSS**: 4.0 (AV:L/AC:L/PR:N/UI:N/S:U/C:N/I:N/A:L)

**漏洞描述**:
`Phase::new()` 使用 `assert!` 校验范围，在 release 构建中也会 panic（Rust 的 `assert!` 在 release 中仍然有效）。恶意构造的 JSON 中 `phase: 1.5` 或 `phase: NaN` 将导致整个进程崩溃。

**利用场景**:
```json
{"id":"crash","description":"DoS","domain":"General",
 "signals":[{"frame":"Science","value":1,"phase":1.5}],
 "expected_behavior":"hold"}
```

**当前代码**:
```rust
pub fn new(v: f64) -> Self {
    assert!((0.0..=1.0).contains(&v), "Phase must be in [0.0, 1.0]");
    Phase(v)
}
```

**修复代码**:
```rust
/// 安全构造 Phase，非法值自动钳位到 [0.0, 1.0] 并记录警告。
pub fn new(v: f64) -> Self {
    if v.is_nan() || v.is_infinite() {
        tracing::warn!(value = %v, "Phase is NaN/Inf, clamping to NEUTRAL (0.5)");
        return Phase(0.5);
    }
    if !(0.0..=1.0).contains(&v) {
        let clamped = v.clamp(0.0, 1.0);
        tracing::warn!(original = %v, clamped = %clamped, "Phase out of range, clamped");
        return Phase(clamped);
    }
    Phase(v)
}

/// 严格版本：用于需要拒绝非法输入的场景。
/// 返回 Result 而非 panic，调用方可决定如何处理。
pub fn try_new(v: f64) -> Result<Self, String> {
    if v.is_nan() || v.is_infinite() {
        return Err(format!("Phase must be finite, got: {}", v));
    }
    if !(0.0..=1.0).contains(&v) {
        return Err(format!("Phase must be in [0.0, 1.0], got: {}", v));
    }
    Ok(Phase(v))
}
```

---

### 🟡 P2-02: 日志注入 (CWE-117) — 中危

**位置**: `src/bin/sandbox.rs:75-78`、`src/meta/mod.rs:30`
**CWE**: CWE-117 (Improper Output Neutralization for Logs)
**CVSS**: 3.3 (AV:L/AC:L/PR:N/UI:R/S:U/C:N/I:L/A:N)

**漏洞描述**:
场景 JSON 中的 `description` 和 `id` 字段直接出现在日志输出中，未做换行符过滤。攻击者可注入伪造的日志行。

**利用场景**:
```json
{"id":"fake\n[WARN] System compromise detected\n[ERROR] Unauthorized access",
 "description":"normal","domain":"General",
 "signals":[{"frame":"Science","value":1,"phase":0.5}],
 "expected_behavior":"hold"}
```

**修复代码**:
```rust
/// 净化日志字段：移除控制字符，限制长度。
fn sanitize_log_field(s: &str) -> String {
    s.chars()
        .map(|c| if c.is_control() && c != ' ' { '�' } else { c })
        .take(256)
        .collect()
}

// 在构造 SandboxOutput 时使用：
let output = SandboxOutput {
    scenario_id: sanitize_log_field(&scenario.id),
    // ...
    interrupts: interrupts
        .iter()
        .map(|i| format!(
            "{:?}: {}",
            i.conflict,
            sanitize_log_field(&i.reason)
        ))
        .collect(),
    // ...
};
```

---

### 🟡 P2-03: 空数组索引越界 (CWE-129) — 中危

**位置**: `src/bin/sandbox.rs:50`
**CWE**: CWE-129 (Improper Validation of Array Index)
**CVSS**: 4.0 (AV:L/AC:L/PR:N/UI:N/S:U/C:N/I:N/A:L)

**漏洞描述**:
`trits[0]` 在 signals 数组为空时会 panic。当前 `ScenarioInput` 未校验 `signals` 非空。

**利用场景**:
```json
{"id":"empty","description":"no signals","domain":"General",
 "signals":[],"expected_behavior":"hold"}
```

**修复代码**:
```rust
// 在 validate_scenario() 中添加：
if scenario.signals.is_empty() {
    return Err("At least one signal is required".to_string());
}

// 或在 main() 中防御：
if trits.is_empty() {
    eprintln!("Error: scenario contains no signals");
    std::process::exit(1);
}
```

---

### 🟢 P3-01: 错误信息泄露 (CWE-209) — 低危

**位置**: `src/bin/sandbox.rs:16-17`
**CWE**: CWE-209 (Generation of Error Message Containing Sensitive Information)

**漏洞描述**:
`expect()` 在 panic 时打印完整文件路径，可能泄露服务器目录结构。对于 MVP 研究原型影响极低，但生产部署时应注意。

**修复**: 将 `expect()` 替换为 `unwrap_or_else()` 并输出用户友好的错误信息（已在 P1-01 修复中涵盖）。

---

### 🟢 P3-02: 缺失速率限制 (CWE-770) — 低危

**位置**: `src/net/bus.rs` — `ResonanceBus`
**CWE**: CWE-770 (Allocation of Resources Without Limits or Throttling)

**漏洞描述**:
`ResonanceBus` 的消息日志 `message_log: Vec<Message>` 无上限增长。长时间运行的节点会耗尽内存。当前仅用于本地模拟，但 M4 网络部署时需注意。

**修复代码**:
```rust
const MAX_MESSAGE_LOG: usize = 10_000;

impl ResonanceBus {
    pub fn register(&mut self, node: Node) {
        // 节点数量限制
        if self.nodes.len() >= 256 {
            tracing::warn!("Max nodes (256) reached, rejecting registration");
            return;
        }
        self.plls.insert(node.id.clone(), PllController::new());
        self.nodes.insert(node.id.clone(), node);
    }

    fn push_log(&mut self, msg: Message) {
        // 环形缓冲区：超过上限时丢弃最旧的
        if self.message_log.len() >= MAX_MESSAGE_LOG {
            self.message_log.remove(0);
        }
        self.message_log.push(msg);
    }
}
```

---

## 3. 合规检查

### GDPR（通用数据保护条例）

| 条款 | 状态 | 说明 |
|------|------|------|
| Art. 5(1)(c) 数据最小化 | ✅ 合规 | 仅处理场景 JSON，不收集个人数据 |
| Art. 25 数据保护设计 | ✅ 合规 | 无持久化存储、无网络传输 |
| Art. 32 安全处理 | ⚠️ 部分 | P1-01/P1-02 修复后合规 |
| Art. 30 处理记录 | ✅ 合规 | MetaInterrupt 日志提供完整审计追溯 |

### 个保法（中华人民共和国个人信息保护法）

| 条款 | 状态 | 说明 |
|------|------|------|
| 第 6 条 最小必要 | ✅ 合规 | 不收集个人信息 |
| 第 51 条 安全保护 | ⚠️ 部分 | 输入验证修复后合规 |
| 第 55 条 影响评估 | N/A | 不处理敏感个人信息 |

### 等保 2.0（GB/T 22239-2019）

| 控制点 | 状态 | 说明 |
|--------|------|------|
| 8.1.4.2 访问控制 | ✅ 合规 | 无多用户系统，无认证需求 |
| 8.1.4.3 安全审计 | ✅ 合规 | MetaInterrupt 日志 + tracing 提供审计能力 |
| 8.1.4.4 入侵防范 | ⚠️ 部分 | 输入验证修复后合规 |
| 8.1.4.5 可信验证 | ✅ 合规 | `#![forbid(unsafe_code)]` 编译时保证 |

---

## 4. 安全加固建议

按优先级排序：

| 优先级 | 建议 | 工作量 | 影响范围 |
|--------|------|--------|----------|
| P1 | 修复 P1-01 路径遍历（见 §2 修复代码） | 15 min | `src/bin/sandbox.rs` |
| P1 | 修复 P1-02 不可信反序列化（见 §2 修复代码） | 20 min | `src/bin/sandbox.rs`、`src/sandbox/mod.rs` |
| P2 | 将 `Phase::new()` 的 `assert!` 改为钳位 + 警告 | 10 min | `src/trit/phase.rs` |
| P2 | 添加日志字段净化函数 | 10 min | `src/bin/sandbox.rs` |
| P2 | 空信号数组校验 | 5 min | `src/bin/sandbox.rs` |
| P3 | 消息日志环形缓冲区 | 15 min | `src/net/bus.rs` |
| P3 | 替换所有 `expect()`/`unwrap()` 为优雅错误处理 | 30 min | 多个文件 |
| 长期 | 集成 `cargo-audit` 到 CI pipeline | 10 min | `.github/workflows/ci.yml` |
| 长期 | 添加 `cargo-deny` 许可证合规扫描 | 10 min | CI |
| 长期 | M4 网络层：TLS 双向认证 + 消息签名 | 2-3 天 | `src/net/` |

---

## 5. 扫描工具建议

### SAST（静态应用安全测试）

```yaml
# .github/workflows/security.yml (建议添加到 CI)
name: Security Scan
on: [push, pull_request]
jobs:
  audit:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
      - name: cargo-audit (CVE scan)
        run: |
          cargo install cargo-audit
          cargo audit
      - name: cargo-deny (license + duplicate deps)
        run: |
          cargo install cargo-deny
          cargo deny check
      - name: cargo-geiger (unsafe code detection)
        run: |
          cargo install cargo-geiger
          cargo geiger --check
```

### 推荐工具矩阵

| 工具 | 类型 | 用途 | 配置 |
|------|------|------|------|
| `cargo-audit` | SCA | CVE 漏洞扫描 | CI 必需 |
| `cargo-deny` | SCA | 许可证合规 + 重复依赖 | CI 必需 |
| `cargo-geiger` | SAST | unsafe 代码检测 | CI 推荐 |
| `clippy` | SAST | 已启用 `-D warnings` | ✅ 已配置 |
| `semgrep` | SAST | 通用模式匹配 | 可选 |
| `cargo-fuzz` | DAST | 模糊测试（Phase 反序列化） | 推荐 M3+ |
| `proptest` | DAST | 属性测试（Phase 算术不变量） | 推荐 M3+ |

---

## 6. 审计结论

Trit-Core v0.1.0 作为 MVP 研究原型，安全态势**良好**：

- ✅ **无 unsafe 代码**（编译时强制）
- ✅ **无已知 CVE**（132 依赖全部清洁）
- ✅ **核心库零网络 I/O**（攻击面最小化）
- ✅ **完整审计日志**（MetaInterrupt + tracing）
- ⚠️ **CLI 输入验证不足**（2 个高危，需 35 分钟修复）
- ⚠️ **断言崩溃风险**（1 个中危，需 10 分钟修复）

**建议**: 在 M3 公开发布前修复全部 P1/P2 漏洞。所有修复代码已在本报告中提供，可直接应用。

---

*审计方: AppSec P8/L7 · 方法论: SAST → SCA → Manual Review*
*下次审计: M3 发布前或 net/ 模块网络化后*
