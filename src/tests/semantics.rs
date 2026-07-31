//! Regression tests for two bugs. Comparing anything against `NULL` answered
//! `false` whichever operator was used, leaving no way to test for absence. And a
//! `CATCH` block ran in a private scope, discarding its assignments.

use super::{assert_output, get_error};

// ---------------------------------------------------------------------------
// Comparing against NULL
// ---------------------------------------------------------------------------

#[test]
fn null_equals_itself_and_nothing_else() {
    assert_output(
        r#"
        DISPLAY(NULL = NULL)
        DISPLAY(NULL NOT= NULL)
        "#,
        "true\nfalse",
    );
}

#[test]
fn a_value_that_is_not_null_is_unequal_to_null() {
    // The bug: both of these were `false`, for every value kind.
    assert_output(
        r#"
        DISPLAY("s" NOT= NULL)
        DISPLAY("s" = NULL)
        DISPLAY(0 NOT= NULL)
        DISPLAY(0 = NULL)
        DISPLAY(1.5 NOT= NULL)
        DISPLAY(FALSE NOT= NULL)
        DISPLAY(FALSE = NULL)
        DISPLAY([] NOT= NULL)
        DISPLAY({} NOT= NULL)
        "#,
        "true\nfalse\ntrue\nfalse\ntrue\ntrue\nfalse\ntrue\ntrue",
    );
}

#[test]
fn null_comparison_works_with_null_on_either_side() {
    assert_output(
        r#"
        DISPLAY(NULL NOT= "s")
        DISPLAY(NULL = "s")
        DISPLAY(NULL NOT= 0)
        DISPLAY(NULL = 0)
        "#,
        "true\nfalse\ntrue\nfalse",
    );
}

#[test]
fn a_null_check_guards_an_optional_result() {
    // The idiom the guide recommends for the built-ins that may report nothing.
    assert_output(
        r#"
        found <- WHICH("psl-definitely-not-installed-xyz")
        IF found = NULL
        {
            DISPLAY("not installed")
        } ELSE
        {
            DISPLAY("installed")
        }
        DISPLAY(PROCESSINFO(999999999) = NULL)
        "#,
        "not installed\ntrue",
    );
}

#[test]
fn a_null_check_composes_with_and_or_and_not() {
    assert_output(
        r#"
        x <- NULL
        y <- 5
        DISPLAY(x = NULL AND y NOT= NULL)
        DISPLAY(x NOT= NULL OR y NOT= NULL)
        DISPLAY(NOT (x NOT= NULL))
        "#,
        "true\ntrue\ntrue",
    );
}

#[test]
fn ordering_against_null_stays_false_both_ways() {
    // Deliberately unchanged: there is no sensible order between a value and an
    // absent one, so neither direction is true.
    assert_output(
        r#"
        DISPLAY(1 < NULL)
        DISPLAY(1 > NULL)
        DISPLAY(NULL < 1)
        DISPLAY(NULL > 1)
        DISPLAY(NULL <= NULL)
        "#,
        "false\nfalse\nfalse\nfalse\nfalse",
    );
}

#[test]
fn nan_keeps_its_own_comparison_rules() {
    // NAN is *not* NULL: it is unequal to everything including itself, which is
    // the documented behaviour and must not have been swept up by the fix.
    assert_output(
        r#"
        DISPLAY(NAN = NAN)
        DISPLAY(NAN NOT= NAN)
        DISPLAY(NAN = 1)
        DISPLAY(NAN NOT= 1)
        DISPLAY(NAN = NULL)
        DISPLAY(NAN NOT= NULL)
        "#,
        "false\ntrue\nfalse\ntrue\nfalse\ntrue",
    );
}

#[test]
fn null_equality_inside_a_list_and_a_dictionary_value() {
    assert_output(
        r#"
        xs <- [NULL, 1]
        DISPLAY(xs[1] = NULL)
        DISPLAY(xs[2] = NULL)
        d <- {"a": NULL}
        DISPLAY(d["a"] = NULL)
        DISPLAY(CONTAINS(xs, NULL))
        "#,
        "true\nfalse\ntrue\ntrue",
    );
}

