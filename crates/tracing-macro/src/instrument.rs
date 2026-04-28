use proc_macro::TokenStream;
use quote::{ToTokens, quote};
use syn::punctuated::Punctuated;
use syn::spanned::Spanned;
use syn::{ItemFn, Meta, Token, parse_macro_input};

use crate::target;

pub(crate) fn instrument(attr: TokenStream, item: TokenStream) -> TokenStream {
    let args = parse_macro_input!(attr with Punctuated::<Meta, Token![,]>::parse_terminated);
    let function = parse_macro_input!(item as ItemFn);

    let instrument_args = match instrument_args(args) {
        Ok(args) => args,
        Err(err) => return err.to_compile_error().into(),
    };

    expand_instrument(instrument_args, function).into()
}

fn instrument_args(args: Punctuated<Meta, Token![,]>) -> syn::Result<proc_macro2::TokenStream> {
    let mut target_seen = false;
    let mut args = args
        .into_iter()
        .map(|arg| {
            let ident = arg.path().get_ident().map(ToString::to_string);

            match ident.as_deref() {
                Some("name" | "level") => {
                    validate_name_value_arg(&arg, "`name` and `level`")?;
                    Ok(arg.into_token_stream())
                },
                Some("target") => {
                    let Meta::NameValue(meta) = arg else {
                        return Err(syn::Error::new_spanned(
                            arg,
                            "`target` must be specified as a name-value argument",
                        ));
                    };
                    if target_seen {
                        return Err(syn::Error::new_spanned(
                            meta,
                            "`target` may only be specified once",
                        ));
                    }
                    target_seen = true;

                    let target = target::parse(&meta.value)?;
                    let target = syn::LitStr::new(&target, meta.value.span());

                    Ok(quote! { target = #target })
                },
                Some("skip_all") => Err(syn::Error::new_spanned(
                    arg,
                    "`skip_all` is always applied by this macro",
                )),
                Some("skip") => Err(syn::Error::new_spanned(
                    arg,
                    "`skip` is not supported; this macro always skips all arguments",
                )),
                Some("fields") => Err(syn::Error::new_spanned(
                    arg,
                    "`fields` is not supported; record fields with `miden_node_tracing::Span`",
                )),
                Some("err") => Err(syn::Error::new_spanned(
                    arg,
                    "`err` is not supported; this macro records returned errors with `Span::record_error`",
                )),
                Some(_) => Err(syn::Error::new_spanned(
                    arg,
                    "unsupported instrument argument; only `name`, `target`, and `level` are supported",
                )),
                None => Err(syn::Error::new_spanned(arg, "unsupported instrument argument")),
            }
        })
        .collect::<syn::Result<Vec<_>>>()?;

    if !target_seen {
        return Err(syn::Error::new(
            proc_macro2::Span::call_site(),
            format!("`target` is required; expected one of: {}", target::allowed_targets()),
        ));
    }

    args.insert(0, quote! { skip_all });

    Ok(quote! { #(#args),* })
}

fn validate_name_value_arg(arg: &Meta, name: &str) -> syn::Result<()> {
    if matches!(arg, Meta::NameValue(_)) {
        Ok(())
    } else {
        Err(syn::Error::new_spanned(
            arg,
            format!("{name} must be specified as name-value arguments"),
        ))
    }
}

fn expand_instrument(
    instrument_args: proc_macro2::TokenStream,
    function: ItemFn,
) -> proc_macro2::TokenStream {
    let attrs = function.attrs;
    let vis = function.vis;
    let sig = function.sig;
    let block = function.block;
    let body = if sig.asyncness.is_some() {
        quote! {{
            let __miden_node_tracing_result = (async #block).await;
            if let ::core::result::Result::Err(ref __miden_node_tracing_error) =
                __miden_node_tracing_result
            {
                ::miden_node_tracing::Span::current().record_error(__miden_node_tracing_error);
            }
            __miden_node_tracing_result
        }}
    } else {
        quote! {{
            let __miden_node_tracing_result = (|| #block)();
            if let ::core::result::Result::Err(ref __miden_node_tracing_error) =
                __miden_node_tracing_result
            {
                ::miden_node_tracing::Span::current().record_error(__miden_node_tracing_error);
            }
            __miden_node_tracing_result
        }}
    };

    quote! {
        #(#attrs)*
        #[::miden_node_tracing::tracing::instrument(#instrument_args)]
        #vis #sig #body
    }
}

#[cfg(test)]
mod tests {
    use quote::quote;
    use syn::parse::Parser;
    use syn::punctuated::Punctuated;
    use syn::{Meta, Token};

    use super::instrument_args;

    fn parse_args(
        tokens: proc_macro2::TokenStream,
    ) -> syn::punctuated::Punctuated<Meta, Token![,]> {
        Punctuated::<Meta, Token![,]>::parse_terminated
            .parse2(tokens)
            .expect("test args should parse")
    }

    #[test]
    fn requires_target() {
        let err = instrument_args(parse_args(quote!(name = "test"))).unwrap_err();

        assert!(err.to_string().contains("`target` is required"));
    }

    #[test]
    fn rewrites_allowed_target_to_literal() {
        let args = instrument_args(parse_args(quote!(target = store::database, name = "test")))
            .unwrap()
            .to_string();

        assert!(args.contains("skip_all"));
        assert!(args.contains("target = \"store::database\""));
        assert!(args.contains("name = \"test\""));
    }
}
