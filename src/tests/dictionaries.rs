use super::{assert_output, get_error};

#[test]
fn test_dictionary_creation() {
    assert_output("d <- {}\nDISPLAY(d)", "{}");
    assert_output("d <- DICTIONARY()\nDISPLAY(d)", "{}");
    assert_output("d <- {}\nDISPLAY(LENGTH(d))", "0");
    assert_output("d <- DICTIONARY()\nDISPLAY(LENGTH(d))", "0");
    assert_output("d <- {\"a\": 1}\nDISPLAY(d)", "{a: 1}");
}

#[test]
fn test_dictionary_display_form() {
    assert_output(
        "d <- {\"name\": \"Bob\", \"age\": 30}\nDISPLAY(d)",
        "{name: Bob, age: 30}",
    );
    assert_output(
        "d <- {\"flag\": TRUE, \"ratio\": 1.5}\nDISPLAY(d)",
        "{flag: true, ratio: 1.5}",
    );
    assert_output("d <- {\"items\": [1, 2]}\nDISPLAY(d)", "{items: [1, 2]}");
}

#[test]
fn test_dictionary_multiline_literal() {
    assert_output(
        r#"
            d <- {
                "a": 1,
                "b": 2,
                "c": 3
            }
            DISPLAY(d)
            "#,
        "{a: 1, b: 2, c: 3}",
    );

    assert_output(
        r#"
            d <- {
                "a":
                    1,
                "b": 2
            }
            DISPLAY(LENGTH(d))
            "#,
        "2",
    );
}

#[test]
fn test_dictionary_nesting() {
    assert_output(
        r#"
            d <- {"outer": {"inner": 5}}
            DISPLAY(d)
            DISPLAY(d["outer"]["inner"])
            "#,
        "{outer: {inner: 5}}\n5",
    );

    assert_output(
        r#"
            d <- {"nums": [1, 2, 3]}
            DISPLAY(d)
            DISPLAY(d["nums"][2])
            "#,
        "{nums: [1, 2, 3]}\n2",
    );

    assert_output(
        r#"
            list <- [{"a": 1}, {"b": 2}]
            DISPLAY(list)
            DISPLAY(list[2]["b"])
            "#,
        "[{a: 1}, {b: 2}]\n2",
    );
}

#[test]
fn test_dictionary_get() {
    assert_output(
        r#"
            d <- {"name": "Bob", "age": 30}
            DISPLAY(d["name"])
            DISPLAY(d["age"])
            "#,
        "Bob\n30",
    );

    assert_output(
        r#"
            d <- {"name": "Bob"}
            key <- "name"
            DISPLAY(d[key])
            "#,
        "Bob",
    );
}

#[test]
fn test_dictionary_missing_key_error() {
    let err = get_error("d <- {\"a\": 1}\nDISPLAY(d[\"zzz\"])");
    assert!(err.contains("Key not found: zzz"), "{}", err);

    let err = get_error("d <- {}\nDISPLAY(d[1])");
    assert!(err.contains("Key not found: 1"), "{}", err);
}

#[test]
fn test_dictionary_set_creates_key() {
    assert_output(
        r#"
            d <- {}
            d["a"] <- 1
            d["b"] <- 2
            DISPLAY(d)
            "#,
        "{a: 1, b: 2}",
    );

    assert_output(
        r#"
            d <- {"a": 1}
            d["b"] <- 2
            DISPLAY(LENGTH(d))
            "#,
        "2",
    );
}

#[test]
fn test_dictionary_overwrite_preserves_position() {
    assert_output(
        r#"
            d <- {"a": 1, "b": 2, "c": 3}
            d["a"] <- 99
            DISPLAY(d)
            DISPLAY(KEYS(d))
            "#,
        "{a: 99, b: 2, c: 3}\n[a, b, c]",
    );

    assert_output(
        r#"
            d <- {"a": 1, "b": 2}
            SETKEY(d, "a", 10)
            SETKEY(d, "c", 30)
            DISPLAY(d)
            "#,
        "{a: 10, b: 2, c: 30}",
    );

    // A duplicate key inside one literal keeps the first position, last value.
    assert_output(
        "d <- {\"a\": 1, \"b\": 2, \"a\": 3}\nDISPLAY(d)",
        "{a: 3, b: 2}",
    );
}

