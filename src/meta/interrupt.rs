use crate::frame::Frame;
use crate::trit::{TritValue, TritWord};

#[derive(Clone, Debug, PartialEq)]
pub struct MetaInterrupt {
    pub conflict: ConflictType,
    pub reason: String,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

impl MetaInterrupt {
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

    fn build_frame_mismatch_reason(op: &str, a: &Frame, b: &Frame) -> String {
        // Maximum: "TAND conflict: Consensus vs Individual" ≈ 40 bytes
        let mut reason = String::with_capacity(48);
        reason.push_str(op);
        reason.push_str(" conflict: ");
        use std::fmt::Write;
        let _ = write!(reason, "{}", a);
        reason.push_str(" vs ");
        let _ = write!(reason, "{}", b);
        reason
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ConflictType {
    FrameMismatch,
    OutOfScope,
    PhaseDrift,
    PolicyViolation,
}

/// Meta-monitor: records interrupt events and enforces invariants.
#[derive(Debug, Clone)]
pub struct MetaMonitor {
    #[allow(dead_code)]
    policy: crate::meta::ResolutionPolicy,
    log: Vec<MetaInterrupt>,
}

impl MetaMonitor {
    pub fn new(policy: crate::meta::ResolutionPolicy) -> Self {
        Self {
            policy,
            log: vec![],
        }
    }

    pub fn record(&mut self, interrupt: MetaInterrupt) {
        self.log.push(interrupt);
    }

    pub fn log(&self) -> &[MetaInterrupt] {
        &self.log
    }

    /// Enforce invariants. Currently: Absolute frame must remain Hold.
    pub fn inspect(&self, word: &TritWord) -> Option<MetaInterrupt> {
        if word.frame == Frame::Absolute && word.value != TritValue::Hold {
            return Some(MetaInterrupt::new(
                ConflictType::PolicyViolation,
                "Absolute frame must remain Hold".to_string(),
            ));
        }
        None
    }
}
