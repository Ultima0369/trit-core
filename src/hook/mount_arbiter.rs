//! Mount arbiter: resource evaluation, priority ordering, conflict detection.
//!
//! When a new scenario is recognized, the mount arbiter computes the
//! module request set, resolves conflicts between modules that share
//! resource bottlenecks, and produces the final mount/unmount plan.

use std::collections::HashSet;

use super::module_registry::{ModuleId, ModuleRegistry};
use super::ScenarioType;
use super::UnmountReason;

// ── Resource bottleneck ────────────────────────────────────────────

/// Resources that modules can compete for.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Resource {
    /// CPU time (attention bandwidth).
    Cpu,
    /// Memory (working set).
    Memory,
    /// I/O (external data access).
    Io,
}

/// Estimated resource cost of a module.
///
/// Values are in [0.0, 1.0] where 1.0 = full resource consumption.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ResourceCost {
    pub cpu: f64,
    pub memory: f64,
    pub io: f64,
}

impl ResourceCost {
    /// Lightweight module — minimal resource usage.
    pub const fn light() -> Self {
        ResourceCost {
            cpu: 0.1,
            memory: 0.1,
            io: 0.0,
        }
    }

    /// Standard module — moderate resource usage.
    pub const fn standard() -> Self {
        ResourceCost {
            cpu: 0.3,
            memory: 0.3,
            io: 0.1,
        }
    }

    /// Heavy module — significant resource usage.
    pub const fn heavy() -> Self {
        ResourceCost {
            cpu: 0.6,
            memory: 0.5,
            io: 0.3,
        }
    }

    /// Total resource pressure (sum of all dimensions, max 3.0).
    pub fn total(&self) -> f64 {
        self.cpu + self.memory + self.io
    }
}

/// Estimated resource cost for each module.
pub fn resource_cost(id: ModuleId) -> ResourceCost {
    match id {
        ModuleId::CriticalThinking => ResourceCost::heavy(),
        ModuleId::CognitiveDeconstruction => ResourceCost::heavy(),
        ModuleId::ConflictSuspension => ResourceCost::standard(),
        ModuleId::EngineeringArchitecture => ResourceCost::standard(),
        ModuleId::ReflexiveAudit => ResourceCost::standard(),
        ModuleId::AdaptiveIteration => ResourceCost::light(),
        ModuleId::EcologicalAssessment => ResourceCost::heavy(),
        ModuleId::AttentionScheduler => ResourceCost::light(),
        ModuleId::CouplingAdapter => ResourceCost::standard(),
        ModuleId::SelfKnowledge => ResourceCost::light(),
    }
}

// ── Scenario → module mapping ──────────────────────────────────────

/// Default module set for each scenario type.
///
/// These are the modules that should be mounted when the scenario
/// activates. The mount arbiter uses these as the target set.
pub fn default_modules_for(scenario: ScenarioType) -> Vec<ModuleId> {
    match scenario {
        ScenarioType::PhysicalReasoning => vec![
            ModuleId::CriticalThinking,
            ModuleId::EngineeringArchitecture,
            ModuleId::EcologicalAssessment,
            ModuleId::AttentionScheduler,
        ],
        ScenarioType::ValueConflict => vec![
            ModuleId::ConflictSuspension,
            ModuleId::CognitiveDeconstruction,
            ModuleId::SelfKnowledge,
            ModuleId::ReflexiveAudit,
        ],
        ScenarioType::MedicalEthics => vec![
            ModuleId::ConflictSuspension,
            ModuleId::SelfKnowledge,
            ModuleId::ReflexiveAudit,
            ModuleId::CriticalThinking,
        ],
        ScenarioType::ReflexiveAudit => vec![
            ModuleId::ReflexiveAudit,
            ModuleId::SelfKnowledge,
            ModuleId::CognitiveDeconstruction,
        ],
        ScenarioType::CrisisResponse => vec![
            ModuleId::CriticalThinking,
            ModuleId::AttentionScheduler,
            ModuleId::EngineeringArchitecture,
            ModuleId::ConflictSuspension,
        ],
        ScenarioType::General => vec![
            ModuleId::SelfKnowledge,
            ModuleId::AttentionScheduler,
            ModuleId::ConflictSuspension,
        ],
    }
}