// ---------------------------------------------------------------------------
// CONTAINS over containers
// ---------------------------------------------------------------------------

#[test]
fn test_contains_on_lists_and_dictionaries() {
    assert_output(
        r#"
        DISPLAY(CONTAINS([1, 2, 3], 2))
        DISPLAY(CONTAINS([1, 2, 3], 9))
        DISPLAY(CONTAINS(["a", "b"], "b"))
        DISPLAY(CONTAINS([], 1))
        DISPLAY(CONTAINS({"k": 1, "j": 2}, "k"))
        DISPLAY(CONTAINS({"k": 1}, "missing"))
        DISPLAY(CONTAINS("hello", "ell"))
        "#,
        "true\nfalse\ntrue\nfalse\ntrue\nfalse\ntrue",
    );
}

#[test]
fn test_contains_on_a_dictionary_with_an_unusable_key_is_false() {
    // A float can never be a dictionary key, so it is certainly not present.
    assert_output(r#"DISPLAY(CONTAINS({"a": 1}, 1.5))"#, "false");
}

#[test]
fn test_contains_still_rejects_a_number_as_its_first_argument() {
    let err = get_error("DISPLAY(CONTAINS(5, 5))");
    assert!(
        err.contains("CONTAINS requires a string, list or dictionary"),
        "{}",
        err
    );
}

// ---------------------------------------------------------------------------
// Block scoping
// ---------------------------------------------------------------------------

#[test]
fn an_assignment_in_a_catch_block_survives_the_block() {
    // The bug: CATCH ran in a child scope, so `config` was undefined afterwards.
    assert_output(
        r#"
        TRY
        {
            config <- READFILE("no-such-file-at-all.txt")
        } CATCH (err)
        {
            config <- "defaults"
        }
        DISPLAY(config)
        "#,
        "defaults",
    );
}

#[test]
fn every_block_form_leaves_its_assignments_behind() {
    // One rule for all of them, which is what makes the CATCH behaviour a bug
    // rather than a design choice.
    assert_output(
        r#"
        IF TRUE
        {
            from_if <- "if"
        }
        FOR EACH i IN [1]
        {
            from_for <- "for"
        }
        REPEAT 1 TIMES
        {
            from_repeat <- "repeat"
        }
        TRY
        {
            from_try <- "try"
        } CATCH (e)
        {
            unreached <- TRUE
        }
        TRY
        {
            x <- 1 / 0
        } CATCH (e)
        {
            from_catch <- "catch"
        }
        DISPLAY(from_if)
        DISPLAY(from_for)
        DISPLAY(from_repeat)
        DISPLAY(from_try)
        DISPLAY(from_catch)
        DISPLAY(ISDEFINED("unreached"))
        "#,
        "if\nfor\nrepeat\ntry\ncatch\nfalse",
    );
}

#[test]
fn the_error_variable_is_scoped_to_the_catch_block() {
    // Only the error variable is contained: it must not linger afterwards.
    assert_output(
        r#"
        TRY
        {
            x <- 1 / 0
        } CATCH (err)
        {
            DISPLAY(LENGTH(err) > 0)
        }
        DISPLAY(ISDEFINED("err"))
        "#,
        "true\nfalse",
    );
}

#[test]
fn the_error_variable_restores_a_name_it_shadowed() {
    assert_output(
        r#"
        err <- "mine"
        TRY
        {
            x <- 1 / 0
        } CATCH (err)
        {
            DISPLAY(err NOT= "mine")
        }
        DISPLAY(err)
        "#,
        "true\nmine",
    );
}

#[test]
fn the_error_variable_is_restored_even_when_the_catch_block_itself_fails() {
    assert_output(
        r#"
        err <- "mine"
        TRY
        {
            TRY
            {
                x <- 1 / 0
            } CATCH (err)
            {
                y <- 1 / 0
            }
        } CATCH (outer)
        {
            DISPLAY("inner catch failed")
        }
        DISPLAY(err)
        "#,
        "inner catch failed\nmine",
    );
}

#[test]
fn a_catch_block_without_an_error_variable_still_keeps_its_assignments() {
    assert_output(
        r#"
        TRY
        {
            x <- 1 / 0
        } CATCH
        {
            recovered <- "yes"
        }
        DISPLAY(recovered)
        "#,
        "yes",
    );
}

#[test]
fn a_catch_block_can_update_a_variable_from_the_enclosing_scope() {
    assert_output(
        r#"
        attempts <- 0
        FOR EACH i IN [1, 2, 3]
        {
            TRY
            {
                x <- 1 / 0
            } CATCH (e)
            {
                attempts <- attempts + 1
            }
        }
        DISPLAY(attempts)
        "#,
        "3",
    );
}

#[test]
fn a_catch_block_inside_a_procedure_writes_the_procedure_scope() {
    assert_output(
        r#"
        PROCEDURE safe_divide(a, b)
        {
            TRY
            {
                result <- a / b
            } CATCH (e)
            {
                result <- 0
            }
            RETURN result
        }
        DISPLAY(safe_divide(10, 2))
        DISPLAY(safe_divide(10, 0))
        "#,
        "5\n0",
    );
}

#[test]
fn a_catch_block_that_returns_still_returns() {
    assert_output(
        r#"
        PROCEDURE attempt()
        {
            TRY
            {
                x <- 1 / 0
            } CATCH (e)
            {
                RETURN "from catch"
            }
            RETURN "not reached"
        }
        DISPLAY(attempt())
        "#,
        "from catch",
    );
}

#[test]
fn a_procedure_scope_is_still_private_from_its_caller() {
    // Blocks share the enclosing scope; a *procedure* does not. That distinction
    // has to survive the CATCH change.
    let err = get_error(
        r#"
        PROCEDURE hides()
        {
            secret <- 1
            RETURN 0
        }
        x <- hides()
        DISPLAY(secret)
        "#,
    );
    assert!(err.contains("secret"), "{}", err);
}

// ---------------------------------------------------------------------------
// EXIT unwinds instead of killing the host process
// ---------------------------------------------------------------------------

#[test]
fn exit_in_capture_mode_returns_the_output_so_far() {
    // EXIT called `process::exit` at the point of the call, which took the library
    // caller's whole process with it and threw away everything captured. It now
    // unwinds, and only the CLI turns a status into a process exit code.
    assert_output(
        r#"
        DISPLAY("before")
        EXIT(3)
        DISPLAY("never runs")
        "#,
        "before",
    );
}

#[test]
fn exit_with_no_code_also_returns() {
    assert_output("DISPLAY(\"done\")\nEXIT()", "done");
}

#[test]
fn exit_unwinds_out_of_a_procedure_and_a_loop() {
    assert_output(
        r#"
        PROCEDURE bail()
        {
            DISPLAY("bailing")
            EXIT(1)
        }
        FOR EACH i IN [1, 2, 3]
        {
            DISPLAY(i)
            IF i = 2
            {
                x <- bail()
            }
        }
        DISPLAY("never runs")
        "#,
        "1\n2\nbailing",
    );
}

#[test]
fn a_try_block_does_not_catch_exit() {
    assert_output(
        r#"
        TRY
        {
            DISPLAY("trying")
            EXIT(0)
        } CATCH (e)
        {
            DISPLAY("never runs")
        }
        DISPLAY("never runs either")
        "#,
        "trying",
    );
}

#[test]
fn an_invalid_exit_code_is_still_an_error() {
    let err = get_error("EXIT(256)");
    assert!(err.contains("between 0 and 255"), "{}", err);
}
