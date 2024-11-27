use crate::core::execute_code;
use std::io::{self, Read};

#[no_mangle]
pub extern "C" fn _start() {
    let mut input = String::new();
    if let Err(e) = io::stdin().read_to_string(&mut input) {
        eprintln!("Error reading input: {}", e);
        std::process::exit(1);
    }

    if let Err(e) = execute_code(&input, false, false) {
        eprintln!("Error: {}", e);
        std::process::exit(1);
    }
}
