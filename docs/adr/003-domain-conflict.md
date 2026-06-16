# ADR-003: Domain-Based Conflict Resolution

## Status
Accepted

## Context
When a Science-frame signal and an Individual-frame signal collide (e.g., clinical trial says drug works, but patient is allergic), the system cannot simply "average" them. Different domains require different arbitration rules.

We need a **policy engine** that selects which frame dominates—or whether to remain undetermined—based on the operational context.

## Decision
Implement a **ResolutionPolicy** with five hardcoded domains:
- Physical / Engineering: Science frame dominates; forced collapse allowed.
- MedicalEthics: Individual frame dominates; no forced collapse.
- ValueJudgment: No frame dominates; must remain undetermined.
- General: Attempt negotiation; fall back to Hold if frames mismatch.

## Consequences

### Positive
- **Hard safety boundaries**: Physical and engineering contexts enforce empirical truth, preventing relativism where it is dangerous.
- **Agency preservation**: Medical and ethical contexts prevent the system from overriding the user's context with statistical averages.
- **Explicit incommensurability**: ValueJudgment domain makes it impossible for the system to fake consensus when there is none.

### Negative
- **Domain taxonomy is finite and western-biased**: The five domains may not cover all cultures or future AI alignment scenarios. Extension requires code changes, not runtime configuration.
- **Risk of policy ossification**: If domain rules are hardcoded, they become difficult to update without recompilation. We may need a domain-rule DSL in future ADRs.
- **No learning**: The policy engine does not learn from outcomes. It is a rule-based system, not an ML policy.

## Alternatives Considered
- **ML-trained arbitrator**: Rejected. Using RL to learn conflict resolution would reintroduce the RLHF problem we are trying to solve (statistical averaging overwriting individual cases).
- **User-configurable weights**: Rejected. Users should not need to tune hyperparameters to receive ethical treatment. The system should have principled defaults.
- **Market-based consensus**: Rejected. Economic or voting mechanisms would privilege majority frames, violating the minority-frame protection goal.

## Validation Criteria
1. For every domain, a unit test must demonstrate the correct arbitration outcome for a 3-frame conflict (Science vs. Individual vs. Consensus).
2. MetaInterrupt must log the policy action taken for auditability.
3. ValueJudgment domain must never output a committed True or False state.

## References
- IEEE P7000 (Model Process for Addressing Ethical Concerns during System Design).
- Medical ethics: Beauchamp & Childress, *Principles of Biomedical Ethics* (autonomy principle).
