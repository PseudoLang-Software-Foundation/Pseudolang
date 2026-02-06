use std::fs;
use std::io::Read;

mod core;
mod error;
mod interpreter;
mod lexer;
mod parser;
#[cfg(target_arch = "wasm32")]
mod wasm;

use clap::{Parser, Subcommand};
use core::*;

const HELP_TEMPLATE: &str = r#"PseudoLang Usage:
    fplc [OPTIONS] COMMAND [ARGS]

COMMANDS:
    run <input_file.psl>    Execute a PseudoLang program

OPTIONS:
    -h, --help       Display this help message
    -V, --version    Display version information
    -d, --debug      Enable debug output during execution

Examples:
    fplc run program.psl
    fplc run --debug source.psl
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

fn run_program(input_file: &str, debug: bool) -> Result<(), String> {
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

    match execute_code(&source_code, debug, false) {
        Ok(_) => Ok(()),
        Err(e) => Err(e),
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();

    if cli.debug {
        println!("\n=== Debug Mode Enabled ===\n");
    }

    match cli.command {
        Commands::Run { ref input_file } => {
            if let Err(error) = run_program(input_file, cli.debug) {
                eprintln!("Error: {}", error);
                std::process::exit(1);
            }
        }
    }

    Ok(())
}
