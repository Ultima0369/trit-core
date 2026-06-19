//! Dynamic adapter module pool.
//!
//! Each module implements the `CognitiveModule` trait and is mounted/unmounted
//! by the Hook Manager (Layer 2) according to scenario needs. No module is
//! "always on" — even the reflexive auditor runs only when the scenario demands it.
//!
//! ## Design Rules
//!
//! 1. Modules do not call each other. All cross-module communication goes through `HookContext`.
//! 2. Every module output includes a confidence score in [0.0, 1.0].
//! 3. Unmount = release. When `on_unmount()` is called, the module must persist
//!    any state it needs and release computational resources.
//! 4. The Adaptive Iteration module is the only module that can modify system behavior,
//!    and its permissions are strictly bounded.

pub mod adaptive_iteration;
pub mod bandwidth_scheduler;
pub mod cognitive_deconstruction;
pub mod conflict_suspension;
pub mod coupling_adapter;
pub mod critical_thinking;
pub mod ecological_assessment;
pub mod engineering;
pub mod reflexive_audit;
pub mod self_knowledge;

use serde::{Deserialize, Serialize};

use crate::feedback::FeedbackSignal;
use crate::hook::{HookContext, UnmountReason};

/// Unique identifier for a module instance.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Deserialize, Serialize)]
pub struct ModuleId(pub String);

impl ModuleId {
    pub fn new(id: impl Into<String>) -> Self {
        ModuleId(id.into())
    }
}

impl std::fmt::Display for ModuleId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// The operational state of a module.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
pub enum ModuleState {
    /// Module is mounted and actively processing.
    Active,
    /// Module is mounted but idle (waiting for input).
    Idle,
    /// Module is suspended (soft-unmounted, context preserved).
    Suspended,
    /// Module is unmounted (hard-unmounted, resources released).
    Unmounted,
    /// Module encountered an error and requires intervention.
    Error,
}

/// Input to a cognitive module's `process` method.
#[derive(Debug, Clone)]
pub struct ModuleInput {
    /// The raw signal words being processed.
    pub signals: Vec<crate::core::word::TritWord>,
    /// Optional text/narrative input.
    pub text: Option<String>,
    /// Optional metadata key-value pairs.
    pub metadata: std::collections::HashMap<String, String>,
}

/// Output from a cognitive module's `process` method.
#[derive(Debug, Clone)]
pub struct ModuleOutput {
    /// The module's processed result as trit words.
    pub results: Vec<crate::core::word::TritWord>,
    /// Confidence in this output, in [0.0, 1.0].
    pub confidence: f64,
    /// Whether this output should trigger an explanation impulse alert.
    pub explanation_impulse_detected: bool,
    /// Human-readable summary of the module's processing.
    pub summary: String,
    /// Any warnings or alerts the module wants to emit.
    pub warnings: Vec<String>,
}

/// The core trait that every cognitive module must implement.
pub trait CognitiveModule: Send + Sync {
    /// Unique identifier for this module instance.
    fn id(&self) -> ModuleId;

    /// Human-readable name of this module.
    fn name(&self) -> &'static str;

    /// Process input signals and produce an output.
    fn process_signals(&mut self, input: &ModuleInput, ctx: &HookContext) -> ModuleOutput;

    /// Called when the module is mounted (activated).
    fn on_mount(&mut self);

    /// Called when the module is unmounted (deactivated).
    /// `reason` explains why the unmount happened, for auditability.
    fn on_unmount(&mut self, reason: UnmountReason);

    /// Current operational state of the module.
    fn state(&self) -> ModuleState;

    /// Calibrate internal parameters based on feedback from Layer 5.
    /// Returns the updated confidence after calibration.
    fn calibrate(&mut self, feedback: &FeedbackSignal) -> f64;
}
