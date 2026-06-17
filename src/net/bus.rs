use std::collections::VecDeque;

use crate::net::message::Message;
use crate::net::node::Node;
use crate::net::pll::PllController;

/// Maximum number of messages retained in the log (ring buffer).
pub(crate) const MAX_MESSAGE_LOG: usize = 10_000;
/// Maximum number of registered nodes.
pub(crate) const MAX_NODES: usize = 256;

/// In-memory message bus for local multi-node simulation.
///
/// Routes messages between nodes, applies PLL corrections,
/// and manages the coupling lifecycle. Message log is a capped
/// VecDeque ring buffer for O(1) push/pop at both ends.
pub struct ResonanceBus {
    /// All registered nodes indexed by id.
    pub nodes: std::collections::HashMap<String, Node>,
    /// PLL controllers per node id.
    pub plls: std::collections::HashMap<String, PllController>,
    /// Message log for audit trail (capped ring buffer, O(1) push/pop).
    pub message_log: VecDeque<Message>,
}

impl ResonanceBus {
    pub fn new() -> Self {
        Self {
            nodes: std::collections::HashMap::new(),
            plls: std::collections::HashMap::new(),
            message_log: VecDeque::new(),
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

    /// Get the message log as an iterator.
    pub fn log(&self) -> std::collections::vec_deque::Iter<'_, Message> {
        self.message_log.iter()
    }

    /// Get a node by id.
    pub fn get_node(&self, id: &str) -> Option<&Node> {
        self.nodes.get(id)
    }

    /// Push a message to the log, capping at MAX_MESSAGE_LOG.
    /// Uses VecDeque for O(1) amortized push/pop at both ends.
    pub(crate) fn push_log(&mut self, msg: Message) {
        if self.message_log.len() >= MAX_MESSAGE_LOG {
            self.message_log.pop_front();
        }
        self.message_log.push_back(msg);
    }
}

impl Default for ResonanceBus {
    fn default() -> Self {
        Self::new()
    }
}
