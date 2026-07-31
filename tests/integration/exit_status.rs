//! `EXIT` and the exit status of the process.

use crate::harness::Program;

#[test]
fn exit_with_no_argument_succeeds() {
    Program::new(
        r#"
        DISPLAY("before")
        EXIT()
        DISPLAY("never runs")
        "#,
    )
    .run()
    .code(0)
    .stdout_is("before");
}

#[test]
fn exit_with_a_code_sets_the_process_status() {
    for code in [0, 1, 3, 42, 255] {
        Program::new(&format!("EXIT({})", code)).run().code(code);
    }
}

#[test]
fn exit_flushes_output_written_before_it() {
    // `process::exit` runs no destructors, so a buffered sink would be dropped
    // along with everything the program had printed. This is the regression test
    // for that: the text must be on stdout even though nothing unwound.
    Program::new(
        r#"
        DISPLAY("line one")
        DISPLAY("line two")
        EXIT(7)
        "#,
    )
    .run()
    .code(7)
    .stdout_is("line one\nline two");
}

#[test]
fn exit_flushes_displayinline_output_with_no_trailing_newline() {
    // The hardest flush case: nothing has ended a line, so the text is still
    // sitting in the buffer with no newline to have triggered a write.
    Program::new(
        r#"
        DISPLAYINLINE("partial")
        EXIT(0)
        "#,
    )
    .run()
    .code(0)
    .stdout_is("partial");
}

#[test]
fn exit_inside_a_procedure_still_exits_the_program() {
    Program::new(
        r#"
        PROCEDURE bail()
        {
            DISPLAY("bailing")
            EXIT(5)
        }
        bail()
        DISPLAY("never runs")
        "#,
    )
    .run()
    .code(5)
    .stdout_is("bailing");
}

#[test]
fn exit_inside_a_loop_and_a_conditional() {
    Program::new(
        r#"
        FOR EACH n IN [1, 2, 3]
        {
            DISPLAY(n)
            IF n = 2
            {
                EXIT(2)
            }
        }
        "#,
    )
    .run()
    .code(2)
    .stdout_is("1\n2");
}

#[test]
fn exit_inside_an_imported_file_exits_the_whole_program() {
    Program::new(
        r#"
        IMPORT "lib.psl"
        DISPLAY("never runs")
        "#,
    )
    .file(
        "lib.psl",
        r#"
        DISPLAY("from the library")
        EXIT(9)
        "#,
    )
    .run()
    .code(9)
    .stdout_is("from the library");
}

#[test]
fn exit_is_not_caught_by_try_catch() {
    // EXIT is not an error, so a TRY around it must not swallow it.
    Program::new(
        r#"
        TRY
        {
            EXIT(4)
        } CATCH (err)
        {
            DISPLAY("should not be reached")
        }
        "#,
    )
    .run()
    .code(4)
    .stdout_is_empty();
}

#[test]
fn an_out_of_range_exit_code_is_an_error_not_a_truncated_status() {
    // 256 would wrap to 0 on all three platforms and silently report success, so
    // it is refused instead.
    Program::new("EXIT(256)")
        .run()
        .code(1)
        .stderr_contains("between 0 and 255");

    Program::new("EXIT(-1)")
        .run()
        .code(1)
        .stderr_contains("between 0 and 255");
}

#[test]
fn a_non_integer_exit_code_is_refused() {
    Program::new(r#"EXIT("three")"#)
        .run()
        .code(1)
        .stderr_contains("integer exit code");
}

#[test]
fn a_program_that_ends_normally_exits_zero_with_nothing_on_stderr() {
    Program::new(r#"DISPLAY("done")"#)
        .run()
        .code(0)
        .stdout_is("done")
        .stderr_is_empty();
}

#[test]
fn a_runtime_error_exits_one() {
    Program::new(
        r#"
        DISPLAY("before")
        x <- 1 / 0
        DISPLAY("after")
        "#,
    )
    .run()
    .code(1)
    // Output written before the failure must still reach the terminal.
    .stdout_contains("before")
    .stdout_excludes("after")
    .stderr_contains("Division by zero");
}

#[test]
fn a_parse_error_exits_one_and_says_where() {
    Program::new("DISPLAY(")
        .run()
        .code(1)
        .stderr_contains("Line 1");
}
