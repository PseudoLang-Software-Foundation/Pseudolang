use std::fs;
use std::io::Read;

mod interpreter;
mod lexer;
mod parser;

use lexer::Lexer;

// Add a struct to hold configuration
struct Config {
    input_file: String,
    output_file: String,
    debug: bool,
}

fn parse_args() -> Result<Config, String> {
    let args: Vec<String> = std::env::args().skip(1).collect();

    match args.len() {
        2 => Ok(Config {
            input_file: args[0].clone(),
            output_file: args[1].clone(),
            debug: false,
        }),
        3 if args[0] == "--debug" => Ok(Config {
            input_file: args[1].clone(),
            output_file: args[2].clone(),
            debug: true,
        }),
        _ => Err(format!(
            "Usage: {} [--debug] <input_file.pc> <output_file>",
            env!("CARGO_PKG_NAME")
        )),
    }
}

fn main() {
    let config = match parse_args() {
        Ok(config) => config,
        Err(error) => {
            eprintln!("{}", error);
            std::process::exit(1);
        }
    };

    // Add debug banner
    if config.debug {
        println!("\n=== Debug Mode Enabled ===\n");
    }

    let mut file = match fs::File::open(&config.input_file) {
        Ok(file) => file,
        Err(error) => {
            eprintln!("Error opening file {}: {}", config.input_file, error);
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
    println!("Successfully lexed program");

    if config.debug {
        println!("\n=== Parser Starting ===");
    }

    match parser::parse(tokens, config.debug) {
        Ok(ast) => {
            if config.debug {
                println!("\n=== Parser Output ===");
                println!("AST: {:#?}", ast); // Changed to pretty print
            }
            println!("Successfully parsed program");

            let output = interpreter::run(ast);

            match output {
                Ok(result) => {
                    if let Err(error) = fs::write(&config.output_file, result) {
                        eprintln!("Error writing to file {}: {}", config.output_file, error);
                        std::process::exit(1);
                    }
                    println!(
                        "Successfully executed program and wrote output to {}",
                        config.output_file
                    );
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
