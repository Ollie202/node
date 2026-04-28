use proc_macro::TokenStream;
use quote::{ToTokens, quote};
use syn::punctuated::Punctuated;
use syn::{ItemFn, Meta, Token, parse_macro_input};

pub(crate) fn instrument(attr: TokenStream, item: TokenStream) -> TokenStream {
    let args = parse_macro_input!(attr with Punctuated::<Meta, Token![,]>::parse_terminated);
    let function = parse_macro_input!(item as ItemFn);

    if let Err(err) = validate_args(&args) {
        return err.to_compile_error().into();
    }

    expand_instrument(args, function).into()
}

fn validate_args(args: &Punctuated<Meta, Token![,]>) -> syn::Result<()> {
    for arg in args {
        let Some(ident) = arg.path().get_ident() else {
            return Err(syn::Error::new_spanned(arg, "unsupported instrument argument"));
        };

        match ident.to_string().as_str() {
            "name" | "target" | "level" => {
                if !matches!(arg, Meta::NameValue(_)) {
                    return Err(syn::Error::new_spanned(
                        arg,
                        "`name`, `target`, and `level` must be specified as name-value arguments",
                    ));
                }
            },
            "skip_all" => {
                return Err(syn::Error::new_spanned(
                    arg,
                    "`skip_all` is always applied by this macro",
                ));
            },
            "skip" => {
                return Err(syn::Error::new_spanned(
                    arg,
                    "`skip` is not supported; this macro always skips all arguments",
                ));
            },
            "fields" => {
                return Err(syn::Error::new_spanned(
                    arg,
                    "`fields` is not supported; record fields with `miden_node_tracing::Span`",
                ));
            },
            "err" => {
                return Err(syn::Error::new_spanned(
                    arg,
                    "`err` is not supported; this macro records returned errors with `Span::record_error`",
                ));
            },
            _ => {
                return Err(syn::Error::new_spanned(
                    arg,
                    "unsupported instrument argument; only `name`, `target`, and `level` are supported",
                ));
            },
        }
    }

    Ok(())
}

fn expand_instrument(
    args: Punctuated<Meta, Token![,]>,
    function: ItemFn,
) -> proc_macro2::TokenStream {
    let attrs = function.attrs;
    let vis = function.vis;
    let sig = function.sig;
    let block = function.block;
    let instrument_args = instrument_args(args);
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

fn instrument_args(args: Punctuated<Meta, Token![,]>) -> proc_macro2::TokenStream {
    if args.is_empty() {
        quote! { skip_all }
    } else {
        let args = args.into_iter().map(|arg| arg.into_token_stream());
        quote! { skip_all, #(#args),* }
    }
}
