// Distributed node protocol (M4).
//
// Implements T_RESONATE / T_DECOUPLE operations for multi-node
// harmonic coupling based on ADR-004.
//
// Sub-modules:
// - bus: ResonanceBus struct + node registration + message log
// - coupling: RESONATE_REQ/ACK + DECOUPLE_REQ handling
// - negotiate: multi-node negotiation (single-pass)
// - message: protocol message types and constructors
// - node: Node state machine
// - pll: software phase-locked loop controller

pub mod bus;
pub mod coupling;
pub mod message;
pub mod negotiate;
pub mod node;
pub mod pll;

pub use bus::ResonanceBus;
pub use message::{
    Message, MessageHeader, MessagePayload, NegotiatePayload, OpCode, ResonateAck, ResonateReq,
};
pub use node::{Interference, Node, NodeState};
pub use pll::PllController;
