# Aurora 技术审查清单

**版本**: 0.1.0
**日期**: 2026-06-20
**状态**: 活跃
**分类**: 04_engineering — 工程规格

---

## 使用说明

本文档是**每次发布前的硬性门槛**。不是可选检查项，是必须全部通过的 checklist。

**审查原则**：
- 用技术检验技术，用物理检验物理，用代码检验代码
- 定心盘（CHARTER.md）是最高判据——任何代码如果背叛了定心盘，无论功能多完美，都是失败的
- 不审查哲学立场，只审查代码是否如实表达了哲学立场

**执行方式**：
1. 每次发布前，由开发者逐项自检
2. 勾选每项，不勾选的项必须有 ADR 记录（`05_adr/`）
3. 发现违规时，不允许发布，必须修复或写 ADR 说明为什么这是例外

---

## 一、定心盘一致性审查（最高优先级）

### 1.1 不剥夺（用户可关闭、可覆盖、可离开）

- [ ] **所有功能都有关闭开关**：没有"必须开启"的功能。用户可以选择关闭任何功能。
- [ ] **SafeFallback 可关闭**：`SafeFallback::disabled()` 存在且可访问。用户可以选择关闭安全回退。
- [ ] **Awareness 不阻断**：`Awareness` 状态的实现中，没有任何 `return Err` 或 `panic!`。系统只通知，不阻止。
- [ ] **用户可覆盖 Hold**：当系统输出 `Hold` 时，用户界面提供"忽略 Hold，自行决定"的选项。代码中不阻止用户覆盖。
- [ ] **用户可导出全部数据**：导出功能不限制格式、不限制频率、不限制内容。用户带走自己的数据，系统不阻止。
- [ ] **用户可卸载/删除**：卸载程序不残留守护进程、不残留远程连接、不残留数据锁定。
- [ ] **没有强制更新**：系统不强制更新。旧版本继续运行，不中断。
- [ ] **没有远程控制**：没有远程 kill switch、没有远程功能降级、没有远程数据擦除。

### 1.2 不自欺（系统不猜测、不消冲突、不假装知道）

- [ ] **Hold 时不猜测**：当系统输出 `Hold` 时，代码不提供"最可能"选项、不排序可能性、不给出概率估计。
- [ ] **Meta 帧不伪装**：`Frame::Meta` 只由系统内部冲突输出使用。代码中没有任何路径让外部输入映射到 `Meta`。
- [ ] **数据不足时输出 Unknown**：`TritValue::Unknown` 是合法输出。代码不将 Unknown 强制转为 True/False。
- [ ] **不消冲突**：跨 `Frame` 运算输出 `Hold` + `MetaInterrupt`。代码不尝试"调和"冲突，不输出"折中"结果。
- [ ] **Phase 漂移不修正**：当 `Phase` 漂移时，代码不自动"拉回原位"，只标记漂移并通知用户。
- [ ] **没有"推荐系统"**：代码不基于历史数据预测用户偏好、不推荐"你可能喜欢"。

### 1.3 不进化（系统不基于用户数据学习）

- [ ] **没有模型微调**：代码不基于用户输入数据调整 `TritWord` 的权重、阈值、参数。
- [ ] **没有用户画像**：代码不记录用户行为模式、不构建用户画像、不预测用户偏好。
- [ ] **出厂设置不变**：所有参数、阈值、规则在出厂时固定。运行时不改变。
- [ ] **没有 A/B 测试**：代码不根据用户反馈调整输出策略。
- [ ] **没有强化学习**：没有 RL 循环、没有奖励函数、没有策略优化。

### 1.4 公开可审查（全部逻辑、代码、数据公开）

- [ ] **代码开源**：MIT 协议，GitHub 公开仓库，完整源代码可获取。
- [ ] **审计日志可读取**：审计日志格式公开，用户可以读取、解析、验证。
- [ ] **审计日志不可篡改**：日志写入追加模式，有链式哈希或 Merkle Tree 验证。
- [ ] **文档完整**：架构文档、API 文档、数据格式文档、伦理文档全部公开。
- [ ] **无黑盒**：没有预编译模型、没有加密算法、没有隐藏逻辑。全部可审查。
- [ ] **依赖透明**：`Cargo.toml` 完整列出所有依赖，包括版本号。没有私有依赖或 git 依赖指向私有仓库。

