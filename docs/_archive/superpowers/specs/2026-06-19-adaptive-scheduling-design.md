# Adaptive Scheduling Layer: Unified Layers 4–5

**Date:** 2026-06-19
**Status:** design
**Scope:** `src/attention/`, `src/knowledge/`, `src/clock.rs`, `src/budget/` (new), `src/calibration/` (new)

## Motivation

The current pipeline has three disconnected pieces that share a single purpose —
dynamic resource allocation — but don't talk to each other:

| Component | Current state | Problem |
|-----------|--------------|---------|
| `AttentionScheduler` | Static `bandwidth: f64`, never updated | Can't adapt to load |
| `HarmonicClock` | Marked "experimental", not in pipeline | No temporal context |
| `SelfKnowledge::calibrate()` | Exists but never called | No feedback loop |

The user's insight: all of computer science is fundamentally a timing problem.
Compute is cheap; the hard problem is *when* to compute, *how deep* to go, and
*what to do* with the result. This layer answers those three questions.

## Design

### Architecture

```
OS telemetry (CPU, mem, threads)
         │
         ▼
   ComputeBudget ── depth_level (1..=5)
         │
    ┌────┼────┐
    ▼    ▼    ▼
Attention Clock SelfKnowledge
    │    │    │
    ▼    ▼    ▼
  CalibrationLog (feedback)
         │
         └──────→ next run weights
```

### New module: `src/budget/`

`ComputeBudget` samples OS-level metrics and produces a `depth_level`:

```rust
pub struct ComputeBudget {
    pub depth_level: DepthLevel,
    pub cpu_load: f64,
    pub mem_pressure: f64,
    pub available_threads: u32,
}

pub enum DepthLevel {
    Minimal = 1,   // TAND + arbitrate only
    Reduced = 2,   // + SafeFallback, no extensions
    Standard = 3,  // + attention, self_knowledge, phase_trace
    Deep = 4,      // + reflexive verify
    Exhaustive = 5,// + multi-path TAND comparison
}
```

`DepthLevel` controls which pipeline stages execute. High CPU load → drop to
Minimal/Reduced. Idle → Deep/Exhaustive. This is the "frequency scaling"
analogy: the system throttles compute depth the way a CPU throttles clock speed.

`ComputeBudget::sample()` uses `sysinfo` or a lightweight `/proc` reader on
Linux, falling back to a conservative default on unsupported platforms.

### Refactored: `AttentionScheduler`

`bandwidth` becomes a function of `DepthLevel`:

| DepthLevel | bandwidth |
|-----------|-----------|
| Minimal   | 0.2       |
| Reduced   | 0.4       |
| Standard  | 0.6       |
| Deep      | 0.8       |
| Exhaustive| 1.0       |

`suggest_reprioritization()` gains a `budget: &ComputeBudget` parameter.
When depth is Minimal, it always returns `Continue` (no time for attention
shifts). When Exhaustive, it runs the full loop-entrainment + overload check.

New: consecutive `HoldCurrent` tracking. If the scheduler suggests Hold for
N consecutive runs (default N=3), it escalates to `Recalibrate`.

### Integrated: `HarmonicClock`

The clock moves from standalone experimental to pipeline-integrated:

- Pipeline owns a `HarmonicClock` instance
- Each `run()` call ticks the clock: `dt` is the elapsed wall-clock time
- Domain selects clock preset: `physical()` for Physical/Engineering,
  `deliberative()` for MedicalEthics/ValueJudgment/General
- `clock.to_phase()` feeds into `AttentionScheduler` as a modulation signal:
  near phase peaks (0.8–1.0), the scheduler is more likely to suggest
  `ShiftTo`; near troughs (0.0–0.2), more likely to `HoldCurrent`

### New module: `src/calibration/`

`CalibrationLog` records every pipeline run:

```rust
pub struct CalibrationLog {
    entries: VecDeque<CalibrationEntry>,
    window_size: usize,  // default 64
}

pub struct CalibrationEntry {
    pub scenario_id: String,
    pub domain: Domain,
    pub result: TritValue,
    pub phase: f64,
    pub interrupt_count: usize,
    pub elapsed_us: u64,
    pub depth_level: u8,
    pub attention_cmd: Option<AttentionCmd>,
}
```

