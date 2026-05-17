extern crate proc_macro;

use proc_macro::TokenStream;
use quote::{quote, ToTokens};
use syn::{parse_macro_input, FnArg, ItemFn, Pat, PatType, Type, ReturnType, Error};

#[proc_macro_attribute]
pub fn netter_plugin(_attr: TokenStream, item: TokenStream) -> TokenStream {
    let input_fn = parse_macro_input!(item as ItemFn);

    let fn_vis = &input_fn.vis;
    let fn_sig = &input_fn.sig;
    let fn_name = &fn_sig.ident;
    let fn_name_str = fn_name.to_string();
    let fn_body = &input_fn.block;
    let fn_inputs = &fn_sig.inputs;
    let fn_output = &fn_sig.output;

    match fn_output {
        ReturnType::Type(_, ty) => {
            let type_str = ty.to_token_stream().to_string();
            if !type_str.contains("Result") {
                return Error::new_spanned(fn_output, "#[netter_plugin] function must return a Result")
                    .to_compile_error().into();
            }
        }
        ReturnType::Default => {
            return Error::new_spanned(fn_sig, "#[netter_plugin] function must return a Result")
                .to_compile_error().into();
        }
    }

    let internal_fn_name = quote::format_ident!("_internal_{}", fn_name);
    let internal_fn = quote! {
        #fn_vis fn #internal_fn_name(#fn_inputs) #fn_output {
            #fn_body
        }
    };

    let mut arg_parsers = Vec::new();
    let mut arg_names_for_call = Vec::new();
    let expected_arg_count = fn_inputs.len();

    for (index, arg) in fn_inputs.iter().enumerate() {
         if let FnArg::Typed(PatType { pat, ty, .. }) = arg {
            if let Pat::Ident(pat_ident) = &**pat {
                let arg_name = &pat_ident.ident;
                arg_names_for_call.push(arg_name.clone());

                let type_path = if let Type::Path(type_path) = &**ty { type_path }
                else if let Type::Reference(type_ref) = &**ty {
                    if let Type::Path(type_path) = &*type_ref.elem { type_path }
                    else { return Error::new_spanned(ty, "Unsupported argument type reference").to_compile_error().into(); }
                }
                else { return Error::new_spanned(ty, "Unsupported argument type").to_compile_error().into(); };
                let type_ident = type_path.path.segments.last().map(|seg| &seg.ident);

                let parser_code = match type_ident.map(|id| id.to_string()).as_deref() {
                    Some("String") => quote! {
                        let #arg_name: String = match rdl_args_vec.get(#index)
                            .ok_or_else(|| format!("Missing argument #{}", #index))? {
                                RDLTypes::String(s) => s.clone(),
                                other => return Err(format!("Argument #{} must be a String, got {:?}", #index, other)),
                            };
                    },
                    Some("str") => quote! {
                        let temp_arg_string_for_ref = match rdl_args_vec.get(#index)
                            .ok_or_else(|| format!("Missing argument #{}", #index))? {
                                RDLTypes::String(s) => s,
                                other => return Err(format!("Argument #{} must be a String slice, got {:?}", #index, other)),
                            };
                        let #arg_name: &str = temp_arg_string_for_ref.as_str();
                    },
                    Some("i32") | Some("i64") | Some("isize") => quote! {
                        let #arg_name = match rdl_args_vec.get(#index)
                            .ok_or_else(|| format!("Missing argument #{}", #index))? {
                                RDLTypes::Number(n) => *n as #type_ident,
                                other => return Err(format!("Argument #{} must be a Number, got {:?}", #index, other)),
                            };
                    },
                    Some("bool") => quote! {
                        let #arg_name = match rdl_args_vec.get(#index)
                            .ok_or_else(|| format!("Missing argument #{}", #index))? {
                                RDLTypes::Boolean(b) => *b,
                                other => return Err(format!("Argument #{} must be a Boolean, got {:?}", #index, other)),
                            };
                    },
                    _ => {
                        let type_str = ty.to_token_stream().to_string();
                        return Error::new_spanned(ty, format!("Unsupported argument type for RDL FFI dispatch: {}", type_str))
                            .to_compile_error().into();
                    }
                };
                arg_parsers.push(parser_code);
            } else { return Error::new_spanned(pat, "Unsupported argument pattern").to_compile_error().into(); }
         } else { return Error::new_spanned(arg, "Unsupported argument type (e.g., self)").to_compile_error().into(); }
    }

    let ctor_fn_name = quote::format_ident!("_register_{}", fn_name);
    let registration_code = quote! {
        #[ctor::ctor]
        fn #ctor_fn_name() {
            let function_name = #fn_name_str.to_string();
            let function_name_clone = function_name.clone();

            let handler: DispatchableFn = Box::new(move |rdl_args_vec: Vec<RDLTypes>| -> Result<RDLTypes, String> {
                if rdl_args_vec.len() != #expected_arg_count {
                    return Err(format!("Function '{}' expects {} arguments, but received {}", function_name_clone, #expected_arg_count, rdl_args_vec.len()));
                }

                #( #arg_parsers )*
                
                #internal_fn_name(#(#arg_names_for_call),*).map(|res| res.into())
            });

            let mut registry = PLUGIN_REGISTRY.get_or_init(std::collections::HashMap::new);
            
            if let Ok(mut reg) = PLUGIN_REGISTRY.lock() {
                if reg.contains_key(&function_name) {
                    println!("Netter Plugin Warning: Duplicate registration for function '{}'. Overwriting.", function_name);
                }
                reg.insert(function_name, handler);
            }
        }
    };

    let output = quote! {
        #internal_fn
        #registration_code
    };

    output.into()
}

#[proc_macro]
pub fn generate_dispatch_func(_item: TokenStream) -> TokenStream {
    quote! {
        use std::collections::HashMap;
        use std::sync::OnceLock;
        use std::ffi::{CStr, CString};
        use std::os::raw::c_char;
        use netter_sdk::

        type DispatchableFn = Box<dyn Fn(Vec<RDLTypes>) -> Result<RDLTypes, String> + Send + Sync>;

        static PLUGIN_REGISTRY: std::sync::Mutex<std::collections::HashMap<String, DispatchableFn>> =
            std::sync::Mutex::new(std::collections::HashMap::new());

        #[unsafe(no_mangle)]
        #[unsafe(export_name = "__netter_dispatch")]
        pub unsafe extern "C" fn __netter_dispatch(
            func_name_ptr: *const c_char,
            args_ptr: *const FfiValue,
            args_len: usize,
        ) -> FfiResult {

            unsafe fn run(
                func_name_ptr: *const c_char,
                args_ptr: *const FfiValue,
                args_len: usize,
            ) -> Result<RDLTypes, String> {
                if func_name_ptr.is_null() {
                    return Err("Function name pointer is null".to_string());
                }
                
                let func_name = match CStr::from_ptr(func_name_ptr).to_str() {
                    Ok(s) => s.to_string(),
                    Err(e) => return Err(format!("Invalid UTF-8 in function name: {}", e)),
                };

                if args_ptr.is_null() && args_len > 0 {
                    return Err("Arguments pointer is null but length is greater than zero".to_string());
                }

                let ffi_slice = std::slice::from_raw_parts(args_ptr, args_len);
                let mut rdl_args = Vec::with_capacity(args_len);

                for ffi_val in ffi_slice {
                    let rdl_val = match ffi_val.tag {
                        FfiTag::Number => RDLTypes::Number(ffi_val.data.number),
                        FfiTag::Boolean => RDLTypes::Boolean(ffi_val.data.boolean),
                        FfiTag::String => {
                            let str_slice = std::slice::from_raw_parts(
                                ffi_val.data.string.ptr as *const u8,
                                ffi_val.data.string.len
                            );
                            let s = match std::str::from_utf8(str_slice) {
                                Ok(valid_str) => valid_str.to_string(),
                                Err(e) => return Err(format!("Invalid UTF-8 in string argument: {}", e)),
                            };
                            RDLTypes::String(s)
                        }
                        FfiTag::Vector => return Err("Vector arguments unpacked handling not implemented yet".to_string()),
                    };
                    rdl_args.push(rdl_val);
                }

                let registry = PLUGIN_REGISTRY.get_or_init(HashMap::new);

                match registry.get(&func_name) {
                    Some(handler) => handler(rdl_args),
                    None => Err(format!("Function '{}' not found in plugin registry", func_name)),
                }
            }

            let execution_result = std::panic::catch_unwind(|| {
                run(func_name_ptr, args_ptr, args_len)
            });

            match execution_result {
                Ok(Ok(rdl_result)) => {
                    let result_str = rdl_result.to_string(); 
                    FfiResult {
                        status: FfiStatus::Ok,
                        data_ptr: CString::new(result_str).unwrap().into_raw(),
                    }
                }
                Ok(Err(err_msg)) => {
                    FfiResult {
                        status: FfiStatus::Err,
                        data_ptr: CString::new(err_msg).unwrap().into_raw(),
                    }
                }
                Err(panic_payload) => {
                    let panic_msg = if let Some(s) = panic_payload.downcast_ref::<&str>() { *s }
                    else if let Some(s) = panic_payload.downcast_ref::<String>() { s.as_str() }
                    else { "Unknown panic payload" };
                    
                    let complete_err = format!("Panic during plugin dispatch: {}", panic_msg);
                    FfiResult {
                        status: FfiStatus::Err,
                        data_ptr: CString::new(complete_err).unwrap().into_raw(),
                    }
                }
            }
        }
    }
    .into()
}