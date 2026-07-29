//! Non-ASCII string coverage.
//!
//! Every string position in PseudoLang is a *character* position, never a byte
//! offset. Before this group existed there was no non-ASCII coverage at all,
//! and `SUBSTRING` byte-sliced a `&str`, so `SUBSTRING("héllo", 1, 2)` aborted
//! the whole process with SIGABRT. These tests pin the character-based
//! contract down for every string builtin that takes or returns an index.

use super::{assert_output, get_error};

// ---------------------------------------------------------------------------
// SUBSTRING must never panic on a multi-byte boundary
// ---------------------------------------------------------------------------

#[test]
fn test_substring_non_ascii_no_panic() {
    // Regression: this used to slice bytes 0..=1, splitting 'é' in half, which
    // panicked and aborted the process (exit 134) instead of returning a value.
    assert_output(r#"DISPLAY(SUBSTRING("héllo", 1, 2))"#, "hé");
    assert_output(r#"DISPLAY(SUBSTRING("héllo", 2, 2))"#, "é");
    assert_output(r#"DISPLAY(SUBSTRING("héllo", 2, 5))"#, "éllo");
    assert_output(r#"DISPLAY(SUBSTRING("héllo", 1, 5))"#, "héllo");
}

#[test]
fn test_substring_multibyte_scripts() {
    assert_output(r#"DISPLAY(SUBSTRING("日本語テキスト", 1, 3))"#, "日本語");
    assert_output(r#"DISPLAY(SUBSTRING("日本語テキスト", 4, 7))"#, "テキスト");
    // Emoji are 4-byte scalars.
    assert_output(r#"DISPLAY(SUBSTRING("a🙂b", 2, 2))"#, "🙂");
    assert_output(r#"DISPLAY(SUBSTRING("Ω≈ç√", 2, 3))"#, "≈ç");
}

#[test]
fn test_substring_out_of_range_is_an_error_not_a_crash() {
    // One past the end, in characters. Previously the AST path allowed
    // `end == len` and then panicked on the inclusive byte slice.
    assert!(get_error(r#"DISPLAY(SUBSTRING("abc", 1, 4))"#).contains("Invalid substring indices"));
    assert!(
        get_error(r#"DISPLAY(SUBSTRING("héllo", 1, 6))"#).contains("Invalid substring indices")
    );
    // Reversed range.
    assert!(
        get_error(r#"DISPLAY(SUBSTRING("héllo", 3, 2))"#).contains("Invalid substring indices")
    );
    // Zero / negative start (PseudoLang indexes from 1).
    assert!(
        get_error(r#"DISPLAY(SUBSTRING("héllo", 0, 2))"#).contains("Invalid substring indices")
    );
}

// ---------------------------------------------------------------------------
// LENGTH counts characters, not bytes
// ---------------------------------------------------------------------------

#[test]
fn test_length_counts_characters() {
    // "héllo" is 6 bytes but 5 characters.
    assert_output(r#"DISPLAY(LENGTH("héllo"))"#, "5");
    // 7 characters, 21 bytes.
    assert_output(r#"DISPLAY(LENGTH("日本語テキスト"))"#, "7");
    // 4 bytes, 1 character.
    assert_output(r#"DISPLAY(LENGTH("🙂"))"#, "1");
    assert_output(r#"DISPLAY(LENGTH(""))"#, "0");
    // ASCII is unchanged.
    assert_output(r#"DISPLAY(LENGTH("hello"))"#, "5");
}

// ---------------------------------------------------------------------------
// Indexing is character based, and so is its out-of-bounds report
// ---------------------------------------------------------------------------

#[test]
fn test_string_index_non_ascii() {
    assert_output(
        r#"
        s <- "héllo"
        DISPLAY(s[1])
        DISPLAY(s[2])
        DISPLAY(s[5])
        "#,
        "h\né\no",
    );
    assert_output(
        r#"
        s <- "日本語"
        DISPLAY(s[3])
        "#,
        "語",
    );
}

#[test]
fn test_string_index_out_of_bounds_reports_character_size() {
    // The reported size must be the character count (5), not the byte count (6).
    let err = get_error(
        r#"
        s <- "héllo"
        DISPLAY(s[6])
        "#,
    );
    assert!(
        err.contains("String index out of bounds: 6 (size: 5)"),
        "unexpected error: {err}"
    );
}

// ---------------------------------------------------------------------------
// FIND returns a character position, so it round-trips into the other builtins
// ---------------------------------------------------------------------------

#[test]
fn test_find_returns_character_position() {
    // Byte offset of "llo" in "héllo" is 3; its character position is 3 as
    // well only because the accent comes first -- check a harder one too.
    assert_output(r#"DISPLAY(FIND("héllo", "llo"))"#, "3");
    assert_output(r#"DISPLAY(FIND("ééééx", "x"))"#, "5");
    assert_output(r#"DISPLAY(FIND("日本語テキスト", "テ"))"#, "4");
    assert_output(r#"DISPLAY(FIND("héllo", "zzz"))"#, "-1");
}

#[test]
fn test_find_result_feeds_back_into_index_and_substring() {
    // This is the consistency property that byte-based FIND broke: the index
    // FIND hands back must select the same text through `s[i]` and SUBSTRING.
    assert_output(
        r#"
        s <- "ééééx"
        i <- FIND(s, "x")
        DISPLAY(s[i])
        DISPLAY(SUBSTRING(s, i, i))
        DISPLAY(SUBSTRING(s, 1, i))
        DISPLAY(LENGTH(s))
        "#,
        "x\nx\nééééx\n5",
    );
    assert_output(
        r#"
        s <- "日本語テキスト"
        i <- FIND(s, "テ")
        DISPLAY(SUBSTRING(s, i, LENGTH(s)))
        "#,
        "テキスト",
    );
}

// ---------------------------------------------------------------------------
// The rest of the string surface on non-ASCII input
// ---------------------------------------------------------------------------

#[test]
fn test_other_string_builtins_non_ascii() {
    assert_output(r#"DISPLAY(CONCAT("héllo", " wörld"))"#, "héllo wörld");
    assert_output(r#"DISPLAY(UPPERCASE("héllo"))"#, "HÉLLO");
    assert_output(r#"DISPLAY(LOWERCASE("HÉLLO"))"#, "héllo");
    assert_output(r#"DISPLAY(TRIM("  héllo  "))"#, "héllo");
    assert_output(r#"DISPLAY(REPLACE("héllo", "é", "e"))"#, "hello");
    assert_output(r#"DISPLAY(CONTAINS("héllo", "é"))"#, "true");
    assert_output(r#"DISPLAY(STARTSWITH("héllo", "hé"))"#, "true");
    assert_output(r#"DISPLAY(ENDSWITH("héllo", "llo"))"#, "true");
    assert_output(r#"DISPLAY("héllo" + "!")"#, "héllo!");
}

#[test]
fn test_non_ascii_in_identifiers_and_containers() {
    // Non-ASCII text as list elements, dictionary keys and f-string values.
    assert_output(
        r#"
        words <- ["héllo", "日本語"]
        DISPLAY(LENGTH(words))
        DISPLAY(words[2])
        d <- {"clé": "valeur"}
        DISPLAY(d["clé"])
        name <- "wörld"
        DISPLAY(f"héllo {name}")
        "#,
        "2\n日本語\nvaleur\nhéllo wörld",
    );
}

#[test]
fn test_non_ascii_sort_and_comparison() {
    assert_output(r#"DISPLAY("é" = "é")"#, "true");
    assert_output(r#"DISPLAY("a" < "é")"#, "true");
    assert_output(r#"DISPLAY(SORT(["é", "a", "z"]))"#, "[a, z, é]");
}
