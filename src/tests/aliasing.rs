//! Value-semantics tests.
//!
//! PseudoLang assigns *copies*: `aList <- bList` gives `aList` its own list, a
//! list handed to a procedure is the callee's own, and the same holds for
//! dictionaries and for containers nested inside containers. The interpreter
//! mutates containers in place for speed, so these tests pin down that none of
//! that mutation is ever observable through a second name.
//!
//! They also cover the self-referential forms -- `a[i] <- a[j]`,
//! `APPEND(a, a[1])` -- where the value being written is read out of the very
//! container being written to.

use super::assert_output;

#[test]
fn test_list_copy_on_assign() {
    // Mutating the copy must not touch the original, whichever mutator is used.
    assert_output(
        r#"
            a <- [1, 2, 3]
            b <- a
            APPEND(b, 4)
            DISPLAY(a)
            DISPLAY(b)"#,
        "[1, 2, 3]\n[1, 2, 3, 4]",
    );
    assert_output(
        r#"
            a <- [1, 2, 3]
            b <- a
            b[1] <- 99
            DISPLAY(a)
            DISPLAY(b)"#,
        "[1, 2, 3]\n[99, 2, 3]",
    );
    assert_output(
        r#"
            a <- [1, 2, 3]
            b <- a
            INSERT(b, 1, 0)
            REMOVE(b, 4)
            DISPLAY(a)
            DISPLAY(b)"#,
        "[1, 2, 3]\n[0, 1, 2]",
    );
    // ...and mutating the original must not touch the copy.
    assert_output(
        r#"
            a <- [1, 2, 3]
            b <- a
            APPEND(a, 4)
            DISPLAY(a)
            DISPLAY(b)"#,
        "[1, 2, 3, 4]\n[1, 2, 3]",
    );
}

#[test]
fn test_dictionary_copy_on_assign() {
    assert_output(
        r#"
            d <- DICTIONARY()
            SETKEY(d, "a", 1)
            e <- d
            SETKEY(e, "b", 2)
            REMOVEKEY(e, "a")
            DISPLAY(d)
            DISPLAY(e)"#,
        "{a: 1}\n{b: 2}",
    );
    assert_output(
        r#"
            d <- {"a": 1}
            e <- d
            e["a"] <- 99
            DISPLAY(d)
            DISPLAY(e)"#,
        "{a: 1}\n{a: 99}",
    );
}

#[test]
fn test_procedure_argument_is_a_copy() {
    assert_output(
        r#"
            PROCEDURE mutate(inner) {
                APPEND(inner, 100)
                inner[1] <- -1
                RETURN (inner)
            }
            c <- [5, 6]
            r <- mutate(c)
            DISPLAY(c)
            DISPLAY(r)"#,
        "[5, 6]\n[-1, 6, 100]",
    );
    assert_output(
        r#"
            PROCEDURE mutate(d) {
                SETKEY(d, "added", TRUE)
                RETURN (LENGTH(d))
            }
            src <- {"a": 1}
            DISPLAY(mutate(src))
            DISPLAY(src)"#,
        "2\n{a: 1}",
    );
}

#[test]
fn test_procedure_cannot_mutate_an_enclosing_scope_container() {
    // A procedure that names an outer list writes into its OWN scope, so the
    // caller's list is untouched once the call returns.
    assert_output(
        r#"
            g <- [1, 2]
            PROCEDURE touch() {
                APPEND(g, 3)
                DISPLAY(g)
            }
            touch()
            DISPLAY(g)"#,
        "[1, 2, 3]\n[1, 2]",
    );
    assert_output(
        r#"
            g <- {"a": 1}
            PROCEDURE touch() {
                SETKEY(g, "b", 2)
                DISPLAY(g)
            }
            touch()
            DISPLAY(g)"#,
        "{a: 1, b: 2}\n{a: 1}",
    );
    assert_output(
        r#"
            g <- [[1, 2], [3, 4]]
            PROCEDURE touch() {
                g[1][1] <- 99
                DISPLAY(g)
            }
            touch()
            DISPLAY(g)"#,
        "[[99, 2], [3, 4]]\n[[1, 2], [3, 4]]",
    );
}

#[test]
fn test_list_inside_dictionary_is_a_copy() {
    // Storing a list under a key copies it, so later growth of the source is
    // invisible to the dictionary, and vice versa.
    assert_output(
        r#"
            inner <- [1, 2]
            d <- DICTIONARY()
            SETKEY(d, "k", inner)
            APPEND(inner, 3)
            DISPLAY(d)
            DISPLAY(inner)"#,
        "{k: [1, 2]}\n[1, 2, 3]",
    );
    assert_output(
        r#"
            inner <- [1, 2]
            d <- DICTIONARY()
            SETKEY(d, "k", inner)
            e <- d["k"]
            APPEND(e, 9)
            DISPLAY(d)
            DISPLAY(e)"#,
        "{k: [1, 2]}\n[1, 2, 9]",
    );
    assert_output(
        r#"
            inner <- [1, 2]
            d <- DICTIONARY()
            SETKEY(d, "k", inner)
            d["k"][1] <- 77
            DISPLAY(d)
            DISPLAY(inner)"#,
        "{k: [77, 2]}\n[1, 2]",
    );
}

#[test]
fn test_dictionary_inside_list_is_a_copy() {
    assert_output(
        r#"
            dd <- DICTIONARY()
            SETKEY(dd, "x", 1)
            lst <- [dd, 2]
            SETKEY(dd, "y", 2)
            DISPLAY(lst)
            DISPLAY(dd)"#,
        "[{x: 1}, 2]\n{x: 1, y: 2}",
    );
    assert_output(
        r#"
            dd <- {"x": 1}
            lst <- [dd, 2]
            lst[1]["x"] <- 42
            DISPLAY(lst)
            DISPLAY(dd)"#,
        "[{x: 42}, 2]\n{x: 1}",
    );
}

