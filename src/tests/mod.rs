use crate::interpreter;
use crate::lexer::Lexer;
use crate::parser;

mod algorithms;
mod aliasing;
mod arithmetic;
mod basics;
mod cli_args;
mod control_flow;
mod dictionaries;
mod error_handling;
mod indexing;
mod lists;
mod parsing;
mod procedures;
mod recursion_limits;
mod stdlib;
mod strings;
mod unicode;

pub fn run_test(input: &str) -> Result<String, String> {
    let mut lexer = Lexer::new(input);
    let tokens = lexer.tokenize();
    let ast = parser::parse_with_source(tokens, input, false).map_err(|e| e.format(input))?;
    let output = interpreter::run_with_source(ast, input, &[]).map_err(|e| e.format(input))?;
    Ok(output.trim_end().to_string())
}

pub fn assert_output(input: &str, expected: &str) {
    match run_test(input) {
        Ok(output) => assert_eq!(output, expected),
        Err(e) => panic!("Test failed for input '{}': {}", input, e),
    }
}

pub fn get_error(input: &str) -> String {
    match run_test(input) {
        Ok(output) => panic!("Expected error but got output: {}", output),
        Err(e) => e,
    }
}

pub fn run_test_with_args(input: &str, args: &[&str]) -> Result<String, String> {
    let args: Vec<String> = args.iter().map(|s| s.to_string()).collect();
    let mut lexer = Lexer::new(input);
    let tokens = lexer.tokenize();
    let ast = parser::parse_with_source(tokens, input, false).map_err(|e| e.format(input))?;
    let output = interpreter::run_with_source(ast, input, &args).map_err(|e| e.format(input))?;
    Ok(output.trim_end().to_string())
}

pub fn assert_output_with_args(input: &str, args: &[&str], expected: &str) {
    match run_test_with_args(input, args) {
        Ok(output) => assert_eq!(output, expected),
        Err(e) => panic!("Test failed: {}", e),
    }
}
