# Aurora Contacts Pipeline Integration Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Wire `RelationshipAnnotation` BC contacts into the analysis and attention pipeline links, so loaded contacts participate in ternary decisions and their participation is recorded in the audit trail.

**Architecture:** Four-layer change: (1) Add `ContactInput`/`FrameAnnotationInput` deserialization types + `From<ContactInput> for ContactProfile` to `bc/relationship_annotation.rs`; (2) Add `ContactAuditRecord` + extend `AuditDecisionSnapshot` with `contact_participation` in `bc/audit_trail.rs`; (3) Add `contacts_to_tritwords()` + extend `run_analysis()` and `AnalysisReport` in `pipeline/analysis.rs`; (4) Extend `build_snapshot()` + `run_attention()`/`run_attention_in_memory()` in `pipeline/attention.rs`; (5) Extend `SqliteAuditLog::record()` serialization in `db/audit_log.rs`; (6) Wire `main.rs` to load `ContactInput` → convert → pipeline.

**Tech Stack:** Rust, Trit-Core (`TritWord`, `Frame`, `Phase`, `TritValue`), rusqlite, serde, chrono

## Global Constraints

- `#![forbid(unsafe_code)]` — both crates enforce this
- Do not modify any BC trait signatures (`AnnotationStore`, `AuditPort`, `DecisionPort`, `RenderPort`)
- `InMemoryAnnotationStore` and `SqliteAnnotationStore` remain unchanged
- All existing 114 tests must keep passing
- `cargo test ethics_` — 10 tests must pass
- `cargo clippy --workspace --all-targets --all-features -- -D warnings` must pass
- `cargo fmt --check` must pass
- `Phase::new(f64)` returns `Result<Phase, PhaseError>` — never unwrap without handling
- `Frame::from_str(&str)` via `FromStr` trait — handle the `Err` case
- `ContactProfile` fields are all `pub`: `id`, `name`, `relation_label`, `frames`, `history`, `deleted`, `notes`
- `FrameAnnotation` fields are all `pub`: `frame`, `annotation`, `phase`
- `RelationLabel::as_str()` returns `&str` — use this for serialization, not `Debug` formatting
- BC trait `_owned` methods exist for SQLite compatibility — use `query_owned()` and `get_contact_owned()`

---

### Task 1: Add ContactInput + FrameAnnotationInput types and From impl

**Files:**
- Modify: `aurora/src/bc/relationship_annotation.rs` — add types after `RelationLabel` impl block (after line 37), add `From` impl after `ContactProfile` impl block (after line 169)

**Interfaces:**
- Produces: `ContactInput` (Deserialize, Debug, Clone), `FrameAnnotationInput` (Deserialize, Debug, Clone), `impl From<ContactInput> for ContactProfile`

- [ ] **Step 1: Add `ContactInput` and `FrameAnnotationInput` structs**

Insert after line 37 (after `RelationLabel::as_str()` impl block, before `FrameAnnotation` struct):

```rust
// ── Deserialization types ───────────────────────────────────────────────────

/// JSON-deserializable contact input format.
///
/// This is the intermediate representation used when loading contacts
/// from a JSON data source. It is converted to [`ContactProfile`] via
/// the `From` trait, which handles `RelationLabel` parsing and
/// `FrameAnnotation` validation.
#[derive(Debug, Clone, serde::Deserialize)]
pub struct ContactInput {
    pub id: String,
    pub name: String,
    pub relation_label: String,
    #[serde(default)]
    pub annotations: Vec<FrameAnnotationInput>,
    #[serde(default)]
    pub notes: String,
}

/// JSON-deserializable frame annotation input.
#[derive(Debug, Clone, serde::Deserialize)]
pub struct FrameAnnotationInput {
    pub frame: String,
    #[serde(default)]
    pub annotation: String,
    pub phase: f64,
}
```

- [ ] **Step 2: Add `From<ContactInput> for ContactProfile` impl**

Insert after line 169 (after `ContactProfile::history_len()` method, before `fn chrono_now()`):

```rust
impl From<ContactInput> for ContactProfile {
    fn from(input: ContactInput) -> Self {
        let relation_label = match input.relation_label.as_str() {
            "colleague" => RelationLabel::Colleague,
            "friend" => RelationLabel::Friend,
            "family" => RelationLabel::Family,
            "partner" => RelationLabel::Partner,
            "self" => RelationLabel::Self_,
            other => RelationLabel::Other(other.to_string()),
        };
        let mut profile = ContactProfile::new(input.id, input.name, relation_label);
        profile.notes = input.notes;
        for ann in input.annotations {
            if let Ok(fa) = FrameAnnotation::new(ann.frame, ann.annotation, ann.phase) {
                profile.annotate_frame(fa);
            }
        }
        profile
    }
}
```

- [ ] **Step 3: Add `#[cfg(test)]` tests for ContactInput deserialization and From conversion**

Insert at the end of the existing `#[cfg(test)] mod tests` block (before the closing `}` of the module):

