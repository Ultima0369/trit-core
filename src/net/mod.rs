// Distributed node protocol (M4).
//
// Implements T_RESONATE / T_DECOUPLE operations for multi-node
// harmonic coupling based on ADR-004.
//
// Architecture:
//   node.rs     — Node struct, NodeState machine, sovereign identity
//   message.rs  — Protocol message types (RESONATE_REQ/ACK, DECOUPLE_REQ/ACK, NEGOTIATE)
//   pll.rs      — Software phase-locked loop controller
//   bus.rs      — ResonanceBus: in-memory message bus for local multi-node simulation

pub mod bus;
pub mod message;
pub mod node;
pub mod pll;

pub use bus::ResonanceBus;
pub use message::{Message, MessageHeader, NegotiatePayload, OpCode, ResonateAck, ResonateReq};
pub use node::{Node, NodeState};
pub use pll::PllController;
