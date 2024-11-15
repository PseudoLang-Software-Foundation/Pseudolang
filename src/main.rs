use std::fs;
use std::io::Read;

mod interpreter;
mod lexer;
mod parser;

use lexer::Lexer;

fn main() {
    let args = std::env::args().skip(1).collect::<Vec<String>>();

    if args.len() != 2 {
        eprintln!(
            "Usage: {} <input_file.pc> <output_file>",
            env!("CARGO_PKG_NAME")
        );
        std::process::exit(0);
    }

    let input_file = &args[0];
    let mut file = match fs::File::open(input_file) {
        Ok(file) => file,
        Err(error) => {
            eprintln!("Error opening file {}: {}", input_file, error);
            std::process::exit(1);
        }
    };

    let mut source_code = String::new();
    if let Err(error) = file.read_to_string(&mut source_code) {
        eprintln!("Error reading file {}: {}", input_file, error);
        std::process::exit(1);
    }

    let mut lexer = Lexer::new(&source_code);
    let tokens = lexer.tokenize();
    println!("{:?}", tokens);
    println!("Successfully lexed program");

    match parser::parse(tokens) {
        Ok(ast) => {
            let output = interpreter::run(ast);

            if let Err(err) = output {
                eprintln!("Error during execution: {}", err);
                std::process::exit(1);
            }

            let output_file = &args[1];
            if let Err(error) = fs::write(output_file, output.unwrap()) {
                eprintln!("Error writing to file {}: {}", output_file, error);
                std::process::exit(1);
            }

            println!(
                "Successfully executed program and wrote output to {}",
                output_file
            );
        }
        Err(err) => {
            eprintln!("Error parsing code: {}", err);
            std::process::exit(1);
        }
    }
}
