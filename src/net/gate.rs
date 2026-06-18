// Byzantine gatekeeper for Trit-Core distributed nodes (M8).
//
// The ByzantineGatekeeper sits between TCP deserialization and bus dispatch.
// It validates every incoming message against a set of safety checks:
//
// 1. Phase bounds: all phase values must be in [0.0, 1.0], finite, not NaN
// 2. Sender validation: non-empty sender ID, known node check
// 3. Frame validation: frame strings must match known Frame variants
// 4. Payload consistency: array lengths must agree (Negotiate)
// 5. Rate limiting: per-peer message cap per time window (DoS prevention)
// 6. Per-peer log cap: maximum message-log entries per peer
//
// ## Design
//
// The gatekeeper is optional — ResonanceBus holds `Option<ByzantineGatekeeper>`.
// When `None`, zero overhead. This preserves backward compatibility with
// existing tests that don't need Byzantine protection.

use crate::frame::Frame;
use crate::net::message::{Message, MessagePayload};
use std::collections::{HashMap, HashSet};
use std::str::FromStr;
use std::time::{Duration, Instant};

/// Default maximum messages per peer per rate-limit window.
pub const DEFAULT_MAX_MESSAGES_PER_WINDOW: usize = 100;
/// Default rate-limit window in seconds.
pub const DEFAULT_RATE_WINDOW_SECS: u64 = 1;
/// Default maximum message-log entries per peer.
pub const DEFAULT_MAX_PER_PEER_LOG: usize = 1000;

/// Reasons a message can be rejected by the gatekeeper.
#[derive(Debug, Clone, PartialEq)]
pub enum GateRejection {
    /// Sender ID is empty, whitespace-only, or too long.
    InvalidSender(String),
    /// A phase value is out of the valid range [0.0, 1.0] or non-finite.
    PhaseOutOfRange { field: String, value: f64 },
    /// A frame string does not match any known Frame variant.
    InvalidFrame(String),
    /// Payload fields are inconsistent (e.g., mismatched array lengths).
    PayloadInconsistent(String),
    /// Peer has exceeded the per-window message rate limit.
    RateLimited { peer: String, count: usize },
    /// Peer has exceeded the per-peer message-log entry cap.
    PerPeerLogFull { peer: String, count: usize },
    /// Sender is not in the set of known/registered nodes.
    UnknownSender(String),
}

impl std::fmt::Display for GateRejection {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            GateRejection::InvalidSender(detail) => {
                write!(f, "Invalid sender: {}", detail)
            }
            GateRejection::PhaseOutOfRange { field, value } => {
                write!(f, "Phase out of range in '{}': {}", field, value)
            }
            GateRejection::InvalidFrame(name) => {
                write!(f, "Invalid frame name: '{}'", name)
            }
            GateRejection::PayloadInconsistent(detail) => {
                write!(f, "Payload inconsistent: {}", detail)
            }
            GateRejection::RateLimited { peer, count } => {
                write!(
                    f,
                    "Rate limited: peer '{}' sent {} messages in current window",
                    peer, count
                )
            }
            GateRejection::PerPeerLogFull { peer, count } => {
                write!(
                    f,
                    "Per-peer log full: peer '{}' has {} entries in the message log",
                    peer, count
                )
            }
            GateRejection::UnknownSender(peer) => {
                write!(f, "Unknown sender: '{}'", peer)
            }
        }
    }
}

/// Per-peer statistics for rate limiting and log tracking.
struct PeerStats {
    /// Number of messages received in the current rate-limit window.
    message_count: usize,
    /// Start time of the current rate-limit window.
    window_start: Instant,
    /// Number of entries this peer has in the global message log.
    log_entries: usize,
    /// Time of the last message from this peer.
    #[allow(dead_code)]
    last_message_time: Instant,
}

impl PeerStats {
    fn new() -> Self {
        Self {
            message_count: 0,
            window_start: Instant::now(),
            log_entries: 0,
            last_message_time: Instant::now(),
        }
    }
}

