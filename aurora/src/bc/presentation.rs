//! Presentation BC — renders internal data to user-visible views.
//!
//! # Aggregate root
//! [`ViewState`] — bundles all data needed for rendering.

use crate::bc::attention_guidance::AttentionSession;
use crate::bc::BcError;

/// HTML-escape a string for safe interpolation into HTML text/attribute context.
///
/// Defense-in-depth: current view fields are engine-generated, but any future
/// field carrying user input (e.g. contact names reaching reminders) must not
/// become a stored-XSS vector. All user-visible strings pass through this.
fn esc(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&#x27;"),
            _ => out.push(c),
        }
    }
    out
}

// ── Entities ──────────────────────────────────────────────────────────────

/// A conflict card displayed in the UI.
#[derive(Debug, Clone)]
pub struct ConflictCard {
    /// The type of conflict (e.g. "FrameMismatch").
    pub conflict_type: String,
    /// Human-readable reason.
    pub reason: String,
    /// The two frames in conflict.
    pub frame_a: String,
    pub frame_b: String,
    /// Whether the user has acknowledged this conflict.
    pub acknowledged: bool,
}

// ── Aggregate root ────────────────────────────────────────────────────────

/// The view state — aggregate root for presentation.
///
/// Bundles all data the renderer needs: decision summary, conflicts,
/// and attention data.
#[derive(Debug, Clone)]
pub struct ViewState {
    /// Decision summary text.
    pub decision_summary: String,
    /// Conflict cards to display.
    pub conflicts: Vec<ConflictCard>,
    /// Attention session data.
    pub attention: AttentionSession,
}

impl ViewState {
    /// Create a new view state with minimal required data.
    pub fn new(decision_summary: String, attention: AttentionSession) -> Self {
        Self {
            decision_summary,
            conflicts: Vec::new(),
            attention,
        }
    }

    /// Add a conflict card.
    pub fn add_conflict(&mut self, card: ConflictCard) {
        self.conflicts.push(card);
    }

    /// Whether any conflicts are present.
    pub fn has_conflicts(&self) -> bool {
        !self.conflicts.is_empty()
    }

    /// Number of unacknowledged conflicts.
    pub fn unacknowledged_count(&self) -> usize {
        self.conflicts.iter().filter(|c| !c.acknowledged).count()
    }
}

// ── M0 implementation ─────────────────────────────────────────────────────

/// M0 HTML renderer — produces self-contained HTML with dark theme.
pub struct AuroraRenderer;

