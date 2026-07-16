#![deny(clippy::undocumented_unsafe_blocks)]

mod bridge_script;

#[cfg(feature = "wry-backend")]
mod assets;
#[cfg(feature = "wry-backend")]
mod native_parent;
#[cfg(feature = "wry-backend")]
mod runtime;

pub use bridge_script::*;

#[cfg(feature = "wry-backend")]
pub use native_parent::*;
#[cfg(feature = "wry-backend")]
pub use runtime::*;

#[cfg(test)]
mod tests;
