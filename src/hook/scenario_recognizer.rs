//! Scenario recognizer: feature vector extraction and prototype matching.
//!
//! Maps a scenario input to a [`ScenarioType`] by computing a feature
//! vector from the input signals and comparing it against known prototypes
//! via cosine similarity. Falls back to `General` when no prototype
//! exceeds the similarity threshold.

use super::ScenarioType;
use crate::sandbox::ScenarioInput;

/// Cosine similarity threshold below which the recognizer falls back
/// to `General`. Range: [0.0, 1.0]. Higher = stricter matching.
const SIMILARITY_THRESHOLD: f64 = 0.3;

/// A feature vector extracted from a scenario input.
///
/// Each dimension captures a structural property of the input signals.
#[derive(Debug, Clone, PartialEq)]
struct FeatureVector {
    /// Fraction of signals in the Science frame.
    science_ratio: f64,
    /// Fraction of signals in the Individual frame.
    individual_ratio: f64,
    /// Fraction of signals in the Consensus frame.
    consensus_ratio: f64,
    /// Fraction of signals in the FirstPerson frame.
    first_person_ratio: f64,
    /// Fraction of signals that are True.
    true_ratio: f64,
    /// Fraction of signals that are False.
    false_ratio: f64,
    /// Fraction of signals that are Hold.
    hold_ratio: f64,
    /// Mean phase across all signals.
    mean_phase: f64,
    /// Number of distinct frames present.
    distinct_frames: f64,
    /// Normalized signal count (capped at 10).
    signal_count_norm: f64,
}

impl FeatureVector {
    /// Extract a feature vector from a scenario input.
    fn from_scenario(scenario: &ScenarioInput) -> Self {
        let n = scenario.signals.len().max(1) as f64;

        let mut science = 0usize;
        let mut individual = 0usize;
        let mut consensus = 0usize;
        let mut first_person = 0usize;
        let mut tru = 0usize;
        let mut fals = 0usize;
        let mut hold = 0usize;
        let mut phase_sum = 0.0f64;
        let mut frame_set = std::collections::HashSet::new();

        for signal in &scenario.signals {
            match signal.frame.as_str() {
                "Science" => science += 1,
                "Individual" => individual += 1,
                "Consensus" => consensus += 1,
                "FirstPerson" => first_person += 1,
                _ => {}
            }
            frame_set.insert(signal.frame.clone());

            match signal.value {
                1 => tru += 1,
                -1 => fals += 1,
                0 => hold += 1,
                _ => {} // Unknown or invalid
            }
            phase_sum += signal.phase;
        }

        FeatureVector {
            science_ratio: science as f64 / n,
            individual_ratio: individual as f64 / n,
            consensus_ratio: consensus as f64 / n,
            first_person_ratio: first_person as f64 / n,
            true_ratio: tru as f64 / n,
            false_ratio: fals as f64 / n,
            hold_ratio: hold as f64 / n,
            mean_phase: phase_sum / n,
            distinct_frames: frame_set.len() as f64,
            signal_count_norm: (scenario.signals.len() as f64 / 10.0).min(1.0),
        }
    }

    /// Convert to a slice of 10 dimensions (for cosine similarity).
    fn as_slice(&self) -> [f64; 10] {
        [
            self.science_ratio,
            self.individual_ratio,
            self.consensus_ratio,
            self.first_person_ratio,
            self.true_ratio,
            self.false_ratio,
            self.hold_ratio,
            self.mean_phase,
            self.distinct_frames / 5.0, // normalize to [0, 1]
            self.signal_count_norm,
        ]
    }

    /// Cosine similarity with another feature vector.
    fn cosine_similarity(&self, other: &[f64; 10]) -> f64 {
        let a = self.as_slice();
        let dot: f64 = a.iter().zip(other.iter()).map(|(x, y)| x * y).sum();
        let norm_a: f64 = a.iter().map(|x| x * x).sum::<f64>().sqrt();
        let norm_b: f64 = other.iter().map(|x| x * x).sum::<f64>().sqrt();

        if norm_a < 1e-12 || norm_b < 1e-12 {
            return 0.0;
        }
        dot / (norm_a * norm_b)
    }
}

/// Prototype feature vectors for each scenario type.
///
/// These are hand-crafted based on expected signal patterns:
/// - PhysicalReasoning: high Science ratio, high True or False (decisive)
/// - ValueConflict: mixed frames, high Hold ratio
/// - MedicalEthics: high Individual or FirstPerson ratio
/// - ReflexiveAudit: high distinct_frames, mixed values
/// - CrisisResponse: high signal_count, high phase (urgency)
fn prototypes() -> Vec<(ScenarioType, [f64; 10])> {
    vec![
        (
            ScenarioType::PhysicalReasoning,
            // science, individual, consensus, first_person, true, false, hold, phase, frames, count
            [0.7, 0.0, 0.1, 0.0, 0.5, 0.4, 0.1, 0.7, 0.3, 0.2],
        ),
        (
            ScenarioType::ValueConflict,
            [0.2, 0.2, 0.2, 0.1, 0.3, 0.3, 0.4, 0.5, 0.6, 0.2],
        ),
        (
            ScenarioType::MedicalEthics,
            [0.2, 0.5, 0.1, 0.2, 0.3, 0.3, 0.4, 0.5, 0.4, 0.2],
        ),
        (
            ScenarioType::ReflexiveAudit,
            [0.2, 0.2, 0.2, 0.1, 0.3, 0.3, 0.3, 0.5, 0.8, 0.3],
        ),
        (
            ScenarioType::CrisisResponse,
            [0.4, 0.1, 0.1, 0.1, 0.5, 0.4, 0.1, 0.8, 0.3, 0.5],
        ),
    ]
}

