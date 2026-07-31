//! Parser- and lexer-level invariants.

use super::{assert_output, get_error};
use crate::lexer::{Lexer, Token};
use crate::parser::MAX_NESTING_DEPTH;

/// Build `DISPLAY((((...1...))))` with `n` layers of redundant parentheses.
fn nested_parens(n: usize) -> String {
    format!("DISPLAY({}1{})", "(".repeat(n), ")".repeat(n))
}

/// Parsing is recursive descent, so a deeply nested source uses a lot of stack.
/// Run these on a thread with a generous stack: the point of the test is what
/// the *limit* does, not how much stack a debug build happens to burn per
/// frame (~21 KiB, which would overflow the 2 MiB default test-thread stack
/// well before the limit is reached).
fn on_big_stack(body: impl FnOnce() + Send + 'static) {
    std::thread::Builder::new()
        .stack_size(32 * 1024 * 1024)
        .spawn(body)
        .expect("failed to spawn test thread")
        .join()
        .expect("test thread panicked");
}

#[test]
fn test_nesting_just_under_the_limit_parses() {
    on_big_stack(|| {
        // The DISPLAY argument itself is one level, so `MAX - 1` parens sits
        // exactly at the limit.
        assert_output(&nested_parens(MAX_NESTING_DEPTH - 1), "1");
    });
}

#[test]
fn test_nesting_just_over_the_limit_is_a_clean_error() {
    on_big_stack(|| {
        let err = get_error(&nested_parens(MAX_NESTING_DEPTH));
        assert!(
            err.contains("Maximum nesting depth exceeded"),
            "unexpected error: {err}"
        );
    });
}

#[test]
fn test_pathological_nesting_does_not_abort() {
    // Regression: somewhere between 2000 and 2500 nested parens the parser used
    // to blow the stack and take the process down with SIGABRT (exit 134).
    on_big_stack(|| {
        let err = get_error(&nested_parens(5000));
        assert!(
            err.contains("Maximum nesting depth exceeded"),
            "unexpected error: {err}"
        );
    });
}

#[test]
fn test_nesting_limit_applies_to_lists_calls_and_blocks() {
    on_big_stack(|| {
        let list = format!("x <- {}1{}", "[".repeat(300), "]".repeat(300));
        assert!(get_error(&list).contains("Maximum nesting depth exceeded"));

        let calls = format!("DISPLAY({}1{})", "ABS(".repeat(300), ")".repeat(300));
        assert!(get_error(&calls).contains("Maximum nesting depth exceeded"));

        let blocks = format!(
            "{}\nDISPLAY(1)\n{}",
            "IF (TRUE) {".repeat(300),
            "}".repeat(300)
        );
        assert!(get_error(&blocks).contains("Maximum nesting depth exceeded"));
    });
}

/// The `eval_builtin` dispatcher used to carry arms for LENGTH, REMOVE,
/// APPEND, INSERT, CONCAT and SUBSTRING. They were unreachable: a
/// `ProcedureCall` node is only ever built from a `Token::Identifier`, and the
/// lexer turns each of these words into its own dedicated token instead. This
/// test pins that lexer behaviour down so the arms cannot silently become
/// necessary again.
#[test]
fn test_builtin_keywords_never_lex_as_identifiers() {
    for (source, expected) in [
        ("LENGTH", Token::ListLength),
        ("REMOVE", Token::ListRemove),
        ("APPEND", Token::ListAppend),
        ("INSERT", Token::ListInsert),
        ("CONCAT", Token::Concat),
        ("SUBSTRING", Token::Substring),
    ] {
        let tokens = Lexer::new(source).tokenize();
        assert_eq!(
            tokens.first().map(|(t, _)| t),
            Some(&expected),
            "{source} did not lex to its dedicated token"
        );
    }
}

#[test]
fn test_a_stray_closing_brace_at_top_level_is_an_error_not_a_hang() {
    // `parse_statement` yields an empty statement for `}` without consuming it,
    // because a block's parser consumes it itself. At top level nothing ever did, so
    // the program loop spun forever appending empty blocks until memory ran out.
    let err = get_error("DISPLAY(\"a\")\n}\nDISPLAY(\"b\")");
    assert!(err.contains("no block is open"), "{}", err);

    let err = get_error("}");
    assert!(err.contains("no block is open"), "{}", err);

    let err = get_error("IF TRUE\n{\n    DISPLAY(1)\n}\n}\n");
    assert!(err.contains("no block is open"), "{}", err);
}

#[test]
fn test_sort_takes_part_in_the_precedence_chain() {
    // SORT was intercepted above the operator chain and returned early, so none of
    // these parsed.
    assert_output("DISPLAY(SORT([1, 2]) = SORT([1, 2]))", "true");
    assert_output("DISPLAY(SORT([2, 1]) + [3])", "[1, 2, 3]");
    assert_output("DISPLAY(SORT([2, 1]) NOT= [9])", "true");
    assert_output("DISPLAY(LENGTH(SORT([3, 1])) = 2)", "true");
    // And the shapes that already worked keep working.
    assert_output("DISPLAY(SORT([3, 1, 2]))", "[1, 2, 3]");
    assert_output("DISPLAY(SORT([3, 1, 2])[1])", "1");
    assert_output("xs <- SORT([2, 1])\nDISPLAY(xs)", "[1, 2]");
}