```rust
    // ── ContactInput deserialization ────────────────────────────────────

    #[test]
    fn contact_input_deserializes_minimal() {
        let json = r#"{"id":"c1","name":"Alice","relation_label":"friend"}"#;
        let input: ContactInput = serde_json::from_str(json).unwrap();
        assert_eq!(input.id, "c1");
        assert_eq!(input.name, "Alice");
        assert_eq!(input.relation_label, "friend");
        assert!(input.annotations.is_empty());
        assert!(input.notes.is_empty());
    }

    #[test]
    fn contact_input_deserializes_with_annotations() {
        let json = r#"{
            "id": "c1",
            "name": "Alice",
            "relation_label": "colleague",
            "annotations": [
                {"frame": "Embodied", "annotation": "高频", "phase": 0.8}
            ],
            "notes": "test note"
        }"#;
        let input: ContactInput = serde_json::from_str(json).unwrap();
        assert_eq!(input.annotations.len(), 1);
        assert_eq!(input.annotations[0].frame, "Embodied");
        assert_eq!(input.annotations[0].phase, 0.8);
        assert_eq!(input.notes, "test note");
    }

    #[test]
    fn contact_input_converts_to_profile() {
        let input = ContactInput {
            id: "c1".into(),
            name: "Alice".into(),
            relation_label: "friend".into(),
            annotations: vec![FrameAnnotationInput {
                frame: "Embodied".into(),
                annotation: "高频".into(),
                phase: 0.8,
            }],
            notes: "test".into(),
        };
        let profile = ContactProfile::from(input);
        assert_eq!(profile.name, "Alice");
        assert_eq!(profile.relation_label, RelationLabel::Friend);
        assert_eq!(profile.frame_count(), 1);
        assert_eq!(profile.frames[0].phase, 0.8);
        assert_eq!(profile.notes, "test");
    }

    #[test]
    fn contact_input_skips_invalid_phase_in_conversion() {
        let input = ContactInput {
            id: "c1".into(),
            name: "Alice".into(),
            relation_label: "friend".into(),
            annotations: vec![FrameAnnotationInput {
                frame: "Embodied".into(),
                annotation: "bad phase".into(),
                phase: 1.5, // invalid — FrameAnnotation::new will reject
            }],
            notes: String::new(),
        };
        let profile = ContactProfile::from(input);
        // Invalid phase annotation is silently skipped (FrameAnnotation::new returns Err)
        assert_eq!(profile.frame_count(), 0);
    }

    #[test]
    fn contact_input_unknown_relation_label_becomes_other() {
        let input = ContactInput {
            id: "c1".into(),
            name: "Alice".into(),
            relation_label: "mentor".into(),
            annotations: vec![],
            notes: String::new(),
        };
        let profile = ContactProfile::from(input);
        assert_eq!(
            profile.relation_label,
            RelationLabel::Other("mentor".into())
        );
    }
```

- [ ] **Step 4: Build and run tests**

Run: `cargo test --lib bc::relationship_annotation -- --test-threads=2`
Expected: All tests pass, including the 5 new `contact_input_*` tests

- [ ] **Step 5: Commit**

```bash
git add aurora/src/bc/relationship_annotation.rs
git commit -m "feat(bc): add ContactInput/FrameAnnotationInput deserialization types with From<ContactProfile>"
```

---

### Task 2: Add ContactAuditRecord + extend AuditDecisionSnapshot

**Files:**
- Modify: `aurora/src/bc/audit_trail.rs` — add `ContactAuditRecord` struct after `AuditDecisionSnapshot` (after line 52), add `contact_participation` field to `AuditDecisionSnapshot`

**Interfaces:**
- Consumes: none (standalone type addition)
- Produces: `ContactAuditRecord` (Debug, Clone), `AuditDecisionSnapshot.contact_participation: Option<Vec<ContactAuditRecord>>`

- [ ] **Step 1: Add `ContactAuditRecord` struct**

Insert after line 52 (after the closing `}` of `AuditDecisionSnapshot`):

```rust
/// A record of a single contact's participation in a decision.
#[derive(Debug, Clone)]
pub struct ContactAuditRecord {
    /// Contact's unique ID.
    pub contact_id: String,
    /// Contact's display name.
    pub contact_name: String,
    /// Relationship type as a string (e.g. "colleague", "friend").
    pub relation_label: String,
    /// The frame being annotated (e.g. "Embodied", "Individual").
    pub frame: String,
    /// The phase value [0.0, 1.0] from the annotation.
    pub phase: f64,
    /// The derived ternary value ("True" or "False").
    pub trit_value: String,
}
```

- [ ] **Step 2: Add `contact_participation` field to `AuditDecisionSnapshot`**

Replace the existing `AuditDecisionSnapshot` struct (lines 42-52):

```rust
/// A snapshot of a decision for audit purposes.
#[derive(Debug, Clone)]
pub struct AuditDecisionSnapshot {
    /// Number of input signals.
    pub signal_count: usize,
    /// Frame of each input signal (debug string).
    pub signal_frames: Vec<String>,
    /// The final decision value.
    pub result_value: String,
    /// The final decision frame.
    pub result_frame: String,
    /// Per-contact participation records (None if no contacts participated).
    pub contact_participation: Option<Vec<ContactAuditRecord>>,
}
```

- [ ] **Step 3: Update all existing `AuditDecisionSnapshot` construction sites**

The `contact_participation` field is new, so every existing construction of `AuditDecisionSnapshot` must add it. There are three sites:

**Site A — `pipeline/attention.rs:28-35` (`build_snapshot`):** Add `contact_participation: None`:

```rust
fn build_snapshot(signals: &[TritWord]) -> AuditDecisionSnapshot {
    AuditDecisionSnapshot {
        signal_count: signals.len(),
        signal_frames: signals.iter().map(|s| s.frame().to_string()).collect(),
        result_value: "pending".into(),
        result_frame: "Meta".into(),
        contact_participation: None,
    }
}
```

