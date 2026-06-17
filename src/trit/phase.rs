/// Phase: continuous tendency `0.0..1.0`.
/// `0.5` = perfectly neutral.
/// `> 0.5` = tendency toward True.
/// `< 0.5` = tendency toward False.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Phase(f64);

impl Phase {
    pub const NEUTRAL: f64 = 0.5;
    pub const FULL_TRUE: f64 = 1.0;
    pub const FULL_FALSE: f64 = 0.0;

    /// Construct a Phase, clamping out-of-range values to [0.0, 1.0].
    /// NaN and Infinity are clamped to NEUTRAL (0.5).
    /// Logs a warning at the tracing layer when clamping occurs.
    pub fn new(v: f64) -> Self {
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

    /// Strict constructor: returns Err for invalid values.
    /// Use this when the caller needs to reject bad input rather than clamp.
    pub fn try_new(v: f64) -> Result<Self, String> {
        if v.is_nan() || v.is_infinite() {
            return Err(format!("Phase must be finite, got: {}", v));
        }
        if !(0.0..=1.0).contains(&v) {
            return Err(format!("Phase must be in [0.0, 1.0], got: {}", v));
        }
        Ok(Phase(v))
    }

    pub fn inner(self) -> f64 {
        self.0
    }

    /// Arithmetic mean of two phases, auto-quantized to prevent drift.
    pub fn mean(a: Phase, b: Phase) -> Phase {
        Phase::new((a.0 + b.0) / 2.0).quantize(1e-6)
    }

    /// Complement: 1.0 - phase, auto-quantized.
    pub fn complement(self) -> Phase {
        Phase::new(1.0 - self.0).quantize(1e-6)
    }

    /// Quantize to standard values (0.0, 0.5, 1.0) when within epsilon distance.
    /// This prevents phase drift over long cascades where 0.50000001 and
    /// 0.49999999 would be semantically different despite both meaning "neutral".
    ///
    /// Anchors are checked in order: 0.5 first (most common), then 0.0, then 1.0.
    pub fn quantize(self, epsilon: f64) -> Phase {
        let v = self.0;
        if (v - 0.5).abs() < epsilon {
            Phase(0.5)
        } else if v < epsilon {
            Phase(0.0)
        } else if (v - 1.0).abs() < epsilon {
            Phase(1.0)
        } else {
            self
        }
    }

    /// Determine commitment direction.
    pub fn commitment(self) -> Commitment {
        if self.0 > 0.5 + f64::EPSILON {
            Commitment::TowardTrue
        } else if self.0 < 0.5 - f64::EPSILON {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn quantize_snaps_near_neutral() {
        let p = Phase(0.5000000001);
        let q = p.quantize(1e-6);
        assert!((q.inner() - 0.5).abs() < f64::EPSILON);
    }

    #[test]
    fn quantize_snaps_near_zero() {
        let p = Phase(0.0000000001);
        let q = p.quantize(1e-6);
        assert!((q.inner() - 0.0).abs() < f64::EPSILON);
    }

    #[test]
    fn quantize_snaps_near_one() {
        let p = Phase(0.9999999999);
        let q = p.quantize(1e-6);
        assert!((q.inner() - 1.0).abs() < f64::EPSILON);
    }

    #[test]
    fn quantize_preserves_normal_value() {
        let p = Phase(0.73);
        let q = p.quantize(1e-6);
        assert!((q.inner() - 0.73).abs() < f64::EPSILON);
    }

    #[test]
    fn quantize_neutral_priority_over_extremes() {
        // 0.50000001 is closer to 0.5 than 1.0 but we anchor 0.5 first
        let p = Phase(0.50000001);
        let q = p.quantize(1e-3);
        assert!((q.inner() - 0.5).abs() < f64::EPSILON);
    }

    #[test]
    fn mean_auto_quantizes() {
        let a = Phase(0.3);
        let b = Phase(0.7);
        let m = Phase::mean(a, b);
        // 0.3 + 0.7 = 1.0 / 2 = 0.5 exactly → should be 0.5
        assert!((m.inner() - 0.5).abs() < f64::EPSILON);
    }

    #[test]
    fn complement_auto_quantizes() {
        let p = Phase(0.5);
        let c = p.complement();
        // 1.0 - 0.5 = 0.5 → should be 0.5
        assert!((c.inner() - 0.5).abs() < f64::EPSILON);
    }
}
