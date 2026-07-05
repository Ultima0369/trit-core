use std::time::Instant;

use tracing::{debug, error, info, instrument, trace, warn};

use crate::adapters::bandwidth_scheduler::AttentionScheduler;
use crate::adapters::reflexive_audit::{ReflexiveAlert, ReflexiveAuditor};
use crate::adapters::self_knowledge::SelfKnowledge;
use crate::adapters::{AttentionCmd, ShiftTarget};
use crate::anchor::{check_all, AnchorConstraint};
use crate::budget::ComputeBudget;
use crate::calibration::{CalibrationEntry, CalibrationLog};
use crate::clock::HarmonicClock;
use crate::core::frame::Frame;
use crate::core::hold::{HoldState, HolderConfig};
use crate::core::phase::Phase;
use crate::core::value::TritValue;
use crate::core::word::TritWord;
use crate::core::TernaryAlgebra;
use crate::feedback::FeedbackLoop;
use crate::meta::SafeFallback;
use crate::meta::{ArbitrationResult, Domain};
use crate::sandbox::diagnostic::SandboxDiagnostics;
use crate::sandbox::error::SandboxError;
use crate::sandbox::input::{ScenarioInput, SignalInput};
use crate::sandbox::output::SandboxOutput;
use crate::sandbox::validate::{sanitize_log_field, validate_scenario};

/// Standard sandbox pipeline: TAND cascade → policy arbitration → SafeFallback.
///
/// Mind-engineering extensions (reflexive audit, attention scheduling,
/// self-knowledge) are opt-in via builder methods and do not change the
/// default behavior unless explicitly enabled.
pub struct SandboxPipeline {
    pub(crate) dry_run: bool,
    pub(crate) decision_engine: crate::core::decision_engine::DecisionEngine,
    pub(crate) attention: Option<AttentionScheduler>,
    pub(crate) self_knowledge: Option<SelfKnowledge>,
    pub(crate) holder_config: Option<HolderConfig>,
    pub(crate) trace_phase: bool,
    pub(crate) hold_final: bool,
    /// Anchor constraints checked before every decision.
    pub(crate) anchor_constraints: Vec<Box<dyn AnchorConstraint>>,
    /// Hardware-aware compute budget for depth gating.
    pub(crate) budget: ComputeBudget,
    /// Harmonic clock for temporal context.
    pub(crate) clock: HarmonicClock,
    /// Calibration log for feedback-driven learning.
    pub(crate) calibration_log: CalibrationLog,
    /// Feedback loop for practice testing (Layer 5).
    pub(crate) feedback: Option<FeedbackLoop>,
}

impl std::fmt::Debug for SandboxPipeline {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SandboxPipeline")
            .field("dry_run", &self.dry_run)
            .field("decision_engine", &"DecisionEngine { .. }")
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
    /// Create a new pipeline.
    pub fn new() -> Self {
        Self {
            dry_run: false,
            decision_engine: crate::core::decision_engine::DecisionEngine::new(),
            attention: None,
            self_knowledge: None,
            holder_config: None,
            trace_phase: false,
            hold_final: false,
            anchor_constraints: Vec::new(),
            budget: ComputeBudget::conservative(),
            clock: HarmonicClock::deliberative(),
            calibration_log: CalibrationLog::default(),
            feedback: None,
        }
    }

    /// Enable dry-run mode: build trits and run TAND, but skip arbitration and SafeFallback.
    pub fn with_dry_run(mut self, dry_run: bool) -> Self {
        self.dry_run = dry_run;
        self
    }

    /// Inject a custom SafeFallback configuration.
    pub fn with_safe_fallback(mut self, safe_fallback: SafeFallback) -> Self {
        self.decision_engine = self.decision_engine.with_safe_fallback(safe_fallback);
        self
    }

