use crate::interpreter;
use crate::lexer::Lexer;
use crate::parser;

mod algorithms;
mod arithmetic;
mod basics;
mod control_flow;
mod error_handling;
mod lists;
mod procedures;
mod stdlib;
mod strings;

pub fn run_test(input: &str) -> Result<String, String> {
    let mut lexer = Lexer::new(input);
    let tokens = lexer.tokenize();
    let ast = parser::parse_with_source(tokens, input, false).map_err(|e| e.format())?;
    let output = interpreter::run_with_source(ast, input).map_err(|e| e.format())?;
    Ok(output.trim_end().to_string())
}

pub fn assert_output(input: &str, expected: &str) {
    match run_test(input) {
        Ok(output) => assert_eq!(output, expected),
        Err(e) => panic!("Test failed for input '{}': {}", input, e),
    }
}
