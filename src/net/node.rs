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
