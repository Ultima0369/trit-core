use std::collections::HashMap;

use crate::frame::Frame;
use crate::net::message::{Message, MessagePayload};
use crate::net::node::Node;
use crate::net::pll::PllController;
use crate::trit::{TritValue, TritWord};

/// Maximum number of messages retained in the log (ring buffer).
const MAX_MESSAGE_LOG: usize = 10_000;
/// Maximum number of registered nodes.
const MAX_NODES: usize = 256;

/// In-memory message bus for local multi-node simulation.
///
/// Routes messages between nodes, applies PLL corrections,
/// and manages the coupling lifecycle. Message log is a capped
/// ring buffer to prevent unbounded memory growth (CWE-770).
pub struct ResonanceBus {
    /// All registered nodes indexed by id.
    pub nodes: HashMap<String, Node>,
    /// PLL controllers per node id.
    pub plls: HashMap<String, PllController>,
    /// Message log for audit trail (capped ring buffer).
    pub message_log: Vec<Message>,
}

impl ResonanceBus {
    pub fn new() -> Self {
        Self {
            nodes: HashMap::new(),
            plls: HashMap::new(),
            message_log: vec![],
        }
    }

    /// Register a node on the bus. Rejects registration if the node limit
    /// (MAX_NODES) has been reached (CWE-770).
    pub fn register(&mut self, node: Node) {
        if self.nodes.len() >= MAX_NODES {
            tracing::warn!(
                node_id = %node.id,
                "Max nodes ({}) reached, rejecting registration",
                MAX_NODES
            );
            return;
        }
        self.plls.insert(node.id.clone(), PllController::new());
        self.nodes.insert(node.id.clone(), node);
    }

    /// Process a RESONATE_REQ: sender requests coupling with a target node.
    ///
    /// Returns the RESONATE_ACK that the target should send back.
    pub fn handle_resonate_req(
        &mut self,
        from_id: &str,
        to_id: &str,
        msg: &Message,
    ) -> Option<Message> {
        let (from_phase, from_frame) = {
            let from = self.nodes.get(from_id)?;
            (from.current_phase, from.frame.clone())
        };

        let to = self.nodes.get(to_id)?;

        // Compute interference
        let interference = match (&from_frame, &to.frame) {
            (a, b) if a == b => "constructive",
            _ => {
                if PllController::is_conflict_phase_gap(from_phase, to.current_phase) {
                    "destructive"
                } else {
                    "neutral"
                }
            }
        };

        let coupled_phase = (from_phase + to.current_phase) / 2.0;
        let conflict_detected = interference == "destructive";
        let recommendation = if conflict_detected {
            "hold"
        } else if interference == "constructive" {
            "commit"
        } else {
            "negotiate"
        };

        let ack = Message::resonate_ack(
            to_id,
            coupled_phase,
            interference,
            conflict_detected,
            recommendation,
        );
        self.push_log(msg.clone());
        self.push_log(ack.clone());

        // Update sender state
        if let Some(from_node) = self.nodes.get_mut(from_id) {
            from_node.initiate_coupling(to_id);
            from_node.current_phase = coupled_phase;
        }

        Some(ack)
    }

    /// Process a RESONATE_ACK: confirm coupling for the sender.
    pub fn handle_resonate_ack(&mut self, node_id: &str, ack: &Message) {
        if let Some(node) = self.nodes.get_mut(node_id) {
            node.confirm_coupling();
        }
        // Apply PLL correction for constructive interference
        if let MessagePayload::ResonateAck(ref ack_data) = ack.payload {
            if ack_data.interference == "constructive" {
                if let Some(pll) = self.plls.get_mut(node_id) {
                    let node = self.nodes.get(node_id).unwrap();
                    let correction =
                        pll.compute_correction(node.current_phase, ack_data.coupled_phase);
                    if let Some(node) = self.nodes.get_mut(node_id) {
                        node.adjust_phase(correction);
                    }
                }
            }
        }
        self.message_log.push(ack.clone());
    }

    /// Process a DECOUPLE_REQ: break coupling for the sender.
    pub fn handle_decouple_req(
        &mut self,
        node_id: &str,
        msg: &Message,
        cycles_coupled: u64,
    ) -> Message {
        let restored_phase = if let Some(node) = self.nodes.get_mut(node_id) {
            node.decouple();
            // Reset PLL
            if let Some(pll) = self.plls.get_mut(node_id) {
                pll.reset();
            }
            node.sovereign_phase
        } else {
            0.5
        };

        self.message_log.push(msg.clone());
        let ack = Message::decouple_ack(node_id, restored_phase, cycles_coupled);
        self.message_log.push(ack.clone());
        ack
    }

    /// Run a negotiation among a set of participant nodes.
    ///
    /// Returns the consensus TritWord and whether a conflict was detected.
    pub fn negotiate(&mut self, participant_ids: &[String]) -> (TritWord, bool) {
        let participants: Vec<&Node> = participant_ids
            .iter()
            .filter_map(|id| self.nodes.get(id))
            .collect();

        if participants.is_empty() {
            return (TritWord::hold(Frame::Meta), false);
        }

        let frames: Vec<String> = participants
            .iter()
            .map(|n| format!("{}", n.frame))
            .collect();
        let phases: Vec<f64> = participants.iter().map(|n| n.current_phase).collect();
        let consensus_phase = phases.iter().sum::<f64>() / phases.len() as f64;

        // Check for cross-frame conflict
        let first_frame = &participants[0].frame;
        let has_cross_frame = participants.iter().any(|n| &n.frame != first_frame);

        let conflict_resolution = if has_cross_frame {
            "hold"
        } else {
            "commit_true"
        };

        // Record negotiation message
        let msg = Message::negotiate(
            "resonance-bus",
            participant_ids.to_vec(),
            frames,
            phases,
            conflict_resolution,
        );
        self.push_log(msg);

        let result = if has_cross_frame {
            TritWord::hold(Frame::Meta)
        } else {
            TritWord::new(TritValue::True, consensus_phase, Frame::Meta)
        };

        (result, has_cross_frame)
    }

