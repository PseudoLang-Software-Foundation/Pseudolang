use std::fs;
use std::io::Read;

mod interpreter;
mod lexer;
mod parser;

use lexer::Lexer;

include!(concat!(env!("OUT_DIR"), "/version.rs"));

const HELP_MESSAGE: &str = r#"PseudoLang Compiler Usage:
    fplc [OPTIONS] <input_file.psl> <output_file>

OPTIONS:
    -h, --help       Display this help message
    -v, --version    Display version information
    --debug         Enable debug output during compilation

ARGUMENTS:
    <input_file.psl>  Source file to compile (must have .psl extension)
    <output_file>     Output file name (will add .exe on Windows)

EXAMPLES:
    fplc source.psl output
    fplc --debug program.psl program.exe
    fplc -v
    fplc -h"#;

#[derive(Debug)]
struct Config {
    input_file: String,
    output_file: String,
    debug: bool,
    show_version: bool,
    show_help: bool,
}

fn parse_args() -> Result<Config, String> {
    let args: Vec<String> = std::env::args().skip(1).collect();

    if args.is_empty() {
        return Err(format!(
            "{}\n\nTip: Use -h or --help for detailed usage information.",
            HELP_MESSAGE
        ));
    }

    let debug = args.iter().any(|arg| arg == "--debug");
    let args: Vec<String> = args.into_iter().filter(|arg| arg != "--debug").collect();

    match args.get(0).map(String::as_str) {
        Some("-h") | Some("--help") => Ok(Config {
            input_file: String::new(),
            output_file: String::new(),
            debug: false,
            show_version: false,
            show_help: true,
        }),
        Some("-v") | Some("--version") => Ok(Config {
            input_file: String::new(),
            output_file: String::new(),
            debug: false,
            show_version: true,
            show_help: false,
        }),
        Some(input_file) => {
            if args.len() < 2 {
                return Err("Missing required argument: output_file\n\nTip: Use -h or --help for detailed usage information.".to_string());
            }

            if !input_file.ends_with(".psl") {
                return Err(format!(
                    "Input file must have .psl extension, got: {}\n\nTip: Use -h or --help for detailed usage information.",
                    input_file
                ));
            }

            Ok(Config {
                input_file: input_file.to_string(),
                output_file: args[1].clone(),
                debug,
                show_version: false,
                show_help: false,
            })
        }
        None => Err(
            "No arguments provided\n\nTip: Use -h or --help for detailed usage information."
                .to_string(),
        ),
    }
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
    } else {
        println!("PseudoLang Compiler version {}", VERSION);
        println!("Processing {} -> {}", config.input_file, config.output_file);
    }

    let mut file = match fs::File::open(&config.input_file) {
        Ok(file) => file,
        Err(error) => {
            eprintln!(
                "Error opening file {}: {}\nPlease ensure the file exists and you have read permissions.",
                config.input_file, error
            );
            std::process::exit(1);
        }
    };

    let mut source_code = String::new();
    if let Err(error) = file.read_to_string(&mut source_code) {
        eprintln!("Error reading file {}: {}", config.input_file, error);
        std::process::exit(1);
    }

    let mut lexer = Lexer::new(&source_code);
    let tokens = lexer.tokenize();
    if config.debug {
        println!("\n=== Lexer Output ===");
        println!("Tokens: {:?}", tokens);
    }

    if !config.debug {
        println!("✓ Successfully lexed program");
    }

    if config.debug {
        println!("\n=== Parser Starting ===");
    }

    match parser::parse(tokens, config.debug) {
        Ok(ast) => {
            if config.debug {
                println!("\n=== Parser Output ===");
                println!("AST: {:#?}", ast);
                println!("\n=== Starting Execution ===");
                println!("Executing program...");
            }

            if !config.debug {
                println!("✓ Successfully parsed program");
            }

            match interpreter::run(ast) {
                Ok(result) => {
                    if config.debug {
                        println!("Program output:");
                        println!("{}", result);
                    }

                    #[cfg(target_os = "windows")]
                    let output_file = if !config.output_file.ends_with(".exe") {
                        format!("{}.exe", config.output_file)
                    } else {
                        config.output_file
                    };

                    #[cfg(not(target_os = "windows"))]
                    let output_file = config.output_file;

                    #[cfg(target_os = "windows")]
                    let content = format!(
                        "
                        {}",
                        result
                            .lines()
                            .map(|line| format!("echo {}", line.replace("\"", "\"\"")))
                            .collect::<Vec<_>>()
                            .join("\r\n")
                    );

                    #[cfg(not(target_os = "windows"))]
                    let content = format!(
                        "#!/bin/bash\n\
                        {}",
                        result
                            .lines()
                            .map(|line| format!("echo \"{}\"", line.replace("\"", "\\\"")))
                            .collect::<Vec<_>>()
                            .join("\n")
                    );

                    if let Err(error) = fs::write(&output_file, content) {
                        eprintln!("Error writing to file {}: {}", output_file, error);
                        std::process::exit(1);
                    }

                    #[cfg(not(target_os = "windows"))]
                    {
                        use std::os::unix::fs::PermissionsExt;
                        if let Err(error) =
                            fs::set_permissions(&output_file, fs::Permissions::from_mode(0o755))
                        {
                            eprintln!("Error setting permissions: {}", error);
                            std::process::exit(1);
                        }
                    }

                    if !config.debug {
                        println!("✓ Successfully executed program");
                        println!("✓ Output written to: {}", output_file);
                    }
                }
                Err(err) => {
                    eprintln!("Error during execution: {}", err);
                    std::process::exit(1);
                }
            }
        }
        Err(err) => {
            eprintln!("Error parsing code: {}", err);
            std::process::exit(1);
        }
    }
}
