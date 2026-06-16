# Trit-Core Technical Specification

**Version**: 0.1.0-alpha  
**Date**: 2026-06-17  
**Status**: MVP Architecture Draft

---

## 1. Problem Statement

Binary AI alignment systems force a definitive output for every input, collapsing conflicting evidence into a smoothed average. In human-centric advisory scenarios (medical, ethical, emotional), this overwrites individual context and removes the option to suspend judgment.

**Goal**: Build a ternary decision engine that supports an explicit undetermined state, detects cross-domain conflicts, and resolves them according to principled safety rules rather than statistical averaging.

---

## 2. System Architecture

### 2.1 Layer Stack

```
┌─────────────────────────────────────┐
│  Application Layer (CLI / API)        │  trit-sandbox binary
├─────────────────────────────────────┤
│  Sandbox Engine                       │  Scenario parsing, JSON I/O
├─────────────────────────────────────┤
│  Policy Engine (Meta-Monitor)         │  Domain rules, arbitration
├─────────────────────────────────────┤
│  Ternary ALU (Harmonic Ternary Alg) │  TAND, TOR, TNOT, phase math
├─────────────────────────────────────┤
│  Frame Registry                       │  Context domain management
├─────────────────────────────────────┤
│  Trit Data Model                      │  TritWord { value, phase, frame }
└─────────────────────────────────────┘
```

### 2.2 Data Model

**TritWord**: The atomic unit.
- `value`: TritValue enum { True(+1), Hold(0), False(-1) }
- `phase`: f64 in [0.0, 1.0], 0.5 = neutral
- `frame`: Frame enum { Science, Individual, Consensus, Absolute, Meta }

**MetaInterrupt**: Runtime conflict record.
- `conflict`: ConflictType enum
- `reason`: Human-readable string
- `timestamp`: UTC DateTime

**ResolutionPolicy**: Hardcoded domain arbitration.
- `domain`: Domain enum
- `arbitrate(inputs: &[TritWord]) -> ArbitrationResult`

---

## 3. Core Logic Specification

### 3.1 TAND (Harmonic Conjunction)

Same frame:
- TT → T
- TF or FT or FF → F
- Any Hold involved → Hold
- Phase = mean(a.phase, b.phase)

Different frame:
- Output = Hold(0.5, Meta)
- Trigger MetaInterrupt(ConflictType::FrameMismatch)

### 3.2 TOR (Harmonic Disjunction)

Same frame:
- Any T → T
- FF → F
- Other → Hold
- Phase = mean(a.phase, b.phase)

Different frame:
- Output = Hold(0.5, Meta)
- Trigger MetaInterrupt(ConflictType::FrameMismatch)

### 3.3 TNOT (Phase Negation)

- True → False, phase = 1.0 - phase
- False → True, phase = 1.0 - phase
- Hold → Hold, phase unchanged

### 3.4 Policy Arbitration

| Domain | Priority Frame | Collapse Behavior |
|--------|---------------|-------------------|
| Physical | Science | Hard collapse to Science; if absent, force nearest |
| Engineering | Science | Hard collapse to Science; if absent, force nearest |
| MedicalEthics | Individual | Preserve individual; never force |
| ValueJudgment | None | Always Hold |
| General | None | Commit if all same frame; else Negotiate (Hold) |

---

## 4. I/O Specification

### 4.1 Input Format (JSON)

```json
{
  "id": "medical_conflict_01",
  "description": "Patient allergic to recommended drug",
  "domain": "MedicalEthics",
  "signals": [
    { "frame": "Science", "value": 1, "phase": 0.8 },
    { "frame": "Individual", "value": -1, "phase": 0.2 }
  ],
  "expected_behavior": "hold"
}
```

### 4.2 Output Format (JSON)

```json
{
  "scenario_id": "medical_conflict_01",
  "final_value": 0,
  "final_frame": "Meta",
  "final_phase": 0.5,
  "interrupts": [
    "FrameMismatch: TAND conflict: Science vs Individual"
  ],
  "policy_action": "Preserve(Individual)"
}
```

---

## 5. Performance Targets (MVP)

- **Throughput**: 10,000 scenario evaluations per second on a single core (acceptable for research/demo).
- **Latency**: P99 < 1ms per scenario.
- **Memory**: < 50MB RSS for 1000 concurrent scenarios.
- **Correctness**: 100% pass rate on validated scenario suite (10–20 cases).

---

## 6. Security & Safety

- **No network I/O in core library**: The `trit_core` crate must be pure computation; networking is sandbox-only.
- **No unsafe code**: `#![forbid(unsafe_code)]` in lib.rs.
- **Deterministic output**: Same input + same domain must always produce same output (no RNG in core logic).
- **Auditability**: Every interrupt and policy action is logged with UTC timestamp and reason string.

---

## 7. Future Work (Post-MVP)

- **Distributed node protocol**: T_RESONATE / T_DECOUPLE for multi-node phase locking.
- **Hardware emulation**: FPGA-based ternary gate simulation using multi-voltage thresholds.
- **Formal verification**: Coq/Lean proof of safety properties (Hold on cross-frame, no policy override in MedicalEthics).
- **Domain Rule DSL**: Allow runtime policy configuration without recompilation.

---

## 8. References

- ADR-001: Ternary Logic over Binary Logic
- ADR-002: Phase Arithmetic
- ADR-003: Domain-Based Conflict Resolution
- Knuth, D.E. *The Art of Computer Programming* (Vol. 2, balanced ternary).
- Brusentsov, N.P. *Ternary Computers: The Setun and the Setun 70*.
