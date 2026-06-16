# ADR-002: Phase Arithmetic

## Status
Accepted

## Context
In a ternary system, the discrete value {-1, 0, +1} alone is insufficient to model real-world decision-making. A doctor may "lean toward" treatment A without being fully committed. A user may be "mostly sure" they want to quit their job but retain residual doubt. 

We need a continuous variable that captures **commitment tendency** without collapsing into a discrete decision.

## Decision
Attach a **Phase** (continuous float, 0.0..1.0) to every TritWord. 0.5 = neutral. Values above 0.5 trend toward True; below 0.5 trend toward False.

## Consequences

### Positive
- **Smooth gradients**: Enables arithmetic-like operations (averaging, complement, weighted blending) impossible with discrete ternary alone.
- **Pre-commitment visibility**: A system can output "Hold with phase 0.8" to signal "I am not deciding yet, but I am strongly leaning toward True." This is more informative than a raw probability.
- **Resonance potential**: Future distributed versions can use phase difference to compute interference patterns (constructive/destructive).

### Negative
- **Float precision issues**: Phase arithmetic introduces rounding errors. We must cap operations at 6 decimal places and provide an epsilon tolerance for equality checks.
- **State explosion**: Each trit now carries a value, a phase, and a frame. Memory footprint per trit is higher than a boolean.
- **No hardware float-free path**: For FPGA or embedded targets, phase arithmetic would require fixed-point approximations (future ADR).

## Alternatives Considered
- **Discrete phase levels (e.g., 5-step)**: Rejected. Too coarse. Real-world decision gradients require smooth resolution.
- **Separate confidence vector**: Rejected. Would decouple confidence from the trit, making the algebra more complex. Phase is an intrinsic property.
- **Probability (Bayesian)**: Rejected. Probability implies a distribution over possible worlds. Phase implies a *tendency within a single world*—it is not normalized, does not sum to 1, and does not require prior distributions.

## Validation Criteria
1. Phase averaging must be associative and commutative within epsilon.
2. Complement(Complement(phase)) must equal original phase within epsilon.
3. No phase value may exceed 1.0 or fall below 0.0 after any valid operation (panic/clip in debug, clip in release).

## References
- Harmonic oscillator physics (classical, not quantum): phase-locking and synchronization theory.
- Control systems: PID-like error correction via phase difference.
