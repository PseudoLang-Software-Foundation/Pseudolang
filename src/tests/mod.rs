#[cfg(test)]
mod test {
    use crate::interpreter;
    use crate::lexer::Lexer;
    use crate::parser;

    fn run_test(input: &str) -> Result<String, String> {
        let mut lexer = Lexer::new(input);
        let tokens = lexer.tokenize();
        let ast = parser::parse(tokens, false)?;
        interpreter::run(ast)
    }

    fn assert_output(input: &str, expected: &str) {
        match run_test(input) {
            Ok(output) => assert_eq!(output.trim(), expected),
            Err(e) => panic!("Test failed for input '{}': {}", input, e),
        }
    }

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
            COMMENT This is a comment
            DISPLAY(42)
            "#,
            "42",
        );

        assert_output(
            r#"
            COMMENTBLOCK
            This is a comment
            43
            COMMENTBLOCK
            DISPLAY(42)
            "#,
            "42",
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
    fn test_list_operations() {
        assert_output("list <- [1, 2, 3]\nDISPLAY(list[1])", "1");
        assert_output(
            "list <- [1, 2, 3]\nAPPEND(list, 4)\nDISPLAY(list)",
            "[1, 2, 3, 4]",
        );
        assert_output(
            "list <- [1, 2, 3]\nREMOVE(list, 2)\nDISPLAY(list)",
            "[1, 3]",
        );
        assert_output("list <- [1, 2, 3]\nDISPLAY(LENGTH(list))", "3");

        assert_output(
            r#"
            list <- [1, 2, 3]
            idx <- 1
            val <- list[idx]
            DISPLAY(val)"#,
            "1",
        );

        assert_output(
            r#"
            list <- [1, 2, 3]
            item <- 4
            APPEND(list, item)
            DISPLAY(list)"#,
            "[1, 2, 3, 4]",
        );

        assert_output(
            r#"
            list <- [1, 2, 3]
            idx <- 2
            REMOVE(list, idx)
            DISPLAY(list)"#,
            "[1, 3]",
        );

        assert_output(
            r#"
            list <- [1, 2, 3]
            b <- REMOVE(list, 2)
            DISPLAY(b)
            "#,
            "2",
        );

        assert_output(
            r#"
            list <- [1, 2, 3]
            b <- APPEND(list, 4)
            DISPLAY(b)
            "#,
            "4",
        );

        assert_output(
            r#"
            list <- [1, 3, 4]
            b <- INSERT(list, 2, 2)
            DISPLAY(b)
            "#,
            "2",
        );

        assert_output(
            r#"
            a <- [1, 2, 3]
            b <- [4, 5, 6]
            DISPLAY(a + b)
            "#,
            "[1, 2, 3, 4, 5, 6]",
        );

        assert_output(
            r#"
            empty <- []
            full <- [1, 2, 3]
            DISPLAY(empty + full)
            DISPLAY(full + empty)
            "#,
            "[1, 2, 3]\n[1, 2, 3]",
        );

        assert_output(
            r#"
            a <- [1]
            b <- [2]
            c <- [3]
            DISPLAY(a + b + c)
            "#,
            "[1, 2, 3]",
        );
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
    fn test_string_operations() {
        let test_cases = vec![
            (r#"DISPLAY(CONCAT("Hello, ", "World!"))"#, "Hello, World!\n"),
            (
                r#"
                s1 <- "Hello, "
                s2 <- "World!"
                result <- CONCAT(s1, s2)
                DISPLAY(result)
                "#,
                "Hello, World!\n",
            ),
            (
                r#"
                str <- "Hello"
                len <- LENGTH(str)
                DISPLAY(len)
                "#,
                "5\n",
            ),
            (
                r#"
                str <- "Hello"
                sub <- SUBSTRING(str, 1, 2)
                DISPLAY(sub)
                "#,
                "He\n",
            ),
        ];

        for (input, expected_output) in test_cases {
            let ast = crate::parser::parse(crate::lexer::Lexer::new(input).tokenize(), false)
                .expect("Failed to parse");
            let output = crate::interpreter::run(ast).expect("Interpreter error");
            assert_eq!(output, expected_output, "Test failed for input '{}'", input);
        }
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
    fn test_random() {
        let result = run_test("x <- RANDOM(1, 10)\nDISPLAY(x)").unwrap();
        let trimmed_result = result.trim();
        let num: i32 = trimmed_result.parse().unwrap();
        assert!(num >= 1 && num <= 10);

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
        assert!(num >= 1 && num <= 10);
    }

    #[test]
    fn test_sort() {
        assert_output(
            "list <- [3, 1, 4, 1, 5]\nDISPLAY(SORT(list))",
            "[1, 1, 3, 4, 5]",
        );

        assert_output(
            r#"
            list <- [3, 1, 4, 1, 5]
            sorted <- SORT(list)
            DISPLAY(sorted)"#,
            "[1, 1, 3, 4, 5]",
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
    fn test_raw_string() {
        assert_output(r#"DISPLAY(r"Hello\nWorld")"#, r"Hello\nWorld");
    }

    #[test]
    fn test_formatted_string() {
        assert_output(
            r#"
                name <- "World"
                DISPLAY(f"Hello {name}!")
            "#,
            "Hello World!",
        );

        assert_output(
            r#"
                first <- "Hello"
                second <- "World"
                DISPLAY(f"{first} {second}!")
            "#,
            "Hello World!",
        );
    }

    #[test]
    fn test_fibonacci_seq() {
        assert_output(
            r#"
        PROCEDURE fibonacci(n)
        {
            a <- 0
            b <- 1
            result <- [a, b]

            REPEAT (n-2) TIMES
            {
                temp <- a + b
                APPEND(result, temp)
                a <- b
                b <- temp
            }

            RETURN(result)
        }

        n <- 10
        fibSequence <- fibonacci(n)
        DISPLAY(fibSequence)
        "#,
            "[0, 1, 1, 2, 3, 5, 8, 13, 21, 34]",
        );

        assert_output(
            r#"
            PROCEDURE fibonacci(n)
            {
                IF(n <= 0)
                {
                    RETURN(0)
                }
                IF(n = 1)
                {
                    RETURN(1)
                }
                RETURN(fibonacci(n - 1) + fibonacci(n - 2))
            }
            DISPLAY(fibonacci(10))"#,
            "55",
        );
    }

    #[test]
    #[should_panic]
    fn test_division_by_zero() {
        run_test("DISPLAY(5 / 0)").unwrap();
    }

    #[test]
    #[should_panic(expected = "Undefined variable")]
    fn test_undefined_variable() {
        run_test("DISPLAY(undefined_var)").unwrap();
    }

    #[test]
    #[should_panic]
    fn test_invalid_list_access() {
        run_test("list <- [1, 2, 3]\nDISPLAY(list[4])").unwrap();
    }

    #[test]
    fn test_list_complex_operations() {
        assert_output(
            "list <- [1, 2, 3]\nlist[2] <- 5\nDISPLAY(list)",
            "[1, 5, 3]",
        );

        assert_output(
            "list <- [1, 2, 3]\nINSERT(list, 2, 5)\nDISPLAY(list)",
            "[1, 5, 2, 3]",
        );

        assert_output(
            r#"
            list <- [1, 2, 3]
            INSERT(list, 2, 5)
            list[4] <- 6
            REMOVE(list, 1)
            DISPLAY(list)
            "#,
            "[5, 2, 6]",
        );

        assert_output(
            r#"
            list <- [1, 2, 3]
            second <- [4, 5, 6]
            list[2] <- second[1]
            DISPLAY(list)
            "#,
            "[1, 4, 3]",
        );

        assert_output(
            r#"
            list <- [1, 2, 3]
            INSERT(list, 1, 0)
            APPEND(list, 4)
            INSERT(list, 3, 2)
            DISPLAY(list)
            "#,
            "[0, 1, 2, 2, 3, 4]",
        );
    }

    #[test]
    fn test_list_modifications() {
        assert_output(
            r#"
            nums <- [1, 2, 3, 4, 5]
            REMOVE(nums, 2)
            nums[2] <- 6
            INSERT(nums, 4, 7)
            APPEND(nums, 8)
            DISPLAY(nums)
            "#,
            "[1, 6, 4, 7, 5, 8]",
        );
    }

    #[test]
    #[should_panic(expected = "List index out of bounds")]
    fn test_list_invalid_insert() {
        run_test("list <- [1, 2, 3]\nINSERT(list, 5, 4)").unwrap();
    }

    #[test]
    #[should_panic(expected = "List index out of bounds")]
    fn test_list_invalid_assignment() {
        run_test("list <- [1, 2, 3]\nlist[4] <- 5").unwrap();
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
    fn test_list_complex() {
        assert_output(
            r#"
            PROCEDURE reverseList(list) {
                result <- []
                i <- LENGTH(list)
                REPEAT LENGTH(list) TIMES {
                    APPEND(result, list[i])
                    i <- i - 1
                }
                RETURN(result)
            }
            list <- [1, 2, 3, 4]
            reversed <- reverseList(list)
            DISPLAY(reversed)
            "#,
            "[4, 3, 2, 1]",
        );

        assert_output(
            r#"
            list <- [1, 2, 3]
            APPEND(list, 4)
            INSERT(list, 2, 5)
            removed <- REMOVE(list, 3)
            DISPLAY(removed)
            DISPLAY(list)
            "#,
            "2\n[1, 5, 3, 4]",
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
    fn test_string_manipulation() {
        assert_output(
            r#"
            str <- "Hello"
            DISPLAY(LENGTH(str))
            sub <- SUBSTRING(str, 2, 4)
            DISPLAY(sub)
            combined <- CONCAT(sub, "!")
            DISPLAY(combined)
            "#,
            "5\nell\nell!",
        );

        assert_output(
            r#"
            PROCEDURE replaceChar(str, oldChar, newChar) {
                result <- ""
                FOR EACH char IN str {
                    IF (char = oldChar) {
                        result <- CONCAT(result, newChar)
                    } ELSE {
                        result <- CONCAT(result, char)
                    }
                }
                RETURN(result)
            }
            DISPLAY(replaceChar("hello", "l", "w"))
            "#,
            "hewwo",
        );
    }

    #[test]
    fn test_error_handling() {
        assert!(run_test("DISPLAY(5 / 0)").is_err());

        assert!(run_test("list <- [1, 2, 3]\nDISPLAY(list[4])").is_err());

        assert!(run_test("DISPLAY(undefined)").is_err());

        assert!(run_test("nonexistent(123)").is_err());
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
    fn test_string_iteration() {
        assert_output(
            r#"
            PROCEDURE replaceChar(str, oldChar, newChar) {
                result <- ""
                FOR EACH char IN str {
                    IF (char = oldChar) {
                        result <- CONCAT(result, newChar)
                    } ELSE {
                        result <- CONCAT(result, char)
                    }
                }
                RETURN(result)
            }
            DISPLAY(replaceChar("hello", "l", "w"))
            "#,
            "hewwo",
        );

        assert_output(
            r#"
            str <- "Hello"
            count <- 0
            FOR EACH char IN str {
                IF (char = "l") {
                    count <- count + 1
                }
            }
            DISPLAY(count)
            "#,
            "2",
        );
    }

    #[test]
    fn test_list_and_string_indexing() {
        assert_output(
            r#"
            list <- [10, 20, 30]
            DISPLAY(list[1])
            DISPLAY(list[2])
            DISPLAY(list[3])
            "#,
            "10\n20\n30",
        );

        assert_output(
            r#"
            str <- "Hello"
            DISPLAY(str[1])
            DISPLAY(str[5])
            "#,
            "H\no",
        );

        assert_output(
            r#"
            list <- [1, 2, 3, 4, 5]
            idx <- 3
            DISPLAY(list[idx])
            "#,
            "3",
        );
    }

    #[test]
    #[should_panic(expected = "List index out of bounds")]
    fn test_list_index_out_of_bounds_high() {
        run_test("list <- [1, 2, 3]\nDISPLAY(list[4])").unwrap();
    }

    #[test]
    #[should_panic(expected = "List index out of bounds")]
    fn test_list_index_out_of_bounds_low() {
        run_test("list <- [1, 2, 3]\nDISPLAY(list[0])").unwrap();
    }

    #[test]
    #[should_panic(expected = "String index out of bounds: 3 (size: 2)")]
    fn test_string_index_out_of_bounds_high() {
        run_test(
            r#"str <- "hi"
DISPLAY(str[3])"#,
        )
        .unwrap();
    }

    #[test]
    #[should_panic(expected = "String index out of bounds: index cannot be less than 1")]
    fn test_string_index_out_of_bounds_low() {
        run_test(
            r#"str <- "hi"
DISPLAY(str[0])"#,
        )
        .unwrap();
    }

    #[test]
    fn test_string_indexing_edge_cases() {
        assert_output(
            r#"
            str <- "A"
            DISPLAY(str[1])
            "#,
            "A",
        );

        assert!(run_test(r#"str <- ""\nDISPLAY(str[1])"#).is_err());
    }

    #[test]
    fn test_list_manipulation_with_indexes() {
        assert_output(
            r#"
            list <- [1, 2, 3]
            first <- list[1]
            last <- list[3]
            list[1] <- last
            list[3] <- first
            DISPLAY(list)
            "#,
            "[3, 2, 1]",
        );
    }

    #[test]
    fn test_string_reverse() {
        assert_output(
            r#"
            PROCEDURE reverse_string(s)
            {
                result <- ""
                FOR EACH char IN s
                {
                    result <- CONCAT(char, result)
                }
                RETURN(result)
            }
            DISPLAY(reverse_string("hello"))
            "#,
            "olleh",
        );
    }

    #[test]
    fn test_bubble_sort() {
        assert_output(
            r#"
                    PROCEDURE bubbleSort(aList)
                    {
                        n <- LENGTH(aList)
                        REPEAT n TIMES
                        {
                            j <- 1
                            REPEAT n-1 TIMES
                            {
                                IF(aList[j] > aList[j + 1])
                                {
                                    temp <- aList[j]
                                    aList[j] <- aList[j + 1]
                                    aList[j + 1] <- temp
                                }
                                j <- j + 1
                            }
                        }
                        RETURN (aList)
                    }

                    a <- [1, 3, 2, 3, 4, 7, 2]
                    DISPLAY(bubbleSort(a))"#,
            "[1, 2, 2, 3, 3, 4, 7]",
        );
    }

    #[test]
    fn test_math_functions() {
        assert_output("DISPLAY(ABS(-5.5))", "5.5");
        assert_output("DISPLAY(ABS(3))", "3");

        assert_output("DISPLAY(CEIL(3.1))", "4");
        assert_output("DISPLAY(CEIL(-3.1))", "-3");

        assert_output("DISPLAY(FLOOR(3.9))", "3");
        assert_output("DISPLAY(FLOOR(-3.1))", "-4");

        assert_output("DISPLAY(POW(2, 3))", "8");
        assert_output("DISPLAY(POW(2.5, 2))", "6.25");

        assert_output("DISPLAY(SQRT(16))", "4");
        assert_output("DISPLAY(SQRT(2))", "1.4142135");

        assert_output("DISPLAY(SIN(0))", "0");
        assert_output("DISPLAY(SIN(1.5707964))", "1");

        assert_output("DISPLAY(COS(0))", "1");
        assert_output("DISPLAY(COS(3.1415927))", "-1");

        assert_output("DISPLAY(TAN(0))", "0");
        assert_output("DISPLAY(TAN(0.7853982))", "1");

        assert_output("DISPLAY(ASIN(0))", "0");
        assert_output("DISPLAY(ASIN(1))", "1.5707964");

        assert_output("DISPLAY(ACOS(1))", "0");
        assert_output("DISPLAY(ACOS(-1))", "3.1415927");

        assert_output("DISPLAY(ATAN(0))", "0");
        assert_output("DISPLAY(ATAN(1))", "0.7853982");

        assert_output("DISPLAY(EXP(0))", "1");
        assert_output("DISPLAY(EXP(1))", "2.7182817");

        assert_output("DISPLAY(LOG(1))", "0");
        assert_output("DISPLAY(LOG(2.7182817))", "0.99999994");

        assert_output("DISPLAY(LOGTEN(10))", "1");
        assert_output("DISPLAY(LOGTEN(100))", "2");

        assert_output("DISPLAY(LOGTWO(2))", "1");
        assert_output("DISPLAY(LOGTWO(8))", "3");

        assert_output("DISPLAY(GCD(48, 18))", "6");
        assert_output("DISPLAY(GCD(17, 5))", "1");

        assert_output("DISPLAY(FACTORIAL(0))", "1");
        assert_output("DISPLAY(FACTORIAL(5))", "120");

        assert_output("DISPLAY(DEGREES(3.1415927))", "180");
        assert_output("DISPLAY(DEGREES(1.5707964))", "90");

        assert_output("DISPLAY(RADIANS(180))", "3.1415927");
        assert_output("DISPLAY(RADIANS(90))", "1.5707964");

        assert_output("DISPLAY(HYPOT(3, 4))", "5");
        assert_output("DISPLAY(HYPOT(5, 12))", "13");

        assert_output("DISPLAY(ABS(-5.5))", "5.5");
        assert_output("DISPLAY(ABS(-42))", "42");
        assert_output("DISPLAY(POW(-2, 3))", "-8");
        assert_output("DISPLAY(POW(-2, 2))", "4");

        assert_output("DISPLAY(FLOOR(-3.1))", "-4");
        assert_output("DISPLAY(FLOOR(-3.9))", "-4");
        assert_output("DISPLAY(CEIL(-3.1))", "-3");
        assert_output("DISPLAY(CEIL(-3.9))", "-3");

        assert_output("DISPLAY(SIN(-1.5707964))", "-1");
        assert_output("DISPLAY(COS(-3.1415927))", "-1");
        assert_output("DISPLAY(TAN(-0.7853982))", "-1");

        assert_output("DISPLAY(ASIN(-1))", "-1.5707964");
        assert_output("DISPLAY(ACOS(0))", "1.5707964");
        assert_output("DISPLAY(ATAN(-1))", "-0.7853982");

        assert_output("DISPLAY(LOG(2.7182817))", "0.99999994");
        assert_output("DISPLAY(LOGTEN(0.1))", "-1");
        assert_output("DISPLAY(LOGTWO(0.5))", "-1");

        assert_output("DISPLAY(GCD(-48, 18))", "6");
        assert_output("DISPLAY(GCD(-48, -18))", "6");

        assert_output("DISPLAY(DEGREES(-3.1415927))", "-180");
        assert_output("DISPLAY(RADIANS(-180))", "-3.1415927");

        assert_output("DISPLAY(HYPOT(-3, 4))", "5");
        assert_output("DISPLAY(HYPOT(-3, -4))", "5");
    }

    #[test]
    #[should_panic]
    fn test_math_error_handling() {
        assert!(run_test("DISPLAY(SQRT(-1))").is_err());
        assert!(run_test("DISPLAY(FACTORIAL(-1))").is_err());
        assert!(run_test("DISPLAY(LOG(-1))").is_err());
        assert!(run_test("DISPLAY(LOGTEN(-1))").is_err());
        assert!(run_test("DISPLAY(LOGTWO(-1))").is_err());

        assert!(run_test("DISPLAY(GCD(1.5, 2))").is_err());
        assert!(run_test("DISPLAY(GCD(1, TRUE))").is_err());

        assert!(run_test("DISPLAY(FACTORIAL(1.5))").is_err());
        assert!(run_test("DISPLAY(FACTORIAL(TRUE))").is_err());

        assert!(run_test("DISPLAY(SQRT(-1))").is_err());
        assert!(run_test("DISPLAY(SQRT(-4))").is_err());
        assert!(run_test("DISPLAY(LOG(-1))").is_err());
        assert!(run_test("DISPLAY(LOG(0))").is_err());
        assert!(run_test("DISPLAY(LOGTEN(-1))").is_err());
        assert!(run_test("DISPLAY(LOGTEN(0))").is_err());
        assert!(run_test("DISPLAY(LOGTWO(-1))").is_err());
        assert!(run_test("DISPLAY(LOGTWO(0))").is_err());

        assert!(run_test("DISPLAY(GCD(1.5, 2))").is_err());
        assert!(run_test("DISPLAY(GCD(1, TRUE))").is_err());
        assert!(run_test("DISPLAY(GCD(\"1\", 2))").is_err());

        assert!(run_test("DISPLAY(FACTORIAL(-1))").is_err());
        assert!(run_test("DISPLAY(FACTORIAL(1.5))").is_err());
        assert!(run_test("DISPLAY(FACTORIAL(TRUE))").is_err());
        assert!(run_test("DISPLAY(FACTORIAL(\"1\"))").is_err());

        assert!(run_test("DISPLAY(ASIN(1.1))").is_err());
        assert!(run_test("DISPLAY(ASIN(-1.1))").is_err());
        assert!(run_test("DISPLAY(ACOS(1.1))").is_err());
        assert!(run_test("DISPLAY(ACOS(-1.1))").is_err());

        assert!(run_test("DISPLAY(HYPOT(\"3\", 4))").is_err());
        assert!(run_test("DISPLAY(POW(TRUE, 2))").is_err());
        assert!(run_test("DISPLAY(ABS(TRUE))").is_err());
        assert!(run_test("DISPLAY(CEIL(TRUE))").is_err());
        assert!(run_test("DISPLAY(FLOOR(\"3.14\"))").is_err());

        assert!(run_test("DISPLAY(HYPOT(1))").is_err());
        assert!(run_test("DISPLAY(POW(2))").is_err());
        assert!(run_test("DISPLAY(GCD(1))").is_err());
        assert!(run_test("DISPLAY(GCD(1, 2, 3))").is_err());
    }

    #[test]
    fn test_misc() {
        assert_output(
            r#"
                    DISPLAYINLINE("Hello, ")
                    DISPLAYINLINE("World!")"#,
            "Hello, World!",
        );
    }
}