**Site B — `db/audit_log.rs:286-291` (test `entry_with_decision_snapshot_roundtrips`):** Add `contact_participation: None`:

```rust
        let snapshot = AuditDecisionSnapshot {
            signal_count: 2,
            signal_frames: vec!["Embodied".into(), "Individual".into()],
            result_value: "Hold".into(),
            result_frame: "Meta".into(),
            contact_participation: None,
        };
```

**Site C — `bc/audit_trail.rs:403-408` (test `entry_with_decision_snapshot`):** Add `contact_participation: None`:

```rust
        let snapshot = AuditDecisionSnapshot {
            signal_count: 2,
            signal_frames: vec!["Embodied".into(), "Individual".into()],
            result_value: "Hold".into(),
            result_frame: "Meta".into(),
            contact_participation: None,
        };
```

- [ ] **Step 4: Build and run tests**

Run: `cargo test --workspace -- --test-threads=2`
Expected: All 114+ tests pass (existing tests updated with new field, no behavioral change)

- [ ] **Step 5: Commit**

```bash
git add aurora/src/bc/audit_trail.rs aurora/src/pipeline/attention.rs aurora/src/db/audit_log.rs
git commit -m "feat(bc): add ContactAuditRecord + extend AuditDecisionSnapshot with contact_participation"
```

---

### Task 3: Add contacts_to_tritwords() + extend run_analysis()

**Files:**
- Modify: `aurora/src/pipeline/analysis.rs` — add `contacts_to_tritwords()` function, extend `run_analysis()` signature, extend `AnalysisReport` struct

**Interfaces:**
- Consumes: `ContactProfile` (from `bc/relationship_annotation`), `TritWord`, `Frame`, `Phase`, `TritValue` (from `truncore::core`)
- Produces: `pub fn contacts_to_tritwords(contacts: &[ContactProfile]) -> Vec<TritWord>`, `run_analysis(spec, frequency_threshold, user_feels_normal, contact_signals: &[TritWord]) -> Result<AnalysisReport, BcError>`, `AnalysisReport { spectrum, decision, contact_count: usize }`

- [ ] **Step 1: Add imports for `ContactProfile` and `Phase`/`TritValue`**

Replace the existing imports (lines 6-13) with:

```rust
use crate::bc::relationship_annotation::ContactProfile;
use crate::bc::signal_analysis::{FftWaveletEngine, FrequencySpectrum, TimeSeries, WaveletEngine};
use crate::bc::ternary_decision::{
    DecisionPort, DecisionRecord, DecisionSession, TritDecisionEngine,
};
use crate::bc::BcError;
use crate::wavelet::sine_wave;
use serde::Deserialize;
use std::str::FromStr;
use truncore::core::{Frame, Phase, TritValue, TritWord};
```

- [ ] **Step 2: Add `contact_count` field to `AnalysisReport`**

Replace the `AnalysisReport` struct (lines 25-31):

```rust
/// Structured report from the analysis pipeline link.
#[derive(Debug, Clone)]
pub struct AnalysisReport {
    /// The frequency spectrum detected by FFT analysis.
    pub spectrum: FrequencySpectrum,
    /// The ternary decision record.
    pub decision: DecisionRecord,
    /// Number of contact-derived TritWords that participated in the decision.
    pub contact_count: usize,
}
```

- [ ] **Step 3: Add `contacts_to_tritwords()` function**

Insert after the `AnalysisReport` struct (after line 31, before `frequency_to_embodied`):

```rust
/// Convert loaded contacts to TritWords for decision input.
///
/// Each contact's frame annotations are mapped to TritWords.
/// Annotations with invalid phase values or unknown frame names
/// are skipped with a warning to stderr.
pub fn contacts_to_tritwords(contacts: &[ContactProfile]) -> Vec<TritWord> {
    let mut words = Vec::new();
    for contact in contacts {
        for ann in &contact.frames {
            match Phase::new(ann.phase) {
                Ok(phase) => {
                    let value = if phase.inner() >= 0.5 {
                        TritValue::True
                    } else {
                        TritValue::False
                    };
                    let frame = match Frame::from_str(&ann.frame) {
                        Ok(f) => f,
                        Err(_) => {
                            eprintln!(
                                "warning: unknown frame '{}' for contact {}, skipping",
                                ann.frame, contact.name
                            );
                            continue;
                        }
                    };
                    words.push(TritWord::new(value, phase, frame));
                }
                Err(_) => {
                    eprintln!(
                        "warning: invalid phase {} for contact {}, skipping",
                        ann.phase, contact.name
                    );
                }
            }
        }
    }
    words
}
```

- [ ] **Step 4: Extend `run_analysis()` with `contact_signals` parameter**

Replace the `run_analysis` function signature and body (lines 67-95):

