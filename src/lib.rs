pub mod core;
pub mod interpreter;
pub mod lexer;
pub mod parser;
#[cfg(test)]
mod tests;
#[cfg(target_arch = "wasm32")]
pub mod wasm;
