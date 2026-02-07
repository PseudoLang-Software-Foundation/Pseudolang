use super::{assert_output, run_test};

#[test]
fn test_display() {
    assert_output("DISPLAY(42)", "42");
    assert_output("DISPLAY(TRUE)", "true");
    assert_output(r#"DISPLAY("Hello")"#, "Hello");
    assert_output("DISPLAY([1, 2, 3])", "[1, 2, 3]");
    assert_output("DISPLAY(5.5)", "5.5");
    assert_output("DISPLAY(-42)", "-42");
    assert_output("DISPLAY(FALSE)", "false");
    assert_output("DISPLAY([])", "[]");
    assert_output(r#"DISPLAYINLINE("3")"#, "3");

    assert_output("DISPLAYINLINE(\"Hello\")", "Hello");

    assert_output(
        r#"
            DISPLAYINLINE("A")
            DISPLAYINLINE("B")
            DISPLAYINLINE("C")"#,
        "ABC",
    );

    assert_output(
        r#"
            DISPLAY("First")
            DISPLAYINLINE("Hello ")
            DISPLAYINLINE("World")
            DISPLAY("\nLast")"#,
        "First\nHello World\nLast",
    );
}

#[test]
fn test_variable_assignment() {
    assert_output("a <- 42\nDISPLAY(a)", "42");
    assert_output("a <- 5\nb <- a + 3\nDISPLAY(b)", "8");
    assert_output(
        r#"
            a <- 5
            b <- a
            a <- 10
            DISPLAY(b)
            "#,
        "5",
    );
    assert_output(
        r#"
            x <- 1
            y <- 2
            z <- x + y
            x <- 3
            DISPLAY(z)
            "#,
        "3",
    );
}

#[test]
fn test_comments() {
    assert_output(
        r#"
            COMMENT DISPLAY(43)
            DISPLAY(42)
            "#,
        "42",
    );

    assert_output(
        r#"
            COMMENTBLOCK
            This is a comment
            DISPLAY(43)
            COMMENTBLOCK
            DISPLAY(42)
            "#,
        "42",
    );

    assert_output(
        r#"
            COMMENT DISPLAY(43)
            COMMENTBLOCK
            DISPLAY(43)
            COMMENTBLOCK
            DISPLAY(42)
            COMMENT DISPLAY(43)
            "#,
        "42",
    );

    assert_output(
        r#"
            COMMENTBLOCK
            DISPLAY(43)
            DISPLAY(44)
            COMMENTBLOCK
            COMMENT DISPLAY(43)
            DISPLAY(42)
            COMMENT DISPLAY(43)
            "#,
        "42",
    );
}

#[test]
fn test_type_conversions() {
    assert_output(r#"DISPLAY(TOSTRING(42))"#, "42");
    assert_output(r#"DISPLAY(TONUM("42"))"#, "42");

    assert_output(
        r#"
            str <- TOSTRING(42)
            DISPLAY(str)"#,
        "42",
    );
    assert_output(
        r#"
            num <- TONUM("42")
            DISPLAY(num)"#,
        "42",
    );
}

#[test]
fn test_null_and_nan() {
    assert_output("DISPLAY(NULL)", "NULL");
    assert_output("DISPLAY(NAN)", "NAN");

    assert_output(
        r#"
            x <- NULL
            y <- NULL
            DISPLAY(x = y)
            DISPLAY(x NOT= y)
            "#,
        "true\nfalse",
    );

    assert_output(
        r#"
            x <- NAN
            y <- NAN
            DISPLAY(x = y)
            DISPLAY(x NOT= y)
            "#,
        "false\ntrue",
    );

    assert_output(
        r#"
            x <- NULL
            y <- 42
            DISPLAY(x = y)
            "#,
        "false",
    );

    assert_output(
        r#"
            x <- NAN
            y <- 42
            DISPLAY(x = y)
            "#,
        "false",
    );

    assert_output(
        r#"
            x <- NAN
            y <- x + 5
            DISPLAY(y)
            "#,
        "NAN",
    );
}

#[test]
fn test_boolean_operations() {
    assert_output("DISPLAY(TRUE AND FALSE)", "false");
    assert_output("DISPLAY(TRUE OR FALSE)", "true");
    assert_output("DISPLAY(NOT TRUE)", "false");

    assert_output(
        r#"
            a <- TRUE
            b <- FALSE
            result <- a AND b
            DISPLAY(result)"#,
        "false",
    );

    assert_output(
        r#"
            a <- TRUE
            b <- FALSE
            result <- a OR b
            DISPLAY(result)"#,
        "true",
    );

    assert_output(
        r#"
            val <- TRUE
            result <- NOT val
            DISPLAY(result)"#,
        "false",
    );
}

#[test]
fn test_boolean_operations_complex() {
    assert_output(
        r#"
            PROCEDURE isPositive(num) {
                RETURN(num > 0)
            }
            PROCEDURE isEven(num) {
                RETURN(num MOD 2 = 0)
            }
            a <- 42
            b <- -3
            result <- isPositive(a) AND isEven(a)
            DISPLAY(result)
            result <- isPositive(b) OR isEven(b)
            DISPLAY(result)
            result <- NOT (isPositive(b) AND isEven(b))
            DISPLAY(result)
            "#,
        "true\nfalse\ntrue",
    );

    assert_output(
        r#"
            x <- TRUE
            y <- FALSE
            DISPLAY(x = y)
            DISPLAY(x NOT= y)
            DISPLAY(TRUE = TRUE)
            DISPLAY(FALSE = FALSE)
            "#,
        "false\ntrue\ntrue\ntrue",
    );

    assert_output(
        r#"
            PROCEDURE boolToNum(bool) {
                IF (bool = FALSE) {
                    RETURN (0)
                } ELSE {
                    RETURN (1)
                }
            }

            DISPLAY(boolToNum(TRUE))
            DISPLAY(boolToNum(FALSE))
            "#,
        "1\n0",
    );

    assert_output(
        r#"
            PROCEDURE isInRange(num, min, max) {
                RETURN(num >= min AND num <= max)
            }
            PROCEDURE isValidScore(score) {
                RETURN(isInRange(score, 0, 100))
            }
            DISPLAY(isValidScore(75))
            DISPLAY(isValidScore(-5))
            DISPLAY(isValidScore(150))
            "#,
        "true\nfalse\nfalse",
    );
}

#[test]
fn test_empty_display() {
    assert_output("DISPLAY()", "");
}

#[test]
fn test_display_empty_line_between() {
    assert_output("DISPLAY(1)\nDISPLAY()\nDISPLAY(2)", "1\n\n2");
}

#[test]
fn test_display_multiline_program() {
    assert_output("DISPLAY(1)\nDISPLAY(2)\nDISPLAY(3)", "1\n2\n3");
}

#[test]
fn test_variable_reassignment() {
    assert_output("x <- 1\nx <- 2\nx <- 3\nDISPLAY(x)", "3");
}

#[test]
fn test_variable_self_reference() {
    assert_output("x <- 5\nx <- x + 1\nDISPLAY(x)", "6");
    assert_output("x <- 10\nx <- x * x\nDISPLAY(x)", "100");
}

#[test]
fn test_tostring_types() {
    assert_output("DISPLAY(TOSTRING(42))", "42");
    assert_output("DISPLAY(TOSTRING(3.14))", "3.14");
    assert_output("DISPLAY(TOSTRING(TRUE))", "true");
    assert_output("DISPLAY(TOSTRING(FALSE))", "false");
}

#[test]
fn test_tonum_float_string() {
    assert_output("DISPLAY(TONUM(\"3.14\"))", "3.14");
    assert_output("DISPLAY(TONUM(\"0\"))", "0");
    assert_output("DISPLAY(TONUM(\"-5\"))", "-5");
}

#[test]
fn test_tonum_invalid() {
    assert!(run_test("DISPLAY(TONUM(\"abc\"))").is_err());
    assert!(run_test("DISPLAY(TONUM(42))").is_err());
}

#[test]
fn test_null_display() {
    assert_output("DISPLAY(NULL)", "NULL");
}

#[test]
fn test_nan_display() {
    assert_output("DISPLAY(NAN)", "NAN");
}

#[test]
fn test_boolean_not_operator() {
    assert_output("DISPLAY(NOT TRUE)", "false");
    assert_output("DISPLAY(NOT FALSE)", "true");
}

#[test]
fn test_comment_does_not_affect_code() {
    assert_output("x <- 10 COMMENT this is x\nDISPLAY(x)", "10");
}

#[test]
fn test_double_slash_comments() {
    assert_output("// this is a comment\nDISPLAY(42)", "42");
    assert_output("x <- 10\n// DISPLAY(99)\nDISPLAY(x)", "10");
    assert_output("// first\n// second\nDISPLAY(1)", "1");
}

#[test]
fn test_hash_comments() {
    assert_output("# this is a comment\nDISPLAY(42)", "42");
    assert_output("x <- 10\n# DISPLAY(99)\nDISPLAY(x)", "10");
    assert_output("# first\n# second\nDISPLAY(1)", "1");
}