/// Recognize the scenario type from a scenario input.
///
/// Computes a feature vector from the input signals, compares it against
/// known prototypes via cosine similarity, and returns the best match.
/// Falls back to `General` if no prototype exceeds the threshold.
pub fn recognize(scenario: &ScenarioInput) -> ScenarioType {
    let fv = FeatureVector::from_scenario(scenario);

    let mut best_type = ScenarioType::General;
    let mut best_sim = SIMILARITY_THRESHOLD;

    for (st, proto) in prototypes() {
        let sim = fv.cosine_similarity(&proto);
        if sim > best_sim {
            best_sim = sim;
            best_type = st;
        }
    }

    best_type
}

/// Recognize the scenario type and return it with the similarity score.
///
/// Useful for diagnostics: the similarity score indicates how well the
/// input matched the prototype.
pub fn recognize_with_score(scenario: &ScenarioInput) -> (ScenarioType, f64) {
    let fv = FeatureVector::from_scenario(scenario);

    let mut best_type = ScenarioType::General;
    let mut best_sim = SIMILARITY_THRESHOLD;

    for (st, proto) in prototypes() {
        let sim = fv.cosine_similarity(&proto);
        if sim > best_sim {
            best_sim = sim;
            best_type = st;
        }
    }

    (best_type, best_sim)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sandbox::SignalInput;

    fn make_scenario(frame: &str, value: i8, phase: f64) -> ScenarioInput {
        ScenarioInput {
            id: "test".into(),
            description: "test".into(),
            domain: "General".into(),
            signals: vec![SignalInput {
                frame: frame.into(),
                value,
                phase,
                sensor: None,
            }],
            expected_behavior: String::new(),
            environmental_context: None,
        }
    }

    #[test]
    fn empty_scenario_is_general() {
        let scenario = ScenarioInput {
            id: "empty".into(),
            description: "empty".into(),
            domain: "General".into(),
            signals: vec![],
            expected_behavior: String::new(),
            environmental_context: None,
        };
        let st = recognize(&scenario);
        assert_eq!(st, ScenarioType::General);
    }

    #[test]
    fn high_science_recognized_as_physical() {
        let scenario = ScenarioInput {
            id: "phys".into(),
            description: "test".into(),
            domain: "Physical".into(),
            signals: vec![
                SignalInput {
                    frame: "Science".into(),
                    value: 1,
                    phase: 0.9,
                    sensor: None,
                },
                SignalInput {
                    frame: "Science".into(),
                    value: -1,
                    phase: 0.8,
                    sensor: None,
                },
            ],
            expected_behavior: String::new(),
            environmental_context: None,
        };
        let st = recognize(&scenario);
        assert_eq!(st, ScenarioType::PhysicalReasoning);
    }

    #[test]
    fn high_individual_recognized_as_medical_ethics() {
        let scenario = ScenarioInput {
            id: "med".into(),
            description: "test".into(),
            domain: "MedicalEthics".into(),
            signals: vec![
                SignalInput {
                    frame: "Individual".into(),
                    value: 1,
                    phase: 0.5,
                    sensor: None,
                },
                SignalInput {
                    frame: "Individual".into(),
                    value: 0,
                    phase: 0.5,
                    sensor: None,
                },
            ],
            expected_behavior: String::new(),
            environmental_context: None,
        };
        let st = recognize(&scenario);
        assert_eq!(st, ScenarioType::MedicalEthics);
    }

    #[test]
    fn mixed_frames_recognized_as_value_conflict() {
        let scenario = ScenarioInput {
            id: "vc".into(),
            description: "test".into(),
            domain: "ValueJudgment".into(),
            signals: vec![
                SignalInput {
                    frame: "Science".into(),
                    value: 1,
                    phase: 0.5,
                    sensor: None,
                },
                SignalInput {
                    frame: "Individual".into(),
                    value: -1,
                    phase: 0.5,
                    sensor: None,
                },
                SignalInput {
                    frame: "Consensus".into(),
                    value: 0,
                    phase: 0.5,
                    sensor: None,
                },
            ],
            expected_behavior: String::new(),
            environmental_context: None,
        };
        let st = recognize(&scenario);
        assert_eq!(st, ScenarioType::ValueConflict);
    }

    #[test]
    fn recognize_with_score_returns_both() {
        let scenario = make_scenario("Science", 1, 0.9);
        let (st, score) = recognize_with_score(&scenario);
        assert_eq!(st, ScenarioType::PhysicalReasoning);
        assert!(score > 0.0);
    }

    #[test]
    fn feature_vector_from_empty_is_all_zeros() {
        let scenario = ScenarioInput {
            id: "empty".into(),
            description: "empty".into(),
            domain: "General".into(),
            signals: vec![],
            expected_behavior: String::new(),
            environmental_context: None,
        };
        let fv = FeatureVector::from_scenario(&scenario);
        assert_eq!(fv.science_ratio, 0.0);
        assert_eq!(fv.true_ratio, 0.0);
        assert_eq!(fv.mean_phase, 0.0);
    }

    #[test]
    fn cosine_similarity_identical_is_one() {
        let fv = FeatureVector {
            science_ratio: 0.5,
            individual_ratio: 0.3,
            consensus_ratio: 0.2,
            first_person_ratio: 0.0,
            true_ratio: 0.6,
            false_ratio: 0.2,
            hold_ratio: 0.2,
            mean_phase: 0.6,
            distinct_frames: 3.0,
            signal_count_norm: 0.3,
        };
        let proto = fv.as_slice();
        let sim = fv.cosine_similarity(&proto);
        assert!((sim - 1.0).abs() < 1e-9);
    }
}
