# ADR-003: Domain-Based Conflict Resolution

> **⚠️ 当前状态注（Current Status）：** 本 ADR 的 "Decision" 节描述的是历史初始设计（五个硬编码 domain）。代码真相源已演化为 **10 个 `Domain` 变体**（`src/meta/domain.rs`）：保留原文的 `Physical` / `Engineering` / `MedicalEthics` / `ValueJudgment` / `General`，加上 `Custom(String)`（外部加载规则，见 `src/meta/rules.rs` 的 `JsonRuleLoader`）以及新增的 `Organizational` / `Relational` / `Cognitive` / `Environmental`。新增四个 domain 分别优先 `Relational` / `Embodied` / `GeoEco` 帧，或在多帧时 `Negotiate`（见 `arbitrate_organizational` / `arbitrate_relational` / `arbitrate_cognitive` / `arbitrate_environmental`）。此外 arbitration 现经 `FrameMask`（`src/meta/frame_mask.rs`，12 位 / 16-bit `u16`）做 O(1) 帧存在检查，并有 FirstPerson 优先级安全门（在 `Physical` / `Engineering` 下跳过）。下方原文作为设计参考保留，计数与列表以本注为准。

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
