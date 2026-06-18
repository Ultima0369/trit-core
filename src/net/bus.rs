use std::collections::VecDeque;
use std::time::Instant;

use crate::net::gate::{ByzantineGatekeeper, GateRejection};
use crate::net::message::Message;
use crate::net::node::Node;
use crate::net::pll::PllController;

/// Maximum number of messages retained in the log (ring buffer).
pub(crate) const MAX_MESSAGE_LOG: usize = 10_000;
/// Maximum number of registered nodes.
pub(crate) const MAX_NODES: usize = 256;

/// Heartbeat timeout in seconds (M7).
/// Peers without a heartbeat within this window are considered dead.
pub const HEARTBEAT_TIMEOUT_SECS: u64 = 30;
/// Split-brain detection timeout in seconds (M7).
/// Two nodes that both think they're coupled but haven't communicated
/// for this long are in a split-brain scenario.
pub const SPLIT_BRAIN_TIMEOUT_SECS: u64 = 60;

/// In-memory message bus for local multi-node simulation.
///
/// Routes messages between nodes, applies PLL corrections,
/// and manages the coupling lifecycle. Message log is a capped
/// VecDeque ring buffer for O(1) push/pop at both ends.
///
/// ## M7 Extensions
///
/// - `last_heartbeat`: per-node heartbeat timestamps for dead peer detection
/// - `stale_peers()`: return peer IDs whose heartbeat has timed out
/// - `detect_split_brain()`: detect mutual-coupling-without-communication pairs
/// - `record_heartbeat()`: update a node's last heartbeat timestamp
pub struct ResonanceBus {
    /// All registered nodes indexed by id.
    pub nodes: std::collections::HashMap<String, Node>,
    /// PLL controllers per node id.
    pub plls: std::collections::HashMap<String, PllController>,
    /// Message log for audit trail (capped ring buffer, O(1) push/pop).
    pub message_log: VecDeque<Message>,
    /// Last heartbeat time per node id (M7).
    pub last_heartbeat: std::collections::HashMap<String, Instant>,
    /// Optional Byzantine gatekeeper (M8). When None, validation is skipped.
    pub gatekeeper: Option<ByzantineGatekeeper>,
}

impl ResonanceBus {
    pub fn new() -> Self {
        Self {
            nodes: std::collections::HashMap::new(),
            plls: std::collections::HashMap::new(),
            message_log: VecDeque::new(),
            last_heartbeat: std::collections::HashMap::new(),
            gatekeeper: None,
        }
    }

    /// Create a new ResonanceBus with a Byzantine gatekeeper (M8).
    pub fn with_gatekeeper(gk: ByzantineGatekeeper) -> Self {
        Self {
            nodes: std::collections::HashMap::new(),
            plls: std::collections::HashMap::new(),
            message_log: VecDeque::new(),
            last_heartbeat: std::collections::HashMap::new(),
            gatekeeper: Some(gk),
        }
    }

    /// Create a new ResonanceBus with a default gatekeeper (M8).
    pub fn with_default_gatekeeper() -> Self {
        Self::with_gatekeeper(ByzantineGatekeeper::default())
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
        let node_id = node.id.clone();
        if let Some(ref mut gk) = self.gatekeeper {
            gk.register_node(&node_id);
        }
        self.last_heartbeat.insert(node_id.clone(), Instant::now());
        self.plls.insert(node_id.clone(), PllController::new());
        self.nodes.insert(node_id, node);
    }

    /// Record a heartbeat from a node (M7).
    ///
    /// Called whenever a HEARTBEAT message is received. Updates the
    /// node's last-seen timestamp.
    pub fn record_heartbeat(&mut self, node_id: &str) {
        self.last_heartbeat
            .insert(node_id.to_string(), Instant::now());
    }

    /// Return the IDs of peers whose last heartbeat is older than
    /// `HEARTBEAT_TIMEOUT_SECS` (M7).
    ///
    /// These peers are considered dead/disconnected and should be
    /// decoupled.
    pub fn stale_peers(&self) -> Vec<String> {
        let now = Instant::now();
        self.last_heartbeat
            .iter()
            .filter(|(_id, instant)| {
                now.duration_since(**instant).as_secs() > HEARTBEAT_TIMEOUT_SECS
            })
            .map(|(id, _)| id.clone())
            .collect()
    }

