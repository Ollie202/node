use syn::parse::ParseStream;
use syn::{Ident, Token};

const USER_ARG: &str = "user";

// `user` is deliberately a marker rather than a name-value option. Span/event names carry the
// action text; this flag only opts the telemetry item into a user-facing log path.
pub(crate) fn try_parse_marker(input: ParseStream<'_>) -> syn::Result<bool> {
    let fork = input.fork();
    if !fork.peek(Ident) {
        return Ok(false);
    }

    let ident = fork.parse::<Ident>()?;
    if ident != USER_ARG {
        return Ok(false);
    }
    if fork.peek(Token![=]) || fork.peek(Token![:]) {
        return Err(syn::Error::new_spanned(ident, "`user` is a bare marker"));
    }

    input.parse::<Ident>()?;
    Ok(true)
}
