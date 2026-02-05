use super::{assert_output, get_error, run_test};

#[test]
fn test_comparisons() {
    assert_output("DISPLAY(5 > 3)", "true");
    assert_output("DISPLAY(5 < 3)", "false");
    assert_output("DISPLAY(5 = 5)", "true");
    assert_output("DISPLAY(5 NOT= 5)", "false");
    assert_output("DISPLAY(5 >= 5)", "true");
    assert_output("DISPLAY(5 <= 4)", "false");

    assert_output(
        r#"
            a <- 5
            b <- 3
            result <- a > b
            DISPLAY(result)"#,
        "true",
    );

    assert_output(
        r#"
            a <- 5
            b <- 5
            result <- a = b
            DISPLAY(result)"#,
        "true",
    );
}

#[test]
fn test_if_statements() {
    assert_output("IF(TRUE) { DISPLAY(42) }", "42");
    assert_output("IF(FALSE) { DISPLAY(42) } ELSE { DISPLAY(24) }", "24");
}

#[test]
fn test_loops() {
    assert_output("x <- 0\nREPEAT 3 TIMES { x <- x + 1 }\nDISPLAY(x)", "3");
    assert_output(
        "x <- 0\nREPEAT UNTIL(x = 3) { x <- x + 1 }\nDISPLAY(x)",
        "3",
    );
}

#[test]
fn test_nested_loops() {
    assert_output(
        r#"
            result <- 0
            REPEAT 3 TIMES {
                REPEAT 2 TIMES {
                    result <- result + 1
                }
            }
            DISPLAY(result)
            "#,
        "6",
    );
}

#[test]
fn test_foreach() {
    let foreach_test = r#"
            sum <- 0
            list <- [1, 2, 3, 4]
            FOR EACH num IN list {
                sum <- sum + num
            }
            DISPLAY(sum)
        "#;
    assert_output(foreach_test, "10");
}

#[test]
fn test_if_without_else() {
    assert_output(
        r#"
            x <- 5
            IF(x > 3)
            {
                DISPLAY("big")
            }
        "#,
        "big",
    );
}

#[test]
fn test_if_false_no_else_no_output() {
    let result = run_test(
        r#"
            x <- 1
            IF(x > 10)
            {
                DISPLAY("never")
            }
        "#,
    )
    .unwrap();
    assert_eq!(result, "");
}

#[test]
fn test_nested_if_else() {
    assert_output(
        r#"
            x <- 5
            IF(x > 10)
            {
                DISPLAY("big")
            }
            ELSE
            {
                IF(x > 3)
                {
                    DISPLAY("medium")
                }
                ELSE
                {
                    DISPLAY("small")
                }
            }
        "#,
        "medium",
    );
}

#[test]
fn test_repeat_zero_times() {
    let result = run_test(
        r#"
            REPEAT 0 TIMES
            {
                DISPLAY("nope")
            }
        "#,
    )
    .unwrap();
    assert_eq!(result, "");
}

#[test]
fn test_repeat_one_time() {
    assert_output(
        r#"
            REPEAT 1 TIMES
            {
                DISPLAY("once")
            }
        "#,
        "once",
    );
}

#[test]
fn test_repeat_until_is_do_while() {
    assert_output(
        r#"
            REPEAT UNTIL(TRUE)
            {
                DISPLAY("once")
            }
        "#,
        "once",
    );
}

#[test]
fn test_repeat_until_runs_body_then_checks() {
    assert_output(
        r#"
            x <- 0
            REPEAT UNTIL(x >= 3)
            {
                x <- x + 1
            }
            DISPLAY(x)
        "#,
        "3",
    );
}

#[test]
fn test_foreach_empty_list() {
    let result = run_test(
        r#"
            list <- []
            FOR EACH item IN list
            {
                DISPLAY(item)
            }
        "#,
    )
    .unwrap();
    assert_eq!(result, "");
}

#[test]
fn test_foreach_string() {
    assert_output(
        r#"
            FOR EACH ch IN "abc"
            {
                DISPLAYINLINE(ch)
            }
        "#,
        "abc",
    );
}

#[test]
fn test_if_non_boolean_condition_error() {
    let err = get_error("IF(42)\n{\nDISPLAY(1)\n}");
    assert!(
        !err.is_empty(),
        "Expected error for non-boolean IF condition"
    );
}

#[test]
fn test_repeat_times_non_integer_error() {
    let err = get_error("REPEAT \"abc\" TIMES\n{\nDISPLAY(1)\n}");
    assert!(
        !err.is_empty(),
        "Expected error for non-integer REPEAT count"
    );
}

#[test]
fn test_foreach_on_integer_error() {
    let err = get_error("FOR EACH item IN 42\n{\nDISPLAY(item)\n}");
    assert!(
        !err.is_empty(),
        "Expected error for FOR EACH on non-iterable"
    );
}