    /// Detect split-brain pairs (M7).
    ///
    /// A split-brain occurs when two nodes both have each other in their
    /// peer lists, but neither has received a heartbeat from the other
    /// within `SPLIT_BRAIN_TIMEOUT_SECS`.
    ///
    /// Returns a list of (node_a, node_b) pairs that are split-brained.
    pub fn detect_split_brain(&self) -> Vec<(String, String)> {
        let now = Instant::now();
        let mut pairs = Vec::new();
        let mut seen = std::collections::HashSet::new();

        for (id_a, node_a) in &self.nodes {
            for peer_id in &node_a.peers {
                // Avoid reporting each pair twice
                let key = if id_a < peer_id {
                    (id_a.clone(), peer_id.clone())
                } else {
                    (peer_id.clone(), id_a.clone())
                };
                if seen.contains(&key) {
                    continue;
                }
                seen.insert(key.clone());

                // Check if peer also has us in their peer list
                if let Some(node_b) = self.nodes.get(peer_id) {
                    if node_b.peers.contains(id_a) {
                        // Both claim to be coupled — check last heartbeat for both
                        let a_stale = self
                            .last_heartbeat
                            .get(id_a)
                            .map(|t| now.duration_since(*t).as_secs() > SPLIT_BRAIN_TIMEOUT_SECS)
                            .unwrap_or(true);
                        let b_stale = self
                            .last_heartbeat
                            .get(peer_id)
                            .map(|t| now.duration_since(*t).as_secs() > SPLIT_BRAIN_TIMEOUT_SECS)
                            .unwrap_or(true);

                        if a_stale || b_stale {
                            pairs.push(key);
                        }
                    }
                }
            }
        }

        pairs
    }

    /// Force-decouple stale peers from all nodes (M7).
    ///
    /// Removes stale peer IDs from every node's peer list and returns
    /// the list of nodes that were affected.
    pub fn purge_stale_peers(&mut self) -> Vec<String> {
        let stale = self.stale_peers();
        let mut affected = Vec::new();

        for (id, node) in &mut self.nodes {
            let before = node.peers.len();
            node.peers.retain(|p| !stale.contains(p));
            if node.peers.len() < before {
                // If no peers left and was Coupled, return to Sovereign
                if node.peers.is_empty()
                    && (node.state == crate::net::node::NodeState::Coupled
                        || node.state == crate::net::node::NodeState::Coupling)
                {
                    node.decouple();
                }
                affected.push(id.clone());
            }
        }

        // Also remove stale entries from last_heartbeat
        for stale_id in &stale {
            self.last_heartbeat.remove(stale_id);
        }

        affected
    }

