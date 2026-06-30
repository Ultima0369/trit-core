# Design Spec: Philosophy-to-Docs Integration

**Date**: 2026-06-24
**Status**: Draft → Review
**Scope**: Add 2 new insight documents to `docs/explanation/insights/`, cross-link with existing docs

---

## 1. Goal

Integrate seven core insights from the Trit-Core evaluation dialogue into the project documentation system as standalone deep-dive articles. These insights are not yet captured in the existing docs.

## 2. Design Decision: New Standalone Files (Option A)

Two new files under `docs/explanation/insights/`:

| File | Theme | Role |
|------|-------|------|
| `EULER-HOMOLOGY.md` | Euler identity as cognitive architecture blueprint; mathematics as mind's self-portrait; three-layer mind model | Math metaphor + cognitive science |
| `TIANDIREN-MATRIX.md` | Heaven-Earth-Human knot matrix; time is not uniform; information as mind-world interface event; Buddhist epistemology in engineering; compute-constrained optimization | Architecture vision + epistemology |

They form a triangle with the existing `PHILOSOPHY.md`:
- **PHILOSOPHY.md** — why Trit-Core must exist (thermodynamics, myelination, incommensurability)
- **EULER-HOMOLOGY.md** — the mathematical structure inside the system
- **TIANDIREN-MATRIX.md** — the world-modeling framework outside the system

## 3. File Specifications

### 3.1 `docs/explanation/insights/EULER-HOMOLOGY.md`

**Sections**:

1. **Homology Mapping Table** — `e^(iπ) + 1 = 0` term-by-term mapping to Trit-Core: 1=proposition, i=Hook rotation, π=self-monitoring stop, e=multi-transform, -1=Hold, +1=return, 0=unity. Core claim: Euler's identity is not *used* by Trit-Core but *re-implemented* as cognitive architecture.

2. **π: Self-Monitoring + Active Stop** — π is not 3.14159 but `monitor(S) → stop_condition_met ? STOP : CONTINUE`. Formal definition of "completed a full circle". Maps to `MetaMonitor` + `HoldBudget`.

3. **e: Multiple Function Implementations** — Fourier=time→frequency domain (celestial), Wavelet=localized time-frequency (earthly events), Phase-space reconstruction=nonlinear chaos (human). Dynamic transform selection based on signal features. Maps to `FftWaveletEngine` and future wavelet engines.

4. **i: Hook as Rotation Operator** — i²=-1 as cognitive operation: first i rotates from common-sense to discipline-frame, second i rotates back, original judgment is negated (not error, but spiral ascent). i feeds back to e: Hook choice determines transform choice. Maps to `ScenarioRecognizer` + `MountArbiter`.

5. **From 1 to 0: Complete Decision Path** — Six stages: input 1 → apply i → select e → π iterative recursion → arrive at -1 (Hold) → +1 (return with audit trail) → = 0 (difference dissolves).

6. **Euler's Identity as Mind's Self-Portrait** — Original thesis: Euler's identity is beautiful not because it describes the universe, but because it describes the mind's complete cognitive cycle. Academic genealogy: Kant → Piaget → Lakoff & Núñez → Rotman → this position. Web search result: this specific claim appears to be unstated in published literature. Distinction between "phenomena are illusory" and "material world is illusory".

7. **Three-Layer Mind Model** — Rational mind (layer 3, Trit-Core current), Somatic mind (layer 2, needs `Frame::Somatic` + `Hook::Interoception`), Environmental mind (layer 1, needs `Anchor::Environmental_Baseline`). π stops only when all three layers agree.

8. **Back to Code** — Concrete file/module paths mapped to Euler components. Reader can go from philosophy to `cargo test`.

### 3.2 `docs/explanation/insights/TIANDIREN-MATRIX.md`

**Sections**:

1. **Heaven-Earth-Human Knot Matrix** — Three-layer grid architecture diagram. Each layer's "knot" = Frame + constant library. Cross-layer knot = bidirectional mapping. Full-layer knot = recursive cross-validation of all three. Compute budget determines how deep and how many recursions.

2. **Heaven (Celestial) Layer** — Not "weather + calendar". Four dimensions: climate & ecology (ω=10.0), action timing (Kairos, not Chronos), hardware timing constraints, all things' intrinsic ωᵢ. Maps to `HarmonicClock` + `EcologicalBase`.

3. **Earth (Earthly) Layer** — Not "latitude + longitude". Four dimensions: spatial geography, historical sedimentation (events layered in space), custom/folkway modeling (spatiotemporal patterns of collective behavior), cultural topology (symbol systems and meaning networks). Maps to `Frame::Consensus` + `Frame::Relational`.

