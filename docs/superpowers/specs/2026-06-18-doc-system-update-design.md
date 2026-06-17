# Doc System Update — Design Spec

**Date**: 2026-06-18
**Status**: Approved

## Goal

Systematically audit, fix, merge, and supplement trit-core's `docs/` directory so the documentation system is complete, consistent, and free of stale information.

## Scope

### Phase 1: Merge duplicates + fix hard data errors

**P1.1 — Merge `performance-audit.md` → `performance-validation.md`**
- Delete `docs/performance-audit.md`
- Ensure `performance-validation.md` is the single source of performance truth
- Update all cross-references: BENCHMARK.md, FUTURE.md, INDEX.md, README.md, CHANGELOG.md

**P1.2 — Merge `whitepaper.md` → `technical-whitepaper.md`**
- Delete `docs/whitepaper.md` (English, content overlaps with technical-whitepaper.md)
- `technical-whitepaper.md` is the authoritative technical whitepaper (Chinese, more comprehensive)
- `preprint.md` is the separate English academic paper — no overlap issue
- Update all cross-references: INDEX.md, README.md, REVIEWER_GUIDE.md, zh/README.zh.md

**P1.3 — Fix all hard data errors**
Across all docs, fix the following stale numbers:

| File | Current (wrong) | Correct |
|------|----------------|---------|
| QUICKSTART.md | 170 tests | 227 tests |
| CONTRIBUTING.md | 170 tests | 227 tests |
| technical-whitepaper.md L8 | ~3,900 lines / 114 tests | ~6,500 lines / 227 tests |
| code-quality-audit.md L19 | 34 tests | 227 tests |
| README.md L73 | `cargo test --all-features` shows "(34 tests)" | remove count |
| CHANGELOG.md L52 | "end-to-end benchmarks pending" | "end-to-end benchmarks complete; 29 criterion benchmarks across 9 groups; 10,000 TPS target exceeded by 65-101x" |
| CLAUDE.md L112 | "end-to-end benchmarks (JSON I/O, TCP roundtrip, concurrent bus) are planned" | "end-to-end benchmarks (JSON I/O, TCP roundtrip, concurrent bus) complete — see docs/performance-validation.md" |
| zh/architecture-audit.zh.md | likely has stale test count | check and fix |
| zh/whitepaper.zh.md L8 | "~3,900 行 Rust / 114 个测试" | update to current |

### Phase 2: Update stale content

**P2.1 — ARCHITECTURE.md §7 title**
- "分布式协议（M4 + M5）" → "分布式协议（M4–M6）"
- Add M6 seed discovery subsection (brief, ~3 paragraphs)
- Update M4→M5 evolution table to M4→M5→M6

**P2.2 — FUTURE.md §4**
- Current title: "性能目标未经验证"
- New title: "性能目标已初步验证"
- Update content: mention performance-validation.md, cite key TPS numbers, note that further validation (dhat heap analysis, multi-threaded load tests) is still needed

**P2.3 — BENCHMARK.md**
- Update criterion groups from 5 to 9 (add: pipeline, tcp_roundtrip, concurrent_bus, json_serde)
- Add the 4 new groups with descriptions
- Update performance numbers to match performance-validation.md
- Update JSON serde bottleneck note (was "90%" now "25-49%")

**P2.4 — MODULES.md §net/ header**
- "`net/` — 分布式协议（M4，存根）" → "`net/` — 分布式协议（M4–M6）"
- Add discovery.rs row to file table
- Remove "(存根)" from text

**P2.5 — GLOSSARY.md**
- Update Hot Path latency: "~3ns" → "~1.5ns" (match benchmark data)
- Update Cold Path latency: "~95ns" → "~95ns" (unchanged, confirmed)

### Phase 3: Full Chinese mirror sync

**P3.1 — zh/README.zh.md**
- Add `src/net/` to project structure table
- Add `src/baseline/` to project structure table (currently missing)
- Update test count
- Add new doc links: performance-validation.md, REVIEWER_GUIDE.md

**P3.2 — zh/api.zh.md**
- Verify §5 (net module) matches English api.md
- Add any missing M5/M6 API entries

**P3.3 — zh/roadmap.zh.md**
- All M1-M6 checkboxes already `[x]` — verify
- MVP DoD has discrepancy: Chinese version marks both items `[x]`, English version has them `[ ]`. Leave as-is — this reflects a factual question (repo publicity, human reviewer status) that requires user decision.

**P3.4 — zh/whitepaper.zh.md**
- Fix code scale data (line 8: ~3,900→current, 114→227)
- Verify no other stale data

**P3.5 — zh/preprint.zh.md**
- Check for hard data errors (test count, line count)
- Fix as needed

**P3.6 — zh/architecture-audit.zh.md**
- Fix test count reference if stale

### Phase 4: Reorganize navigation — new "Reports & Audits" layer

**P4.1 — INDEX.md**
Add a new 6th layer between "深度洞察" and "历史文档":

```
### 📊 第六层：报告与审计

| 文档 | 内容 |
|---|---|
| [validation-report](validation-report.md) | M2 三元 vs 二元对比验证 |
| [performance-validation](performance-validation.md) | 端到端性能验证（TPS 对比、瓶颈分析） |
| [security-audit](security-audit.md) | 应用安全审计（P1/P2 已修复） |
| [code-quality-audit](code-quality-audit.md) | 代码质量审计（SOLID/DRY/复杂度） |
| [REVIEWER_GUIDE](REVIEWER_GUIDE.md) | 评审者指引（核心声明验证步骤） |
```

Remove these 5 files from the "历史文档" appendix. Keep adr/, zh/, preprint, whitepaper (if not deleted), roadmap, api, CHANGELOG in "历史文档".

**P4.2 — README.md**
- Sync the "Deep Dives" and "Historical Documents" sections with INDEX.md changes
- Update document counts and descriptions

**P4.3 — REVIEWER_GUIDE.md**
- Update document navigation table if any linked docs changed

### Phase 5: Fill remaining gaps

**P5.1 — QUICKSTART.md**
- Test count: 170 → 227
- Benchmark groups mention: add note about 9 criterion groups
- Update distributed node section to mention M6 discovery flags

**P5.2 — CONTRIBUTING.md**
- Test count: 170 → 227
- Add mention of multi_node_test.rs in test types table
- Ensure benchmark description matches current state

**P5.3 — CLAUDE.md**
- Fix known limitations: benchmark status

**P5.4 — CHANGELOG.md**
- Fix "end-to-end benchmarks pending"

**P5.5 — README.md**
- Update test count reference
- Update doc links for deleted whitepaper.md (if deleted)
- Update net/ description: "M4" → "M4-M6"

## Out of Scope

- Core concept docs (PHILOSOPHY.md, CONCEPTS.md, WHAT_IS_TRIT.md) — content is accurate
- ADR series (4 files) — unchanged
- Scenario files and test code
- Creating new documents (this is a cleanup pass, not an expansion)
- zh/ADR translations — verified to exist, content mirrors English ADRs

## Verification

```bash
# After all changes, run:
grep -rn "170" --include="*.md" docs/ README.md CLAUDE.md CHANGELOG.md
# Should return ZERO matches

grep -rn "3,900" --include="*.md" docs/
# Should return ZERO matches

grep -rn "114" --include="*.md" docs/
# Should return ZERO matches (as test count)

grep -rn "pending" --include="*.md" docs/ CHANGELOG.md CLAUDE.md
# Only valid "pending" usages (not referring to benchmarks)

cargo test --all-features -- --test-threads=1  # must pass
cargo clippy --all-targets --all-features -- -D warnings  # must pass
```
