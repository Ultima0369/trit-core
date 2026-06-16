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

    /// Arithmetic mean of two phases.
    pub fn mean(a: Phase, b: Phase) -> Phase {
        Phase::new((a.0 + b.0) / 2.0)
    }

    /// Complement: 1.0 - phase.
    pub fn complement(self) -> Phase {
        Phase::new(1.0 - self.0)
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
