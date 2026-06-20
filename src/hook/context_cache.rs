//! Context cache: ephemeral state for the current scenario.
//!
//! The context cache holds scenario-specific temporary state that
//! modules can read but not mutate. It is cleared on scenario
//! transitions (hard unmount).

use super::ScenarioType;
use crate::anchor::AnchorReport;
use std::collections::HashMap;

/// Key-value cache for scenario-specific ephemeral state.
///
/// Modules can store lightweight context here (e.g., detected patterns,
/// intermediate computations) for cross-module visibility. This is
/// intentionally NOT a general-purpose store — values are strings
/// to keep the interface simple and auditable.
#[derive(Debug, Clone, Default)]
pub struct ContextCache {
    /// Scenario this cache belongs to.
    scenario: Option<ScenarioType>,
    /// Arbitrary key-value pairs.
    entries: HashMap<String, String>,
    /// The most recent anchor report (if any).
    last_anchor_report: Option<AnchorReport>,
    /// Scene change counter — incremented on each scenario transition.
    transition_count: u64,
}

impl ContextCache {
    /// Create an empty cache.
    pub fn new() -> Self {
        ContextCache {
            scenario: None,
            entries: HashMap::new(),
            last_anchor_report: None,
            transition_count: 0,
        }
    }

    /// Returns the current scenario type, if set.
    pub fn scenario(&self) -> Option<ScenarioType> {
        self.scenario
    }

    /// Returns the number of scenario transitions so far.
    pub fn transition_count(&self) -> u64 {
        self.transition_count
    }

    /// Returns the last anchor report.
    pub fn last_anchor_report(&self) -> Option<&AnchorReport> {
        self.last_anchor_report.as_ref()
    }

    /// Set the anchor report for the current iteration.
    pub fn set_anchor_report(&mut self, report: AnchorReport) {
        self.last_anchor_report = Some(report);
    }

    /// Read a cached value.
    pub fn get(&self, key: &str) -> Option<&str> {
        self.entries.get(key).map(|s| s.as_str())
    }

    /// Write a cached value.
    pub fn set(&mut self, key: impl Into<String>, value: impl Into<String>) {
        self.entries.insert(key.into(), value.into());
    }

    /// Remove a cached value.
    pub fn remove(&mut self, key: &str) {
        self.entries.remove(key);
    }

    /// Returns true if the cache has any entries.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Number of cached entries.
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Detect a scenario transition.
    ///
    /// If `new_scenario` differs from the cached scenario, clear the
    /// cache, set the new scenario, and increment the transition counter.
    /// Returns true if a transition occurred.
    pub fn detect_transition(&mut self, new_scenario: ScenarioType) -> bool {
        if self.scenario == Some(new_scenario) {
            return false;
        }
        // Hard clear: full context reset on scenario change.
        self.entries.clear();
        self.last_anchor_report = None;
        self.scenario = Some(new_scenario);
        self.transition_count = self.transition_count.saturating_add(1);
        true
    }

    /// Clear all cached state (hard reset).
    pub fn clear(&mut self) {
        self.entries.clear();
        self.last_anchor_report = None;
        self.scenario = None;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_cache_is_empty() {
        let cache = ContextCache::new();
        assert!(cache.is_empty());
        assert_eq!(cache.len(), 0);
        assert_eq!(cache.scenario(), None);
        assert_eq!(cache.transition_count(), 0);
    }

    #[test]
    fn set_and_get() {
        let mut cache = ContextCache::new();
        cache.set("key1", "value1");
        assert_eq!(cache.get("key1"), Some("value1"));
        assert_eq!(cache.len(), 1);
    }

    #[test]
    fn remove_entry() {
        let mut cache = ContextCache::new();
        cache.set("key1", "value1");
        cache.remove("key1");
        assert_eq!(cache.get("key1"), None);
    }

    #[test]
    fn detect_transition_clears_cache() {
        let mut cache = ContextCache::new();
        cache.set("key1", "value1");
        cache.set_anchor_report(crate::anchor::AnchorReport::clean());

        let changed = cache.detect_transition(ScenarioType::MedicalEthics);
        assert!(changed);
        assert!(cache.is_empty());
        assert_eq!(cache.scenario(), Some(ScenarioType::MedicalEthics));
        assert_eq!(cache.transition_count(), 1);
        assert!(cache.last_anchor_report().is_none());
    }

    #[test]
    fn same_scenario_is_not_a_transition() {
        let mut cache = ContextCache::new();
        cache.detect_transition(ScenarioType::MedicalEthics);
        cache.set("key1", "value1");

        let changed = cache.detect_transition(ScenarioType::MedicalEthics);
        assert!(!changed);
        assert_eq!(cache.get("key1"), Some("value1")); // preserved
    }

    #[test]
    fn multiple_transitions_increment_counter() {
        let mut cache = ContextCache::new();
        cache.detect_transition(ScenarioType::MedicalEthics);
        cache.detect_transition(ScenarioType::PhysicalReasoning);
        cache.detect_transition(ScenarioType::General);
        assert_eq!(cache.transition_count(), 3);
    }

    #[test]
    fn clear_resets_everything() {
        let mut cache = ContextCache::new();
        cache.set("key1", "value1");
        cache.set_anchor_report(crate::anchor::AnchorReport::clean());
        cache.detect_transition(ScenarioType::MedicalEthics);

        cache.clear();
        assert!(cache.is_empty());
        assert_eq!(cache.scenario(), None);
        assert_eq!(cache.transition_count(), 1); // counter NOT reset by clear
    }
}
