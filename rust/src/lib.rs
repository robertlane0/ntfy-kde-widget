//! ntfy subscription engine for the KDE Plasma widget.
//!
//! All network and state handling lives here; the Qt layer only forwards
//! commands and renders the JSON events this crate emits.

#![forbid(unsafe_op_in_unsafe_fn)]

pub mod client;
pub mod engine;
pub mod ffi;
pub mod json;
pub mod model;
pub mod store;
pub mod util;