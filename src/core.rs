use crate::{interpreter, lexer::Lexer, parser};
use std::fmt::Write;

pub fn execute_code(
    source_code: &str,
    debug: bool,
    return_output: bool,
    args: &[String],
) -> Result<String, String> {
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

    // `return_output` now actually selects the sink. Callers that want the text
    // back capture it into a `String`; callers that do not (the CLI) stream
    // straight to stdout instead of holding the whole run's output in RAM,
    // and get an empty `String` back.
    let mode = if return_output {
        interpreter::OutputMode::Capture
    } else {
        interpreter::OutputMode::Stdout
    };

    match interpreter::run_with_mode(ast, source_code, args, mode, debug) {
        Ok(output) => Ok(output),
        Err(e) => Err(e.format(source_code)),
    }
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

    let program_output = match interpreter::run_with_source(ast, input, &[]) {
        Ok(output) => output,
        Err(e) => return Err(e.format(input)),
    };

    writeln!(output, "{}", program_output).unwrap();

    Ok(output)
}
