use super::{assert_output, get_error, run_test};

// ---------------------------------------------------------------------------
// Basic error detection (existing tests, preserved)
// ---------------------------------------------------------------------------

#[test]
fn test_error_handling() {
    assert!(run_test("DISPLAY(5 / 0)").is_err());
    assert!(run_test("list <- [1, 2, 3]\nDISPLAY(list[4])").is_err());
    assert!(run_test("DISPLAY(undefined)").is_err());
    assert!(run_test("nonexistent(123)").is_err());
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
#[should_panic(expected = "List index out of bounds: 4 (size: 3)")]
fn test_list_index_out_of_bounds_high() {
    run_test("list <- [1, 2, 3]\nDISPLAY(list[4])").unwrap();
}

#[test]
#[should_panic(expected = "List index out of bounds: index cannot be less than 1")]
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

// ---------------------------------------------------------------------------
// Error format: line, column, source line, caret
// ---------------------------------------------------------------------------

#[test]
fn test_error_format_division_by_zero() {
    let err = get_error("x <- 10\ny <- 0\nz <- x / y");
    assert!(err.contains("Line 3"), "Should report line 3: {}", err);
    assert!(err.contains("Division by zero"), "{}", err);
    assert!(
        err.contains("z <- x / y"),
        "Should show source line: {}",
        err
    );
    assert!(err.contains("^"), "Should have caret: {}", err);
}

#[test]
fn test_error_format_undefined_variable() {
    let err = get_error("a <- 1\nb <- 2\nDISPLAY(c)");
    assert!(err.contains("Line 3"), "Should report line 3: {}", err);
    assert!(err.contains("Undefined variable: c"), "{}", err);
    assert!(
        err.contains("DISPLAY(c)"),
        "Should show source line: {}",
        err
    );
}

#[test]
fn test_error_format_list_out_of_bounds() {
    let err = get_error("myList <- [10, 20, 30]\nDISPLAY(myList[5])");
    assert!(err.contains("Line 2"), "Should report line 2: {}", err);
    assert!(
        err.contains("List index out of bounds: 5 (size: 3)"),
        "{}",
        err,
    );
    assert!(
        err.contains("DISPLAY(myList[5])"),
        "Should show source line: {}",
        err,
    );
}

#[test]
fn test_error_format_string_out_of_bounds() {
    let err = get_error("s <- \"hi\"\nDISPLAY(s[10])");
    assert!(err.contains("Line 2"), "Should report line 2: {}", err);
    assert!(err.contains("String index out of bounds"), "{}", err);
    assert!(
        err.contains("DISPLAY(s[10])"),
        "Should show source: {}",
        err
    );
}

#[test]
fn test_error_format_type_mismatch_if() {
    let err = get_error("x <- 42\nIF(x) {\n    DISPLAY(x)\n}");
    assert!(err.contains("Line 2"), "Should report line 2: {}", err);
    assert!(err.contains("Condition must be a boolean"), "{}", err);
    assert!(err.contains("IF(x) {"), "Should show source: {}", err);
}

#[test]
fn test_error_format_type_mismatch_binary_op() {
    let err = get_error("x <- \"hello\"\ny <- x + 5");
    assert!(err.contains("Line 2"), "Should report line 2: {}", err);
    assert!(err.contains("Invalid operation"), "{}", err);
    assert!(err.contains("y <- x + 5"), "Should show source: {}", err);
}

#[test]
fn test_error_format_modulo_by_zero() {
    let err = get_error("x <- 10 MOD 0");
    assert!(err.contains("Line 1"), "Should report line 1: {}", err);
    assert!(err.contains("Modulo by zero"), "{}", err);
}

#[test]
fn test_error_format_repeat_count_not_integer() {
    let err = get_error("REPEAT \"five\" TIMES {\n    DISPLAY(1)\n}");
    assert!(err.contains("REPEAT count must be an integer"), "{}", err);
}

#[test]
fn test_error_format_foreach_on_integer() {
    let err = get_error("FOR EACH item IN 42 {\n    DISPLAY(item)\n}");
    assert!(err.contains("FOR EACH requires list or string"), "{}", err);
}

#[test]
fn test_error_format_undefined_procedure() {
    let err = get_error("DISPLAY(notAProcedure(1, 2))");
    assert!(
        err.contains("Procedure 'notAProcedure' not found"),
        "{}",
        err,
    );
}

#[test]
fn test_error_format_tonum_invalid() {
    let err = get_error("x <- TONUM(\"abc\")");
    assert!(err.contains("Cannot convert string to number"), "{}", err);
    assert!(err.contains("Line 1"), "{}", err);
}

// ---------------------------------------------------------------------------
// Multi-line programs: correct line tracking across comments and blank lines
// ---------------------------------------------------------------------------

#[test]
fn test_error_line_with_comments() {
    let err =
        get_error("// setup\nx <- 10\n// middle comment\ny <- 20\n// last comment\nDISPLAY(z)");
    assert!(err.contains("Line 6"), "Should report line 6: {}", err);
    assert!(err.contains("Undefined variable: z"), "{}", err);
}

#[test]
fn test_error_line_with_blank_lines() {
    let err = get_error("x <- 1\n\n\ny <- 2\n\nDISPLAY(z)");
    assert!(err.contains("Line 6"), "Should report line 6: {}", err);
    assert!(err.contains("Undefined variable: z"), "{}", err);
}

#[test]
fn test_error_on_first_line() {
    let err = get_error("DISPLAY(badVar)");
    assert!(err.contains("Line 1"), "Should report line 1: {}", err);
    assert!(err.contains("Column"), "Should report column: {}", err);
}

#[test]
fn test_error_deep_in_program() {
    let input = (1..=9)
        .map(|i| format!("x{} <- {}", i, i))
        .collect::<Vec<_>>()
        .join("\n")
        + "\nDISPLAY(nonexistent)";
    let err = get_error(&input);
    assert!(err.contains("Line 10"), "Should report line 10: {}", err);
}

// ---------------------------------------------------------------------------
// Stack traces
// ---------------------------------------------------------------------------

#[test]
fn test_stack_trace_single_procedure() {
    let err = get_error("PROCEDURE foo(x) {\n    RETURN(x / 0)\n}\nDISPLAY(foo(5))");
    assert!(err.contains("Division by zero"), "{}", err);
    assert!(err.contains("in foo"), "Should show stack frame: {}", err);
}

#[test]
fn test_stack_trace_two_levels() {
    let err = get_error(
        "PROCEDURE inner(x) {\n    RETURN(x / 0)\n}\n\
         PROCEDURE outer(x) {\n    RETURN(inner(x))\n}\n\
         DISPLAY(outer(5))",
    );
    assert!(err.contains("Division by zero"), "{}", err);
    assert!(err.contains("in outer"), "Should show outer frame: {}", err);
    assert!(err.contains("in inner"), "Should show inner frame: {}", err);
}

#[test]
fn test_stack_trace_three_levels() {
    let err = get_error(
        "PROCEDURE a(x) {\n    RETURN(x / 0)\n}\n\
         PROCEDURE b(x) {\n    RETURN(a(x))\n}\n\
         PROCEDURE c(x) {\n    RETURN(b(x))\n}\n\
         DISPLAY(c(5))",
    );
    assert!(err.contains("Division by zero"), "{}", err);
    assert!(err.contains("in c"), "Should show c frame: {}", err);
    assert!(err.contains("in b"), "Should show b frame: {}", err);
    assert!(err.contains("in a"), "Should show a frame: {}", err);
}

#[test]
fn test_stack_trace_shows_call_line() {
    let err = get_error("PROCEDURE boom() {\n    x <- 1 / 0\n}\nboom()");
    assert!(err.contains("in boom"), "{}", err);
    assert!(
        err.contains("line"),
        "Should include line reference: {}",
        err
    );
}

#[test]
fn test_stack_trace_undefined_var_in_procedure() {
    let err = get_error("PROCEDURE greet() {\n    DISPLAY(name)\n}\ngreet()");
    assert!(err.contains("Undefined variable: name"), "{}", err);
    assert!(err.contains("in greet"), "Should show stack frame: {}", err);
}

#[test]
fn test_stack_trace_list_error_in_procedure() {
    let err = get_error(
        "PROCEDURE getItem(lst, idx) {\n    RETURN(lst[idx])\n}\n\
         myList <- [1, 2, 3]\nDISPLAY(getItem(myList, 10))",
    );
    assert!(err.contains("List index out of bounds"), "{}", err);
    assert!(
        err.contains("in getItem"),
        "Should show getItem frame: {}",
        err,
    );
}

#[test]
fn test_stack_trace_type_error_in_nested_call() {
    let err = get_error(
        "PROCEDURE addOne(x) {\n    RETURN(x + 1)\n}\n\
         PROCEDURE process(s) {\n    RETURN(addOne(s))\n}\n\
         DISPLAY(process(\"text\"))",
    );
    assert!(err.contains("Invalid operation"), "{}", err);
    assert!(
        err.contains("in process"),
        "Should show process frame: {}",
        err,
    );
    assert!(
        err.contains("in addOne"),
        "Should show addOne frame: {}",
        err,
    );
}

// ---------------------------------------------------------------------------
// Column accuracy
// ---------------------------------------------------------------------------

#[test]
fn test_error_column_at_start() {
    let err = get_error("undefinedFunc()");
    assert!(err.contains("Column 1"), "Should report column 1: {}", err);
}

#[test]
fn test_error_column_deep_in_expression() {
    let err = get_error("x <- 1 + 2 + 3 / 0");
    assert!(err.contains("Division by zero"), "{}", err);
    assert!(err.contains("^"), "Should have caret: {}", err);
}

#[test]
fn test_error_with_indented_code() {
    let err = get_error("IF(TRUE) {\n    x <- 1 / 0\n}");
    assert!(err.contains("Division by zero"), "{}", err);
    assert!(
        err.contains("x <- 1 / 0"),
        "Should show trimmed source: {}",
        err
    );
    assert!(err.contains("^"), "Should have caret: {}", err);
}

// ---------------------------------------------------------------------------
// Parser errors
// ---------------------------------------------------------------------------

#[test]
fn test_parser_error_missing_paren() {
    let err = get_error("IF(x > 5 {\n    DISPLAY(x)\n}");
    assert!(err.contains("Expected ')'"), "{}", err);
    assert!(err.contains("^"), "Should have caret: {}", err);
}

#[test]
fn test_parser_error_unexpected_token() {
    let err = get_error("DISPLAY(*)");
    assert!(!err.is_empty(), "Should produce an error");
    assert!(err.contains("^"), "Should have caret: {}", err);
}

// ---------------------------------------------------------------------------
// TRY/CATCH with new error format
// ---------------------------------------------------------------------------

#[test]
fn test_try_catch_error_message_only() {
    assert_output(
        "TRY {\n    x <- 1 / 0\n} CATCH (e) {\n    DISPLAY(e)\n}",
        "Division by zero",
    );
}

#[test]
fn test_try_catch_list_error_message() {
    assert_output(
        "TRY {\n    list <- [1]\n    DISPLAY(list[5])\n} CATCH (e) {\n    DISPLAY(e)\n}",
        "List index out of bounds: 5 (size: 1)",
    );
}

#[test]
fn test_try_catch_undefined_var_message() {
    assert_output(
        "TRY {\n    DISPLAY(noSuchVar)\n} CATCH (e) {\n    DISPLAY(e)\n}",
        "Undefined variable: noSuchVar",
    );
}

#[test]
fn test_try_catch_in_procedure() {
    assert_output(
        "PROCEDURE safe_div(a, b) {\n\
             TRY {\n\
                 RETURN(a / b)\n\
             } CATCH (e) {\n\
                 RETURN(e)\n\
             }\n\
         }\n\
         DISPLAY(safe_div(10, 0))",
        "Division by zero",
    );
}

#[test]
fn test_try_catch_does_not_catch_return() {
    assert_output(
        "PROCEDURE foo() {\n\
             TRY {\n\
                 RETURN(42)\n\
             } CATCH (e) {\n\
                 RETURN(-1)\n\
             }\n\
         }\n\
         DISPLAY(foo())",
        "42",
    );
}

// ---------------------------------------------------------------------------
// Edge cases and regression tests
// ---------------------------------------------------------------------------

#[test]
fn test_error_in_loop_body() {
    let err = get_error("REPEAT 3 TIMES {\n    x <- 1 / 0\n}");
    assert!(err.contains("Division by zero"), "{}", err);
    assert!(err.contains("Line 2"), "Should report line 2: {}", err);
}

#[test]
fn test_error_in_foreach_body() {
    let err = get_error("FOR EACH item IN [1, 2, 3] {\n    DISPLAY(item / 0)\n}");
    assert!(err.contains("Division by zero"), "{}", err);
}

#[test]
fn test_error_in_repeat_until_body() {
    let err = get_error("i <- 0\nREPEAT UNTIL(true) {\n    x <- 1 / 0\n}");
    assert!(err.contains("Division by zero"), "{}", err);
}

#[test]
fn test_error_in_else_branch() {
    let err = get_error("IF(FALSE) {\n    DISPLAY(1)\n} ELSE {\n    x <- 1 / 0\n}");
    assert!(err.contains("Division by zero"), "{}", err);
    assert!(err.contains("Line 4"), "Should report line 4: {}", err);
}

#[test]
fn test_error_in_nested_if() {
    let err = get_error("IF(TRUE) {\n    IF(TRUE) {\n        DISPLAY(noVar)\n    }\n}");
    assert!(err.contains("Undefined variable: noVar"), "{}", err);
    assert!(err.contains("Line 3"), "Should report line 3: {}", err);
}

#[test]
fn test_multiple_errors_first_wins() {
    let err = get_error("DISPLAY(bad1)\nDISPLAY(bad2)");
    assert!(err.contains("bad1"), "First error should win: {}", err,);
}

#[test]
fn test_error_format_has_all_parts() {
    let err = get_error("x <- 1\ny <- x / 0");
    let lines: Vec<&str> = err.lines().collect();
    assert!(
        lines.len() >= 3,
        "Error should have at least 3 lines (header, source, caret), got: {}",
        err,
    );
    assert!(
        lines[0].starts_with("Line "),
        "First line should start with 'Line ': {}",
        err,
    );
    assert!(
        lines[2].trim().contains('^'),
        "Third line should have caret: {}",
        err,
    );
}

#[test]
fn test_error_caret_alignment() {
    let err = get_error("DISPLAY(undefinedVar)");
    let lines: Vec<&str> = err.lines().collect();
    assert!(lines.len() >= 3, "Need at least 3 lines: {}", err);
    let source_line = lines[1];
    let caret_line = lines[2];
    let caret_pos = caret_line.find('^').unwrap();
    assert!(
        caret_pos >= 4,
        "Caret should be indented at least 4 spaces (indent prefix): {}",
        err,
    );
    let source_start = source_line.find(|c: char| !c.is_whitespace()).unwrap();
    assert!(
        caret_pos >= source_start,
        "Caret should be at or after source start: {}",
        err,
    );
}

#[test]
fn test_stack_overflow_detection() {
    std::thread::Builder::new()
        .stack_size(256 * 1024 * 1024)
        .name("test-stack_overflow".into())
        .spawn(|| {
            let err = get_error("PROCEDURE inf() {\n    RETURN(inf())\n}\ninf()");
            assert!(
                err.contains("Stack overflow") || err.contains("maximum recursion depth exceeded"),
                "Should detect infinite recursion: {}",
                err,
            );
        })
        .unwrap()
        .join()
        .unwrap();
}

#[test]
fn test_error_after_successful_output() {
    assert!(run_test("DISPLAY(1)\nDISPLAY(2)\nx <- 1/0").is_err());
}

#[test]
fn test_concat_type_error() {
    let err = get_error("DISPLAY(CONCAT(\"hello\", 5))");
    assert!(err.contains("CONCAT requires string arguments"), "{}", err);
}

#[test]
fn test_length_type_error() {
    let err = get_error("DISPLAY(LENGTH(42))");
    assert!(
        err.contains("LENGTH requires a list or string argument"),
        "{}",
        err,
    );
}

#[test]
fn test_sqrt_type_error() {
    let err = get_error("DISPLAY(SQRT(\"text\"))");
    assert!(err.contains("SQRT requires a numeric argument"), "{}", err);
}

#[test]
fn test_not_boolean_error() {
    let err = get_error("DISPLAY(NOT 5)");
    assert!(err.contains("Invalid unary operation"), "{}", err);
}

#[test]
fn test_class_decl_not_implemented() {
    let err = get_error("CLASS Foo\n{\nPROCEDURE bar()\n{\nDISPLAY(1)\n}\n}");
    assert!(err.contains("not yet implemented"), "{}", err);
}
