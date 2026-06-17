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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn should_detect_rising_zero_crossing_on_tick() {
        let mut clock = HarmonicClock::new(1.0, 0.0);
        assert!(clock.tick(0.1));
    }

    #[test]
    fn should_not_detect_crossing_when_descending() {
        let mut clock = HarmonicClock::new(1.0, std::f64::consts::PI);
        assert!(!clock.tick(0.1));
    }

    #[test]
    fn phase_now_should_return_sine_of_current_angle() {
        let clock = HarmonicClock::new(1.0, std::f64::consts::FRAC_PI_2);
        assert!((clock.phase_now() - 1.0).abs() < f64::EPSILON);
    }

    #[test]
    fn physical_clock_should_have_high_frequency() {
        let clock = HarmonicClock::physical();
        assert!(clock.omega > 5.0);
    }

    #[test]
    fn deliberative_clock_should_have_low_frequency() {
        let clock = HarmonicClock::deliberative();
        assert!(clock.omega < 5.0);
    }

    #[test]
    fn multiple_ticks_should_accumulate_time() {
        let mut clock = HarmonicClock::new(1.0, 0.0);
        clock.tick(0.1);
        clock.tick(0.1);
        clock.tick(0.1);
        assert!((clock.phase_now() - 0.3_f64.sin()).abs() < 0.01);
    }
}
