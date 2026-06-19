# M2/M3 Validation Report: Ternary vs Binary Baseline

**Date**: 2026-06-19
**Version**: 0.3.0
**Status**: Final

---

## Executive Summary

This report compares Trit-Core's ternary decision protocol against a binary majority-rule baseline across the current `scenarios/*.json` suite. The key finding remains: **in scenarios with genuine domain conflicts, the binary baseline produces a "smoothed" or "overriding" output where Trit-Core correctly identifies and preserves the conflict.**

v0.3.0 introduces `t_and_n` batch TAND, structured observability, and `SandboxDiagnostics`. The scenario suite now contains 16 unique English cases (plus 7 Chinese translations) and the core validation claim — that ternary logic preserves conflicts binary logic cannot express — remains unchanged.

---

## Methodology

### Ternary Protocol (Trit-Core v0.3.0)
- Each signal carries a **frame** (Science, Individual, Consensus, Absolute) and a **phase** (0.0–1.0).
- Cross-frame operations trigger `MetaInterrupt` and produce `Hold` instead of forced collapse.
- `t_and_n` computes the batch TAND for 3+ signals with equal-weight phase averaging, eliminating left-fold bias.
- Domain-specific `ResolutionPolicy` arbitrates conflicts (e.g., MedicalEthics preserves Individual, Engineering/Physical commit Science, ValueJudgment always holds).
- `SafeFallback` forces `False` in dangerous domains when the result is `Unknown` or `Hold` with interrupts.

### Binary Baseline
- Simple majority voting: count True vs False signals.
- Hold signals treated as abstentions.
- Ties default to False (conservative).
- **No concept of frames or domain conflicts.**

---

## Scenario Results

The table below covers the 19 unique English scenarios in `scenarios/*.json`. Each scenario also has a Chinese counterpart in `scenarios/*.zh.json` that shares the same domain, signals, and expected behavior; those bilingual variants are validated automatically but are not counted separately in the comparison.

| # | Scenario file | Domain | Expected | Ternary Result | Binary Result | Conflict Preserved? |
|---|---------------|--------|----------|---------------|---------------|-------------------|
| 1 | `medical_conflict_01.json` | MedicalEthics | `commit_false` | **Preserve(False, Individual)** | False (tie→False) | **Yes** — binary ignores patient-specific risk |
| 2 | `medical_conflict_02.json` | MedicalEthics | `commit_true` | **Preserve(True, Individual)** | False (tie→False) | **Yes** — binary would deny patient autonomy |
| 3 | `medical_conflict_03.json` | MedicalEthics | `commit_false` | **Preserve(False, Individual)** | True (2:1 majority) | **Yes** — binary overrides minority adverse reaction risk |
| 4 | `medical_pain_dismissed.json` | MedicalEthics | `commit_false` | **Preserve(False, Individual)** | False (tie→False) | **Yes** — binary ignores first-person suffering |
| 5 | `career_value_conflict.json` | ValueJudgment | `hold` | **Hold** | False (tie→False) | **Yes** — binary forces a decision on incommensurable values |
| 6 | `career_value_conflict_02.json` | ValueJudgment | `hold` | **Hold** | False (tie→False) | **Yes** |
| 7 | `career_value_conflict_03.json` | ValueJudgment | `hold` | **Hold** | False (tie→False) | **Yes** |
| 8 | `bridge_safety.json` | Engineering | `commit_false` | **Commit(False, Science)** | False (tie→False) | No — both agree |
| 9 | `engineering_bridge_retrofit.json` | Engineering | `commit_false` | **Commit(False, Science)** | False (tie→False) | No — both agree |
| 10 | `engineering_material_tradeoff.json` | Engineering | `commit_false` | **Commit(False, Science)** | False (2:1 majority) | No — both agree |
| 11 | `engineering_evacuation_consensus.json` | Engineering | `commit_false` | **Commit(False, Science)** | False (tie→False) | No — both agree, but Trit-Core records conflict |
| 12 | `physical_crane_overload.json` | Physical | `commit_false` | **Commit(False, Science)** | False (tie→False) | No — both agree |
| 13 | `physical_runway_length.json` | Physical | `commit_false` | **Commit(False, Science)** | False (tie→False) | No — both agree |
| 14 | `general_negotiation.json` | General | `commit_true` | **Commit(True, Science)** | True (2:1 majority) | No — both agree |
| 15 | `general_negotiation_02.json` | General | `negotiate` | **Negotiate** (Hold) | True (2:1 majority) | **Yes** — binary forces a winner in a multi-stakeholder negotiation |
| 16 | `general_conceptual_spin.json` | General | `negotiate` | **Negotiate** (Hold) | True (2:1 majority) | **Yes** — binary forces a decision while reasoning drifts from facts |
| 17 | `value_algorithmic_displacement.json` | ValueJudgment | `hold` | **Hold** | False (tie→False) | **Yes** — binary forces a decision on efficiency vs human dignity |
| 18 | `general_water_rights.json` | General | `negotiate` | **Negotiate** (Hold) | True (2:1 majority) | **Yes** — binary forces a winner among rights that cannot be ranked |
| 19 | `engineering_dam_breach_risk.json` | Engineering | `commit_false` | **Commit(False, Science)** | False (2:1 majority) | No — both agree, but Trit-Core records conflict |

