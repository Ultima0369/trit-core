use crate::frame::Frame;
use serde::{Deserialize, Serialize};
use std::str::FromStr;

/// Protocol message operations per ADR-004 §3.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum OpCode {
    #[serde(rename = "RESONATE_REQ")]
    ResonateReq,
    #[serde(rename = "RESONATE_ACK")]
    ResonateAck,
    #[serde(rename = "DECOUPLE_REQ")]
    DecoupleReq,
    #[serde(rename = "DECOUPLE_ACK")]
    DecoupleAck,
    #[serde(rename = "NEGOTIATE")]
    Negotiate,
    #[serde(rename = "HEARTBEAT")]
    Heartbeat,
}

/// Common header for all messages per ADR-004 §2.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessageHeader {
    pub proto: String,
    pub msg_id: String,
    pub timestamp: String,
    pub sender: String,
}

impl MessageHeader {
    pub fn new(sender: &str) -> Self {
        Self {
            proto: "trit-proto/1.0".to_string(),
            msg_id: uuid::Uuid::new_v4().to_string(),
            timestamp: chrono::Utc::now().to_rfc3339(),
            sender: sender.to_string(),
        }
    }
}

/// Generic protocol message envelope.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub header: MessageHeader,
    pub payload: MessagePayload,
}

/// Payload variants for all protocol operations.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "op")]
pub enum MessagePayload {
    #[serde(rename = "RESONATE_REQ")]
    ResonateReq(ResonateReq),
    #[serde(rename = "RESONATE_ACK")]
    ResonateAck(ResonateAck),
    #[serde(rename = "DECOUPLE_REQ")]
    DecoupleReq(DecoupleReq),
    #[serde(rename = "DECOUPLE_ACK")]
    DecoupleAck(DecoupleAck),
    #[serde(rename = "NEGOTIATE")]
    Negotiate(NegotiatePayload),
    #[serde(rename = "HEARTBEAT")]
    Heartbeat(HeartbeatPayload),
}

/// RESONATE_REQ: request phase coupling with a peer.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResonateReq {
    pub frame: String,
    pub phase: f64,
    pub history: Vec<f64>,
}

/// RESONATE_ACK: response with coupling result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResonateAck {
    pub coupled_phase: f64,
    pub interference: String, // "constructive" | "neutral" | "destructive"
    pub conflict_detected: bool,
    pub recommendation: String, // "commit" | "continue" | "hold" | "negotiate"
}

/// DECOUPLE_REQ: request to break coupling.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DecoupleReq {
    pub reason: String, // "user_disconnect" | "timeout" | "policy_violation"
}

/// DECOUPLE_ACK: confirm decoupling complete.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DecoupleAck {
    pub restored_phase: f64,
    pub cycles_coupled: u64,
}

/// NEGOTIATE: multi-node negotiation result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NegotiatePayload {
    pub participants: Vec<String>,
    pub frames: Vec<String>,
    pub phases: Vec<f64>,
    pub consensus_phase: f64,
    pub conflict_resolution: String, // "hold" | "commit_true" | "commit_false"
}

/// HEARTBEAT: keep-alive ping.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HeartbeatPayload {
    pub node_state: String,
    pub current_phase: f64,
}

// -- Constructor helpers for messages --

impl Message {
    pub fn resonate_req(sender: &str, frame: &str, phase: f64, history: Vec<f64>) -> Self {
        Self {
            header: MessageHeader::new(sender),
            payload: MessagePayload::ResonateReq(ResonateReq {
                frame: frame.to_string(),
                phase,
                history,
            }),
        }
    }

    pub fn resonate_ack(
        sender: &str,
        coupled_phase: f64,
        interference: &str,
        conflict_detected: bool,
        recommendation: &str,
    ) -> Self {
        Self {
            header: MessageHeader::new(sender),
            payload: MessagePayload::ResonateAck(ResonateAck {
                coupled_phase,
                interference: interference.to_string(),
                conflict_detected,
                recommendation: recommendation.to_string(),
            }),
        }
    }

