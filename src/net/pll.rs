/// Software Phase-Locked Loop controller per ADR-004 §5.
///
/// Maintains phase synchronization between coupled nodes using
/// proportional correction with a deadband to prevent over-correction
/// of small phase differences.
#[derive(Debug, Clone)]
pub struct PllController {
    /// Proportional gain (default 0.3).
    pub kp: f64,
    /// Deadband threshold: corrections below this are ignored (default 0.05).
    pub deadband: f64,
    /// Maximum single-step correction (default 0.1).
    pub max_correction: f64,
    /// Accumulated correction over the coupling lifetime.
    pub total_correction: f64,
}

impl PllController {
    /// Create a PLL with default parameters.
    pub fn new() -> Self {
        Self {
            kp: 0.3,
            deadband: 0.05,
            max_correction: 0.1,
            total_correction: 0.0,
        }
    }

    /// Create a PLL with custom parameters.
    pub fn with_params(kp: f64, deadband: f64, max_correction: f64) -> Self {
        Self {
            kp,
            deadband,
            max_correction,
            total_correction: 0.0,
        }
    }

    /// Compute the phase correction needed to synchronize local_phase toward peer_phase.
    ///
    /// Returns 0.0 if the error is within the deadband.
    /// Returns a clamped correction proportional to the error otherwise.
    pub fn compute_correction(&mut self, local_phase: f64, peer_phase: f64) -> f64 {
        let error = peer_phase - local_phase;

        // Deadband: ignore small errors to prevent oscillation
        if error.abs() <= self.deadband {
            return 0.0;
        }

        // Proportional correction
        let raw_correction = error * self.kp;

        // Clamp to max single-step correction
        let correction = raw_correction.clamp(-self.max_correction, self.max_correction);

        self.total_correction += correction;
        correction
    }

    /// Check if the phase difference between two nodes indicates a conflict.
    /// Per ADR-004 §3.2: |phase_a - phase_b| > 0.3 → destructive interference.
    pub fn is_conflict_phase_gap(phase_a: f64, phase_b: f64) -> bool {
        (phase_a - phase_b).abs() > 0.3
    }

    /// Check if the phase difference indicates a jump anomaly (>0.5 single-step).
    pub fn is_phase_jump_anomaly(old_phase: f64, new_phase: f64) -> bool {
        (new_phase - old_phase).abs() > 0.5
    }

    /// Reset accumulated correction (called on decouple).
    pub fn reset(&mut self) {
        self.total_correction = 0.0;
    }
}

impl Default for PllController {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pll_corrects_toward_peer() {
        let mut pll = PllController::new();
        let correction = pll.compute_correction(0.5, 0.8);
        assert!(correction > 0.0); // should pull local toward peer
        assert!(correction <= pll.max_correction);
    }

    #[test]
    fn pll_deadband_ignores_small_error() {
        let mut pll = PllController::new();
        let correction = pll.compute_correction(0.5, 0.52); // error = 0.02 < deadband 0.05
        assert_eq!(correction, 0.0);
    }

    #[test]
    fn pll_clamps_large_correction() {
        let mut pll = PllController::new();
        let correction = pll.compute_correction(0.0, 1.0); // error = 1.0, raw = 0.3
        assert!(correction <= pll.max_correction);
    }

    #[test]
    fn conflict_phase_gap_detected() {
        assert!(PllController::is_conflict_phase_gap(0.1, 0.9));
        assert!(!PllController::is_conflict_phase_gap(0.4, 0.6));
    }

    #[test]
    fn phase_jump_anomaly_detected() {
        assert!(PllController::is_phase_jump_anomaly(0.2, 0.8));
        assert!(!PllController::is_phase_jump_anomaly(0.2, 0.5));
    }
}
