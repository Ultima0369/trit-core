# MOC — 架构决策（ADR）

> **Scope**: 所有 Architecture Decision Records，按项目归属和主题分类。adr/ 记录的是"为什么这样设计"，不是"怎么做"。
>
> #trit-core #aurora #adr #architecture #decisions

---

## docs/adr/ — Trit-Core Crate 英文 ADR

面向 Rust 开发者和外部研究者。记录 crate 层面的技术决策。

| ADR | 文件 | 主题 | 状态 |
|---|---|---|---|
| ADR-001 | [[001-ternary-logic]] | `docs/adr/001-ternary-logic.md` | 为什么用三值逻辑替代二值 | Accepted |
| ADR-002 | [[002-phase-arithmetic]] | `docs/adr/002-phase-arithmetic.md` | 为什么用浮点相位 [0.0, 1.0] | Accepted |
| ADR-003 | [[003-domain-conflict]] | `docs/adr/003-domain-conflict.md` | 域冲突检测与仲裁机制 | Accepted |
| ADR-004 | [[004-distributed-protocol]] | `docs/adr/004-distributed-protocol.md` | 分布式协议（已在 v0.2.0 移除） | Deprecated |

**跨链连接**：
- ADR-001 对应 `src/core/trit.rs` + `src/core/algebra.rs`
- ADR-002 对应 `src/core/phase.rs`
- ADR-003 对应 `src/meta/arbitration.rs`
- ADR-004 对应 `src/net/`（已删除）

---

## aurora/05_adr/ — Aurora 应用中文 ADR

面向 Aurora 中文用户和开发者。记录应用层面的产品决策。

| ADR | 文件 | 主题 | 状态 |
|---|---|---|---|
| ADR-001 | [[001-local-first]] | `aurora/05_adr/001-local-first.md` | 本地优先：数据不出设备 | Accepted |
| ADR-002 | [[002-wavelet-over-fft]] | `aurora/05_adr/002-wavelet-over-fft.md` | 小波变换替代 FFT | Accepted |
| ADR-003 | [[003-ternary-over-binary]] | `aurora/05_adr/003-ternary-over-binary.md` | 三值决策优于二值平均 | Accepted |
| ADR-004 | [[004-geoeco-frame]] | `aurora/05_adr/004-geoeco-frame.md` | Frame 从 4 个扩展到 9 个 | Accepted |
| ADR-005 | [[005-rust-over-python]] | `aurora/05_adr/005-rust-over-python.md` | 为什么用 Rust 而非 Python | Accepted |
| ADR-006 | [[006-tauri-over-electron]] | `aurora/05_adr/006-tauri-over-electron.md` | 为什么用 Tauri 而非 Electron | Accepted |
| ADR-007 | [[007-sqlite-over-postgres]] | `aurora/05_adr/007-sqlite-over-postgres.md` | 为什么用 SQLite 而非 PostgreSQL | Accepted |
| ADR-008 | [[008-subscription-over-ads]] | `aurora/05_adr/008-subscription-over-ads.md` | 开源免费 vs 商业模式（已重写，废止订阅制） | 已重写 |
| ADR-009 | [[009-ethics-hardening]] | `aurora/05_adr/009-ethics-hardening.md` | 伦理硬化：系统不阻断运算 | Accepted |

---

## 主题聚类

### 三值逻辑（两条链的交汇点）

| docs ADR | aurora ADR | 差异说明 |
|---|---|---|
| [[001-ternary-logic]] | [[003-ternary-over-binary]] | docs 侧重数学/代数层面；aurora 侧重决策质量/用户体验层面。同一决策，不同受众。 |

### 技术栈选择

| aurora ADR | 涉及代码 |
|---|---|
| [[005-rust-over-python]] | 全部 `src/` |
| [[006-tauri-over-electron]] | `src-tauri/`（Aurora GUI） |
| [[007-sqlite-over-postgres]] | `src/db/`（Aurora 数据层） |
| [[002-wavelet-over-fft]] | `src/wavelet/`（Aurora 分析引擎） |

### 伦理与架构

| aurora ADR | 涉及代码 |
|---|---|
| [[001-local-first]] | 架构层面（无远程连接） |
| [[009-ethics-hardening]] | `src/meta/security_mode.rs` |
| [[008-subscription-over-ads]] | 开源免费 / 不要注意力（无代码） |

---

## 维护提示

- **新增 crate 级 ADR** → 放入 `docs/adr/`，在上方 docs 表格登记，更新 [[02_concepts]] 中的跨链连接
- **新增 Aurora 级 ADR** → 放入 `aurora/05_adr/`，在上方 aurora 表格登记
- **ADR 状态变更**（Accepted → Deprecated）→ 在两个 MOC 中同步更新状态列
- **ADR 引用代码** → 在 ADR 文件末尾添加 "Implementation" 小节，链接到具体源码文件

---

**相关 MOC**: [[01_manifest]] · [[02_concepts]] · [[05_engineering]] · [[06_code]]

#map-of-content #adr #architecture-decisions #engineering #trit-core #aurora
