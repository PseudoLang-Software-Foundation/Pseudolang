#[cfg(all(target_arch = "wasm32", not(feature = "wasi")))]
use crate::core::execute_code;

#[cfg(all(target_arch = "wasm32", not(feature = "wasi")))]
#[no_mangle]
pub extern "C" fn run_pseudolang_raw(ptr: *const u8, len: usize, debug: bool) -> u64 {
    let input = unsafe {
        let slice = std::slice::from_raw_parts(ptr, len);
        std::str::from_utf8_unchecked(slice)
    };

    match execute_code(input, debug, true) {
        Ok(output) => {
            let bytes = output.into_bytes();
            let ptr = bytes.as_ptr() as u64;
            let len = bytes.len() as u64;
            std::mem::forget(bytes);
            (ptr << 32) | len
        }
        Err(_) => 0,
    }
}

#[cfg(all(target_arch = "wasm32", not(feature = "wasi")))]
use wasm_bindgen::prelude::*;

#[cfg(all(target_arch = "wasm32", not(feature = "wasi")))]
#[wasm_bindgen]
pub fn run_pseudolang(input: &str, debug: bool) -> Result<String, JsValue> {
    console_error_panic_hook::set_once();
    execute_code(input, debug, true).map_err(|e| JsValue::from_str(&e))
}

#[cfg(all(target_arch = "wasm32", not(feature = "wasi")))]
#[no_mangle]
pub extern "C" fn alloc(size: usize) -> *mut u8 {
    let mut buf = Vec::with_capacity(size);
    let ptr = buf.as_mut_ptr();
    std::mem::forget(buf);
    ptr
}

#[cfg(all(target_arch = "wasm32", not(feature = "wasi")))]
#[no_mangle]
pub extern "C" fn dealloc(ptr: *mut u8, size: usize) {
    unsafe {
        let _ = Vec::from_raw_parts(ptr, 0, size);
    }
}
