// Distributed node protocol (M4).
//
// Implements T_RESONATE / T_DECOUPLE operations for multi-node
// harmonic coupling based on ADR-004.

pub mod bus;
pub mod message;
pub mod node;
pub mod pll;

pub use bus::ResonanceBus;
pub use message::{
    Message, MessageHeader, MessagePayload, NegotiatePayload, OpCode, ResonateAck, ResonateReq,
};
pub use node::{Interference, Node, NodeState};
pub use pll::PllController;
