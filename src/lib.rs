pub mod core;
pub mod error;
pub mod interpreter;
pub mod lexer;
pub mod parser;
pub mod system;
#[cfg(test)]
mod tests;
#[cfg(target_arch = "wasm32")]
pub mod wasm;
