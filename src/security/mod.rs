//! Security-mode state machine for Trit-Core.
//!
//! Implements the four-state model from the Aurora CHARTER:
//! - Service: normal operation
//! - Refusal: output Hold + explanation, stop computation
//! - Awareness: output notification, continue computation
//! - Transparency: disclose all internal state, continue computation

use serde::{Deserialize, Serialize};

/// Operating mode of the security layer.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Deserialize, Serialize)]
pub enum SecurityMode {
    /// Normal operation: compute, arbitrate, and emit results as usual.
    #[default]
    Service,
    /// Stop computation and emit Hold with an explanation.
    Refusal,
    /// Continue computation but emit a policy-violation notification.
    Awareness,
    /// Continue computation and disclose all internal state.
    Transparency,
}

impl SecurityMode {
    /// Whether the mode permits computation to continue.
    pub fn allows_computation(&self) -> bool {
        !matches!(self, SecurityMode::Refusal)
    }

    /// Whether the mode requires a notification to be emitted.
    pub fn requires_notification(&self) -> bool {
        matches!(self, SecurityMode::Awareness | SecurityMode::Transparency)
    }
}