impl AuroraRenderer {
    /// Render the view state as an HTML string.
    pub fn render_html(&self, state: &ViewState) -> String {
        let conflict_html = render_conflict_panel(state);
        let attention_html = render_attention_section(state);
        let reminder_html = render_reminder_history(state);

        format!(
            r#"<!DOCTYPE html>
<html lang="zh-CN">
<head>
    <meta charset="UTF-8">
    <title>Aurora Report</title>
    <style>
        body {{ font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', sans-serif;
               margin: 2rem; background: #0d1117; color: #c9d1d9; }}
        h1 {{ color: #58a6ff; }}
        h2 {{ color: #7ee787; margin-top: 2rem; border-bottom: 1px solid #30363d; padding-bottom: 0.5rem; }}
        table {{ border-collapse: collapse; margin-top: 1rem; width: 100%; max-width: 800px; }}
        th, td {{ border: 1px solid #30363d; padding: 0.6rem 1rem; text-align: left; }}
        th {{ background: #161b22; color: #8b949e; }}
        .conflict-panel {{ background: #161b22; border: 1px solid #d2991d; border-radius: 6px;
                           padding: 1rem; margin-top: 1rem; max-width: 800px; }}
        .conflict-panel h3 {{ color: #d2991d; margin-top: 0; }}
        .no-conflict {{ background: #161b22; border: 1px solid #3fb950; border-radius: 6px;
                        padding: 1rem; margin-top: 1rem; max-width: 800px; }}
        .asi-gauge {{ display: flex; align-items: center; gap: 1rem; margin-top: 1rem; }}
        .asi-bar {{ flex: 1; height: 24px; background: #21262d; border-radius: 12px; overflow: hidden; }}
        .asi-fill {{ height: 100%; border-radius: 12px; transition: width 0.5s; }}
        .asi-value {{ font-size: 1.5rem; font-weight: bold; min-width: 4rem; text-align: right; }}
        .reminder {{ padding: 0.5rem; border-left: 3px solid #30363d; margin: 0.5rem 0; }}
        .reminder.shifted {{ border-left-color: #3fb950; }}
        .reminder.ignored {{ border-left-color: #f85149; }}
        .reminder.pending {{ border-left-color: #d2991d; }}
        footer {{ margin-top: 3rem; color: #484f58; font-size: 0.85rem; }}
    </style>
</head>
<body>
    <h1>Aurora Decision Report</h1>
    <p>{summary}</p>

    {conflict_section}

    {attention_section}

    <h2>Reminder History</h2>
    <table>
        <tr><th>Time</th><th>Action</th><th>Target</th><th>Response</th></tr>
        {reminder_rows}
    </table>

    <footer>
        <p>Generated by Aurora v0.1.0 — M1 BC architecture. 不是指教，是提醒。</p>
    </footer>
</body>
</html>"#,
            summary = esc(&state.decision_summary),
            conflict_section = conflict_html,
            attention_section = attention_html,
            reminder_rows = reminder_html,
        )
    }

    /// Render the view state as a JSON string.
    pub fn render_json(&self, state: &ViewState) -> Result<String, BcError> {
        let json = serde_json::json!({
            "decision_summary": state.decision_summary,
            "conflict_count": state.conflicts.len(),
            "unacknowledged_count": state.unacknowledged_count(),
            "asi": state.attention.asi(),
            "reminder_count": state.attention.reminder_count(),
            "active_shift_count": state.attention.user_active_shift_count(),
        });
        serde_json::to_string_pretty(&json).map_err(|e| BcError::Domain {
            bc: "Presentation".into(),
            message: e.to_string(),
        })
    }
}

fn render_conflict_panel(state: &ViewState) -> String {
    if state.conflicts.is_empty() {
        return r#"<div class="no-conflict">
    <p>✅ No conflict detected — signals are aligned.</p>
</div>"#
            .to_string();
    }

    let cards: Vec<String> = state
        .conflicts
        .iter()
        .map(|c| {
            format!(
                r#"<div class="conflict-panel">
    <h3>⚡ {conflict_type}</h3>
    <p><strong>Reason:</strong> {reason}</p>
    <p><strong>Structure:</strong> {frame_a} vs {frame_b}</p>
    <p style="color: #8b949e; font-style: italic;">
        💡 系统不替你判断哪个更"真实"。这是你的注意力被两个方向拉扯的信号。
    </p>
</div>"#,
                conflict_type = esc(&c.conflict_type),
                reason = esc(&c.reason),
                frame_a = esc(&c.frame_a),
                frame_b = esc(&c.frame_b),
            )
        })
        .collect();

    cards.join("\n")
}

fn render_attention_section(state: &ViewState) -> String {
    let asi = state.attention.asi();
    let asi_pct = (asi * 100.0) as u32;
    let bar_color = if asi > 0.6 {
        "#3fb950"
    } else if asi > 0.3 {
        "#d2991d"
    } else {
        "#f85149"
    };

    format!(
        r#"<h2>Attention Sovereignty Index (ASI)</h2>
<div class="asi-gauge">
    <div class="asi-bar">
        <div class="asi-fill" style="width:{asi_pct}%; background:{bar_color};"></div>
    </div>
    <div class="asi-value" style="color:{bar_color};">{asi:.2}</div>
</div>
<p style="color: #8b949e;">
    ASI = 用户主动调度次数 / 系统提醒次数。越高 = 你越自主。
</p>
<p><strong>Active shifts:</strong> {active} / <strong>Reminders:</strong> {total}</p>"#,
        asi_pct = asi_pct,
        bar_color = bar_color,
        asi = asi,
        active = state.attention.user_active_shift_count(),
        total = state.attention.reminder_count(),
    )
}

fn render_reminder_history(state: &ViewState) -> String {
    state
        .attention
        .reminders()
        .iter()
        .map(|r| {
            let (response_text, row_class) = match &r.user_response {
                Some(crate::bc::attention_guidance::UserResponse::ShiftedTo(t)) => {
                    (format!("Shifted → {}", t), "shifted")
                }
                Some(crate::bc::attention_guidance::UserResponse::OverrodeHold {
                    chosen_frame,
                }) => (format!("Overrode Hold → {}", chosen_frame), "shifted"),
                Some(crate::bc::attention_guidance::UserResponse::Ignored) => {
                    ("Ignored".into(), "ignored")
                }
                Some(crate::bc::attention_guidance::UserResponse::Dismissed) => {
                    ("Dismissed".into(), "ignored")
                }
                None => ("Pending".into(), "pending"),
            };
            format!(
                r#"<tr class="reminder {row_class}">
    <td>{time}</td><td>{action}</td><td>{target}</td><td>{response}</td>
</tr>"#,
                time = esc(&r.timestamp.format("%H:%M:%S").to_string()),
                action = esc(&r.action),
                target = esc(&r.target),
                response = esc(&response_text),
                row_class = row_class,
            )
        })
        .collect::<Vec<_>>()
        .join("\n")
}

// ── Tests ─────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn esc_encodes_html_special_chars() {
        // Defense-in-depth XSS check: a malicious reason string must not reach
        // the HTML output as raw markup.
        let malicious = r#"<script>alert("x")</script>'&"#;
        let escaped = esc(malicious);
        assert!(!escaped.contains('<'));
        assert!(!escaped.contains('>'));
        assert!(!escaped.contains('"'));
        assert!(!escaped.contains('\''));
        assert!(escaped.contains("&amp;"));
        assert!(escaped.contains("&lt;"));
    }

    #[test]
    fn html_render_escapes_conflict_reason() {
        let mut state = make_view_state();
        state.add_conflict(ConflictCard {
            conflict_type: "FrameMismatch".into(),
            reason: r#"<img src=x onerror=alert(1)>"#.into(),
            frame_a: "Embodied".into(),
            frame_b: "Individual".into(),
            acknowledged: false,
        });
        let html = AuroraRenderer.render_html(&state);
        assert!(!html.contains(r#"<img src=x onerror=alert(1)>"#));
        assert!(html.contains("&lt;img"));
    }

    fn make_view_state() -> ViewState {
        ViewState::new("Test decision".into(), AttentionSession::new("test"))
    }

    #[test]
    fn view_state_starts_with_no_conflicts() {
        let state = make_view_state();
        assert!(!state.has_conflicts());
        assert_eq!(state.unacknowledged_count(), 0);
    }

    #[test]
    fn view_state_tracks_conflicts() {
        let mut state = make_view_state();
        state.add_conflict(ConflictCard {
            conflict_type: "FrameMismatch".into(),
            reason: "Embodied vs Individual".into(),
            frame_a: "Embodied".into(),
            frame_b: "Individual".into(),
            acknowledged: false,
        });

        assert!(state.has_conflicts());
        assert_eq!(state.unacknowledged_count(), 1);
    }

    #[test]
    fn html_rendering_includes_required_sections() {
        let mut state = make_view_state();
        state.add_conflict(ConflictCard {
            conflict_type: "FrameMismatch".into(),
            reason: "test conflict".into(),
            frame_a: "Embodied".into(),
            frame_b: "Individual".into(),
            acknowledged: false,
        });

        let renderer = AuroraRenderer;
        let html = renderer.render_html(&state);

        assert!(html.contains("Aurora Decision Report"));
        assert!(html.contains("Attention Sovereignty Index"));
        assert!(html.contains("Reminder History"));
        assert!(html.contains("FrameMismatch"));
    }

    #[test]
    fn html_rendering_shows_no_conflict_when_empty() {
        let state = make_view_state();
        let renderer = AuroraRenderer;
        let html = renderer.render_html(&state);

        assert!(html.contains("No conflict detected"));
    }

    #[test]
    fn json_rendering_includes_key_fields() {
        // Add a reminder + active response to get non-zero ASI
        let mut session = AttentionSession::new("test");
        session.record_reminder("ShiftTo", "Body", "test");
        session.record_user_response(crate::bc::attention_guidance::UserResponse::ShiftedTo(
            "Body".into(),
        ));

        let state = ViewState::new("Test".into(), session);
        let renderer = AuroraRenderer;
        let json = renderer.render_json(&state).unwrap();

        assert!(json.contains("decision_summary"));
        assert!(json.contains("asi"));
        assert!(json.contains("reminder_count"));
    }
}
