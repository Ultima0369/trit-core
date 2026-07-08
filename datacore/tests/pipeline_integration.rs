//! End-to-end pipeline integration tests.
//!
//! Validates the full data flow: RawSignal → normalize → timeseries → anomaly.

use chrono::Utc;
use datacore::{AnomalyConfig, AnomalyDetector, SignalNormalizer, TimeSeriesStore};
use dataforge::{DataCategory, GeoPoint, RawSignal};

/// Build a batch of synthetic RawSignals mimicking a 30-day climate data feed.
fn synthetic_climate_signals() -> Vec<RawSignal> {
    let t0 = Utc::now();
    let mut signals = Vec::new();
    // 30 days of CO2 readings with a steady trend + one spike
    for day in 0..30 {
        let ppm = 432.0 + (day as f64 * 0.1);
        let t = t0 + chrono::Duration::days(day);
        signals.push(RawSignal {
            id: format!("co2_{day}"),
            source_url: "https://test.example.com".into(),
            source_name: "TestClimate".into(),
            category: DataCategory::Climate,
            raw_content: format!("co2_ppm:{ppm:.2} day:{day}"),
            captured_at: t,
            data_period: Some(t.format("%Y-%m-%d").to_string()),
            location: Some(GeoPoint {
                lat: 19.5,
                lng: -155.6,
            }),
        });
    }
    // Add a spike on day 31
    signals.push(RawSignal {
        id: "co2_31_spike".into(),
        source_url: "https://test.example.com".into(),
        source_name: "TestClimate".into(),
        category: DataCategory::Climate,
        raw_content: "co2_ppm:550.00 day:31".into(),
        captured_at: t0 + chrono::Duration::days(31),
        data_period: Some(
            (t0 + chrono::Duration::days(31))
                .format("%Y-%m-%d")
                .to_string(),
        ),
        location: Some(GeoPoint {
            lat: 19.5,
            lng: -155.6,
        }),
    });
    signals
}

#[test]
fn pipeline_normalize_all_signals() {
    let raw = synthetic_climate_signals();
    let normalizer = SignalNormalizer::new();
    let normalized = normalizer.normalize_batch(&raw);
    // All 31 signals have numeric co2_ppm values
    assert_eq!(normalized.len(), raw.len());
    // Last one is the spike
    let spike = normalized.last().unwrap();
    let co2 = spike.values.iter().find(|v| v.name == "co2_ppm").unwrap();
    assert!((co2.value - 550.0).abs() < 0.01);
}

#[test]
fn pipeline_normalize_to_timeseries() {
    let raw = synthetic_climate_signals();
    let normalizer = SignalNormalizer::new();
    let normalized = normalizer.normalize_batch(&raw);

    let mut store = TimeSeriesStore::new();
    let inserted = store.insert_batch(&normalized);
    assert_eq!(inserted, 31); // one value per signal
    assert_eq!(store.len(), 31);

    // Query co2_ppm parameter
    let points = store.query_parameter("co2_ppm");
    assert_eq!(points.len(), 31);
}

#[test]
fn pipeline_detect_spike_anomaly() {
    let raw = synthetic_climate_signals();
    let normalizer = SignalNormalizer::new();
    let normalized = normalizer.normalize_batch(&raw);

    let mut store = TimeSeriesStore::new();
    store.insert_batch(&normalized);

    let detector = AnomalyDetector::new(AnomalyConfig {
        window_size: 30,
        threshold: 3.0,
    });
    let results = detector.score_parameter("co2_ppm", &store);

    // The last point (spike of 550 vs steady ~435) should be anomalous
    let spike_result = results.last().unwrap();
    assert!(
        spike_result.is_anomalous,
        "550 ppm spike should be anomalous"
    );
    assert!(spike_result.z_score.unwrap() > 3.0);

    // Earlier points should not be anomalous
    let early_anomalies = results.iter().take(30).filter(|r| r.is_anomalous).count();
    assert_eq!(
        early_anomalies, 0,
        "steady trend should produce no anomalies"
    );
}

#[test]
fn pipeline_multiple_parameters_with_location() {
    let t = Utc::now();
    let raw = vec![
        RawSignal {
            id: "multi_1".into(),
            source_url: "https://test.example.com".into(),
            source_name: "MultiTest".into(),
            category: DataCategory::Climate,
            raw_content: "t2m_c:25.50 precip_mm:3.20 solar_w_m2:180.50".into(),
            captured_at: t,
            data_period: None,
            location: Some(GeoPoint {
                lat: 35.0,
                lng: 139.0,
            }),
        },
        RawSignal {
            id: "multi_2".into(),
            source_url: "https://test.example.com".into(),
            source_name: "MultiTest".into(),
            category: DataCategory::Climate,
            raw_content: "t2m_c:26.00 precip_mm:0.00 solar_w_m2:200.00".into(),
            captured_at: t + chrono::Duration::hours(1),
            data_period: None,
            location: Some(GeoPoint {
                lat: 35.0,
                lng: 139.0,
            }),
        },
    ];

    let normalizer = SignalNormalizer::new();
    let normalized = normalizer.normalize_batch(&raw);
    assert_eq!(normalized.len(), 2);

    let mut store = TimeSeriesStore::new();
    let count = store.insert_batch(&normalized);
    assert_eq!(count, 6); // 3 params × 2 signals

    let params = store.parameters();
    assert_eq!(params.len(), 3);
    assert!(params.contains(&"t2m_c"));
    assert!(params.contains(&"precip_mm"));
    assert!(params.contains(&"solar_w_m2"));
}
