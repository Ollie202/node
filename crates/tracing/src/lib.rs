//! Tracing and OpenTelemetry integration for Miden node.

extern crate self as miden_node_tracing;

mod field;
mod object;
mod span;

#[cfg(test)]
mod test_utils;

pub use field::OpenTelemetryField;
pub use miden_node_tracing_macro::instrument;
pub use object::{OpenTelemetryObject, OpenTelemetryObjectRecorder};
pub use span::Span;
#[doc(hidden)]
pub use tracing;