```rust
/// Run the analysis pipeline link.
///
/// 1. Generate a synthetic sine wave from the signal spec.
/// 2. Create a TimeSeries and analyze it via FFT → FrequencySpectrum.
/// 3. Map frequency → Embodied TritWord, user state → Individual TritWord.
/// 4. Merge contact-derived TritWords into the signal set.
/// 5. Evaluate the ternary decision via TritDecisionEngine.
///
/// Returns an [`AnalysisReport`] containing the spectrum, decision, and contact count.
pub fn run_analysis(
    spec: &SignalSpec,
    frequency_threshold: f64,
    user_feels_normal: bool,
    contact_signals: &[TritWord],
) -> Result<AnalysisReport, BcError> {
    // Step 1: Generate synthetic signal
    let signal = sine_wave(
        spec.freq,
        spec.sample_rate,
        spec.duration_secs,
        spec.noise_std,
    );

    // Step 2: Analyze via FFT
    let ts = TimeSeries::new(spec.sample_rate, signal)?;
    let engine = FftWaveletEngine;
    let spectrum = engine.analyze(&ts)?;

    // Step 3: Map to TritWords
    let embodied = frequency_to_embodied(spectrum.fundamental_hz, frequency_threshold);
    let individual = user_state_to_individual(user_feels_normal);

    // Step 4: Merge all signals (embodied + individual + contacts)
    let mut all_signals = vec![embodied, individual];
    all_signals.extend_from_slice(contact_signals);

    // Step 5: Evaluate ternary decision
    let decision_engine = TritDecisionEngine;
    let mut session = DecisionSession::new("analysis_session".into());
    let decision = decision_engine.evaluate(&mut session, &all_signals, "General")?;

    Ok(AnalysisReport {
        spectrum,
        decision,
        contact_count: contact_signals.len(),
    })
}
```

- [ ] **Step 5: Update all existing `run_analysis()` call sites**

There are three call sites that need the new `contact_signals` parameter:

**Site A — `pipeline/analysis.rs` tests (5 call sites in `#[cfg(test)]`):** Each call to `run_analysis(&spec, threshold, feels_normal)` becomes `run_analysis(&spec, threshold, feels_normal, &[])`.

Update lines 152, 175, 188, 197, 204 — replace each `run_analysis(&spec, ...)` call:

For `run_analysis_detects_2_5hz` (line 152):
```rust
        let report = run_analysis(&spec, 1.0, true, &[]).unwrap();
```

For `run_analysis_cross_frame_produces_hold` (line 175):
```rust
        let report = run_analysis(&spec, 1.0, true, &[]).unwrap();
```

For `run_analysis_high_freq_above_threshold` (line 188):
```rust
        let report = run_analysis(&spec, 5.0, true, &[]).unwrap();
```

For `run_analysis_low_freq_below_threshold` (line 197):
```rust
        let report = run_analysis(&spec, 3.0, false, &[]).unwrap();
```

**Site B — `tests/decision_conflict.rs` (2 call sites):** Each call to `run_analysis(&spec, threshold, feels_normal)` becomes `run_analysis(&spec, threshold, feels_normal, &[])`.

Line 28:
```rust
    let report = run_analysis(&spec, 2.0, true, &[]).unwrap();
```

Line 51:
```rust
    let report = run_analysis(&spec, 2.0, false, &[]).unwrap();
```

**Site C — `main.rs` line 38-40:** Will be updated in Task 6.

- [ ] **Step 6: Add `#[cfg(test)]` tests for `contacts_to_tritwords` and contacts-in-analysis**

Insert at the end of the existing `#[cfg(test)] mod tests` block (before the closing `}`):

```rust
    // ── contacts_to_tritwords ───────────────────────────────────────────

    #[test]
    fn contacts_to_tritwords_empty() {
        let contacts: Vec<ContactProfile> = vec![];
        let words = contacts_to_tritwords(&contacts);
        assert!(words.is_empty());
    }

    #[test]
    fn contacts_to_tritwords_maps_frame_and_phase() {
        let mut profile = ContactProfile::new(
            "c1".into(),
            "Alice".into(),
            crate::bc::relationship_annotation::RelationLabel::Friend,
        );
        profile.annotate_frame(
            crate::bc::relationship_annotation::FrameAnnotation::new(
                "Embodied".into(),
                "高频".into(),
                0.8,
            )
            .unwrap(),
        );

        let contacts = vec![profile];
        let words = contacts_to_tritwords(&contacts);
        assert_eq!(words.len(), 1);
        assert_eq!(words[0].frame(), Frame::Embodied);
        assert_eq!(words[0].value(), TritValue::True);
    }

    #[test]
    fn contacts_to_tritwords_low_phase_is_false() {
        let mut profile = ContactProfile::new(
            "c1".into(),
            "Bob".into(),
            crate::bc::relationship_annotation::RelationLabel::Colleague,
        );
        profile.annotate_frame(
            crate::bc::relationship_annotation::FrameAnnotation::new(
                "Individual".into(),
                "低频".into(),
                0.3,
            )
            .unwrap(),
        );

        let contacts = vec![profile];
        let words = contacts_to_tritwords(&contacts);
        assert_eq!(words.len(), 1);
        assert_eq!(words[0].value(), TritValue::False);
    }

    #[test]
    fn run_analysis_with_contacts_includes_contact_count() {
        let mut profile = ContactProfile::new(
            "c1".into(),
            "Alice".into(),
            crate::bc::relationship_annotation::RelationLabel::Friend,
        );
        profile.annotate_frame(
            crate::bc::relationship_annotation::FrameAnnotation::new(
                "Science".into(),
                "test".into(),
                0.7,
            )
            .unwrap(),
        );

        let contact_signals = contacts_to_tritwords(&[profile]);
        assert_eq!(contact_signals.len(), 1);

        let spec = SignalSpec {
            freq: 2.5,
            sample_rate: 100.0,
            duration_secs: 1.0,
            noise_std: 0.0,
        };
        let report = run_analysis(&spec, 1.0, true, &contact_signals).unwrap();
        assert_eq!(report.contact_count, 1);
    }

    #[test]
    fn run_analysis_without_contacts_has_zero_contact_count() {
        let spec = SignalSpec {
            freq: 2.5,
            sample_rate: 100.0,
            duration_secs: 1.0,
            noise_std: 0.0,
        };
        let report = run_analysis(&spec, 1.0, true, &[]).unwrap();
        assert_eq!(report.contact_count, 0);
    }
```