    pub fn decouple_req(sender: &str, reason: &str) -> Self {
        Self {
            header: MessageHeader::new(sender),
            payload: MessagePayload::DecoupleReq(DecoupleReq {
                reason: reason.to_string(),
            }),
        }
    }

    pub fn decouple_ack(sender: &str, restored_phase: f64, cycles_coupled: u64) -> Self {
        Self {
            header: MessageHeader::new(sender),
            payload: MessagePayload::DecoupleAck(DecoupleAck {
                restored_phase,
                cycles_coupled,
            }),
        }
    }

    pub fn negotiate(
        sender: &str,
        participants: Vec<String>,
        frames: Vec<String>,
        phases: Vec<f64>,
        conflict_resolution: &str,
    ) -> Self {
        let consensus_phase = phases.iter().sum::<f64>() / phases.len() as f64;
        Self {
            header: MessageHeader::new(sender),
            payload: MessagePayload::Negotiate(NegotiatePayload {
                participants,
                frames,
                phases,
                consensus_phase,
                conflict_resolution: conflict_resolution.to_string(),
            }),
        }
    }

    pub fn heartbeat(sender: &str, node_state: &str, current_phase: f64) -> Self {
        Self {
            header: MessageHeader::new(sender),
            payload: MessagePayload::Heartbeat(HeartbeatPayload {
                node_state: node_state.to_string(),
                current_phase,
            }),
        }
    }

    // ── M8 Byzantine validation helpers ──

    /// Validate that all phase values in this message are in [0.0, 1.0],
    /// finite, and not NaN.
    pub fn validate_phase(&self) -> Result<(), &'static str> {
        match &self.payload {
            MessagePayload::ResonateReq(req) => {
                if !req.phase.is_finite() || !(0.0..=1.0).contains(&req.phase) {
                    return Err("ResonateReq phase out of [0.0, 1.0] or non-finite");
                }
                for (i, &h) in req.history.iter().enumerate() {
                    if !h.is_finite() || !(0.0..=1.0).contains(&h) {
                        return Err("ResonateReq history phase out of [0.0, 1.0] or non-finite");
                    }
                    let _ = i;
                }
            }
            MessagePayload::ResonateAck(ack) => {
                if !ack.coupled_phase.is_finite() || !(0.0..=1.0).contains(&ack.coupled_phase) {
                    return Err("ResonateAck coupled_phase out of [0.0, 1.0] or non-finite");
                }
            }
            MessagePayload::Negotiate(payload) => {
                for (i, &p) in payload.phases.iter().enumerate() {
                    if !p.is_finite() || !(0.0..=1.0).contains(&p) {
                        return Err("Negotiate phase out of [0.0, 1.0] or non-finite");
                    }
                    let _ = i;
                }
                if !payload.consensus_phase.is_finite()
                    || !(0.0..=1.0).contains(&payload.consensus_phase)
                {
                    return Err("Negotiate consensus_phase out of [0.0, 1.0] or non-finite");
                }
            }
            MessagePayload::Heartbeat(hb) => {
                if !hb.current_phase.is_finite() || !(0.0..=1.0).contains(&hb.current_phase) {
                    return Err("Heartbeat current_phase out of [0.0, 1.0] or non-finite");
                }
            }
            // DecoupleReq and DecoupleAck have no phase fields
            MessagePayload::DecoupleReq(_) | MessagePayload::DecoupleAck(_) => {}
        }
        Ok(())
    }

    /// Validate the sender field: non-empty, no whitespace-only, reasonable length.
    pub fn validate_sender(&self) -> Result<(), &'static str> {
        let sender = self.header.sender.trim();
        if sender.is_empty() {
            return Err("Sender is empty or whitespace-only");
        }
        if sender.len() > 128 {
            return Err("Sender exceeds 128 characters");
        }
        Ok(())
    }

    /// Validate that frame names in the message match known Frame variants.
    pub fn validate_frame_name(&self) -> Result<(), &'static str> {
        match &self.payload {
            MessagePayload::ResonateReq(req) => {
                if Frame::from_str(&req.frame).is_err() {
                    return Err("ResonateReq frame is not a valid Frame name");
                }
            }
            MessagePayload::Negotiate(payload) => {
                for (i, f) in payload.frames.iter().enumerate() {
                    if Frame::from_str(f).is_err() {
                        return Err("Negotiate frame is not a valid Frame name");
                    }
                    let _ = i;
                }
            }
            _ => {} // Other payloads don't carry frame strings
        }
        Ok(())
    }

    /// Validate semantic completeness: required fields are present and
    /// array lengths are consistent.
    pub fn validate_completeness(&self) -> Result<(), &'static str> {
        if let MessagePayload::Negotiate(payload) = &self.payload {
            let n = payload.participants.len();
            if payload.frames.len() != n || payload.phases.len() != n {
                return Err("Negotiate participants/frames/phases lengths are inconsistent");
            }
            if n == 0 {
                return Err("Negotiate has zero participants");
            }
        }
        Ok(())
    }

    /// Run all validation checks, collecting errors.
    /// Returns Ok(()) if all pass, or Err with all failure messages.
    pub fn validate_all(&self) -> Result<(), Vec<&'static str>> {
        let mut errors = Vec::new();
        if let Err(e) = self.validate_sender() {
            errors.push(e);
        }
        if let Err(e) = self.validate_phase() {
            errors.push(e);
        }
        if let Err(e) = self.validate_frame_name() {
            errors.push(e);
        }
        if let Err(e) = self.validate_completeness() {
            errors.push(e);
        }
        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }
}

