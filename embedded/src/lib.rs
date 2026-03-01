#![cfg_attr(not(feature = "simulator"), no_std)]

// ── Embedded firmware modules (no_std) ──
pub mod drivers;
pub mod hal;
pub mod net;
pub mod tasks;

// ── Simulator / desktop modules (std) ──
#[cfg(feature = "simulator")]
pub mod actuators;
#[cfg(feature = "simulator")]
pub mod config;
#[cfg(feature = "simulator")]
pub mod mqtt;
#[cfg(feature = "simulator")]
pub mod sensors;
