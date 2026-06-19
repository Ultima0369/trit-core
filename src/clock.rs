/// Oscillator for phase-based time-scale management.
///
/// Each computation domain can request a different sampling frequency.
/// Note: `phase_now()` returns values in `[-1.0, 1.0]` (raw sine output),
/// which differs from [`Phase`](crate::core::phase::Phase)'s `[0.0, 1.0]` domain.
/// Use [`to_phase()`](HarmonicClock::to_phase) for a mapped `[0.0, 1.0]` output.
///
/// ## Pipeline integration (v0.3.0)
///
/// The clock is now integrated into `SandboxPipeline`. Each `run()` call
/// ticks the clock by the elapsed wall-clock time. The domain selects the
/// clock preset via [`for_domain()`](HarmonicClock::for_domain):
/// - `physical()` (ω=10.0) for Physical, Engineering
/// - `deliberative()` (ω=0.5) for MedicalEthics, ValueJudgment, General, Custom
///
/// The clock phase feeds into `AttentionScheduler` as a modulation signal:
/// near phase peaks (0.8–1.0), the scheduler is more likely to suggest
/// [`ShiftTo`](crate::attention::AttentionCmd::ShiftTo); near troughs
/// (0.0–0.2), more likely to [`HoldCurrent`](crate::attention::AttentionCmd::HoldCurrent).
#[derive(Debug, Clone, PartialEq)]
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

    /// Current phase mapped to `[0.0, 1.0]` for [`Phase`](crate::core::phase::Phase) compatibility.
    ///
    /// Uses the transformation `(sin(angle) + 1.0) / 2.0` which maps:
    /// - `-1.0 → 0.0` (full false)
    /// - `0.0 → 0.5` (neutral)
    /// - `1.0 → 1.0` (full true)
    pub fn to_phase(&self) -> crate::core::phase::Phase {
        let raw = (self.omega * self.t + self.phi0).sin();
        crate::core::phase::Phase::new_clamped((raw + 1.0) / 2.0)
    }

    /// Fast clock for physical/ engineering domains.
    pub fn physical() -> Self {
        Self::new(10.0, 0.0)
    }

    /// Slow clock for ethical / value domains.
    pub fn deliberative() -> Self {
        Self::new(0.5, 0.0)
    }

    /// Select the appropriate clock preset for a domain.
    ///
    /// | Domain | Preset | ω |
    /// |--------|--------|---|
    /// | Physical, Engineering | `physical()` | 10.0 |
    /// | MedicalEthics, ValueJudgment, General, Custom | `deliberative()` | 0.5 |
    pub fn for_domain(domain: &crate::meta::Domain) -> Self {
        match domain {
            crate::meta::Domain::Physical | crate::meta::Domain::Engineering => Self::physical(),
            _ => Self::deliberative(),
        }
    }

    /// Returns the current time accumulator value.
    pub fn elapsed_time(&self) -> f64 {
        self.t
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::meta::Domain;

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

    #[test]
    fn tick_with_zero_dt_is_not_a_crossing() {
        let mut clock = HarmonicClock::new(1.0, 0.0);
        assert!(!clock.tick(0.0));
    }

    #[test]
    fn no_crossing_within_same_half_period() {
        // Start at peak (PI/2) and move a little; no rising zero crossing.
        let mut clock = HarmonicClock::new(1.0, std::f64::consts::FRAC_PI_2);
        assert!(!clock.tick(0.1));
        assert!(!clock.tick(0.1));
    }

    #[test]
    fn physical_clock_starts_at_zero_phase() {
        let clock = HarmonicClock::physical();
        assert!(clock.phase_now().abs() < f64::EPSILON);
    }

    #[test]
    fn deliberative_clock_starts_at_zero_phase() {
        let clock = HarmonicClock::deliberative();
        assert!(clock.phase_now().abs() < f64::EPSILON);
    }

    #[test]
    fn phase_now_bounds_are_within_one() {
        let mut clock = HarmonicClock::new(2.5, 0.3);
        for _ in 0..20 {
            clock.tick(0.05);
            let p = clock.phase_now();
            assert!((-1.0..=1.0).contains(&p), "phase {} out of [-1, 1]", p);
        }
    }

    // ── domain preset mapping ─────────────────────────────────────

    #[test]
    fn for_domain_physical_is_fast() {
        let clock = HarmonicClock::for_domain(&Domain::Physical);
        assert!(clock.omega > 5.0);
    }

    #[test]
    fn for_domain_engineering_is_fast() {
        let clock = HarmonicClock::for_domain(&Domain::Engineering);
        assert!(clock.omega > 5.0);
    }

    #[test]
    fn for_domain_medical_ethics_is_slow() {
        let clock = HarmonicClock::for_domain(&Domain::MedicalEthics);
        assert!(clock.omega < 5.0);
    }

    #[test]
    fn for_domain_value_judgment_is_slow() {
        let clock = HarmonicClock::for_domain(&Domain::ValueJudgment);
        assert!(clock.omega < 5.0);
    }

    #[test]
    fn for_domain_general_is_slow() {
        let clock = HarmonicClock::for_domain(&Domain::General);
        assert!(clock.omega < 5.0);
    }

    #[test]
    fn for_domain_custom_is_slow() {
        let clock = HarmonicClock::for_domain(&Domain::Custom("test".into()));
        assert!(clock.omega < 5.0);
    }

    #[test]
    fn elapsed_time_accumulates() {
        let mut clock = HarmonicClock::new(1.0, 0.0);
        clock.tick(0.5);
        clock.tick(0.3);
        assert!((clock.elapsed_time() - 0.8).abs() < f64::EPSILON);
    }

    #[test]
    fn to_phase_maps_to_unit_interval() {
        let mut clock = HarmonicClock::new(1.0, 0.0);
        for _ in 0..20 {
            clock.tick(0.1);
            let phase = clock.to_phase();
            let inner = phase.inner();
            assert!(
                (0.0..=1.0).contains(&inner),
                "to_phase {} not in [0,1]",
                inner
            );
        }
    }
}
