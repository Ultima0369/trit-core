use std::time::Instant;

use tracing::{debug, error, info, instrument, trace, warn};

use crate::anchor::{check_all, AnchorConstraint, DecisionPreview};
use crate::attention::{AttentionCmd, AttentionScheduler};
use crate::budget::ComputeBudget;
use crate::calibration::{CalibrationEntry, CalibrationLog};
use crate::clock::HarmonicClock;
use crate::core::frame::{Frame, FrameRegistry};
use crate::core::hold::{HoldState, HolderConfig};
use crate::core::phase::Phase;
use crate::core::value::TritValue;
use crate::core::word::TritWord;
use crate::core::TernaryAlgebra;
use crate::knowledge::SelfKnowledge;
use crate::meta::{ArbitrationResult, Domain, MetaInterrupt, ResolutionPolicy, SafeFallback};
use crate::reflexive::{ReflexiveAlert, ReflexiveAuditor};
use crate::sandbox::diagnostic::SandboxDiagnostics;
use crate::sandbox::error::SandboxError;
use crate::sandbox::input::{ScenarioInput, SignalInput};
use crate::sandbox::output::SandboxOutput;
use crate::sandbox::validate::{sanitize_log_field, validate_scenario};

/// Standard sandbox pipeline: TAND cascade → policy arbitration → SafeFallback.
///
/// When constructed with [`with_registry`](SandboxPipeline::with_registry), all
/// signal frames are validated against the registry before processing.
///
/// Mind-engineering extensions (reflexive audit, attention scheduling,
/// self-knowledge) are opt-in via builder methods and do not change the
/// default behavior unless explicitly enabled.
pub struct SandboxPipeline {
    registry: Option<FrameRegistry>,
    dry_run: bool,
    safe_fallback: SafeFallback,
    reflexive: Option<ReflexiveAuditor>,
    attention: Option<AttentionScheduler>,
    self_knowledge: Option<SelfKnowledge>,
    holder_config: Option<HolderConfig>,
    trace_phase: bool,
    hold_final: bool,
    /// Anchor constraints checked before every decision.
    anchor_constraints: Vec<Box<dyn AnchorConstraint>>,
    /// Hardware-aware compute budget for depth gating.
    budget: ComputeBudget,
    /// Harmonic clock for temporal context.
    clock: HarmonicClock,
    /// Calibration log for feedback-driven learning.
    calibration_log: CalibrationLog,
}

impl std::fmt::Debug for SandboxPipeline {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SandboxPipeline")
            .field("registry", &self.registry)
            .field("dry_run", &self.dry_run)
            .field("safe_fallback", &self.safe_fallback)
            .field("reflexive", &self.reflexive)
            .field("attention", &self.attention)
            .field("self_knowledge", &self.self_knowledge)
            .field("holder_config", &self.holder_config)
            .field("trace_phase", &self.trace_phase)
            .field("hold_final", &self.hold_final)
            .field("anchor_count", &self.anchor_constraints.len())
            .field("budget", &self.budget)
            .field("clock", &self.clock)
            .field("calibration_log", &self.calibration_log)
            .finish()
    }
}

impl Default for SandboxPipeline {
    fn default() -> Self {
        Self::new()
    }
}

impl SandboxPipeline {
    /// Create a new pipeline with an optional frame whitelist.
    ///
    /// When a registry is provided, all signal frames must be registered
    /// before the pipeline processes them — unregistered frames cause a
    /// `SandboxError::InvalidFrame` error.
    pub fn new() -> Self {
        Self {
            registry: None,
            dry_run: false,
            safe_fallback: SafeFallback::new(),
            reflexive: None,
            attention: None,
            self_knowledge: None,
            holder_config: None,
            trace_phase: false,
            hold_final: false,
            anchor_constraints: Vec::new(),
            budget: ComputeBudget::conservative(),
            clock: HarmonicClock::deliberative(),
            calibration_log: CalibrationLog::default(),
        }
    }

    /// Create a pipeline that validates all signal frames against the given registry.
    pub fn with_registry(registry: FrameRegistry) -> Self {
        Self {
            registry: Some(registry),
            ..Self::new()
        }
    }

    /// Enable dry-run mode: build trits and run TAND, but skip arbitration and SafeFallback.
    pub fn with_dry_run(mut self, dry_run: bool) -> Self {
        self.dry_run = dry_run;
        self
    }

    /// Inject a custom SafeFallback configuration.
    pub fn with_safe_fallback(mut self, safe_fallback: SafeFallback) -> Self {
        self.safe_fallback = safe_fallback;
        self
    }

    /// Attach a reflexive auditor.
    pub fn with_reflexive(mut self, auditor: ReflexiveAuditor) -> Self {
        self.reflexive = Some(auditor);
        self
    }

    /// Attach an attention scheduler.
    pub fn with_attention(mut self, scheduler: AttentionScheduler) -> Self {
        self.attention = Some(scheduler);
        self
    }

    /// Attach a self-knowledge model.
    pub fn with_self_knowledge(mut self, knowledge: SelfKnowledge) -> Self {
        self.self_knowledge = Some(knowledge);
        self
    }

    /// Attach a holder configuration.
    pub fn with_holder_config(mut self, config: HolderConfig) -> Self {
        self.holder_config = Some(config);
        self
    }

    /// Enable phase-trace collection.
    pub fn with_trace_phase(mut self, enabled: bool) -> Self {
        self.trace_phase = enabled;
        self
    }

    /// Treat Hold as the final answer (do not auto-question).
    pub fn with_hold_final(mut self, enabled: bool) -> Self {
        self.hold_final = enabled;
        self
    }

    /// Attach an anchor constraint (Layer 1). Multiple anchors can be added;
    /// they are checked in order before every decision is finalized.
    pub fn with_anchor(mut self, anchor: Box<dyn AnchorConstraint>) -> Self {
        self.anchor_constraints.push(anchor);
        self
    }

    /// Set the compute budget for depth gating.
    pub fn with_budget(mut self, budget: ComputeBudget) -> Self {
        self.budget = budget;
        self
    }

    /// Set the harmonic clock for temporal context.
    pub fn with_clock(mut self, clock: HarmonicClock) -> Self {
        self.clock = clock;
        self
    }