impl HeartbeatPayload {
    /// Validate that current_phase is in [0.0, 1.0] and finite.
    pub fn validate_phase(&self) -> Result<(), &'static str> {
        if !self.current_phase.is_finite() || !(0.0..=1.0).contains(&self.current_phase) {
            return Err("Heartbeat current_phase out of [0.0, 1.0] or non-finite");
        }
        Ok(())
    }

    /// Validate that node_state matches a known NodeState variant.
    pub fn validate_node_state(&self) -> Result<(), &'static str> {
        if matches!(
            self.node_state.as_str(),
            "Sovereign" | "Coupling" | "Coupled" | "Hold"
        ) {
            Ok(())
        } else {
            Err("Heartbeat node_state is not a valid NodeState")
        }
    }
}

impl ResonateReq {
    /// Validate history: all entries in [0.0, 1.0], length bounded.
    pub fn validate_history(&self) -> Result<(), &'static str> {
        if self.history.len() > 1000 {
            return Err("ResonateReq history exceeds 1000 entries");
        }
        for (i, &h) in self.history.iter().enumerate() {
            if !h.is_finite() || !(0.0..=1.0).contains(&h) {
                return Err("ResonateReq history entry out of [0.0, 1.0] or non-finite");
            }
            let _ = i;
        }
        Ok(())
    }
}

impl NegotiatePayload {
    /// Validate that participants, frames, and phases arrays have consistent lengths.
    pub fn validate_consistency(&self) -> Result<(), &'static str> {
        let n = self.participants.len();
        if self.frames.len() != n || self.phases.len() != n {
            return Err("Negotiate participants/frames/phases lengths are inconsistent");
        }
        if n == 0 {
            return Err("Negotiate has zero participants");
        }
        Ok(())
    }

    /// Validate all phases in the payload are in [0.0, 1.0] and finite.
    pub fn validate_phases(&self) -> Result<(), &'static str> {
        for (i, &p) in self.phases.iter().enumerate() {
            if !p.is_finite() || !(0.0..=1.0).contains(&p) {
                return Err("Negotiate phase out of [0.0, 1.0] or non-finite");
            }
            let _ = i;
        }
        if !self.consensus_phase.is_finite()
            || self.consensus_phase < 0.0
            || self.consensus_phase > 1.0
        {
            return Err("Negotiate consensus_phase out of [0.0, 1.0] or non-finite");
        }
        Ok(())
    }
}

