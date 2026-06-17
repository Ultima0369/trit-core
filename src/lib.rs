#![deny(warnings)]
#![forbid(unsafe_code)]

pub mod baseline;
pub mod clock;
pub mod frame;
pub mod meta;
pub mod net;
pub mod sandbox;
pub mod tracing_init;
pub mod trit;

pub use frame::Frame;
pub use meta::{
    ArbitrationResult, ConflictType, CustomRule, Domain, JsonRuleLoader, MetaInterrupt,
    MetaMonitor, ResolutionPolicy, RuleLoader, SafeFallback,
};
pub use net::{
    bus::ResonanceBus,
    message::{Message, MessageHeader, MessagePayload, OpCode},
    node::{Interference, Node, NodeState},
    pll::PllController,
};
pub use sandbox::{SandboxOutput, ScenarioInput};
pub use trit::algebra::TernaryAlgebra;
pub use trit::phase::Commitment;
pub use trit::{Phase, TritValue, TritWord};
