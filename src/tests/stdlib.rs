use super::{assert_output, run_test};

#[test]
fn test_random() {
    let result = run_test("x <- RANDOM(1, 10)\nDISPLAY(x)").unwrap();
    let trimmed_result = result.trim();
    let num: i32 = trimmed_result.parse().unwrap();
    assert!((1..=10).contains(&num));

    let result = run_test(
        r#"
            min <- 1
            max <- 10
            x <- RANDOM(min, max)
            DISPLAY(x)"#,
    )
    .unwrap();
    let trimmed_result = result.trim();
    let num: i32 = trimmed_result.parse().unwrap();
    assert!((1..=10).contains(&num));
}

#[test]
fn test_range() {
    assert_output("DISPLAY(RANGE(5))", "[1, 2, 3, 4, 5]");
    assert_output("DISPLAY(RANGE(2, 5))", "[2, 3, 4, 5]");
    assert_output("DISPLAY(RANGE(1))", "[1]");
    assert_output("DISPLAY(RANGE(1, 1))", "[1]");

    assert!(
        run_test("DISPLAY(RANGE(5, 2))").is_err(),
        "Expected error for invalid range"
    );

    assert_output(
        r#"
            list <- RANGE(3)
            DISPLAY(list[2])
            "#,
        "2",
    );

    assert_output(
        r#"
            list <- RANGE(2, 4)
            DISPLAY(list[2])
            "#,
        "3",
    );
}

#[test]
fn test_eval() {
    assert_output(r#"DISPLAY(EVAL("1 + 2"))"#, "3");
    assert_output(r#"DISPLAY(EVAL("2 * (3 + 4)"))"#, "14");
    assert_output(r#"DISPLAY(EVAL("10 / 2"))"#, "5");
    assert_output(r#"DISPLAY(EVAL("7 MOD 3"))"#, "1");

    assert_output(
        r#"
            x <- 5
            DISPLAY(EVAL("x * 2"))
            DISPLAY(EVAL("x * (x + 1)"))
            "#,
        "10\n30",
    );

    assert_output(
        r#"
            nums <- [1, 3, 5]
            DISPLAY(EVAL("nums[1] + nums[2]"))
            "#,
        "4",
    );

    assert_output(
        r#"
        expression <- "x* (x+1)*(x+2)"
        x <- 3
        DISPLAY(EVAL(expression))"#,
        "60",
    );

    assert_output(r#"DISPLAY(EVAL("POW(2, 3)"))"#, "8");

    assert_output(
        r#"x <- 4
        DISPLAY(TOSTRING(EVAL("x=3")) + " " + TOSTRING(EVAL("x = 4")))"#,
        "false true",
    );

    assert_output(
        r#"
            x <- 10
            y <- 20
            DISPLAY(EVAL("x < y"))
            DISPLAY(EVAL("x = 10"))
            DISPLAY(EVAL("(x > 5) AND (y < 30)"))
            "#,
        "true\ntrue\ntrue",
    );

    assert_output(r#"DISPLAY(EVAL("1.5 + 2.3"))"#, "3.8");
    assert_output(r#"DISPLAY(EVAL("3.0 * (4.5 - 2.5)"))"#, "6");

    assert!(run_test(r#"DISPLAY(EVAL("1 + "))"#).is_err());
    assert!(run_test(r#"DISPLAY(EVAL("1 / 0"))"#).is_err());
    assert!(run_test(r#"DISPLAY(EVAL("invalid"))"#).is_err());
}

#[test]
fn test_timestamp_functions() {
    let result = run_test("DISPLAY(TIMESTAMP())").unwrap();
    let timestamp = result.trim().parse::<f64>().unwrap();
    assert!(timestamp > 0.0);

    assert_output(
        r#"
            ts <- 1625329272
            DISPLAY(TIME(ts))
            "#,
        "2021-07-03 16:21:12",
    );

    assert_output(
        r#"
            dt <- "2021-07-03 16:21:12.000000"
            DISPLAY(TOSTRING(FLOOR(TIMESTAMP(dt))))
            "#,
        "1625329272",
    );

    let roundtrip = run_test(
        r#"
            ts <- TIMESTAMP()
            t <- TIME(ts)
            ts2 <- TIMESTAMP(t)
            DISPLAY(FLOOR(ts) = FLOOR(ts2))
        "#,
    )
    .unwrap();
    assert_eq!(roundtrip.trim(), "true");
}

#[test]
fn test_timezone_functions() {
    let result = run_test(
        r#"
            ts <- 1625329272
            DISPLAY(TIMEZONE(ts, "America/New_York"))
        "#,
    )
    .unwrap();
    assert_eq!(result.trim(), "2021-07-03 12:21:12");

    let result = run_test(
        r#"
            zones <- TIMEZONES()
            found <- FALSE
            FOR EACH zone IN zones {
                IF(zone = "Europe/London") {
                    found <- TRUE
                }
            }
            DISPLAY(found)
        "#,
    )
    .unwrap();
    assert_eq!(result.trim(), "true");

    assert!(
        run_test(
            r#"
            ts <- TIMESTAMP()
            DISPLAY(TIMEZONE(ts, "Invalid/Zone"))
        "#
        )
        .is_err()
    );
}