- [ ] **Step 7: Build and run tests**

Run: `cargo test --workspace -- --test-threads=2`
Expected: All tests pass, including the 5 new `contacts_to_tritwords_*` and `run_analysis_with_*` tests

- [ ] **Step 8: Commit**

```bash
git add aurora/src/pipeline/analysis.rs aurora/tests/decision_conflict.rs
git commit -m "feat(pipeline): add contacts_to_tritwords() + extend run_analysis with contact_signals"
```

---

### Task 4: Extend build_snapshot() + run_attention() with contacts

**Files:**
- Modify: `aurora/src/pipeline/attention.rs` — extend `build_snapshot()`, `run_attention()`, `run_attention_in_memory()`, and tests

**Interfaces:**
- Consumes: `ContactProfile` (from `bc/relationship_annotation`), `ContactAuditRecord` (from `bc/audit_trail`)
- Produces: `build_snapshot(signals, contacts) -> AuditDecisionSnapshot`, `run_attention(signals, db, contacts) -> Result<AttentionOutcome, BcError>`, `run_attention_in_memory(signals, contacts) -> Result<AttentionOutcome, BcError>`

- [ ] **Step 1: Add imports for `ContactProfile` and `ContactAuditRecord`**

Replace the existing imports (lines 6-12):

```rust
use crate::bc::attention_guidance::{AttentionManager, AttentionPort, AttentionSession};
use crate::bc::audit_trail::{
    AuditDecisionSnapshot, AuditEntry, AuditEventType, AuditPort, ContactAuditRecord,
};
use crate::bc::relationship_annotation::ContactProfile;
use crate::bc::BcError;
use crate::db::audit_log::SqliteAuditLog;
use crate::db::Database;
use truncore::adapters::AttentionCmd;
use truncore::core::TritWord;
```

- [ ] **Step 2: Extend `build_snapshot()` with `contacts` parameter**

Replace the existing `build_snapshot` function (lines 28-35):

```rust
/// Build an [`AuditDecisionSnapshot`] from signals and contacts.
fn build_snapshot(signals: &[TritWord], contacts: &[ContactProfile]) -> AuditDecisionSnapshot {
    let contact_participation = if contacts.is_empty() {
        None
    } else {
        Some(
            contacts
                .iter()
                .flat_map(|c| {
                    c.frames.iter().map(|ann| {
                        let value = if ann.phase >= 0.5 { "True" } else { "False" };
                        ContactAuditRecord {
                            contact_id: c.id.clone(),
                            contact_name: c.name.clone(),
                            relation_label: c.relation_label.as_str().to_string(),
                            frame: ann.frame.clone(),
                            phase: ann.phase,
                            trit_value: value.to_string(),
                        }
                    })
                })
                .collect(),
        )
    };

    AuditDecisionSnapshot {
        signal_count: signals.len(),
        signal_frames: signals.iter().map(|s| s.frame().to_string()).collect(),
        result_value: "pending".into(),
        result_frame: "Meta".into(),
        contact_participation,
    }
}
```

- [ ] **Step 3: Extend `run_attention()` with `contacts` parameter**

Replace the `run_attention` function signature and body (lines 43-67):

```rust
/// Run the attention pipeline link with SQLite persistence.
///
/// 1. Create an AttentionManager and run one scheduling cycle.
/// 2. Build an audit snapshot from the signals and contacts.
/// 3. Persist the audit entry to SQLite via SqliteAuditLog.
/// 4. Return the attention outcome.
pub fn run_attention(
    signals: &[TritWord],
    db: Database,
    contacts: &[ContactProfile],
) -> Result<AttentionOutcome, BcError> {
    let mut attention = AttentionManager::new("attention_session");
    let cmd = attention.run_cycle(signals);

    // Build and persist audit entry
    let snapshot = build_snapshot(signals, contacts);
    let entry = AuditEntry::new(
        AuditEventType::Decision,
        attention.session().session_id().to_string(),
        "attention cycle".into(),
    )
    .with_decision_snapshot(snapshot);

    let mut audit = SqliteAuditLog::new(db);
    audit.record(entry)?;

    let session = attention.session().clone();

    Ok(AttentionOutcome {
        cmd,
        asi: attention.asi(),
        reminder_count: attention.session().reminder_count(),
        session,
    })
}
```

- [ ] **Step 4: Extend `run_attention_in_memory()` with `contacts` parameter**

Replace the `run_attention_in_memory` function signature (lines 72-84):

```rust
/// Run the attention pipeline link without SQLite persistence (in-memory only).
///
/// Useful for testing or when no database is available.
pub fn run_attention_in_memory(
    signals: &[TritWord],
    contacts: &[ContactProfile],
) -> Result<AttentionOutcome, BcError> {
    let mut attention = AttentionManager::new("attention_session");
    let cmd = attention.run_cycle(signals);

    let session = attention.session().clone();

    Ok(AttentionOutcome {
        cmd,
        asi: attention.asi(),
        reminder_count: attention.session().reminder_count(),
        session,
    })
}
```