---

## 二、Trit-Core API 正确性审查

### 2.1 Frame 使用正确

- [ ] **Frame::Meta 不用于外部输入**：搜索 `Frame::Meta`，确认没有外部输入（用户输入、文件读取、网络数据）映射到 `Frame::Meta`。
- [ ] **Frame 扩展正确**：如果使用了扩展 Frame（GeoEco/Developmental/Role/Environmental），确认它们是 `Frame` enum 的独立变体，不是 wrapper + `From` 映射到 `Meta`。
- [ ] **Frame 解析大小写敏感**：`Frame::from_str` 对大小写敏感，不自动转换大小写。
- [ ] **未知 Frame 返回 Err**：`Frame::from_str("Unknown")` 返回 `Err`，不默认映射到 `Meta` 或 `Science`。

```bash
# 自检命令
grep -r "Frame::Meta" trit-core/src/ --include="*.rs" -B 2 -A 2
grep -r "impl From<.*> for Frame" trit-core/src/ --include="*.rs"
```

### 2.2 TritWord 构造正确

- [ ] **Phase 范围验证**：`Phase::new(v)` 在 `v < 0.0 || v > 1.0` 时返回 `Err`，不静默 clamp。
- [ ] **Phase::new_clamped 仅用于显式场景**：`new_clamped` 的使用必须有注释说明为什么可以 clamp。
- [ ] **TritWord::tru 只接受 Frame**：`TritWord::tru(frame)` 调用中只有一个参数（Frame）。如果传了 Phase，是 API 误用。
- [ ] **TritWord::fals 只接受 Frame**：同上。
- [ ] **自定义 Phase 用 TritWord::new**：`TritWord::new(value, phase, frame)` 是构造自定义 Phase 的正确方式。

```bash
# 自检命令
grep -r "TritWord::tru\|TritWord::fals" trit-core/src/ --include="*.rs" -n
grep -r "Phase::new(" trit-core/src/ --include="*.rs" -n
grep -r "Phase::new_clamped" trit-core/src/ --include="*.rs" -n
```

### 2.3 代数运算正确

- [ ] **跨 Frame 返回 Hold**：`TernaryAlgebra::t_and` 和 `t_or` 在跨 `Frame` 时返回 `TritValue::Hold` + `MetaInterrupt`。
- [ ] **同 Frame 运算正确**：`True AND True = True`，`True AND False = Hold`，`False AND False = False`（同 Frame 时）。
- [ ] **Phase 合并正确**：`t_and` 的 Phase 合并使用 `min(a.phase(), b.phase())`；`t_or` 使用 `max(a.phase(), b.phase())`。
- [ ] **Hold 的 Phase 为 0.5**：`TritWord::hold(frame)` 的 Phase 为 `0.5`。

```bash
# 自检命令：运行属性测试
cargo test proptest
```

---

## 三、安全模型审查

### 3.1 SecurityMode 正确性

- [ ] **Normal 状态允许运算**：`SecurityMode::Normal` 下，所有运算正常执行。
- [ ] **Awareness 状态允许运算**：`SecurityMode::Awareness` 下，所有运算继续执行，不返回 `Err`。
- [ ] **Transparency 状态允许运算**：`SecurityMode::Transparency` 下，所有运算继续执行，额外输出内部状态。
- [ ] **没有 SafeMode 状态**：确认 `SecurityMode` enum 中没有 `SafeMode` 或 `Lockdown` 变体。
- [ ] **allows_computation() 恒返回 true**：`allows_computation()` 方法在所有状态下返回 `true`。

```bash
# 自检命令
grep -r "SafeMode\|Lockdown" trit-core/src/ --include="*.rs"
grep -r "allows_computation" trit-core/src/ --include="*.rs" -A 5
```

### 3.2 ConflictType 正确性

