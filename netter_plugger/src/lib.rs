extern crate proc_macro;

use proc_macro::TokenStream;
use quote::{quote, ToTokens};
use syn::{parse_macro_input, FnArg, ItemFn, Pat, PatType, Type, ReturnType, Error};

#[allow(dead_code)]
type DispatchableFn = Box<dyn Fn(Vec<serde_json::Value>) -> Result<String, String> + Send + Sync>;

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
            if !type_str.contains("Result") || !type_str.contains("String") {
                return Error::new_spanned(fn_output, "#[netter_plugin] function must return Result<String, String>")
                    .to_compile_error().into();
            }
        }
        ReturnType::Default => {
            return Error::new_spanned(fn_sig, "#[netter_plugin] function must return Result<String, String>")
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
                        let #arg_name: String = args_json_vec
                            .get(#index)
                            .ok_or_else(|| format!("Missing argument #{}", #index))?
                            .as_str()
                            .ok_or_else(|| format!("Argument #{} must be a string", #index))?
                            .to_string();
                    },
                    Some("str") => quote! {
                        let temp_arg_string_for_ref: String = args_json_vec
                            .get(#index)
                            .ok_or_else(|| format!("Missing argument #{}", #index))?
                            .as_str()
                            .ok_or_else(|| format!("Argument #{} must be a string", #index))?
                            .to_string();
                        let #arg_name: &str = &temp_arg_string_for_ref;
                    },
                    Some("i32") | Some("i64") | Some("isize") => quote! {
                        let value = args_json_vec.get(#index)
                            .ok_or_else(|| format!("Missing argument #{}", #index))?;
                        let #arg_name = match value {
                            serde_json::Value::Number(n) => n.as_i64()
                                .ok_or_else(|| format!("Argument #{} (JSON Number) cannot be represented as i64", #index))?
                                as #type_ident,
                            serde_json::Value::String(s) => s.parse::<#type_ident>()
                                .map_err(|e| format!("Argument #{} (String) cannot be parsed as {}: {}", #index, stringify!(#type_ident), e))?,
                            _ => return Err(format!("Argument #{} must be a number or string", #index)),
                        };
                    },
                    Some("f32") | Some("f64") => quote! {
                        let value = args_json_vec.get(#index)
                            .ok_or_else(|| format!("Missing argument #{}", #index))?;
                        let #arg_name = match value {
                            serde_json::Value::Number(n) => n.as_f64()
                                .ok_or_else(|| format!("Argument #{} (JSON Number) cannot be represented as f64", #index))?
                                as #type_ident,
                            serde_json::Value::String(s) => s.parse::<#type_ident>()
                                .map_err(|e| format!("Argument #{} (JSON String '{}') failed to parse as {}: {}", #index, s, stringify!(#type_ident), e))?,
                            _ => return Err(format!("Argument #{} must be a JSON number or a numeric JSON string for type {}", #index, stringify!(#type_ident))),
                        };
                    },
                    Some("bool") => quote! {
                        let #arg_name = args_json_vec
                            .get(#index)
                            .ok_or_else(|| format!("Missing argument #{}", #index))?
                            .as_bool()
                            .ok_or_else(|| format!("Argument #{} must be a boolean", #index))?;
                    },
                    _ => {
                        let type_str = ty
                            .to_token_stream()
                            .to_string();
                        return Error::new_spanned(ty, format!("Unsupported argument type for JSON dispatch: {}", type_str))
                            .to_compile_error()
                            .into();
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
            use serde_json;

            let function_name = #fn_name_str.to_string();
            let function_name_clone = function_name.clone();

            let handler: DispatchableFn = Box::new(move |args_json_vec: Vec<serde_json::Value>| -> Result<String, String> {
                if args_json_vec.len() != #expected_arg_count {
                    return Err(format!("Function '{}' expects {} arguments, but received {}", function_name_clone, #expected_arg_count, args_json_vec.len()));
                }
                #( #arg_parsers )*
                #internal_fn_name(#(#arg_names_for_call),*)
            });

            match PLUGIN_REGISTRY.lock() {
                 Ok(mut registry) => {
                    if registry.contains_key(&function_name) {
                        println!("Netter Plugin Warning: Duplicate registration for function '{}'. Overwriting.", function_name);
                    }
                    registry.insert(function_name, handler);
                 },
                 Err(e) => { eprintln!("Netter Plugin Critical Error: Failed to lock plugin registry during registration: {}", e); }
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
        use std::sync::Mutex;
        use lazy_static::lazy_static;
        use serde_json;

        type DispatchableFn = Box<dyn Fn(Vec<serde_json::Value>) -> Result<String, String> + Send + Sync>;

        lazy_static! {
            static ref PLUGIN_REGISTRY: Mutex<HashMap<String, DispatchableFn>> =
                Mutex::new(HashMap::new());
        }

        #[unsafe(no_mangle)]
        pub extern "C" fn __netter_dispatch(
            func_name_ptr: *const std::os::raw::c_char,
            args_json_ptr: *const std::os::raw::c_char,
        ) -> *mut std::os::raw::c_char {

            fn run( func_name_ptr: *const std::os::raw::c_char, args_json_ptr: *const std::os::raw::c_char)
                -> Result<String, String>
            {
                let func_name = unsafe {
                    if func_name_ptr.is_null() { return Err("Function name pointer is null".to_string()); }
                    match std::ffi::CStr::from_ptr(func_name_ptr).to_str() {
                        Ok(s) => s.to_string(),
                        Err(e) => return Err(format!("Invalid UTF-8 in function name: {}", e)),
                    }
                };
                let args_json_str = unsafe {
                    if args_json_ptr.is_null() { "[]".to_string() }
                    else {
                         match std::ffi::CStr::from_ptr(args_json_ptr).to_str() {
                            Ok(s) => s.to_string(),
                            Err(e) => return Err(format!("Invalid UTF-8 in arguments JSON: {}", e)),
                        }
                    }
                };

                let args_json_vec: Vec<serde_json::Value> = match serde_json::from_str(&args_json_str) {
                    Ok(v @ serde_json::Value::Array(_)) => { if let serde_json::Value::Array(arr) = v { arr } else { unreachable!() } },
                    Ok(_) => return Err("Arguments JSON must be a JSON array".to_string()),
                    Err(e) => return Err(format!("Failed to parse arguments JSON: {}", e)),
                };

                let registry = match PLUGIN_REGISTRY.lock() {
                     Ok(r) => r,
                     Err(e) => return Err(format!("FATAL: Failed to lock plugin registry for dispatch: {}", e)),
                };

                match registry.get(&func_name) {
                    Some(handler) => handler(args_json_vec),
                    None => Err(format!("Function '{}' not found in plugin registry", func_name)),
                }
            }

            let result = std::panic::catch_unwind(|| { run(func_name_ptr, args_json_ptr) });
            let formatted_string = match result {
                Ok(Ok(ok_val)) => format!("OK:{}", ok_val),
                Ok(Err(err_val)) => format!("ERR:{}", err_val),
                Err(panic_payload) => {
                    let panic_msg = if let Some(s) = panic_payload.downcast_ref::<&str>() { *s }
                    else if let Some(s) = panic_payload.downcast_ref::<String>() { s.as_str() }
                    else { "Unknown panic payload" };
                    format!("ERR:Panic during plugin dispatch: {}", panic_msg)
                }
            };

             match std::ffi::CString::new(formatted_string) {
                Ok(c_string) => c_string.into_raw(),
                Err(_) => {
                    static ERR_MSG_BYTES: &[u8] = b"ERR:FATAL: Failed to create CString for dispatch result\0";
                    ERR_MSG_BYTES.as_ptr() as *mut std::os::raw::c_char
                }
             }
        }
    }
    .into()
}