/// Byzantine gatekeeper that validates incoming messages before they
/// reach the ResonanceBus.
///
/// ## Usage
///
/// ```
/// use trit_core::net::gate::ByzantineGatekeeper;
/// use trit_core::net::message::Message;
///
/// let mut gk = ByzantineGatekeeper::default();
/// gk.register_node("node-a");
/// let msg = Message::heartbeat("node-a", "Sovereign", 0.5);
/// assert!(gk.validate(&msg).is_ok());
/// ```
pub struct ByzantineGatekeeper {
    /// Per-peer message counters for rate limiting.
    peer_message_counts: HashMap<String, PeerStats>,
    /// Registered node IDs (populated from bus registration).
    known_nodes: HashSet<String>,
    /// Maximum messages per peer per rate-limit window.
    max_messages_per_window: usize,
    /// Rate-limit window duration.
    rate_window: Duration,
    /// Maximum total message log entries per peer.
    max_per_peer_log: usize,
}

impl ByzantineGatekeeper {
    /// Create a new gatekeeper with the given limits.
    pub fn new(
        max_messages_per_window: usize,
        rate_window_secs: u64,
        max_per_peer_log: usize,
    ) -> Self {
        Self {
            peer_message_counts: HashMap::new(),
            known_nodes: HashSet::new(),
            max_messages_per_window,
            rate_window: Duration::from_secs(rate_window_secs),
            max_per_peer_log,
        }
    }

    /// Register a node as known to the gatekeeper.
    ///
    /// Called by `ResonanceBus::register()` when a gatekeeper is present.
    pub fn register_node(&mut self, node_id: &str) {
        self.known_nodes.insert(node_id.to_string());
    }

    /// Unregister a node from the gatekeeper.
    pub fn unregister_node(&mut self, node_id: &str) {
        self.known_nodes.remove(node_id);
        self.peer_message_counts.remove(node_id);
    }

    /// Record that a peer has a new entry in the global message log.
    ///
    /// Called by `ResonanceBus::push_log()` when a gatekeeper is present.
    pub fn record_log_entry(&mut self, peer_id: &str) {
        let stats = self
            .peer_message_counts
            .entry(peer_id.to_string())
            .or_insert_with(PeerStats::new);
        stats.log_entries += 1;
    }

    /// Record that a peer's log entry was pruned (popped from the ring buffer).
    ///
    /// Called by `ResonanceBus::push_log()` when old entries are evicted.
    pub fn prune_log_entry(&mut self, peer_id: &str) {
        if let Some(stats) = self.peer_message_counts.get_mut(peer_id) {
            stats.log_entries = stats.log_entries.saturating_sub(1);
        }
    }

    /// Reset all rate-limit counters for a peer (e.g., after reconnection).
    pub fn reset_peer(&mut self, peer_id: &str) {
        if let Some(stats) = self.peer_message_counts.get_mut(peer_id) {
            stats.message_count = 0;
            stats.window_start = Instant::now();
        }
    }

    /// Main validation entry point.
    ///
    /// Runs all Byzantine checks on the message. Returns `Ok(())` if the
    /// message passes all checks, or `Err(GateRejection)` with the first
    /// failure reason.
    pub fn validate(&mut self, msg: &Message) -> Result<(), GateRejection> {
        // 1. Sender validation
        let sender = msg.header.sender.trim();
        if sender.is_empty() {
            return Err(GateRejection::InvalidSender(
                "empty or whitespace-only".into(),
            ));
        }
        if sender.len() > 128 {
            return Err(GateRejection::InvalidSender(
                "exceeds 128 characters".into(),
            ));
        }
        let sender = sender.to_string();

        // 2. Unknown sender check (only when known_nodes is populated)
        if !self.known_nodes.is_empty() && !self.known_nodes.contains(&sender) {
            return Err(GateRejection::UnknownSender(sender));
        }

        // 3. Phase validation
        self.validate_phases(msg)?;

        // 4. Frame validation
        self.validate_frames(msg)?;

        // 5. Payload consistency
        self.validate_consistency(msg)?;

        // 6. Rate limiting
        self.check_rate_limit(&sender)?;

        // 7. Per-peer log cap
        self.check_per_peer_log(&sender)?;

        Ok(())
    }

