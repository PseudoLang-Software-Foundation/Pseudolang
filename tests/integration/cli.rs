//! The command-line interface: argument handling, error reporting and exit
//! statuses. All of it in `main.rs` and `core.rs`, which the library tests never
//! enter.

use crate::harness::Program;
use std::process::Command;

const FPLI: &str = env!("CARGO_BIN_EXE_fpli");

/// Run `fpli` with raw arguments and no program file, for the flag-level cases.
fn raw(args: &[&str]) -> (Option<i32>, String, String) {
    let output = Command::new(FPLI)
        .args(args)
        .output()
        .expect("could not run fpli");
    (
        output.status.code(),
        String::from_utf8_lossy(&output.stdout).replace("\r\n", "\n"),
        String::from_utf8_lossy(&output.stderr).replace("\r\n", "\n"),
    )
}

#[test]
fn version_is_reported_and_matches_the_crate() {
    let (code, stdout, _) = raw(&["--version"]);
    assert_eq!(code, Some(0));
    assert!(
        stdout.contains(env!("CARGO_PKG_VERSION")),
        "unexpected version output: {:?}",
        stdout
    );
}

#[test]
fn help_lists_the_run_subcommand() {
    let (code, stdout, _) = raw(&["--help"]);
    assert_eq!(code, Some(0));
    assert!(stdout.contains("run"), "unexpected help: {:?}", stdout);
}

#[test]
fn no_arguments_prints_help_and_fails() {
    let (code, stdout, stderr) = raw(&[]);
    assert_ne!(code, Some(0), "expected a failure exit with no arguments");
    let combined = format!("{}{}", stdout, stderr);
    assert!(
        combined.contains("run") || combined.to_lowercase().contains("usage"),
        "expected usage text, got: {:?}",
        combined
    );
}

#[test]
fn a_file_without_the_psl_extension_is_refused() {
    let (code, _, stderr) = raw(&["run", "program.txt"]);
    assert_eq!(code, Some(1));
    assert!(
        stderr.contains(".psl"),
        "expected the extension to be named: {:?}",
        stderr
    );
}

#[test]
fn a_missing_file_is_reported_with_its_name() {
    let (code, _, stderr) = raw(&["run", "definitely-not-here.psl"]);
    assert_eq!(code, Some(1));
    assert!(
        stderr.contains("definitely-not-here.psl"),
        "expected the path in the error: {:?}",
        stderr
    );
}

#[test]
fn a_runtime_error_goes_to_stderr_with_line_column_and_a_caret() {
    let run = Program::new(
        r#"
        x <- 1
        y <- x / 0
        "#,
    )
    .run();
    run.code(1)
        .stderr_contains("Line 3")
        .stderr_contains("Column")
        .stderr_contains("Division by zero")
        // The caret line that points at the offending source.
        .stderr_contains("^");
    // Diagnostics must not be mixed into the program's own output.
    run.stdout_is_empty();
}

#[test]
fn an_error_inside_a_procedure_includes_a_stack_trace() {
    Program::new(
        r#"
        PROCEDURE inner()
        {
            RETURN 1 / 0
        }
        PROCEDURE outer()
        {
            RETURN inner()
        }
        DISPLAY(outer())
        "#,
    )
    .run()
    .code(1)
    .stderr_contains("inner")
    .stderr_contains("outer");
}

#[test]
fn program_arguments_are_forwarded_after_the_file() {
    Program::new(
        r#"
        DISPLAY(ARGCOUNT)
        DISPLAY(ARGS)
        DISPLAY(POSITIONALS)
        DISPLAY(GETARG("n"))
        DISPLAY(HASARG("verbose"))
        "#,
    )
    .arg("--verbose")
    .arg("-n")
    .arg("5")
    .arg("output.txt")
    .run()
    .success()
    .stdout_is("4\n[--verbose, -n, 5, output.txt]\n[output.txt]\n5\ntrue");
}

#[test]
fn a_program_argument_that_looks_like_an_fpli_flag_is_not_eaten() {
    // Everything after the `.psl` path belongs to the program, including `--debug`.
    Program::new(
        r#"
        DISPLAY(ARGCOUNT)
        DISPLAY(HASARG("debug"))
        "#,
    )
    .arg("--debug")
    .run()
    .success()
    .stdout_is("1\ntrue");
}

#[test]
fn no_program_arguments_gives_empty_lists() {
    Program::new(
        r#"
        DISPLAY(ARGCOUNT)
        DISPLAY(ARGS)
        DISPLAY(POSITIONALS)
        "#,
    )
    .run()
    .success()
    .stdout_is("0\n[]\n[]");
}

#[test]
fn the_debug_flag_before_the_subcommand_adds_diagnostics_without_breaking_output() {
    let run = Program::new(r#"DISPLAY("still works")"#)
        .flag("--debug")
        .run();
    run.success().stdout_contains("still works");
    // Debug mode prints the token and AST dumps as well; the program's own output
    // must survive alongside them.
    run.stdout_contains("Debug Mode");
}

#[test]
fn the_debug_flag_after_the_subcommand_is_also_accepted() {
    let output = Command::new(FPLI)
        .args(["run", "--debug", "does-not-exist.psl"])
        .output()
        .expect("could not run fpli");
    // The point is that clap accepts the flag in this position; the file being
    // absent is what then fails.
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("does-not-exist.psl"),
        "the flag position should be accepted, got: {:?}",
        stderr
    );
}

#[test]
fn output_is_streamed_to_stdout_not_captured_and_dropped() {
    // The CLI uses `OutputMode::Stdout`, which the library tests never exercise
    // because they always capture. A regression there would show up as no output
    // at all.
    Program::new(
        r#"
        DISPLAY("one")
        DISPLAYINLINE("two")
        DISPLAYINLINE(" three")
        DISPLAY("")
        DISPLAY("four")
        "#,
    )
    .run()
    .success()
    .stdout_is("one\ntwo three\nfour");
}

#[test]
fn a_large_amount_of_output_survives_the_streaming_sink() {
    Program::new(
        r#"
        i <- 1
        REPEAT UNTIL (i > 2000)
        {
            DISPLAY(i)
            i <- i + 1
        }
        "#,
    )
    .run()
    .success()
    .stdout_contains("\n2000")
    .stdout_contains("1\n");
}

#[test]
fn a_unicode_program_round_trips_through_the_terminal() {
    Program::new(
        r#"
        s <- "héllo wörld — ünïcode"
        DISPLAY(s)
        DISPLAY(LENGTH(s))
        DISPLAY(UPPERCASE("ß"))
        "#,
    )
    .run()
    .success()
    .stdout_contains("héllo wörld — ünïcode")
    .stdout_contains("21");
}
