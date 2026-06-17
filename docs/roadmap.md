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
- [x] `docs/technical-whitepaper.md` finalized
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
- [x] `src/net/` module (Node, Resonate, Decouple)
- [x] Phase-lock loop (PLL) simulation for node synchronization
- [x] `trit-node` binary for running a sovereign node
- [x] Docker compose setup for 3-node local cluster

**Acceptance Criteria**:
- 3 nodes with different domains can couple and produce a negotiated Hold output.
- Nodes can decouple without global consensus failure.

---

### M5: TCP Transport Layer (Week 4–5)
**Goal**: Real network transport for distributed nodes via TCP.

**Deliverables**:
- [x] `src/net/frame_codec.rs` — Length-prefix framing protocol (4-byte BE length + JSON, max 1 MiB)
- [x] `src/net/tcp_server.rs` — `TcpNodeServer` with tokio async accept/dispatch
- [x] `src/net/tcp_client.rs` — `TcpClient` with resonate/decouple/heartbeat/negotiate methods
- [x] Tests: frame roundtrip (small/empty/large/oversized/multi-frame), server bind/accept/heartbeat/resonate/decouple, client connect/heartbeat/resonate/decouple

**Acceptance Criteria**:
- Full duplex TCP communication between nodes.
- Length-prefix framing handles binary-safe JSON payloads up to 1 MiB.
- Rejects oversized frames to prevent CWE-770 memory exhaustion.

---

### M6: Seed Node Discovery (Week 5)
**Goal**: Automatic peer discovery at startup via seed nodes.

**Deliverables**:
- [x] `src/net/discovery.rs` — `parse_seeds()` and `bootstrap()` functions
- [x] `trit-node` CLI upgraded with `--port` and `--peers` flags + `TRIT_PEERS` env var
- [x] `docker-compose.yml` full TCP mesh: 3 nodes (Science:9000, Individual:9001, Consensus:9002)
- [x] Discovery unit tests + 9 multi-node integration tests

**Acceptance Criteria**:
- Nodes discover each other via HEARTBEAT exchange at startup.
- Docker Compose cluster forms full mesh automatically.
- All seeds unreachable = graceful standalone mode.

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

- [x] Code compiles, tests pass, no unsafe blocks.
- [x] 10–20 scenarios with binary comparison.
- [x] Whitepaper + ADRs + preprint complete.
- [ ] GitHub public repository live.
- [ ] At least one human reviewer has validated the claim: "Trit-Core preserves conflict better than binary RLHF proxies."