**Summary**: MedicalEthics and ValueJudgment scenarios demonstrate consistent binary failure. Binary either ignores individual patient context or overrides minority risk signals. Physical and Engineering scenarios show agreement when Science data is decisive, but Trit-Core still records the conflict path via `MetaInterrupt`.

---

## Quantitative Summary

| Metric | Count |
|--------|-------|
| Unique English scenarios | 19 |
| Chinese translation scenarios | 19 |
| Total JSON scenario files | 38 |
| Binary agrees with ternary (output value) | 8 / 19 (42%) |
| Binary overrides/smooths conflict | 11 / 19 (58%) |
| Scenarios where binary *cannot* express Hold | 19 / 19 (100%) |

---

## Key Findings

1. **Binary systems structurally cannot express "undetermined."** Every scenario that involves cross-frame signals or incommensurable values is forced into a True/False decision by the binary baseline.

2. **ValueJudgment domain is impossible in binary.** All ValueJudgment scenarios demonstrate that binary majority voting cannot represent "this decision should not be made by algorithm." Trit-Core's Hold state is the correct output.

3. **MedicalEthics requires Individual frame priority.** Binary treats all votes equally, but medical ethics demands that individual patient context can override statistical evidence. Trit-Core's domain-aware arbitration correctly implements this.

4. **Even when outcomes agree, Trit-Core provides auditability.** In Physical and Engineering scenarios where both systems reach the same conclusion, Trit-Core records the conflict path (`MetaInterrupt` log) and now exposes `SandboxDiagnostics` for per-stage telemetry.

5. **The Hold state is not a failure mode — it's the feature.** Trit-Core's Hold output is the correct answer when domains conflict.

---

## v0.3.0 Observability Additions

- `SandboxDiagnostics` records signal count, frame distribution, interrupt types, per-stage timing, and SafeFallback activation.
- `--trace` and `--diagnostic` CLI flags allow reproduction of every decision path.
- `SandboxError::report()` provides categorized errors with actionable help text.

---

## Limitations

- Scenario sample size is small and not statistically powered.
- Binary baseline uses simple majority; real RLHF systems use more sophisticated averaging (but still collapse to a single preference direction).
- No human subjects have validated the "authenticity" claim (planned for future work).
- All scenarios are synthetic; real-world deployment would require domain expert validation.

---

## Next Steps

- Expand to 30+ scenarios with real-world case studies.
- Human subject study: present ternary vs binary outputs to domain experts, measure perceived authenticity.
- Formalize the comparison methodology for preprint.
- External review of the claim: "Trit-Core preserves conflict better than binary RLHF proxies."
