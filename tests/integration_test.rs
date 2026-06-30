use trit_core::core::frame::Frame;
use trit_core::core::phase::{Commitment, Phase};
use trit_core::core::value::TritValue;
use trit_core::core::word::TritWord;
use trit_core::core::TernaryAlgebra;
use trit_core::meta::{ArbitrationResult, Domain, MetaMonitor, ResolutionPolicy};

#[cfg(test)]
mod trit_tests {
    use super::*;

    #[test]
    fn tand_same_frame_true_true() {
        let a = TritWord::tru(Frame::Science);
        let b = TritWord::tru(Frame::Science);
        let (res, int) = TernaryAlgebra::t_and(&a, &b);
        assert_eq!(res.value(), TritValue::True);
        assert!(int.is_none());
    }

    #[test]
    fn tand_cross_frame_conflict() {
        let a = TritWord::tru(Frame::Science);
        let b = TritWord::fals(Frame::Individual);
        let (res, int) = TernaryAlgebra::t_and(&a, &b);
        assert_eq!(res.value(), TritValue::Hold);
        assert_eq!(res.frame(), Frame::Meta);
        assert!(int.is_some());
    }

    #[test]
    fn tnot_flips_phase() {
        let a = TritWord::new(TritValue::True, Phase::new(0.8).unwrap(), Frame::Science);
        let res = TernaryAlgebra::t_not(&a);
        assert_eq!(res.value(), TritValue::False);
        assert!((res.phase().inner() - 0.2).abs() < f64::EPSILON);
    }

    #[test]
    fn phase_neutral_commitment() {
        let p = Phase::new(0.5).unwrap();
        assert_eq!(p.commitment(), Commitment::Neutral);
    }

    #[test]
    fn hot_path_panics_on_mismatched_frames() {
        let a = TritWord::tru(Frame::Science);
        let b = TritWord::tru(Frame::Individual);
        let result = std::panic::catch_unwind(|| TernaryAlgebra::t_and_hot(&a, &b));
        assert!(result.is_err());
    }
}

#[cfg(test)]
mod meta_tests {
    use super::*;

    #[test]
    fn medical_ethics_preserves_individual() {
        let policy = ResolutionPolicy::new(Domain::MedicalEthics);
        let inputs = vec![
            TritWord::tru(Frame::Science),
            TritWord::fals(Frame::Individual),
        ];
        let result = policy.arbitrate(&inputs).unwrap();
        match result {
            ArbitrationResult::Preserve(w) => {
                assert_eq!(w.frame(), Frame::Individual);
            }
            _ => panic!("Expected Preserve(Individual), got {:?}", result),
        }
    }

    #[test]
    fn value_judgment_always_hold() {
        let policy = ResolutionPolicy::new(Domain::ValueJudgment);
        let inputs = vec![
            TritWord::tru(Frame::Science),
            TritWord::fals(Frame::Individual),
        ];
        let result = policy.arbitrate(&inputs).unwrap();
        assert_eq!(result, ArbitrationResult::Hold);
    }

    #[test]
    fn meta_monitor_can_record_and_drain() {
        let mut monitor = MetaMonitor::new();
        assert_eq!(monitor.drain_log().len(), 0);
    }
}

#[cfg(test)]
mod scenario_tests {
    use super::*;

    /// Simulates the sandbox pipeline: TAND cascade + policy arbitration.
    fn run_pipeline(
        domain: Domain,
        signals: Vec<(Frame, i8, f64)>,
    ) -> (TritValue, Vec<String>, ArbitrationResult) {
        let trits: Vec<TritWord> = signals
            .iter()
            .map(|(frame, val, phase)| {
                TritWord::try_new(TritValue::from(*val), *phase, &format!("{}", frame)).unwrap()
            })
            .collect();

        let policy = ResolutionPolicy::new(domain);
        let mut monitor = MetaMonitor::new();

        let mut current = trits[0];
        let mut interrupts: Vec<String> = vec![];

        for next in &trits[1..] {
            let (result, maybe_int) = TernaryAlgebra::t_and(&current, next);
            if let Some(int) = maybe_int {
                monitor.record(int.clone());
                interrupts.push(format!("{:?}: {}", int.conflict, int.reason));
            }
            current = result;
        }

        let policy_result = policy.arbitrate(&trits).unwrap();
        let final_word = match &policy_result {
            ArbitrationResult::Commit(w) | ArbitrationResult::Preserve(w) => *w,
            _ => current,
        };

        (final_word.value(), interrupts, policy_result)
    }

    #[test]
    fn scenario_medical_conflict_preserves_individual() {
        let (value, interrupts, policy_action) = run_pipeline(
            Domain::MedicalEthics,
            vec![(Frame::Science, 1, 0.8), (Frame::Individual, -1, 0.2)],
        );
        assert_eq!(value, TritValue::False);
        assert!(!interrupts.is_empty());
        assert!(matches!(policy_action, ArbitrationResult::Preserve(_)));
    }

