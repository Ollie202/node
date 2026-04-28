use proc_macro::TokenStream;

mod instrument;

/// Instruments a function with Miden tracing defaults.
///
/// This macro delegates span creation to `tracing::instrument`, but always applies `skip_all`,
/// rejects field and error recording options, and records returned errors through
/// `miden_node_tracing::Span::record_error`.
#[proc_macro_attribute]
pub fn instrument(attr: TokenStream, item: TokenStream) -> TokenStream {
    instrument::instrument(attr, item)
}
