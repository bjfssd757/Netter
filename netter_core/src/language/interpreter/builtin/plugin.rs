use std::collections::HashMap;
use std::ffi::CString;
use std::os::raw::c_char;
use libloading::{Library, Symbol};
use log::{debug, error, trace};
use serde_json;
use crate::language::error::{Result, Error, ErrorKind};
use crate::runtime_error;
use std::error::Error as StdError;
use crate::language::rdl_types::RDLTypes;

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
            
            let args_json = serde_json::to_string(args).map_err(|e| {
                Error {
                    kind: ErrorKind::Runtime,
                    message: format!("Argument serialization error for {}::{}: {}", plugin_name, function_name, e),
                    line: None,
                    column: None,
                }
            })?;

            trace!("Arguments (JSON) for {}::{}: {}", plugin_name, function_name, args_json);

            let c_name = CString::new(function_name.as_bytes()).map_err(|e| {
                Error {
                    kind: ErrorKind::Runtime,
                    message: format!("Error creating CString for function name {}::{}: {}", plugin_name, function_name, e),
                    line: None,
                    column: None,
                }
            })?;

            let c_args_json = CString::new(args_json).map_err(|e| {
                Error {
                    kind: ErrorKind::Runtime,
                    message: format!("Error creating CString for argument {}::{}: {}", plugin_name, function_name, e),
                    line: None,
                    column: None,
                }
            })?;

            type DispatchFuncSig = unsafe extern "C" fn(
                func_name_ptr: *const c_char,
                args_json_ptr: *const c_char,
            ) -> *mut c_char;

            unsafe {
                let dispatch_func: Symbol<DispatchFuncSig> =
                    match library.get(b"__netter_dispatch\0") {
                        Ok(func) => func,
                        Err(e) => {
                            return runtime_error!(
                                format!("Dispatcher function '__netter_dispatch' not found in plugin '{}': {}. \
                                Make sure it is used netter_plugger::generate_dispatcher!\n\nError OS: {:?}\n\n", plugin_name, e, e.source())
                            );
                        }
                    };

                let result_ptr = dispatch_func(c_name.as_ptr(), c_args_json.as_ptr());

                if result_ptr.is_null() {
                    return runtime_error!(
                        format!("Plugin dispatcher function {} returned null for call {}", plugin_name, function_name)
                    );
                }

                let result_string = match CString::from_raw(result_ptr).into_string() {
                    Ok(s) => s,
                    Err(e) => {
                        return runtime_error!(
                            format!("Conversion error result of dispatcher for plugin {} (call {}): {}",
                                plugin_name, function_name, e
                            )
                        );
                    }
                };

                trace!("Dispatcher result {} for {}: {}", plugin_name, function_name, result_string);

                if let Some(ok_result) = result_string.strip_prefix("OK:") {
                    Ok(ok_result.into())
                } else if let Some(err_msg) = result_string.strip_prefix("ERR:") {
                    runtime_error!(err_msg.to_string())
                } else {
                    runtime_error!(
                        format!("Incorrect answer format from dispatcher plugin {} (call {}): not prefix 'OK:' or 'ERR:'",
                            plugin_name, function_name
                        )
                    )
                }
            }
        } else {
            runtime_error!(format!("Plugin '{}' not found", plugin_name))
        }
    }
}