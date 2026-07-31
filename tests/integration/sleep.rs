//! `SLEEP`, and the flushing around anything slow. Assertions about elapsed
//! wall-clock time and about when bytes reach a file descriptor.

use crate::harness::Program;
use std::time::Duration;

#[test]
fn sleep_really_waits() {
    Program::new(
        r#"
        SLEEP(0.4)
        DISPLAY("awake")
        "#,
    )
    .run()
    .success()
    .stdout_is("awake")
    // Deliberately loose: a busy CI runner may take much longer, but it can
    // never legitimately finish early.
    .took_at_least(Duration::from_millis(350));
}

#[test]
fn sleep_accepts_an_integer_and_a_float() {
    Program::new("SLEEP(0)\nDISPLAY(\"integer ok\")")
        .run()
        .success()
        .stdout_is("integer ok");

    Program::new("SLEEP(0.25)\nDISPLAY(\"float ok\")")
        .run()
        .success()
        .stdout_is("float ok")
        .took_at_least(Duration::from_millis(200));
}

#[test]
fn several_sleeps_accumulate() {
    Program::new(
        r#"
        SLEEP(0.2)
        SLEEP(0.2)
        DISPLAY("done")
        "#,
    )
    .run()
    .success()
    .took_at_least(Duration::from_millis(350));
}

#[test]
fn sleep_of_zero_does_not_delay() {
    Program::new("SLEEP(0)\nDISPLAY(\"immediate\")")
        .run()
        .success()
        .took_less_than(Duration::from_secs(5));
}

#[test]
fn sleep_rejects_a_non_numeric_argument() {
    Program::new(r#"SLEEP("a while")"#)
        .run()
        .code(1)
        .stderr_contains("SLEEP requires a numeric argument");
}

#[test]
fn sleep_requires_exactly_one_argument() {
    Program::new("SLEEP()")
        .run()
        .code(1)
        .stderr_contains("SLEEP requires one argument");

    Program::new("SLEEP(1, 2)")
        .run()
        .code(1)
        .stderr_contains("SLEEP requires one argument");
}

#[test]
fn unterminated_output_survives_a_sleep_in_order() {
    // A test running the program to completion cannot observe *when* the bytes were
    // written, only that they all arrived and in order -- including a fragment with
    // no newline, which is the case a line-buffered sink would hold back. Whether it
    // appeared during the stall is what SLEEP's flush is for, and is not asserted.
    Program::new(
        r#"
        DISPLAYINLINE("working")
        SLEEP(0.3)
        DISPLAY("...done")
        "#,
    )
    .run()
    .success()
    .stdout_is("working...done")
    .took_at_least(Duration::from_millis(250));
}

#[test]
fn output_before_a_child_process_survives_in_order() {
    // As above: ordering and survival are checkable from outside, timing is not.
    let echo = if cfg!(target_os = "windows") {
        r#"EXEC("cmd", ["/C", "echo from the child"])"#
    } else {
        r#"EXEC("echo", ["from the child"])"#
    };
    Program::new(&format!(
        r#"
        DISPLAYINLINE("parent first, ")
        r <- {}
        DISPLAY(TRIM(r["stdout"]))
        "#,
        echo
    ))
    .run()
    .success()
    .stdout_is("parent first, from the child");
}

#[test]
fn a_program_killed_by_the_harness_timeout_is_reported_as_such() {
    // A guard on the harness itself: if the watchdog did not work, a hung program
    // would hang the suite instead of failing it.
    let run = Program::new("SLEEP(30)\nDISPLAY(\"never\")")
        .timeout(Duration::from_millis(600))
        .run();
    assert!(run.timed_out, "the watchdog did not kill the child");
    assert!(
        run.elapsed < Duration::from_secs(10),
        "the watchdog waited far too long: {:?}",
        run.elapsed
    );
}
