use super::{assert_output, get_error, run_test};

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

#[test]
fn test_integer_division_truncation() {
    assert_output("DISPLAY(5 / 2)", "2");
    assert_output("DISPLAY(-5 / 2)", "-2");
    assert_output("DISPLAY(19 / 4)", "4");
    assert_output("DISPLAY(7 / 3)", "2");
    assert_output("DISPLAY(-7 / 3)", "-2");
    assert_output("DISPLAY(1 / 3)", "0");
}

#[test]
fn test_integer_overflow_promotion() {
    assert_output("DISPLAY(9223372036854775807 + 1)", "9223372036854775808");
    assert_output("DISPLAY(9223372036854775807 * 2)", "18446744073709551614");
}

#[test]
fn test_nan_arithmetic_propagation() {
    assert_output("x <- NAN\nDISPLAY(x + 1)", "NAN");
    assert_output("x <- NAN\nDISPLAY(x - 5)", "NAN");
    assert_output("x <- NAN\nDISPLAY(x * 10)", "NAN");
    assert_output("x <- NAN\nDISPLAY(1 + NAN)", "NAN");
}

#[test]
fn test_nan_comparison_semantics() {
    assert_output("DISPLAY(NAN = NAN)", "false");
    assert_output("DISPLAY(NAN NOT= NAN)", "true");
    assert_output("DISPLAY(NAN = 0)", "false");
    assert_output("DISPLAY(NAN NOT= 0)", "true");
}

#[test]
fn test_null_comparison_semantics() {
    assert_output("DISPLAY(NULL = NULL)", "true");
    assert_output("DISPLAY(NULL NOT= NULL)", "false");
    assert_output("DISPLAY(NULL = 0)", "false");
    assert_output("DISPLAY(NULL = \"\")", "false");
}

#[test]
fn test_mixed_int_float_arithmetic() {
    assert_output("DISPLAY(1 + 0.5)", "1.5");
    assert_output("DISPLAY(10 - 2.5)", "7.5");
    assert_output("DISPLAY(3 * 1.5)", "4.5");
    assert_output("DISPLAY(7 / 2.0)", "3.5");
    assert_output("DISPLAY(0.5 + 1)", "1.5");
}

#[test]
fn test_mixed_int_float_comparison() {
    assert_output("DISPLAY(1 = 1.0)", "true");
    assert_output("DISPLAY(2 > 1.5)", "true");
    assert_output("DISPLAY(1.5 < 2)", "true");
    assert_output("DISPLAY(3 >= 3.0)", "true");
    assert_output("DISPLAY(3.0 <= 3)", "true");
    assert_output("DISPLAY(1 NOT= 1.1)", "true");
}

#[test]
fn test_nlog_alias() {
    let result = run_test("DISPLAY(LOG(1))").unwrap();
    assert_eq!(result, "0");
}

#[test]
fn test_modulo_basic() {
    assert_output("DISPLAY(10 MOD 3)", "1");
    assert_output("DISPLAY(15 MOD 5)", "0");
    assert_output("DISPLAY(7 MOD 2)", "1");
}

#[test]
fn test_modulo_by_zero_error() {
    let err = get_error("DISPLAY(10 MOD 0)");
    assert!(err.contains("Modulo by zero"), "{}", err);
}

/// The numeric tower has to be consistent: `Int MOD Float` and `Float MOD Int`
/// always worked, but `Float MOD Float` fell through to the catch-all arm and
/// reported "Invalid operation" instead of computing a remainder.
#[test]
fn test_float_modulo() {
    assert_output("DISPLAY(7.5 MOD 2.0)", "1.5");
    assert_output("DISPLAY(7.5 MOD 2)", "1.5");
    assert_output("DISPLAY(7 MOD 2.5)", "2");
    assert_output("DISPLAY(10.0 MOD 2.5)", "0");
    assert_output("DISPLAY(-7.5 MOD 2.0)", "-1.5");
    assert_output("DISPLAY(1.5 MOD 0.5)", "0");
}

#[test]
fn test_float_modulo_by_zero_error() {
    let err = get_error("DISPLAY(7.5 MOD 0.0)");
    assert!(err.contains("Modulo by zero"), "{}", err);
    let err = get_error("DISPLAY(7.5 MOD 0)");
    assert!(err.contains("Modulo by zero"), "{}", err);
}

#[test]
fn test_unary_negation() {
    assert_output("DISPLAY(-5)", "-5");
    assert_output("DISPLAY(-(-3))", "3");
    assert_output("x <- 10\nDISPLAY(-x)", "-10");
    assert_output("DISPLAY(-2.5)", "-2.5");
}

#[test]
fn test_zero_edge_cases() {
    assert_output("DISPLAY(0 + 0)", "0");
    assert_output("DISPLAY(0 * 999)", "0");
    assert_output("DISPLAY(0 - 0)", "0");
}

#[test]
fn test_bigint_literal_beyond_i64() {
    assert_output("DISPLAY(99999999999999999999)", "99999999999999999999");
    assert_output(
        "DISPLAY(100000000000000000000 + 1)",
        "100000000000000000001",
    );
}

#[test]
fn test_bigint_arithmetic() {
    assert_output(
        "DISPLAY(9223372036854775808 + 9223372036854775808)",
        "18446744073709551616",
    );
    assert_output("DISPLAY(9223372036854775808 * 2)", "18446744073709551616");
    assert_output(
        "DISPLAY(999999999999999999999999 - 999999999999999999999998)",
        "1",
    );
}