- [ ] **Step 5: Update all existing call sites in `#[cfg(test)]`**

Update the four test functions in `pipeline/attention.rs`:

**`run_attention_in_memory_does_not_panic` (line 99):**
```rust
        let outcome = run_attention_in_memory(&signals, &[]).unwrap();
```

**`run_attention_in_memory_tracks_reminders` (line 113):**
```rust
        let outcome = run_attention_in_memory(&signals, &[]).unwrap();
```

**`run_attention_with_sqlite_persists_audit_entry` (line 134):**
```rust
        let outcome = run_attention(&signals, db, &[]).unwrap();
```

**`attention_outcome_contains_session_data` (line 143):**
```rust
        let outcome = run_attention_in_memory(&signals, &[]).unwrap();
```

- [ ] **Step 6: Add `#[cfg(test)]` tests for contacts in snapshot**

Insert at the end of the existing `#[cfg(test)] mod tests` block (before the closing `}`):

```rust
    #[test]
    fn build_snapshot_with_contacts_includes_participation() {
        use crate::bc::relationship_annotation::{FrameAnnotation, RelationLabel};

        let mut profile = ContactProfile::new("c1".into(), "Alice".into(), RelationLabel::Friend);
        profile.annotate_frame(
            FrameAnnotation::new("Embodied".into(), "高频".into(), 0.8).unwrap(),
        );

        let signals = vec![TritWord::tru(Frame::Embodied)];
        let snapshot = build_snapshot(&signals, &[profile]);

        assert!(snapshot.contact_participation.is_some());
        let records = snapshot.contact_participation.unwrap();
        assert_eq!(records.len(), 1);
        assert_eq!(records[0].contact_id, "c1");
        assert_eq!(records[0].contact_name, "Alice");
        assert_eq!(records[0].relation_label, "friend");
        assert_eq!(records[0].frame, "Embodied");
        assert_eq!(records[0].phase, 0.8);
        assert_eq!(records[0].trit_value, "True");
    }

    #[test]
    fn build_snapshot_without_contacts_has_none_participation() {
        let signals = vec![TritWord::tru(Frame::Science)];
        let snapshot = build_snapshot(&signals, &[]);
        assert!(snapshot.contact_participation.is_none());
    }

    #[test]
    fn build_snapshot_low_phase_is_false_trit_value() {
        use crate::bc::relationship_annotation::{FrameAnnotation, RelationLabel};

        let mut profile = ContactProfile::new("c2".into(), "Bob".into(), RelationLabel::Colleague);
        profile.annotate_frame(
            FrameAnnotation::new("Individual".into(), "低频".into(), 0.3).unwrap(),
        );

        let signals = vec![];
        let snapshot = build_snapshot(&signals, &[profile]);

        let records = snapshot.contact_participation.unwrap();
        assert_eq!(records[0].trit_value, "False");
    }
```

- [ ] **Step 7: Build and run tests**

Run: `cargo test --workspace -- --test-threads=2`
Expected: All tests pass, including the 3 new `build_snapshot_*` tests

- [ ] **Step 8: Commit**

```bash
git add aurora/src/pipeline/attention.rs
git commit -m "feat(pipeline): extend build_snapshot + run_attention with contacts parameter"
```

---

### Task 5: Extend SqliteAuditLog serialization for contact_participation

**Files:**
- Modify: `aurora/src/db/audit_log.rs` — extend `record()` snapshot_json serialization, extend `query_owned()` deserialization, add test

**Interfaces:**
- Consumes: `AuditDecisionSnapshot.contact_participation` (from Task 2), `ContactAuditRecord` (from Task 2)
- Produces: Round-trippable `contact_participation` in `snapshot_json` column

- [ ] **Step 1: Extend `record()` snapshot_json serialization**

Replace the `snapshot_json` construction in `record()` (lines 34-42):

```rust
        let snapshot_json = entry.decision_snapshot.as_ref().map(|s| {
            let mut json = serde_json::json!({
                "signal_count": s.signal_count,
                "signal_frames": s.signal_frames,
                "result_value": s.result_value,
                "result_frame": s.result_frame,
            });
            if let Some(ref cp) = s.contact_participation {
                let records: Vec<serde_json::Value> = cp
                    .iter()
                    .map(|r| {
                        serde_json::json!({
                            "contact_id": r.contact_id,
                            "contact_name": r.contact_name,
                            "relation_label": r.relation_label,
                            "frame": r.frame,
                            "phase": r.phase,
                            "trit_value": r.trit_value,
                        })
                    })
                    .collect();
                json["contact_participation"] = serde_json::Value::Array(records);
            }
            json.to_string()
        });
```

- [ ] **Step 2: Extend `query_owned()` deserialization of `contact_participation`**

Replace the `decision_snapshot` deserialization in `query_owned()` (lines 127-140):

