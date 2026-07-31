use std::fs;
use std::io::Read;

mod core;
mod error;
mod interpreter;
mod lexer;
mod parser;
mod system;
#[cfg(target_arch = "wasm32")]
mod wasm;

use clap::{Parser, Subcommand};
use core::*;

const HELP_TEMPLATE: &str = r#"PseudoLang Usage:
    fpli [OPTIONS] COMMAND [ARGS]

COMMANDS:
    run <input_file.psl> [PROGRAM_ARGS...]    Execute a PseudoLang program

OPTIONS:
    -h, --help       Display this help message
    -V, --version    Display version information
    -d, --debug      Enable debug output during execution

Examples:
    fpli run program.psl
    fpli --debug run source.psl
    fpli run --debug source.psl
    fpli run program.psl --verbose -n 5 output.txt
"#;

#[derive(Parser)]
#[command(
    name = "PseudoLang",
    version = concat!("version ", env!("CARGO_PKG_VERSION")),
    help_template = HELP_TEMPLATE,
    disable_help_subcommand = true,
    subcommand_required = true,
    arg_required_else_help = true
)]
struct Cli {
    #[arg(
        short = 'd',
        long,
        global = true,
        help = "Enable debug output during execution"
    )]
    debug: bool,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    #[command(about = "Execute a PseudoLang program")]
    Run {
        #[arg(help = "Path to a .psl file")]
        input_file: String,
    },
}

fn split_args() -> (Vec<String>, Vec<String>) {
    let all: Vec<String> = std::env::args().collect();
    match all.iter().position(|a| a.ends_with(".psl")) {
        Some(pos) => (all[..=pos].to_vec(), all[pos + 1..].to_vec()),
        None => (all, vec![]),
    }
}

fn run_program(input_file: &str, debug: bool, program_args: &[String]) -> Result<(), String> {
    if !input_file.ends_with(".psl") {
        return Err(format!(
            "Input file must have .psl extension, got: {}",
            input_file
        ));
    }

    let mut file = fs::File::open(input_file)
        .map_err(|e| format!("Error opening file {}: {}", input_file, e))?;

    let mut source_code = String::default();
    file.read_to_string(&mut source_code)
        .map_err(|e| format!("Error reading file {}: {}", input_file, e))?;

    match execute_code(
        &source_code,
        debug,
        false,
        program_args,
        Some(std::path::Path::new(input_file)),
    ) {
        Ok(_) => Ok(()),
        Err(e) => Err(e),
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let (clap_args, program_args) = split_args();
    let cli = Cli::parse_from(clap_args);

    if cli.debug {
        println!("\n=== Debug Mode Enabled ===\n");
    }

    match cli.command {
        Commands::Run { ref input_file } => {
            if let Err(error) = run_program(input_file, cli.debug, &program_args) {
                eprintln!("Error: {}", error);
                std::process::exit(1);
            }
        }
    }

    Ok(())
}
