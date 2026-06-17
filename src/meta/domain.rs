use crate::frame::Frame;
use crate::meta::frame_mask::FrameMask;
use crate::trit::TritWord;
use tracing::{debug, info};

/// Domain rules for conflict resolution.
/// Each domain defines which frame has priority and whether
/// forced resolution (hard collapse) is allowed.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Domain {
    Physical,       // Hard science constraints: Science priority, forced collapse
    Engineering,    // Applied constraints: Science priority, forced collapse
    MedicalEthics,  // Soft constraints: Individual priority, no forced collapse
    ValueJudgment,  // Incommensurable: no priority, must remain Hold
    General,        // Default: attempt negotiation
    Custom(String), // Externally loaded domain rules
}

/// Policy engine that decides how to resolve conflicts.
#[derive(Debug, Clone)]
pub struct ResolutionPolicy {
    pub domain: Domain,
}

impl ResolutionPolicy {
    pub fn new(domain: Domain) -> Self {
        info!(?domain, "ResolutionPolicy created");
        Self { domain }
    }

    /// Given conflicting inputs, return the arbitration result.
    /// Uses FrameMask for O(1) frame presence checks.
    #[tracing::instrument(skip_all, fields(domain = ?self.domain))]
    pub fn arbitrate(&self, inputs: &[TritWord]) -> ArbitrationResult {
        debug!(input_count = inputs.len(), "arbitration started");
        let mask = FrameMask::from_inputs(inputs);
        let result = match self.domain {
            Domain::Physical | Domain::Engineering => {
                if mask.has(&Frame::Science) {
                    let t = inputs.iter().find(|t| t.frame == Frame::Science).unwrap();
                    ArbitrationResult::Commit(t.clone())
                } else {
                    ArbitrationResult::ForceCollapse
                }
            }
            Domain::MedicalEthics => {
                if mask.has(&Frame::Individual) {
                    let t = inputs
                        .iter()
                        .find(|t| t.frame == Frame::Individual)
                        .unwrap();
                    ArbitrationResult::Preserve(t.clone())
                } else {
                    ArbitrationResult::Negotiate
                }
            }
            Domain::ValueJudgment => ArbitrationResult::Hold,
            Domain::Custom(ref name) => {
                info!(custom_domain = %name, "custom domain arbitration: defaulting to Negotiate");
                ArbitrationResult::Negotiate
            }
            Domain::General => {
                if mask.count() == 1 {
                    // All same frame (single bit set)
                    ArbitrationResult::Commit(inputs[0].clone())
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
