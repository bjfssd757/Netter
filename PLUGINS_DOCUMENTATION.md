# Plugins

You can create your own plugins for RDL in the Rust language.

## Crate

To simplify plugin development, a special crate named `netter_plugger` has been created ([crates.io](https://crates.io/crates/netter_plugger)).
Installing the crate:

`cargo add netter_plugger` - this command adds the crate for creating plugins to your code dependencies.

### Code Structure

Let's start with a simple example:

```rust
use netter_plugger::{netter_plugin, generate_dispatch_func};

generate_dispatch_func!();

#[netter_plugin]
fn something(
    path: String,
) -> Result<String, String> {
    let result = "heey".to_string();
    if path == "error" {
        Err("err from plugin".to_string())
    } else {
        Ok(format!("{result}: {path}"))
    }
}

#[netter_plugin]
fn add_numbers(a: i32, b: i32) -> Result<String, String> {
    Ok((a + b).to_string())
}

#[netter_plugin]
fn check_flag(flag: bool) -> Result<String, String> {
    if flag {
        Ok("Flag is set".to_string())
    } else {
        Ok("Flag is not set".to_string())
    }
}
```

* `use netter_plugger::{netter_plugin, generate_dispatch_func};` - imports the attribute and macro from the crate for simplification;
* `generate_dispatch_func!();` - this macro initializes the entry point for the plugin. It is crucial to place this macro at the very top of your code (right after all `use` statements);
* `#[netter_plugin]` - an attribute that marks functions for integration into RDL.

> [!WARNING]
> Functions integrated into RDL have important restrictions:
> **Input Data Types**
> The function can only accept the following input types: *String*, *&str*, *i64*, *i32*, *f64*, *f32*, *bool*.
> **Output Types**
> The function must return the type *Result<String, String>*.
>
> These restrictions are due to the string-based typing of RDL and interpreter limitations. Over time, the list of supported types will be expanded.

* After the attribute, the function and all its logic are declared as usual.

### Under the Hood

All your function code is converted to C-compatible types, which involves `unsafe extern "C"` (you can read more about it [here](https://doc.rust-lang.org/book/ch20-01-unsafe-rust.html)).
Keep this in mind, although problems are usually rare.

## Function Calls from RDL

To call your plugin functions from RDL, you need to generate a dynamic library from your plugin code:

> [!NOTE]
> Your Rust project must be initialized with the `--lib` flag:
> `cargo init --lib`

### Configuring Cargo.toml

Add the following to your `Cargo.toml`:

```toml
[dependencies]
syn = { version = "2.0", features = ["full"] }
quote = "1.0"
lazy_static = "1.5.0"
serde = "1.0.219"
serde_json = "1.0.140"
ctor = "0.4.2"
netter_plugger = "0.1.0"
...

[lib]
crate-type = ["cdylib"]
```

All dependencies listed in the code above must be added to your Cargo.toml.
Choose the latest plugin version to avoid compatibility issues and to access all the crate's functionality!

### Building

Run the following command in the terminal:

```powershell
cargo build --release
```

This will generate a dynamic library (.dll or .so).

### Integration into RDL

Before declaring routes (the keyword `route`), you need to import the previously built dynamic library:

```rdl
import "path/to/file.dll" as plugin_alias; // "path/to/file.dll" - relative or absolute path to your dynamic library
// plugin_alias - alias for future use
```

After this, you can use your functions in the code:

```rdl
route "/" GET {
    val a = 1;
    val b = 2;
    val plugin = plugin_alias::add_numbers(a, b)?;
    Response.body(plugin);
    Response.send();
} onError(e) {
    Response.status(500);
    Response.body(e);
    Response.send();
};
```

> [!IMPORTANT]
> You may have noticed the `?` after calling the plugin function. It is required to catch an error if the function encounters one, so you need to handle this error just like any other errors.
> However, if you are confident that no error will occur, you can ignore it using `!!`, but in case of an error, the code will terminate with a panic (emergency exit).