#[test]
fn test_dictionary_mixed_key_types() {
    assert_output(
        r#"
            d <- {"s": 1, 2: "two", TRUE: "yes", FALSE: "no"}
            DISPLAY(d)
            DISPLAY(d["s"])
            DISPLAY(d[2])
            DISPLAY(d[TRUE])
            DISPLAY(d[FALSE])
            "#,
        "{s: 1, 2: two, true: yes, false: no}\n1\ntwo\nyes\nno",
    );

    // Integer and string keys that print alike are distinct keys.
    assert_output(
        r#"
            d <- {1: "int", "1": "str"}
            DISPLAY(LENGTH(d))
            DISPLAY(d[1])
            DISPLAY(d["1"])
            "#,
        "2\nint\nstr",
    );
}

#[test]
fn test_dictionary_illegal_key_types() {
    let err = get_error("d <- {1.5: \"x\"}");
    assert!(
        err.contains("Dictionary keys must be strings, integers, or booleans"),
        "{}",
        err,
    );

    let err = get_error("d <- {NULL: \"x\"}");
    assert!(
        err.contains("Dictionary keys must be strings, integers, or booleans"),
        "{}",
        err,
    );

    let err = get_error("d <- {NAN: \"x\"}");
    assert!(
        err.contains("Dictionary keys must be strings, integers, or booleans"),
        "{}",
        err,
    );

    let err = get_error("d <- {[1, 2]: \"x\"}");
    assert!(
        err.contains("Dictionary keys must be strings, integers, or booleans"),
        "{}",
        err,
    );

    let err = get_error("d <- {{\"a\": 1}: \"x\"}");
    assert!(
        err.contains("Dictionary keys must be strings, integers, or booleans"),
        "{}",
        err,
    );

    // Illegal keys are rejected on read and on write too, not just in literals.
    let err = get_error("d <- {\"a\": 1}\nDISPLAY(d[2.5])");
    assert!(
        err.contains("Dictionary keys must be strings, integers, or booleans"),
        "{}",
        err,
    );

    let err = get_error("d <- {\"a\": 1}\nd[NULL] <- 2");
    assert!(
        err.contains("Dictionary keys must be strings, integers, or booleans"),
        "{}",
        err,
    );
}

#[test]
fn test_dictionary_keys_and_values_order() {
    assert_output(
        r#"
            d <- {"b": 2, "a": 1, "c": 3}
            DISPLAY(KEYS(d))
            DISPLAY(VALUES(d))
            "#,
        "[b, a, c]\n[2, 1, 3]",
    );

    assert_output("DISPLAY(KEYS({}))\nDISPLAY(VALUES({}))", "[]\n[]");

    // KEYS, VALUES, DISPLAY and FOR EACH all agree on the insertion order.
    assert_output(
        r#"
            d <- {}
            d["z"] <- 1
            d["y"] <- 2
            d["x"] <- 3
            d["y"] <- 20
            DISPLAY(d)
            DISPLAY(KEYS(d))
            DISPLAY(VALUES(d))
            FOR EACH k IN d {
                DISPLAYINLINE(k)
            }
            DISPLAY("")
            "#,
        "{z: 1, y: 20, x: 3}\n[z, y, x]\n[1, 20, 3]\nzyx",
    );
}

#[test]
fn test_dictionary_haskey() {
    assert_output(
        r#"
            d <- {"a": 1, 2: "two", TRUE: "yes"}
            DISPLAY(HASKEY(d, "a"))
            DISPLAY(HASKEY(d, 2))
            DISPLAY(HASKEY(d, TRUE))
            DISPLAY(HASKEY(d, "missing"))
            DISPLAY(HASKEY(d, 99))
            DISPLAY(HASKEY(d, FALSE))
            "#,
        "true\ntrue\ntrue\nfalse\nfalse\nfalse",
    );

    assert_output("DISPLAY(HASKEY({}, \"a\"))", "false");
}