#[test]
fn test_nested_container_copy_depth_two() {
    assert_output(
        r#"
            leaf <- [1]
            mid <- {"leaf": leaf}
            outer <- [mid]
            outer[1]["leaf"][1] <- 5
            DISPLAY(outer)
            DISPLAY(mid)
            DISPLAY(leaf)"#,
        "[{leaf: [5]}]\n{leaf: [1]}\n[1]",
    );
}

#[test]
fn test_self_referential_list_assignment() {
    // The right-hand side is read out of the same list that is written.
    assert_output(
        r#"
            s <- [10, 20, 30]
            s[1] <- s[3]
            DISPLAY(s)"#,
        "[30, 20, 30]",
    );
    assert_output(
        r#"
            s <- [10, 20, 30]
            i <- 2
            j <- 3
            s[i] <- s[j] + s[i]
            DISPLAY(s)"#,
        "[10, 50, 30]",
    );
    assert_output(
        r#"
            s <- [1, 2, 3]
            s[2] <- s
            DISPLAY(s)"#,
        "[1, [1, 2, 3], 3]",
    );
    assert_output(
        r#"
            u <- [1, 2, 3]
            u <- u
            DISPLAY(u)"#,
        "[1, 2, 3]",
    );
}

#[test]
fn test_self_referential_list_builtins() {
    assert_output(
        r#"
            s <- [10, 20, 30]
            APPEND(s, s[1])
            DISPLAY(s)"#,
        "[10, 20, 30, 10]",
    );
    assert_output(
        r#"
            s <- [10, 20, 30]
            INSERT(s, 1, s[2])
            DISPLAY(s)"#,
        "[20, 10, 20, 30]",
    );
    assert_output(
        r#"
            s <- [3, 20, 30]
            DISPLAY(REMOVE(s, s[1]))
            DISPLAY(s)"#,
        "30\n[3, 20]",
    );
    assert_output(
        r#"
            s <- [1, 2]
            APPEND(s, s)
            DISPLAY(s)"#,
        "[1, 2, [1, 2]]",
    );
    assert_output(
        r#"
            s <- [1, 2]
            APPEND(s, LENGTH(s))
            DISPLAY(s)"#,
        "[1, 2, 2]",
    );
}

#[test]
fn test_self_referential_dictionary_builtins() {
    assert_output(
        r#"
            d <- DICTIONARY()
            SETKEY(d, "a", 1)
            SETKEY(d, "b", d["a"])
            SETKEY(d, "c", d)
            DISPLAY(d)"#,
        "{a: 1, b: 1, c: {a: 1, b: 1}}",
    );
    assert_output(
        r#"
            d <- {"a": 1, "b": 2}
            ks <- KEYS(d)
            DISPLAY(REMOVEKEY(d, ks[1]))
            DISPLAY(d)"#,
        "1\n{b: 2}",
    );
    assert_output(
        r#"
            d <- {"a": 1}
            d["b"] <- LENGTH(d)
            DISPLAY(d)"#,
        "{a: 1, b: 1}",
    );
}

#[test]
fn test_self_referential_nested_assignment() {
    assert_output(
        r#"
            m <- [[1, 2], [3, 4]]
            m[1][1] <- m[2][2]
            DISPLAY(m)"#,
        "[[4, 2], [3, 4]]",
    );
    assert_output(
        r#"
            m <- [[1, 2], [3, 4]]
            m[2] <- m[1]
            m[1][1] <- 9
            DISPLAY(m)"#,
        "[[9, 2], [1, 2]]",
    );
    assert_output(
        r#"
            d <- {"in": {"z": 1}}
            d["in"]["z"] <- d["in"]["z"] + 1
            DISPLAY(d)"#,
        "{in: {z: 2}}",
    );
}

#[test]
fn test_indexed_read_does_not_disturb_the_container() {
    // The in-place read path hands out a copy of one element; writing through
    // that copy must not reach back into the container.
    assert_output(
        r#"
            m <- [[1, 2], [3, 4]]
            row <- m[1]
            APPEND(row, 99)
            DISPLAY(m)
            DISPLAY(row)"#,
        "[[1, 2], [3, 4]]\n[1, 2, 99]",
    );
    assert_output(
        r#"
            d <- {"k": {"inner": 1}}
            got <- d["k"]
            SETKEY(got, "inner", 2)
            DISPLAY(d)
            DISPLAY(got)"#,
        "{k: {inner: 1}}\n{inner: 2}",
    );
    assert_output(
        r#"
            m <- [[[1]]]
            deep <- m[1][1]
            APPEND(deep, 2)
            DISPLAY(m)
            DISPLAY(deep)"#,
        "[[[1]]]\n[1, 2]",
    );
}

#[test]
fn test_side_effecting_index_still_reads_the_pre_call_container() {
    // An index expression that mutates the container is evaluated against the
    // container as it stood when the access started.
    assert_output(
        r#"
            b <- [1, 2, 3]
            PROCEDURE shrink() {
                REMOVE(b, 3)
                RETURN (3)
            }
            DISPLAY(b[shrink()])
            DISPLAY(b)"#,
        "3\n[1, 2, 3]",
    );
}

#[test]
fn test_for_each_binding_is_a_copy() {
    assert_output(
        r#"
            m <- [[1], [2]]
            FOR EACH row IN m {
                APPEND(row, 0)
            }
            DISPLAY(m)"#,
        "[[1], [2]]",
    );
}
