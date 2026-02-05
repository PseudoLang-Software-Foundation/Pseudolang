use super::assert_output;

#[test]
fn test_procedures() {
    assert_output(
        r#"
            PROCEDURE add(a, b) {
                RETURN(a + b)
            }
            DISPLAY(add(5, 3))
        "#,
        "8",
    );

    assert_output(
        r#"
            PROCEDURE displayXTimes(text, times) {
                REPEAT times TIMES {
                    DISPLAY(text)
                }
            }
            displayXTimes("Hello", 2)
            "#,
        "Hello\nHello",
    );
}

#[test]
fn test_procedures_complex() {
    assert_output(
        r#"
            PROCEDURE factorial(n) {
                IF (n <= 1) {
                    RETURN(1)
                }
                RETURN(n * factorial(n-1))
            }
            DISPLAY(factorial(5))
            "#,
        "120",
    );

    assert_output(
        r#"
            PROCEDURE power(base, exp) {
                result <- 1
                REPEAT exp TIMES {
                    result <- result * base
                }
                RETURN(result)
            }
            DISPLAY(power(2, 3))
            "#,
        "8",
    );

    assert_output(
        r#"
            PROCEDURE factorial(n)
            {
                IF(n <= 1)
                {
                    RETURN(1)
                }
                ELSE
                {
                    RETURN(n * factorial(n - 1))
                }
            }
            DISPLAY(factorial(5))
            "#,
        "120",
    );
}

#[test]
fn test_return() {
    assert_output(
        r#"
        PROCEDURE test1(num) {
            RETURN (num)
        }

        PROCEDURE test2(num) {
            RETURN num
        }

        PROCEDURE test3() {
            RETURN
        }

        PROCEDURE test4() {
            RETURN ()
        }

        DISPLAY(test1(5))
        DISPLAY(test2(6))
        DISPLAY(test3())
        DISPLAY(test4())
        "#,
        "5\n6",
    );
}

#[test]
fn test_void_returns() {
    assert_output(
        r#"
            PROCEDURE printAndReturn(x) {
                DISPLAY(x)
                RETURN()
            }
            DISPLAY(printAndReturn(42))
            "#,
        "42",
    );

    assert_output(
        r#"
            PROCEDURE early_exit(x) {
                IF (x < 0) {
                    RETURN
                }
                DISPLAY(x)
            }
            early_exit(-1)
            early_exit(5)
            "#,
        "5",
    );

    assert_output(
        r#"
            PROCEDURE getEqual(arr, num) {
                IF (LENGTH(arr) NOT= num) {
                    DISPLAY("Not equal")
                    RETURN
                } ELSE {
                    DISPLAY("Equal")
                    RETURN
                }
            }
            arr <- [1, 2, 3]
            num <- 3
            getEqual(arr, num)
            "#,
        "Equal",
    );

    assert_output(
        r#"
            PROCEDURE getEqual(arr, num) {
                IF (LENGTH(arr) NOT= num) {
                    DISPLAY("Not equal")
                    RETURN()
                }
                DISPLAY("Equal")
                RETURN()
            }
            arr <- [1, 2]
            num <- 3
            getEqual(arr, num)
            "#,
        "Not equal",
    );
}

#[test]
fn test_optional_parentheses() {
    assert_output("IF TRUE { DISPLAY(42) }", "42");
    assert_output("IF FALSE { DISPLAY(42) } ELSE { DISPLAY(24) }", "24");

    assert_output(
        r#"
            IF TRUE {
                IF FALSE {
                    DISPLAY(1)
                } ELSE {
                    DISPLAY(2)
                }
            }
        "#,
        "2",
    );

    assert_output(
        r#"
            x <- 0
            REPEAT UNTIL x = 3 {
                x <- x + 1
            }
            DISPLAY(x)
            "#,
        "3",
    );

    assert_output(
        r#"
            IF (TRUE) { DISPLAY(1) }
            IF TRUE { DISPLAY(2) }
            IF FALSE { DISPLAY(3) } ELSE IF TRUE { DISPLAY(4) }
            IF (FALSE) { DISPLAY(5) } ELSE IF (TRUE) { DISPLAY(6) }
        "#,
        "1\n2\n4\n6",
    );
}