#[test]
fn test_dictionary_getkey() {
    assert_output(
        r#"
            d <- {"a": 1, "b": 2}
            DISPLAY(GETKEY(d, "a"))
            DISPLAY(GETKEY(d, "b", 0))
            DISPLAY(GETKEY(d, "missing", 0))
            DISPLAY(GETKEY(d, "missing", "fallback"))
            "#,
        "1\n2\n0\nfallback",
    );

    let err = get_error("d <- {\"a\": 1}\nDISPLAY(GETKEY(d, \"missing\"))");
    assert!(err.contains("Key not found: missing"), "{}", err);
}

#[test]
fn test_dictionary_setkey() {
    assert_output(
        r#"
            d <- {"a": 1}
            SETKEY(d, "b", 2)
            DISPLAY(d)
            "#,
        "{a: 1, b: 2}",
    );

    // SETKEY returns the value that was stored, mirroring APPEND.
    assert_output(
        r#"
            d <- {"a": 1}
            r <- SETKEY(d, "a", 42)
            DISPLAY(r)
            DISPLAY(d)
            "#,
        "42\n{a: 42}",
    );

    assert_output(
        r#"
            d <- DICTIONARY()
            SETKEY(d, 1, "one")
            SETKEY(d, TRUE, "yes")
            DISPLAY(d)
            "#,
        "{1: one, true: yes}",
    );
}

#[test]
fn test_dictionary_removekey() {
    assert_output(
        r#"
            d <- {"a": 1, "b": 2, "c": 3}
            removed <- REMOVEKEY(d, "b")
            DISPLAY(removed)
            DISPLAY(d)
            DISPLAY(LENGTH(d))
            "#,
        "2\n{a: 1, c: 3}\n2",
    );

    assert_output(
        r#"
            d <- {"a": [1, 2]}
            DISPLAY(REMOVEKEY(d, "a"))
            DISPLAY(d)
            "#,
        "[1, 2]\n{}",
    );

    let err = get_error("d <- {\"a\": 1}\nREMOVEKEY(d, \"missing\")");
    assert!(err.contains("Key not found: missing"), "{}", err);
}

#[test]
fn test_dictionary_length() {
    assert_output("DISPLAY(LENGTH({}))", "0");
    assert_output("DISPLAY(LENGTH({\"a\": 1, \"b\": 2, \"c\": 3}))", "3");
    assert_output(
        r#"
            d <- {"a": 1}
            d["b"] <- 2
            DISPLAY(LENGTH(d))
            REMOVEKEY(d, "a")
            DISPLAY(LENGTH(d))
            "#,
        "2\n1",
    );
}

#[test]
fn test_dictionary_remove() {
    // The list REMOVE builtin accepts dictionaries and removes by key.
    assert_output(
        r#"
            d <- {"a": 1, "b": 2}
            removed <- REMOVE(d, "a")
            DISPLAY(removed)
            DISPLAY(d)
            "#,
        "1\n{b: 2}",
    );

    assert_output(
        r#"
            d <- {1: "one", 2: "two"}
            REMOVE(d, 1)
            DISPLAY(d)
            "#,
        "{2: two}",
    );

    let err = get_error("d <- {\"a\": 1}\nREMOVE(d, \"zz\")");
    assert!(err.contains("Key not found: zz"), "{}", err);
}

#[test]
fn test_dictionary_for_each_iterates_keys() {
    assert_output(
        r#"
            d <- {"a": 1, "b": 2, "c": 3}
            FOR EACH k IN d {
                DISPLAY(k)
            }
            "#,
        "a\nb\nc",
    );

    assert_output(
        r#"
            d <- {"a": 1, "b": 2, "c": 3}
            total <- 0
            FOR EACH k IN d {
                total <- total + d[k]
            }
            DISPLAY(total)
            "#,
        "6",
    );

    assert_output(
        r#"
            d <- {}
            FOR EACH k IN d {
                DISPLAY(k)
            }
            DISPLAY("done")
            "#,
        "done",
    );

    assert_output(
        r#"
            d <- {2: "two", 1: "one", TRUE: "yes"}
            FOR EACH k IN d {
                DISPLAY(k)
            }
            "#,
        "2\n1\ntrue",
    );
}