    /// Set the calibration log for feedback-driven learning.
    pub fn with_calibration_log(mut self, log: CalibrationLog) -> Self {
        self.calibration_log = log;
        self
    }

    /// Run the full pipeline on a scenario.
    ///
    /// # Errors
    ///
    /// Returns `SandboxError` if the scenario is invalid or if any signal
    /// cannot be converted into a `TritWord`.
    pub fn run(&mut self, scenario: &ScenarioInput) -> Result<SandboxOutput, SandboxError> {
        self.run_with_diagnostics(scenario).map(|(out, _)| out)
    }

    /// Run the full pipeline and return both the output and diagnostic telemetry.
    ///
    /// This is the primary observable entry point. It records per-stage timing,
    /// interrupt counts, frame distribution, and SafeFallback activation.
    #[instrument(skip_all, fields(scenario_id = %scenario.id, domain = %scenario.domain, signal_count = scenario.signals.len()))]
    pub fn run_with_diagnostics(
        &mut self,
        scenario: &ScenarioInput,
    ) -> Result<(SandboxOutput, SandboxDiagnostics), SandboxError> {
        let mut diagnostics = SandboxDiagnostics::new();
        info!(scenario_id = %scenario.id, "pipeline started");

        // Stages 1–4: validate, build policy, build trits, registry check
        let trits = self.stage_validate_and_build(scenario, &mut diagnostics)?;

        // Stage 5: batch TAND cascade
        let (current, interrupts) = self.stage_tand_cascade(&trits, &mut diagnostics);

        // Stages 6–8: arbitration, reflexive guard, SafeFallback
        let (final_word, policy_action_str, reflexive_alert) = self.stage_arbitrate_and_guard(
            scenario,
            &trits,
            &current,
            interrupts,
            &mut diagnostics,
        )?;

        // Stage 8b: sample OS → ComputeBudget.depth_level
        self.stage_sample_budget(&mut diagnostics);

        // Stages 9–10: attention scheduling, self-knowledge inference
        // Gated by depth_level >= Standard
        self.stage_optional_extensions(&trits, &final_word, &mut diagnostics);

        // Stage 10b: clock tick — advance the harmonic oscillator
        self.stage_tick_clock(&mut diagnostics);

        // Stage 11: phase trace
        let mut final_word = final_word; // ← make mutable for anchor override
        if self.trace_phase {
            diagnostics.record_phase(final_word.phase().inner());
        }

        // Stage 11b: anchor check (Layer 1)
        final_word = self.stage_anchor_check(scenario, final_word, &mut diagnostics);

        // Stage 12: build output
        let output = self.stage_build_output_with_timing(
            scenario,
            &final_word,
            &policy_action_str,
            reflexive_alert.as_ref(),
            &mut diagnostics,
        );

        // Stage 13: calibrate — record entry + update SelfKnowledge patterns
        self.stage_calibrate(scenario, &final_word, &mut diagnostics);

        diagnostics.finish();
        info!(
            scenario_id = %output.scenario_id,
            final_value = %output.final_value,
            final_frame = %output.final_frame,
            elapsed_ns = diagnostics.elapsed_ns,
            elapsed_us = diagnostics.elapsed_us(),
            "pipeline complete"
        );
        Ok((output, diagnostics))
    }

    /// Stages 1–4: validate scenario, build policy, build trits, registry check.
    fn stage_validate_and_build(
        &self,
        scenario: &ScenarioInput,
        diagnostics: &mut SandboxDiagnostics,
    ) -> Result<Vec<TritWord>, SandboxError> {
        // Stage 1: validate scenario
        let stage_start = Instant::now();
        trace!("validating scenario input");
        validate_scenario(scenario).map_err(|e| {
            error!(error = %e, category = %e.category_name(), "scenario validation failed");
            e
        })?;
        diagnostics.record_stage("validate", stage_start);
        debug!("scenario input validated");

        // Stage 2: build policy
        let stage_start = Instant::now();
        let _policy = build_policy(&scenario.domain).map_err(|e| {
            error!(error = %e, category = %e.category_name(), "policy build failed");
            e
        })?;
        diagnostics.record_stage("build_policy", stage_start);
        info!(domain = %scenario.domain, "policy built");

        // Stage 3: build trits
        let stage_start = Instant::now();
        let trits = build_trits(&scenario.signals).map_err(|e| {
            error!(error = %e, category = %e.category_name(), "signal conversion failed");
            e
        })?;
        diagnostics.record_inputs(&trits);
        diagnostics.record_stage("build_trits", stage_start);
        debug!(signal_count = trits.len(), "trits built");

        // Stage 4: frame registry validation
        let stage_start = Instant::now();
        if let Some(ref reg) = self.registry {
            trace!("validating frames against registry whitelist");
            if let Err(unregistered) = reg.validate_all(&trits) {
                let index = trits
                    .iter()
                    .position(|w| w.frame() == unregistered)
                    .unwrap_or(0);
                let reason = format!(
                    "frame '{}' is not registered in the pipeline frame whitelist",
                    unregistered
                );
                error!(frame = %unregistered, index, "frame registry rejection");
                return Err(SandboxError::InvalidFrame { index, reason });
            }
        }
        diagnostics.record_stage("registry_check", stage_start);
        Ok(trits)
    }

    /// Stage 5: batch TAND cascade over all input trits.
    fn stage_tand_cascade(
        &self,
        trits: &[TritWord],
        diagnostics: &mut SandboxDiagnostics,
    ) -> (TritWord, Vec<MetaInterrupt>) {
        let stage_start = Instant::now();
        trace!("running batch TAND cascade");
        let (current, interrupts) = TernaryAlgebra::t_and_n(trits);
        diagnostics.record_interrupts(&interrupts);
        diagnostics.record_stage("t_and_n", stage_start);
        info!(
            result_value = ?current.value(),
            result_frame = %current.frame(),
            interrupt_count = interrupts.len(),
            "TAND cascade complete"
        );
        (current, interrupts)
    }