- [ ] **PolicyViolation 只通知**：`PolicyViolation` 的处理路径只返回 `MetaInterrupt`（通知），不返回 `Err`（阻断）。
- [ ] **FrameContamination 检测**：代码检测到 `Meta` 帧作为外部输入时，返回 `PolicyViolation::FrameContamination`。
- [ ] **ForcedCollapse 检测**：代码检测到外部试图强制系统输出 True/False（而非 Hold）时，返回 `PolicyViolation::ForcedCollapse`。
- [ ] **DataAnomaly 检测**：代码检测到输入模式与历史基线偏离 > 3σ 时，返回 `PolicyViolation::DataAnomaly`。

### 3.3 审计日志正确性

- [ ] **追加写入**：审计日志使用 `OpenOptions::new().append(true)`，不是 `write(true)` 或 `truncate(true)`。
- [ ] **链式哈希**：每条日志记录包含前一条的哈希，形成链。哈希算法为 SHA-256 或 Blake3。
- [ ] **日志不可删除**：代码中没有删除日志的 API。用户只能导出，不能删除。
- [ ] **日志包含完整状态**：每条日志包含：时间戳、操作、输入、输出、SecurityMode、Frame、用户覆盖标记。

---

## 四、数据主权审查

### 4.1 本地优先

- [ ] **SQLite 本地存储**：数据存储在本地 SQLite 文件，不远程同步。
- [ ] **没有远程服务器**：代码中没有 HTTP 客户端、没有 WebSocket、没有 gRPC 调用外部服务。
- [ ] **没有遥测**：代码不发送使用数据、崩溃报告、性能指标到任何服务器。
- [ ] **没有云服务依赖**：没有 AWS S3、Google Cloud、Azure 等云服务的 SDK 依赖。

```bash
# 自检命令
grep -r "reqwest\|hyper\|tokio-tungstenite\|tonic" trit-core/Cargo.toml
grep -r "http\|https\|ws://\|wss://" trit-core/src/ --include="*.rs"
```

### 4.2 加密正确性

- [ ] **SQLite 使用 SQLCipher**：`Cargo.toml` 中 `rusqlite` 的 features 包含 `bundled-sqlcipher`。
- [ ] **AES-256-CBC + HMAC-SHA256**：加密方案明确为 AES-256-CBC + HMAC-SHA256，不是"AES-256-GCM"（SQLite 原生不支持 GCM）。
- [ ] **密钥本地生成**：密钥从用户密码通过 Argon2id 派生，不远程获取。
- [ ] **密钥不持久化**：密钥在内存中，程序关闭后清除。不写入文件。

```bash
# 自检命令
grep -r "sqlcipher" trit-core/Cargo.toml
grep -r "AES-256-GCM" trit-core/ --include="*.rs" --include="*.md"
grep -r "argon2\|Argon2" trit-core/src/ --include="*.rs"
```

---

## 五、性能与可靠性审查

### 5.1 性能基准

- [ ] **热路径 < 5ns**：`TernaryAlgebra::t_and` / `t_or` 的 Criterion 基准 < 5ns。
- [ ] **端到端 < 1s**：从数据采集到 Trit-Core 输出的完整管道 < 1s（1000 条信号）。
- [ ] **内存 < 500MB**：单用户运行时的内存占用 < 500MB。
- [ ] **启动 < 3s**：从点击到可交互 < 3s。

```bash
# 自检命令
cargo bench
# 检查 criterion 输出中的 t_and、t_or 延迟
```

### 5.2 错误处理正确性

- [ ] **无 unwrap 在热路径**：热路径（`t_and`、`t_or`、`Phase::new`）中没有 `unwrap`。
- [ ] **无 panic 在公共 API**：公共 API 函数不 panic，所有错误返回 `Result`。
- [ ] **错误信息可理解**：`thiserror` 派生的错误信息是人类可读的，不暴露内部状态。

```bash
# 自检命令
grep -r "unwrap()" trit-core/src/core/ --include="*.rs" -n
grep -r "panic!" trit-core/src/ --include="*.rs" -n
```

### 5.3 资源管理正确性

- [ ] **无内存泄漏**：使用 `dhat` 或 `valgrind` 验证无内存泄漏。
- [ ] **文件句柄正确关闭**：SQLite 连接、日志文件在程序退出时正确关闭。
- [ ] **线程安全**：所有公共 API 是 `Send + Sync`，或者明确标记为 `!Send`/`!Sync` 并说明原因。

