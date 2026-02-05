use super::{assert_output, get_error, run_test};

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

#[test]
fn test_procedure_scope_isolation() {
    assert_output(
        r#"
            x <- 10
            PROCEDURE setX()
            {
                x <- 99
                DISPLAY(x)
            }
            setX()
            DISPLAY(x)
        "#,
        "99\n10",
    );
}

#[test]
fn test_procedure_parameter_shadows_outer() {
    assert_output(
        r#"
            x <- 100
            PROCEDURE show(x)
            {
                DISPLAY(x)
            }
            show(42)
        "#,
        "42",
    );
}

#[test]
fn test_procedure_no_args() {
    assert_output(
        r#"
            PROCEDURE greet()
            {
                DISPLAY("hello")
            }
            greet()
        "#,
        "hello",
    );
}

#[test]
fn test_procedure_returns_list() {
    assert_output(
        r#"
            PROCEDURE makeList()
            {
                list <- [1, 2, 3]
                RETURN(list)
            }
            result <- makeList()
            DISPLAY(result)
        "#,
        "[1, 2, 3]",
    );
}

#[test]
fn test_mutual_recursion() {
    assert_output(
        r#"
            PROCEDURE isEven(n)
            {
                IF(n = 0)
                {
                    RETURN(TRUE)
                }
                RETURN(isOdd(n - 1))
            }
            PROCEDURE isOdd(n)
            {
                IF(n = 0)
                {
                    RETURN(FALSE)
                }
                RETURN(isEven(n - 1))
            }
            DISPLAY(isEven(4))
            DISPLAY(isOdd(3))
        "#,
        "true\ntrue",
    );
}

#[test]
fn test_procedure_with_list_arg() {
    assert_output(
        r#"
            PROCEDURE sumList(lst)
            {
                total <- 0
                FOR EACH item IN lst
                {
                    total <- total + item
                }
                RETURN(total)
            }
            nums <- [10, 20, 30]
            DISPLAY(sumList(nums))
        "#,
        "60",
    );
}

#[test]
fn test_undefined_procedure_error() {
    let err = get_error("doesNotExist()");
    assert!(!err.is_empty(), "Expected error for undefined procedure");
}

#[test]
fn test_procedure_wrong_arity_error() {
    let result = run_test(
        r#"
            PROCEDURE add(a, b)
            {
                RETURN(a + b)
            }
            add(1)
        "#,
    );
    assert!(
        result.is_err(),
        "Expected error for wrong number of arguments"
    );
}

#[test]
fn test_early_return_skips_rest() {
    assert_output(
        r#"
            PROCEDURE earlyReturn()
            {
                DISPLAY("before")
                RETURN()
                DISPLAY("after")
            }
            earlyReturn()
        "#,
        "before",
    );
}

#[test]
fn test_return_from_loop() {
    assert_output(
        r#"
            PROCEDURE findFirst(lst, target)
            {
                i <- 1
                FOR EACH item IN lst
                {
                    IF(item = target)
                    {
                        RETURN(i)
                    }
                    i <- i + 1
                }
                RETURN(-1)
            }
            DISPLAY(findFirst([10, 20, 30, 40], 30))
        "#,
        "3",
    );
}
