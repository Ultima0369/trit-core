//! TernaryDecision BC — maps signals to TritWords and executes ternary decisions.
//!
//! # Aggregate root
//! [`DecisionSession`] — a single decision evaluation session.
//!
//! # Port
//! [`DecisionPort`] trait — the single interface for ternary decision-making.

use crate::bc::BcError;
use truncore::core::{TernaryAlgebra, TritWord};
use truncore::meta::MetaInterrupt;

// ── Entities ──────────────────────────────────────────────────────────────

/// A record of a single decision evaluation.
#[derive(Debug, Clone)]
pub struct DecisionRecord {
    /// The input signals that were evaluated.
    pub input_signals: Vec<TritWord>,
    /// The final decision value.
    pub result: TritWord,
    /// Any interrupts raised during evaluation.
    pub interrupts: Vec<MetaInterrupt>,
    /// The domain this decision was made in.
    pub domain: String,
}

impl DecisionRecord {
    /// Whether this decision resulted in a Hold (conflict detected).
    pub fn is_hold(&self) -> bool {
        self.result.value() == truncore::core::TritValue::Hold
    }

    /// Whether any interrupts were raised.
    pub fn has_conflicts(&self) -> bool {
        !self.interrupts.is_empty()
    }

    /// Summary of all conflicts, or None if none.
    pub fn conflict_summary(&self) -> Option<String> {
        if self.interrupts.is_empty() {
            return None;
        }
        Some(
            self.interrupts
                .iter()
                .map(|i| format!("{:?}: {}", i.conflict, i.reason))
                .collect::<Vec<_>>()
                .join("; "),
        )
    }
}

/// A snapshot of the decision state for audit purposes.
#[derive(Debug, Clone)]
pub struct DecisionSnapshot {
    /// The input signals at the time of the snapshot.
    pub signals: Vec<TritWord>,
    /// The result at the time of the snapshot.
    pub result: TritWord,
    /// ISO 8601 timestamp.
    pub timestamp: String,
}

// ── Aggregate root ────────────────────────────────────────────────────────

/// A decision session — the aggregate root for ternary decision-making.
///
/// Tracks the sequence of decisions made within a single session.
#[derive(Debug, Clone)]
pub struct DecisionSession {
    /// Unique session identifier.
    pub session_id: String,
    /// Records of all decisions made in this session.
    pub records: Vec<DecisionRecord>,
}

impl DecisionSession {
    /// Create a new decision session.
    pub fn new(session_id: String) -> Self {
        Self {
            session_id,
            records: Vec::new(),
        }
    }

    /// Add a decision record to this session.
    pub fn record(&mut self, record: DecisionRecord) {
        self.records.push(record);
    }

    /// Number of decisions made in this session.
    pub fn decision_count(&self) -> usize {
        self.records.len()
    }

    /// Number of decisions that resulted in Hold.
    pub fn hold_count(&self) -> usize {
        self.records.iter().filter(|r| r.is_hold()).count()
    }

    /// Take a snapshot of the current state for audit.
    pub fn snapshot(&self) -> Option<DecisionSnapshot> {
        self.records.last().map(|r| DecisionSnapshot {
            signals: r.input_signals.clone(),
            result: r.result,
            timestamp: chrono::Utc::now().to_rfc3339(),
        })
    }
}

// ── Port trait ────────────────────────────────────────────────────────────

/// The single interface for ternary decision-making.
///
/// Takes signals and a domain, returns a decision record with interrupts.
pub trait DecisionPort {
    /// Evaluate a set of signals within a session and domain.
    ///
    /// All signals are combined via TAND (batch ternary AND).
    /// Cross-frame signals produce Hold + MetaInterrupt.
    fn evaluate(
        &self,
        session: &mut DecisionSession,
        signals: &[TritWord],
        domain: &str,
    ) -> Result<DecisionRecord, BcError>;
}

// ── M0 implementation ─────────────────────────────────────────────────────

/// M0 decision engine — uses Trit-Core's TernaryAlgebra directly.
pub struct TritDecisionEngine;

impl DecisionPort for TritDecisionEngine {
    fn evaluate(
        &self,
        session: &mut DecisionSession,
        signals: &[TritWord],
        domain: &str,
    ) -> Result<DecisionRecord, BcError> {
        if signals.is_empty() {
            return Err(BcError::InvalidInput {
                field: "signals".into(),
                reason: "at least one signal required".into(),
            });
        }

        // Batch TAND all signals
        let (result, interrupts) = TernaryAlgebra::t_and_n(signals);

        let record = DecisionRecord {
            input_signals: signals.to_vec(),
            result,
            interrupts,
            domain: domain.to_string(),
        };

        session.record(record.clone());
        Ok(record)
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use truncore::core::Frame;

    #[test]
    fn same_frame_signals_commit() {
        let engine = TritDecisionEngine;
        let mut session = DecisionSession::new("test".into());

        let signals = vec![TritWord::tru(Frame::Science), TritWord::tru(Frame::Science)];

        let record = engine.evaluate(&mut session, &signals, "Physical").unwrap();
        assert_eq!(record.result.value(), truncore::core::TritValue::True);
        assert!(!record.has_conflicts());
        assert_eq!(session.decision_count(), 1);
    }

    #[test]
    fn cross_frame_signals_produce_hold() {
        let engine = TritDecisionEngine;
        let mut session = DecisionSession::new("test".into());

        let signals = vec![
            TritWord::tru(Frame::Embodied),
            TritWord::fals(Frame::Individual),
        ];

        let record = engine
            .evaluate(&mut session, &signals, "Relational")
            .unwrap();
        assert!(record.is_hold());
        assert!(record.has_conflicts());
        assert_eq!(session.hold_count(), 1);
    }

    #[test]
    fn empty_signals_rejected() {
        let engine = TritDecisionEngine;
        let mut session = DecisionSession::new("test".into());
        let result = engine.evaluate(&mut session, &[], "General");
        assert!(result.is_err());
    }

    #[test]
    fn session_tracks_multiple_decisions() {
        let engine = TritDecisionEngine;
        let mut session = DecisionSession::new("test".into());

        engine
            .evaluate(
                &mut session,
                &[TritWord::tru(Frame::Science), TritWord::tru(Frame::Science)],
                "Physical",
            )
            .unwrap();

        engine
            .evaluate(
                &mut session,
                &[
                    TritWord::tru(Frame::Embodied),
                    TritWord::fals(Frame::Individual),
                ],
                "Relational",
            )
            .unwrap();

        assert_eq!(session.decision_count(), 2);
        assert_eq!(session.hold_count(), 1);
    }

    #[test]
    fn snapshot_captures_last_decision() {
        let engine = TritDecisionEngine;
        let mut session = DecisionSession::new("test".into());

        engine
            .evaluate(&mut session, &[TritWord::tru(Frame::Science)], "Physical")
            .unwrap();

        let snap = session.snapshot().unwrap();
        assert_eq!(snap.signals.len(), 1);
        assert!(!snap.timestamp.is_empty());
    }

    #[test]
    fn conflict_summary_lists_all_interrupts() {
        let engine = TritDecisionEngine;
        let mut session = DecisionSession::new("test".into());

        let record = engine
            .evaluate(
                &mut session,
                &[
                    TritWord::tru(Frame::Embodied),
                    TritWord::fals(Frame::Individual),
                ],
                "Relational",
            )
            .unwrap();

        let summary = record.conflict_summary().unwrap();
        assert!(summary.contains("FrameMismatch"));
    }
}
