use crate::frame::Frame;
use crate::meta::{ConflictType, MetaInterrupt, MetaMonitor};
use crate::trit::{TritValue, TritWord};
use uuid::Uuid;

/// Node state machine per ADR-004 §4.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NodeState {
    /// Independent oscillation, no peer coupling.
    Sovereign,
    /// RESONATE_REQ sent, awaiting ACK.
    Coupling,
    /// Phase-locked with one or more peers.
    Coupled,
    /// Negotiation detected unresolvable cross-frame conflict.
    Hold,
}

/// Sovereign node in a distributed Trit-Core network.
///
/// Each node has an immutable frame and a sovereign phase that represents
/// its independent oscillation. Coupling with peers temporarily adjusts
/// phase via PLL, but the sovereign phase is always preserved for decoupling.
#[derive(Debug, Clone)]
pub struct Node {
    pub id: String,
    pub frame: Frame,
    /// The node's intrinsic phase — never modified by coupling.
    pub sovereign_phase: f64,
    /// Current operational phase (may differ from sovereign when coupled).
    pub current_phase: f64,
    pub state: NodeState,
    /// IDs of currently coupled peers.
    pub peers: Vec<String>,
    /// Number of tick cycles spent in current coupling.
    pub cycles_coupled: u64,
    /// Conflict/interrupt log.
    pub monitor: MetaMonitor,
}

impl Node {
    pub fn new(id: String, frame: Frame, phase: f64) -> Self {
        Self {
            id,
            frame,
            sovereign_phase: phase,
            current_phase: phase,
            state: NodeState::Sovereign,
            peers: vec![],
            cycles_coupled: 0,
            monitor: MetaMonitor::new(crate::meta::ResolutionPolicy::new(
                crate::meta::Domain::General,
            )),
        }
    }

    /// Create a node with a random UUID id.
    pub fn with_uuid(frame: Frame, phase: f64) -> Self {
        Self::new(Uuid::new_v4().to_string(), frame, phase)
    }

    /// Begin coupling: transition Sovereign → Coupling.
    pub fn initiate_coupling(&mut self, peer_id: &str) {
        if self.state == NodeState::Sovereign {
            self.state = NodeState::Coupling;
            self.peers.push(peer_id.to_string());
        }
    }

    /// Confirm coupling: transition Coupling → Coupled.
    pub fn confirm_coupling(&mut self) {
        if self.state == NodeState::Coupling {
            self.state = NodeState::Coupled;
        }
    }

    /// Transition to Hold state (unresolvable conflict).
    pub fn enter_hold(&mut self, reason: &str) {
        self.state = NodeState::Hold;
        self.monitor.record(MetaInterrupt::new(
            ConflictType::FrameMismatch,
            reason.to_string(),
        ));
    }

    /// Decouple from all peers: transition any state → Sovereign.
    /// Restores sovereign phase and clears peer list.
    pub fn decouple(&mut self) {
        self.state = NodeState::Sovereign;
        self.current_phase = self.sovereign_phase;
        self.peers.clear();
        self.cycles_coupled = 0;
    }

    /// Apply a phase adjustment from PLL correction.
    /// Clamped to [0.0, 1.0].
    pub fn adjust_phase(&mut self, delta: f64) {
        self.current_phase = (self.current_phase + delta).clamp(0.0, 1.0);
    }

    /// Tick the coupling cycle counter.
    pub fn tick(&mut self) {
        if self.state == NodeState::Coupled {
            self.cycles_coupled += 1;
        }
    }

    /// Check if this node can couple with another based on frame compatibility.
    /// Same frame → constructive. Different frame with close phases → neutral.
    /// Different frame with divergent phases → destructive (conflict).
    pub fn interference_with(&self, other: &Node) -> Interference {
        if self.frame == other.frame {
            Interference::Constructive
        } else if (self.current_phase - other.current_phase).abs() > 0.3 {
            Interference::Destructive
        } else {
            Interference::Neutral
        }
    }

    /// Create a TritWord representing this node's current state.
    pub fn to_trit(&self) -> TritWord {
        TritWord::new(TritValue::Hold, self.current_phase, self.frame.clone())
    }
}

