//! Attention scheduling layer: decide when to shift, hold, or recalibrate.
//!
//! This module provides [`AttentionScheduler`], which monitors recent
//! signal patterns and cognitive load to suggest attention commands.

pub mod scheduler;

pub use scheduler::{
    bandwidth_from_depth, AttentionCmd, AttentionScheduler, LoadProfile, ShiftTarget,
};
