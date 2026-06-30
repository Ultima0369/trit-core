//! Experience recorder — ring buffer of feedback outcomes.
//!
//! Stores a fixed-size window of [`ExperienceRecord`] entries for
//! pattern analysis. Oldest entries are silently evicted when the
//! buffer is full.

use std::collections::VecDeque;

/// A single recorded experience from the feedback loop.
#[derive(Debug, Clone)]
pub struct ExperienceRecord {
    /// ID of the scenario that produced this experience.
    pub scenario_id: String,
    /// Final value code of the decision (-1, 0, 1).
    pub result_value: i8,
    /// Final phase of the decision.
    pub result_phase: f64,
    /// Whether the decision matched the proxy prediction.
    pub matched: bool,
    /// Deviation delta (0.0 for matched, >0.0 for deviated).
    pub deviation_delta: f64,
}

/// Fixed-size ring buffer of experience records.
///
/// # Window eviction
///
/// When the buffer exceeds `window_size`, the oldest entry is silently
/// dropped. This prevents unbounded memory growth.
#[derive(Debug, Clone)]
pub struct ExperienceRecorder {
    entries: VecDeque<ExperienceRecord>,
    window_size: usize,
}

impl ExperienceRecorder {
    /// Create a new experience recorder with the given window size.
    pub fn new(window_size: usize) -> Self {
        ExperienceRecorder {
            entries: VecDeque::with_capacity(window_size),
            window_size,
        }
    }

    /// Record a new experience. Evicts the oldest if the window is full.
    pub fn record(&mut self, entry: ExperienceRecord) {
        if self.entries.len() >= self.window_size {
            self.entries.pop_front();
        }
        self.entries.push_back(entry);
    }

    /// Number of entries currently stored.
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Returns true if no entries are stored.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// The window size (maximum entries).
    pub fn window_size(&self) -> usize {
        self.window_size
    }

    /// Fraction of entries that were matched. Returns 0.0 if empty.
    pub fn match_rate(&self) -> f64 {
        if self.entries.is_empty() {
            return 0.0;
        }
        self.entries.iter().filter(|e| e.matched).count() as f64 / self.entries.len() as f64
    }

    /// Average deviation delta across all entries. Returns 0.0 if empty.
    pub fn average_delta(&self) -> f64 {
        if self.entries.is_empty() {
            return 0.0;
        }
        self.entries.iter().map(|e| e.deviation_delta).sum::<f64>() / self.entries.len() as f64
    }

    /// Iterate over entries from oldest to newest.
    pub fn iter(&self) -> impl Iterator<Item = &ExperienceRecord> {
        self.entries.iter()
    }

    /// Clear all entries.
    pub fn clear(&mut self) {
        self.entries.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn record(matched: bool, delta: f64) -> ExperienceRecord {
        ExperienceRecord {
            scenario_id: "test".into(),
            result_value: 1,
            result_phase: 0.9,
            matched,
            deviation_delta: delta,
        }
    }

    #[test]
    fn new_recorder_is_empty() {
        let er = ExperienceRecorder::new(32);
        assert!(er.is_empty());
        assert_eq!(er.len(), 0);
    }

    #[test]
    fn record_adds_entry() {
        let mut er = ExperienceRecorder::new(32);
        er.record(record(true, 0.0));
        assert_eq!(er.len(), 1);
    }

    #[test]
    fn window_eviction_drops_oldest() {
        let mut er = ExperienceRecorder::new(3);
        er.record(record(true, 0.0));
        er.record(record(false, 0.3));
        er.record(record(false, 0.5));
        er.record(record(true, 0.1)); // first entry should be evicted
        assert_eq!(er.len(), 3);
        // Oldest should now be the second entry (delta=0.3)
        let first = er.iter().next().unwrap();
        assert_float_eq!(first.deviation_delta, 0.3);
    }

    #[test]
    fn match_rate_all_matched() {
        let mut er = ExperienceRecorder::new(32);
        er.record(record(true, 0.0));
        er.record(record(true, 0.0));
        assert_float_eq!(er.match_rate(), 1.0);
    }

    #[test]
    fn match_rate_mixed() {
        let mut er = ExperienceRecorder::new(32);
        er.record(record(true, 0.0));
        er.record(record(false, 0.4));
        assert_float_eq!(er.match_rate(), 0.5);
    }

    #[test]
    fn match_rate_empty_is_zero() {
        let er = ExperienceRecorder::new(32);
        assert_float_eq!(er.match_rate(), 0.0);
    }

    #[test]
    fn average_delta() {
        let mut er = ExperienceRecorder::new(32);
        er.record(record(true, 0.0));
        er.record(record(false, 0.4));
        assert_float_eq!(er.average_delta(), 0.2);
    }

    #[test]
    fn average_delta_empty_is_zero() {
        let er = ExperienceRecorder::new(32);
        assert_float_eq!(er.average_delta(), 0.0);
    }

    #[test]
    fn clear_empties() {
        let mut er = ExperienceRecorder::new(32);
        er.record(record(true, 0.0));
        er.clear();
        assert!(er.is_empty());
    }

    #[test]
    fn window_size_is_preserved() {
        let er = ExperienceRecorder::new(16);
        assert_eq!(er.window_size(), 16);
    }
}