    /// Attach a reflexive auditor.
    pub fn with_reflexive(mut self, auditor: ReflexiveAuditor) -> Self {
        self.decision_engine = self.decision_engine.with_reflexive(auditor);
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
        self.decision_engine = self.decision_engine.with_trace_phase(enabled);
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

    /// Attach a feedback loop for practice testing (Layer 5).
    pub fn with_feedback(mut self, feedback: FeedbackLoop) -> Self {
        self.feedback = Some(feedback);
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

        // Stages 1–3: validate, build policy, build trits
        let trits = self.stage_validate_and_build(scenario, &mut diagnostics)?;

        // Stages 5–8: TAND cascade → arbitration → reflexive guard → SafeFallback
        let stage_start = Instant::now();
        let decision_result = self.stage_decide(&scenario.domain, &trits)?;
        diagnostics.record_stage("t_and_n", stage_start);

        // Record arbitration result
        diagnostics.record_policy_action(&decision_result.policy_action);
        diagnostics.record_stage("arbitrate", Instant::now());

        // Record interrupts
        diagnostics.record_interrupts(&decision_result.interrupts);

        // Record reflexive guard
        if decision_result.reflexive_alert.is_some() {
            diagnostics.mark_reflexive_guard();
        }
        diagnostics.record_stage("reflexive_guard", Instant::now());

        // Record SafeFallback
        if decision_result.safe_fallback_triggered {
            diagnostics.mark_safe_fallback();
        }
        diagnostics.interrupts = decision_result.interrupts.clone();
        diagnostics.record_stage("safe_fallback", Instant::now());

        // Stage 8b: sample OS → ComputeBudget.depth_level
        self.stage_sample_budget(&mut diagnostics);

        // Stages 9–10: attention scheduling, self-knowledge inference
        let final_word = decision_result.final_word;
        let policy_action_str = format!("{}", decision_result.policy_action);
        let reflexive_alert = decision_result.reflexive_alert;
        self.stage_optional_extensions(&trits, &final_word, &mut diagnostics);

        // Stage 10b: clock tick — advance the harmonic oscillator
        self.stage_tick_clock(&mut diagnostics);

        // Stage 11: phase trace
        let mut final_word = final_word;
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

        // Stage 14: feedback loop (Layer 5)
        self.stage_feedback_loop(scenario, &output, &mut diagnostics);

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

    /// Stages 1–3: validate scenario, build policy, build trits.
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

        // Stage 2: validate domain (parse-only, full policy is built in DecisionEngine)
        let stage_start = Instant::now();
        let _domain: Domain = scenario.domain.parse().map_err(|e| {
            error!(error = %e, category = "domain", "domain parse failed");
            SandboxError::InvalidDomain(format!("{}", e))
        })?;
        diagnostics.record_stage("build_policy", stage_start);
        info!(domain = %scenario.domain, "domain validated");

        // Stage 3: build trits
        let stage_start = Instant::now();
        let trits = build_trits(&scenario.signals).map_err(|e| {
            error!(error = %e, category = %e.category_name(), "signal conversion failed");
            e
        })?;
        diagnostics.record_inputs(&trits);
        diagnostics.record_stage("build_trits", stage_start);
        debug!(signal_count = trits.len(), "trits built");

        Ok(trits)
    }

    /// Stages 5–8: delegate to DecisionEngine for TAND → arbitration → guard → SafeFallback.
    fn stage_decide(
        &mut self,
        domain_str: &str,
        trits: &[TritWord],
    ) -> Result<crate::core::decision_engine::DecisionResult, SandboxError> {
        if self.dry_run {
            // In dry-run mode we still run the TAND cascade so that callers can
            // see the raw ternary conflict (e.g. cross-frame → Hold), but we skip
            // domain arbitration, reflexive guard, and SafeFallback.
            let (current, interrupts) = TernaryAlgebra::t_and_n(trits);
            return Ok(crate::core::decision_engine::DecisionResult {
                final_word: current,
                policy_action: ArbitrationResult::DryRun,
                interrupts,
                reflexive_alert: None,
                safe_fallback_triggered: false,
            });
        }

        let domain: Domain = domain_str
            .parse()
            .map_err(|e| SandboxError::InvalidDomain(format!("{}", e)))?;

        self.decision_engine.decide(trits, &domain)
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

        // Stage 9: attention scheduling (with clock phase modulation)
        let stage_start = Instant::now();
        if let Some(ref mut scheduler) = self.attention {
            let cmd = scheduler.suggest_with_budget(&self.budget, trits);
            // Modulate with clock phase: near peaks bias toward ShiftTo,
            // near troughs bias toward HoldCurrent.
            let modulated = modulate_attention_with_clock_phase(cmd, self.clock.to_phase().inner());
            diagnostics.record_attention_cmd(&modulated);
            if matches!(modulated, AttentionCmd::HoldCurrent) {
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
        let preview = crate::anchor::build_decision_preview(scenario, &final_word, None);
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

    /// Stage 14: feedback loop — practice test the decision against a proxy
    /// environment (Layer 5).
    ///
    /// Runs after calibration. If a feedback loop is configured, it predicts
    /// the expected consequence, compares against the actual decision, and
    /// emits a FeedbackSignal if correction is needed.
    fn stage_feedback_loop(
        &mut self,
        _scenario: &ScenarioInput,
        output: &SandboxOutput,
        diagnostics: &mut SandboxDiagnostics,
    ) {
        let stage_start = Instant::now();
        if let Some(ref mut feedback) = self.feedback {
            let signal = feedback.run_cycle(output);
            if let Some(ref sig) = signal {
                info!(
                    deviation_delta = sig.deviation_delta,
                    recommended_scenario = ?sig.recommended_scenario,
                    "feedback loop: correction triggered"
                );
                diagnostics.record_feedback_signal(sig.clone());

                // Calibrate self-knowledge with the feedback signal
                if let Some(ref mut knowledge) = self.self_knowledge {
                    let frame: crate::core::frame::Frame = output
                        .final_frame
                        .parse()
                        .unwrap_or(crate::core::frame::Frame::Meta);
                    knowledge.calibrate_from_result(
                        frame,
                        crate::core::value::TritValue::from(output.final_value_code),
                        output.final_phase_raw,
                        diagnostics.interrupt_count,
                    );
                }
            } else {
                debug!("feedback loop: decision matched proxy prediction");
            }
        }
        diagnostics.record_stage("feedback_loop", stage_start);
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
            reflexive_alert: reflexive_alert
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

    /// Access the clock's elapsed time (for integration tests).
    pub fn clock_elapsed(&self) -> f64 {
        self.clock.elapsed_time()
    }

    /// Access the clock's phase (for integration tests).
    pub fn clock_phase_value(&self) -> f64 {
        self.clock.to_phase().inner()
    }

    /// Access the calibration log length (for integration tests).
    pub fn calibration_log_len(&self) -> usize {
        self.calibration_log.len()
    }

    /// Access the self-knowledge model (for integration tests).
    pub fn self_knowledge_ref(&self) -> Option<&SelfKnowledge> {
        self.self_knowledge.as_ref()
    }
}

/// Parse a serialized attention command string back to an `AttentionCmd`.
fn parse_attention_cmd(s: &str) -> Option<AttentionCmd> {
    use crate::adapters::ShiftTarget;
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

/// Modulate the attention command based on clock phase.
///
/// Near phase peaks (0.8–1.0): bias toward `ShiftTo(ConflictTrace)` if the
/// scheduler would otherwise hold or continue — the system is at peak
/// temporal energy and should reconsider its focus.
/// Near phase troughs (0.0–0.2): bias toward `HoldCurrent` — the system
/// is at minimum temporal energy and should conserve attention.
/// In between (0.2–0.8): pass through the scheduler's original command.
pub fn modulate_attention_with_clock_phase(cmd: AttentionCmd, clock_phase: f64) -> AttentionCmd {
    if clock_phase > 0.8 {
        match &cmd {
            AttentionCmd::HoldCurrent | AttentionCmd::Continue => {
                trace!(
                    clock_phase,
                    original_cmd = ?cmd,
                    "clock phase peak → shifting to ConflictTrace"
                );
                AttentionCmd::ShiftTo(ShiftTarget::ConflictTrace)
            }
            _ => cmd,
        }
    } else if clock_phase < 0.2 {
        match &cmd {
            AttentionCmd::ShiftTo(_) | AttentionCmd::Continue => {
                trace!(
                    clock_phase,
                    original_cmd = ?cmd,
                    "clock phase trough → holding current"
                );
                AttentionCmd::HoldCurrent
            }
            _ => cmd,
        }
    } else {
        cmd
    }
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
