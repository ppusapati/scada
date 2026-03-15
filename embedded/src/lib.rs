#![cfg_attr(feature = "stm32h743", no_std)]

#[cfg(feature = "stm32h743")]
extern crate alloc;

// Simulator modules (existing, runs on host PC)
#[cfg(feature = "simulator")]
pub mod actuators;
#[cfg(feature = "simulator")]
pub mod config;
#[cfg(feature = "simulator")]
pub mod mqtt;
#[cfg(feature = "simulator")]
pub mod sensors;

// STM32H743 hardware modules (no_std, Embassy-based)
#[cfg(feature = "stm32h743")]
pub mod hal;
#[cfg(feature = "stm32h743")]
pub mod storage;
#[cfg(feature = "stm32h743")]
pub mod comm;
#[cfg(feature = "stm32h743")]
pub mod datalogger;
