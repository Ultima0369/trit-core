//! Module registry: registered modules, mount/unmount lifecycle.
//!
//! The registry maintains the set of currently mounted modules and
//! provides operations for computing mount/unmount diffs against a
//! target set for a new scenario.

use std::collections::HashSet;

use super::UnmountReason;

// ── Module ID ──────────────────────────────────────────────────────

/// Unique identifier for a cognitive module.
///
/// Each module in the adapter pool has a fixed `ModuleId`. The Hook
/// Manager uses these IDs to track which modules are currently mounted.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ModuleId {
    LogicalConsistencyCheck,
    CognitiveDeconstruction,
    ConflictSuspension,
    EngineeringArchitecture,
    ReflexiveAudit,
    AdaptiveIteration,
    EcologicalAssessment,
    AttentionScheduler,
    CouplingAdapter,
    ResponsePatternCache,
}

impl ModuleId {
    /// Human-readable module name.
    pub fn as_str(&self) -> &'static str {
        match self {
            ModuleId::LogicalConsistencyCheck => "critical_thinking",
            ModuleId::CognitiveDeconstruction => "cognitive_deconstruction",
            ModuleId::ConflictSuspension => "conflict_suspension",
            ModuleId::EngineeringArchitecture => "engineering_architecture",
            ModuleId::ReflexiveAudit => "reflexive_audit",
            ModuleId::AdaptiveIteration => "adaptive_iteration",
            ModuleId::EcologicalAssessment => "ecological_assessment",
            ModuleId::AttentionScheduler => "attention_scheduler",
            ModuleId::CouplingAdapter => "coupling_adapter",
            ModuleId::ResponsePatternCache => "self_knowledge",
        }
    }
}

impl std::fmt::Display for ModuleId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

// ── Module state ───────────────────────────────────────────────────

/// Lifecycle state of a mounted module.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ModuleState {
    /// Module is idle, waiting for input.
    #[default]
    Idle,
    /// Module is actively processing.
    Processing,
    /// Module completed its work for this cycle.
    Completed,
    /// Module encountered an error.
    Error,
}

// ── Module entry ───────────────────────────────────────────────────

/// Entry in the module registry representing a mounted module.
#[derive(Debug, Clone)]
pub struct ModuleEntry {
    /// Module identifier.
    pub id: ModuleId,
    /// Current lifecycle state.
    pub state: ModuleState,
    /// How many decision cycles this module has been mounted.
    pub cycles_mounted: usize,
}

impl ModuleEntry {
    /// Create a new entry for a freshly mounted module.
    pub fn new(id: ModuleId) -> Self {
        ModuleEntry {
            id,
            state: ModuleState::Idle,
            cycles_mounted: 1,
        }
    }

    /// Increment the mount cycle counter.
    pub fn increment_cycle(&mut self) {
        self.cycles_mounted = self.cycles_mounted.saturating_add(1);
    }
}

// ── Module registry ────────────────────────────────────────────────

/// Tracks currently mounted modules and their lifecycle states.
#[derive(Debug, Clone, Default)]
pub struct ModuleRegistry {
    /// Currently mounted modules.
    entries: Vec<ModuleEntry>,
    /// Log of recent mount/unmount events for auditability.
    event_log: Vec<RegistryEvent>,
}

/// A mount or unmount event.
#[derive(Debug, Clone, PartialEq)]
pub struct RegistryEvent {
    /// Module that was mounted or unmounted.
    pub module_id: ModuleId,
    /// Whether this was a mount or unmount.
    pub action: RegistryAction,
    /// Reason for unmount (only populated for unmount actions).
    pub reason: Option<UnmountReason>,
    /// Wall-clock timestamp.
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RegistryAction {
    Mount,
    Unmount,
}

impl ModuleRegistry {
    /// Create an empty registry.
    pub fn new() -> Self {
        ModuleRegistry {
            entries: Vec::new(),
            event_log: Vec::new(),
        }
    }

    /// Returns the set of currently mounted module IDs.
    pub fn mounted_ids(&self) -> HashSet<ModuleId> {
        self.entries.iter().map(|e| e.id).collect()
    }

    /// Returns the current set of mounted modules.
    pub fn entries(&self) -> &[ModuleEntry] {
        &self.entries
    }

    /// Returns the event log.
    pub fn event_log(&self) -> &[RegistryEvent] {
        &self.event_log
    }

    /// Returns true if the given module is mounted.
    pub fn is_mounted(&self, id: ModuleId) -> bool {
        self.entries.iter().any(|e| e.id == id)
    }

    /// Compute the modules that need to be unmounted (current but not in target).
    pub fn modules_to_unmount(&self, target: &HashSet<ModuleId>) -> Vec<ModuleId> {
        self.entries
            .iter()
            .map(|e| e.id)
            .filter(|id| !target.contains(id))
            .collect()
    }

    /// Compute the modules that need to be mounted (in target but not current).
    pub fn modules_to_mount(&self, target: &HashSet<ModuleId>) -> Vec<ModuleId> {
        let current = self.mounted_ids();
        target
            .iter()
            .filter(|id| !current.contains(id))
            .copied()
            .collect()
    }

    /// Mount a module (idempotent — returns false if already mounted).
    pub fn mount(&mut self, id: ModuleId) -> bool {
        if self.is_mounted(id) {
            return false;
        }
        self.entries.push(ModuleEntry::new(id));
        self.event_log.push(RegistryEvent {
            module_id: id,
            action: RegistryAction::Mount,
            reason: None,
            timestamp: chrono::Utc::now(),
        });
        true
    }

