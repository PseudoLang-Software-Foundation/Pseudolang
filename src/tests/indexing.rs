//! Indexing applied to expressions other than a bare variable.
//!
//! `name[i]` has always worked. What these cover is `[i]` after *any* primary
//! expression -- a procedure call, a literal, a parenthesised expression, a
//! built-in's result -- and the chaining of several such suffixes.

use super::{assert_output, get_error};

#[test]
fn test_index_a_procedure_call() {
    assert_output(
        r#"
        PROCEDURE pair()
        {
            RETURN [10, 20, 30]
        }
        DISPLAY(pair()[1])
        DISPLAY(pair()[3])
        "#,
        "10\n30",
    );
}

#[test]
fn test_chained_indexing_on_a_call() {
    assert_output(
        r#"
        PROCEDURE grid()
        {
            RETURN [[1, 2], [3, 4]]
        }
        DISPLAY(grid()[1][2])
        DISPLAY(grid()[2][1])
        "#,
        "2\n3",
    );
}

#[test]
fn test_index_a_builtin_result() {
    assert_output(
        r#"
        DISPLAY(SPLIT("a,b,c", ",")[2])
        DISPLAY(RANGE(5, 9)[3])
        DISPLAY(SORT([3, 1, 2])[1])
        "#,
        "b\n7\n1",
    );
}

