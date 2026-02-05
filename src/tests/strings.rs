use super::{assert_output, run_test};

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
        let ast = crate::parser::parse_with_source(
            crate::lexer::Lexer::new(input).tokenize(),
            input,
            false,
        )
        .expect("Failed to parse");
        let output = crate::interpreter::run_with_source(ast, input).expect("Interpreter error");
        assert_eq!(output, expected_output, "Test failed for input '{}'", input);
    }
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
fn test_string_split() {
    assert_output(
        r#"DISPLAY(SPLIT("Hello, World!", ","))"#,
        "[Hello,  World!]",
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
fn test_string_prefix_suffix() {
    assert_output(
        r#"
        DISPLAY(STARTSWITH("Hello World", "Hello"))
        DISPLAY(STARTSWITH("Hello World", "World"))
        DISPLAY(STARTSWITH("", ""))
        DISPLAY(STARTSWITH("Hello", "HelloWorld"))
        DISPLAY(STARTSWITH("testing", "test"))
        "#,
        "true\nfalse\ntrue\nfalse\ntrue",
    );

    assert_output(
        r#"
        DISPLAY(ENDSWITH("Hello World", "World"))
        DISPLAY(ENDSWITH("Hello World", "Hello"))
        DISPLAY(ENDSWITH("", ""))
        DISPLAY(ENDSWITH("World", "WorldLong"))
        DISPLAY(ENDSWITH("testing", "ing"))
        "#,
        "true\nfalse\ntrue\nfalse\ntrue",
    );

    assert_output(
        r#"
        text <- "Hello World"
        start <- "Hello"
        end <- "World"
        DISPLAY(STARTSWITH(text, start))
        DISPLAY(ENDSWITH(text, end))
        "#,
        "true\ntrue",
    );

    assert!(run_test(r#"STARTSWITH(123, "abc")"#).is_err());
    assert!(run_test(r#"STARTSWITH("abc", 123)"#).is_err());
    assert!(run_test(r#"ENDSWITH(123, "abc")"#).is_err());
    assert!(run_test(r#"ENDSWITH("abc", 123)"#).is_err());
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
fn test_string_functions_error_handling() {
    assert!(run_test("TRIM(123)").is_err());
    assert!(run_test("REPLACE(123, 'a', 'b')").is_err());
    assert!(run_test("UPPERCASE(123)").is_err());
    assert!(run_test("LOWERCASE(123)").is_err());

    assert!(run_test(r#"REPLACE("hello", 123, "a")"#).is_err());
    assert!(run_test(r#"REPLACE("hello", "a", 123)"#).is_err());
}

#[test]
fn test_escape_characters() {
    assert_output(r#"DISPLAY("Hello\tWorld")"#, "Hello\tWorld");

    assert_output(r#"DISPLAY("C:\\Program Files\\")"#, r"C:\Program Files\");

    assert_output(r#"DISPLAY("Line1\rLine2")"#, "Line1\rLine2");

    assert_output(r#"DISPLAY("ABC\bD")"#, "ABC\x08D");
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