    /// Unmount a module (idempotent — returns false if not mounted).
    pub fn unmount(&mut self, id: ModuleId, reason: UnmountReason) -> bool {
        let idx = match self.entries.iter().position(|e| e.id == id) {
            Some(i) => i,
            None => return false,
        };
        self.entries.remove(idx);
        self.event_log.push(RegistryEvent {
            module_id: id,
            action: RegistryAction::Unmount,
            reason: Some(reason),
            timestamp: chrono::Utc::now(),
        });
        true
    }

    /// Apply a mount/unmount diff:
    /// - Mount all modules in `to_mount`
    /// - Unmount all modules in `to_unmount` with the given reason
    pub fn apply_diff(
        &mut self,
        to_mount: &[ModuleId],
        to_unmount: &[ModuleId],
        unmount_reason: UnmountReason,
    ) {
        for id in to_unmount {
            self.unmount(*id, unmount_reason);
        }
        for id in to_mount {
            self.mount(*id);
        }
    }

    /// Increment the cycle counter for all mounted modules.
    pub fn tick_all(&mut self) {
        for entry in &mut self.entries {
            entry.increment_cycle();
        }
    }

    /// Number of currently mounted modules.
    pub fn count(&self) -> usize {
        self.entries.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_registry_is_empty() {
        let reg = ModuleRegistry::new();
        assert_eq!(reg.count(), 0);
        assert!(reg.mounted_ids().is_empty());
    }

    #[test]
    fn mount_adds_module() {
        let mut reg = ModuleRegistry::new();
        assert!(reg.mount(ModuleId::ReflexiveAudit));
        assert!(reg.is_mounted(ModuleId::ReflexiveAudit));
        assert_eq!(reg.count(), 1);
    }

    #[test]
    fn mount_is_idempotent() {
        let mut reg = ModuleRegistry::new();
        assert!(reg.mount(ModuleId::ReflexiveAudit));
        assert!(!reg.mount(ModuleId::ReflexiveAudit)); // already mounted
        assert_eq!(reg.count(), 1);
    }

    #[test]
    fn unmount_removes_module() {
        let mut reg = ModuleRegistry::new();
        reg.mount(ModuleId::ReflexiveAudit);
        assert!(reg.unmount(ModuleId::ReflexiveAudit, UnmountReason::Completed));
        assert!(!reg.is_mounted(ModuleId::ReflexiveAudit));
        assert_eq!(reg.count(), 0);
    }

    #[test]
    fn unmount_is_idempotent() {
        let mut reg = ModuleRegistry::new();
        assert!(!reg.unmount(ModuleId::ReflexiveAudit, UnmountReason::Completed));
    }

    #[test]
    fn compute_diff_correctly() {
        let mut reg = ModuleRegistry::new();
        reg.mount(ModuleId::ReflexiveAudit);
        reg.mount(ModuleId::AttentionScheduler);

        let target: HashSet<ModuleId> =
            [ModuleId::ResponsePatternCache, ModuleId::AttentionScheduler].into();

        let to_unmount = reg.modules_to_unmount(&target);
        let to_mount = reg.modules_to_mount(&target);

        assert_eq!(to_unmount, vec![ModuleId::ReflexiveAudit]);
        assert_eq!(to_mount, vec![ModuleId::ResponsePatternCache]);
    }

    #[test]
    fn apply_diff_transitions_correctly() {
        let mut reg = ModuleRegistry::new();
        reg.mount(ModuleId::ReflexiveAudit);
        reg.mount(ModuleId::AttentionScheduler);

        reg.apply_diff(
            &[ModuleId::ResponsePatternCache],
            &[ModuleId::ReflexiveAudit],
            UnmountReason::Completed,
        );

        assert!(!reg.is_mounted(ModuleId::ReflexiveAudit));
        assert!(reg.is_mounted(ModuleId::AttentionScheduler));
        assert!(reg.is_mounted(ModuleId::ResponsePatternCache));
        assert_eq!(reg.count(), 2);
    }

    #[test]
    fn tick_all_increments_cycles() {
        let mut reg = ModuleRegistry::new();
        reg.mount(ModuleId::ReflexiveAudit);
        reg.mount(ModuleId::ResponsePatternCache);
        reg.tick_all();
        assert_eq!(reg.entries()[0].cycles_mounted, 2);
        assert_eq!(reg.entries()[1].cycles_mounted, 2);
    }

    #[test]
    fn event_log_records_all_actions() {
        let mut reg = ModuleRegistry::new();
        reg.mount(ModuleId::ReflexiveAudit);
        reg.unmount(ModuleId::ReflexiveAudit, UnmountReason::Completed);
        assert_eq!(reg.event_log().len(), 2);

        let mount_event = &reg.event_log()[0];
        assert!(matches!(mount_event.action, RegistryAction::Mount));
        assert_eq!(mount_event.module_id, ModuleId::ReflexiveAudit);

        let unmount_event = &reg.event_log()[1];
        assert!(matches!(unmount_event.action, RegistryAction::Unmount));
        assert_eq!(unmount_event.reason, Some(UnmountReason::Completed));
    }

    #[test]
    fn all_module_ids_have_labels() {
        let ids = [
            ModuleId::LogicalConsistencyCheck,
            ModuleId::CognitiveDeconstruction,
            ModuleId::ConflictSuspension,
            ModuleId::EngineeringArchitecture,
            ModuleId::ReflexiveAudit,
            ModuleId::AdaptiveIteration,
            ModuleId::EcologicalAssessment,
            ModuleId::AttentionScheduler,
            ModuleId::CouplingAdapter,
            ModuleId::ResponsePatternCache,
        ];
        for id in ids {
            assert!(!id.as_str().is_empty());
        }
    }
}
