use std::fs;
use std::io::Read;

mod core;
mod error;
mod interpreter;
mod lexer;
mod parser;
#[cfg(target_arch = "wasm32")]
mod wasm;

use core::*;

include!(concat!(env!("OUT_DIR"), "/version.rs"));

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
        return Err(format!("{}\n{}", HELP_MESSAGE, USAGE_TIP));
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
                _ => return Err(format_unknown_option_error(arg)),
            }
            indices_to_remove.push(index);
        } else if arg.starts_with('-') && arg.len() > 1 {
            let chars = arg[1..].chars();
            for c in chars {
                match c {
                    'd' => debug = true,
                    'h' => show_help = true,
                    'v' => show_version = true,
                    _ => return Err(format_unknown_option_error(&format!("-{}", c))),
                }
            }
            indices_to_remove.push(index);
        }
    }

    for &index in indices_to_remove.iter().rev() {
        args.remove(index);
    }

    if show_help || show_version {
        return Ok(Config {
            command: Command::None,
            input_file: String::new(),
            debug,
            show_version,
            show_help,
        });
    }

    match args.get(0).map(String::as_str) {
        Some("run") => {
            if args.len() < 2 {
                return Err(format_missing_input_error());
            }
            let input_file = args[1].clone();
            if !input_file.ends_with(".psl") {
                return Err(format_invalid_extension_error(&input_file));
            }

            Ok(Config {
                command: Command::Run,
                input_file,
                debug,
                show_version,
                show_help,
            })
        }
        Some(cmd) => Err(format_unknown_command_error(cmd)),
        None => Err(format_no_command_error()),
    }
}

fn run_program(input_file: &str, debug: bool) -> Result<(), String> {
    let mut file = fs::File::open(input_file)
        .map_err(|e| format!("Error opening file {}: {}", input_file, e))?;

    let mut source_code = String::default();
    file.read_to_string(&mut source_code)
        .map_err(|e| format!("Error reading file {}: {}", input_file, e))?;

    match execute_code(&source_code, debug, false) {
        Ok(_) => Ok(()),
        Err(e) => Err(e),
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = match parse_args() {
        Ok(config) => config,
        Err(error) => {
            eprintln!("Error: {}", error);
            std::process::exit(1);
        }
    };

    if config.show_help {
        println!("{}", HELP_MESSAGE);
        return Ok(());
    }

    if config.show_version {
        println!("PseudoLang version {}", get_version());
        return Ok(());
    }

    if config.debug {
        println!("\n=== Debug Mode Enabled ===\n");
    }

    if let Command::Run = config.command {
        if let Err(error) = run_program(&config.input_file, config.debug) {
            eprintln!("Error: {}", error);
            std::process::exit(1);
        }
    }

    Ok(())
}
