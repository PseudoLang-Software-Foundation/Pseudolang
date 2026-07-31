use crate::interpreter;
use crate::lexer::Lexer;
use crate::parser;
use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};

mod algorithms;
mod aliasing;
mod arithmetic;
mod basics;
mod cli_args;
mod control_flow;
mod dictionaries;
mod docs;
mod error_handling;
mod file_io;
mod indexing;
mod lists;
mod meta;
mod modules;
mod parsing;
mod paths;
mod procedures;
mod recursion_limits;
mod semantics;
mod stdlib;
mod strings;
mod system;
mod unicode;

/// A scratch directory that removes itself when the test ends.
///
/// Tests run concurrently in one process, so every one of them needs its own
/// directory: a shared name would let two tests write and delete the same file.
/// The name is built from the process id and a counter rather than from a random
/// source, which keeps it unique both across concurrent runs of the suite and
/// between tests within a run. `Drop` does the cleanup so a failing assertion
/// (which unwinds) still leaves nothing behind.
pub struct Scratch {
    dir: PathBuf,
}

impl Scratch {
    pub fn new() -> Self {
        static COUNTER: AtomicUsize = AtomicUsize::new(0);
        let n = COUNTER.fetch_add(1, Ordering::Relaxed);
        let dir = std::env::temp_dir().join(format!("psl-test-{}-{}", std::process::id(), n));
        std::fs::create_dir_all(&dir).expect("create scratch dir");
        Scratch { dir }
    }

    /// A path inside the scratch directory, escaped for use in PSL source.
    ///
    /// PSL string literals process `\` escapes, so a Windows path pasted in raw
    /// would turn `\t` in a directory name into a tab.
    pub fn psl_path(&self, name: &str) -> String {
        self.dir.join(name).to_string_lossy().replace('\\', "\\\\")
    }

    pub fn path(&self, name: &str) -> PathBuf {
        self.dir.join(name)
    }

    /// Write a file inside the scratch directory, creating parent directories.
    pub fn write(&self, name: &str, contents: &str) -> PathBuf {
        let path = self.path(name);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).expect("create parent dir");
        }
        std::fs::write(&path, contents).expect("write scratch file");
        path
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.dir);
    }
}

pub fn run_test(input: &str) -> Result<String, String> {
    let mut lexer = Lexer::new(input);
    let tokens = lexer.tokenize();
    let ast = parser::parse_with_source(tokens, input, false).map_err(|e| e.format(input))?;
    let output = interpreter::run_with_source(ast, input, &[]).map_err(|e| e.format(input))?;
    Ok(output.trim_end().to_string())
}

pub fn assert_output(input: &str, expected: &str) {
    match run_test(input) {
        Ok(output) => assert_eq!(output, expected),
        Err(e) => panic!("Test failed for input '{}': {}", input, e),
    }
}

pub fn get_error(input: &str) -> String {
    match run_test(input) {
        Ok(output) => panic!("Expected error but got output: {}", output),
        Err(e) => e,
    }
}

/// Run a program as though it had been loaded from `script_path`.
///
/// This is what gives IMPORT a directory to resolve against and what makes
/// SCRIPTPATH and ISMAIN meaningful, so the module tests need it; the file itself
/// does not have to contain the source being run.
pub fn run_test_at(input: &str, script_path: &std::path::Path) -> Result<String, String> {
    let mut lexer = Lexer::new(input);
    let tokens = lexer.tokenize();
    let ast = parser::parse_with_source(tokens, input, false).map_err(|e| e.format(input))?;
    let output = interpreter::run_with_source_at(ast, input, &[], Some(script_path))
        .map_err(|e| e.format(input))?;
    Ok(output.trim_end().to_string())
}

pub fn assert_output_at(input: &str, script_path: &std::path::Path, expected: &str) {
    match run_test_at(input, script_path) {
        Ok(output) => assert_eq!(output, expected),
        Err(e) => panic!("Test failed for input '{}': {}", input, e),
    }
}

pub fn get_error_at(input: &str, script_path: &std::path::Path) -> String {
    match run_test_at(input, script_path) {
        Ok(output) => panic!("Expected error but got output: {}", output),
        Err(e) => e,
    }
}

pub fn run_test_with_args(input: &str, args: &[&str]) -> Result<String, String> {
    let args: Vec<String> = args.iter().map(|s| s.to_string()).collect();
    let mut lexer = Lexer::new(input);
    let tokens = lexer.tokenize();
    let ast = parser::parse_with_source(tokens, input, false).map_err(|e| e.format(input))?;
    let output = interpreter::run_with_source(ast, input, &args).map_err(|e| e.format(input))?;
    Ok(output.trim_end().to_string())
}

pub fn assert_output_with_args(input: &str, args: &[&str], expected: &str) {
    match run_test_with_args(input, args) {
        Ok(output) => assert_eq!(output, expected),
        Err(e) => panic!("Test failed: {}", e),
    }
}
