use std::fs;
use std::io::Read;

mod interpreter;
mod lexer;
mod parser;

use lexer::Lexer;

include!(concat!(env!("OUT_DIR"), "/version.rs"));

struct Config {
    input_file: String,
    output_file: String,
    debug: bool,
    show_version: bool,
}

fn parse_args() -> Result<Config, String> {
    let args: Vec<String> = std::env::args().skip(1).collect();

    match args.len() {
        0 => Err(format!(
            "Usage: {} [--debug] [--version] <input_file.psl> <output_file>",
            env!("CARGO_PKG_NAME")
        )),
        1 if args[0] == "--version" || args[0] == "-v" => Ok(Config {
            input_file: String::new(),
            output_file: String::new(),
            debug: false,
            show_version: true,
        }),
        2 => Ok(Config {
            input_file: args[0].clone(),
            output_file: args[1].clone(),
            debug: false,
            show_version: false,
        }),
        3 if args[0] == "--debug" => Ok(Config {
            input_file: args[1].clone(),
            output_file: args[2].clone(),
            debug: true,
            show_version: false,
        }),
        _ => Err(format!(
            "Usage: {} [--debug] [--version] <input_file.psl> <output_file>",
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

    if config.show_version {
        println!("Pseudolang version {}", VERSION);
        return;
    }

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

    if !config.debug {
        println!("Successfully lexed program");
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
                println!("Successfully parsed program");
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
                        println!(
                            "Successfully executed program and wrote output to {}",
                            output_file
                        );
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
