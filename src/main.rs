use std::fs;
use std::io::{Read, Write};
use std::path::Path;

mod interpreter;
mod lexer;
mod parser;

use lexer::Lexer;

include!(concat!(env!("OUT_DIR"), "/version.rs"));

const HELP_MESSAGE: &str = r#"PseudoLang Interpreter Usage:
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

#[derive(Debug)]
struct Config {
    command: Command,
    input_file: String,
    debug: bool,
    show_version: bool,
    show_help: bool,
}

#[derive(Debug)]
enum Command {
    Run,
    None,
}

fn parse_args() -> Result<Config, String> {
    let mut args: Vec<String> = std::env::args().skip(1).collect();

    if args.is_empty() {
        return Err(format!(
            "{}\n\nTip: Use -h or --help for detailed usage information.",
            HELP_MESSAGE
        ));
    }

    let mut debug = false;
    let mut show_help = false;
    let mut show_version = false;

    let mut indices_to_remove = Vec::new();

    for (index, arg) in args.iter().enumerate() {
        if arg.starts_with("--") {
            match arg.as_str() {
                "--debug" => debug = true,
                "--help" => show_help = true,
                "--version" => show_version = true,
                _ => {
                    return Err(format!(
                        "Unknown option: {}\n\nTip: Use -h or --help for detailed usage information.",
                        arg
                    ));
                }
            }
            indices_to_remove.push(index);
        } else if arg.starts_with('-') && arg.len() > 1 {
            let chars = arg[1..].chars();
            for c in chars {
                match c {
                    'd' => debug = true,
                    'h' => show_help = true,
                    'v' => show_version = true,
                    _ => {
                        return Err(format!(
                            "Unknown option: -{}\n\nTip: Use -h or --help for detailed usage information.",
                            c
                        ));
                    }
                }
            }
            indices_to_remove.push(index);
        }
    }

    for &index in indices_to_remove.iter().rev() {
        args.remove(index);
    }

    match args.get(0).map(String::as_str) {
        Some("run") => {
            if args.len() < 2 {
                return Err("Missing required argument: input_file\n\nTip: Use -h or --help for detailed usage information.".to_string());
            }
            let input_file = args[1].clone();
            if !input_file.ends_with(".psl") {
                return Err(format!(
                    "Input file must have .psl extension, got: {}\n\nTip: Use -h or --help for detailed usage information.",
                    input_file
                ));
            }

            Ok(Config {
                command: Command::Run,
                input_file,
                debug,
                show_version,
                show_help,
            })
        }
        Some(cmd) => Err(format!(
            "Unknown command: {}\n\nTip: Use -h or --help for detailed usage information.",
            cmd
        )),
        None => Err(
            "No command provided\n\nTip: Use -h or --help for detailed usage information."
                .to_string(),
        ),
    }
}

fn process_file(
    input_file: &str,
    debug: bool,
) -> Result<(Vec<lexer::Token>, parser::AstNode), String> {
    let mut file = match fs::File::open(input_file) {
        Ok(file) => file,
        Err(error) => {
            return Err(format!(
                "Error opening file {}: {}\nPlease ensure the file exists and you have read permissions.",
                input_file, error
            ));
        }
    };

    let mut source_code = String::new();
    if let Err(error) = file.read_to_string(&mut source_code) {
        return Err(format!("Error reading file {}: {}", input_file, error));
    }

    let mut lexer = Lexer::new(&source_code);
    let tokens = lexer.tokenize();

    if debug {
        println!("\n=== Lexer Output ===");
        println!("Tokens: {:?}", tokens);
        println!("\n=== Parser Starting ===");
    }

    let ast = parser::parse(tokens.clone(), debug)?;

    if debug {
        println!("\n=== Parser Output ===");
        println!("AST: {:#?}", ast);
    }

    Ok((tokens, ast))
}

fn run_program(input_file: &str, debug: bool) -> Result<(), String> {
    let (_, ast) = process_file(input_file, debug)?;

    if debug {
        println!("\n=== Starting Execution ===");
        println!("Executing program...\n");
    }

    let output = interpreter::run(ast)?;
    print!("{}", output);
    Ok(())
}

fn main() {
    let config = match parse_args() {
        Ok(config) => config,
        Err(error) => {
            eprintln!("Error: {}", error);
            std::process::exit(1);
        }
    };

    if config.show_help {
        println!("{}", HELP_MESSAGE);
        return;
    }

    if config.show_version {
        println!("PseudoLang Compiler version {}", VERSION);
        return;
    }

    if config.debug {
        println!("\n=== Debug Mode Enabled ===\n");
    }

    let result = match config.command {
        Command::Run => run_program(&config.input_file, config.debug),
        Command::None => Ok(()),
    };

    if let Err(error) = result {
        eprintln!("Error: {}", error);
        std::process::exit(1);
    }
}
