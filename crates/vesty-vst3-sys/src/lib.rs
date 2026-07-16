//! Binding selection metadata for Vesty's VST3 adapter.
//!
//! The MVP uses the upstream `vst3` crate as the active Rust binding source.
//! This crate reserves the `vesty-vst3-sys` layer required by the architecture so
//! Vesty can add generated bindings from Steinberg headers without changing the
//! safe `vesty-vst3` API surface.

#![forbid(unsafe_code)]

mod backend;
mod baseline;
mod constants;
mod emit;
mod emit_interfaces;
mod helpers;
mod models;
mod plan;
mod probe;
mod surface;

pub use backend::*;
pub use baseline::*;
pub use constants::*;
pub use emit::*;
pub use emit_interfaces::*;
pub use models::*;
pub use plan::*;
pub use probe::*;
pub use surface::*;

pub(crate) use helpers::*;

#[cfg(test)]
mod tests;
