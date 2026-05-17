use std::collections::HashMap;
use std::ffi::CString;
use std::os::raw::c_char;
use libloading::{Library, Symbol};
use log::{debug, error, trace};
use crate::language::error::{Result, Error, ErrorKind};
use crate::runtime_error;
use netter_sdk::{RDLTypes, FFIResult, FFIValue, FFIStatus};

#[derive(Debug)]
pub struct PluginManager {
    loaded_plugins: HashMap<String, Library>,
}

impl PluginManager {
    pub fn new() -> Self {
        PluginManager {
            loaded_plugins: HashMap::new(),
        }
    }

    pub fn load_plugin(&mut self, path: &str, alias: &str) -> Result<()> {
        debug!("Plugin loading: '{}' from '{}'", alias, path);

        unsafe {
            match Library::new(path) {
                Ok(lib) => {
                    if self.loaded_plugins.contains_key(alias) {
                        debug!("Plugin redefinition with alias: {}", alias);
                    }
                    self.loaded_plugins.insert(alias.to_string(), lib);
                    debug!("Plugin '{}' loaded successfully.", alias);
                    Ok(())
                }
                Err(e) => {
                    let err_msg = format!(
                        "Critical error: Failed while loading plugin '{}' from {}: {}",
                        alias, path, e
                    );
                    error!("{}", err_msg);
                    runtime_error!(err_msg)
                }
            }
        }
    }

    pub fn has_plugin(&self, name: &str) -> bool {
        self.loaded_plugins.contains_key(name)
    }

    pub fn call_plugin_function(&self, plugin_name: &str, function_name: &str, args: &[RDLTypes]) -> Result<RDLTypes> {
        if let Some(library) = self.loaded_plugins.get(plugin_name) {
            trace!("Dispatching plugin call: {}::{}", plugin_name, function_name);

            let ffi_args: Vec<FFIValue> = args.iter().map(|arg| arg.to_ffi()).collect();

            let c_name = CString::new(function_name.as_bytes()).map_err(|e| {
                Error {
                    kind: ErrorKind::Runtime,
                    message: format!("Error creating CString for function name: {e}"),
                    line: None, column: None,
                }
            })?;

            type DispatchFuncSig = unsafe extern "C" fn(
                func_name_ptr: *const c_char,
                args_ptr: *const FFIValue,
                args_len: usize,
            ) -> FFIResult;

            unsafe {
                let dispatch_func: Symbol<DispatchFuncSig> = match library.get(b"__netter_dispatch\0") {
                    Ok(func) => func,
                    Err(e) => return runtime_error!(format!("__netter_dispatch not found: {e}")),
                };

                let result = dispatch_func(c_name.as_ptr(), ffi_args.as_ptr(), ffi_args.len());

                if result.data_ptr.is_null() {
                    return runtime_error!("Plugin returned null data pointer!".to_string());
                }

                let raw_string = CString::from_raw(result.data_ptr).into_string().map_err(|e| {
                    Error {
                        kind: ErrorKind::Runtime,
                        message: format!("Conversion error: {e}"),
                        line: None, column: None,
                    }
                })?;

                match result.status {
                    FFIStatus::Ok => {
                        Ok(RDLTypes::String(raw_string))
                    },
                    FFIStatus::Err => {
                        runtime_error!(format!("Plugin Error: {raw_string}"))
                    }
                }
            }
        } else {
            runtime_error!(format!("Plugin '{}' not found", plugin_name))
        }
    }
}