4. **Human Layer** — Physical body (`SurvivalMotives`, `ThermalBaseline`), rational reasoning (`Frame::Science`), emotional experience (`Frame::Individual`, `Frame::FirstPerson`), unity at the peak (beyond `Frame::Meta`).

5. **Time Is Not Uniform** — Each entity has its own intrinsic frequency ωᵢ. System's perceived time at moment t = sampling coverage of ωᵢ × sampling frequency match. Maps to `HarmonicClock` presets and future multi-clock parallelism.

6. **Mathematical Definition of "Surprise"** — Surprise(E) = 1 - P(detected precursor changes of E | system's ωᵢ covers E's time-scale). When sampling frequency << entity's natural change frequency → surprise is inevitable. Not a defect — cognitive boundary awareness. Maps to `Unknown` + `SafeFallback`.

7. **Information = Mind-World Interface Event** — Overturns Shannon: information is not an independently existing signal, but an interface event produced when mind touches world. Five senses are lossy encoders (narrow bandwidth). Language/symbols are further lossy quantization. Maps to `Phase` — Phase is not "objective probability" but "system's tendency strength about its own judgment."

8. **"境不自境，因心故境" (Environment Does Not Self-Environment)** — Fourteen Buddhist characters that say what science took centuries to circle back to. Everything Trit-Core processes — Frame, Phase, Hook, Hold — none are "objective properties of the real world." They are all "相" (the intermediate product when mind meets environment). This is not nihilism — it's using tools more soberly after knowing their limits.

9. **Compute-Constrained Approximation to Optimal** — Compute decides depth, attitude decides direction. Abundant compute → deep recursive audit. Limited compute → shallow Hold + admit unknown + request more data. Regardless of compute: never pretend to know, never forcibly dissolve conflict, never treat Hold as failure. Maps to `ComputeBudget` + `DepthLevel` + `HoldStrategy`.

10. **From Vision to Code** — Map Heaven-Earth-Human matrix back to existing modules and file paths. What's implemented (Anchor layer, Frame system, HarmonicClock), what's missing (dynamic Frame registration, multi-clock parallelism, somatic/environmental layers).

## 4. Cross-Linking Plan

### Links INTO the new docs (from existing files):
- `PHILOSOPHY.md` §10 (summary): add pointer to EULER-HOMOLOGY and TIANDIREN-MATRIX
- `EPISTEMIC-HUMILITY.md` "Related Documents" section: add both new files
- `docs/INDEX.md`: add both under `explanation/insights/`
- `FUTURE.md`: reference TIANDIREN-MATRIX in §5 (Frame types) and §1 (formal verification)

### Links OUT of the new docs (to existing files):
- Both link to `PHILOSOPHY.md`, `CONCEPTS.md`, `EPISTEMIC-HUMILITY.md`
- EULER-HOMOLOGY links to `ARCHITECTURE.md`, `src/core/`, `src/hook/`, `src/meta/`
- TIANDIREN-MATRIX links to `ARCHITECTURE.md`, `FUTURE.md`, `src/anchor/`, `src/clock.rs`, `src/budget/`

### Cross-links between the two:
- EULER-HOMOLOGY §6 → TIANDIREN-MATRIX §7 (information as interface event)
- EULER-HOMOLOGY §7 → TIANDIREN-MATRIX §4 (three-layer mind ↔ human layer)
- TIANDIREN-MATRIX §1 → EULER-HOMOLOGY §1 (knot matrix ↔ Euler mapping)

## 5. Writing Principles

1. **Chinese primary, English terms where precise** — Match existing `PHILOSOPHY.md` and `CONCEPTS.md` convention
2. **Code-backed claims** — Every abstract claim gets a concrete file path or module name
3. **Reminder, not instruction** — Follow `EPISTEMIC-HUMILITY.md` tone; these are working hypotheses, not closed doctrines
4. **Map to runnable code** — Each section should let the reader go from insight to `cargo test`
5. **No religious language** — Per `PHILOSOPHY.md` §11.5: strip religious context, keep the cognitive kernel

## 6. What Does NOT Change

- `PHILOSOPHY.md` — add one cross-reference line in §10, no content changes
- `CONCEPTS.md` — unchanged (these are insights, not concept definitions)
- `ARCHITECTURE.md` — unchanged
- `EPISTEMIC-HUMILITY.md` — add links to new files in "Related Documents" section
- `FUTURE.md` — add one reference in §5
- `docs/INDEX.md` — add two entries under insights/
- All code, tests, scenarios — unchanged

## 7. Out of Scope

- Writing the actual full content of the two .md files (that's implementation)
- Creating new Frame variants or code changes
- Updating Aurora documentation
- Chinese-only: no English mirror for these insight files (consistent with existing insights/)
