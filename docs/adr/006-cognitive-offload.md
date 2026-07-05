# ADR-006: Cognitive Offload Protocol

## Status
Accepted

## Context

Trit-Core's core decision engine produces one of three outcomes: `True` (commit), `False` (reject), or `Hold` (suspend judgment). When the system returns `Hold`, it knows *that* it can't decide — but the caller (human, UI, downstream system) doesn't know *why*. Is it a data conflict? A missing variable? A domain boundary where trit-core simply isn't competent?

Without structured metadata about the Hold, the human is left to guess. In an AI alignment context, this is dangerous: a hold caused by a violated anchor constraint (e.g., ecological threshold exceeded) is qualitatively different from a hold caused by two news sources disagreeing about an event. Conflating them is a category error.

## Decision

Introduce a `CognitiveOffload` protocol: when trit-core returns `Hold`, it also returns a structured explanation of *why* it held.

```rust
/// Structured explanation for a Hold decision.
///
/// This is NOT an "answer." It's a map of what's missing, what's conflicting,
/// and what cognitive operations the human might try next.
pub struct CognitiveOffload {
    /// The primary reason the system could not decide
    pub reason: HoldReason,

    /// Which sources are in conflict (if any)
    pub conflicting_sources: Vec<SourceConflict>,

    /// Variables that were absent from the input but would have helped
    pub missing_variables: Vec<String>,

    /// Concrete suggestions for what additional information would resolve the hold
    pub what_would_help: Vec<String>,

    /// Suggested cognitive operations (not "answers")
    pub suggested_cognitive_ops: Vec<String>,
}

pub enum HoldReason {
    /// Two or more sources disagree about the same signal
    SourceConflict,

    /// Not enough data to reach any conclusion
    InsufficientData,

    /// Signals from incommensurable frames cannot be collapsed
    FrameMismatch,

    /// The decision crosses a domain boundary where trit-core has no competence
    DomainBoundary,

    /// An anchor constraint (thermal, ecological, wellbeing) was violated
    AnchorViolation,

    /// Other reason not covered above
    Other(String),
}

pub struct SourceConflict {
    pub signal_a: String,
    pub frame_a: String,
    pub signal_b: String,
    pub frame_b: String,
    pub nature: String, // e.g. "contradictory CO₂ readings"
}
```

### Auto-population in the pipeline

`SandboxPipeline::stage_build_output()` automatically builds a `CognitiveOffload` when the final word is `Hold`:

| Diagnostic signal | Inferred `HoldReason` |
|-------------------|-----------------------|
| `anchor_report` has violations | `AnchorViolation` |
| `interrupts` contain `FrameMismatch` | `FrameMismatch` |
| `interrupts` contain `ExplainImpulse` | `InsufficientData` |
| `interrupts` is empty | `DomainBoundary` |
| Fallback | `Other("unresolved conflict")` |

### Output field

`SandboxOutput` carries `cognitive_offload: Option<CognitiveOffload>` — `None` when the decision is `True` or `False`, `Some(...)` when `Hold`.

## Consequences

### Positive
- **Auditable Holds**: Every Hold comes with a machine-readable reason. Humans and downstream systems don't need to guess.
- **Debugging aid**: During development, `CognitiveOffload` makes it obvious when a Hold is a bug (wrong frame) vs. a feature (genuine source conflict).
- **Human-in-the-loop UX**: A UI can render `what_would_help` as actionable suggestions ("Add a temperature data source from this region").
- **Graduation metric**: If the user consistently provides the `missing_variables` that resolve holds, the system is doing its job — expanding the user's reference frame.

### Negative
- **HoldReason taxonomy may grow**: The initial 5 variants + `Other` are coarse. As more data sources and domains are added, new hold reasons will emerge.
- **Auto-population is heuristic**: Mapping `MetaInterrupt` variants to `HoldReason` is a best-effort heuristic. It can misclassify complex cases.
- **No learning from resolved holds**: The system doesn't track which `what_would_help` suggestions actually resolved the user's uncertainty. Future ADRs may add a feedback loop.

## See Also
- ADR-005: Instrumental Frame (used by CognitiveOffload for `missing_variables` suggestions)
- `src/meta/interrupt.rs`: `HoldReason`, `CognitiveOffload`, `SourceConflict`
- `src/sandbox/pipeline.rs`: `build_cognitive_offload()`
- `src/sandbox/output.rs`: `SandboxOutput.cognitive_offload`