    /// Validate all phase values in the message are in [0.0, 1.0] and finite.
    fn validate_phases(&self, msg: &Message) -> Result<(), GateRejection> {
        match &msg.payload {
            MessagePayload::ResonateReq(req) => {
                if !req.phase.is_finite() || !(0.0..=1.0).contains(&req.phase) {
                    return Err(GateRejection::PhaseOutOfRange {
                        field: "ResonateReq.phase".into(),
                        value: req.phase,
                    });
                }
                for &h in &req.history {
                    if !h.is_finite() || !(0.0..=1.0).contains(&h) {
                        return Err(GateRejection::PhaseOutOfRange {
                            field: "ResonateReq.history".into(),
                            value: h,
                        });
                    }
                }
            }
            MessagePayload::ResonateAck(ack) => {
                if !ack.coupled_phase.is_finite() || !(0.0..=1.0).contains(&ack.coupled_phase) {
                    return Err(GateRejection::PhaseOutOfRange {
                        field: "ResonateAck.coupled_phase".into(),
                        value: ack.coupled_phase,
                    });
                }
            }
            MessagePayload::Negotiate(payload) => {
                for &p in &payload.phases {
                    if !p.is_finite() || !(0.0..=1.0).contains(&p) {
                        return Err(GateRejection::PhaseOutOfRange {
                            field: "Negotiate.phases".into(),
                            value: p,
                        });
                    }
                }
                if !payload.consensus_phase.is_finite()
                    || !(0.0..=1.0).contains(&payload.consensus_phase)
                {
                    return Err(GateRejection::PhaseOutOfRange {
                        field: "Negotiate.consensus_phase".into(),
                        value: payload.consensus_phase,
                    });
                }
            }
            MessagePayload::Heartbeat(hb) => {
                if !hb.current_phase.is_finite() || !(0.0..=1.0).contains(&hb.current_phase) {
                    return Err(GateRejection::PhaseOutOfRange {
                        field: "Heartbeat.current_phase".into(),
                        value: hb.current_phase,
                    });
                }
            }
            MessagePayload::DecoupleReq(_) | MessagePayload::DecoupleAck(_) => {}
        }
        Ok(())
    }

    /// Validate frame strings in the message match known Frame variants.
    fn validate_frames(&self, msg: &Message) -> Result<(), GateRejection> {
        match &msg.payload {
            MessagePayload::ResonateReq(req) => {
                if Frame::from_str(&req.frame).is_err() {
                    return Err(GateRejection::InvalidFrame(req.frame.clone()));
                }
            }
            MessagePayload::Negotiate(payload) => {
                for f in &payload.frames {
                    if Frame::from_str(f).is_err() {
                        return Err(GateRejection::InvalidFrame(f.clone()));
                    }
                }
            }
            _ => {}
        }
        Ok(())
    }

    /// Validate payload consistency (array lengths, required fields).
    fn validate_consistency(&self, msg: &Message) -> Result<(), GateRejection> {
        if let MessagePayload::Negotiate(payload) = &msg.payload {
            let n = payload.participants.len();
            if payload.frames.len() != n || payload.phases.len() != n {
                return Err(GateRejection::PayloadInconsistent(
                    "Negotiate participants/frames/phases lengths mismatch".into(),
                ));
            }
            if n == 0 {
                return Err(GateRejection::PayloadInconsistent(
                    "Negotiate has zero participants".into(),
                ));
            }
        }
        Ok(())
    }