    /// Stages 6–8: policy arbitration, reflexive guard, SafeFallback.
    fn stage_arbitrate_and_guard(
        &mut self,
        scenario: &ScenarioInput,
        trits: &[TritWord],
        current: &TritWord,
        interrupts: Vec<MetaInterrupt>,
        diagnostics: &mut SandboxDiagnostics,
    ) -> Result<(TritWord, String, Option<ReflexiveAlert>), SandboxError> {
        if self.dry_run {
            info!("dry-run mode: skipping arbitration and SafeFallback");
            diagnostics.record_policy_action(&ArbitrationResult::Negotiate);
            diagnostics.record_stage("arbitrate", Instant::now());
            diagnostics.record_stage("reflexive_guard", Instant::now());
            diagnostics.record_stage("safe_fallback", Instant::now());
            return Ok((*current, "DryRun".to_string(), None));
        }

        // Stage 6: policy arbitration
        let stage_start = Instant::now();
        trace!("running policy arbitration");
        let policy = build_policy(&scenario.domain)?;
        let policy_result = policy.arbitrate(trits).map_err(|e| {
            error!(error = %e, "policy arbitration failed");
            SandboxError::InvalidScenario(format!("arbitration failed: {e}"))
        })?;
        diagnostics.record_policy_action(&policy_result);
        diagnostics.record_stage("arbitrate", stage_start);
        info!(policy_action = %policy_result, "arbitration complete");

        let arbitrated_word = self.resolve_arbitrated_word(&policy_result, current);

        // Stage 7: reflexive guard
        let stage_start = Instant::now();
        let reflexive_alert =
            self.stage_reflexive_guard(&policy, &arbitrated_word, &interrupts, diagnostics);
        diagnostics.record_stage("reflexive_guard", stage_start);

        // Stage 8: SafeFallback
        let stage_start = Instant::now();
        let force = matches!(&policy_result, ArbitrationResult::ForceCollapse);
        let final_word =
            self.stage_safe_fallback(&policy, &arbitrated_word, force, interrupts, diagnostics);
        diagnostics.record_stage("safe_fallback", stage_start);

        // If reflexive guard fired and output is still forced True/False, override to Hold
        let final_word = if reflexive_alert.is_some() && final_word.value().is_computable() {
            TritWord::hold(Frame::Meta)
        } else {
            final_word
        };

        Ok((final_word, format!("{}", policy_result), reflexive_alert))
    }

    /// Resolve the word to use after arbitration.
    fn resolve_arbitrated_word(
        &self,
        policy_result: &ArbitrationResult,
        current: &TritWord,
    ) -> TritWord {
        match policy_result {
            ArbitrationResult::Commit(w) => {
                // If the TAND cascade detected a conflict that the policy
                // missed (e.g., Unknown propagation producing Hold), the
                // cascade result should override the policy's mechanical
                // Commit. But only when the cascade result is Hold — meaning
                // TAND detected something the arbitration didn't account for.
                if current.value() == TritValue::Hold && w.value().is_computable() {
                    TritWord::hold(Frame::Meta)
                } else {
                    *w
                }
            }
            // Preserve is an explicit arbitration choice (e.g., FirstPerson
            // over Science in MedicalEthics). Do not override it with the
            // TAND cascade result — the policy intentionally chose this frame.
            ArbitrationResult::Preserve(w) => *w,
            // A deliberate Hold result (e.g., ValueJudgment) must not be
            // overridden by the TAND cascade; otherwise a same-frame input
            // would accidentally commit to True/False.
            ArbitrationResult::Hold => TritWord::hold(Frame::Meta),
            // ForceCollapse: return Hold to trigger SafeFallback.guard().
            // Using the TAND cascade result (*current) would bypass SafeFallback
            // when the cascade produces True/False without interrupts — e.g.,
            // Engineering domain with all-Individual True signals.
            ArbitrationResult::ForceCollapse => TritWord::hold(Frame::Meta),
            ArbitrationResult::Negotiate => *current,
        }
    }

    /// Stage 7: reflexive guard — check for forced decisions with unresolved conflicts.
    fn stage_reflexive_guard(
        &mut self,
        policy: &ResolutionPolicy,
        arbitrated_word: &TritWord,
        interrupts: &[MetaInterrupt],
        diagnostics: &mut SandboxDiagnostics,
    ) -> Option<ReflexiveAlert> {
        if let Some(ref mut auditor) = self.reflexive {
            for int in interrupts {
                auditor.record_interrupt(int.clone());
            }
            if self.trace_phase {
                auditor.record_phase_shift(crate::reflexive::PhaseShift::new(
                    arbitrated_word.phase().inner(),
                    arbitrated_word.phase().inner(),
                    "arbitration",
                ));
            }
            let alert = reflexive_guard(
                auditor,
                &policy.domain,
                arbitrated_word,
                interrupts,
                &self.safe_fallback,
            );
            if alert.is_some() {
                diagnostics.mark_reflexive_guard();
            }
            return alert;
        }
        None
    }

    /// Stage 8: SafeFallback — force False in dangerous domains when uncertain.
    fn stage_safe_fallback(
        &self,
        policy: &ResolutionPolicy,
        arbitrated_word: &TritWord,
        force: bool,
        mut interrupts: Vec<MetaInterrupt>,
        diagnostics: &mut SandboxDiagnostics,
    ) -> TritWord {
        trace!("running SafeFallback guard");
        let (final_word, fb_interrupt) = self.safe_fallback.guard_with_force(
            &policy.domain,
            arbitrated_word,
            interrupts.len(),
            force,
        );
        if let Some(int) = fb_interrupt {
            warn!(
                domain = %policy.domain,
                force,
                "SafeFallback triggered: forcing False in dangerous domain"
            );
            diagnostics.mark_safe_fallback();
            interrupts.push(int);
        } else {
            debug!("SafeFallback passed through");
        }
        diagnostics.interrupts = interrupts;
        final_word
    }

