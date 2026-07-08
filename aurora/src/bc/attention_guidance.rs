//! AttentionGuidance BC — attention reminders, user responses, and ASI tracking.
//!
//! # Aggregate root
//! [`AttentionSession`] — tracks reminders, responses, and computes ASI.
//!
//! # Migration note
//! The core data structures were migrated from `aurora::attention` (M0)
//! to this BC module (M1). The old module re-exports for backward compat.

use chrono::Utc;
use trit_core::adapters::bandwidth_scheduler::AttentionScheduler;
use trit_core::adapters::AttentionCmd;
use trit_core::core::TritWord;

// ── Entities ──────────────────────────────────────────────────────────────

/// A user's response to an attention reminder.
#[derive(Debug, Clone, PartialEq)]
pub enum UserResponse {
    /// User actively shifted attention to the suggested target.
    ShiftedTo(String),
    /// User overrode a Hold by choosing a specific frame.
    OverrodeHold { chosen_frame: String },
    /// User saw the reminder but took no action.
    Ignored,
    /// User explicitly dismissed the reminder.
    Dismissed,
}

/// A single attention reminder event.
#[derive(Debug, Clone)]
pub struct ReminderRecord {
    pub timestamp: chrono::DateTime<Utc>,
    pub action: String,
    pub target: String,
    pub rationale: String,
    pub user_response: Option<UserResponse>,
}

/// A target for attention shifting.
#[derive(Debug, Clone)]
pub struct ShiftTarget {
    /// The domain or frame to shift attention to.
    pub target: String,
    /// Priority level (higher = more urgent).
    pub priority: u8,
}

/// Attention Sovereignty Index (ASI) metric.
#[derive(Debug, Clone, Copy)]
pub struct ASIMetric {
    /// ASI value in [0.0, 1.0].
    pub value: f64,
    /// Number of reminders issued.
    pub reminder_count: usize,
    /// Number of active shifts by the user.
    pub active_shift_count: usize,
}

// ── Aggregate root ────────────────────────────────────────────────────────

/// Tracks attention reminders and user responses across a session.
///
/// Computes the Attention Sovereignty Index (ASI):
///
/// ```text
/// ASI = (user_active_shift_count) / max(reminder_count, 1) * phase_recovery_coefficient
/// ```
///
/// In M0, `phase_recovery_coefficient` defaults to 1.0 (not yet measured).
/// M1 will integrate HarmonicClock phase data.
#[derive(Debug, Clone)]
pub struct AttentionSession {
    session_id: String,
    reminders: Vec<ReminderRecord>,
    user_active_shift_count: usize,
    phase_recovery_coefficient: f64,
}

impl AttentionSession {
    /// Create a new attention session.
    pub fn new(session_id: impl Into<String>) -> Self {
        Self {
            session_id: session_id.into(),
            reminders: Vec::new(),
            user_active_shift_count: 0,
            phase_recovery_coefficient: 1.0, // M0 default
        }
    }

    /// Record a system-generated attention reminder.
    pub fn record_reminder(
        &mut self,
        action: impl Into<String>,
        target: impl Into<String>,
        rationale: impl Into<String>,
    ) {
        self.reminders.push(ReminderRecord {
            timestamp: Utc::now(),
            action: action.into(),
            target: target.into(),
            rationale: rationale.into(),
            user_response: None,
        });
    }

    /// Record the user's response to the most recent reminder.
    pub fn record_user_response(&mut self, response: UserResponse) {
        if let Some(last) = self.reminders.last_mut() {
            if matches!(
                response,
                UserResponse::ShiftedTo(_) | UserResponse::OverrodeHold { .. }
            ) {
                self.user_active_shift_count += 1;
            }
            last.user_response = Some(response);
        }
    }

    /// Attention Sovereignty Index in [0.0, 1.0].
    ///
    /// Higher = user is more actively managing their own attention.
    pub fn asi(&self) -> f64 {
        let denominator = self.reminders.len().max(1) as f64;
        (self.user_active_shift_count as f64) / denominator * self.phase_recovery_coefficient
    }

    /// The ASI as a structured metric.
    pub fn asi_metric(&self) -> ASIMetric {
        ASIMetric {
            value: self.asi(),
            reminder_count: self.reminders.len(),
            active_shift_count: self.user_active_shift_count,
        }
    }

    /// Total number of reminders issued.
    pub fn reminder_count(&self) -> usize {
        self.reminders.len()
    }

    /// Number of times the user actively shifted or overrode.
    pub fn user_active_shift_count(&self) -> usize {
        self.user_active_shift_count
    }

    /// All reminder records (for rendering).
    pub fn reminders(&self) -> &[ReminderRecord] {
        &self.reminders
    }

    /// Session identifier.
    pub fn session_id(&self) -> &str {
        &self.session_id
    }
}

// ── M0 implementation ─────────────────────────────────────────────────────

/// Manages the attention scheduling loop for a session.
///
/// Wraps Trit-Core's [`AttentionScheduler`] and feeds its commands
/// into an [`AttentionSession`] for tracking.
pub struct AttentionManager {
    scheduler: AttentionScheduler,
    session: AttentionSession,
}