#[test]
fn test_bigint_division_modulo() {
    assert_output("DISPLAY(100000000000000000000 / 3)", "33333333333333333333");
    assert_output("DISPLAY(100000000000000000000 MOD 3)", "1");
}

#[test]
fn test_bigint_comparisons() {
    assert_output(
        "DISPLAY(99999999999999999999 > 99999999999999999998)",
        "true",
    );
    assert_output(
        "DISPLAY(99999999999999999999 = 99999999999999999999)",
        "true",
    );
    assert_output(
        "DISPLAY(99999999999999999999 < 100000000000000000000)",
        "true",
    );
}

#[test]
fn test_bigint_factorial() {
    assert_output(
        "DISPLAY(FACTORIAL(50))",
        "30414093201713378043612608166064768844377641568960512000000000000",
    );
    assert_output("DISPLAY(FACTORIAL(0))", "1");
    assert_output("DISPLAY(FACTORIAL(1))", "1");
    assert_output("DISPLAY(FACTORIAL(20))", "2432902008176640000");
    assert_output("DISPLAY(FACTORIAL(21))", "51090942171709440000");
}

#[test]
fn test_bigint_pow() {
    assert_output(
        "DISPLAY(POW(2, 200))",
        "1606938044258990275541962092341162602522202993782792835301376",
    );
    assert_output("DISPLAY(POW(10, 30))", "1000000000000000000000000000000");
}

#[test]
fn test_bigint_tonum() {
    assert_output(
        "DISPLAY(TONUM(\"99999999999999999999\"))",
        "99999999999999999999",
    );
}

#[test]
fn test_bigint_gcd() {
    assert_output("DISPLAY(GCD(100000000000000000000, 3))", "1");
    assert_output(
        "DISPLAY(GCD(100000000000000000000, 100000000000000000000))",
        "100000000000000000000",
    );
}

#[test]
fn test_bigint_tostring() {
    assert_output(
        "x <- 99999999999999999999\nDISPLAY(TOSTRING(x))",
        "99999999999999999999",
    );
}

#[test]
fn test_bigint_negation() {
    assert_output("DISPLAY(-99999999999999999999)", "-99999999999999999999");
    assert_output(
        "x <- 99999999999999999999\nDISPLAY(-x)",
        "-99999999999999999999",
    );
}

#[test]
fn test_bigint_mixed_with_float() {
    assert_output("DISPLAY(1000000 + 0.5)", "1000000.5");
    assert_output("DISPLAY(1000000 * 1.5)", "1500000");
    assert_output(
        "DISPLAY(99999999999999999999 + 1.0)",
        "100000000000000000000",
    );
}

#[test]
fn test_negative_modulo() {
    assert_output("DISPLAY(-10 MOD 3)", "-1");
    assert_output("DISPLAY(10 MOD -3)", "1");
    assert_output("DISPLAY(-10 MOD -3)", "-1");
    assert_output("DISPLAY(-7 MOD 2)", "-1");
    assert_output("DISPLAY(7 MOD -2)", "1");
    assert_output("DISPLAY(-1 MOD 5)", "-1");
}

#[test]
fn test_bigint_modulo() {
    assert_output("DISPLAY(100000000000000000000 MOD 7)", "2");
    assert_output(
        "DISPLAY(999999999999999999999 MOD 1000000000000000000000)",
        "999999999999999999999",
    );
    assert_output("DISPLAY(-100000000000000000000 MOD 3)", "-1");
}

#[test]
fn test_ceil_floor_large_float() {
    assert_output(
        "DISPLAY(CEIL(1000000000000000000.0))",
        "1000000000000000000",
    );
    assert_output(
        "DISPLAY(FLOOR(1000000000000000000.0))",
        "1000000000000000000",
    );
    assert_output(
        "DISPLAY(CEIL(-1000000000000000000.0))",
        "-1000000000000000000",
    );
    assert_output(
        "DISPLAY(FLOOR(-1000000000000000000.0))",
        "-1000000000000000000",
    );
    assert_output(
        "DISPLAY(ROUND(1000000000000000000.0))",
        "1000000000000000000",
    );
    assert_output(
        "DISPLAY(ROUND(-1000000000000000000.0))",
        "-1000000000000000000",
    );
    assert_output("DISPLAY(CEIL(999999999999.5))", "1000000000000");
    assert_output("DISPLAY(FLOOR(999999999999.5))", "999999999999");
}

#[test]
fn test_ceil_floor_on_integers() {
    assert_output(
        "DISPLAY(CEIL(99999999999999999999))",
        "99999999999999999999",
    );
    assert_output(
        "DISPLAY(FLOOR(99999999999999999999))",
        "99999999999999999999",
    );
    assert_output(
        "DISPLAY(ROUND(99999999999999999999))",
        "99999999999999999999",
    );
}

#[test]
fn test_negative_ceil_floor() {
    assert_output("DISPLAY(CEIL(-3.1))", "-3");
    assert_output("DISPLAY(FLOOR(-3.9))", "-4");
    assert_output("DISPLAY(ROUND(-3.5))", "-4");
    assert_output("DISPLAY(ROUND(-3.4))", "-3");
}