### Feedback loop: `SelfKnowledge::calibrate()` called

After each pipeline run, the system:

1. Records a `CalibrationEntry` into `CalibrationLog`
2. If the result was Hold with interrupts, calls `SelfKnowledge::calibrate()`
   with a negative phase delta (the pattern that produced conflict gets
   weakened)
3. If the result was a clean Commit, calls `calibrate()` with a positive
   delta (the pattern that produced consensus gets strengthened)
4. `infer_receiver_state()` now weights its estimate by calibration count:
   more calibrations → higher confidence ceiling

### Pipeline changes

`SandboxPipeline` gains:

```rust
pub struct SandboxPipeline {
    // ... existing fields ...
    budget: ComputeBudget,
    clock: HarmonicClock,
    calibration_log: CalibrationLog,
}
```

New builder methods: `with_budget()`, `with_clock()`, `with_calibration_log()`.

`run_with_diagnostics()` flow (updated):

```
Stage  1-4:  validate → build_policy → build_trits → registry_check
Stage  5:    TAND cascade
Stage  6-8:  arbitrate → reflexive → SafeFallback
Stage  8b:   sample OS → ComputeBudget.depth_level  ← NEW
Stage  9:    attention (gated by depth_level >= Standard)
Stage  10:   self_knowledge (gated by depth_level >= Standard)
Stage  10b:  clock.tick(elapsed)                     ← NEW
Stage  11:   phase_trace (gated by depth_level >= Standard)
Stage  11b:  anchor_check
Stage  12:   build_output
Stage  13:   calibrate (record + update patterns)    ← NEW
```

### Depth gating

Each optional stage is gated:

```rust
fn should_run_extensions(&self) -> bool {
    self.budget.depth_level as u8 >= DepthLevel::Standard as u8
}
```

When depth < Standard, `attention_cmd` defaults to `Continue`,
`receiver_estimate` is `None`, `phase_trace` is `None`.

## Error handling

- `ComputeBudget::sample()` returns `Result` — on OS error, falls back to
  `DepthLevel::Standard` with `cpu_load = 0.5` (conservative)
- `CalibrationLog` window overflow: oldest entries silently evicted (ring buffer)
- Clock tick with zero `dt`: no-op, returns current phase

## Testing strategy

| Test category | Count | What it covers |
|--------------|-------|----------------|
| `ComputeBudget` unit | 4 | depth_level mapping, OS fallback, thread count, bounds |
| `AttentionScheduler` extended | 3 | depth-gated behavior, consecutive Hold escalation |
| `HarmonicClock` pipeline | 3 | domain→preset mapping, tick accumulation, phase bounds |
| `CalibrationLog` unit | 3 | window eviction, entry recording, empty log query |
| `SelfKnowledge` feedback | 3 | calibrate positive/negative delta, confidence ceiling |
| Pipeline integration | 4 | full run with budget gating, clock tick, calibration write |
| **Total new tests** | **20** | |

## Non-goals (YAGNI)

- Real sensor data streams for `ComputeBudget` (MVP uses OS metrics only)
- Persistent calibration storage (in-memory only for v0.3.0)
- Multi-threaded budget sampling (single-threaded, called once per run)
- `DepthLevel::Exhaustive` multi-path TAND (reserved for future; currently
  behaves identically to Deep)

## Files changed

| File | Change |
|------|--------|
| `src/budget/mod.rs` | **New** — `ComputeBudget`, `DepthLevel` |
| `src/calibration/mod.rs` | **New** — `CalibrationLog`, `CalibrationEntry` |
| `src/attention/scheduler.rs` | Refactor — depth-gated bandwidth, consecutive Hold tracking |
| `src/knowledge/self_model.rs` | Refactor — `calibrate()` called, confidence ceiling |
| `src/clock.rs` | Refactor — pipeline integration, domain→preset mapping |
| `src/sandbox/pipeline.rs` | Refactor — new stages 8b/10b/13, depth gating |
| `src/sandbox/diagnostic.rs` | Add `depth_level: u8`, `clock_phase: f64` |
| `src/lib.rs` | Add `pub mod budget`, `pub mod calibration` |
