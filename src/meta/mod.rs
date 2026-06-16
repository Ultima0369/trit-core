use crate::frame::Frame;
use crate::trit::{TritValue, TritWord};
use tracing::{debug, info, warn};

/// Domain rules for conflict resolution.
/// Each domain defines which frame has priority and whether
/// forced resolution (hard collapse) is allowed.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Domain {
    Physical,      // Hard science constraints: Science priority, forced collapse
    Engineering,   // Applied constraints: Science priority, forced collapse
    MedicalEthics, // Soft constraints: Individual priority, no forced collapse
    ValueJudgment, // Incommensurable: no priority, must remain Hold
    General,       // Default: attempt negotiation
}

/// Policy engine that decides how to resolve conflicts.
#[derive(Clone)]
pub struct ResolutionPolicy {
    pub domain: Domain,
}

impl ResolutionPolicy {
    pub fn new(domain: Domain) -> Self {
        info!(?domain, "ResolutionPolicy created");
        Self { domain }
    }

    /// Given conflicting inputs, return the arbitration result.
    #[tracing::instrument(skip_all, fields(domain = ?self.domain))]
    pub fn arbitrate(&self, inputs: &[TritWord]) -> ArbitrationResult {
        debug!(input_count = inputs.len(), "arbitration started");
        let result = match self.domain {
            Domain::Physical | Domain::Engineering => {
                if let Some(t) = inputs.iter().find(|t| t.frame == Frame::Science) {
                    ArbitrationResult::Commit(t.clone())
                } else {
                    ArbitrationResult::ForceCollapse
                }
            }
            Domain::MedicalEthics => {
                if let Some(t) = inputs.iter().find(|t| t.frame == Frame::Individual) {
                    ArbitrationResult::Preserve(t.clone())
                } else {
                    ArbitrationResult::Negotiate
                }
            }
            Domain::ValueJudgment => ArbitrationResult::Hold,
            Domain::General => {
                let first = &inputs[0];
                if inputs.iter().all(|t| t.frame == first.frame) {
                    ArbitrationResult::Commit(first.clone())
                } else {
                    ArbitrationResult::Negotiate
                }
            }
        };
        info!(?result, "arbitration completed");
        result
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum ArbitrationResult {
    Commit(TritWord),
    Preserve(TritWord),
    ForceCollapse,
    Hold,
    Negotiate,
}

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
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ConflictType {
    FrameMismatch,
    OutOfScope,
    PhaseDrift,
    PolicyViolation,
}

pub struct MetaMonitor {
    #[allow(dead_code)]
    policy: ResolutionPolicy,
    log: Vec<MetaInterrupt>,
}

impl MetaMonitor {
    pub fn new(policy: ResolutionPolicy) -> Self {
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
