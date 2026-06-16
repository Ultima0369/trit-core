#![deny(warnings)]
#![forbid(unsafe_code)]

pub mod baseline;
pub mod clock;
pub mod frame;
pub mod meta;
pub mod sandbox;
pub mod trit;

pub use frame::Frame;
pub use meta::{
    ArbitrationResult, ConflictType, Domain, MetaInterrupt, MetaMonitor, ResolutionPolicy,
};
pub use trit::algebra::TernaryAlgebra;
pub use trit::phase::Commitment;
pub use trit::{Phase, TritValue, TritWord};
