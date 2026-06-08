use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, ItemFn};
use syn::parse::{Parse, ParseStream};

struct WorkerArgs {
    worker_type: syn::Ident,
}

impl Parse for WorkerArgs {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let worker_type: syn::Ident = input.parse()?;
        Ok(Self {
            worker_type,
        })
    }
}

#[proc_macro_attribute]
pub fn worker(attr: TokenStream, item: TokenStream) -> TokenStream {
    let input = parse_macro_input!(item as ItemFn);

    let args = parse_macro_input!(attr as WorkerArgs);

    let is_main = if args.worker_type == "main" { true } else { false };

    let vis = &input.vis;
    let name = &input.sig.ident;
    let generics = &input.sig.generics;
    let inputs = &input.sig.inputs;
    let output = &input.sig.output;
    let block = &input.block;

    let worker_name_str = name.to_string();

    let entrypoint_ident = syn::Ident::new("_cesium_entrypoint", proc_macro2::Span::call_site());

    let entrypoint_code = if is_main {
        quote! {
            #[no_mangle]
            pub extern "C" fn #entrypoint_ident(ptr: *const u8, len: usize) -> u64 {
                crate::init_worker_context(#worker_name_str);

                let context_slice = unsafe {
                    if ptr.is_null() || len == 0 { &[] } else { std::slice::from_raw_parts(ptr, len) }
                };

                let output = #name(context_slice);

                let logs = ::cesium_sdk::collect_logs_to_bytes();

                ::cesium_sdk::prepare_response(
                    ::cesium_sdk::STATUS_SUCCESS,
                    0,
                    output,
                    logs
                )
            }
        }
    } else {
        quote! {}
    };

    let expanded = quote! {
        #vis fn #name #generics(#inputs) #output {
            #block
        }

        #entrypoint_code
    };

    TokenStream::from(expanded)
}