use super::{assert_output, get_error, run_test};

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
        let output =
            crate::interpreter::run_with_source(ast, input, &[]).expect("Interpreter error");
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

#[test]
fn test_string_concatenation_with_plus() {
    assert_output(r#"DISPLAY("hello" + " " + "world")"#, "hello world");
    assert_output(r#"DISPLAY("" + "abc")"#, "abc");
    assert_output(r#"DISPLAY("abc" + "")"#, "abc");
}

#[test]
fn test_empty_string_operations() {
    assert_output(r#"DISPLAY(LENGTH(""))"#, "0");
    assert_output(r#"DISPLAY(UPPERCASE(""))"#, "");
    assert_output(r#"DISPLAY(LOWERCASE(""))"#, "");
    assert_output(r#"DISPLAY(TRIM(""))"#, "");
}

#[test]
fn test_contains_edge_cases() {
    assert_output(r#"DISPLAY(CONTAINS("hello", ""))"#, "true");
    assert_output(r#"DISPLAY(CONTAINS("", "a"))"#, "false");
    assert_output(r#"DISPLAY(CONTAINS("", ""))"#, "true");
    assert_output(r#"DISPLAY(CONTAINS("abcabc", "abc"))"#, "true");
}

#[test]
fn test_find_edge_cases() {
    assert_output(r#"DISPLAY(FIND("hello", "xyz"))"#, "-1");
    assert_output(r#"DISPLAY(FIND("hello", "h"))"#, "1");
    assert_output(r#"DISPLAY(FIND("hello", "o"))"#, "5");
}

#[test]
fn test_split_edge_cases() {
    assert_output(r#"DISPLAY(SPLIT("a,b,c", ","))"#, "[a, b, c]");
    assert_output(r#"DISPLAY(SPLIT("hello", ","))"#, "[hello]");
}

#[test]
fn test_substring_boundary() {
    assert_output(r#"DISPLAY(SUBSTRING("abcde", 1, 5))"#, "abcde");
    assert_output(r#"DISPLAY(SUBSTRING("abcde", 3, 3))"#, "c");
    assert_output(r#"DISPLAY(SUBSTRING("abcde", 1, 1))"#, "a");
}

#[test]
fn test_startswith_endswith_edge() {
    assert_output(r#"DISPLAY(STARTSWITH("hello", "hello"))"#, "true");
    assert_output(r#"DISPLAY(ENDSWITH("hello", "hello"))"#, "true");
    assert_output(r#"DISPLAY(STARTSWITH("hello", ""))"#, "true");
    assert_output(r#"DISPLAY(ENDSWITH("hello", ""))"#, "true");
}

#[test]
fn test_length_on_string_vs_list() {
    assert_output(r#"DISPLAY(LENGTH("hello"))"#, "5");
    assert_output("DISPLAY(LENGTH([1, 2, 3]))", "3");
}

#[test]
fn test_formatted_string_expression() {
    assert_output(
        r#"
            x <- 5
            y <- 10
            DISPLAY(f"sum is {x + y}")
        "#,
        "sum is 15",
    );
}

#[test]
fn test_formatted_string_nested() {
    assert_output(
        r#"
            name <- "world"
            DISPLAY(f"hello {name}!")
        "#,
        "hello world!",
    );
}

#[test]
fn test_raw_string_no_escape() {
    assert_output(r#"DISPLAY(r"hello\nworld")"#, "hello\\nworld");
}

#[test]
fn test_replace_no_match() {
    assert_output(r#"DISPLAY(REPLACE("hello", "xyz", "abc"))"#, "hello");
}

// In-place string self-append (`s <- s + x`, `s <- CONCAT(s, x)`) is a fast
// path in the interpreter. These pin the semantics it has to keep.

#[test]
fn test_self_append_builds_the_same_string() {
    assert_output(
        r#"
            s <- ""
            REPEAT 5 TIMES
            {
                s <- s + "x"
            }
            DISPLAY(s)
            DISPLAY(LENGTH(s))
        "#,
        "xxxxx\n5",
    );
    assert_output(
        r#"
            s <- ""
            REPEAT 4 TIMES
            {
                s <- CONCAT(s, "ab")
            }
            DISPLAY(s)
        "#,
        "abababab",
    );
}

#[test]
fn test_self_append_leaves_the_copied_string_alone() {
    assert_output(
        r#"
            a <- "abc"
            b <- a
            b <- b + "d"
            b <- CONCAT(b, "e")
            DISPLAY(a)
            DISPLAY(b)
        "#,
        "abc\nabcde",
    );
}

#[test]
fn test_self_append_with_itself_and_with_a_call() {
    assert_output(
        r#"
            d <- "ab"
            REPEAT 3 TIMES
            {
                d <- d + d
            }
            DISPLAY(d)
        "#,
        "abababababababab",
    );
    assert_output(
        r#"
            PROCEDURE tag (v)
            {
                RETURN "<" + v + ">"
            }
            p <- "p"
            p <- p + tag(p)
            p <- CONCAT(p, tag("q"))
            DISPLAY(p)
        "#,
        "p<p><q>",
    );
}

#[test]
fn test_prepending_to_a_variable_still_works() {
    assert_output(
        r#"
            t <- "start-"
            u <- "end"
            u <- t + u
            DISPLAY(u)
        "#,
        "start-end",
    );
}

#[test]
fn test_self_append_inside_for_each() {
    assert_output(
        r#"
            h <- ""
            FOR EACH w IN ["a", "b", "c"]
            {
                h <- h + w
            }
            DISPLAY(h)
        "#,
        "abc",
    );
    assert_output(
        r#"
            it <- "abc"
            FOR EACH ch IN it
            {
                it <- it + ch
            }
            DISPLAY(it)
        "#,
        "abcabc",
    );
}

#[test]
fn test_self_append_keeps_strings_character_indexed() {
    assert_output(
        r#"
            u <- ""
            REPEAT 3 TIMES
            {
                u <- u + "é"
            }
            u <- CONCAT(u, "日本語")
            DISPLAY(LENGTH(u))
            DISPLAY(SUBSTRING(u, 4, 6))
            DISPLAY(u[4])
            DISPLAY(FIND(u, "日"))
        "#,
        "6\n日本語\n日\n4",
    );
}

#[test]
fn test_self_append_is_the_value_of_a_procedure_without_return() {
    assert_output(
        r#"
            PROCEDURE build (v)
            {
                acc <- "["
                acc <- acc + v
                acc <- acc + "]"
            }
            DISPLAY(build("mid"))
        "#,
        "[mid]",
    );
}

#[test]
fn test_appending_to_a_global_does_not_escape_the_procedure() {
    assert_output(
        r#"
            g <- "global"
            PROCEDURE touch ()
            {
                g <- g + "-local"
                DISPLAY(g)
            }
            touch()
            DISPLAY(g)
        "#,
        "global-local\nglobal",
    );
}

#[test]
fn test_self_add_on_non_strings_is_unchanged() {
    assert_output(
        r#"
            i <- 0
            REPEAT 4 TIMES
            {
                i <- i + 1
            }
            DISPLAY(i)
            l <- [1, 2]
            l2 <- l
            l <- l + [3]
            DISPLAY(l)
            DISPLAY(l2)
        "#,
        "4\n[1, 2, 3]\n[1, 2]",
    );
    assert!(
        get_error(
            r#"
            s <- "abc"
            REPEAT 2 TIMES
            {
                s <- s + 1
            }
        "#
        )
        .contains("Invalid operation")
    );
    assert!(
        get_error(
            r#"
            s <- "abc"
            REPEAT 2 TIMES
            {
                s <- CONCAT(s, 7)
            }
        "#
        )
        .contains("CONCAT requires string arguments")
    );
}
