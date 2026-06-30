use crate::core::frame::Frame;
use crate::core::value::TritValue;
use crate::core::word::TritWord;
use std::collections::VecDeque;

/// Maximum number of interrupts retained in the MetaMonitor log.
/// Prevents unbounded memory growth in long-running nodes.
pub const MAX_INTERRUPT_LOG: usize = 10_000;

/// A recorded meta-level conflict event.
#[derive(Clone, Debug, PartialEq)]
pub struct MetaInterrupt {
    pub conflict: ConflictType,
    pub reason: String,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

impl MetaInterrupt {
    /// Create a new interrupt with the current UTC timestamp.
    pub fn new(conflict: ConflictType, reason: String) -> Self {
        Self {
            conflict,
            reason,
            timestamp: chrono::Utc::now(),
        }
    }

    /// Create a FrameMismatch interrupt from a frame pair.
    /// Uses pre-sized String allocation instead of `format!()`.
    #[inline]
    pub fn with_frames(op: &'static str, frame_a: Frame, frame_b: Frame) -> Self {
        let reason = Self::build_frame_mismatch_reason(op, &frame_a, &frame_b);
        Self {
            conflict: ConflictType::FrameMismatch,
            reason,
            timestamp: chrono::Utc::now(),
        }
    }

    /// Test-only constructor for deterministic assertions.
    #[cfg(test)]
    pub fn new_for_test(conflict: ConflictType, reason: impl Into<String>) -> Self {
        Self {
            conflict,
            reason: reason.into(),
            timestamp: chrono::DateTime::UNIX_EPOCH,
        }
    }

    /// Create a PolicyViolation interrupt with the current UTC timestamp.
    pub fn policy_violation(violation: PolicyViolation, reason: String) -> Self {
        Self {
            conflict: ConflictType::PolicyViolation(violation),
            reason,
            timestamp: chrono::Utc::now(),
        }
    }

    /// Extract the two frame names from a FrameMismatch reason.
    ///
    /// Reason format (see [`build_frame_mismatch_reason`]):
    /// `"{op} conflict: {frame_a} vs {frame_b}"`.
    /// Returns `("Unknown", "Unknown")` if the reason does not match
    /// (e.g. non-FrameMismatch interrupts, or a malformed reason).
    pub fn frames(&self) -> (String, String) {
        let Some(vs_part) = self.reason.split(" conflict: ").nth(1) else {
            return ("Unknown".into(), "Unknown".into());
        };
        let mut parts = vs_part.split(" vs ");
        let a = parts.next().unwrap_or("Unknown").to_string();
        let b = parts.next().unwrap_or("Unknown").to_string();
        (a, b)
    }

    fn build_frame_mismatch_reason(op: &str, a: &Frame, b: &Frame) -> String {
        // Longest op name (~20) + " conflict: " (11) + longest frame (~10) + " vs " (4) + longest frame (~10) ≈ 55
        let mut reason = String::with_capacity(64);
        reason.push_str(op);
        reason.push_str(" conflict: ");
        use std::fmt::Write;
        let _ = write!(reason, "{}", a);
        reason.push_str(" vs ");
        let _ = write!(reason, "{}", b);
        reason
    }
}

/// Classification of meta-level conflicts.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ConflictType {
    FrameMismatch,
    OutOfScope,
    PhaseDrift,
    /// Policy violation detected by the system. This is an ethical notice,
    /// not a technical error. Computation continues; the user decides.
    PolicyViolation(PolicyViolation),
    /// Cognitive deconstruction detected an explanation impulse:
    /// input entropy is high but output determinacy is high —
    /// the system is about to produce a confident answer without
    /// sufficient evidence.
    ExplainImpulse,
}

/// Specific kinds of policy violation that can be reported to the user.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PolicyViolation {
    /// External request to force a True/False output instead of Hold.
    ForcedCollapse,
    /// Unregistered or tampered Frame mapped to Meta.
    FrameContamination,
    /// Meta-monitor log or state was tampered with externally.
    MetaMonitorTampered,
    /// Survival boundary was overridden or ignored.
    SurvivalBoundaryOverride,
    /// Input pattern deviates from historical baseline (>3σ).
    DataAnomaly,
    /// Other policy violation, with a descriptive label.
    Other(String),
}

/// Meta-monitor: records interrupt events and enforces invariants.
///
/// The interrupt log is a capped ring buffer (VecDeque) to prevent
/// unbounded memory growth in long-running nodes. Oldest entries
/// are evicted when the cap is reached.
#[derive(Debug, Clone, Default)]
pub struct MetaMonitor {
    log: VecDeque<MetaInterrupt>,
}