impl AttentionManager {
    /// Create a new attention manager.
    pub fn new(session_id: impl Into<String>) -> Self {
        Self {
            scheduler: AttentionScheduler::default(),
            session: AttentionSession::new(session_id),
        }
    }

    /// Run one cycle: feed signals to the scheduler, record any reminder.
    ///
    /// Returns the scheduler's command if a non-Continue action was suggested.
    pub fn run_cycle(&mut self, signals: &[TritWord]) -> Option<AttentionCmd> {
        let cmd = self.scheduler.suggest_reprioritization(signals);

        match &cmd {
            AttentionCmd::Continue => None,
            AttentionCmd::HoldCurrent => {
                self.session
                    .record_reminder("HoldCurrent", "Meta", "带宽不足，建议暂停当前处理");
                Some(cmd)
            }
            AttentionCmd::ShiftTo(target) => {
                let target_str = format!("{:?}", target);
                self.session
                    .record_reminder("ShiftTo", &target_str, "注意力调度建议切换焦点");
                Some(cmd)
            }
            AttentionCmd::Recalibrate => {
                self.session.record_reminder(
                    "Recalibrate",
                    "Meta",
                    "连续 Hold 超过阈值，建议重新校准",
                );
                Some(cmd)
            }
        }
    }

    /// Record the user's response to the last reminder.
    pub fn respond(&mut self, response: UserResponse) {
        self.session.record_user_response(response);
    }

    /// Current ASI value.
    pub fn asi(&self) -> f64 {
        self.session.asi()
    }

    /// Access the underlying session.
    pub fn session(&self) -> &AttentionSession {
        &self.session
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use trit_core::core::Frame;

    #[test]
    fn new_session_starts_with_zero_asi() {
        let session = AttentionSession::new("test");
        assert_eq!(session.asi(), 0.0);
        assert_eq!(session.reminder_count(), 0);
        assert_eq!(session.user_active_shift_count(), 0);
    }

    #[test]
    fn user_shift_increases_active_count_and_asi() {
        let mut session = AttentionSession::new("test");

        session.record_reminder("ShiftTo", "ConflictTrace", "跨 Frame 冲突");
        session.record_user_response(UserResponse::ShiftedTo("ConflictTrace".into()));

        assert_eq!(session.reminder_count(), 1);
        assert_eq!(session.user_active_shift_count(), 1);
        assert!(session.asi() > 0.0);
    }

    #[test]
    fn user_ignore_does_not_increase_asi() {
        let mut session = AttentionSession::new("test");

        session.record_reminder("ShiftTo", "ConflictTrace", "跨 Frame 冲突");
        session.record_user_response(UserResponse::Ignored);

        assert_eq!(session.reminder_count(), 1);
        assert_eq!(session.user_active_shift_count(), 0);
        assert_eq!(session.asi(), 0.0);
    }

    #[test]
    fn user_override_hold_increases_active_count() {
        let mut session = AttentionSession::new("test");

        session.record_reminder("HoldCurrent", "Meta", "冲突悬置");
        session.record_user_response(UserResponse::OverrodeHold {
            chosen_frame: "Individual".into(),
        });

        assert_eq!(session.user_active_shift_count(), 1);
        assert!(session.asi() > 0.0);
    }

    #[test]
    fn mixed_responses_compute_correct_asi() {
        let mut session = AttentionSession::new("test");

        // Shift
        session.record_reminder("ShiftTo", "ConflictTrace", "c1");
        session.record_user_response(UserResponse::ShiftedTo("ConflictTrace".into()));

        // Ignore
        session.record_reminder("ShiftTo", "Body", "c2");
        session.record_user_response(UserResponse::Ignored);

        // Override
        session.record_reminder("HoldCurrent", "Meta", "c3");
        session.record_user_response(UserResponse::OverrodeHold {
            chosen_frame: "Individual".into(),
        });

        assert_eq!(session.reminder_count(), 3);
        assert_eq!(session.user_active_shift_count(), 2);
        // ASI = 2/3 ≈ 0.667
        assert!(session.asi() > 0.5 && session.asi() < 0.7);
    }

    #[test]
    fn asi_metric_is_consistent() {
        let mut session = AttentionSession::new("test");
        session.record_reminder("ShiftTo", "Body", "test");
        session.record_user_response(UserResponse::ShiftedTo("Body".into()));

        let metric = session.asi_metric();
        assert_eq!(metric.value, session.asi());
        assert_eq!(metric.reminder_count, session.reminder_count());
        assert_eq!(metric.active_shift_count, session.user_active_shift_count());
    }

    #[test]
    fn attention_manager_runs_cycle_and_tracks_asi() {
        let mut mgr = AttentionManager::new("test");

        // Feed multiple signals to trigger a non-Continue response
        let signals: Vec<TritWord> = (0..10)
            .map(|_| TritWord::tru(Frame::Embodied))
            .chain((0..5).map(|_| TritWord::fals(Frame::Individual)))
            .collect();

        let cmd = mgr.run_cycle(&signals);

        // With many mixed-frame signals, the scheduler should detect something
        // and the reminder count should be tracked
        // (Even if cmd is Continue for small inputs, we verify the cycle ran)
        if cmd.is_some() {
            assert!(mgr.session().reminder_count() > 0);
        }
        // At minimum, the cycle should complete without panicking
    }
}