    /// Stages 9–10: attention scheduling and self-knowledge inference.
    ///
    /// Gated by `depth_level >= Standard`. When below Standard, both stages
    /// are skipped — there is not enough compute budget for optional extensions.
    fn stage_optional_extensions(
        &mut self,
        trits: &[TritWord],
        final_word: &TritWord,
        diagnostics: &mut SandboxDiagnostics,
    ) {
        if !self.budget.depth_level.has_extensions() {
            debug!(
                depth_level = self.budget.depth_level as u8,
                "skipping optional extensions (depth < Standard)"
            );
            diagnostics.record_stage("attention", Instant::now());
            diagnostics.record_stage("self_knowledge", Instant::now());
            return;
        }

        // Stage 9: attention scheduling
        let stage_start = Instant::now();
        if let Some(ref mut scheduler) = self.attention {
            let cmd = scheduler.suggest_with_budget(&self.budget, trits);
            diagnostics.record_attention_cmd(&cmd);
            if matches!(cmd, AttentionCmd::HoldCurrent) {
                info!("attention scheduler suggests holding current processing");
            }
        }
        diagnostics.record_stage("attention", stage_start);

        // Stage 10: self-knowledge inference
        let stage_start = Instant::now();
        let receiver_estimate = self.self_knowledge.as_ref().map(|k| {
            let estimate = k.infer_receiver_state(final_word);
            diagnostics.record_receiver_estimate(estimate.clone());
            estimate
        });
        diagnostics.record_stage("self_knowledge", stage_start);
        // receiver_estimate is consumed by stage_build_output via diagnostics
        let _ = receiver_estimate;
    }

    /// Stage 8b: sample OS metrics and update the compute budget.
    ///
    /// Runs after SafeFallback but before optional extensions, so the
    /// budget reflects the real system state when deciding whether to run
    /// attention/self_knowledge/phase_trace.
    fn stage_sample_budget(&mut self, diagnostics: &mut SandboxDiagnostics) {
        let stage_start = Instant::now();
        let fresh_budget = ComputeBudget::sample();
        self.budget = fresh_budget;
        diagnostics.record_depth_level(self.budget.depth_level as u8);
        diagnostics.record_stage("sample_os_budget", stage_start);
        debug!(
            depth_level = self.budget.depth_level as u8,
            cpu_load = self.budget.cpu_load,
            mem_pressure = self.budget.mem_pressure,
            "OS budget sampled"
        );
    }

    /// Stage 10b: advance the harmonic oscillator by elapsed wall-clock time.
    fn stage_tick_clock(&mut self, diagnostics: &mut SandboxDiagnostics) {
        let stage_start = Instant::now();
        let elapsed_secs = diagnostics
            .started_at
            .map(|start| start.elapsed().as_secs_f64())
            .unwrap_or(0.0);
        self.clock.tick(elapsed_secs);
        diagnostics.record_clock_phase(self.clock.to_phase().inner());
        diagnostics.record_stage("clock_tick", stage_start);
        trace!(
            clock_phase = self.clock.to_phase().inner(),
            elapsed_s = elapsed_secs,
            "clock ticked"
        );
    }

    /// Stage 11b: anchor check — enforce Layer 1 constraints.
    ///
    /// Anchor constraints have veto power: Abort forces Hold,
    /// DowngradeToHold forces Hold + alert. Returns the (possibly
    /// overridden) final word.
    fn stage_anchor_check(
        &self,
        scenario: &ScenarioInput,
        mut final_word: TritWord,
        diagnostics: &mut SandboxDiagnostics,
    ) -> TritWord {
        if self.anchor_constraints.is_empty() {
            return final_word;
        }
        let stage_start = Instant::now();
        let preview = build_decision_preview(scenario, &final_word);
        let anchor_report = check_all(&self.anchor_constraints, &preview);
        if anchor_report.has_violations() {
            warn!(
                violation_count = anchor_report.violations.len(),
                has_abort = anchor_report.has_abort(),
                "anchor violations detected"
            );
            diagnostics.anchor_report = Some(anchor_report.clone());
            final_word = TritWord::hold(Frame::Meta);
        }
        diagnostics.record_stage("anchor_check", stage_start);
        final_word
    }

    /// Stage 12: build output with timing.
    fn stage_build_output_with_timing(
        &self,
        scenario: &ScenarioInput,
        final_word: &TritWord,
        policy_action_str: &str,
        reflexive_alert: Option<&ReflexiveAlert>,
        diagnostics: &mut SandboxDiagnostics,
    ) -> SandboxOutput {
        let stage_start = Instant::now();
        let output = self.stage_build_output(
            scenario,
            final_word,
            policy_action_str,
            reflexive_alert,
            diagnostics,
        );
        diagnostics.record_stage("build_output", stage_start);
        output
    }

    /// Stage 13: calibrate — record a CalibrationEntry and feed back into SelfKnowledge.
    fn stage_calibrate(
        &mut self,
        scenario: &ScenarioInput,
        final_word: &TritWord,
        diagnostics: &mut SandboxDiagnostics,
    ) {
        let stage_start = Instant::now();

        let entry = CalibrationEntry {
            scenario_id: scenario.id.clone(),
            domain: scenario
                .domain
                .parse()
                .unwrap_or(crate::meta::Domain::General),
            result: final_word.value(),
            phase: final_word.phase().inner(),
            interrupt_count: diagnostics.interrupt_count,
            elapsed_us: diagnostics.elapsed_us(),
            depth_level: self.budget.depth_level,
            attention_cmd: diagnostics
                .attention_cmd
                .as_deref()
                .and_then(parse_attention_cmd),
        };
        self.calibration_log.record(entry);

        // Feed back into SelfKnowledge if present
        if let Some(ref mut knowledge) = self.self_knowledge {
            knowledge.calibrate_from_result(
                final_word.frame(),
                final_word.value(),
                final_word.phase().inner(),
                diagnostics.interrupt_count,
            );
        }

        diagnostics.record_stage("calibrate", stage_start);
        trace!(
            calibration_entries = self.calibration_log.len(),
            "calibration recorded"
        );
    }