    /// Check and update the per-peer rate limit.
    fn check_rate_limit(&mut self, sender: &str) -> Result<(), GateRejection> {
        let now = Instant::now();
        let stats = self
            .peer_message_counts
            .entry(sender.to_string())
            .or_insert_with(PeerStats::new);

        // Reset window if expired
        if now.duration_since(stats.window_start) >= self.rate_window {
            stats.message_count = 0;
            stats.window_start = now;
        }

        stats.message_count += 1;
        stats.last_message_time = now;

        if stats.message_count > self.max_messages_per_window {
            return Err(GateRejection::RateLimited {
                peer: sender.to_string(),
                count: stats.message_count,
            });
        }
        Ok(())
    }

    /// Check the per-peer message-log entry cap.
    fn check_per_peer_log(&self, sender: &str) -> Result<(), GateRejection> {
        if let Some(stats) = self.peer_message_counts.get(sender) {
            if stats.log_entries >= self.max_per_peer_log {
                return Err(GateRejection::PerPeerLogFull {
                    peer: sender.to_string(),
                    count: stats.log_entries,
                });
            }
        }
        Ok(())
    }
}

impl Default for ByzantineGatekeeper {
    fn default() -> Self {
        Self::new(
            DEFAULT_MAX_MESSAGES_PER_WINDOW,
            DEFAULT_RATE_WINDOW_SECS,
            DEFAULT_MAX_PER_PEER_LOG,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::net::message::Message;

    fn make_heartbeat(sender: &str, phase: f64) -> Message {
        Message::heartbeat(sender, "Sovereign", phase)
    }

    fn make_resonate_req(sender: &str, frame: &str, phase: f64) -> Message {
        Message::resonate_req(sender, frame, phase, vec![])
    }

    // ── Valid message passes ──

    #[test]
    fn valid_heartbeat_passes() {
        let mut gk = ByzantineGatekeeper::default();
        gk.register_node("node1");
        let msg = make_heartbeat("node1", 0.5);
        assert!(gk.validate(&msg).is_ok());
    }

    #[test]
    fn valid_resonate_req_passes() {
        let mut gk = ByzantineGatekeeper::default();
        gk.register_node("node1");
        let msg = make_resonate_req("node1", "Science", 0.7);
        assert!(gk.validate(&msg).is_ok());
    }

    #[test]
    fn valid_decouple_req_passes() {
        let mut gk = ByzantineGatekeeper::default();
        gk.register_node("node1");
        let msg = Message::decouple_req("node1", "user_disconnect");
        assert!(gk.validate(&msg).is_ok());
    }

    // ── Phase out of range ──

    #[test]
    fn phase_above_one_rejected() {
        let mut gk = ByzantineGatekeeper::default();
        gk.register_node("node1");
        let msg = make_heartbeat("node1", 1.5);
        match gk.validate(&msg) {
            Err(GateRejection::PhaseOutOfRange { value, .. }) => {
                assert!((value - 1.5).abs() < f64::EPSILON);
            }
            other => panic!("Expected PhaseOutOfRange, got {:?}", other),
        }
    }

    #[test]
    fn phase_below_zero_rejected() {
        let mut gk = ByzantineGatekeeper::default();
        gk.register_node("node1");
        let msg = make_heartbeat("node1", -0.1);
        assert!(matches!(
            gk.validate(&msg),
            Err(GateRejection::PhaseOutOfRange { .. })
        ));
    }

    #[test]
    fn phase_nan_rejected() {
        let mut gk = ByzantineGatekeeper::default();
        gk.register_node("node1");
        let msg = make_heartbeat("node1", f64::NAN);
        assert!(matches!(
            gk.validate(&msg),
            Err(GateRejection::PhaseOutOfRange { .. })
        ));
    }

    #[test]
    fn phase_infinity_rejected() {
        let mut gk = ByzantineGatekeeper::default();
        gk.register_node("node1");
        let msg = make_heartbeat("node1", f64::INFINITY);
        assert!(matches!(
            gk.validate(&msg),
            Err(GateRejection::PhaseOutOfRange { .. })
        ));
    }

    // ── Invalid sender ──

    #[test]
    fn empty_sender_rejected() {
        let mut gk = ByzantineGatekeeper::default();
        gk.register_node("node1");
        let mut msg = make_heartbeat("node1", 0.5);
        msg.header.sender = String::new();
        assert!(matches!(
            gk.validate(&msg),
            Err(GateRejection::InvalidSender(_))
        ));
    }

    #[test]
    fn whitespace_sender_rejected() {
        let mut gk = ByzantineGatekeeper::default();
        gk.register_node("node1");
        let mut msg = make_heartbeat("node1", 0.5);
        msg.header.sender = "   ".to_string();
        assert!(matches!(
            gk.validate(&msg),
            Err(GateRejection::InvalidSender(_))
        ));
    }

    #[test]
    fn overlong_sender_rejected() {
        let mut gk = ByzantineGatekeeper::default();
        gk.register_node("node1");
        let mut msg = make_heartbeat("node1", 0.5);
        msg.header.sender = "x".repeat(129);
        assert!(matches!(
            gk.validate(&msg),
            Err(GateRejection::InvalidSender(_))
        ));
    }

    // ── Unknown sender ──

    #[test]
    fn unknown_sender_rejected_when_known_nodes_populated() {
        let mut gk = ByzantineGatekeeper::default();
        gk.register_node("node-a");
        gk.register_node("node-b");
        let msg = make_heartbeat("unknown-intruder", 0.5);
        assert!(matches!(
            gk.validate(&msg),
            Err(GateRejection::UnknownSender(ref s)) if s == "unknown-intruder"
        ));
    }

    #[test]
    fn unknown_sender_allowed_when_known_nodes_empty() {
        // When known_nodes is empty, the gatekeeper doesn't enforce registration
        let mut gk = ByzantineGatekeeper::default();
        let msg = make_heartbeat("anyone", 0.5);
        assert!(gk.validate(&msg).is_ok());
    }

    // ── Invalid frame ──

    #[test]
    fn invalid_frame_rejected() {
        let mut gk = ByzantineGatekeeper::default();
        gk.register_node("node1");
        let msg = make_resonate_req("node1", "__MALICIOUS__", 0.5);
        assert!(matches!(
            gk.validate(&msg),
            Err(GateRejection::InvalidFrame(ref f)) if f == "__MALICIOUS__"
        ));
    }

    #[test]
    fn valid_frames_pass() {
        let mut gk = ByzantineGatekeeper::default();
        gk.register_node("node1");
        for frame in &["Science", "Individual", "Consensus", "Absolute", "Meta"] {
            let msg = make_resonate_req("node1", frame, 0.5);
            assert!(gk.validate(&msg).is_ok(), "Frame '{}' should pass", frame);
        }
    }

    // ── Negotiate consistency ──

    #[test]
    fn negotiate_mismatched_lengths_rejected() {
        let mut gk = ByzantineGatekeeper::default();
        gk.register_node("node1");
        let msg = Message::negotiate(
            "node1",
            vec!["a".into(), "b".into()],
            vec!["Science".into()], // only 1 frame
            vec![0.5, 0.6],
            "hold",
        );
        assert!(matches!(
            gk.validate(&msg),
            Err(GateRejection::PayloadInconsistent(_))
        ));
    }

    #[test]
    fn negotiate_zero_participants_rejected() {
        let mut gk = ByzantineGatekeeper::default();
        gk.register_node("node1");
        // Message::negotiate with empty vecs produces consensus_phase = NaN (0.0/0.0).
        // The phase check catches this before the consistency check, which is correct:
        // NaN is a phase-out-of-range error regardless.
        let msg = Message::negotiate("node1", vec![], vec![], vec![], "hold");
        // Expect either PhaseOutOfRange (NaN consensus_phase) or PayloadInconsistent
        match gk.validate(&msg) {
            Err(GateRejection::PhaseOutOfRange { .. })
            | Err(GateRejection::PayloadInconsistent(_)) => {}
            other => panic!(
                "Expected PhaseOutOfRange or PayloadInconsistent, got {:?}",
                other
            ),
        }
    }

    // ── Rate limiting ──

    #[test]
    fn rate_limit_exceeded_rejected() {
        let mut gk = ByzantineGatekeeper::new(5, 60, 1000); // 5 msg/min window
        gk.register_node("flooder");

        // Send 5 messages (should pass)
        for _ in 0..5 {
            let msg = make_heartbeat("flooder", 0.5);
            assert!(gk.validate(&msg).is_ok());
        }

        // 6th message should be rate-limited
        let msg = make_heartbeat("flooder", 0.5);
        match gk.validate(&msg) {
            Err(GateRejection::RateLimited { ref peer, count }) => {
                assert_eq!(peer, "flooder");
                assert_eq!(count, 6);
            }
            other => panic!("Expected RateLimited, got {:?}", other),
        }
    }

    #[test]
    fn rate_limit_resets_after_window_expires() {
        let mut gk = ByzantineGatekeeper::new(3, 60, 1000);
        gk.register_node("flooder");

        // Exhaust the limit
        for _ in 0..3 {
            let msg = make_heartbeat("flooder", 0.5);
            assert!(gk.validate(&msg).is_ok());
        }
        let msg = make_heartbeat("flooder", 0.5);
        assert!(matches!(
            gk.validate(&msg),
            Err(GateRejection::RateLimited { .. })
        ));

        // Manually expire the window
        if let Some(stats) = gk.peer_message_counts.get_mut("flooder") {
            stats.window_start = Instant::now().checked_sub(Duration::from_secs(61)).unwrap();
        }

        // Now should pass again
        let msg = make_heartbeat("flooder", 0.5);
        assert!(gk.validate(&msg).is_ok());
    }

    // ── Per-peer log cap ──

    #[test]
    fn per_peer_log_full_rejected() {
        let mut gk = ByzantineGatekeeper::new(100, 60, 3); // max 3 log entries per peer
        gk.register_node("logger");

        // Fill up the log quota
        for _ in 0..3 {
            gk.record_log_entry("logger");
        }

        // 4th message should be rejected
        let msg = make_heartbeat("logger", 0.5);
        match gk.validate(&msg) {
            Err(GateRejection::PerPeerLogFull { ref peer, count }) => {
                assert_eq!(peer, "logger");
                assert_eq!(count, 3);
            }
            other => panic!("Expected PerPeerLogFull, got {:?}", other),
        }
    }

    // ── Register / unregister ──

    #[test]
    fn unregistered_node_rejected_after_removal() {
        let mut gk = ByzantineGatekeeper::default();
        gk.register_node("node1");
        gk.register_node("node2"); // keep known_nodes non-empty
        gk.unregister_node("node1");

        let msg = make_heartbeat("node1", 0.5);
        assert!(matches!(
            gk.validate(&msg),
            Err(GateRejection::UnknownSender(ref s)) if s == "node1"
        ));
    }

    #[test]
    fn reset_peer_clears_rate_limit() {
        let mut gk = ByzantineGatekeeper::new(2, 60, 1000);
        gk.register_node("flooder");

        for _ in 0..2 {
            let msg = make_heartbeat("flooder", 0.5);
            assert!(gk.validate(&msg).is_ok());
        }
        let msg = make_heartbeat("flooder", 0.5);
        assert!(matches!(
            gk.validate(&msg),
            Err(GateRejection::RateLimited { .. })
        ));

        gk.reset_peer("flooder");
        let msg = make_heartbeat("flooder", 0.5);
        assert!(gk.validate(&msg).is_ok());
    }

    // ── Display ──

    #[test]
    fn gate_rejection_display() {
        let r = GateRejection::InvalidSender("test".into());
        assert!(r.to_string().contains("test"));

        let r = GateRejection::PhaseOutOfRange {
            field: "f".into(),
            value: 1.5,
        };
        assert!(r.to_string().contains("1.5"));

        let r = GateRejection::UnknownSender("evil".into());
        assert!(r.to_string().contains("evil"));
    }
}
