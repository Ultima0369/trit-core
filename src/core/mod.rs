//! Core ternary algebra layer.
//!
//! This module contains the foundational types of Trit-Core:
//! - [`TritValue`]: the four discrete ternary states.
//! - [`Phase`]: continuous tendency in `[0.0, 1.0]`.
//! - [`Frame`]: decision domain / reference frame.
//! - [`TritWord`]: composite ternary word with enforced invariants.
//! - [`TernaryAlgebra`]: `t_and`, `t_or`, `t_not`, and hot-path variants.
//! - [`Domain`]: epistemic domain classification.
//! - [`MetaInterrupt`], [`ConflictType`], [`MetaMonitor`]: core monitoring types.

pub mod algebra;
pub mod domain;
pub mod frame;
pub mod hold;
pub mod interrupt;
pub mod phase;
pub mod sensor;
pub mod value;
pub mod word;

pub use algebra::TernaryAlgebra;
pub use domain::{Domain, DomainParseError};
pub use frame::{Frame, FrameError};
pub use hold::{HoldFinality, HoldState, HolderConfig};
pub use interrupt::{
    CognitiveOffload, ConflictType, HoldReason, MetaInterrupt, MetaMonitor, PolicyViolation,
    SourceConflict, MAX_INTERRUPT_LOG,
};
pub use phase::{Commitment, Phase, PhaseError, PhaseTracker, Trend};
pub use sensor::{
    BodyState, CogState, EnvSnapshot, EnvironmentalContext, SensorSignal, TemporalScale, TextInput,
};
pub use value::TritValue;
pub use word::{Trit, TritWord, WordError};
