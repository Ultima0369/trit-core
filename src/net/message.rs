use serde::{Deserialize, Serialize};

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
}
