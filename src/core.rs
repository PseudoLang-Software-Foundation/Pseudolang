use crate::{interpreter, lexer::Lexer, parser};
use std::fmt::Write;

include!(concat!(env!("OUT_DIR"), "/version.rs"));

pub fn execute_code(source_code: &str, debug: bool, return_output: bool) -> Result<String, String> {
    let mut lexer = Lexer::new(source_code);
    let tokens = lexer.tokenize();

    if debug {
        println!("\n=== Lexer Output ===");
        println!("Tokens: {:?}", tokens);
        println!("\n=== Parser Starting ===");
    }

    let ast = parser::parse_with_source(tokens, source_code, debug).map_err(|e| e.format())?;

    if debug {
        println!("\n=== Parser Output ===");
        println!("AST: {:#?}", ast);
        println!("\n=== Starting Execution ===");
    }

    let output = match interpreter::run_with_source(ast, source_code) {
        Ok(output) => output,
        Err(e) => {
            let err_str = e.format();
            return Err(err_str);
        }
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

    let ast = parser::parse_with_source(tokens, input, false).map_err(|e| e.format())?;

    if debug {
        writeln!(output, "\n=== Parser Output ===").unwrap();
        writeln!(output, "AST: {:#?}", ast).unwrap();
        writeln!(output, "\n=== Starting Execution ===").unwrap();
    }

    let program_output = match interpreter::run_with_source(ast, input) {
        Ok(output) => output,
        Err(e) => return Err(e.format()),
    };

    writeln!(output, "{}", program_output).unwrap();

    Ok(output)
}

pub const HELP_MESSAGE: &str = r#"PseudoLang Usage:
    fplc [OPTIONS] COMMAND [ARGS]

COMMANDS:
    run <input_file.psl>    Execute a PseudoLang program

OPTIONS:
    -h, --help       Display this help message
    -v, --version    Display version information
    -d, --debug      Enable debug output during execution

Examples:
    fplc run program.psl
    fplc run --debug source.psl
"#;

pub const UNKNOWN_OPTION_ERROR: &str = "Unknown option: {}";
pub const UNKNOWN_COMMAND_ERROR: &str = "Unknown command: {}";
pub const MISSING_INPUT_ERROR: &str = "Missing required argument: input_file";
pub const NO_COMMAND_ERROR: &str = "No command provided";
pub const INVALID_EXTENSION_ERROR: &str = "Input file must have .psl extension, got: {}";
pub const USAGE_TIP: &str = "\n\nTip: Use -h or --help for detailed usage information.";

pub fn format_unknown_option_error(option: &str) -> String {
    format!(
        "{}{}",
        UNKNOWN_OPTION_ERROR.replace("{}", option),
        USAGE_TIP
    )
}

pub fn format_unknown_command_error(cmd: &str) -> String {
    format!("{}{}", UNKNOWN_COMMAND_ERROR.replace("{}", cmd), USAGE_TIP)
}

pub fn format_missing_input_error() -> String {
    format!("{}{}", MISSING_INPUT_ERROR, USAGE_TIP)
}

pub fn format_no_command_error() -> String {
    format!("{}{}", NO_COMMAND_ERROR, USAGE_TIP)
}

pub fn format_invalid_extension_error(file: &str) -> String {
    format!(
        "{}{}",
        INVALID_EXTENSION_ERROR.replace("{}", file),
        USAGE_TIP
    )
}

pub fn get_version() -> String {
    VERSION.to_string()
}
