use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, ItemFn, ReturnType};

/// Procedural macro attribute for creating asynchronous gRPC callbacks.
///
/// This macro hides the low-level routine of manually packing asynchronous code
/// into dynamic pointers [`BoxFuture`](netter_proto::BoxFuture) and [`Box::pin`].
///
/// # How it works
/// At compile time, the macro performs the following actions:
/// 1. Strips the `async` keyword from your function signature.
/// 2. Wraps the original function return type in `Pin<Box<dyn Future<Output = ...> + Send + 'static>>`.
/// 3. Wraps the entire function body in a `Box::pin(async move { ... })` block.
///
/// # Example
///
/// ```rust
/// #[async_callback]
/// async fn start_server_handler(
///     _ctx: Arc<Context>,
///     _req: StartServerRequest
/// ) -> Result<StartServerResponse, Status> {
///     tokio::time::sleep(std::time::Duration::from_millis(250)).await;
///
///     Ok(StartServerResponse { server_id: 2 })
/// }
/// ```
///
/// # Compile-time Errors
/// The macro will throw a compile-time error if you try to attach it to a
/// synchronous function (without the `async` keyword).
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