#[cfg(all(target_arch = "wasm32", feature = "wasi"))]
use crate::core::execute_code;
#[cfg(all(target_arch = "wasm32", not(feature = "wasi")))]
use crate::core::execute_code_with_capture;

#[cfg(all(target_arch = "wasm32", not(feature = "wasi")))]
use wasm_bindgen::prelude::*;

#[cfg(all(target_arch = "wasm32", feature = "wasi"))]
use std::io::{self, Read, Write};

include!(concat!(env!("OUT_DIR"), "/version.rs"));

#[cfg(all(target_arch = "wasm32", not(feature = "wasi")))]
#[wasm_bindgen]
pub fn get_version() -> String {
    VERSION.to_string()
}

#[cfg(all(target_arch = "wasm32", not(feature = "wasi")))]
#[wasm_bindgen]
pub fn run_pseudolang(input: &str, debug: bool) -> Result<String, JsValue> {
    console_error_panic_hook::set_once();

    match execute_code_with_capture(input, debug) {
        Ok(output) => Ok(output),
        Err(error_msg) => Err(JsValue::from_str(&format!("Error: {}", error_msg))),
    }
}

#[cfg(all(target_arch = "wasm32", feature = "wasi"))]
pub fn main() {
    println!("PseudoLang version {}", VERSION);
    io::stdout().flush().unwrap();

    let mut input = String::default();
    if let Err(e) = std::io::stdin().read_to_string(&mut input) {
        eprintln!("Error reading input: {}", e);
        std::process::exit(1);
    }

    if let Err(e) = execute_code(&input, false, false) {
        eprintln!("Error: {}", e);
        std::process::exit(1);
    }
}