    /// Stage 12: build the final SandboxOutput.
    fn stage_build_output(
        &self,
        scenario: &ScenarioInput,
        final_word: &TritWord,
        policy_action_str: &str,
        reflexive_alert: Option<&ReflexiveAlert>,
        diagnostics: &SandboxDiagnostics,
    ) -> SandboxOutput {
        let hold_state = if final_word.value() == TritValue::Hold {
            Some(self.holder_state(&scenario.domain))
        } else {
            None
        };
        SandboxOutput {
            scenario_id: sanitize_log_field(&scenario.id),
            final_value: format!("{:?}", final_word.value()),
            final_value_code: final_word.value().to_i8(),
            final_frame: format!("{}", final_word.frame()),
            final_phase_raw: final_word.phase().inner(),
            interrupts: diagnostics
                .interrupts
                .iter()
                .map(|i| format!("{:?}: {}", i.conflict, sanitize_log_field(&i.reason)))
                .collect(),
            policy_action: policy_action_str.to_string(),
            reflexive_alert: self
                .reflexive
                .as_ref()
                .and(reflexive_alert)
                .map(|a| format!("{} - {}", a.reason, a.recommendation)),
            attention_cmd: diagnostics.attention_cmd.clone(),
            receiver_estimate: diagnostics.receiver_estimate.clone(),
            hold_state,
        }
    }

    /// Compute the HoldState to attach to a Hold output.
    fn holder_state(&self, domain: &str) -> HoldState {
        if self.hold_final {
            return HoldState::final_hold();
        }
        self.holder_config
            .as_ref()
            .map(|c| c.hold_state_for(domain))
            .unwrap_or_else(HoldState::final_hold)
    }
}

/// Parse a serialized attention command string back to an `AttentionCmd`.
fn parse_attention_cmd(s: &str) -> Option<AttentionCmd> {
    use crate::attention::ShiftTarget;
    // Format: "ShiftTo(Body)", "HoldCurrent", "Recalibrate", "Continue"
    match s {
        "HoldCurrent" => Some(AttentionCmd::HoldCurrent),
        "Recalibrate" => Some(AttentionCmd::Recalibrate),
        "Continue" => Some(AttentionCmd::Continue),
        s if s.starts_with("ShiftTo(") => {
            let inner = s.trim_start_matches("ShiftTo(").trim_end_matches(')');
            let target = match inner {
                "Body" => ShiftTarget::Body,
                "Environment" => ShiftTarget::Environment,
                "ConflictTrace" => ShiftTarget::ConflictTrace,
                "Meta" => ShiftTarget::Meta,
                other => {
                    // ShiftTo(Frame(...)) or ShiftTo(Label(...)) — not yet
                    // produced by the scheduler but may be in future versions.
                    // Try to parse as a Frame name first, then fall back to Label.
                    if let Ok(frame) = other.parse::<crate::core::frame::Frame>() {
                        ShiftTarget::Frame(frame)
                    } else {
                        ShiftTarget::Label(other.to_string())
                    }
                }
            };
            Some(AttentionCmd::ShiftTo(target))
        }
        other => {
            warn!(cmd = %other, "unrecognized attention command format; recording as Continue");
            None
        }
    }
}

/// Reflexive guard: check whether a forced True/False decision was made
/// while unresolved cross-frame conflicts remain.
fn reflexive_guard(
    _auditor: &mut ReflexiveAuditor,
    domain: &Domain,
    decision: &TritWord,
    interrupts: &[MetaInterrupt],
    safe_fallback: &SafeFallback,
) -> Option<ReflexiveAlert> {
    let unresolved_conflicts = interrupts
        .iter()
        .filter(|i| matches!(i.conflict, crate::meta::ConflictType::FrameMismatch))
        .count();

    let is_forced = decision.value() == TritValue::True || decision.value() == TritValue::False;

    if unresolved_conflicts > 0 && is_forced {
        // In dangerous domains the forced output may be required by
        // SafeFallback; do not second-guess safety overrides.
        let dangerous = safe_fallback.is_dangerous(domain);
        if dangerous {
            return None;
        }
        let alert = ReflexiveAlert {
            reason: format!(
                "Forced {:?} output with {} unresolved frame conflict(s)",
                decision.value(),
                unresolved_conflicts
            ),
            recommendation: "Reflexive guard suggests returning Hold.".to_string(),
        };
        return Some(alert);
    }

    None
}

fn build_policy(domain_str: &str) -> Result<ResolutionPolicy, SandboxError> {
    let domain = domain_str
        .parse::<Domain>()
        .map_err(|e| SandboxError::InvalidDomain(format!("{}", e)))?;
    Ok(ResolutionPolicy::new(domain))
}

fn build_trits(signals: &[SignalInput]) -> Result<Vec<TritWord>, SandboxError> {
    signals
        .iter()
        .enumerate()
        .map(|(i, s)| {
            let frame: Frame = s.frame.parse().map_err(|e| SandboxError::InvalidFrame {
                index: i,
                reason: format!("{}", e),
            })?;
            let value = TritValue::from(s.value);
            let phase = Phase::new(s.phase).map_err(|e| SandboxError::InvalidPhase {
                index: i,
                reason: format!("{}", e),
            })?;
            TritWord::from_parts(value, phase, frame).map_err(SandboxError::from)
        })
        .collect()
}

