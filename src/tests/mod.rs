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
    fn test_display() {
        assert_output("DISPLAY(42)", "42");
        assert_output("DISPLAY(TRUE)", "true");
        assert_output(r#"DISPLAY("Hello")"#, "Hello");
        assert_output("DISPLAY([1, 2, 3])", "[1, 2, 3]");
        assert_output("DISPLAY(5.5)", "5.5");
        assert_output("DISPLAY(-42)", "-42");
        assert_output("DISPLAY(FALSE)", "false");
        assert_output("DISPLAY([])", "[]");
        assert_output(
            r#"
                    DISPLAYINLINE("Hello, ")
                    DISPLAYINLINE("World!")"#,
            "Hello, World!",
        );
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
    fn test_string_split() {
        assert_output(
            r#"DISPLAY(SPLIT("Hello, World!", ","))"#,
            "[Hello,  World!]",
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
    fn test_escape_characters() {
        assert_output(r#"DISPLAY("Hello\tWorld")"#, "Hello\tWorld");

        assert_output(r#"DISPLAY("C:\\Program Files\\")"#, r"C:\Program Files\");

        assert_output(r#"DISPLAY("Line1\rLine2")"#, "Line1\rLine2");

        assert_output(r#"DISPLAY("ABC\bD")"#, "ABC\x08D");
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

        assert_output(
            r#"
            x <- 5
            y <- 10
            str1 <- "Hello, "
            str2 <- "world!"

            str <- f"{str1 + str2} {x} {y} {x + y}"
            "#,
            "Hello, world! 5 10 15",
        )
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
            DISPLAY(fibonacci(6))"#,
            "8",
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
            DISPLAY(x + y)
            DISPLAY(x * y)
            DISPLAY(x / y)
            "#,
            "8\n15\n1",
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
    fn test_merge_sort() {
        assert_output(
            r#"
            PROCEDURE merge(left, right) {
                result <- []
                leftIndex <- 1
                rightIndex <- 1
                
                REPEAT UNTIL(leftIndex > LENGTH(left) AND rightIndex > LENGTH(right)) {
                    IF(leftIndex > LENGTH(left)) {
                        APPEND(result, right[rightIndex])
                        rightIndex <- rightIndex + 1
                    } ELSE IF(rightIndex > LENGTH(right)) {
                        APPEND(result, left[leftIndex])
                        leftIndex <- leftIndex + 1
                    } ELSE IF(left[leftIndex] <= right[rightIndex]) {
                        APPEND(result, left[leftIndex])
                        leftIndex <- leftIndex + 1
                    } ELSE {
                        APPEND(result, right[rightIndex])
                        rightIndex <- rightIndex + 1
                    }
                }
                RETURN(result)
            }

            PROCEDURE mergeSort(arr) {
                IF(LENGTH(arr) <= 1) {
                    RETURN(arr)
                }
                
                mid <- LENGTH(arr) / 2
                left <- []
                right <- []
                
                i <- 1
                REPEAT mid TIMES {
                    APPEND(left, arr[i])
                    i <- i + 1
                }
                
                REPEAT LENGTH(arr) - mid TIMES {
                    APPEND(right, arr[i])
                    i <- i + 1
                }
                
                left <- mergeSort(left)
                right <- mergeSort(right)
                RETURN(merge(left, right))
            }

            arr <- [64, 34, 25, 12, 22, 11, 90]
            DISPLAY(mergeSort(arr))"#,
            "[11, 12, 22, 25, 34, 64, 90]",
        );
    }

    #[test]
    fn test_binary_search() {
        assert_output(
            r#"
            PROCEDURE binarySearch(arr, target) {
                left <- 1
                right <- LENGTH(arr)
                
                REPEAT UNTIL(left > right) {
                    mid <- (left + right) / 2
                    
                    IF(arr[mid] = target) {
                        RETURN(mid)
                    } ELSE IF(arr[mid] < target) {
                        left <- mid + 1
                    } ELSE {
                        right <- mid - 1
                    }
                }
                RETURN(-1)
            }

            arr <- [1, 2, 3, 4, 5, 6, 7, 8, 9, 10]
            DISPLAY(binarySearch(arr, 7))
            DISPLAY(binarySearch(arr, 11))"#,
            "7\n-1",
        );
    }

    #[test]
    fn test_quick_sort() {
        assert_output(
            r#"
            PROCEDURE partition(arr, low, high) {
    pivot <- arr[high]
    i <- low - 1

    j <- low
    REPEAT high - low TIMES {
        IF(arr[j] <= pivot) {
            i <- i + 1
            temp <- arr[i]
            arr[i] <- arr[j]
            arr[j] <- temp
        }
        j <- j + 1
    }

    temp <- arr[i + 1]
    arr[i + 1] <- arr[high]
    arr[high] <- temp
    RETURN([arr, i + 1])
}
PROCEDURE quickSort(arr, low, high) {
    IF(low < high) {
        partitionResult <- partition(arr, low, high)
        arr <- partitionResult[1]
        pi <- partitionResult[2]
        arr <- quickSort(arr, low, pi - 1)
        arr <- quickSort(arr, pi + 1, high)
    }
    RETURN(arr)
}

arr <- [64, 34, 25, 12, 22, 11, 90]
arr <- quickSort(arr, 1, LENGTH(arr))
DISPLAY(arr)"#,
            "[11, 12, 22, 25, 34, 64, 90]",
        );
    }

    #[test]
    fn test_insertion_sort() {
        assert_output(
            r#"
            PROCEDURE insertionSort(arr) {
                    i <- 2
                    REPEAT LENGTH(arr) - 1 TIMES {
                        key <- arr[i]
                        j <- i - 1
                        
                        IF(j >= 1 AND arr[j] > key) {
                            REPEAT UNTIL(j < 1 OR arr[j] <= key) {
                                arr[j + 1] <- arr[j]
                                j <- j - 1
                            }
                        }
                        
                        arr[j + 1] <- key
                        i <- i + 1
                    }
                    RETURN(arr)
                }

                arr <- [64, 34, 25, 12, 22, 11, 90]
                DISPLAY(insertionSort(arr))"#,
            "[11, 12, 22, 25, 34, 64, 90]",
        );
    }

    #[test]
    fn test_selection_sort() {
        assert_output(
            r#"
            PROCEDURE selectionSort(arr) {
                n <- LENGTH(arr)
                i <- 1
                
                REPEAT n - 1 TIMES {
                    minIdx <- i
                    j <- i + 1
                    
                    REPEAT n - i TIMES {
                        IF(arr[j] < arr[minIdx]) {
                            minIdx <- j
                        }
                        j <- j + 1
                    }
                    
                    IF(minIdx NOT= i) {
                        temp <- arr[minIdx]
                        arr[minIdx] <- arr[i]
                        arr[i] <- temp
                    }
                    i <- i + 1
                }
                RETURN(arr)
            }

            arr <- [64, 34, 25, 12, 22, 11, 90]
            DISPLAY(selectionSort(arr))"#,
            "[11, 12, 22, 25, 34, 64, 90]",
        );
    }

    #[test]
    fn test_linear_search() {
        assert_output(
            r#"
            PROCEDURE linearSearch(arr, target) {
                i <- 1
                REPEAT LENGTH(arr) TIMES {
                    IF(arr[i] = target) {
                        RETURN(i)
                    }
                    i <- i + 1
                }
                RETURN(-1)
            }

            arr <- [64, 34, 25, 12, 22, 11, 90]
            DISPLAY(linearSearch(arr, 22))
            DISPLAY(linearSearch(arr, 100))"#,
            "5\n-1",
        );
    }

    #[test]
    fn test_gcd_recursive() {
        assert_output(
            r#"
            PROCEDURE gcd(a, b) {
                IF(b = 0) {
                    RETURN(a)
                }
                RETURN(gcd(b, a MOD b))
            }
            
            DISPLAY(gcd(48, 18))
            DISPLAY(gcd(54, 24))
            DISPLAY(gcd(17, 5))"#,
            "6\n6\n1",
        );
    }

    #[test]
    fn test_calc_functions() {
        assert_output(
            r#"
            PROCEDURE DERIVATIVE(coefficients, exponents)
            {
                result_coeffs <- []
                result_exps <- []
                i <- 1
                
                REPEAT LENGTH(coefficients) TIMES
                {
                    IF (exponents[i] NOT= 0)
                    {
                        new_coeff <- coefficients[i] * exponents[i]
                        new_exp <- exponents[i] - 1
                        APPEND(result_coeffs, new_coeff)
                        APPEND(result_exps, new_exp)
                    }
                    i <- i + 1
                }
                RETURN([result_coeffs, result_exps])
            }

            PROCEDURE ANTIDERIVATIVE(coefficients, exponents)
            {
                result_coeffs <- []
                result_exps <- []
                i <- 1
                
                REPEAT LENGTH(coefficients) TIMES
                {
                    new_exp <- exponents[i] + 1
                    new_coeff <- coefficients[i] / new_exp
                    APPEND(result_coeffs, new_coeff)
                    APPEND(result_exps, new_exp)
                    i <- i + 1
                }
                APPEND(result_coeffs, 0)
                APPEND(result_exps, 0)
                RETURN([result_coeffs, result_exps])
            }

            coeffs <- [3, 2, 1]
            exps <- [2, 1, 0]

            deriv <- DERIVATIVE(coeffs, exps)
            DISPLAY("Derivative coefficients: " + TOSTRING(deriv[1]))
            DISPLAY("Derivative exponents: " + TOSTRING(deriv[2]))

            coeffs2 <- deriv[1]
            exps2 <- deriv[2]

            antideriv <- ANTIDERIVATIVE(coeffs2, exps2)
            DISPLAY("Antiderivative coefficients: " + TOSTRING(antideriv[1]))
            DISPLAY("Antiderivative exponents: " + TOSTRING(antideriv[2]))
        "#, "Derivative coefficients: [6, 2]\nDerivative exponents: [1, 0]\nAntiderivative coefficients: [3, 2, 0]\nAntiderivative exponents: [2, 1, 0]",
        );
    }

    #[test]
    fn test_heap_sort() {
        assert_output(
            r#"
        PROCEDURE heapify(arr, n, i) {
            largest <- i
            left <- 2 * i
            right <- 2 * i + 1

            IF(left <= n AND arr[left] > arr[largest]) {
                largest <- left
            }

            IF(right <= n AND arr[right] > arr[largest]) {
                largest <- right
            }

            IF(largest NOT= i) {
                temp <- arr[i]
                arr[i] <- arr[largest]
                arr[largest] <- temp

                arr <- heapify(arr, n, largest)
            }
            RETURN(arr)
        }

        PROCEDURE heapSort(arr) {
            n <- LENGTH(arr)
            i <- n / 2
            REPEAT UNTIL(i < 1) {
                arr <- heapify(arr, n, i)
                i <- i - 1
            }

            i <- n
            REPEAT UNTIL(i < 1) {
                temp <- arr[1]
                arr[1] <- arr[i]
                arr[i] <- temp

                arr <- heapify(arr, i - 1, 1)
                i <- i - 1
            }
            RETURN(arr)
        }

        arr <- [12, 11, 13, 5, 6, 7]
        arr <- heapSort(arr)
        DISPLAY(arr)
        "#,
            "[5, 6, 7, 11, 12, 13]",
        );
    }

    #[test]
    fn test_counting_sort() {
        assert_output(
            r#"
        PROCEDURE countingSort(arr, max_val) {
            count <- []
            i <- 1
            REPEAT (max_val + 1) TIMES {
                APPEND(count, 0)
                i <- i + 1
            }

            i <- 1
            REPEAT LENGTH(arr) TIMES {
                count[arr[i]] <- count[arr[i]] + 1
                i <- i + 1
            }

            i <- 2
            REPEAT max_val TIMES {
                count[i] <- count[i] + count[i - 1]
                i <- i + 1
            }

            output <- []
            i <- 1
            REPEAT LENGTH(arr) TIMES {
                APPEND(output, 0)
                i <- i + 1
            }

            i <- LENGTH(arr)
            REPEAT LENGTH(arr) TIMES {
                index <- count[arr[i]]
                output[index] <- arr[i]
                count[arr[i]] <- count[arr[i]] - 1
                i <- i - 1
            }
            RETURN(output)
        }

        arr <- [4, 2, 2, 8, 3, 3, 1]
        sorted <- countingSort(arr, 8)
        DISPLAY(sorted)
        "#,
            "[1, 2, 2, 3, 3, 4, 8]",
        );
    }

    #[test]
    fn test_kmp_string_matching() {
        assert_output(
            r#"
        PROCEDURE computeLPS(pattern) {
            lps <- []
            length <- 0
            i <- 1
            APPEND(lps, 0)

            REPEAT UNTIL(i >= LENGTH(pattern)) {
                IF(pattern[i + 1] = pattern[length + 1]) {
                    length <- length + 1
                    APPEND(lps, length)
                    i <- i + 1
                } ELSE {
                    IF(length NOT= 0) {
                        length <- lps[length]
                    } ELSE {
                        APPEND(lps, 0)
                        i <- i + 1
                    }
                }
            }
            RETURN(lps)
        }

        PROCEDURE kmpSearch(text, pattern) {
            lps <- computeLPS(pattern)
            i <- 1
            j <- 1
            positions <- []
            n <- LENGTH(text)
            m <- LENGTH(pattern)

            REPEAT UNTIL(i > n) {
                IF(pattern[j] = text[i]) {
                    i <- i + 1
                    j <- j + 1
                }

                IF(j > m) {
                    APPEND(positions, i - m)
                    j <- lps[j - 1] + 1
                } ELSE IF(i <= n AND pattern[j] NOT= text[i]) {
                    IF(j NOT= 1) {
                        j <- lps[j - 1] + 1
                    } ELSE {
                        i <- i + 1
                    }
                }
            }
            RETURN(positions)
        }

        text <- "ABABDABACDABABCABAB"
        pattern <- "ABABCABAB"
        positions <- kmpSearch(text, pattern)
        DISPLAY(positions)
        "#,
            "[11]",
        );
    }

    #[test]
    fn test_min_max_functions() {
        assert_output(
            r#"
            PROCEDURE test_min_max() {
                a <- MIN(5, 10)
                b <- MIN(10, 5)
                c <- MAX(5, 10)
                d <- MAX(10, 5)
                DISPLAY(a)
                DISPLAY(b)
                DISPLAY(c)
                DISPLAY(d)
            }
            
            test_min_max()
            "#,
            "5\n5\n10\n10",
        );
    }

    #[test]
    fn test_string_comparisons() {
        assert_output(
            r#"
            a <- "hello"
            b <- "hello"
            IF(a = b) {
                DISPLAY("equal")
            } ELSE {
                DISPLAY("not equal")
            }
            "#,
            "equal",
        );

        assert_output(
            r#"
            PROCEDURE compareStrings(s1, s2) {
                RETURN(s1 = s2)
            }
            DISPLAY(compareStrings("hello", "hello"))
            DISPLAY(compareStrings("hello", "world"))
            "#,
            "true\nfalse",
        );

        assert_output(
            r#"
            a <- "asd"
            b <- "asd"
            c <- (a = b)
            DISPLAY(c)
            "#,
            "true",
        );

        assert_output(
            r#"
            a <- "asd"
            b <- "asd"
            DISPLAY(a = b)
            DISPLAY((a = b))
            "#,
            "true\ntrue",
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
    fn test_multidimensional_arrays() {
        assert_output(
            r#"
            matrix <- [[1, 2, 3], [4, 5, 6], [7, 8, 9]]
            DISPLAY(matrix[2][3])
            "#,
            "6",
        );

        assert_output(
            r#"
            mixed_matrix <- [[1, "two", TRUE], [4.5, FALSE, "six"]]
            DISPLAY(mixed_matrix[1][2])
            DISPLAY(mixed_matrix[2][1])
            "#,
            "two\n4.5",
        );

        assert_output(
            r#"
            empty_matrix <- [[""]]
            empty_matrix[1][1] <- "filled"
            DISPLAY(empty_matrix[1][1])
            "#,
            "filled",
        );
    }

    #[test]
    fn test_three_dimensional_arrays() {
        assert_output(
            r#"
            cube <- [[[1, 2], [3, 4]], [[5, 6], [7, 8]]]
            DISPLAY(cube[1][2][1])
            DISPLAY(cube[2][1][2])
            "#,
            "3\n6",
        );
    }

    #[test]
    fn test_2d_linear_search() {
        assert_output(
            r#"
            PROCEDURE linearSearch2D(matrix, target) {
                rows <- LENGTH(matrix)
                columns <- LENGTH(matrix[1])
                
                i <- 1
                REPEAT rows TIMES {
                    j <- 1
                    REPEAT columns TIMES {
                        IF(matrix[i][j] = target) {
                            RETURN([i, j])
                        }
                        j <- j + 1
                    }
                    i <- i + 1
                }
                RETURN([-1, -1])
            }

            matrix <- [[1, 2, 3], [4, 5, 6], [7, 8, 9]]
            result <- linearSearch2D(matrix, 5)
            DISPLAY(result)
            result <- linearSearch2D(matrix, 10)
            DISPLAY(result)
            "#,
            "[2, 2]\n[-1, -1]",
        );
    }

    #[test]
    fn test_matrix_operations() {
        assert_output(
            r#"
            matrix <- [[1, 2, 3], [4, 5, 6]]
            matrix[1][3] <- 10
            DISPLAY(matrix[1][3])
            
            matrix[2] <- [7, 8, 9]
            DISPLAY(matrix[2][2])
            "#,
            "10\n8",
        );

        assert_output(
            r#"
            matrix <- [[1, "two"], [TRUE, 4.5]]
            DISPLAY(matrix[1][2])
            DISPLAY(matrix[2][1])
            DISPLAY(matrix[2][2])
            "#,
            "two\ntrue\n4.5",
        );
    }

    #[test]
    fn test_ml_algorithms() {
        assert_output(
            r#"
            PROCEDURE calculateDistance(point1, point2)
            {
                sum <- 0
                i <- 1
                REPEAT LENGTH(point1) TIMES
                {
                    diff <- point1[i] - point2[i]
                    sum <- sum + POW(diff, 2)
                    i <- i + 1
                }
                RETURN(SQRT(sum))
            }

            PROCEDURE findKNearest(trainingData, trainingLabels, testPoint, k)
            {
                distances <- []
                labels <- []
                
                i <- 1
                REPEAT LENGTH(trainingData) TIMES
                {
                    distance <- calculateDistance(testPoint, trainingData[i])
                    APPEND(distances, distance)
                    APPEND(labels, trainingLabels[i])
                    i <- i + 1
                }
                
                sortedIndices <- []
                i <- 1
                REPEAT LENGTH(distances) TIMES
                {
                    minIndex <- 1
                    j <- 1
                    REPEAT LENGTH(distances) TIMES
                    {
                        IF(distances[j] < distances[minIndex])
                        {
                            minIndex <- j
                        }
                        j <- j + 1
                    }
                    APPEND(sortedIndices, minIndex)
                    distances[minIndex] <- 999999999
                    i <- i + 1
                }
                
                kNearest <- []
                i <- 1
                REPEAT k TIMES
                {
                    APPEND(kNearest, labels[sortedIndices[i]])
                    i <- i + 1
                }
                
                RETURN(kNearest)
            }

            PROCEDURE getMajorityVote(labels)
            {
                counts <- []
                uniqueLabels <- []
                
                FOR EACH label IN labels
                {
                    found <- FALSE
                    i <- 1
                    REPEAT LENGTH(uniqueLabels) TIMES
                    {
                        IF(label = uniqueLabels[i])
                        {
                            counts[i] <- counts[i] + 1
                            found <- TRUE
                        }
                        i <- i + 1
                    }
                    IF(NOT found)
                    {
                        APPEND(uniqueLabels, label)
                        APPEND(counts, 1)
                    }
                }
                
                maxCount <- 0
                maxLabel <- uniqueLabels[1]
                i <- 1
                REPEAT LENGTH(counts) TIMES
                {
                    IF(counts[i] > maxCount)
                    {
                        maxCount <- counts[i]
                        maxLabel <- uniqueLabels[i]
                    }
                    i <- i + 1
                }
                
                RETURN(maxLabel)
            }

            PROCEDURE knn(trainingData, trainingLabels, testPoint, k)
            {
                nearestLabels <- findKNearest(trainingData, trainingLabels, testPoint, k)
                prediction <- getMajorityVote(nearestLabels)
                RETURN(prediction)
            }

            trainingData <- [[1,2], [2,3], [3,1], [6,5], [7,8], [8,7]]
            trainingLabels <- [1, 1, 1, 2, 2, 2]
            testPoint <- [5,5]
            k <- 3

            prediction <- knn(trainingData, trainingLabels, testPoint, k)
            DISPLAY(prediction)
            "#,
            "2",
        );

        assert_output(
            r#"
        PROCEDURE linearRegression(x, y) {
            n <- LENGTH(x)
            sumX <- 0
            sumY <- 0
            sumXY <- 0 
            sumXSquare <- 0
            
            i <- 1
            REPEAT n TIMES {
                sumX <- sumX + x[i]
                sumY <- sumY + y[i]
                sumXY <- sumXY + x[i] * y[i]
                sumXSquare <- sumXSquare + x[i] * x[i]
                i <- i + 1
            }
            
            slope <- (n * sumXY - sumX * sumY) / (n * sumXSquare - sumX * sumX)
            intercept <- (sumY - slope * sumX) / n
            
            RETURN([slope, intercept])
        }

        PROCEDURE predict(x, coefficients) {
            slope <- coefficients[1]
            intercept <- coefficients[2]
            RETURN(slope * x + intercept)
        }

        x <- [1, 2, 3, 4, 5]
        y <- [2, 4, 6, 8, 10]
        
        coefficients <- linearRegression(x, y)
        prediction <- predict(6, coefficients)
        DISPLAY(FLOOR(prediction))
        "#,
            "12",
        );
    }

    #[test]
    fn test_timestamp_functions() {
        let result = run_test("DISPLAY(TIMESTAMP())").unwrap();
        let timestamp = result.trim().parse::<i64>().unwrap();
        assert!(timestamp > 0);

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
            DISPLAY(TIMESTAMP(dt))
            "#,
            "1625329272",
        );

        let roundtrip = run_test(
            r#"
            ts <- TIMESTAMP()
            t <- TIME(ts)
            ts2 <- TIMESTAMP(t)
            DISPLAY(ts = ts2)
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

        assert!(run_test(
            r#"
            ts <- TIMESTAMP()
            DISPLAY(TIMEZONE(ts, "Invalid/Zone"))
        "#
        )
        .is_err());
    }

    #[test]
    fn test_try_catch() {
        assert_output(
            r#"
            TRY {
                DISPLAY("Before error")
                x <- 1 / 0
                DISPLAY("After error")
            } CATCH (err) {
                DISPLAY("Caught error: " + err)
            }
            "#,
            "Before error\nCaught error: Division by zero",
        );

        assert_output(
            r#"
            TRY {
                DISPLAY("No error here")
                x <- 42
            } CATCH (err) {
                DISPLAY("This won't run")
            }
            "#,
            "No error here",
        );

        assert_output(
            r#"
            TRY {
                list <- [1, 2, 3]
                DISPLAY(list[4])
            } CATCH (err) {
                DISPLAY("List error: " + err)
            }
            "#,
            "List error: List index out of bounds: 4 (size: 3)",
        );
    }

    #[test]
    fn test_trim() {
        assert_output(
            r#"
            str <- "  hello  "
            trimmed <- TRIM(str)
            DISPLAY(trimmed)
            "#,
            "hello",
        );

        assert_output(r#"DISPLAY(TRIM("  spaces  "))"#, "spaces");

        assert_output(r#"DISPLAY(TRIM("\t\ntabs\t\n"))"#, "tabs");
    }

    #[test]
    fn test_replace() {
        assert_output(
            r#"
            str <- "hello world"
            result <- REPLACE(str, "o", "0")
            DISPLAY(result)
            "#,
            "hell0 w0rld",
        );

        assert_output(r#"DISPLAY(REPLACE("hello hello", "hello", "hi"))"#, "hi hi");

        assert_output(r#"DISPLAY(REPLACE("aaa", "a", "b"))"#, "bbb");
    }

    #[test]
    fn test_case_conversion() {
        assert_output(
            r#"
            str <- "Hello World"
            upper <- UPPERCASE(str)
            lower <- LOWERCASE(str)
            DISPLAY(upper)
            DISPLAY(lower)
            "#,
            "HELLO WORLD\nhello world",
        );

        assert_output(r#"DISPLAY(UPPERCASE("abc123"))"#, "ABC123");

        assert_output(r#"DISPLAY(LOWERCASE("ABC123"))"#, "abc123");
    }

    #[test]
    fn test_string_functions_error_handling() {
        assert!(run_test("TRIM(123)").is_err());
        assert!(run_test("REPLACE(123, 'a', 'b')").is_err());
        assert!(run_test("UPPERCASE(123)").is_err());
        assert!(run_test("LOWERCASE(123)").is_err());

        assert!(run_test(r#"REPLACE("hello", 123, "a")"#).is_err());
        assert!(run_test(r#"REPLACE("hello", "a", 123)"#).is_err());
    }

    #[test]
    fn test_string_function_combinations() {
        assert_output(
            r#"
            str <- "  HELLO world  "
            result <- TRIM(LOWERCASE(str))
            DISPLAY(result)
            "#,
            "hello world",
        );

        assert_output(
            r#"
            str <- "hello WORLD"
            result <- REPLACE(UPPERCASE(str), "L", "1")
            DISPLAY(result)
            "#,
            "HE11O WOR1D",
        );
    }

    #[test]
    fn test_string_search() {
        assert_output(
            r#"
        DISPLAY(CONTAINS("Hello World", "World"))
        DISPLAY(CONTAINS("Hello World", "Goodbye"))
        DISPLAY(CONTAINS("Hello", "ell"))
        DISPLAY(CONTAINS("", ""))
        "#,
            "true\nfalse\ntrue\ntrue",
        );

        assert_output(
            r#"
        DISPLAY(FIND("Hello World", "World"))
        DISPLAY(FIND("Hello World", "Goodbye"))
        DISPLAY(FIND("Hello", "ell"))
        DISPLAY(FIND("Testing", "t"))
        "#,
            "7\n-1\n2\n4",
        );

        assert_output(
            r#"
        text <- "The quick brown fox"
        needle <- "quick"
        DISPLAY(CONTAINS(text, needle))
        DISPLAY(FIND(text, needle))
        "#,
            "true\n5",
        );
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
    fn test_null_and_nan() {
        assert_output("DISPLAY(NULL)", "NULL");
        assert_output("DISPLAY(NaN)", "NaN");

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
            x <- NaN
            y <- NaN
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
            x <- NaN
            y <- 42
            DISPLAY(x = y)
            "#,
            "false",
        );

        assert_output(
            r#"
            x <- NaN
            y <- x + 5
            DISPLAY(y)
            "#,
            "NaN",
        );
    }
}
