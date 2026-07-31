//! Every runnable example in `Pseudolang.md` must parse.
//!
//! A documented example that does not work is worse than no example. One shipped
//! broken (a formatted string interpolating a dictionary access), which is what this
//! exists to catch.

use crate::lexer::Lexer;
use crate::parser;

/// Blocks that are syntax templates rather than programs, marked by a placeholder.
fn is_a_template(source: &str) -> bool {
    source.contains("<statement") || source.contains("...procs") || source.contains("<first")
}

fn psl_blocks(markdown: &str) -> Vec<(usize, String)> {
    let mut blocks = Vec::new();
    let mut lines = markdown.lines().enumerate();
    while let Some((number, line)) = lines.next() {
        if line.trim() != "```psl" {
            continue;
        }
        let mut body = String::new();
        for (_, line) in lines.by_ref() {
            if line.trim() == "```" {
                break;
            }
            body.push_str(line);
            body.push('\n');
        }
        // `number` is 0-based and the fence is one line above the body.
        blocks.push((number + 2, body));
    }
    blocks
}

#[test]
fn every_documented_example_parses() {
    let guide = include_str!("../../Pseudolang.md");
    let blocks = psl_blocks(guide);
    assert!(
        blocks.len() > 30,
        "only found {} psl blocks, the extraction must have broken",
        blocks.len()
    );

    let mut failures = Vec::new();
    let mut checked = 0;
    for (line, source) in &blocks {
        if is_a_template(source) {
            continue;
        }
        checked += 1;
        let tokens = Lexer::new(source).tokenize();
        if let Err(error) = parser::parse_with_source(tokens, source, false) {
            failures.push(format!(
                "  Pseudolang.md:{}\n{}",
                line,
                error.format(source)
            ));
        }
    }

    assert!(
        checked > 25,
        "only {} of {} blocks were runnable examples; the template filter is too broad",
        checked,
        blocks.len()
    );
    assert!(
        failures.is_empty(),
        "{} documented example(s) do not parse:\n{}",
        failures.len(),
        failures.join("\n")
    );
}

#[test]
fn the_template_filter_only_skips_placeholders() {
    // Guard on the filter itself: it must not be quietly excusing a real example.
    assert!(is_a_template("IF(a)\n{\n <statement(s)>\n}\n"));
    assert!(!is_a_template("DISPLAY(1 + 1)\n"));
    assert!(!is_a_template("IF TRUE\n{\n    DISPLAY(\"x\")\n}\n"));
}
