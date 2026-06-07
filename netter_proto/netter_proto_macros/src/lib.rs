use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, ItemFn, ReturnType};

#[proc_macro_attribute]
pub fn async_callback(_attr: TokenStream, item: TokenStream) -> TokenStream {
    let mut input_fn = parse_macro_input!(item as ItemFn);

    if input_fn.sig.asyncness.is_none() {
        return syn::Error::new_spanned(&input_fn.sig, "Macro #[async_callback] can be set only on `async fn`")
            .to_compile_error()
            .into();
    }

    input_fn.sig.asyncness = None;

    let orig_return_type = match &input_fn.sig.output {
        ReturnType::Default => quote! { () },
        ReturnType::Type(_, ty) => quote! { #ty },
    };

    input_fn.sig.output = syn::parse_quote! {
        -> ::netter_proto::BoxFuture<'static, #orig_return_type>
    };

    let fn_body = &input_fn.block;
    input_fn.block = syn::parse_quote! {
        {
            Box::pin(async move #fn_body)
        }
    };

    TokenStream::from(quote! {
        #input_fn
    })
}