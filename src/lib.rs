pub mod core;
pub mod interpreter;
pub mod lexer;
pub mod parser;
#[cfg(test)]
mod tests;
#[cfg(all(target_arch = "wasm32", feature = "wasi"))]
pub mod wasi;
#[cfg(all(target_arch = "wasm32", not(feature = "wasi")))]
pub mod wasm;
