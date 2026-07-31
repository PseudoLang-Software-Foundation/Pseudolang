//! Integration tests: the real `fpli` binary, run as a child process.
//!
//! The in-process suite under `src/tests/` drives the interpreter as a library.
//! That is fast, and it covers most of the language, but there is a whole class of
//! behaviour it structurally cannot reach. This target exists for exactly that
//! class -- see [`harness`] for what and why.
//!
//! Everything lives in one test binary (this file declares the modules) rather
//! than one binary per file, so the harness is compiled once and the suite links
//! once.

mod harness;

mod cli;
mod exit_status;
mod input;
mod multi_file;
mod process_state;
mod programs;
mod sleep;
mod streaming;
