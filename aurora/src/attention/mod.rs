//! Attention guidance layer — minimal closed loop for M0.
//!
//! [`AttentionSession`] tracks reminders, user responses, and computes
//! the Attention Sovereignty Index (ASI). This is the "注意力调度最小闭环"
//! required by M0.
//!
//! [`AttentionManager`] wraps the existing Trit-Core [`AttentionScheduler`]
//! and feeds its output into the session tracker.

use chrono::Utc;
use truncore::adapters::bandwidth_scheduler::AttentionScheduler;
use truncore::adapters::AttentionCmd;
use truncore::core::TritWord;

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

    /// Access the underlying session (for rendering).
    pub fn session(&self) -> &AttentionSession {
        &self.session
    }

    /// Mutable access to the session.
    pub fn session_mut(&mut self) -> &mut AttentionSession {
        &mut self.session
    }
}