    /// Get the message log as a reference.
    pub fn log(&self) -> &[Message] {
        &self.message_log
    }

    /// Get a node by id.
    pub fn get_node(&self, id: &str) -> Option<&Node> {
        self.nodes.get(id)
    }

    /// Push a message to the log, capping at MAX_MESSAGE_LOG (ring buffer).
    fn push_log(&mut self, msg: Message) {
        if self.message_log.len() >= MAX_MESSAGE_LOG {
            self.message_log.remove(0);
        }
        self.message_log.push(msg);
    }
}

impl Default for ResonanceBus {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::frame::Frame;
    use crate::net::node::{Node, NodeState};

    fn make_node(id: &str, frame: Frame, phase: f64) -> Node {
        Node::new(id.to_string(), frame, phase)
    }

    #[test]
    fn same_frame_resonance_constructive() {
        let mut bus = ResonanceBus::new();
        let node_a = make_node("a", Frame::Science, 0.7);
        let node_b = make_node("b", Frame::Science, 0.8);
        bus.register(node_a);
        bus.register(node_b);

        let req = Message::resonate_req("a", "Science", 0.7, vec![]);
        let ack = bus.handle_resonate_req("a", "b", &req);

        assert!(ack.is_some());
        if let MessagePayload::ResonateAck(ref data) = ack.unwrap().payload {
            assert_eq!(data.interference, "constructive");
            assert!(!data.conflict_detected);
            assert_eq!(data.recommendation, "commit");
            assert!((data.coupled_phase - 0.75).abs() < 0.01);
        } else {
            panic!("Expected ResonateAck");
        }
    }

    #[test]
    fn cross_frame_destructive_conflict() {
        let mut bus = ResonanceBus::new();
        let node_a = make_node("a", Frame::Science, 0.9);
        let node_b = make_node("b", Frame::Individual, 0.2);
        bus.register(node_a);
        bus.register(node_b);

        let req = Message::resonate_req("a", "Science", 0.9, vec![]);
        let ack = bus.handle_resonate_req("a", "b", &req);

        assert!(ack.is_some());
        if let MessagePayload::ResonateAck(ref data) = ack.unwrap().payload {
            assert_eq!(data.interference, "destructive");
            assert!(data.conflict_detected);
            assert_eq!(data.recommendation, "hold");
        } else {
            panic!("Expected ResonateAck");
        }
    }

    #[test]
    fn cross_frame_neutral_negotiation() {
        let mut bus = ResonanceBus::new();
        let node_a = make_node("a", Frame::Science, 0.6);
        let node_b = make_node("b", Frame::Consensus, 0.5);
        bus.register(node_a);
        bus.register(node_b);

        let req = Message::resonate_req("a", "Science", 0.6, vec![]);
        let ack = bus.handle_resonate_req("a", "b", &req);

        assert!(ack.is_some());
        if let MessagePayload::ResonateAck(ref data) = ack.unwrap().payload {
            assert_eq!(data.interference, "neutral");
            assert!(!data.conflict_detected);
            assert_eq!(data.recommendation, "negotiate");
        } else {
            panic!("Expected ResonateAck");
        }
    }

    #[test]
    fn decouple_restores_sovereign_phase() {
        let mut bus = ResonanceBus::new();
        let node = make_node("a", Frame::Science, 0.7);
        bus.register(node);

        // Simulate coupling that modified phase
        bus.nodes.get_mut("a").unwrap().current_phase = 0.55;
        bus.nodes.get_mut("a").unwrap().state = NodeState::Coupled;
        bus.nodes.get_mut("a").unwrap().cycles_coupled = 42;

        let req = Message::decouple_req("a", "user_disconnect");
        let ack = bus.handle_decouple_req("a", &req, 42);

        if let MessagePayload::DecoupleAck(ref data) = ack.payload {
            assert!((data.restored_phase - 0.7).abs() < f64::EPSILON);
            assert_eq!(data.cycles_coupled, 42);
        } else {
            panic!("Expected DecoupleAck");
        }

        let node = bus.get_node("a").unwrap();
        assert_eq!(node.state, NodeState::Sovereign);
        assert!((node.current_phase - node.sovereign_phase).abs() < f64::EPSILON);
    }

    #[test]
    fn three_node_negotiation_cross_frame_hold() {
        let mut bus = ResonanceBus::new();
        bus.register(make_node("a", Frame::Science, 0.75));
        bus.register(make_node("b", Frame::Individual, 0.35));
        bus.register(make_node("c", Frame::Consensus, 0.6));

        let (result, has_conflict) =
            bus.negotiate(&["a".to_string(), "b".to_string(), "c".to_string()]);

        assert!(has_conflict);
        assert_eq!(result.value, TritValue::Hold);
    }

    #[test]
    fn three_node_negotiation_same_frame_commits() {
        let mut bus = ResonanceBus::new();
        bus.register(make_node("a", Frame::Science, 0.7));
        bus.register(make_node("b", Frame::Science, 0.8));
        bus.register(make_node("c", Frame::Science, 0.6));

        let (result, has_conflict) =
            bus.negotiate(&["a".to_string(), "b".to_string(), "c".to_string()]);

        assert!(!has_conflict);
        assert_eq!(result.value, TritValue::True);
        assert!((result.phase.inner() - 0.7).abs() < 0.01);
    }
}
