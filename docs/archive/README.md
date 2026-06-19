# Document Archive

This directory contains historical snapshots of documents that describe Trit-Core v0.1.x. They are kept in place for traceability and external reference stability.

## Why Archive?

Trit-Core v0.2.0 is a breaking refactor:

- The distributed network layer (`src/net/`, `ResonanceBus`, `trit-node`, TCP/PLL protocol) was removed.
- Core types were restructured into `src/core/` with private fields and `Result`-based constructors.
- `Phase::new` returns `Result`; `TritWord` fields are private.
- Sandbox pipeline and scenario validation were introduced as library code.

The documents in this directory still describe the v0.1.x architecture and milestones. Each archived file has a header note pointing to the current v0.2.0 documentation.

## Current vs. Archived Documentation

| Topic | Current v0.3.0 | Archived v0.1.x |
|---|---|---|
| Roadmap | `../explanation/roadmap.md` | `roadmap-v0.1.0.md` |
| API Reference | `../reference/api.md` | `../zh/reference/api.zh.md` (partially updated) |
| Architecture | `../explanation/ARCHITECTURE.md` | `technical-whitepaper.md` |
| Preprint | N/A (planned) | `preprint.md` |
| Distributed Protocol | N/A (removed) | `../adr/004-distributed-protocol.md` |
| Audits | `../../audit_log/08_reflexive_audit.md` | `../reports/security-audit.md`, `../reports/code-quality-audit.md`, `../reports/cto-audit-report.md` |

## Maintenance Policy

- **Archived files are read-only snapshots.** Do not update them to reflect v0.2.0 changes.
- If a current-version document is written that supersedes an archived file, update the header note in the archived file to point to the new location.
- The top-level `../INDEX.md` separates "Current Documents" from "Historical Documents" to guide readers.
