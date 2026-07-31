//! `INPUT`, driven from a real standard input pipe.

use crate::harness::Program;

#[test]
fn input_reads_one_line() {
    Program::new(
        r#"
        name <- INPUT()
        DISPLAY(CONCAT("hello, ", name))
        "#,
    )
    .stdin("world\n")
    .run()
    .success()
    .stdout_is("hello, world");
}

#[test]
fn input_reads_successive_lines_in_order() {
    Program::new(
        r#"
        a <- INPUT()
        b <- INPUT()
        c <- INPUT()
        DISPLAY(c)
        DISPLAY(b)
        DISPLAY(a)
        "#,
    )
    .stdin("first\nsecond\nthird\n")
    .run()
    .success()
    .stdout_is("third\nsecond\nfirst");
}

#[test]
fn input_strips_the_line_terminator() {
    // The value must not carry a trailing newline, or every comparison against it
    // would fail.
    Program::new(
        r#"
        line <- INPUT()
        DISPLAY(LENGTH(line))
        DISPLAY(line = "abc")
        "#,
    )
    .stdin("abc\n")
    .run()
    .success()
    .stdout_is("3\ntrue");
}

#[test]
fn input_handles_crlf_terminated_lines() {
    // Piped input on Windows, or a file with CRLF endings piped in anywhere.
    Program::new(
        r#"
        line <- INPUT()
        DISPLAY(LENGTH(line))
        "#,
    )
    .stdin("abc\r\n")
    .run()
    .success()
    .stdout_is("3");
}

#[test]
fn input_with_a_prompt_writes_the_prompt_first() {
    Program::new(
        r#"
        answer <- INPUT("What? ")
        DISPLAY(answer)
        "#,
    )
    .stdin("this\n")
    .run()
    .success()
    .stdout_contains("What?")
    .stdout_contains("this");
}

#[test]
fn input_reads_an_empty_line_as_the_empty_string() {
    Program::new(
        r#"
        line <- INPUT()
        DISPLAY(LENGTH(line))
        DISPLAY(CONCAT("[", CONCAT(line, "]")))
        "#,
    )
    .stdin("\nsecond\n")
    .run()
    .success()
    .stdout_is("0\n[]");
}

#[test]
fn input_at_end_of_input_does_not_hang() {
    // The pipe is closed with nothing in it. Whatever the interpreter decides to
    // do, it must decide promptly rather than blocking for ever -- the harness
    // would kill it and this test would report the timeout.
    let run = Program::new(
        r#"
        line <- INPUT()
        DISPLAY("kept going")
        "#,
    )
    .stdin("")
    .run();
    assert!(
        !run.timed_out,
        "INPUT blocked at EOF instead of returning: {:?}",
        run.stdout
    );
}

#[test]
fn more_inputs_than_lines_does_not_hang() {
    let run = Program::new(
        r#"
        a <- INPUT()
        b <- INPUT()
        c <- INPUT()
        DISPLAY(a)
        DISPLAY("finished")
        "#,
    )
    .stdin("only one\n")
    .run();
    assert!(!run.timed_out, "INPUT blocked once input ran out");
    run.stdout_contains("only one");
}

#[test]
fn input_feeds_tonum_for_numeric_entry() {
    Program::new(
        r#"
        n <- TONUM(INPUT())
        m <- TONUM(INPUT())
        DISPLAY(n + m)
        "#,
    )
    .stdin("20\n22\n")
    .run()
    .success()
    .stdout_is("42");
}

#[test]
fn input_drives_a_loop_until_a_sentinel() {
    Program::new(
        r#"
        total <- 0
        REPEAT UNTIL (FALSE)
        {
            line <- INPUT()
            IF line = "end"
            {
                DISPLAY(total)
                EXIT(0)
            }
            total <- total + TONUM(line)
        }
        "#,
    )
    .stdin("1\n2\n3\nend\n")
    .run()
    .code(0)
    .stdout_is("6");
}

#[test]
fn input_accepts_a_line_containing_spaces_and_punctuation() {
    Program::new(
        r#"
        line <- INPUT()
        DISPLAY(line)
        "#,
    )
    .stdin("a b, \"c\"; d\n")
    .run()
    .success()
    .stdout_is("a b, \"c\"; d");
}

#[test]
fn input_accepts_non_ascii_text() {
    Program::new(
        r#"
        line <- INPUT()
        DISPLAY(line)
        DISPLAY(LENGTH(line))
        "#,
    )
    .stdin("héllo wörld\n")
    .run()
    .success()
    // LENGTH counts characters, not bytes.
    .stdout_is("héllo wörld\n11");
}
