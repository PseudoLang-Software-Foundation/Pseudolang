use crate::{interpreter, lexer::Lexer, parser};

pub fn execute_code(input: &str, debug: bool, return_output: bool) -> Result<String, String> {
    let mut lexer = Lexer::new(input);
    let tokens = lexer.tokenize();

    if debug {
        println!("\n=== Lexer Output ===");
        println!("Tokens: {:?}", tokens);
        println!("\n=== Parser Starting ===");
    }

    let ast = parser::parse(tokens, debug)?;

    if debug {
        println!("\n=== Parser Output ===");
        println!("AST: {:#?}", ast);
        println!("\n=== Starting Execution ===");
    }

    let output = interpreter::run(ast)?;
    if !return_output && !output.is_empty() {
        print!("{}", output);
    }
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