#[test]
fn test_dictionary_equality_is_order_insensitive() {
    assert_output(
        r#"
            a <- {"x": 1, "y": 2}
            b <- {"y": 2, "x": 1}
            DISPLAY(a = b)
            DISPLAY(a NOT= b)
            "#,
        "true\nfalse",
    );

    assert_output(
        r#"
            a <- {"x": 1}
            b <- {"x": 1, "y": 2}
            DISPLAY(a = b)
            DISPLAY(a NOT= b)
            "#,
        "false\ntrue",
    );

    assert_output(
        r#"
            a <- {"x": 1}
            b <- {"x": 2}
            DISPLAY(a = b)
            "#,
        "false",
    );

    assert_output("DISPLAY({} = {})", "true");

    // Deep equality: nested lists and dictionaries compare structurally.
    assert_output(
        r#"
            a <- {"n": {"i": [1, 2]}}
            b <- {"n": {"i": [1, 2]}}
            DISPLAY(a = b)
            "#,
        "true",
    );

    // Ordering comparisons remain unsupported.
    let err = get_error("d <- {\"a\": 1}\nDISPLAY(d < d)");
    assert!(err.contains("Invalid operation"), "{}", err);

    let err = get_error("d <- {\"a\": 1}\nDISPLAY(d >= d)");
    assert!(err.contains("Invalid operation"), "{}", err);
}

#[test]
fn test_dictionary_merge_with_plus() {
    assert_output(
        r#"
            a <- {"x": 1, "y": 2}
            b <- {"y": 99, "z": 3}
            DISPLAY(a + b)
            "#,
        "{x: 1, y: 99, z: 3}",
    );

    // The merge creates a new dictionary; the operands are untouched.
    assert_output(
        r#"
            a <- {"x": 1}
            b <- {"x": 2}
            c <- a + b
            DISPLAY(c)
            DISPLAY(a)
            DISPLAY(b)
            "#,
        "{x: 2}\n{x: 1}\n{x: 2}",
    );

    assert_output(
        r#"
            a <- {"x": 1}
            DISPLAY(a + {})
            DISPLAY({} + a)
            "#,
        "{x: 1}\n{x: 1}",
    );

    let err = get_error("DISPLAY({\"a\": 1} + [1, 2])");
    assert!(err.contains("Invalid operation"), "{}", err);
}

#[test]
fn test_dictionary_tostring() {
    assert_output("DISPLAY(TOSTRING({}))", "{}");
    assert_output(
        "d <- {\"name\": \"Bob\", \"age\": 30}\nDISPLAY(TOSTRING(d))",
        "{name: Bob, age: 30}",
    );
    assert_output(
        r#"
            d <- {"a": 1}
            s <- CONCAT("d = ", TOSTRING(d))
            DISPLAY(s)
            "#,
        "d = {a: 1}",
    );
}

#[test]
fn test_dictionary_in_formatted_string() {
    assert_output(
        r#"
            d <- {"name": "Bob", "age": 30}
            DISPLAY(f"person: {d}")
            "#,
        "person: {name: Bob, age: 30}",
    );

    assert_output(
        r#"
            d <- {"a": 1}
            DISPLAY(f"value {d["a"]} of {LENGTH(d)}")
            "#,
        "value 1 of 1",
    );

    assert_output(
        r#"
            d <- {"n": {"i": 2}}
            DISPLAY(f"nested {d}")
            "#,
        "nested {n: {i: 2}}",
    );
}

#[test]
fn test_dictionary_copy_on_assign() {
    assert_output(
        r#"
            a <- {"k": 1}
            b <- a
            b["k"] <- 99
            DISPLAY(a)
            DISPLAY(b)
            "#,
        "{k: 1}\n{k: 99}",
    );

    // Dictionaries are passed to procedures by value, like lists.
    assert_output(
        r#"
            PROCEDURE bump(d) {
                d["k"] <- 99
                RETURN d
            }
            orig <- {"k": 1}
            copy <- bump(orig)
            DISPLAY(orig)
            DISPLAY(copy)
            "#,
        "{k: 1}\n{k: 99}",
    );
}