```rust
                let decision_snapshot = snapshot_json.and_then(|json_str| {
                    let parsed: serde_json::Value = serde_json::from_str(&json_str).ok()?;
                    let contact_participation =
                        parsed.get("contact_participation").and_then(|cp| {
                            cp.as_array().map(|arr| {
                                arr.iter()
                                    .filter_map(|v| {
                                        Some(ContactAuditRecord {
                                            contact_id: v.get("contact_id")?.as_str()?.to_string(),
                                            contact_name: v
                                                .get("contact_name")?
                                                .as_str()?
                                                .to_string(),
                                            relation_label: v
                                                .get("relation_label")?
                                                .as_str()?
                                                .to_string(),
                                            frame: v.get("frame")?.as_str()?.to_string(),
                                            phase: v.get("phase")?.as_f64()?,
                                            trit_value: v
                                                .get("trit_value")?
                                                .as_str()?
                                                .to_string(),
                                        })
                                    })
                                    .collect()
                            })
                        });
                    Some(AuditDecisionSnapshot {
                        signal_count: parsed.get("signal_count")?.as_u64()? as usize,
                        signal_frames: parsed
                            .get("signal_frames")?
                            .as_array()?
                            .iter()
                            .filter_map(|v| v.as_str().map(String::from))
                            .collect(),
                        result_value: parsed.get("result_value")?.as_str()?.to_string(),
                        result_frame: parsed.get("result_frame")?.as_str()?.to_string(),
                        contact_participation,
                    })
                });
```

- [ ] **Step 3: Add import for `ContactAuditRecord`**

Replace the existing import block (lines 5-9):

```rust
use crate::bc::audit_trail::{
    AuditDecisionSnapshot, AuditEntry, AuditEventType, AuditFilter, AuditPort, ContactAuditRecord,
    OverrideRecord,
};
```

- [ ] **Step 4: Add test for contact_participation round-trip**

Insert at the end of the existing `#[cfg(test)] mod tests` block (before the closing `}`):

```rust
    #[test]
    fn entry_with_contact_participation_roundtrips() {
        let mut log = SqliteAuditLog::new_in_memory().unwrap();

        let snapshot = AuditDecisionSnapshot {
            signal_count: 3,
            signal_frames: vec![
                "Embodied".into(),
                "Individual".into(),
                "Science".into(),
            ],
            result_value: "Hold".into(),
            result_frame: "Meta".into(),
            contact_participation: Some(vec![ContactAuditRecord {
                contact_id: "c1".into(),
                contact_name: "Alice".into(),
                relation_label: "friend".into(),
                frame: "Embodied".into(),
                phase: 0.8,
                trit_value: "True".into(),
            }]),
        };

        let entry = AuditEntry::new(AuditEventType::Decision, "s1".into(), "with contacts".into())
            .with_decision_snapshot(snapshot);

        log.record(entry).unwrap();

        let entries = log.query_owned(&AuditFilter::new()).unwrap();
        assert_eq!(entries.len(), 1);
        let snap = entries[0].decision_snapshot.as_ref().unwrap();
        assert!(snap.contact_participation.is_some());
        let records = snap.contact_participation.as_ref().unwrap();
        assert_eq!(records.len(), 1);
        assert_eq!(records[0].contact_id, "c1");
        assert_eq!(records[0].contact_name, "Alice");
        assert_eq!(records[0].phase, 0.8);
        assert_eq!(records[0].trit_value, "True");
    }
```

- [ ] **Step 5: Build and run tests**

Run: `cargo test --lib db::audit_log -- --test-threads=2`
Expected: All tests pass, including `entry_with_contact_participation_roundtrips`

- [ ] **Step 6: Commit**

```bash
git add aurora/src/db/audit_log.rs
git commit -m "feat(db): extend SqliteAuditLog serialization for contact_participation round-trip"
```

---

### Task 6: Wire main.rs — load contacts, convert, pipeline

**Files:**
- Modify: `aurora/src/main.rs` — load `ContactInput` → convert to `ContactProfile` → `contacts_to_tritwords()` → `run_analysis(+contact_signals)` → `run_attention(+contacts)`

**Interfaces:**
- Consumes: `ContactInput` (from Task 1), `ContactProfile` (from Task 1), `contacts_to_tritwords()` (from Task 3), `run_analysis()` (from Task 3), `run_attention()` (from Task 4)
- Produces: Wired end-to-end pipeline with contacts

- [ ] **Step 1: Add imports for `ContactInput` and `ContactProfile`**

Replace the existing imports (lines 1-8):

```rust
use anyhow::{Context, Result};
use aurora::bc::presentation::{AuroraRenderer, ConflictCard, RenderPort, ViewState};
use aurora::bc::relationship_annotation::{ContactInput, ContactProfile};
use aurora::cli::Args;
use aurora::db::Database;
use aurora::pipeline::{analysis, attention};
use clap::Parser;
use std::fs;
use std::path::Path;
```

- [ ] **Step 2: Replace the contacts loading block**

Replace lines 22-29 (the current `if let Some(ref path) = args.data_source` block that loads contacts then discards them):

```rust
    // Load contacts from data source
    let contacts: Vec<ContactProfile> = if let Some(ref path) = args.data_source {
        let manager = aurora::ingest::IngestManager::with_json_fallback(path)?;
        eprintln!(
            "Loaded {} contacts from {}",
            manager.contact_count(),
            manager.source_name()
        );
        let inputs: Vec<ContactInput> = manager.load()?;
        inputs.into_iter().map(ContactProfile::from).collect()
    } else {
        Vec::new()
    };

    // Convert contacts to TritWords for decision input
    let contact_signals = analysis::contacts_to_tritwords(&contacts);
```

- [ ] **Step 3: Update `run_analysis` call to pass `contact_signals`**

Replace lines 38-40:

```rust
    // ── Link 1: Analysis ────────────────────────────────────────────
    let analysis_report =
        analysis::run_analysis(&spec, args.frequency_threshold, args.user_feels_normal, &contact_signals)
            .map_err(|e| anyhow::anyhow!("analysis link failed: {e}"))?;
```

