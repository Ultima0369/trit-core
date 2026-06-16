/// Oscillator for phase-based time-scale management.
/// Each computation domain can request a different sampling frequency.
pub struct HarmonicClock {
    omega: f64, // angular frequency
    phi0: f64,  // initial phase offset
    t: f64,     // current time
}

impl HarmonicClock {
    pub fn new(omega: f64, phi0: f64) -> Self {
        Self {
            omega,
            phi0,
            t: 0.0,
        }
    }

    /// Sample at current time. Returns true if rising zero-crossing.
    pub fn tick(&mut self, dt: f64) -> bool {
        let h_prev = (self.omega * self.t + self.phi0).sin();
        self.t += dt;
        let h_curr = (self.omega * self.t + self.phi0).sin();
        h_prev <= 0.0 && h_curr > 0.0
    }

    pub fn phase_now(&self) -> f64 {
        (self.omega * self.t + self.phi0).sin()
    }

    /// Fast clock for physical/ engineering domains.
    pub fn physical() -> Self {
        Self::new(10.0, 0.0)
    }

    /// Slow clock for ethical / value domains.
    pub fn deliberative() -> Self {
        Self::new(0.5, 0.0)
    }
}
