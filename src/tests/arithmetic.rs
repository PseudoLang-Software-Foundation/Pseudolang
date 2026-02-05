use super::{assert_output, run_test};

#[test]
fn test_basic_arithmetic() {
    assert_output("DISPLAY(5 + 3)", "8");
    assert_output("DISPLAY(10 - 4)", "6");
    assert_output("DISPLAY(3 * 4)", "12");
    assert_output("DISPLAY(15 / 3)", "5");
    assert_output("DISPLAY(7 MOD 3)", "1");
    assert_output("DISPLAY(-5 + 3)", "-2");
    assert_output("DISPLAY(2 * (3 + 4))", "14");
    assert_output("DISPLAY((10 + 2) / 3)", "4");
    assert_output("DISPLAY(15 MOD 4)", "3");
}

#[test]
fn test_complex_arithmetic() {
    assert_output(
        r#"
            x <- 5
            y <- 3
            z <- (x + y) * 2
            DISPLAY(z)
            z <- x * y + 4
            DISPLAY(z)
            result <- (z - x) / y
            DISPLAY(result)
            "#,
        "16\n19\n4",
    );
}

#[test]
fn test_mixed_arithmetic() {
    assert_output("DISPLAY(2 + 3 * 4)", "14");
    assert_output("DISPLAY((2 + 3) * 4)", "20");
    assert_output("DISPLAY(10 - 2 * 3)", "4");

    assert_output(
        r#"
            x <- 10
            y <- 3
            z <- (x + y) * 2 - (x / y)
            DISPLAY(z)
            "#,
        "23",
    );

    assert_output(
        r#"
            x <- -5
            y <- 3
            DISPLAY(x + y)
            DISPLAY(x * y)
            DISPLAY(x / y)
            "#,
        "-2\n-15\n-1",
    );
}

#[test]
fn test_division_rounding() {
    assert_output("DISPLAY(5 / 2)", "2");
    assert_output("DISPLAY(-5 / 2)", "-2");
    assert_output("DISPLAY(7 / 3)", "2");
    assert_output("DISPLAY(14 / 4)", "3");

    assert_output(
        r#"
            x <- 19
            y <- 4
            DISPLAY(x / y)
            "#,
        "4",
    );

    assert_output(
        r#"
            x <- 5
            y <- 3
            DISPLAY(x + y)
            DISPLAY(x * y)
            DISPLAY(x / y)
            "#,
        "8\n15\n1",
    );
}

#[test]
fn test_float_operations() {
    assert_output("DISPLAY(5.0 + 3.0)", "8");
    assert_output("DISPLAY(10.5 - 4.2)", "6.3");
    assert_output("DISPLAY(3.0 * 4.0)", "12");
    assert_output("DISPLAY(15.0 / 3.0)", "5");

    assert_output("DISPLAY(5.0 > 3.0)", "true");
    assert_output("DISPLAY(5.0 < 3.0)", "false");
    assert_output("DISPLAY(5.0 = 5.0)", "true");
    assert_output("DISPLAY(5.0 NOT= 5.0)", "false");
    assert_output("DISPLAY(5.0 >= 5.0)", "true");
    assert_output("DISPLAY(5.0 <= 4.0)", "false");

    assert_output("DISPLAY(5 + 3.5)", "8.5");
    assert_output("DISPLAY(10.5 - 4)", "6.5");
    assert_output("DISPLAY(3 * 4.0)", "12");
    assert_output("DISPLAY(15.0 / 3)", "5");

    assert_output("DISPLAY(5.0 > 3)", "true");
    assert_output("DISPLAY(5 < 3.0)", "false");
    assert_output("DISPLAY(5.0 = 5)", "true");
    assert_output("DISPLAY(5 NOT= 5.0)", "false");
    assert_output("DISPLAY(5.0 >= 5)", "true");
    assert_output("DISPLAY(5 <= 4.0)", "false");

    assert_output(
        r#"
            x <- 5.5
            y <- 3.2
            z <- (x + y) * 2.0
            DISPLAY(z)
            "#,
        "17.4",
    );

    assert_output(
        r#"
            IF (4.0 > 5.0) {
                a <- 1
            } ELSE {
                a <- 2
            }
            DISPLAY(a)
            "#,
        "2",
    );

    assert_output(
        r#"
            PROCEDURE add(a, b) {
                RETURN(a + b)
            }
            DISPLAY(add(5.5, 3.2))
            "#,
        "8.7",
    );
}

