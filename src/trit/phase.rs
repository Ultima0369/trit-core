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

    pub fn new(v: f64) -> Self {
        assert!((0.0..=1.0).contains(&v), "Phase must be in [0.0, 1.0]");
        Phase(v)
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
