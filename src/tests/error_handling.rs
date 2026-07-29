use super::{assert_output, get_error, run_test};

// ---------------------------------------------------------------------------
// Basic error detection
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
    assert!(
        err.contains("FOR EACH requires list, string, or dictionary"),
        "{}",
        err
    );
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
        err.contains("LENGTH requires a list, string, or dictionary argument"),
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

// ---------------------------------------------------------------------------
// Dictionary errors
// ---------------------------------------------------------------------------

#[test]
fn test_dict_missing_key_read_error() {
    let err = get_error("d <- {\"a\": 1}\nDISPLAY(d[\"b\"])");
    assert!(err.contains("Key not found: b"), "{}", err);
    assert!(err.contains("Line 2"), "Should report line 2: {}", err);
}

#[test]
fn test_dict_illegal_key_type_error() {
    let err = get_error("d <- {2.5: \"x\"}");
    assert!(
        err.contains("Dictionary keys must be strings, integers, or booleans"),
        "{}",
        err,
    );
}

#[test]
fn test_dict_illegal_key_type_on_lookup_error() {
    let err = get_error("d <- {\"a\": 1}\nDISPLAY(d[[1, 2]])");
    assert!(
        err.contains("Dictionary keys must be strings, integers, or booleans"),
        "{}",
        err,
    );
}

#[test]
fn test_dict_literal_missing_colon_error() {
    let err = get_error("d <- {\"a\" 1}");
    assert!(err.contains("Expected ':' after dictionary key"), "{}", err);
}

#[test]
fn test_dict_literal_missing_comma_error() {
    let err = get_error("d <- {\"a\": 1 \"b\": 2}");
    assert!(
        err.contains("Expected comma between dictionary entries"),
        "{}",
        err,
    );
}

#[test]
fn test_dict_literal_at_statement_position_error() {
    let err = get_error("{\"a\": 1}");
    assert!(err.contains("Unexpected token in statement"), "{}", err);
}

#[test]
fn test_keys_type_error() {
    let err = get_error("DISPLAY(KEYS(42))");
    assert!(
        err.contains("KEYS requires a dictionary argument"),
        "{}",
        err
    );
}

#[test]
fn test_values_type_error() {
    let err = get_error("DISPLAY(VALUES([1, 2]))");
    assert!(
        err.contains("VALUES requires a dictionary argument"),
        "{}",
        err,
    );
}

#[test]
fn test_haskey_type_error() {
    let err = get_error("DISPLAY(HASKEY([1, 2], \"a\"))");
    assert!(
        err.contains("HASKEY requires a dictionary argument"),
        "{}",
        err,
    );
}

#[test]
fn test_getkey_missing_key_without_default_error() {
    let err = get_error("d <- {\"a\": 1}\nDISPLAY(GETKEY(d, \"b\"))");
    assert!(err.contains("Key not found: b"), "{}", err);
}

#[test]
fn test_getkey_arity_error() {
    let err = get_error("d <- {\"a\": 1}\nDISPLAY(GETKEY(d))");
    assert!(err.contains("GETKEY requires 2 or 3 arguments"), "{}", err);
}

#[test]
fn test_setkey_on_non_dictionary_error() {
    let err = get_error("x <- 5\nSETKEY(x, \"a\", 1)");
    assert!(err.contains("Variable x is not a dictionary"), "{}", err);
}

#[test]
fn test_setkey_requires_variable_error() {
    let err = get_error("DISPLAY(SETKEY({\"a\": 1}, \"b\", 2))");
    assert!(
        err.contains("SETKEY requires a dictionary variable"),
        "{}",
        err,
    );
}

#[test]
fn test_removekey_missing_key_error() {
    let err = get_error("d <- {\"a\": 1}\nREMOVEKEY(d, \"b\")");
    assert!(err.contains("Key not found: b"), "{}", err);
}