#[test]
#[allow(clippy::approx_constant)]
fn test_math_functions() {
    fn assert_float_eq(got: &str, expected: f64) {
        let got: f64 = got.trim().parse().unwrap();
        let epsilon = 0.0001;
        assert!(
            (got - expected).abs() < epsilon,
            "Expected {} to be approximately {} (within {})",
            got,
            expected,
            epsilon
        );
    }

    assert_output("DISPLAY(ABS(-42))", "42");
    assert_output("DISPLAY(CEIL(4))", "4");
    assert_output("DISPLAY(FLOOR(4))", "4");
    assert_output("DISPLAY(POW(2, 3))", "8");
    assert_output("DISPLAY(GCD(48, 18))", "6");
    assert_output("DISPLAY(GCD(17, 5))", "1");
    assert_output("DISPLAY(FACTORIAL(0))", "1");
    assert_output("DISPLAY(FACTORIAL(5))", "120");
    assert_output(r#"DISPLAY(ROUND(4.5))"#, "5");
    assert_output(r#"DISPLAY(ROUND(4.4))"#, "4");

    let float_tests = vec![
        ("DISPLAY(ABS(-5.5))", 5.5),
        ("DISPLAY(CEIL(3.1))", 4.0),
        ("DISPLAY(CEIL(-3.1))", -3.0),
        ("DISPLAY(FLOOR(3.9))", 3.0),
        ("DISPLAY(FLOOR(-3.1))", -4.0),
        ("DISPLAY(POW(2.5, 2))", 6.25),
        ("DISPLAY(SQRT(16))", 4.0),
        ("DISPLAY(SQRT(2))", 1.4142135),
        ("DISPLAY(SIN(0))", 0.0),
        ("DISPLAY(SIN(1.5707964))", 1.0),
        ("DISPLAY(COS(0))", 1.0),
        ("DISPLAY(COS(3.1415927))", -1.0),
        ("DISPLAY(TAN(0))", 0.0),
        ("DISPLAY(TAN(0.7853982))", 1.0),
        ("DISPLAY(ASIN(0))", 0.0),
        ("DISPLAY(ASIN(1))", 1.5707964),
        ("DISPLAY(ACOS(1))", 0.0),
        ("DISPLAY(ACOS(-1))", 3.1415927),
        ("DISPLAY(ATAN(0))", 0.0),
        ("DISPLAY(ATAN(1))", 0.7853982),
        ("DISPLAY(EXP(0))", 1.0),
        ("DISPLAY(EXP(1))", 2.7182817),
        ("DISPLAY(LOG(1))", 0.0),
        ("DISPLAY(LOG(2.7182817))", 1.0),
        ("DISPLAY(LOGTEN(10))", 1.0),
        ("DISPLAY(LOGTEN(100))", 2.0),
        ("DISPLAY(LOGTWO(2))", 1.0),
        ("DISPLAY(LOGTWO(8))", 3.0),
        ("DISPLAY(HYPOT(3, 4))", 5.0),
        ("DISPLAY(HYPOT(5, 12))", 13.0),
        ("DISPLAY(DEGREES(3.1415927))", 180.0),
        ("DISPLAY(DEGREES(1.5707964))", 90.0),
        ("DISPLAY(RADIANS(180))", 3.1415927),
        ("DISPLAY(RADIANS(90))", 1.5707964),
    ];

    for (input, expected) in float_tests {
        match run_test(input) {
            Ok(output) => assert_float_eq(&output, expected),
            Err(e) => panic!("Test failed for input '{}': {}", input, e),
        }
    }

    let neg_tests = vec![
        ("DISPLAY(SIN(-1.5707964))", -1.0),
        ("DISPLAY(COS(-3.1415927))", -1.0),
        ("DISPLAY(TAN(-0.7853982))", -1.0),
        ("DISPLAY(ASIN(-1))", -1.5707964),
        ("DISPLAY(ACOS(0))", 1.5707964),
        ("DISPLAY(ATAN(-1))", -0.7853982),
        ("DISPLAY(LOGTEN(0.1))", -1.0),
        ("DISPLAY(LOGTWO(0.5))", -1.0),
        ("DISPLAY(DEGREES(-3.1415927))", -180.0),
        ("DISPLAY(RADIANS(-180))", -3.1415927),
        ("DISPLAY(HYPOT(-3, 4))", 5.0),
        ("DISPLAY(HYPOT(-3, -4))", 5.0),
    ];

    for (input, expected) in neg_tests {
        match run_test(input) {
            Ok(output) => assert_float_eq(&output, expected),
            Err(e) => panic!("Test failed for input '{}': {}", input, e),
        }
    }
}
