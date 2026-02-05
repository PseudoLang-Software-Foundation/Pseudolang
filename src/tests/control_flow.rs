use super::assert_output;

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
