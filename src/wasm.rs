#[cfg(all(target_arch = "wasm32", not(feature = "wasi")))]
use crate::core::execute_code;
#[cfg(all(target_arch = "wasm32", not(feature = "wasi")))]
use wasm_bindgen::prelude::*;

#[cfg(all(target_arch = "wasm32", not(feature = "wasi")))]
#[wasm_bindgen]
pub fn run_pseudolang(input: &str) -> Result<String, JsValue> {
    console_error_panic_hook::set_once();
    execute_code(input, false, true).map_err(|e| JsValue::from_str(&e))
}

#[cfg(all(target_arch = "wasm32", not(feature = "wasi")))]
#[wasm_bindgen]
pub fn initialize_panic_hook() {
    console_error_panic_hook::set_once();
}
