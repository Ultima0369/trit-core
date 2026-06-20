//! Core ternary algebra layer.
//!
//! This module contains the foundational types of Trit-Core:
//! - [`TritValue`]: the four discrete ternary states.
//! - [`Phase`]: continuous tendency in `[0.0, 1.0]`.
//! - [`Frame`]: decision domain / reference frame.
//! - [`TritWord`]: composite ternary word with enforced invariants.
//! - [`TernaryAlgebra`]: `t_and`, `t_or`, `t_not`, and hot-path variants.

pub mod algebra;
pub mod decision_engine;
pub mod frame;
pub mod hold;
pub mod phase;
pub mod sensor;
pub mod value;
pub mod word;

pub use algebra::TernaryAlgebra;
pub use decision_engine::{DecisionEngine, DecisionResult};
pub use frame::{Frame, FrameError, FrameRegistry};
pub use hold::{HoldFinality, HoldState, HolderConfig};
pub use phase::{Commitment, Phase, PhaseError};
pub use sensor::{
    BodyState, CogState, EnvSnapshot, EnvironmentalContext, SensorSignal, TemporalScale, TextInput,
};
pub use value::TritValue;
pub use word::{Trit, TritWord, WordError};