/// Result of coupling two nodes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Interference {
    /// Same frame — phases reinforce.
    Constructive,
    /// Different frame, close phases — can negotiate.
    Neutral,
    /// Different frame, divergent phases — conflict, must Hold.
    Destructive,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::frame::Frame;

    #[test]
    fn should_start_in_sovereign_state() {
        let node = Node::new("test".into(), Frame::Science, 0.5);
        assert_eq!(node.state, NodeState::Sovereign);
        assert_eq!(node.peers.len(), 0);
        assert_eq!(node.cycles_coupled, 0);
    }

    #[test]
    fn should_preserve_sovereign_phase_after_coupling_phase_change() {
        let mut node = Node::new("test".into(), Frame::Science, 0.7);
        node.current_phase = 0.55; // simulate coupling drift
        assert!((node.sovereign_phase - 0.7).abs() < f64::EPSILON);
        assert!((node.current_phase - 0.55).abs() < f64::EPSILON);
    }

    #[test]
    fn should_transition_sovereign_to_coupling_on_initiate() {
        let mut node = Node::new("test".into(), Frame::Science, 0.5);
        node.initiate_coupling("peer-a");
        assert_eq!(node.state, NodeState::Coupling);
        assert!(node.peers.contains(&"peer-a".to_string()));
    }

    #[test]
    fn should_not_initiate_coupling_unless_sovereign() {
        let mut node = Node::new("test".into(), Frame::Science, 0.5);
        node.state = NodeState::Coupled;
        node.initiate_coupling("peer-a");
        assert_eq!(node.state, NodeState::Coupled);
        assert!(node.peers.is_empty());
    }

    #[test]
    fn should_confirm_coupling_from_coupling_state() {
        let mut node = Node::new("test".into(), Frame::Science, 0.5);
        node.state = NodeState::Coupling;
        node.confirm_coupling();
        assert_eq!(node.state, NodeState::Coupled);
    }

    #[test]
    fn should_enter_hold_and_record_interrupt() {
        let mut node = Node::new("test".into(), Frame::Science, 0.5);
        node.enter_hold("test cross-frame conflict");
        assert_eq!(node.state, NodeState::Hold);
        assert_eq!(node.monitor.log().len(), 1);
        assert_eq!(node.monitor.log()[0].conflict, ConflictType::FrameMismatch);
    }

    #[test]
    fn should_decouple_and_restore_sovereign_phase() {
        let mut node = Node::new("test".into(), Frame::Science, 0.7);
        node.current_phase = 0.55;
        node.peers = vec!["peer-a".into(), "peer-b".into()];
        node.cycles_coupled = 100;
        node.decouple();
        assert_eq!(node.state, NodeState::Sovereign);
        assert!((node.current_phase - 0.7).abs() < f64::EPSILON);
        assert!(node.peers.is_empty());
        assert_eq!(node.cycles_coupled, 0);
    }

    #[test]
    fn should_clamp_phase_adjustment() {
        let mut node = Node::new("test".into(), Frame::Science, 0.9);
        node.adjust_phase(0.2); // 0.9 + 0.2 = 1.1 → clamped to 1.0
        assert!((node.current_phase - 1.0).abs() < f64::EPSILON);
        node.adjust_phase(-0.3);
        assert!((node.current_phase - 0.7).abs() < f64::EPSILON);
    }

    #[test]
    fn should_tick_only_in_coupled_state() {
        let mut node = Node::new("test".into(), Frame::Science, 0.5);
        node.tick();
        assert_eq!(node.cycles_coupled, 0);
        node.state = NodeState::Coupled;
        node.tick();
        node.tick();
        assert_eq!(node.cycles_coupled, 2);
    }

    #[test]
    fn should_detect_constructive_interference_same_frame() {
        let a = Node::new("a".into(), Frame::Science, 0.7);
        let b = Node::new("b".into(), Frame::Science, 0.8);
        assert_eq!(a.interference_with(&b), Interference::Constructive);
    }

    #[test]
    fn should_detect_destructive_interference_divergent_phases() {
        let a = Node::new("a".into(), Frame::Science, 0.1);
        let b = Node::new("b".into(), Frame::Individual, 0.9);
        assert_eq!(a.interference_with(&b), Interference::Destructive);
    }

    #[test]
    fn should_detect_neutral_interference_close_phases() {
        let a = Node::new("a".into(), Frame::Science, 0.5);
        let b = Node::new("b".into(), Frame::Consensus, 0.6);
        assert_eq!(a.interference_with(&b), Interference::Neutral);
    }

    #[test]
    fn to_trit_should_produce_hold_with_node_phase() {
        let node = Node::new("test".into(), Frame::Science, 0.65);
        let trit = node.to_trit();
        assert_eq!(trit.value, TritValue::Hold);
        assert!((trit.phase.inner() - 0.65).abs() < f64::EPSILON);
        assert_eq!(trit.frame, Frame::Science);
    }
}
