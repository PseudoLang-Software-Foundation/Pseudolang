//! Regression tests for the parser's recursion / teardown safety net.
//!
//! Every case here used to abort the whole process with
//! `fatal runtime error: stack overflow` (rc 134) instead of either producing a
//! `PSLError` or running to completion. They are deliberately split into a
//! "just under the limit" and a "just over the limit" pair so that a future
//! change to [`MAX_NESTING_DEPTH`] or to the guard's placement is caught rather
//! than silently absorbed.
//!
//! `MAX_NESTING_DEPTH` is the same constant in debug and release builds; only
//! the amount of real stack behind it differs. So these thresholds are exact
//! and build-independent.

use super::{assert_output, get_error};
use crate::parser::MAX_NESTING_DEPTH;

const NESTING_ERROR: &str = "Maximum nesting depth exceeded";

/// The deepest nesting that is still accepted for a plain expression.
const AT_LIMIT: usize = MAX_NESTING_DEPTH - 1; // 127

/// Parsing is recursive descent, so reaching the limit legitimately costs a lot
/// of stack in a debug build (~21 KiB per level of parenthesis). Run the
/// depth-guard cases on a thread with room to spare: the point is what the
/// *limit* does, not how much stack an unoptimised frame happens to take.
fn on_big_stack(body: impl FnOnce() + Send + 'static) {
    std::thread::Builder::new()
        .stack_size(32 * 1024 * 1024)
        .spawn(body)
        .expect("failed to spawn test thread")
        .join()
        .expect("test thread panicked");
}

fn assert_nesting_error(source: &str) {
    let err = get_error(source);
    assert!(
        err.contains(NESTING_ERROR),
        "expected a nesting-depth error, got: {err}"
    );
}

// --- nested parentheses: the case the guard already covered -----------------

#[test]
fn test_nested_parens_at_limit_parses() {
    on_big_stack(|| {
        let source = format!("DISPLAY({}1{})", "(".repeat(AT_LIMIT), ")".repeat(AT_LIMIT));
        assert_output(&source, "1");
    });
}

#[test]
fn test_nested_parens_over_limit_is_an_error_not_a_crash() {
    on_big_stack(|| {
        let source = format!(
            "DISPLAY({}1{})",
            "(".repeat(MAX_NESTING_DEPTH),
            ")".repeat(MAX_NESTING_DEPTH)
        );
        assert_nesting_error(&source);
    });
}

// --- prefix NOT: `parse_unary` used to recurse without the guard ------------

#[test]
fn test_not_chain_at_limit_parses() {
    on_big_stack(|| {
        // 127 is odd, and an odd number of negations of TRUE is FALSE.
        let source = format!("DISPLAY({}TRUE)", "NOT ".repeat(AT_LIMIT));
        assert_output(&source, "false");
    });
}

#[test]
fn test_not_chain_over_limit_is_an_error_not_a_crash() {
    on_big_stack(|| {
        let source = format!("DISPLAY({}TRUE)", "NOT ".repeat(MAX_NESTING_DEPTH));
        assert_nesting_error(&source);
    });
}

/// The original repro: 20853 `NOT`s aborted the release binary with rc 134
/// (20852 still exited 0). Any length past the limit must now be a clean error,
/// however far past it goes.
#[test]
fn test_pathological_not_chain_does_not_abort() {
    on_big_stack(|| {
        assert_nesting_error(&format!("DISPLAY({}TRUE)", "NOT ".repeat(20_853)));
    });
}

// --- prefix minus: the other unguarded arm of `parse_unary` -----------------

#[test]
fn test_unary_minus_chain_at_limit_parses() {
    on_big_stack(|| {
        let source = format!("DISPLAY({}1)", "-".repeat(AT_LIMIT));
        assert_output(&source, "-1");
    });
}

#[test]
fn test_unary_minus_chain_over_limit_is_an_error_not_a_crash() {
    on_big_stack(|| {
        let source = format!("DISPLAY({}1)", "-".repeat(MAX_NESTING_DEPTH));
        assert_nesting_error(&source);
    });
}

#[test]
fn test_pathological_unary_minus_chain_does_not_abort() {
    on_big_stack(|| {
        assert_nesting_error(&format!("DISPLAY({}1)", "-".repeat(20_853)));
    });
}

