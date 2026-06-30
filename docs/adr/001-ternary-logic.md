# ADR-001: Ternary Logic over Binary Logic

## Status
Accepted

## Context
Current AI alignment systems (RLHF, Constitutional AI) rely on binary or probabilistic decision frameworks where a model must eventually collapse to a definitive output (yes/no, good/bad). This creates two problems in human-centric advisory scenarios:

1. **Individual context is overwritten by statistical consensus**: RLHF averages human preferences into a single reward model, losing user-specific nuance.
2. **Forced determinism removes the option to suspend judgment**: When a model encounters conflicting evidence (e.g., medical data suggests treatment A, but patient history contraindicates it), binary systems must choose one, often hiding the conflict behind smoothed language.

We need a computational primitive that supports an explicit **undetermined state** (Hold, 0) alongside True (+1) and False (-1), rather than treating uncertainty as a probability near 0.5.

## Decision
Adopt **Ternary Logic (MVL-3)** as the foundational algebra for the Trit-Core decision engine.

## Consequences

### Positive
- **Conflict transparency**: Cross-domain collisions produce a Hold state rather than a hidden compromise.
- **User agency preservation**: Medical ethics and value-judgment scenarios can remain suspended until the user provides additional context, rather than the system deciding on their behalf.
- **Mathematical efficiency**: Balanced ternary (-1, 0, +1) has optimal radix economy (closest integer to *e*), offering higher information density than binary for equivalent hardware investment.

### Negative
- **No native hardware support**: Current CPUs, GPUs, and TPUs are optimized for binary logic. Software emulation incurs a performance penalty (estimated 2–5× overhead for simple logic operations).
- **Tooling gap**: No standard MVL-3 compilers, debuggers, or formal verification tools exist in the mainstream Rust ecosystem. We must build custom test harnesses.
- **Cognitive overhead for developers**: Engineers trained on binary logic must learn a new algebra and phase arithmetic.

## Alternatives Considered
- **Fuzzy Logic**: Rejected. Fuzzy logic treats uncertainty as a continuous probability between 0 and 1. It does not provide a distinct **Hold** protocol state; instead, it forces the system to commit to a degree of truth. This fails to preserve the *intentionality* of suspension.
- **Quantum-inspired Amplitude**: Rejected. While superposition resembles our Hold state, quantum computing is hardware-bound and overkill. We need a **classical** ternary model that runs on existing silicon via emulation.
- **Extended Binary (multi-bit confidence)**: Rejected. This merely adds precision to binary states; it does not introduce a semantically distinct third state.

## Validation Criteria
1. Unit tests must demonstrate that cross-frame TAND produces Hold + MetaInterrupt.
2. Scenario tests must show that medical-ethics inputs remain undetermined under the MedicalEthics domain policy.
3. Performance benchmarks must quantify the emulation overhead vs. equivalent boolean logic.

## References
- Knuth, D.E. *The Art of Computer Programming*, Vol. 2: Seminumerical Algorithms. Balanced ternary discussion.
- Brusentsov, N.P. *Ternary Computers: The Setun and the Setun 70* (IFIP, 2006).
- Smith, K.C. "The Prospects for Multivalued Logic" (IEEE, 1981).
