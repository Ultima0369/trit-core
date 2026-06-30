use thiserror::Error;

/// Phase: continuous tendency `0.0..1.0`.
/// `0.5` = perfectly neutral.
/// `> 0.5` = tendency toward True.
/// `< 0.5` = tendency toward False.
///
/// # Invariant
///
/// The wrapped `f64` is always finite and within `[0.0, 1.0]`.
/// Use [`Phase::new`] for strict construction, or [`Phase::new_clamped`]
/// when the caller explicitly wants silent clamping.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Phase(f64);

impl Phase {
    pub const NEUTRAL: f64 = 0.5;
    pub const FULL_TRUE: f64 = 1.0;
    pub const FULL_FALSE: f64 = 0.0;

    /// Constant neutral phase (0.5).
    pub const fn neutral() -> Self {
        Phase(0.5)
    }

    /// Constant full-true phase (1.0).
    pub const fn full_true() -> Self {
        Phase(1.0)
    }

    /// Constant full-false phase (0.0).
    pub const fn full_false() -> Self {
        Phase(0.0)
    }

    /// Strict constructor: returns `Err` for NaN, infinite, or out-of-range values.
    ///
    /// This is the default way to create a `Phase` in library code, because
    /// invalid phase values usually indicate a programming or input error that
    /// should not be silently hidden.
    pub fn new(v: f64) -> Result<Self, PhaseError> {
        if v.is_nan() || v.is_infinite() {
            return Err(PhaseError::NotFinite(v));
        }
        if !(0.0..=1.0).contains(&v) {
            return Err(PhaseError::OutOfRange(v));
        }
        Ok(Phase(v))
    }

    /// Clamping constructor: silently clamps out-of-range values to `[0.0, 1.0]`
    /// and maps NaN/Infinity to `NEUTRAL` (0.5).
    ///
    /// Logs a warning at the tracing layer when clamping occurs.
    /// Prefer [`Phase::new`] unless you explicitly need graceful degradation
    /// for external, untrusted inputs.
    pub fn new_clamped(v: f64) -> Self {
        if v.is_nan() || v.is_infinite() {
            tracing::warn!(value = %v, "Phase is NaN/Inf, clamping to NEUTRAL (0.5)");
            return Phase(0.5);
        }
        if !(0.0..=1.0).contains(&v) {
            let clamped = v.clamp(0.0, 1.0);
            tracing::warn!(original = %v, clamped = %clamped, "Phase out of range, clamped");
            return Phase(clamped);
        }
        Phase(v)
    }

    /// Unwrap the underlying finite, in-range `f64`.
    pub fn inner(self) -> f64 {
        self.0
    }

    /// Arithmetic mean of two phases, auto-quantized to prevent drift.
    pub fn mean(a: Phase, b: Phase) -> Phase {
        // Both values are already validated, so the mean is always valid.
        Phase::new_clamped((a.0 + b.0) / 2.0).quantize(1e-6)
    }

    /// Complement: 1.0 - phase, auto-quantized.
    pub fn complement(self) -> Phase {
        Phase::new_clamped(1.0 - self.0).quantize(1e-6)
    }

    /// Quantize to standard values (0.0, 0.5, 1.0) when within epsilon distance.
    /// This prevents phase drift over long cascades where 0.50000001 and
    /// 0.49999999 would be semantically different despite both meaning "neutral".
    ///
    /// Anchors are checked in order: 0.5 first (most common), then 0.0, then 1.0.
    ///
    /// If `epsilon` is not a positive finite number, the phase is returned
    /// unchanged. This prevents accidental no-op or nonsensical behavior from
    /// invalid epsilon inputs.
    pub fn quantize(self, epsilon: f64) -> Phase {
        if !epsilon.is_finite() || epsilon <= 0.0 {
            return self;
        }
        let v = self.0;
        if (v - 0.5).abs() < epsilon {
            Phase(0.5)
        } else if v < epsilon {
            Phase(0.0)
        } else if (1.0 - v).abs() < epsilon {
            Phase(1.0)
        } else {
            self
        }
    }