---

## 六、依赖安全审查

### 6.1 依赖审计

- [ ] **cargo audit 通过**：`cargo audit` 无高危漏洞。
- [ ] **依赖最小化**：`Cargo.toml` 中的依赖数量 < 20（运行时）。dev 依赖不计入。
- [ ] **无 GPL 依赖**：所有依赖的协议与 MIT 兼容（MIT、Apache-2.0、BSD、ISC 等）。
- [ ] **无预编译二进制**：所有依赖都是 Rust 源码编译，没有预编译的 `.so`、`.dll`、`.a`。

```bash
# 自检命令
cargo audit
cargo tree --edges normal | wc -l  # 统计依赖数量
cargo tree --format "{p} {l}" | grep -i "GPL"
```

### 6.2 版本锁定

- [ ] **Cargo.lock 提交**：`Cargo.lock` 提交到版本控制，确保可复现构建。
- [ ] **版本号明确**：`Cargo.toml` 中依赖使用 `x.y.z` 精确版本，不使用 `*` 或 `>=`。
- [ ] **无 git 依赖**：所有依赖来自 crates.io，不依赖 git 仓库（除非有 ADR 说明）。

---

## 七、文档一致性审查

### 7.1 代码与文档一致

- [ ] **文档中的代码示例可运行**：所有文档（`.md`）中的 Rust 代码示例可以通过 `rustdoc --test` 验证。
- [ ] **API 文档与代码一致**：`cargo doc` 生成的文档与代码实现一致，没有过时描述。
- [ ] **架构文档与代码一致**：`03_architecture/` 和 `07_specs/` 中的描述与代码实现一致。

```bash
# 自检命令
cargo test --doc
```

### 7.2 文档伦理一致性

- [ ] **文档无"保护用户"表述**：搜索 "保护用户"、"系统拒绝"、"系统阻止"，确认没有系统阻止用户操作的表述。
- [ ] **文档无"系统最懂"表述**：搜索 "系统推荐"、"系统建议"、"最优选择"，确认系统不提供替代方案。
- [ ] **文档明确用户自负其责**：所有涉及危险域的文档，明确标注"用户自负其责"、"系统只通知，不决定"。

```bash
# 自检命令
grep -ri "保护用户\|系统拒绝\|系统阻止" trit-core/aurora/ --include="*.md"
grep -ri "系统推荐\|系统建议\|最优选择" trit-core/aurora/ --include="*.md"
```

---

## 八、发布审查签名

**版本**: ______
**审查日期**: ______
**审查人**: ______

### 8.1 定心盘一致性

- [ ] 全部通过（无一未勾选）
- [ ] 有例外项，已写 ADR（编号：_____）

### 8.2 技术正确性

- [ ] 全部通过（无一未勾选）
- [ ] 有例外项，已写 ADR（编号：_____）

### 8.3 安全模型

- [ ] 全部通过（无一未勾选）
- [ ] 有例外项，已写 ADR（编号：_____）

### 8.4 数据主权

- [ ] 全部通过（无一未勾选）
- [ ] 有例外项，已写 ADR（编号：_____）

### 8.5 性能与可靠性

- [ ] 全部通过（无一未勾选）
- [ ] 有例外项，已写 ADR（编号：_____）

### 8.6 依赖安全

- [ ] 全部通过（无一未勾选）
- [ ] 有例外项，已写 ADR（编号：_____）

### 8.7 文档一致性

- [ ] 全部通过（无一未勾选）
- [ ] 有例外项，已写 ADR（编号：_____）

### 签名

> 我确认，本次发布的代码如实表达了定心盘（CHARTER.md）的四条底线。代码没有背叛"不剥夺、不自欺、不进化、公开可审查"的原则。如有例外，已记录在 ADR 中。
>
> **签名**: ________________
> **日期**: ________________

---

*本文档为 Aurora 的发布前技术审查清单。不是形式，是工程纪律。定心盘是最高判据，代码是最终检验。不是指教，是提醒。*
