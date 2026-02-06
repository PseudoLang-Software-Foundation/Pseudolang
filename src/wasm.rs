#[cfg(all(target_arch = "wasm32", not(feature = "wasi")))]
use crate::core::execute_code_with_capture;

#[cfg(all(target_arch = "wasm32", not(feature = "wasi")))]
use wasm_bindgen::prelude::*;

#[cfg(all(target_arch = "wasm32", not(feature = "wasi")))]
#[wasm_bindgen]
pub fn get_version() -> String {
    env!("CARGO_PKG_VERSION").to_string()
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