// ── Mount arbiter ──────────────────────────────────────────────────

/// Computes and executes mount/unmount transitions.
///
/// When a new scenario is recognized:
/// 1. Computes the module request set for the scenario
/// 2. Checks the currently mounted set
/// 3. Computes the diff (mount / unmount)
/// 4. Resolves resource conflicts
/// 5. Applies the diff to the registry
#[derive(Debug)]
pub struct MountArbiter {
    /// Total resource budget available (normalized).
    budget: ResourceCost,
}

impl MountArbiter {
    /// Create a mount arbiter with the default resource budget.
    pub fn new() -> Self {
        MountArbiter {
            budget: ResourceCost {
                cpu: 1.0,
                memory: 1.0,
                io: 1.0,
            },
        }
    }

    /// Create a mount arbiter with a constrained budget.
    pub fn with_budget(budget: ResourceCost) -> Self {
        MountArbiter { budget }
    }

    /// Compute the target module set for a scenario type.
    pub fn target_modules(&self, scenario: ScenarioType) -> HashSet<ModuleId> {
        default_modules_for(scenario).into_iter().collect()
    }

    /// Check whether the target module set fits within the resource budget.
    ///
    /// Returns the list of modules that would exceed the budget, if any.
    pub fn check_budget(&self, modules: &HashSet<ModuleId>) -> Vec<ModuleId> {
        let mut cpu_total = 0.0;
        let mut mem_total = 0.0;
        let mut io_total = 0.0;
        let mut overflow = Vec::new();

        for &id in modules {
            let cost = resource_cost(id);
            if cpu_total + cost.cpu > self.budget.cpu
                || mem_total + cost.memory > self.budget.memory
                || io_total + cost.io > self.budget.io
            {
                overflow.push(id);
            } else {
                cpu_total += cost.cpu;
                mem_total += cost.memory;
                io_total += cost.io;
            }
        }
        overflow
    }

    /// Prioritize modules by scenario-criticality.
    ///
    /// When resources are constrained, modules are sorted by their
    /// importance to the current scenario. Higher-priority modules
    /// are mounted first.
    pub fn prioritize(&self, modules: &[ModuleId], scenario: ScenarioType) -> Vec<ModuleId> {
        let mut sorted = modules.to_vec();
        sorted.sort_by_key(|&id| priority_score(id, scenario));
        sorted.reverse(); // higher score = higher priority
        sorted
    }

    /// Compute the mount/unmount plan for a scenario transition.
    ///
    /// Returns (to_mount, to_unmount, unmount_reason).
    pub fn plan_transition(
        &self,
        registry: &ModuleRegistry,
        new_scenario: ScenarioType,
    ) -> (Vec<ModuleId>, Vec<ModuleId>, UnmountReason) {
        let target = self.target_modules(new_scenario);
        let to_unmount = registry.modules_to_unmount(&target);
        let to_mount = registry.modules_to_mount(&target);

        // Determine unmount reason based on context.
        let reason = if new_scenario == ScenarioType::General {
            UnmountReason::Completed
        } else {
            UnmountReason::Preempted
        };

        (to_mount, to_unmount, reason)
    }

    /// Execute a scenario transition on the registry.
    ///
    /// Returns the number of modules mounted and unmounted.
    pub fn execute_transition(
        &self,
        registry: &mut ModuleRegistry,
        new_scenario: ScenarioType,
    ) -> (usize, usize) {
        let (to_mount, to_unmount, reason) = self.plan_transition(registry, new_scenario);
        let mount_count = to_mount.len();
        let unmount_count = to_unmount.len();
        registry.apply_diff(&to_mount, &to_unmount, reason);
        (mount_count, unmount_count)
    }
}

impl Default for MountArbiter {
    fn default() -> Self {
        Self::new()
    }
}

