//! Self-knowledge layer: model of the system's own response patterns.
//!
//! This module provides [`SelfKnowledge`], which uses the system's own
//! known patterns to estimate the likely cognitive state of a receiver
//! facing the same input.

pub mod self_model;

pub use self_model::{
    CalibrationEvent, ReceiverEstimate, ResponsePattern, SelfKnowledge, TriggerSignature,
};