    /// Get the message log as an iterator.
    pub fn log(&self) -> std::collections::vec_deque::Iter<'_, Message> {
        self.message_log.iter()
    }

    /// Get a node by id.
    pub fn get_node(&self, id: &str) -> Option<&Node> {
        self.nodes.get(id)
    }

    /// Get a mutable reference to a node by id.
    pub fn get_node_mut(&mut self, id: &str) -> Option<&mut Node> {
        self.nodes.get_mut(id)
    }

    /// Remove a node and all its associated state (M8).
    ///
    /// Removes the node from nodes, PLLs, last_heartbeat, and the gatekeeper's
    /// known_nodes list. Use this when a node is permanently evicted (e.g.,
    /// after repeated Byzantine behavior).
    pub fn purge_node(&mut self, node_id: &str) {
        self.nodes.remove(node_id);
        self.plls.remove(node_id);
        self.last_heartbeat.remove(node_id);
        if let Some(ref mut gk) = self.gatekeeper {
            gk.unregister_node(node_id);
        }
    }

    /// Validate an incoming message through the gatekeeper (M8).
    ///
    /// If no gatekeeper is configured, all messages pass.
    /// If a gatekeeper is configured, runs all Byzantine checks.
    /// Returns the gatekeeper's rejection reason on failure.
    pub fn validate_incoming(&mut self, msg: &Message) -> Result<(), GateRejection> {
        match self.gatekeeper {
            Some(ref mut gk) => gk.validate(msg),
            None => Ok(()),
        }
    }

    /// Push a message to the log, capping at MAX_MESSAGE_LOG.
    /// Uses VecDeque for O(1) amortized push/pop at both ends.
    pub(crate) fn push_log(&mut self, msg: Message) {
        // Track per-peer log entries for gatekeeper
        if let Some(ref mut gk) = self.gatekeeper {
            gk.record_log_entry(&msg.header.sender);
        }
        if self.message_log.len() >= MAX_MESSAGE_LOG {
            if let Some(old) = self.message_log.pop_front() {
                if let Some(ref mut gk) = self.gatekeeper {
                    gk.prune_log_entry(&old.header.sender);
                }
            }
        }
        self.message_log.push_back(msg);
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
    use crate::net::node::NodeState;

    #[test]
    fn record_heartbeat_updates_timestamp() {
        let mut bus = ResonanceBus::new();
        let node = Node::new("test".into(), Frame::Science, 0.5);
        bus.register(node);

        // Initially, heartbeat was just set
        assert!(bus.last_heartbeat.contains_key("test"));

        // Record a new heartbeat
        std::thread::sleep(std::time::Duration::from_millis(10));
        bus.record_heartbeat("test");

        // Should still be fresh (not stale)
        let stale = bus.stale_peers();
        assert!(!stale.contains(&"test".to_string()));
    }

    #[test]
    fn stale_peers_returns_empty_when_all_fresh() {
        let mut bus = ResonanceBus::new();
        bus.register(Node::new("a".into(), Frame::Science, 0.5));
        bus.register(Node::new("b".into(), Frame::Individual, 0.3));

        let stale = bus.stale_peers();
        assert!(stale.is_empty());
    }

    #[test]
    fn detect_split_brain_returns_pairs() {
        let mut bus = ResonanceBus::new();

        let mut node_a = Node::new("a".into(), Frame::Science, 0.7);
        node_a.state = NodeState::Coupled;
        node_a.peers = vec!["b".to_string()];
        bus.register(node_a);

        let mut node_b = Node::new("b".into(), Frame::Science, 0.8);
        node_b.state = NodeState::Coupled;
        node_b.peers = vec!["a".to_string()];
        bus.register(node_b);

        // Simulate stale heartbeat for both nodes (manually backdate)
        let stale_time = Instant::now()
            .checked_sub(std::time::Duration::from_secs(SPLIT_BRAIN_TIMEOUT_SECS + 1))
            .unwrap();
        bus.last_heartbeat.insert("a".into(), stale_time);
        bus.last_heartbeat.insert("b".into(), stale_time);

        let pairs = bus.detect_split_brain();
        assert!(!pairs.is_empty());
        // Should find (a, b) or (b, a)
        let found = pairs
            .iter()
            .any(|(x, y)| (x == "a" && y == "b") || (x == "b" && y == "a"));
        assert!(found, "Expected split-brain pair not found: {:?}", pairs);
    }

    #[test]
    fn purge_stale_peers_restores_sovereign() {
        let mut bus = ResonanceBus::new();

        let mut node_a = Node::new("a".into(), Frame::Science, 0.7);
        node_a.state = NodeState::Coupled;
        node_a.peers = vec!["b".to_string()];
        bus.register(node_a);

        bus.register(Node::new("b".into(), Frame::Science, 0.8));

        // Make "b" stale by backdating its heartbeat
        let stale_time = Instant::now()
            .checked_sub(std::time::Duration::from_secs(HEARTBEAT_TIMEOUT_SECS + 5))
            .unwrap();
        bus.last_heartbeat.insert("b".into(), stale_time);

        let affected = bus.purge_stale_peers();
        assert!(affected.contains(&"a".to_string()));

        let node_a = bus.get_node("a").unwrap();
        assert!(node_a.peers.is_empty());
        assert_eq!(node_a.state, NodeState::Sovereign);
    }
}