#[cfg(test)]
mod validation_tests {
    use super::*;

    // ── Message::validate_phase ──

    #[test]
    fn valid_heartbeat_passes_phase_check() {
        let msg = Message::heartbeat("node1", "Sovereign", 0.5);
        assert!(msg.validate_phase().is_ok());
    }

    #[test]
    fn heartbeat_phase_above_one_rejected() {
        let msg = Message::heartbeat("node1", "Sovereign", 1.5);
        assert!(msg.validate_phase().is_err());
    }

    #[test]
    fn heartbeat_phase_below_zero_rejected() {
        let msg = Message::heartbeat("node1", "Sovereign", -0.1);
        assert!(msg.validate_phase().is_err());
    }

    #[test]
    fn heartbeat_phase_nan_rejected() {
        let msg = Message::heartbeat("node1", "Sovereign", f64::NAN);
        assert!(msg.validate_phase().is_err());
    }

    #[test]
    fn heartbeat_phase_infinity_rejected() {
        let msg = Message::heartbeat("node1", "Sovereign", f64::INFINITY);
        assert!(msg.validate_phase().is_err());
    }

    #[test]
    fn valid_resonate_req_passes_phase_check() {
        let msg = Message::resonate_req("node1", "Science", 0.7, vec![0.5, 0.6]);
        assert!(msg.validate_phase().is_ok());
    }

    #[test]
    fn resonate_req_bad_history_rejected() {
        let msg = Message::resonate_req("node1", "Science", 0.7, vec![0.5, 1.5]);
        assert!(msg.validate_phase().is_err());
    }

    #[test]
    fn valid_resonate_ack_passes_phase_check() {
        let msg = Message::resonate_ack("node1", 0.75, "constructive", false, "commit");
        assert!(msg.validate_phase().is_ok());
    }

    #[test]
    fn resonate_ack_bad_phase_rejected() {
        let msg = Message::resonate_ack("node1", -0.5, "constructive", false, "commit");
        assert!(msg.validate_phase().is_err());
    }

    #[test]
    fn valid_negotiate_passes_phase_check() {
        let msg = Message::negotiate(
            "node1",
            vec!["a".into(), "b".into()],
            vec!["Science".into(), "Individual".into()],
            vec![0.5, 0.6],
            "hold",
        );
        assert!(msg.validate_phase().is_ok());
    }

    #[test]
    fn negotiate_bad_phase_rejected() {
        let msg = Message::negotiate(
            "node1",
            vec!["a".into()],
            vec!["Science".into()],
            vec![2.0],
            "hold",
        );
        assert!(msg.validate_phase().is_err());
    }

    #[test]
    fn decouple_req_passes_phase_check() {
        let msg = Message::decouple_req("node1", "user_disconnect");
        assert!(msg.validate_phase().is_ok());
    }

    // ── Message::validate_sender ──

    #[test]
    fn valid_sender_passes() {
        let msg = Message::heartbeat("node-1", "Sovereign", 0.5);
        assert!(msg.validate_sender().is_ok());
    }

    #[test]
    fn empty_sender_rejected() {
        let mut msg = Message::heartbeat("node-1", "Sovereign", 0.5);
        msg.header.sender = String::new();
        assert!(msg.validate_sender().is_err());
    }

    #[test]
    fn whitespace_sender_rejected() {
        let mut msg = Message::heartbeat("node-1", "Sovereign", 0.5);
        msg.header.sender = "   ".to_string();
        assert!(msg.validate_sender().is_err());
    }

    #[test]
    fn overlong_sender_rejected() {
        let mut msg = Message::heartbeat("node-1", "Sovereign", 0.5);
        msg.header.sender = "x".repeat(129);
        assert!(msg.validate_sender().is_err());
    }

    // ── Message::validate_frame_name ──

    #[test]
    fn valid_frame_name_passes() {
        let msg = Message::resonate_req("node1", "Science", 0.5, vec![]);
        assert!(msg.validate_frame_name().is_ok());
    }