#[test]
fn test_removekey_requires_variable_error() {
    let err = get_error("REMOVEKEY([1, 2], 1)");
    assert!(
        err.contains("REMOVEKEY requires a dictionary variable"),
        "{}",
        err,
    );
}

#[test]
fn test_dictionary_builtin_takes_no_arguments_error() {
    let err = get_error("d <- DICTIONARY(1)");
    assert!(err.contains("DICTIONARY takes no arguments"), "{}", err);
}

#[test]
fn test_dict_index_assignment_on_scalar_error() {
    let err = get_error("x <- 5\nx[\"a\"] <- 1");
    assert!(
        err.contains("Variable x is not a list or dictionary"),
        "{}",
        err,
    );
}

#[test]
fn test_dict_ordering_comparison_error() {
    let err = get_error("a <- {\"x\": 1}\nb <- {\"x\": 2}\nDISPLAY(a < b)");
    assert!(err.contains("Invalid operation"), "{}", err);
}

#[test]
fn test_dict_plus_non_dictionary_error() {
    let err = get_error("DISPLAY({\"a\": 1} + \"text\")");
    assert!(err.contains("Invalid operation"), "{}", err);
}

#[test]
fn test_foreach_on_dictionary_is_allowed() {
    assert_output(
        "d <- {\"a\": 1, \"b\": 2}\nFOR EACH k IN d {\n    DISPLAY(k)\n}",
        "a\nb",
    );
}

#[test]
fn test_length_on_dictionary_is_allowed() {
    assert_output("DISPLAY(LENGTH({\"a\": 1, \"b\": 2}))", "2");
}

// ---------------------------------------------------------------------------
// Boolean operators type-check both operands
// ---------------------------------------------------------------------------

/// AND/OR used to inspect only the RIGHT operand's type. A non-boolean on the
/// left silently took the "not false" / "not true" branch, so `1 AND TRUE`
/// evaluated to true instead of being rejected.
#[test]
fn test_and_left_operand_must_be_boolean() {
    for source in [
        "DISPLAY(1 AND TRUE)",
        "DISPLAY(1 AND FALSE)",
        r#"DISPLAY("x" AND TRUE)"#,
        "DISPLAY(0 AND TRUE)",
        "DISPLAY([1] AND TRUE)",
        "DISPLAY(1.5 AND TRUE)",
    ] {
        let err = get_error(source);
        assert!(
            err.contains("Left operand of AND must be boolean"),
            "{source}: {err}"
        );
    }
}

#[test]
fn test_or_left_operand_must_be_boolean() {
    for source in [
        "DISPLAY(1 OR TRUE)",
        "DISPLAY(1 OR FALSE)",
        r#"DISPLAY("x" OR FALSE)"#,
        "DISPLAY(0 OR FALSE)",
        "DISPLAY([1] OR FALSE)",
    ] {
        let err = get_error(source);
        assert!(
            err.contains("Left operand of OR must be boolean"),
            "{source}: {err}"
        );
    }
}

#[test]
fn test_and_or_right_operand_still_checked() {
    let err = get_error("DISPLAY(TRUE AND 1)");
    assert!(
        err.contains("Right operand of AND must be boolean"),
        "{err}"
    );
    let err = get_error("DISPLAY(FALSE OR 1)");
    assert!(err.contains("Right operand of OR must be boolean"), "{err}");
}

#[test]
fn test_and_or_still_short_circuit() {
    // FALSE AND <error> and TRUE OR <error> must not evaluate the right side.
    assert_output("DISPLAY(FALSE AND (1 / 0 = 0))", "false");
    assert_output("DISPLAY(TRUE OR (1 / 0 = 0))", "true");
    assert_output("DISPLAY(FALSE AND 1)", "false");
    assert_output("DISPLAY(TRUE OR 1)", "true");
}

// ---------------------------------------------------------------------------
// Runtime error messages on the string paths (previously unasserted)
// ---------------------------------------------------------------------------