    #[test]
    fn scenario_career_value_conflict_holds() {
        let (value, interrupts, policy_action) = run_pipeline(
            Domain::ValueJudgment,
            vec![(Frame::Individual, -1, 0.3), (Frame::Consensus, 1, 0.7)],
        );
        assert_eq!(value, TritValue::Hold);
        assert!(!interrupts.is_empty());
        assert_eq!(policy_action, ArbitrationResult::Hold);
    }

    #[test]
    fn scenario_bridge_safety_commits_false() {
        let (value, interrupts, policy_action) = run_pipeline(
            Domain::Engineering,
            vec![(Frame::Individual, 1, 0.6), (Frame::Science, -1, 0.4)],
        );
        assert_eq!(value, TritValue::False);
        assert!(!interrupts.is_empty());
        assert!(matches!(policy_action, ArbitrationResult::Commit(_)));
    }

    #[test]
    fn scenario_general_negotiation_commits_first() {
        let (value, interrupts, policy_action) = run_pipeline(
            Domain::General,
            vec![
                (Frame::Science, 1, 0.7),
                (Frame::Science, 1, 0.8),
                (Frame::Science, -1, 0.3),
            ],
        );
        assert_eq!(value, TritValue::True);
        assert!(interrupts.is_empty());
        assert!(matches!(policy_action, ArbitrationResult::Commit(_)));
    }

    #[test]
    fn scenario_physical_domain_science_priority() {
        let (value, interrupts, policy_action) = run_pipeline(
            Domain::Physical,
            vec![(Frame::Consensus, 1, 0.6), (Frame::Science, -1, 0.3)],
        );
        assert_eq!(value, TritValue::False);
        assert!(!interrupts.is_empty());
        assert!(matches!(policy_action, ArbitrationResult::Commit(_)));
    }

    #[test]
    fn scenario_medical_autonomy_holds() {
        let (value, interrupts, policy_action) = run_pipeline(
            Domain::MedicalEthics,
            vec![(Frame::Science, -1, 0.25), (Frame::Individual, 1, 0.85)],
        );
        assert_eq!(value, TritValue::True);
        assert!(!interrupts.is_empty());
        assert!(matches!(policy_action, ArbitrationResult::Preserve(_)));
    }

    #[test]
    fn scenario_medical_mandate_conflict() {
        let (value, interrupts, policy_action) = run_pipeline(
            Domain::MedicalEthics,
            vec![
                (Frame::Science, 1, 0.75),
                (Frame::Consensus, 1, 0.7),
                (Frame::Individual, -1, 0.35),
            ],
        );
        assert_eq!(value, TritValue::False);
        assert!(!interrupts.is_empty());
        assert!(matches!(policy_action, ArbitrationResult::Preserve(_)));
    }

    #[test]
    fn scenario_crane_overload_commits_false() {
        let (value, interrupts, policy_action) = run_pipeline(
            Domain::Physical,
            vec![(Frame::Individual, 1, 0.7), (Frame::Science, -1, 0.45)],
        );
        assert_eq!(value, TritValue::False);
        assert!(!interrupts.is_empty());
        assert!(matches!(policy_action, ArbitrationResult::Commit(_)));
    }

    #[test]
    fn scenario_runway_commits_false() {
        let (value, interrupts, policy_action) = run_pipeline(
            Domain::Physical,
            vec![(Frame::Individual, 1, 0.55), (Frame::Science, -1, 0.85)],
        );
        assert_eq!(value, TritValue::False);
        assert!(!interrupts.is_empty());
        assert!(matches!(policy_action, ArbitrationResult::Commit(_)));
    }

    #[test]
    fn scenario_material_tradeoff_commits_false() {
        let (value, interrupts, policy_action) = run_pipeline(
            Domain::Engineering,
            vec![
                (Frame::Consensus, 1, 0.6),
                (Frame::Individual, -1, 0.75),
                (Frame::Science, -1, 0.55),
            ],
        );
        assert_eq!(value, TritValue::False);
        assert!(!interrupts.is_empty());
        assert!(matches!(policy_action, ArbitrationResult::Commit(_)));
    }

    #[test]
    fn scenario_bridge_retrofit_commits_false() {
        let (value, interrupts, policy_action) = run_pipeline(
            Domain::Engineering,
            vec![(Frame::Consensus, 1, 0.5), (Frame::Science, -1, 0.9)],
        );
        assert_eq!(value, TritValue::False);
        assert!(!interrupts.is_empty());
        assert!(matches!(policy_action, ArbitrationResult::Commit(_)));
    }

    #[test]
    fn scenario_general_multi_domain_negotiates() {
        let (value, interrupts, policy_action) = run_pipeline(
            Domain::General,
            vec![
                (Frame::Science, 1, 0.8),
                (Frame::Consensus, -1, 0.35),
                (Frame::Individual, 1, 0.9),
            ],
        );
        assert_eq!(value, TritValue::Hold);
        assert!(!interrupts.is_empty());
        assert_eq!(policy_action, ArbitrationResult::Negotiate);
    }
}
