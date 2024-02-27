//! Platform-specific functionality.

#[cfg(not(target_arch = "wasm32"))]
pub mod native;
