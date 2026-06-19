use crate::meta::Domain;
use crate::sandbox::error::SandboxError;
use crate::sandbox::input::{ScenarioInput, SignalInput};

/// Security and sanity limits for scenario inputs.
pub const MAX_JSON_SIZE: usize = 64 * 1024;
pub const MAX_SIGNALS: usize = 100;
pub const MAX_STRING_LEN: usize = 1024;

/// Validate a scenario's content.
///
/// Enforces:
/// - ID / description length limits
/// - At least one signal, at most `MAX_SIGNALS`
/// - Known domain
/// - Per-signal phase, value, and frame validity
pub fn validate_scenario(scenario: &ScenarioInput) -> Result<(), SandboxError> {
    if scenario.id.len() > MAX_STRING_LEN {
        return Err(SandboxError::InvalidScenario(format!(
            "id too long: {} chars (max {})",
            scenario.id.len(),
            MAX_STRING_LEN
        )));
    }
    if scenario.description.len() > MAX_STRING_LEN * 4 {
        return Err(SandboxError::InvalidScenario(
            "description too long".to_string(),
        ));
    }
    if scenario.signals.is_empty() {
        return Err(SandboxError::InvalidScenario(
            "At least one signal is required".to_string(),
        ));
    }
    if scenario.signals.len() > MAX_SIGNALS {
        return Err(SandboxError::InvalidScenario(format!(
            "Too many signals: {} (max {})",
            scenario.signals.len(),
            MAX_SIGNALS
        )));
    }

    validate_domain(&scenario.domain)?;

    for (i, signal) in scenario.signals.iter().enumerate() {
        validate_signal(i, signal)?;
    }

    Ok(())
}

/// Validate that the domain string is known.
pub fn validate_domain(domain: &str) -> Result<(), SandboxError> {
    domain
        .parse::<Domain>()
        .map(|_| ())
        .map_err(|e| SandboxError::InvalidDomain(format!("{}", e)))
}

/// Validate a single signal.
pub fn validate_signal(index: usize, signal: &SignalInput) -> Result<(), SandboxError> {
    if signal.phase.is_nan() || signal.phase.is_infinite() || !(0.0..=1.0).contains(&signal.phase) {
        return Err(SandboxError::InvalidPhase {
            index,
            reason: format!(
                "phase {} is invalid (must be finite in [0.0, 1.0])",
                signal.phase
            ),
        });
    }
    if !matches!(signal.value, -1..=1) {
        return Err(SandboxError::InvalidValue {
            index,
            reason: format!("value {} is invalid (must be 1, 0, or -1)", signal.value),
        });
    }
    // Meta frame is system-internal (output of cross-frame conflict resolution).
    // External signal inputs may use concrete decision frames plus the
    // first-person reference frames introduced by the mind-engineering extension.
    match signal.frame.as_str() {
        "Science" | "Individual" | "Consensus" | "Absolute" | "FirstPerson" | "Embodied"
        | "Relational" => Ok(()),
        f => Err(SandboxError::InvalidFrame {
            index,
            reason: format!("unknown frame '{}'", f),
        }),
    }
}

/// Sanitize a free-form string for log/output emission.
pub fn sanitize_log_field(s: &str) -> String {
    s.chars()
        .map(|c| {
            if c.is_control() && c != ' ' {
                '\u{FFFD}'
            } else {
                c
            }
        })
        .take(256)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn signal(frame: &str, value: i8, phase: f64) -> SignalInput {
        SignalInput {
            frame: frame.into(),
            value,
            phase,
            sensor: None,
        }
    }

    fn scenario(id: &str, domain: &str, signals: Vec<SignalInput>) -> ScenarioInput {
        ScenarioInput {
            id: id.into(),
            description: "test".into(),
            domain: domain.into(),
            signals,
            expected_behavior: "hold".into(),
            environmental_context: None,
        }
    }

    #[test]
    fn validate_accepts_valid_scenario() {
        let s = scenario("x", "General", vec![signal("Science", 1, 0.5)]);
        assert!(validate_scenario(&s).is_ok());
    }

    #[test]
    fn validate_rejects_empty_signals() {
        let s = scenario("x", "General", vec![]);
        assert!(validate_scenario(&s).is_err());
    }

    #[test]
    fn validate_rejects_too_many_signals() {
        let signals: Vec<_> = (0..=MAX_SIGNALS)
            .map(|_| signal("Science", 1, 0.5))
            .collect();
        let s = scenario("x", "General", signals);
        assert!(validate_scenario(&s).is_err());
    }

    #[test]
    fn validate_rejects_invalid_domain() {
        let s = scenario("x", "Mystic", vec![signal("Science", 1, 0.5)]);
        assert!(validate_scenario(&s).is_err());
    }

    #[test]
    fn validate_accepts_custom_domain() {
        let s = scenario("x", "Custom(chemistry)", vec![signal("Science", 1, 0.5)]);
        assert!(validate_scenario(&s).is_ok());
    }

    #[test]
    fn validate_rejects_empty_custom_domain() {
        let s = scenario("x", "Custom()", vec![signal("Science", 1, 0.5)]);
        assert!(validate_scenario(&s).is_err());
    }

    #[test]
    fn validate_rejects_invalid_phase() {
        let s = scenario("x", "General", vec![signal("Science", 1, 1.5)]);
        assert!(validate_scenario(&s).is_err());
    }

    #[test]
    fn validate_rejects_nan_phase() {
        let s = scenario("x", "General", vec![signal("Science", 1, f64::NAN)]);
        assert!(validate_scenario(&s).is_err());
    }

    #[test]
    fn validate_rejects_invalid_value() {
        let s = scenario("x", "General", vec![signal("Science", 2, 0.5)]);
        assert!(validate_scenario(&s).is_err());
    }

    #[test]
    fn validate_rejects_invalid_frame() {
        let s = scenario("x", "General", vec![signal("Bogus", 1, 0.5)]);
        assert!(validate_scenario(&s).is_err());
    }

    #[test]
    fn validate_rejects_long_id() {
        let s = scenario(
            &"x".repeat(MAX_STRING_LEN + 1),
            "General",
            vec![signal("Science", 1, 0.5)],
        );
        assert!(validate_scenario(&s).is_err());
    }

    #[test]
    fn sanitize_replaces_control_chars() {
        let sanitized = sanitize_log_field("hello\nworld");
        assert!(!sanitized.contains('\n'));
        assert!(sanitized.contains("hello"));
        assert!(sanitized.contains("world"));
    }

    #[test]
    fn sanitize_truncates_long_strings() {
        let long = "x".repeat(512);
        let sanitized = sanitize_log_field(&long);
        assert_eq!(sanitized.len(), 256);
    }
}
