# Trit-Core MVP Roadmap

**Version**: 0.1.0  
**Status**: Draft  
**Last Updated**: 2026-06-17

---

## Milestones

### M0: Foundation (Week 0–1)
**Goal**: Project skeleton + core algebra + unit tests.

**Deliverables**:
- [x] `Cargo.toml` with dependencies
- [x] `src/lib.rs` public API
- [x] `src/trit/` module (TritValue, Phase, TernaryAlgebra)
- [x] `src/frame/` module (Frame registry)
- [x] `src/meta/` module (MetaMonitor, ResolutionPolicy, 5 domains)
- [x] Unit tests for TAND, TOR, TNOT across all 9 same-frame combinations
- [x] Unit tests for cross-frame conflict detection
- [x] `#![forbid(unsafe_code)]` enforced

**Acceptance Criteria**:
- `cargo test` passes 100%.
- No compiler warnings (deny with `#[deny(warnings)]`).
- Code coverage > 80% for `trit/` and `meta/`.

---

### M1: Sandbox CLI (Week 1–2)
**Goal**: Runnable CLI tool that consumes scenario JSON and produces decision logs.

**Deliverables**:
- [x] `src/bin/sandbox.rs` CLI argument parsing (`--scenario <path>`)
- [x] JSON input schema validation (ScenarioInput, SignalInput)
- [x] JSON output serialization (SandboxOutput)
- [x] `src/sandbox/` module (pipeline engine)
- [x] 5 sample scenario JSON files in `scenarios/`
- [x] Integration test: run CLI on all scenarios, assert expected behavior

**Acceptance Criteria**:
- `cargo run --bin trit-sandbox -- --scenario scenarios/example.json` produces valid JSON.
- All 5 sample scenarios pass end-to-end.
- Decision logs contain non-empty `interrupts` for cross-frame cases.

---

### M2: Scenario Validation Suite (Week 2–3)
**Goal**: Expand to 10–20 human-centric advisory cases; validate against binary baseline.

**Deliverables**:
- [x] 10–20 scenario JSON files covering:
  - Medical ethics (3 cases)
  - Career/value conflict (3 cases)
  - Physical safety (2 cases)
  - Engineering trade-off (2 cases)
  - General negotiation (2 cases)
- [x] Binary baseline comparator (simple majority rule, no Hold state)
- [x] Comparison report: for each case, note where binary baseline fails to preserve conflict
- [x] `docs/validation-report.md` summarizing findings

**Acceptance Criteria**:
- At least 5 cases must demonstrate binary baseline producing a "smoothed" or "overriding" output where Trit-Core correctly outputs Hold.
- Report is reviewable by non-technical stakeholders (plain language summary per case).

---

### M3: Preprint & Open Source (Week 3–4)
**Goal**: Package code, documentation, and validation report for public release.

**Deliverables**:
- [x] GitHub repository initialized with `main` branch
- [x] MIT LICENSE
- [x] README.md with build instructions and architecture diagram
- [x] `docs/whitepaper.md` finalized
- [x] `docs/adr/` complete with 3 ADRs
- [x] Preprint markdown (10–15 pages) in `docs/preprint.md`
- [ ] crates.io publication (optional, if stable)

**Acceptance Criteria**:
- `cargo build --release` succeeds on stable Rust (1.70+).
- Preprint contains: abstract, problem statement, architecture, validation results, limitations, references.
- At least one external reviewer (human) has read the preprint and provided feedback.

---

### M4: Distributed Prototype (Post-MVP, Optional)
**Goal**: Multi-node harmonic coupling over localhost/network.

**Deliverables**:
- [ ] `src/net/` module (Node, Resonate, Decouple)
- [ ] Phase-lock loop (PLL) simulation for node synchronization
- [ ] `trit-node` binary for running a sovereign node
- [ ] Docker compose setup for 3-node local cluster

**Acceptance Criteria**:
- 3 nodes with different domains can couple and produce a negotiated Hold output.
- Nodes can decouple without global consensus failure.

---

## Risk Register

| Risk | Likelihood | Impact | Mitigation |
|------|-----------|--------|------------|
| Rust learning curve delays M1 | Medium | Low | Pair programming; accept "good enough" code quality for MVP |
| Scenario design is subjective | High | Medium | Use anonymized real-world anecdotes; include binary baseline for contrast |
| No academic reviewer available | Medium | High | Post to arXiv and Hacker News; seek community feedback |
| Performance overhead too high | Low | Medium | Benchmark early; if >5× slower, accept for MVP and optimize post-M4 |

---

## Definition of Done (MVP)

- [ ] Code compiles, tests pass, no unsafe blocks.
- [ ] 10–20 scenarios with binary comparison.
- [ ] Whitepaper + ADRs + preprint complete.
- [ ] GitHub public repository live.
- [ ] At least one human reviewer has validated the claim: "Trit-Core preserves conflict better than binary RLHF proxies."