// --- ELSE IF chains: `parse_if` recursed after `parse_block` gave depth back -

/// Longest `ELSE IF` chain that still parses. One level shorter than for
/// parentheses: the trailing `ELSE` block costs a level of its own on top of
/// the chain itself.
const ELSE_IF_AT_LIMIT: usize = MAX_NESTING_DEPTH - 2; // 126

fn else_if_chain(links: usize) -> String {
    let mut source = String::from("IF (FALSE)\n{\n  DISPLAY(0)\n}\n");
    for _ in 0..links {
        source.push_str("ELSE IF (FALSE)\n{\n  DISPLAY(1)\n}\n");
    }
    source.push_str("ELSE\n{\n  DISPLAY(\"done\")\n}\n");
    source
}

#[test]
fn test_else_if_chain_at_limit_parses() {
    on_big_stack(|| assert_output(&else_if_chain(ELSE_IF_AT_LIMIT), "done"));
}

#[test]
fn test_else_if_chain_over_limit_is_an_error_not_a_crash() {
    on_big_stack(|| assert_nesting_error(&else_if_chain(ELSE_IF_AT_LIMIT + 1)));
}

/// The original repro: 14478 chained `ELSE IF`s aborted the release binary with
/// rc 134 (14477 still exited 0).
#[test]
fn test_pathological_else_if_chain_does_not_abort() {
    on_big_stack(|| assert_nesting_error(&else_if_chain(14_478)));
}

// --- blank lines: `parse_statement` used to tail-recurse on Newline ---------

/// Blank lines are not nesting, so a long run of them must neither error nor
/// overflow. This runs on the ordinary test-thread stack on purpose: skipping
/// newlines must cost no stack at all.
#[test]
fn test_long_run_of_blank_lines_does_not_overflow() {
    let source = format!("{}DISPLAY(\"after\")\n", "\n".repeat(200_000));
    assert_output(&source, "after");
}

// --- recursive AST teardown -------------------------------------------------

/// A left-associative `+` chain is built by a *loop* in `parse_term`, so the
/// depth guard never sees it, yet the tree it produces is one level deep per
/// term. Releasing that tree used to abort the process with SIGABRT *after* the
/// program had already printed the right answer, and a program that succeeded
/// must never exit 134. `Drop for Spanned` now dismantles it with a worklist.
///
/// Deliberately on the ordinary (2 MiB) test-thread stack: neither evaluation
/// nor teardown may depend on how much stack the caller happens to have.
#[test]
fn test_long_left_associative_chain_runs_to_completion() {
    const TERMS: usize = 100_000;
    let source = format!("DISPLAY({})\n", vec!["1"; TERMS].join("+"));
    assert_output(&source, &TERMS.to_string());
}

/// `stacker::maybe_grow` in `interpreter::evaluate_node` is load-bearing, not
/// dead weight: without it a deeply left-nested expression overflows the stack
/// while being *evaluated* and the process dies with SIGABRT. Deleting that
/// call makes this test kill the test runner.
#[test]
fn test_deeply_left_nested_expression_still_evaluates() {
    const TERMS: usize = 6_000;
    let source = format!("DISPLAY({})\n", vec!["1"; TERMS].join("+"));
    assert_output(&source, &TERMS.to_string());
}

// --- f-string interpolation: each slot gets its own Parser ------------------

#[test]
fn test_nested_f_strings_still_work() {
    // The depth fix must not break ordinary interpolation, including one
    // f-string nested inside another.
    assert_output("a <- 1\nDISPLAY(f\"{a}\")", "1");
    assert_output("a <- 1\nDISPLAY(f\"{ f\"{a}\" }\")", "1");
}

#[test]
fn test_deeply_nested_f_strings_are_an_error_not_a_crash() {
    // Every interpolation slot is parsed by a fresh `Parser`. Until that
    // sub-parser inherited the enclosing depth, nesting restarted at zero on
    // every level and this aborted the process with a stack overflow instead
    // of reporting the limit.
    on_big_stack(|| {
        let n = 3000;
        let source = format!("x <- {}1{}", "f\"{".repeat(n), "}\"".repeat(n));
        assert_nesting_error(&source);
    });
}
