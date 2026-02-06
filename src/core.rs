use crate::{interpreter, lexer::Lexer, parser};
use std::fmt::Write;

pub fn execute_code(source_code: &str, debug: bool, return_output: bool) -> Result<String, String> {
    let mut lexer = Lexer::new(source_code);
    let tokens = lexer.tokenize();

    if debug {
        println!("\n=== Lexer Output ===");
        println!("Tokens: {:?}", tokens);
        println!("\n=== Parser Starting ===");
    }

    let ast =
        parser::parse_with_source(tokens, source_code, debug).map_err(|e| e.format(source_code))?;

    if debug {
        println!("\n=== Parser Output ===");
        println!("AST: {:#?}", ast);
        println!("\n=== Starting Execution ===");
    }

    let output = match interpreter::run_with_source(ast, source_code) {
        Ok(output) => output,
        Err(e) => return Err(e.format(source_code)),
    };

    if !return_output {
        // placeholder for now
    }
    Ok(output)
}

#[allow(dead_code)]
pub fn execute_code_with_capture(input: &str, debug: bool) -> Result<String, String> {
    let mut output = String::default();
    let mut lexer = Lexer::new(input);
    let tokens = lexer.tokenize();

    if debug {
        writeln!(output, "\n=== Lexer Output ===").unwrap();
        writeln!(output, "Tokens: {:?}", tokens).unwrap();
        writeln!(output, "\n=== Parser Starting ===").unwrap();
    }

    let ast = parser::parse_with_source(tokens, input, false).map_err(|e| e.format(input))?;

    if debug {
        writeln!(output, "\n=== Parser Output ===").unwrap();
        writeln!(output, "AST: {:#?}", ast).unwrap();
        writeln!(output, "\n=== Starting Execution ===").unwrap();
    }

    let program_output = match interpreter::run_with_source(ast, input) {
        Ok(output) => output,
        Err(e) => return Err(e.format(input)),
    };

    writeln!(output, "{}", program_output).unwrap();

    Ok(output)
}