#[test]
fn test_index_a_string_literal() {
    assert_output(r#"DISPLAY("hello"[1])"#, "h");
    assert_output(r#"DISPLAY("hello"[5])"#, "o");
}

#[test]
fn test_index_a_list_literal() {
    assert_output("DISPLAY([9, 8, 7][2])", "8");
    assert_output("DISPLAY([[1, 2], [3, 4]][2][1])", "3");
}

#[test]
fn test_index_a_dictionary_literal() {
    assert_output(r#"DISPLAY({"a": 1, "b": 2}["b"])"#, "2");
    assert_output(r#"DISPLAY({"a": [5, 6]}["a"][2])"#, "6");
}

#[test]
fn test_index_a_parenthesised_expression() {
    assert_output("xs <- [4, 5, 6]\nDISPLAY((xs)[2])", "5");
}

#[test]
fn test_indexing_composes_with_arithmetic() {
    assert_output(
        r#"
        PROCEDURE nums()
        {
            RETURN [10, 20]
        }
        DISPLAY(nums()[1] + nums()[2])
        i <- 2
        DISPLAY(nums()[i] * 2)
        "#,
        "30\n40",
    );
}

#[test]
fn test_indexed_call_result_can_be_assigned() {
    assert_output(
        r#"
        PROCEDURE grid()
        {
            RETURN [[1, 2], [3, 4]]
        }
        x <- grid()[2][2]
        DISPLAY(x)
        "#,
        "4",
    );
}

#[test]
fn test_indexing_a_call_in_a_condition_and_a_loop() {
    assert_output(
        r#"
        PROCEDURE flags()
        {
            RETURN [TRUE, FALSE]
        }
        IF flags()[1]
        {
            DISPLAY("first")
        }
        FOR EACH v IN [1, 2]
        {
            DISPLAY(flags()[v])
        }
        "#,
        "first\ntrue\nfalse",
    );
}

#[test]
fn test_a_list_literal_on_the_next_line_is_a_separate_statement() {
    // The postfix loop must stop at a newline, or the `[1, 2]` below would be
    // read as an index applied to the call above it.
    assert_output(
        r#"
        PROCEDURE one()
        {
            RETURN 1
        }
        DISPLAY(one())
        xs <- [1, 2]
        DISPLAY(LENGTH(xs))
        "#,
        "1\n2",
    );
}

#[test]
fn test_out_of_range_index_on_a_call_still_errors() {
    let err = get_error(
        r#"
        PROCEDURE pair()
        {
            RETURN [1, 2]
        }
        DISPLAY(pair()[5])
        "#,
    );
    assert!(err.to_lowercase().contains("index"), "{}", err);
}

#[test]
fn test_indexing_a_non_container_call_result_errors() {
    let err = get_error(
        r#"
        PROCEDURE num()
        {
            RETURN 7
        }
        DISPLAY(num()[1])
        "#,
    );
    assert!(!err.is_empty(), "expected an error indexing an integer");
}

#[test]
fn test_unclosed_index_is_reported() {
    let err = get_error("xs <- [1, 2]\nDISPLAY(xs[1)");
    assert!(err.contains("Expected ']'"), "{}", err);
}

#[test]
fn test_return_expressions_that_previously_returned_nothing() {
    // `is_expression_start` omitted these tokens, so RETURN silently handed back
    // the empty value instead of the expression's.
    assert_output(
        r#"
        PROCEDURE joined(a, b)
        {
            RETURN CONCAT(a, b)
        }
        PROCEDURE sliced(s)
        {
            RETURN SUBSTRING(s, 2, 4)
        }
        PROCEDURE nothing()
        {
            RETURN NULL
        }
        PROCEDURE formatted(v)
        {
            RETURN f"got {v}"
        }
        PROCEDURE raw()
        {
            RETURN r"a\nb"
        }
        PROCEDURE evaluated()
        {
            RETURN EVAL("2 * 3")
        }
        DISPLAY(joined("x", "y"))
        DISPLAY(sliced("abcdef"))
        DISPLAY(nothing())
        DISPLAY(formatted(3))
        DISPLAY(raw())
        DISPLAY(evaluated())
        "#,
        "xy\nbcd\nNULL\ngot 3\na\\nb\n6",
    );
}

#[test]
fn test_multiline_string_literals_parse() {
    // The lexer produced a MultilineString token that no parser arm consumed, so
    // every `"""..."""` was rejected outright.
    assert_output("x <- \"\"\"one\ntwo\"\"\"\nDISPLAY(x)", "one\ntwo");
    assert_output("DISPLAY(LENGTH(\"\"\"abc\"\"\"))", "3");
    assert_output(
        "PROCEDURE t()\n{\n    RETURN \"\"\"a\nb\"\"\"\n}\nDISPLAY(t())",
        "a\nb",
    );
}

#[test]
fn test_calling_an_indexed_value_is_refused() {
    // Two silent readings preceded this: first the indices were dropped and `a` was
    // called as a procedure, then the `(9)` became a separate statement evaluating to
    // nothing. Either way `handlers[1](arg)` looked like a call and did not call.
    let err = get_error(
        r#"
        a <- [1, 2]
        a[1](9)
        "#,
    );
    assert!(err.contains("Cannot call an indexed value"), "{}", err);
}

#[test]
fn test_a_trailing_comment_still_separates_statements() {
    // The lexer consumed the newline that ended a line comment, so the next line was
    // spliced onto the current expression and `[1, 2]` below was read as an index.
    for comment in ["# note", "// note", "COMMENT \"note\""] {
        let program = format!(
            r#"
            PROCEDURE f()
            {{
                RETURN [9, 8]
            }}
            z <- f() {}
            [1, 2]
            DISPLAY(z)
            "#,
            comment
        );
        assert_output(&program, "[9, 8]");
    }
}

#[test]
fn test_a_trailing_comment_does_not_join_two_statements() {
    assert_output(
        r#"
        a <- 1 # first
        b <- 2 // second
        DISPLAY(a + b)
        "#,
        "3",
    );
}

#[test]
fn test_a_formatted_string_interpolating_a_dictionary_access_needs_a_variable() {
    // Escaped quotes inside an interpolation are not unescaped before the slot is
    // re-lexed, so the value has to be bound first. Pinned because a documented
    // example got this wrong.
    assert_output(
        r#"
        d <- {"k": 7}
        v <- d["k"]
        DISPLAY(f"value {v}")
        "#,
        "value 7",
    );
    let err = get_error(
        r#"
        d <- {"k": 7}
        DISPLAY(f"value {d[\"k\"]}")
        "#,
    );
    assert!(!err.is_empty());
}