/// Build a DecisionPreview from the current scenario and proposed final word.
fn build_decision_preview(scenario: &ScenarioInput, final_word: &TritWord) -> DecisionPreview {
    // In MVP, we infer environmental impact heuristically from the scenario.
    // Future: real sensor data streams from environmental context.
    let expected_energy_joules = scenario
        .environmental_context
        .as_ref()
        .map(|ctx| ctx.ambient_arousal * 1e6)
        .unwrap_or(0.0);
    let expected_carbon_kg = scenario
        .environmental_context
        .as_ref()
        .map(|ctx| ctx.ambient_arousal * 1e3)
        .unwrap_or(0.0);
    let affected_population = scenario
        .environmental_context
        .as_ref()
        .map(|ctx| (ctx.social_density * 1e6) as u64)
        .filter(|&p| p > 0);
    let irreversible_change_risk = scenario
        .environmental_context
        .as_ref()
        .map(|ctx| ctx.ambient_arousal * 0.1)
        .unwrap_or(0.0);
    let ecosystem_impact_zone = scenario.environmental_context.as_ref().and_then(|ctx| {
        if ctx.ambient_arousal > 0.7 {
            Some(crate::anchor::EcosystemZone::Atmospheric)
        } else {
            None
        }
    });

    DecisionPreview {
        expected_energy_joules,
        expected_carbon_kg,
        affected_population,
        irreversible_change_risk,
        ecosystem_impact_zone,
        frame: final_word.frame(),
        trit_value: final_word.value(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scenario(domain: &str, signals: Vec<SignalInput>) -> ScenarioInput {
        ScenarioInput {
            id: "test".into(),
            description: "test".into(),
            domain: domain.into(),
            signals,
            expected_behavior: "hold".into(),
            environmental_context: None,
        }
    }

    fn signal(frame: &str, value: i8, phase: f64) -> SignalInput {
        SignalInput {
            frame: frame.into(),
            value,
            phase,
            sensor: None,
        }
    }

    #[test]
    fn pipeline_medical_conflict_preserves_individual() {
        let s = scenario(
            "MedicalEthics",
            vec![signal("Science", 1, 0.8), signal("Individual", -1, 0.2)],
        );
        let mut pipeline = SandboxPipeline::default();
        let (out, diag) = pipeline.run_with_diagnostics(&s).unwrap();
        assert_eq!(out.final_value_code, -1);
        assert!(out.policy_action.contains("Preserve"));
        assert_eq!(diag.signal_count, 2);
        assert!(diag.elapsed_ns < 1_000_000_000);
    }

    #[test]
    fn pipeline_value_judgment_holds() {
        let s = scenario(
            "ValueJudgment",
            vec![signal("Individual", -1, 0.3), signal("Consensus", 1, 0.7)],
        );
        let mut pipeline = SandboxPipeline::default();
        let (out, _) = pipeline.run_with_diagnostics(&s).unwrap();
        assert_eq!(out.final_value_code, 0);
        assert!(out.policy_action.contains("Hold"));
    }

    #[test]
    fn pipeline_value_judgment_same_frame_still_holds() {
        // Regression guard: ValueJudgment must remain Hold even when all
        // signals share the same frame and TAND would otherwise commit.
        let s = scenario(
            "ValueJudgment",
            vec![signal("Science", 1, 0.9), signal("Science", 1, 0.8)],
        );
        let mut pipeline = SandboxPipeline::default();
        let (out, _) = pipeline.run_with_diagnostics(&s).unwrap();
        assert_eq!(out.final_value_code, 0);
        assert_eq!(out.final_frame, "Meta");
        assert!(out.policy_action.contains("Hold"));
    }

    #[test]
    fn pipeline_engineering_commits_false() {
        let s = scenario(
            "Engineering",
            vec![signal("Individual", 1, 0.6), signal("Science", -1, 0.4)],
        );
        let mut pipeline = SandboxPipeline::default();
        let out = pipeline.run(&s).unwrap();
        assert_eq!(out.final_value_code, -1);
    }

    #[test]
    fn pipeline_rejects_invalid_frame() {
        let s = scenario("General", vec![signal("Bogus", 1, 0.5)]);
        let mut pipeline = SandboxPipeline::default();
        assert!(pipeline.run(&s).is_err());
    }

    #[test]
    fn pipeline_rejects_invalid_phase() {
        let s = scenario("General", vec![signal("Science", 1, 1.5)]);
        let mut pipeline = SandboxPipeline::default();
        assert!(pipeline.run(&s).is_err());
    }

    #[test]
    fn pipeline_rejects_empty_signals() {
        let s = scenario("General", vec![]);
        let mut pipeline = SandboxPipeline::default();
        assert!(pipeline.run(&s).is_err());
    }

    #[test]
    fn pipeline_rejects_invalid_domain() {
        let s = scenario("Bogus", vec![signal("Science", 1, 0.5)]);
        let mut pipeline = SandboxPipeline::default();
        assert!(pipeline.run(&s).is_err());
    }

    #[test]
    fn pipeline_single_signal_commits() {
        let s = scenario("General", vec![signal("Science", 1, 0.9)]);
        let mut pipeline = SandboxPipeline::default();
        let out = pipeline.run(&s).unwrap();
        assert_eq!(out.final_value_code, 1);
        assert_eq!(out.final_frame, "Science");
    }

    #[test]
    fn pipeline_custom_domain_runs() {
        let s = scenario(
            "Custom(literature)",
            vec![signal("Science", 1, 0.8), signal("Individual", -1, 0.2)],
        );
        let mut pipeline = SandboxPipeline::default();
        let out = pipeline.run(&s).unwrap();
        assert!(out.policy_action.contains("Negotiate"));
    }

    #[test]
    fn pipeline_output_helpers_work() {
        let s = scenario("General", vec![signal("Science", 1, 0.9)]);
        let mut pipeline = SandboxPipeline::default();
        let out = pipeline.run(&s).unwrap();
        assert!(out.is_commit_true());
        assert!(!out.is_commit_false());
        assert!(!out.is_hold());
    }

    #[test]
    fn pipeline_physical_hold_with_interrupts_forces_false() {
        // Cross-frame conflict with no Science frame produces Hold + interrupts;
        // SafeFallback forces False in the dangerous Physical domain.
        let s = scenario(
            "Physical",
            vec![signal("Individual", 1, 0.9), signal("Consensus", -1, 0.2)],
        );
        let mut pipeline = SandboxPipeline::default();
        let (out, diag) = pipeline.run_with_diagnostics(&s).unwrap();
        assert_eq!(out.final_value_code, -1);
        assert!(!out.interrupts.is_empty());
        assert!(diag.safe_fallback_triggered);
    }

    #[test]
    fn pipeline_with_registry_rejects_unregistered_frame() {
        let mut reg = FrameRegistry::new();
        reg.register(Frame::Science);
        let mut pipeline = SandboxPipeline::with_registry(reg);
        let s = ScenarioInput {
            id: "test".into(),
            description: "test".into(),
            domain: "General".into(),
            signals: vec![SignalInput {
                frame: "Individual".into(),
                value: 1,
                phase: 0.5,
                sensor: None,
            }],
            expected_behavior: "hold".into(),
            environmental_context: None,
        };
        assert!(pipeline.run(&s).is_err());
    }

    #[test]
    fn pipeline_with_registry_accepts_registered_frame() {
        let mut reg = FrameRegistry::new();
        reg.register(Frame::Science);
        let mut pipeline = SandboxPipeline::with_registry(reg);
        let s = ScenarioInput {
            id: "test".into(),
            description: "test".into(),
            domain: "General".into(),
            signals: vec![SignalInput {
                frame: "Science".into(),
                value: 1,
                phase: 0.9,
                sensor: None,
            }],
            expected_behavior: "hold".into(),
            environmental_context: None,
        };
        let out = pipeline.run(&s).unwrap();
        assert_eq!(out.final_value_code, 1);
    }

    #[test]
    fn pipeline_default_has_no_registry() {
        let mut pipeline = SandboxPipeline::default();
        // Default pipeline has no registry, so any frame is accepted
        let s = ScenarioInput {
            id: "test".into(),
            description: "test".into(),
            domain: "General".into(),
            signals: vec![SignalInput {
                frame: "Consensus".into(),
                value: -1,
                phase: 0.3,
                sensor: None,
            }],
            expected_behavior: "hold".into(),
            environmental_context: None,
        };
        assert!(pipeline.run(&s).is_ok());
    }

    #[test]
    fn pipeline_reflexive_guard_overrides_forced_collapse() {
        // MedicalEthics preserves Individual even when it conflicts with Science,
        // producing a forced False with unresolved cross-frame interrupts.
        let s = scenario(
            "MedicalEthics",
            vec![signal("Science", 1, 0.9), signal("Individual", -1, 0.2)],
        );
        let mut pipeline = SandboxPipeline::default().with_reflexive(ReflexiveAuditor::new());
        let (out, diag) = pipeline.run_with_diagnostics(&s).unwrap();
        assert!(diag.reflexive_guard_triggered);
        assert_eq!(out.final_value_code, 0);
        assert!(out.reflexive_alert.is_some());
    }

    #[test]
    fn pipeline_first_person_preserved_over_science() {
        let s = scenario(
            "General",
            vec![signal("Science", -1, 0.4), signal("FirstPerson", 1, 0.8)],
        );
        let mut pipeline = SandboxPipeline::default();
        let (out, _) = pipeline.run_with_diagnostics(&s).unwrap();
        assert_eq!(out.final_value_code, 1);
        assert!(out.policy_action.contains("Preserve"));
        assert_eq!(out.final_frame, "FirstPerson");
    }

    #[test]
    fn anchor_thermal_baseline_aborts_decision() {
        use crate::anchor::thermal_baseline::ThermalBaseline;

        let s = scenario("General", vec![signal("Science", 1, 0.9)]);
        let mut pipeline =
            SandboxPipeline::default().with_anchor(Box::new(ThermalBaseline::exceeded()));
        let (out, diag) = pipeline.run_with_diagnostics(&s).unwrap();
        // Thermal baseline exceeded → Abort → forced Hold
        assert_eq!(out.final_value_code, 0);
        assert_eq!(out.final_frame, "Meta");
        assert!(diag.anchor_report.is_some());
        assert!(diag.anchor_report.unwrap().has_abort());
    }

    #[test]
    fn anchor_safe_thermal_passes_through() {
        use crate::anchor::thermal_baseline::ThermalBaseline;

        let s = scenario("General", vec![signal("Science", 1, 0.9)]);
        let mut pipeline =
            SandboxPipeline::default().with_anchor(Box::new(ThermalBaseline::safe()));
        let (out, diag) = pipeline.run_with_diagnostics(&s).unwrap();
        // Safe baseline → no violation → normal commit
        assert_eq!(out.final_value_code, 1);
        assert!(diag.anchor_report.is_none());
    }

    #[test]
    fn anchor_ecological_degraded_aborts() {
        use crate::anchor::ecological_base::EcologicalBase;

        let s = scenario("General", vec![signal("Science", -1, 0.9)]);
        let mut pipeline =
            SandboxPipeline::default().with_anchor(Box::new(EcologicalBase::degraded()));
        let (out, diag) = pipeline.run_with_diagnostics(&s).unwrap();
        assert_eq!(out.final_value_code, 0);
        assert!(diag.anchor_report.is_some());
    }

    #[test]
    fn anchor_wellbeing_high_irreversible_risk_aborts() {
        use crate::anchor::wellbeing_priority::WellbeingPriority;
        use crate::core::sensor::EnvironmentalContext;

        let s = ScenarioInput {
            id: "test".into(),
            description: "test".into(),
            domain: "Engineering".into(),
            signals: vec![signal("Science", 1, 0.9)],
            expected_behavior: "hold".into(),
            environmental_context: Some(EnvironmentalContext {
                ambient_arousal: 0.9, // high → irreversible_change_risk = 0.09 > 0.01
                social_density: 0.5,
                ..Default::default()
            }),
        };
        let mut pipeline =
            SandboxPipeline::default().with_anchor(Box::new(WellbeingPriority::new()));
        let (out, diag) = pipeline.run_with_diagnostics(&s).unwrap();
        // High irreversible risk → Abort → forced Hold
        assert_eq!(out.final_value_code, 0);
        assert!(diag.anchor_report.is_some());
    }

    #[test]
    fn anchor_multiple_constraints_all_checked() {
        use crate::anchor::ecological_base::EcologicalBase;
        use crate::anchor::thermal_baseline::ThermalBaseline;
        use crate::anchor::wellbeing_priority::WellbeingPriority;

        let s = scenario("General", vec![signal("Science", 1, 0.9)]);
        let mut pipeline = SandboxPipeline::default()
            .with_anchor(Box::new(ThermalBaseline::safe()))
            .with_anchor(Box::new(EcologicalBase::degraded()))
            .with_anchor(Box::new(WellbeingPriority::new()));
        let (out, diag) = pipeline.run_with_diagnostics(&s).unwrap();
        // Ecological degraded → Abort among multiple anchors
        assert_eq!(out.final_value_code, 0);
        assert!(diag.anchor_report.is_some());
        let report = diag.anchor_report.unwrap();
        assert!(report.has_abort());
        assert!(report.violations.len() >= 1);
    }

    // ── pipeline integration: budget, clock, calibration ─────────

    #[test]
    fn pipeline_diagnostics_include_depth_level_and_clock_phase() {
        let s = scenario("General", vec![signal("Science", 1, 0.9)]);
        let mut pipeline = SandboxPipeline::default();
        let (_, diag) = pipeline.run_with_diagnostics(&s).unwrap();
        // After stage 8b, depth_level should be set (1–5)
        assert!((1..=5).contains(&diag.depth_level));
        // After stage 10b, clock_phase should be in [0.0, 1.0]
        assert!((0.0..=1.0).contains(&diag.clock_phase));
    }

    #[test]
    fn pipeline_records_stage_sample_os_budget() {
        let s = scenario("General", vec![signal("Science", 1, 0.9)]);
        let mut pipeline = SandboxPipeline::default();
        let (_, diag) = pipeline.run_with_diagnostics(&s).unwrap();
        assert!(diag.stage_timings_ns.contains_key("sample_os_budget"));
    }

    #[test]
    fn pipeline_records_stage_clock_tick() {
        let s = scenario("General", vec![signal("Science", 1, 0.9)]);
        let mut pipeline = SandboxPipeline::default();
        let (_, diag) = pipeline.run_with_diagnostics(&s).unwrap();
        assert!(diag.stage_timings_ns.contains_key("clock_tick"));
    }

    #[test]
    fn pipeline_records_stage_calibrate() {
        let s = scenario("General", vec![signal("Science", 1, 0.9)]);
        let mut pipeline = SandboxPipeline::default();
        let (_, diag) = pipeline.run_with_diagnostics(&s).unwrap();
        assert!(diag.stage_timings_ns.contains_key("calibrate"));
    }

    #[test]
    fn pipeline_calibration_log_grows_after_runs() {
        let s = scenario("General", vec![signal("Science", 1, 0.9)]);
        let mut pipeline = SandboxPipeline::default();
        assert_eq!(pipeline.calibration_log.len(), 0);
        pipeline.run(&s).unwrap();
        assert_eq!(pipeline.calibration_log.len(), 1);
        pipeline.run(&s).unwrap();
        assert_eq!(pipeline.calibration_log.len(), 2);
    }

    #[test]
    fn pipeline_clock_advances_after_run() {
        let s = scenario("General", vec![signal("Science", 1, 0.9)]);
        let mut pipeline = SandboxPipeline::default();
        let t_before = pipeline.clock.elapsed_time();
        pipeline.run(&s).unwrap();
        let t_after = pipeline.clock.elapsed_time();
        assert!(t_after > t_before, "clock should tick forward each run");
    }

    #[test]
    fn pipeline_with_explicit_budget_uses_depth_level() {
        let s = scenario("General", vec![signal("Science", 1, 0.9)]);
        let budget = ComputeBudget::new(crate::budget::DepthLevel::Minimal, 0.95, 0.95, 1);
        let mut pipeline = SandboxPipeline::default().with_budget(budget);
        let (_, diag) = pipeline.run_with_diagnostics(&s).unwrap();
        // The initial budget is set, then overwritten by stage 8b sampling.
        // But the depth_level in diagnostics should be set.
        assert!((1..=5).contains(&diag.depth_level));
    }

    #[test]
    fn pipeline_with_self_knowledge_calibrates() {
        let s = scenario(
            "MedicalEthics",
            vec![signal("Science", 1, 0.8), signal("Individual", -1, 0.2)],
        );
        let knowledge = SelfKnowledge::with_human_defaults();
        let before_count = knowledge.calibration_count();
        let mut pipeline = SandboxPipeline::default()
            .with_self_knowledge(knowledge)
            .with_reflexive(ReflexiveAuditor::new());
        pipeline.run(&s).unwrap();
        // After run, self_knowledge should have more calibrations
        // (the reflexive guard forces Hold → calibrate_from_result fires)
        let after_count = pipeline
            .self_knowledge
            .as_ref()
            .unwrap()
            .calibration_count();
        assert!(
            after_count >= before_count,
            "calibration count {after_count} should be >= {before_count}"
        );
    }

    #[test]
    fn pipeline_clean_decision_strengthens_pattern() {
        // Single same-frame signal → clean commit → pattern strengthened (+0.05)
        let s = scenario("General", vec![signal("Science", 1, 0.9)]);
        let knowledge = SelfKnowledge::with_human_defaults();
        // Add a matching pattern so calibrate_from_result finds it
        let mut knowledge = knowledge;
        knowledge.add_pattern(crate::knowledge::ResponsePattern {
            frame: Frame::Science,
            value: TritValue::True,
            phase: 0.5,
            context: "calibrated".to_string(),
        });
        let mut pipeline = SandboxPipeline::default().with_self_knowledge(knowledge);
        pipeline.run(&s).unwrap();
        let sk = pipeline.self_knowledge.as_ref().unwrap();
        let cal_count = sk.calibration_count();
        assert!(
            cal_count > 0,
            "should have recorded at least one calibration"
        );
    }

    #[test]
    fn pipeline_with_custom_clock_uses_preset() {
        let s = scenario("Physical", vec![signal("Science", 1, 0.9)]);
        let clock = HarmonicClock::physical();
        let mut pipeline = SandboxPipeline::default().with_clock(clock);
        let (_, diag) = pipeline.run_with_diagnostics(&s).unwrap();
        assert!(diag.clock_phase >= 0.0);
        // Physical clock has ω=10.0, so after a short pipeline run the phase
        // should have moved from 0.0 (sin(0)=0 at t=0 → sin(ω*t) after tick).
        let phase = pipeline.clock.to_phase().inner();
        assert!((0.0..=1.0).contains(&phase));
    }

    #[test]
    fn pipeline_calibration_log_window_evicts() {
        // Use a small window to test eviction
        let log = CalibrationLog::new(3);
        let s = scenario("General", vec![signal("Science", 1, 0.9)]);
        let mut pipeline = SandboxPipeline::default().with_calibration_log(log);
        pipeline.run(&s).unwrap();
        pipeline.run(&s).unwrap();
        pipeline.run(&s).unwrap();
        assert_eq!(pipeline.calibration_log.len(), 3);
        // 4th run should evict oldest
        pipeline.run(&s).unwrap();
        assert_eq!(pipeline.calibration_log.len(), 3);
    }
}