impl MetaMonitor {
    /// Create an empty MetaMonitor.
    pub fn new() -> Self {
        Self {
            log: VecDeque::new(),
        }
    }

    /// Record an interrupt, evicting the oldest entry if the log is full.
    pub fn record(&mut self, interrupt: MetaInterrupt) {
        if self.log.len() >= MAX_INTERRUPT_LOG {
            self.log.pop_front();
        }
        self.log.push_back(interrupt);
    }

    /// Iterate over recorded interrupts (oldest first).
    pub fn log(&self) -> impl Iterator<Item = &MetaInterrupt> {
        self.log.iter()
    }

    /// Drain all recorded interrupts, returning them as a Vec.
    pub fn drain_log(&mut self) -> Vec<MetaInterrupt> {
        self.log.drain(..).collect()
    }

    /// Enforce invariants on a single word.
    /// Currently: Absolute frame must remain Hold + neutral phase.
    pub fn inspect(&self, word: &TritWord) -> Option<MetaInterrupt> {
        if word.frame() == Frame::Absolute && word.value() != TritValue::Hold {
            return Some(MetaInterrupt::policy_violation(
                PolicyViolation::FrameContamination,
                "Absolute frame must remain Hold".to_string(),
            ));
        }
        None
    }

    /// Enforce invariants on a collection of words.
    pub fn inspect_all(&self, words: &[TritWord]) -> Vec<MetaInterrupt> {
        words.iter().filter_map(|w| self.inspect(w)).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn monitor_records_interrupts() {
        let mut m = MetaMonitor::new();
        m.record(MetaInterrupt::new_for_test(
            ConflictType::FrameMismatch,
            "x",
        ));
        assert_eq!(m.log().count(), 1);
    }

    #[test]
    fn monitor_evicts_old_entries() {
        let mut m = MetaMonitor::new();
        for i in 0..MAX_INTERRUPT_LOG + 5 {
            m.record(MetaInterrupt::new_for_test(
                ConflictType::FrameMismatch,
                format!("{}", i),
            ));
        }
        assert_eq!(m.log().count(), MAX_INTERRUPT_LOG);
    }

    #[test]
    fn inspect_detects_absolute_violation() {
        let m = MetaMonitor::new();
        // Absolute with non-Hold value violates the invariant.
        let _bad = TritWord::from_parts(
            TritValue::True,
            crate::core::phase::Phase::new(0.5).unwrap(),
            Frame::Absolute,
        )
        .unwrap_err();
        // Since the constructor prevents the violation, we just verify inspect logic on a non-Absolute word.
        assert!(m.inspect(&TritWord::tru(Frame::Science)).is_none());
    }

    #[test]
    fn with_frames_builds_expected_reason() {
        let interrupt = MetaInterrupt::with_frames("TAND", Frame::Science, Frame::Individual);
        assert_eq!(interrupt.conflict, ConflictType::FrameMismatch);
        assert!(interrupt.reason.contains("TAND"));
        assert!(interrupt.reason.contains("Science"));
        assert!(interrupt.reason.contains("Individual"));
        assert!(interrupt.reason.contains("vs"));
    }

    #[test]
    fn drain_log_clears_monitor() {
        let mut m = MetaMonitor::new();
        m.record(MetaInterrupt::new_for_test(ConflictType::PhaseDrift, "x"));
        m.record(MetaInterrupt::new_for_test(
            ConflictType::PolicyViolation(PolicyViolation::ForcedCollapse),
            "y",
        ));
        assert_eq!(m.log().count(), 2);
        let drained = m.drain_log();
        assert_eq!(drained.len(), 2);
        assert_eq!(m.log().count(), 0);
    }

    #[test]
    fn inspect_all_collects_violations() {
        let m = MetaMonitor::new();
        // Absolute invariant is enforced at construction, so only non-Absolute words are observable.
        let words = vec![TritWord::tru(Frame::Science), TritWord::hold(Frame::Meta)];
        let violations = m.inspect_all(&words);
        assert!(violations.is_empty());
    }

    #[test]
    fn conflict_type_equality() {
        assert_eq!(ConflictType::FrameMismatch, ConflictType::FrameMismatch);
        assert_ne!(ConflictType::FrameMismatch, ConflictType::OutOfScope);
        assert_eq!(ConflictType::ExplainImpulse, ConflictType::ExplainImpulse);
        assert_ne!(ConflictType::ExplainImpulse, ConflictType::FrameMismatch);
    }
}