#[test]
fn test_dictionary_returned_from_procedure() {
    assert_output(
        r#"
            PROCEDURE build() {
                RETURN {"a": 1, "b": 2}
            }
            DISPLAY(build())
            "#,
        "{a: 1, b: 2}",
    );

    assert_output(
        r#"
            PROCEDURE build() {
                RETURN ({"a": 1})
            }
            d <- build()
            DISPLAY(d["a"])
            "#,
        "1",
    );

    assert_output(
        r#"
            PROCEDURE empty() {
                RETURN {}
            }
            DISPLAY(LENGTH(empty()))
            "#,
        "0",
    );
}

#[test]
fn test_dictionary_nested_assignment() {
    assert_output(
        r#"
            d <- {"inner": {"a": 1}}
            d["inner"]["b"] <- 2
            DISPLAY(d)
            "#,
        "{inner: {a: 1, b: 2}}",
    );

    assert_output(
        r#"
            d <- {"nums": [1, 2, 3]}
            d["nums"][2] <- 99
            DISPLAY(d)
            "#,
        "{nums: [1, 99, 3]}",
    );

    assert_output(
        r#"
            list <- [{"a": 1}]
            list[1]["a"] <- 42
            DISPLAY(list)
            "#,
        "[{a: 42}]",
    );
}

#[test]
fn test_dictionary_brace_at_statement_position_is_parse_error() {
    // `{` must never start a statement: allowing it would let a stray brace
    // swallow a block. Dictionary literals are only valid in expressions.
    let err = get_error("{\"a\": 1}");
    assert!(err.contains("Unexpected token in statement"), "{}", err);

    let err = get_error("DISPLAY(1)\n{\"a\": 1}\nDISPLAY(2)");
    assert!(err.contains("Unexpected token in statement"), "{}", err);

    let err = get_error("{}");
    assert!(err.contains("Unexpected token in statement"), "{}", err);
}

#[test]
fn test_indexed_assignment_evaluates_each_index_once() {
    // A side-effecting index must read and write the same slot, and must not
    // run twice: `m[idx()][1] <- v` used to re-evaluate idx() on write-back.
    assert_output(
        r#"
            PROCEDURE idx()
            {
                DISPLAY("called")
                RETURN 1
            }
            m <- [[0, 0], [0, 0]]
            m[idx()][1] <- 99
            DISPLAY(m)
            "#,
        "called\n[[99, 0], [0, 0]]",
    );

    assert_output(
        r#"
            PROCEDURE which()
            {
                RETURN "a"
            }
            d <- {"a": {"b": 1}, "c": {"b": 2}}
            d[which()]["b"] <- 42
            DISPLAY(d)
            "#,
        "{a: {b: 42}, c: {b: 2}}",
    );
}

#[test]
fn test_dictionary_key_newline_before_colon() {
    assert_output("d <- {\"a\"\n: 1}\nDISPLAY(d)", "{a: 1}");
}

#[test]
fn test_haskey_and_getkey_with_unusable_key() {
    // A value that can never be a key is absent rather than an error, so
    // `IF HASKEY(d, k)` and a GETKEY default stay usable for any k.
    assert_output("d <- {\"a\": 1}\nDISPLAY(HASKEY(d, 1.5))", "false");
    assert_output("d <- {\"a\": 1}\nDISPLAY(HASKEY(d, NULL))", "false");
    assert_output("d <- {\"a\": 1}\nDISPLAY(HASKEY(d, [1]))", "false");
    assert_output(
        "d <- {\"a\": 1}\nDISPLAY(GETKEY(d, 1.5, \"fallback\"))",
        "fallback",
    );

    // Without a default there is still a clear error.
    let err = get_error("d <- {\"a\": 1}\nDISPLAY(GETKEY(d, 1.5))");
    assert!(
        err.contains("Dictionary keys must be strings, integers, or booleans"),
        "{}",
        err
    );
}

#[test]
fn test_list_equality_matches_dictionary_equality() {
    assert_output("DISPLAY([1, 2] = [1, 2])", "true");
    assert_output("DISPLAY([1, 2] = [2, 1])", "false");
    assert_output("DISPLAY([1, 2] NOT= [1])", "true");
    assert_output("DISPLAY([[1], [2]] = [[1], [2]])", "true");
    // The same comparison hoisted out of a dictionary must agree.
    assert_output("DISPLAY({\"a\": [1, 2]} = {\"a\": [1, 2]})", "true");
}