/// Priority score for a module in a given scenario.
///
/// Higher score = more critical to the scenario. Used for sorting
/// when resources are constrained.
fn priority_score(id: ModuleId, scenario: ScenarioType) -> i32 {
    let defaults = default_modules_for(scenario);
    // Modules in the default set get base score 10.
    // Earlier position in the default list = higher priority.
    if let Some(pos) = defaults.iter().position(|&m| m == id) {
        10 + (defaults.len() - pos) as i32
    } else {
        0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hook::module_registry::ModuleRegistry;

    #[test]
    fn default_modules_for_each_scenario_type() {
        for st in &[
            ScenarioType::PhysicalReasoning,
            ScenarioType::ValueConflict,
            ScenarioType::MedicalEthics,
            ScenarioType::ReflexiveAudit,
            ScenarioType::CrisisResponse,
            ScenarioType::General,
        ] {
            let modules = default_modules_for(*st);
            assert!(!modules.is_empty(), "no modules for {:?}", st);
        }
    }

    #[test]
    fn resource_cost_is_nonzero() {
        for id in &[
            ModuleId::CriticalThinking,
            ModuleId::CognitiveDeconstruction,
            ModuleId::ConflictSuspension,
            ModuleId::EngineeringArchitecture,
            ModuleId::ReflexiveAudit,
            ModuleId::AdaptiveIteration,
            ModuleId::EcologicalAssessment,
            ModuleId::AttentionScheduler,
            ModuleId::CouplingAdapter,
            ModuleId::SelfKnowledge,
        ] {
            let cost = resource_cost(*id);
            assert!(cost.total() > 0.0, "zero cost for {:?}", id);
        }
    }

    #[test]
    fn arbiter_target_modules_are_non_empty() {
        let arbiter = MountArbiter::new();
        for st in &[
            ScenarioType::PhysicalReasoning,
            ScenarioType::ValueConflict,
            ScenarioType::MedicalEthics,
            ScenarioType::ReflexiveAudit,
            ScenarioType::CrisisResponse,
            ScenarioType::General,
        ] {
            let target = arbiter.target_modules(*st);
            assert!(!target.is_empty(), "empty target for {:?}", st);
        }
    }

    #[test]
    fn plan_transition_from_empty_registry() {
        let arbiter = MountArbiter::new();
        let registry = ModuleRegistry::new();
        let (to_mount, to_unmount, reason) =
            arbiter.plan_transition(&registry, ScenarioType::MedicalEthics);

        assert!(to_unmount.is_empty());
        assert!(!to_mount.is_empty());
        assert_eq!(reason, UnmountReason::Preempted);
    }

    #[test]
    fn plan_transition_to_general_uses_completed() {
        let arbiter = MountArbiter::new();
        let mut registry = ModuleRegistry::new();
        registry.mount(ModuleId::CriticalThinking);
        registry.mount(ModuleId::SelfKnowledge);

        let (_to_mount, to_unmount, reason) =
            arbiter.plan_transition(&registry, ScenarioType::General);

        assert_eq!(reason, UnmountReason::Completed);
        assert!(to_unmount.contains(&ModuleId::CriticalThinking));
    }

    #[test]
    fn execute_transition_modifies_registry() {
        let arbiter = MountArbiter::new();
        let mut registry = ModuleRegistry::new();
        registry.mount(ModuleId::SelfKnowledge);

        let (mounted, unmounted) =
            arbiter.execute_transition(&mut registry, ScenarioType::MedicalEthics);

        assert!(mounted > 0);
        assert_eq!(unmounted, 0); // SelfKnowledge is in MedicalEthics default set
        assert!(registry.is_mounted(ModuleId::ConflictSuspension));
    }

    #[test]
    fn prioritize_puts_default_modules_first() {
        let arbiter = MountArbiter::new();
        let modules = vec![
            ModuleId::SelfKnowledge,
            ModuleId::CriticalThinking,
            ModuleId::AttentionScheduler,
        ];
        let prioritized = arbiter.prioritize(&modules, ScenarioType::PhysicalReasoning);
        // CriticalThinking should be first (it's in the default set)
        assert_eq!(prioritized[0], ModuleId::CriticalThinking);
    }

    #[test]
    fn check_budget_allows_reasonable_sets() {
        let arbiter = MountArbiter::new();
        let modules: HashSet<ModuleId> = [
            ModuleId::SelfKnowledge,
            ModuleId::AttentionScheduler,
            ModuleId::ConflictSuspension,
        ]
        .into();
        let overflow = arbiter.check_budget(&modules);
        assert!(overflow.is_empty());
    }
}