- [ ] **Step 4: Update `run_attention` call to pass `contacts`**

Replace lines 43-44:

```rust
    // ── Link 2: Attention ───────────────────────────────────────────
    let attention_outcome = attention::run_attention(&analysis_report.decision.input_signals, db, &contacts)
        .map_err(|e| anyhow::anyhow!("attention link failed: {e}"))?;
```

- [ ] **Step 5: Build and run tests**

Run: `cargo build --workspace`
Expected: Build succeeds with no errors

Run: `cargo test --workspace -- --test-threads=2`
Expected: All tests pass

- [ ] **Step 6: Commit**

```bash
git add aurora/src/main.rs
git commit -m "feat(main): wire contacts through analysis and attention pipeline links"
```

---

### Task 7: Integration test — contacts end-to-end via CLI

**Files:**
- Modify: `aurora/tests/cli_end_to_end.rs` — add `contacts_end_to_end` test

**Interfaces:**
- Consumes: `ContactInput` (Task 1), `contacts_to_tritwords` (Task 3), `run_analysis` (Task 3), `run_attention` (Task 4), `main.rs` wiring (Task 6)
- Produces: Integration test verifying contacts flow through the full pipeline

- [ ] **Step 1: Read the current `cli_end_to_end.rs` to understand the test pattern**

Run: `cargo test cli_end_to_end -- --test-threads=2`
Expected: Existing tests pass

- [ ] **Step 2: Add the `contacts_end_to_end` integration test**

Insert at the end of `aurora/tests/cli_end_to_end.rs`:

```rust
#[test]
fn contacts_end_to_end_via_cli() {
    use std::io::Write;

    let dir = std::env::temp_dir().join("aurora_test_contacts_e2e");
    std::fs::create_dir_all(&dir).unwrap();

    // Write input signal spec
    let input_path = dir.join("input.json");
    let input_json = r#"{"freq":2.5,"sample_rate":100.0,"duration_secs":1.0,"noise_std":0.0}"#;
    std::fs::write(&input_path, input_json).unwrap();

    // Write contacts JSON
    let contacts_path = dir.join("contacts.json");
    let contacts_json = r#"[
        {"id":"c1","name":"Alice","relation_label":"friend","annotations":[{"frame":"Embodied","annotation":"高频","phase":0.8}]},
        {"id":"c2","name":"Bob","relation_label":"colleague","annotations":[{"frame":"Individual","annotation":"低频","phase":0.3}]}
    ]"#;
    std::fs::write(&contacts_path, contacts_json).unwrap();

    let output_path = dir.join("report.html");

    // Run aurora CLI
    let output = std::process::Command::new(
        std::env::current_dir()
            .unwrap()
            .join("target")
            .join("debug")
            .join("aurora"),
    )
    .arg("--input")
    .arg(&input_path)
    .arg("--data-source")
    .arg(&contacts_path)
    .arg("--output")
    .arg(&output_path)
    .arg("--user-feels-normal")
    .output()
    .expect("failed to run aurora");

    assert!(
        output.status.success(),
        "aurora exited with: {}\nstderr: {}",
        output.status,
        String::from_utf8_lossy(&output.stderr)
    );

    // Read the HTML report
    let html = std::fs::read_to_string(&output_path).unwrap();

    // Verify contacts are referenced in the output
    assert!(
        html.contains("contact_count")
            || html.contains("contact")
            || html.contains("Alice")
            || html.contains("Bob"),
        "HTML report should reference contacts"
    );

    // Verify the report contains the core decision info
    assert!(html.contains("2.5") || html.contains("Hold"));

    // Cleanup
    std::fs::remove_dir_all(&dir).ok();
}
```

- [ ] **Step 3: Build aurora binary and run the integration test**

Run: `cargo build --bin aurora`
Expected: Build succeeds

Run: `cargo test contacts_end_to_end_via_cli -- --test-threads=2`
Expected: Test passes — aurora runs with `--data-source`, produces HTML with contact info

- [ ] **Step 4: Commit**

```bash
git add aurora/tests/cli_end_to_end.rs
git commit -m "test: add contacts end-to-end integration test via CLI"
```

---

### Task 8: Final verification — full suite + clippy + fmt + ethics

**Files:**
- No code changes — verification only

**Interfaces:**
- Consumes: All prior tasks
- Produces: Clean verification report

- [ ] **Step 1: Run the full test suite**

Run: `cargo test --workspace --all-features -- --test-threads=2`
Expected: All tests pass (existing 114 + new tests from Tasks 1-7)

- [ ] **Step 2: Run ethics gate tests**

Run: `cargo test ethics_`
Expected: All 10 ethics tests pass

- [ ] **Step 3: Run clippy**

Run: `cargo clippy --workspace --all-targets --all-features -- -D warnings`
Expected: No warnings

- [ ] **Step 4: Run fmt check**

Run: `cargo fmt --check`
Expected: No formatting issues

- [ ] **Step 5: Run the CLI end-to-end smoke test**

Run: `cargo run --bin aurora -- --input synthetic_2hz.json --data-source contacts.json --output report.html`
Expected: Runs successfully, produces `report.html`

- [ ] **Step 6: Commit (if any fmt/clippy fixes were needed)**

```bash
git add -u
git commit -m "chore: final verification — fmt, clippy, ethics gate"
```