#[test]
fn test_substring_argument_errors() {
    let err = get_error("DISPLAY(SUBSTRING(5, 1, 2))");
    assert!(err.contains("Invalid substring arguments"), "{err}");
    let err = get_error(r#"DISPLAY(SUBSTRING("abc", "1", 2))"#);
    assert!(err.contains("Invalid substring arguments"), "{err}");
}

#[test]
fn test_find_argument_errors() {
    let err = get_error(r#"DISPLAY(FIND("abc", 1))"#);
    assert!(err.contains("FIND requires two string arguments"), "{err}");
    let err = get_error(r#"DISPLAY(FIND("abc"))"#);
    assert!(err.contains("FIND requires two arguments"), "{err}");
}

#[test]
fn test_concat_argument_errors() {
    let err = get_error(r#"DISPLAY(CONCAT("abc", 1))"#);
    assert!(err.contains("CONCAT requires string arguments"), "{err}");
}

#[test]
fn test_length_argument_errors() {
    let err = get_error("DISPLAY(LENGTH(5))");
    assert!(
        err.contains("LENGTH requires a list, string, or dictionary argument"),
        "{err}"
    );
}

#[test]
fn test_string_index_below_one_error() {
    let err = get_error("s <- \"abc\"\nDISPLAY(s[0])");
    assert!(
        err.contains("String index out of bounds: index cannot be less than 1"),
        "{err}"
    );
}

// ---------------------------------------------------------------------------
// Output produced inside a CATCH block
// ---------------------------------------------------------------------------

/// A CATCH block that DISPLAYs and then RETURNs used to have its output
/// silently dropped in capture mode: the RETURN propagated out of the TRY/CATCH
/// arm before the arm could copy the catch scope's private output buffer up
/// into its parent, so "caught" was lost and only "after" came back. (The CLI
/// never showed the bug, because DISPLAY also printed directly.) Every scope
/// now shares one output sink, so there is no copy-up left to skip.
#[test]
fn test_catch_that_displays_then_returns_keeps_its_output() {
    assert_output(
        "PROCEDURE f() {\n\
         \x20   TRY {\n\
         \x20       DISPLAY(nope)\n\
         \x20   } CATCH (e) {\n\
         \x20       DISPLAY(\"caught\")\n\
         \x20       RETURN (1)\n\
         \x20   }\n\
         }\n\
         f()\n\
         DISPLAY(\"after\")",
        "caught\nafter",
    );
}

/// The same loss one level further out: a CATCH that DISPLAYs and RETURNs from
/// the top level of the program.
#[test]
fn test_top_level_catch_that_displays_then_returns_keeps_its_output() {
    assert_output(
        "DISPLAY(\"before\")\n\
         TRY {\n\
         \x20   DISPLAY(nope)\n\
         } CATCH (e) {\n\
         \x20   DISPLAY(\"caught\")\n\
         \x20   RETURN (1)\n\
         }",
        "before\ncaught",
    );
}

/// Output written by a procedure that RETURNs from inside a CATCH nested in a
/// loop still lands in order relative to the caller's own output.
#[test]
fn test_catch_output_ordering_across_procedure_boundary() {
    assert_output(
        "PROCEDURE f(i) {\n\
         \x20   TRY {\n\
         \x20       DISPLAY(nope)\n\
         \x20   } CATCH (e) {\n\
         \x20       DISPLAY(\"c\" + TOSTRING(i))\n\
         \x20       RETURN (i)\n\
         \x20   }\n\
         }\n\
         DISPLAY(\"start\")\n\
         FOR EACH i IN [1, 2, 3] {\n\
         \x20   DISPLAY(\"a\" + TOSTRING(i))\n\
         \x20   f(i)\n\
         }\n\
         DISPLAY(\"end\")",
        "start\na1\nc1\na2\nc2\na3\nc3\nend",
    );
}
