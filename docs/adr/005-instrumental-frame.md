# ADR-005: Instrumental Frame for Sensor/Instrument Data

## Status
Accepted

## Context

The existing Frame system had 12 variants covering theoretical knowledge (`Science`), subjective experience (`Individual`, `FirstPerson`, `Embodied`), social consensus (`Consensus`), ecological observation (`Environmental`, `GeoEco`), and relational/role-based perspectives. However, none of these frames cleanly captured **direct instrument readings** — raw sensor data from thermometers, CO₂ monitors, spectrometers, GPS trackers.

The distinction matters because instrument data is neither:

- **Science**: Science is theory-laden. A CO₂ measurement of 432.34 ppm is not a theory about climate change — it's a sensor reading. Theories can be debated; instrument readings are what the instrument shows right now.
- **Individual**: An instrument is not a person. It has no subjective experience, no values, no bodily constraints.
- **Consensus**: A temperature anomaly is not social agreement. If every human agrees the temperature is 18°C but the thermometer reads 22°C, the instrument reading stands.

Without an `Instrumental` frame, data from the `dataforge` crate (NOAA CO₂, Open-Meteo temperature, GBIF species counts, UCDP conflict coordinates) would need to be squeezed into `Science` or `Environmental` frames — both of which carry semantic baggage that doesn't apply.

## Decision

Add a 13th Frame variant: `Instrumental`.

```rust
/// Instrument measurement — direct sensor/instrument reading
/// This is not "scientific consensus" (which may be overturned),
/// but "the instrument currently reads X"
Instrumental,
```

### Arbitration priority

| Domain | Instrumental priority |
|--------|----------------------|
| `Climate` | **Priority 1** — `Instrumental` frames dominate. Multiple Instrumental sources that conflict → `Hold` (requires investigation, not averaging). |
| `Environmental` | `Instrumental` ≥ `Environmental`, below `Science`. |
| `Physical` | `Instrumental` below `Science` (theory still dominates in hard sciences). |
| `Engineering` | `Instrumental` below `Engineering` frame (design constraints override raw readings). |
| All others | `Instrumental` at same priority as `Science` (defer to domain-specific rules). |

### DataCategory → Frame mapping (prism degradation path)

When `PrismEngine` degrades to structured parsing (no LLM available):

| `DataCategory` | Default Frame |
|----------------|---------------|
| `Climate` | `Instrumental` |
| `Ecology` | `Instrumental` |
| `ScientificResearch` | `Science` |
| `Geopolitical` | `Consensus` |
| `Other` | `Individual` |

### Threshold-based value inference (structured degradation)

```rust
fn threshold_for(key: &str) -> Option<(f64, TritValue)> {
    match key {
        "co2_ppm"          => Some((420.0, TritValue::False)),  // Paris Agreement threshold
        "temperature_anomaly_c" => Some((1.5, TritValue::False)),
        "deaths"           => Some((0.0, TritValue::False)),
        _ => None,
    }
}
```

These thresholds encode policy judgments (Paris Agreement targets) and are explicitly NOT scientific claims — they represent the system's conservative posture: when an environmental metric exceeds a widely-recognized threshold, the default signal is `False` (alert), not `True` (all-clear).

## Consequences

### Positive
- **Clean category separation**: No more "is this `Science` or `Environmental`?" for sensor data.
- **Instrument priority in Climate domain**: The system correctly prioritizes direct measurements over climate models when they conflict.
- **Explicit policy thresholds**: Documented, inspectable threshold logic rather than implicit magic numbers.
- **Non-human data has a home**: GBIF species occurrence counts, UCDP fatality estimates, arXiv paper metadata — all have appropriate frames.

### Negative
- **Frame count grows to 13**: Each new frame adds complexity to arbitration logic.
- **Thresholds are static**: 1.5°C and 420 ppm are hardcoded. If these targets change (e.g., 1.5°C becomes unrealistic), the code must be updated.
- **Instrument calibration not modeled**: The frame treats instrument readings as ground truth — real instruments drift, have error margins, and need calibration. Future ADRs may need `InstrumentError` metadata.

## See Also
- ADR-003: Domain-Based Conflict Resolution
- ADR-006: Cognitive Offload Protocol (uses `Instrumental` frame for `MissingVariable` suggestions)
- `src/core/frame.rs`: Frame enum definition
- `aurora/src/percept/prism.rs`: PrismEngine degradation logic
- `dataforge/src/types.rs`: DataCategory enum
