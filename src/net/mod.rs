// Distributed node protocol (M4).
//
// Implements T_RESONATE / T_DECOUPLE operations for multi-node
// harmonic coupling based on ADR-004.
//
// Sub-modules:
// - bus: ResonanceBus struct + node registration + message log
// - coupling: RESONATE_REQ/ACK + DECOUPLE_REQ handling
// - discovery: seed-based peer bootstrapping (M6)
// - frame_codec: TCP length-prefix framing protocol (M5)
// - message: protocol message types and constructors
// - negotiate: multi-node negotiation (single-pass)
// - node: Node state machine
// - pll: software phase-locked loop controller
// - tcp_client: TCP client connector (M5)
// - tcp_server: TCP node server (M5)

pub mod bus;
pub mod coupling;
pub mod discovery;
pub mod frame_codec;
pub mod message;
pub mod negotiate;
pub mod node;
pub mod pll;
pub mod tcp_client;
pub mod tcp_server;

pub use bus::{ResonanceBus, HEARTBEAT_TIMEOUT_SECS, SPLIT_BRAIN_TIMEOUT_SECS};
pub use frame_codec::{read_frame, write_frame, MAX_FRAME_SIZE};
pub use message::{
    Message, MessageHeader, MessagePayload, NegotiatePayload, OpCode, ResonateAck, ResonateReq,
};
pub use node::{Interference, Node, NodeState};
pub use pll::PllController;
pub use tcp_client::TcpClient;
pub use tcp_server::TcpNodeServer;