    #[test]
    fn invalid_frame_name_rejected() {
        let msg = Message::resonate_req("node1", "__MALICIOUS__", 0.5, vec![]);
        assert!(msg.validate_frame_name().is_err());
    }

    #[test]
    fn negotiate_invalid_frame_rejected() {
        let msg = Message::negotiate(
            "node1",
            vec!["a".into()],
            vec!["BadFrame".into()],
            vec![0.5],
            "hold",
        );
        assert!(msg.validate_frame_name().is_err());
    }

    // ── Message::validate_completeness ──

    #[test]
    fn consistent_negotiate_passes() {
        let msg = Message::negotiate(
            "node1",
            vec!["a".into(), "b".into()],
            vec!["Science".into(), "Individual".into()],
            vec![0.5, 0.6],
            "hold",
        );
        assert!(msg.validate_completeness().is_ok());
    }

    #[test]
    fn inconsistent_negotiate_rejected() {
        let msg = Message::negotiate(
            "node1",
            vec!["a".into(), "b".into()],
            vec!["Science".into()], // only 1 frame, 2 participants
            vec![0.5, 0.6],
            "hold",
        );
        assert!(msg.validate_completeness().is_err());
    }

    #[test]
    fn empty_negotiate_rejected() {
        let msg = Message::negotiate("node1", vec![], vec![], vec![], "hold");
        assert!(msg.validate_completeness().is_err());
    }

    // ── Message::validate_all ──

    #[test]
    fn validate_all_passes_for_valid_message() {
        let msg = Message::heartbeat("node1", "Sovereign", 0.5);
        assert!(msg.validate_all().is_ok());
    }

    #[test]
    fn validate_all_collects_multiple_errors() {
        let mut msg = Message::heartbeat("node1", "Sovereign", 1.5);
        msg.header.sender = String::new();
        let result = msg.validate_all();
        assert!(result.is_err());
        let errors = result.unwrap_err();
        assert!(
            errors.len() >= 2,
            "Expected at least 2 errors, got {}",
            errors.len()
        );
    }

    // ── HeartbeatPayload::validate_node_state ──

    #[test]
    fn valid_node_states_pass() {
        for state in &["Sovereign", "Coupling", "Coupled", "Hold"] {
            let hb = HeartbeatPayload {
                node_state: state.to_string(),
                current_phase: 0.5,
            };
            assert!(
                hb.validate_node_state().is_ok(),
                "State '{}' should be valid",
                state
            );
        }
    }

    #[test]
    fn invalid_node_state_rejected() {
        let hb = HeartbeatPayload {
            node_state: "Byzantine".to_string(),
            current_phase: 0.5,
        };
        assert!(hb.validate_node_state().is_err());
    }

    // ── ResonateReq::validate_history ──

    #[test]
    fn valid_history_passes() {
        let req = ResonateReq {
            frame: "Science".into(),
            phase: 0.5,
            history: vec![0.1, 0.5, 0.9],
        };
        assert!(req.validate_history().is_ok());
    }

    #[test]
    fn oversized_history_rejected() {
        let req = ResonateReq {
            frame: "Science".into(),
            phase: 0.5,
            history: vec![0.5; 1001],
        };
        assert!(req.validate_history().is_err());
    }

    // ── NegotiatePayload::validate_consistency ──

    #[test]
    fn consistent_negotiate_payload_passes() {
        let payload = NegotiatePayload {
            participants: vec!["a".into(), "b".into()],
            frames: vec!["Science".into(), "Individual".into()],
            phases: vec![0.5, 0.6],
            consensus_phase: 0.55,
            conflict_resolution: "hold".into(),
        };
        assert!(payload.validate_consistency().is_ok());
    }

    #[test]
    fn mismatched_negotiate_payload_rejected() {
        let payload = NegotiatePayload {
            participants: vec!["a".into(), "b".into(), "c".into()],
            frames: vec!["Science".into()],
            phases: vec![0.5],
            consensus_phase: 0.5,
            conflict_resolution: "hold".into(),
        };
        assert!(payload.validate_consistency().is_err());
    }
}