    /// Determine commitment direction.
    ///
    /// Uses the same epsilon as `quantize` (1e-6) for consistency:
    /// a value that `quantize` considers neutral will also be
    /// classified as `Neutral` by `commitment`.
    pub fn commitment(self) -> Commitment {
        let epsilon = 1e-6;
        if self.0 > 0.5 + epsilon {
            Commitment::TowardTrue
        } else if self.0 < 0.5 - epsilon {
            Commitment::TowardFalse
        } else {
            Commitment::Neutral
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Commitment {
    TowardTrue,
    TowardFalse,
    Neutral,
}

/// Error type for strict Phase construction.
#[derive(Debug, Clone, Copy, PartialEq, Error)]
pub enum PhaseError {
    #[error("Phase must be finite, got: {0}")]
    NotFinite(f64),
    #[error("Phase must be in [0.0, 1.0], got: {0}")]
    OutOfRange(f64),
}

// ── PhaseTracker: trend and velocity over time ─────────────────────

/// Tracks Phase evolution over successive observations.
///
/// Computes:
/// - **delta**: change since the previous observation (raw difference)
/// - **velocity**: smoothed rate of change (exponential moving average)
/// - **trend**: direction of movement (TowardTrue / TowardFalse / Stable)
///
/// The tracker answers "which way is this going, and how fast?" —
/// distinct from the instantaneous Phase value which answers
/// "where is it right now?"
#[derive(Debug, Clone, PartialEq)]
pub struct PhaseTracker {
    /// Most recently observed Phase.
    current: Phase,
    /// Previous observation (None until second update).
    previous: Option<Phase>,
    /// Smoothed velocity (EMA of deltas, α = 0.3).
    velocity: f64,
    /// Number of observations recorded.
    observations: u64,
}

impl PhaseTracker {
    /// Create a new tracker seeded with an initial Phase observation.
    pub fn new(initial: Phase) -> Self {
        PhaseTracker {
            current: initial,
            previous: None,
            velocity: 0.0,
            observations: 1,
        }
    }

    /// Feed a new Phase observation. Returns the computed delta.
    ///
    /// On each update, the tracker:
    /// 1. Shifts current → previous
    /// 2. Records new value as current
    /// 3. Computes raw delta = new - old
    /// 4. Updates smoothed velocity via EMA (α = 0.3)
    pub fn update(&mut self, next: Phase) -> f64 {
        let old = self.current;
        self.previous = Some(old);
        self.current = next;
        self.observations += 1;

        let delta = next.inner() - old.inner();
        // EMA: velocity = α·delta + (1-α)·velocity
        const ALPHA: f64 = 0.3;
        self.velocity = ALPHA * delta + (1.0 - ALPHA) * self.velocity;
        delta
    }

    /// Current Phase value.
    pub fn current(&self) -> Phase {
        self.current
    }

    /// Previous Phase value (None until second observation).
    pub fn previous(&self) -> Option<Phase> {
        self.previous
    }

    /// Smoothed velocity (EMA of deltas).
    ///
    /// Positive = trending toward True.
    /// Negative = trending toward False.
    /// Near zero = stable.
    pub fn velocity(&self) -> f64 {
        self.velocity
    }

    /// Direction of movement.
    pub fn trend(&self) -> Trend {
        Trend::from_velocity(self.velocity)
    }

    /// Number of observations recorded.
    pub fn observations(&self) -> u64 {
        self.observations
    }

    /// Whether enough observations exist to trust the trend (≥ 3).
    pub fn trend_reliable(&self) -> bool {
        self.observations >= 3
    }
}

/// Direction of Phase movement over time.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Trend {
    /// Moving toward True (velocity > +ε).
    TowardTrue,
    /// Moving toward False (velocity < -ε).
    TowardFalse,
    /// Not moving significantly (|velocity| ≤ ε).
    Stable,
}

impl Trend {
    /// ε threshold: velocities smaller than this are considered stable.
    const EPSILON: f64 = 1e-4;

    /// Classify a velocity into a trend direction.
    pub fn from_velocity(v: f64) -> Self {
        if v > Self::EPSILON {
            Trend::TowardTrue
        } else if v < -Self::EPSILON {
            Trend::TowardFalse
        } else {
            Trend::Stable
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Test helper: create a Phase from a valid f64, panicking on invalid input.
    /// Reduces `Phase::new(x).unwrap()` boilerplate in tests.
    fn p(v: f64) -> Phase {
        Phase::new(v).expect("test phase value must be valid")
    }

    #[test]
    fn new_rejects_nan() {
        assert!(Phase::new(f64::NAN).is_err());
    }

    #[test]
    fn new_rejects_inf() {
        assert!(Phase::new(f64::INFINITY).is_err());
        assert!(Phase::new(f64::NEG_INFINITY).is_err());
    }

    #[test]
    fn new_rejects_out_of_range() {
        assert!(Phase::new(1.5).is_err());
        assert!(Phase::new(-0.1).is_err());
    }

    #[test]
    fn new_accepts_valid() {
        assert!(Phase::new(0.0).is_ok());
        assert!(Phase::new(0.5).is_ok());
        assert!(Phase::new(1.0).is_ok());
        assert!(Phase::new(0.73).is_ok());
    }

    #[test]
    fn new_clamped_maps_nan_to_neutral() {
        assert_eq!(Phase::new_clamped(f64::NAN).inner(), 0.5);
    }

    #[test]
    fn new_clamped_clamps_out_of_range() {
        assert_eq!(Phase::new_clamped(1.5).inner(), 1.0);
        assert_eq!(Phase::new_clamped(-0.1).inner(), 0.0);
    }

    #[test]
    fn quantize_snaps_near_neutral() {
        let phase = p(0.5000000001);
        let q = phase.quantize(1e-6);
        assert_float_eq!(q.inner(), 0.5);
    }

    #[test]
    fn quantize_snaps_near_zero() {
        let phase = p(0.0000000001);
        let q = phase.quantize(1e-6);
        assert_float_eq!(q.inner(), 0.0);
    }

    #[test]
    fn quantize_snaps_near_one() {
        let phase = p(0.9999999999);
        let q = phase.quantize(1e-6);
        assert_float_eq!(q.inner(), 1.0);
    }

    #[test]
    fn quantize_preserves_normal_value() {
        let phase = p(0.73);
        let q = phase.quantize(1e-6);
        assert_float_eq!(q.inner(), 0.73);
    }

    #[test]
    fn quantize_neutral_priority_over_extremes() {
        // 0.50000001 is closer to 0.5 than 1.0 but we anchor 0.5 first
        let phase = p(0.50000001);
        let q = phase.quantize(1e-3);
        assert_float_eq!(q.inner(), 0.5);
    }

    #[test]
    fn mean_auto_quantizes() {
        let a = p(0.3);
        let b = p(0.7);
        let m = Phase::mean(a, b);
        // 0.3 + 0.7 = 1.0 / 2 = 0.5 exactly → should be 0.5
        assert_float_eq!(m.inner(), 0.5);
    }

    #[test]
    fn complement_auto_quantizes() {
        let phase = p(0.5);
        let c = phase.complement();
        // 1.0 - 0.5 = 0.5 → should be 0.5
        assert_float_eq!(c.inner(), 0.5);
    }

    #[test]
    fn commitment_neutral_at_0_5() {
        assert_eq!(p(0.5).commitment(), Commitment::Neutral);
    }

    #[test]
    fn commitment_neutral_near_0_5_within_epsilon() {
        // 0.5 + 5e-7 is within the 1e-6 epsilon → Neutral
        assert_eq!(p(0.5000005).commitment(), Commitment::Neutral);
        assert_eq!(p(0.4999995).commitment(), Commitment::Neutral);
    }

    #[test]
    fn commitment_toward_true_beyond_epsilon() {
        assert_eq!(p(0.500002).commitment(), Commitment::TowardTrue);
        assert_eq!(p(0.8).commitment(), Commitment::TowardTrue);
        assert_eq!(p(1.0).commitment(), Commitment::TowardTrue);
    }

    #[test]
    fn commitment_toward_false_beyond_epsilon() {
        assert_eq!(p(0.499998).commitment(), Commitment::TowardFalse);
        assert_eq!(p(0.2).commitment(), Commitment::TowardFalse);
        assert_eq!(p(0.0).commitment(), Commitment::TowardFalse);
    }

    #[test]
    fn commitment_consistent_with_quantize() {
        // A value quantized to neutral must also be commitment-neutral
        let phase = p(0.5000001).quantize(1e-6);
        assert_float_eq!(phase.inner(), 0.5);
        assert_eq!(phase.commitment(), Commitment::Neutral);
    }

    #[test]
    fn constants_match_expected_values() {
        assert_eq!(Phase::neutral().inner(), 0.5);
        assert_eq!(Phase::full_true().inner(), 1.0);
        assert_eq!(Phase::full_false().inner(), 0.0);
    }

    #[test]
    fn new_clamped_maps_infinity_to_neutral() {
        assert_eq!(Phase::new_clamped(f64::INFINITY).inner(), 0.5);
        assert_eq!(Phase::new_clamped(f64::NEG_INFINITY).inner(), 0.5);
    }

    #[test]
    fn quantize_rejects_invalid_epsilon() {
        let phase = p(0.5000001);
        // Negative, zero, or non-finite epsilon should leave the phase unchanged.
        assert_float_eq!(phase.quantize(-1e-6).inner(), 0.5000001);
        assert_float_eq!(phase.quantize(0.0).inner(), 0.5000001);
        assert_float_eq!(phase.quantize(f64::NAN).inner(), 0.5000001);
        assert_float_eq!(phase.quantize(f64::INFINITY).inner(), 0.5000001);
    }

    #[test]
    fn mean_of_extremes_is_neutral() {
        let a = Phase::full_false();
        let b = Phase::full_true();
        let m = Phase::mean(a, b);
        assert_float_eq!(m.inner(), 0.5);
    }

    #[test]
    fn complement_of_extremes() {
        assert_float_eq!(Phase::full_true().complement().inner(), 0.0);
        assert_float_eq!(Phase::full_false().complement().inner(), 1.0);
        assert_float_eq!(Phase::neutral().complement().inner(), 0.5);
    }

    #[test]
    fn inner_returns_wrapped_value() {
        let phase = p(0.42);
        assert_eq!(phase.inner(), 0.42);
    }

    // ── PhaseTracker tests ─────────────────────────────────────────

    #[test]
    fn tracker_starts_with_initial_phase() {
        let tracker = PhaseTracker::new(p(0.6));
        assert_float_eq!(tracker.current().inner(), 0.6);
        assert!(tracker.previous().is_none());
        assert_eq!(tracker.velocity(), 0.0);
        assert_eq!(tracker.observations(), 1);
        assert!(!tracker.trend_reliable());
    }

    #[test]
    fn tracker_computes_delta_and_velocity() {
        let mut tracker = PhaseTracker::new(p(0.5));
        // Move toward True
        let delta = tracker.update(p(0.6));
        assert_float_eq!(delta, 0.1);
        // velocity = 0.3 * 0.1 + 0.7 * 0.0 = 0.03
        assert_float_eq!(tracker.velocity(), 0.03);
        assert_eq!(tracker.observations(), 2);
        assert!(!tracker.trend_reliable());
    }

    #[test]
    fn tracker_velocity_ema_smooths_over_time() {
        let mut tracker = PhaseTracker::new(p(0.5));
        tracker.update(p(0.6)); // delta=0.1, v=0.03
        tracker.update(p(0.7)); // delta=0.1, v=0.3*0.1+0.7*0.03=0.051
                                // v = 0.3 * 0.1 + 0.7 * 0.03 = 0.03 + 0.021 = 0.051
        assert_float_eq!(tracker.velocity(), 0.051);
        assert!(tracker.trend_reliable());
    }

    #[test]
    fn tracker_trend_toward_true() {
        let mut tracker = PhaseTracker::new(p(0.5));
        tracker.update(p(0.6));
        tracker.update(p(0.7));
        assert_eq!(tracker.trend(), Trend::TowardTrue);
    }

    #[test]
    fn tracker_trend_toward_false() {
        let mut tracker = PhaseTracker::new(p(0.5));
        tracker.update(p(0.4));
        tracker.update(p(0.3));
        assert_eq!(tracker.trend(), Trend::TowardFalse);
    }

    #[test]
    fn tracker_trend_stable() {
        let mut tracker = PhaseTracker::new(p(0.5));
        // Very small movements — velocity stays near zero
        tracker.update(p(0.50001));
        tracker.update(p(0.49999));
        assert_eq!(tracker.trend(), Trend::Stable);
    }

    #[test]
    fn tracker_trend_reverts_to_stable() {
        let mut tracker = PhaseTracker::new(p(0.5));
        tracker.update(p(0.8)); // big jump, v = 0.3 * 0.3 = 0.09
        assert_eq!(tracker.trend(), Trend::TowardTrue);
        // Many steps at stable 0.8 — velocity decays via EMA toward 0.
        // After 30 steps of delta=0: v = 0.09 * 0.7^30 ≈ 2e-6
        for _ in 0..30 {
            tracker.update(p(0.8));
        }
        assert!(tracker.velocity().abs() < 1e-4);
        assert_eq!(tracker.trend(), Trend::Stable);
    }

    #[test]
    fn trend_from_velocity_classifies() {
        assert_eq!(Trend::from_velocity(0.01), Trend::TowardTrue);
        assert_eq!(Trend::from_velocity(-0.01), Trend::TowardFalse);
        assert_eq!(Trend::from_velocity(0.00005), Trend::Stable);
        assert_eq!(Trend::from_velocity(-0.00005), Trend::Stable);
        assert_eq!(Trend::from_velocity(0.0), Trend::Stable);
    }
}
