use aurora::pipeline::analysis::{self, SignalSpec};
use truncore::{Frame, Phase, TritValue, TritWord};

#[test]
fn run_analysis_from_percept_merges_signals() {
    let spec = SignalSpec {
        freq: 2.0,
        sample_rate: 100.0,
        duration_secs: 1.0,
        noise_std: 0.0,
    };

    let percept_signals = vec![TritWord::new(
        TritValue::Hold,
        Phase::new_clamped(0.5),
        Frame::Individual,
    )];

    let report =
        analysis::run_analysis_from_percept(&spec, 1.0, true, &[], &percept_signals).unwrap();

    assert_eq!(report.contact_count, 0);
    assert!(report.decision.input_signals.len() >= 3);
}

#[test]
fn run_analysis_from_percept_with_empty_percept() {
    let spec = SignalSpec {
        freq: 2.0,
        sample_rate: 100.0,
        duration_secs: 1.0,
        noise_std: 0.0,
    };

    let report = analysis::run_analysis_from_percept(&spec, 1.0, true, &[], &[]).unwrap();
    assert!(report.decision.input_signals.len() >= 2);
}

#[test]
fn existing_run_analysis_still_works() {
    let spec = SignalSpec {
        freq: 2.0,
        sample_rate: 100.0,
        duration_secs: 1.0,
        noise_std: 0.0,
    };

    let report = analysis::run_analysis(&spec, 1.0, true, &[]).unwrap();
    assert!(report.decision.input_signals.len() >= 2);
}
