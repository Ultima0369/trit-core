# Trit-Core Documentation

> **Language note**: This directory (`docs/`) contains the original Trit-Core technical documentation, primarily in English for crate developers and researchers. For the Chinese Aurora application documentation system, see [`aurora/`](../aurora/). **If you are working on Aurora, see [Aurora MASTER_PLAN](../aurora/MASTER_PLAN.md) for the execution entry point.**

---

## Quick Navigation

| If you want to... | Read this |
|---|---|
| Understand what Trit-Core is and why it exists | [`technical-whitepaper.md`](technical-whitepaper.md) |
| Learn the core concepts (TritValue, Frame, Phase, MetaInterrupt) | [`explanation/CONCEPTS.md`](explanation/CONCEPTS.md) |
| Understand the architecture and module structure | [`explanation/ARCHITECTURE.md`](explanation/ARCHITECTURE.md) |
| Run your first scenario | [`tutorials/QUICKSTART.md`](tutorials/QUICKSTART.md) |
| Integrate Trit-Core into your own project | [`reference/api.md`](reference/api.md) |
| Understand the design decisions behind the code | [`adr/`](adr/) |
| Review validation evidence | [`reports/validation-report.md`](reports/validation-report.md) |

---

## Directory Structure

### `adr/` — Architecture Decision Records

English-language ADRs for the Trit-Core crate:

- [`001-ternary-logic.md`](adr/001-ternary-logic.md) — Why ternary logic over binary
- [`002-phase-arithmetic.md`](adr/002-phase-arithmetic.md) — Phase arithmetic design
- [`003-domain-conflict.md`](adr/003-domain-conflict.md) — Domain conflict detection and arbitration
- [`004-distributed-protocol.md`](adr/004-distributed-protocol.md) — Distributed protocol (removed in v0.2.0, kept for historical reference)

### `explanation/` — Conceptual Documentation

- [`ARCHITECTURE.md`](explanation/ARCHITECTURE.md) — System architecture (English)
- [`CONCEPTS.md`](explanation/CONCEPTS.md) — Core type definitions and semantics (Chinese)
- [`PHILOSOPHY.md`](explanation/PHILOSOPHY.md) — Design philosophy and motivation (Chinese)
- [`insights/`](explanation/insights/) — Deep-dive essays on epistemology, conflict patterns, humanities, and dao-science connections (Chinese)

### `how-to/` — Usage Guides

- [`CLI_REFERENCE.md`](how-to/CLI_REFERENCE.md) — `trit-sandbox` CLI usage
- [`CONFIGURATION.md`](how-to/CONFIGURATION.md) — Environment variables and logging
- [`CONTRIBUTING.md`](how-to/CONTRIBUTING.md) — Contribution guidelines
- [`CUSTOM_RULE.md`](how-to/CUSTOM_RULE.md) — Defining custom arbitration domains
- [`REVIEWER_GUIDE.md`](how-to/REVIEWER_GUIDE.md) — Guide for external reviewers

### `reference/` — API & Technical Reference

- [`api.md`](reference/api.md) — Public API contract (English)
- [`MODULES.md`](reference/MODULES.md) — Module-level documentation
- [`BENCHMARK.md`](reference/BENCHMARK.md) — Performance benchmarks

### `reports/` — Validation & Audit Reports

- [`validation-report.md`](reports/validation-report.md) — M2/M3 validation (v0.3.0, English)
- [`performance-validation.md`](reports/performance-validation.md) — Performance validation (v0.2.0)
- [`security-audit.md`](reports/security-audit.md) — Security audit (v0.1.0, **historical**)
- [`code-quality-audit.md`](reports/code-quality-audit.md) — Code quality audit (v0.1.0, **historical**)
- [`cto-audit-report.md`](reports/cto-audit-report.md) — CTO audit (v0.1.0, **historical**)
- [`deep-audit-cto-2026-06-18.md`](reports/deep-audit-cto-2026-06-18.md) — Deep technical audit (v0.1.0, **historical**)

> **Historical reports**: Reports marked **historical** audit Trit-Core v0.1.0 or v0.2.0. Many issues identified have been resolved in subsequent versions. Current status is tracked in `audit_log/08_reflexive_audit.md` and `aurora/08_reports/`.

### `tutorials/` — Getting Started

- [`QUICKSTART.md`](tutorials/QUICKSTART.md) — 3-minute quickstart
- [`WHAT_IS_TRIT.md`](tutorials/WHAT_IS_TRIT.md) — Three stories explaining ternary decisions

### `archive/` — Historical Snapshots (v0.1.x)

- [`preprint.md`](archive/preprint.md) — v0.1.0 preprint
- [`roadmap-v0.1.0.md`](archive/roadmap-v0.1.0.md) — v0.1.x roadmap
- [`technical-whitepaper.md`](archive/technical-whitepaper.md) — v0.1.x whitepaper

### `_archive/superpowers/` — AI Work Logs

Design specs and implementation plans from the AI-assisted documentation and architecture design sessions (June 2026). Kept for traceability.

---

## Relationship to `aurora/`

- **`docs/`** — Trit-Core crate documentation. Mix of English (technical reference, ADRs) and Chinese (concepts, philosophy, insights).面向 Rust 开发者与研究者.
- **`aurora/`** — Aurora application documentation system. Fully Chinese. 面向 Aurora 最终用户与中文社区.

Both are maintained independently. When Trit-Core releases a new version, update `docs/`; when Aurora ships a feature, update `aurora/`.

## 双螺旋知识库

本项目采用 Obsidian 风格的双螺旋知识库架构。`map/` 目录中的 MOC（Map of Content）文件将 `docs/` 与 `aurora/` 两条文档链统一连接，并与 `src/` 代码链建立交叉引用：

- **知识库入口**: [map/00_START_HERE.md](../map/00_START_HERE.md)
- **MOC 导航**: [宪章](../map/01_manifest.md) · [概念](../map/02_concepts.md) · [ADR](../map/03_adr.md) · [数学](../map/04_math.md) · [工程](../map/05_engineering.md) · [代码](../map/06_code.md) · [洞察](../map/07_insights.md) · [标签](../map/99_tag_index.md)

> 用 Obsidian 打开项目根目录，所有 `[[链接]]` 将激活双向导航和图谱视图。

---

**Version**: 0.3.0  
**License**: MIT  
**Last Updated**: 2026-06-20
