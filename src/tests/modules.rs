//! Programs made of several `.psl` files.
//!
//! Every test writes real files into a scratch directory and runs the entry
//! script through [`run_test_at`](super::run_test_at), because the whole point of
//! the resolution rules is that they depend on where the importing file lives.

use super::{Scratch, assert_output_at, get_error_at, run_test_at};

#[test]
fn test_import_brings_in_procedures_and_variables() {
    let scratch = Scratch::new();
    scratch.write(
        "lib.psl",
        r#"
        LIB_NAME <- "the library"
        PROCEDURE double(n)
        {
            RETURN n * 2
        }
        "#,
    );
    let main = scratch.write("main.psl", "");
    assert_output_at(
        r#"
        IMPORT "lib.psl"
        DISPLAY(double(4))
        DISPLAY(LIB_NAME)
        "#,
        &main,
        "8\nthe library",
    );
}

#[test]
fn test_import_resolves_relative_to_the_importing_file_not_the_working_directory() {
    // The entry script lives in `nested/`, so its own IMPORT must find
    // `nested/helper.psl` even though the process was started elsewhere.
    let scratch = Scratch::new();
    scratch.write(
        "nested/helper.psl",
        r#"
        PROCEDURE help()
        {
            RETURN "helped"
        }
        "#,
    );
    let main = scratch.write("nested/main.psl", "");
    assert_output_at(
        r#"
        IMPORT "helper.psl"
        DISPLAY(help())
        "#,
        &main,
        "helped",
    );
}

#[test]
fn test_a_library_can_import_its_own_neighbour() {
    // `lib/outer.psl` imports `inner`, which sits beside it rather than beside
    // the entry script. Resolution has to follow the file being evaluated.
    let scratch = Scratch::new();
    scratch.write(
        "lib/inner.psl",
        r#"
        PROCEDURE inner()
        {
            RETURN "inner"
        }
        "#,
    );
    scratch.write(
        "lib/outer.psl",
        r#"
        IMPORT "inner.psl"
        PROCEDURE outer()
        {
            RETURN CONCAT("outer sees ", inner())
        }
        "#,
    );
    let main = scratch.write("main.psl", "");
    assert_output_at(
        r#"
        IMPORT "lib/outer.psl"
        DISPLAY(outer())
        "#,
        &main,
        "outer sees inner",
    );
}

#[test]
fn test_the_psl_extension_is_optional() {
    let scratch = Scratch::new();
    scratch.write(
        "tools.psl",
        r#"
        PROCEDURE tool()
        {
            RETURN "tooled"
        }
        "#,
    );
    let main = scratch.write("main.psl", "");
    assert_output_at(
        r#"
        IMPORT "tools"
        DISPLAY(tool())
        "#,
        &main,
        "tooled",
    );
}

#[test]
fn test_the_bare_identifier_form_works() {
    // `IMPORT name` is what the guide has always documented.
    let scratch = Scratch::new();
    scratch.write(
        "shapes.psl",
        r#"
        PROCEDURE area(w, h)
        {
            RETURN w * h
        }
        "#,
    );
    let main = scratch.write("main.psl", "");
    assert_output_at(
        r#"
        IMPORT shapes
        DISPLAY(area(3, 4))
        "#,
        &main,
        "12",
    );
}

#[test]
fn test_an_absolute_path_is_used_as_given() {
    let scratch = Scratch::new();
    scratch.write(
        "abs.psl",
        r#"
        PROCEDURE fromabs()
        {
            RETURN "absolute"
        }
        "#,
    );
    let main = scratch.write("main.psl", "");
    assert_output_at(
        &format!(
            r#"
            IMPORT "{p}"
            DISPLAY(fromabs())
            "#,
            p = scratch.psl_path("abs.psl")
        ),
        &main,
        "absolute",
    );
}

#[test]
fn test_a_file_is_imported_at_most_once() {
    // The library's top-level DISPLAY must run exactly once no matter how many
    // times, or by how many spellings, it is imported.
    let scratch = Scratch::new();
    scratch.write("once.psl", r#"DISPLAY("library body ran")"#);
    let main = scratch.write("main.psl", "");
    assert_output_at(
        &format!(
            r#"
            IMPORT "once.psl"
            IMPORT "once.psl"
            IMPORT "once"
            IMPORT once
            IMPORT "{abs}"
            DISPLAY("done")
            "#,
            abs = scratch.psl_path("once.psl")
        ),
        &main,
        "library body ran\ndone",
    );
}

#[test]
fn test_a_diamond_of_imports_runs_the_shared_file_once() {
    let scratch = Scratch::new();
    scratch.write("base.psl", r#"DISPLAY("base ran")"#);
    scratch.write("left.psl", r#"IMPORT "base.psl""#);
    scratch.write("right.psl", r#"IMPORT "base.psl""#);
    let main = scratch.write("main.psl", "");
    assert_output_at(
        r#"
        IMPORT "left.psl"
        IMPORT "right.psl"
        DISPLAY("done")
        "#,
        &main,
        "base ran\ndone",
    );
}

#[test]
fn test_a_circular_import_terminates() {
    // Each file's body runs once; the import that leads back is skipped. Late
    // binding then lets the two files use each other's procedures.
    let scratch = Scratch::new();
    scratch.write(
        "a.psl",
        r#"
        IMPORT "b.psl"
        PROCEDURE fromA()
        {
            RETURN CONCAT("A+", fromB())
        }
        "#,
    );
    scratch.write(
        "b.psl",
        r#"
        IMPORT "a.psl"
        PROCEDURE fromB()
        {
            RETURN "B"
        }
        "#,
    );
    let main = scratch.write("main.psl", "");
    assert_output_at(
        r#"
        IMPORT "a.psl"
        DISPLAY(fromA())
        "#,
        &main,
        "A+B",
    );
}

#[test]
fn test_a_self_import_terminates() {
    let scratch = Scratch::new();
    scratch.write(
        "selfie.psl",
        r#"
        IMPORT "selfie.psl"
        PROCEDURE ok()
        {
            RETURN "ok"
        }
        "#,
    );
    let main = scratch.write("main.psl", "");
    assert_output_at(
        r#"
        IMPORT "selfie.psl"
        DISPLAY(ok())
        "#,
        &main,
        "ok",
    );
}

#[test]
fn test_a_library_importing_the_entry_script_does_not_restart_it() {
    // The entry script counts as already running. Without that, `main` -> `lib`
    // -> `main` re-ran the entry file's top level part-way through the first run,
    // before the declarations it depends on existed.
    let scratch = Scratch::new();
    scratch.write(
        "helper.psl",
        r#"
        IMPORT "main.psl"
        PROCEDURE fromhelper()
        {
            RETURN "helper"
        }
        "#,
    );
    let main = scratch.write("main.psl", "");
    assert_output_at(
        r#"
        IMPORT "helper.psl"
        DISPLAY("entry body ran")
        DISPLAY(fromhelper())
        DISPLAY(ISMAIN())
        "#,
        &main,
        "entry body ran\nhelper\ntrue",
    );
}

#[test]
fn test_an_ismain_block_in_a_directly_run_library_sees_its_own_declarations() {
    // The shape that caught the bug: the library imports a sibling, the sibling
    // imports it back, and the library's ISMAIN block uses both files' procedures.
    let scratch = Scratch::new();
    scratch.write(
        "sibling.psl",
        r#"
        IMPORT "lib.psl"
        PROCEDURE shout(s)
        {
            RETURN UPPERCASE(s)
        }
        "#,
    );
    let lib = scratch.write("lib.psl", "");
    assert_output_at(
        r#"
        IMPORT "sibling.psl"
        PROCEDURE greet()
        {
            RETURN shout("hi")
        }
        IF ISMAIN()
        {
            DISPLAY(greet())
        }
        "#,
        &lib,
        "HI",
    );
}

#[test]
fn test_the_entry_script_is_recognised_through_a_different_spelling() {
    // Resolution canonicalises, and the entry is canonicalised the same way, so a
    // relative spelling of the entry script is still recognised as itself.
    let scratch = Scratch::new();
    scratch.write(
        "side.psl",
        r#"
        IMPORT "./main.psl"
        PROCEDURE side()
        {
            RETURN "side"
        }
        "#,
    );
    let main = scratch.write("main.psl", "");
    assert_output_at(
        r#"
        IMPORT "side.psl"
        DISPLAY(side())
        DISPLAY(LENGTH(MODULES()))
        "#,
        &main,
        "side\n1",
    );
}

#[test]
fn test_an_import_inside_a_procedure_declares_visibly_and_permanently() {
    // The declarations used to land in the procedure's own scope and vanish on the
    // way out, while the file stayed marked loaded -- so a later top-level IMPORT of
    // it was a silent no-op and its names were unreachable for the rest of the run.
    let scratch = Scratch::new();
    scratch.write(
        "late.psl",
        r#"
        PROCEDURE fromlate()
        {
            RETURN "late"
        }
        "#,
    );
    let main = scratch.write("main.psl", "");
    assert_output_at(
        r#"
        PROCEDURE loadit()
        {
            IMPORT "late.psl"
            RETURN fromlate()
        }
        DISPLAY(loadit())
        DISPLAY(CONTAINS(PROCEDURES(), "fromlate"))
        DISPLAY(fromlate())
        "#,
        &main,
        "late\ntrue\nlate",
    );
}

#[test]
fn test_an_import_inside_a_catch_block_declares_visibly() {
    let scratch = Scratch::new();
    scratch.write(
        "fallback.psl",
        r#"
        PROCEDURE fallback()
        {
            RETURN "fallback"
        }
        "#,
    );
    let main = scratch.write("main.psl", "");
    assert_output_at(
        r#"
        TRY
        {
            x <- 1 / 0
        } CATCH (e)
        {
            IMPORT "fallback.psl"
        }
        DISPLAY(fallback())
        "#,
        &main,
        "fallback",
    );
}

#[test]
fn test_a_module_whose_body_failed_is_not_left_marked_loaded() {
    // Recording it before the body ran made the failure permanent: a retry did
    // nothing, and MODULES() listed a file that never finished.
    let scratch = Scratch::new();
    scratch.write("boom.psl", "DISPLAY(\"body ran\")\nx <- 1 / 0");
    let main = scratch.write("main.psl", "");
    assert_output_at(
        r#"
        TRY
        {
            IMPORT "boom.psl"
        } CATCH (e)
        {
            DISPLAY("first attempt failed")
        }
        DISPLAY(LENGTH(MODULES()))
        TRY
        {
            IMPORT "boom.psl"
        } CATCH (e)
        {
            DISPLAY("second attempt failed too")
        }
        "#,
        &main,
        "body ran\nfirst attempt failed\n0\nbody ran\nsecond attempt failed too",
    );
}

#[test]
fn test_a_failure_inside_an_imported_file_is_reported_against_that_file() {
    // The span belongs to the imported file, but the error used to be formatted
    // against the entry script, so the caret pointed at unrelated text.
    let scratch = Scratch::new();
    scratch.write(
        "thrower.psl",
        "PROCEDURE ok()\n{\n    RETURN 1\n}\nbad <- 1 / 0\n",
    );
    let main = scratch.write("main.psl", "");
    let err = get_error_at(r#"IMPORT "thrower.psl""#, &main);
    assert!(err.contains("thrower.psl"), "{}", err);
    assert!(err.contains("Division by zero"), "{}", err);
    // The offending line from the imported file, at its own line number, not the
    // entry script's text at whatever offset the span happened to land on.
    assert!(err.contains("bad <- 1 / 0"), "{}", err);
    assert!(err.contains("Line 5"), "{}", err);
    // Reported once, not nested once per level of import.
    assert_eq!(err.matches("Division by zero").count(), 1, "{}", err);
}

#[test]
fn test_a_failure_in_a_nested_import_is_reported_once_against_the_innermost_file() {
    let scratch = Scratch::new();
    scratch.write("inner.psl", "boom <- 1 / 0\n");
    scratch.write("mid.psl", "IMPORT \"inner.psl\"\n");
    let main = scratch.write("main.psl", "");
    let err = get_error_at(r#"IMPORT "mid.psl""#, &main);
    assert!(err.contains("inner.psl"), "{}", err);
    assert!(err.contains("boom <- 1 / 0"), "{}", err);
    assert_eq!(err.matches("Division by zero").count(), 1, "{}", err);
    // The intermediate file is not part of the diagnostic.
    assert!(!err.contains("mid.psl"), "{}", err);
}

#[test]
fn test_an_error_inside_an_imported_procedure_is_reported_against_its_own_file() {
    // The call happens long after the IMPORT returned, so nothing is on the import
    // stack; the span still belongs to the library, and used to be resolved against
    // the entry script, pointing the caret at unrelated text.
    let scratch = Scratch::new();
    scratch.write("lib.psl", "PROCEDURE boom()\n{\n    RETURN 1 / 0\n}\n");
    let main = scratch.write("main.psl", "");
    let err = get_error_at(
        r#"
        IMPORT "lib.psl"
        DISPLAY(boom())
        "#,
        &main,
    );
    assert!(err.contains("lib.psl"), "{}", err);
    assert!(err.contains("RETURN 1 / 0"), "{}", err);
    assert!(err.contains("Line 3"), "{}", err);
}

#[test]
fn test_a_top_level_return_ends_the_imported_file_not_the_program() {
    // A RETURN at an imported file's top level used to unwind the *importing*
    // program: everything after the IMPORT was skipped and the run ended silently.
    let scratch = Scratch::new();
    scratch.write(
        "early.psl",
        "DISPLAY(\"library body\")\nRETURN 1\nDISPLAY(\"not reached\")",
    );
    let main = scratch.write("main.psl", "");
    assert_output_at(
        r#"
        DISPLAY("before")
        IMPORT "early.psl"
        DISPLAY("after")
        "#,
        &main,
        "before\nlibrary body\nafter",
    );
}

#[test]
fn test_a_procedure_declared_in_a_nested_scope_is_still_private_to_it() {
    // Merging the imported table down the chain must not make an ordinary nested
    // PROCEDURE declaration escape its scope.
    let scratch = Scratch::new();
    let main = scratch.write("main.psl", "");
    assert_output_at(
        r#"
        PROCEDURE outer()
        {
            PROCEDURE hidden()
            {
                RETURN "hidden"
            }
            RETURN hidden()
        }
        DISPLAY(outer())
        DISPLAY(CONTAINS(PROCEDURES(), "hidden"))
        "#,
        &main,
        "hidden\nfalse",
    );
}

#[test]
fn test_a_missing_import_lists_what_was_tried() {
    let scratch = Scratch::new();
    let main = scratch.write("main.psl", "");
    let err = get_error_at(r#"IMPORT "nowhere""#, &main);
    assert!(
        err.contains("Could not find imported file 'nowhere'"),
        "{}",
        err
    );
    assert!(err.contains("Tried:"), "{}", err);
    // Both the .psl-appended candidate and the bare one should be mentioned.
    assert!(err.contains("nowhere.psl"), "{}", err);
}

#[test]
fn test_a_syntax_error_in_an_imported_file_names_that_file() {
    let scratch = Scratch::new();
    scratch.write("broken.psl", "PROCEDURE oops(\n");
    let main = scratch.write("main.psl", "");
    let err = get_error_at(r#"IMPORT "broken.psl""#, &main);
    assert!(err.contains("Failed to parse imported file"), "{}", err);
    assert!(err.contains("broken.psl"), "{}", err);
}

#[test]
fn test_a_runtime_error_in_an_imported_file_is_catchable() {
    let scratch = Scratch::new();
    scratch.write("bad.psl", "x <- 1 / 0");
    let main = scratch.write("main.psl", "");
    let output = run_test_at(
        r#"
        TRY
        {
            IMPORT "bad.psl"
            DISPLAY("not reached")
        } CATCH (err)
        {
            DISPLAY("caught")
        }
        "#,
        &main,
    )
    .expect("TRY/CATCH around IMPORT");
    assert_eq!(output, "caught");
}

#[test]
fn test_a_failed_import_can_be_retried_after_the_file_appears() {
    // A file that failed to resolve was never recorded as loaded, so a later
    // IMPORT of the same name must still work.
    let scratch = Scratch::new();
    let main = scratch.write("main.psl", "");
    let later = scratch.psl_path("later.psl");
    let output = run_test_at(
        &format!(
            r#"
            TRY
            {{
                IMPORT "later"
            }} CATCH (err)
            {{
                DISPLAY("first attempt failed")
            }}
            WRITEFILE("{p}", "PROCEDURE ready()" + "\n" + "{{" + "\n" + "RETURN \"ready\"" + "\n" + "}}")
            IMPORT "later"
            DISPLAY(ready())
            "#,
            p = later
        ),
        &main,
    )
    .expect("retry after the file exists");
    assert_eq!(output, "first attempt failed\nready");
}

// ---------------------------------------------------------------------------
// SCRIPTPATH, ISMAIN, MODULES
// ---------------------------------------------------------------------------

#[test]
fn test_scriptpath_reports_the_entry_file() {
    let scratch = Scratch::new();
    let main = scratch.write("entry.psl", "");
    assert_output_at("DISPLAY(BASENAME(SCRIPTPATH()))", &main, "entry.psl");
}

#[test]
fn test_scriptpath_is_absolute() {
    let scratch = Scratch::new();
    let main = scratch.write("entry.psl", "");
    let output = run_test_at("DISPLAY(SCRIPTPATH())", &main).expect("scriptpath");
    assert!(
        std::path::Path::new(&output).is_absolute(),
        "not absolute: {}",
        output
    );
}

#[test]
fn test_scriptpath_inside_an_import_reports_the_imported_file() {
    let scratch = Scratch::new();
    scratch.write("inner.psl", "DISPLAY(BASENAME(SCRIPTPATH()))");
    let main = scratch.write("outer.psl", "");
    assert_output_at(
        r#"
        IMPORT "inner.psl"
        DISPLAY(BASENAME(SCRIPTPATH()))
        "#,
        &main,
        "inner.psl\nouter.psl",
    );
}

#[test]
fn test_scriptpath_is_null_when_there_is_no_file() {
    // The library API and the browser playground run source with no location.
    super::assert_output("DISPLAY(SCRIPTPATH())", "NULL");
}

#[test]
fn test_ismain_is_true_only_in_the_entry_script() {
    let scratch = Scratch::new();
    scratch.write(
        "libmain.psl",
        r#"
        DISPLAY(ISMAIN())
        IF ISMAIN()
        {
            DISPLAY("library self-test")
        }
        "#,
    );
    let main = scratch.write("main.psl", "");
    assert_output_at(
        r#"
        IMPORT "libmain.psl"
        DISPLAY(ISMAIN())
        "#,
        &main,
        "false\ntrue",
    );
}

#[test]
fn test_ismain_guard_lets_a_library_carry_a_demo() {
    let scratch = Scratch::new();
    scratch.write(
        "demo.psl",
        r#"
        PROCEDURE useful()
        {
            RETURN "useful"
        }
        IF ISMAIN()
        {
            DISPLAY("running the demo")
        }
        "#,
    );
    let main = scratch.write("main.psl", "");
    // Imported: the demo stays quiet.
    assert_output_at(
        r#"
        IMPORT "demo.psl"
        DISPLAY(useful())
        "#,
        &main,
        "useful",
    );
    // Run directly: the demo fires.
    let demo = scratch.path("demo.psl");
    assert_output_at(
        r#"
        PROCEDURE useful()
        {
            RETURN "useful"
        }
        IF ISMAIN()
        {
            DISPLAY("running the demo")
        }
        "#,
        &demo,
        "running the demo",
    );
}

#[test]
fn test_ismain_is_false_when_there_is_no_entry_file() {
    super::assert_output("DISPLAY(ISMAIN())", "false");
}

#[test]
fn test_ismain_is_true_inside_a_procedure_called_from_the_entry_script() {
    // ISMAIN is about which *file* is running, not about call depth.
    let scratch = Scratch::new();
    let main = scratch.write("main.psl", "");
    assert_output_at(
        r#"
        PROCEDURE check()
        {
            RETURN ISMAIN()
        }
        DISPLAY(check())
        "#,
        &main,
        "true",
    );
}

#[test]
fn test_modules_lists_imported_files_in_import_order() {
    let scratch = Scratch::new();
    scratch.write("first.psl", "");
    scratch.write("second.psl", "");
    let main = scratch.write("main.psl", "");
    assert_output_at(
        r#"
        IMPORT "first.psl"
        IMPORT "second.psl"
        loaded <- MODULES()
        DISPLAY(LENGTH(loaded))
        DISPLAY(BASENAME(loaded[1]))
        DISPLAY(BASENAME(loaded[2]))
        "#,
        &main,
        "2\nfirst.psl\nsecond.psl",
    );
}

#[test]
fn test_modules_does_not_list_the_entry_script_and_is_empty_without_imports() {
    let scratch = Scratch::new();
    let main = scratch.write("main.psl", "");
    assert_output_at("DISPLAY(MODULES())", &main, "[]");
}

#[test]
fn test_modules_counts_a_repeated_import_once() {
    let scratch = Scratch::new();
    scratch.write("dup.psl", "");
    let main = scratch.write("main.psl", "");
    assert_output_at(
        r#"
        IMPORT "dup.psl"
        IMPORT "dup"
        DISPLAY(LENGTH(MODULES()))
        "#,
        &main,
        "1",
    );
}

#[test]
fn test_scriptpath_is_lexical_inside_a_procedure() {
    // While a procedure runs, SCRIPTPATH reports the file the procedure was
    // *written* in, not the file that called it -- the same rule as Python's
    // `__file__`, and the only rule under which a library can find its own files.
    let scratch = Scratch::new();
    scratch.write(
        "where.psl",
        r#"
        PROCEDURE whereami()
        {
            RETURN BASENAME(SCRIPTPATH())
        }
        "#,
    );
    let main = scratch.write("caller.psl", "");
    assert_output_at(
        r#"
        IMPORT "where.psl"
        DISPLAY(whereami())
        DISPLAY(BASENAME(SCRIPTPATH()))
        "#,
        &main,
        "where.psl\ncaller.psl",
    );
}

#[test]
fn test_scriptpath_returns_to_the_caller_after_a_cross_file_call() {
    // The file stack has to unwind on the way out, including through nesting.
    let scratch = Scratch::new();
    scratch.write(
        "inner.psl",
        r#"
        PROCEDURE inner()
        {
            RETURN BASENAME(SCRIPTPATH())
        }
        "#,
    );
    scratch.write(
        "middle.psl",
        r#"
        IMPORT "inner.psl"
        PROCEDURE middle()
        {
            RETURN CONCAT(inner(), CONCAT(" then ", BASENAME(SCRIPTPATH())))
        }
        "#,
    );
    let main = scratch.write("main.psl", "");
    assert_output_at(
        r#"
        IMPORT "middle.psl"
        DISPLAY(middle())
        DISPLAY(BASENAME(SCRIPTPATH()))
        "#,
        &main,
        "inner.psl then middle.psl\nmain.psl",
    );
}

#[test]
fn test_ismain_is_false_inside_an_imported_procedure() {
    let scratch = Scratch::new();
    scratch.write(
        "libcheck.psl",
        r#"
        PROCEDURE amimain()
        {
            RETURN ISMAIN()
        }
        "#,
    );
    let main = scratch.write("main.psl", "");
    assert_output_at(
        r#"
        IMPORT "libcheck.psl"
        DISPLAY(amimain())
        DISPLAY(ISMAIN())
        "#,
        &main,
        "false\ntrue",
    );
}

#[test]
fn test_an_import_inside_a_procedure_resolves_against_the_defining_file() {
    let scratch = Scratch::new();
    scratch.write(
        "deps/leaf.psl",
        r#"
        PROCEDURE leaf()
        {
            RETURN "leaf"
        }
        "#,
    );
    scratch.write(
        "deps/lazy.psl",
        r#"
        PROCEDURE loadleaf()
        {
            IMPORT "leaf.psl"
            RETURN leaf()
        }
        "#,
    );
    let main = scratch.write("main.psl", "");
    assert_output_at(
        r#"
        IMPORT "deps/lazy.psl"
        DISPLAY(loadleaf())
        "#,
        &main,
        "leaf",
    );
}

#[test]
fn test_a_library_can_locate_its_own_data_file() {
    // The combination the module bookkeeping exists for: a library reading a file
    // that sits next to it, wherever the program was started from.
    let scratch = Scratch::new();
    scratch.write("data/values.txt", "7\n");
    scratch.write(
        "data/reader.psl",
        r#"
        PROCEDURE readvalue()
        {
            RETURN TONUM(TRIM(READFILE(JOINPATH(DIRNAME(SCRIPTPATH()), "values.txt"))))
        }
        "#,
    );
    let main = scratch.write("main.psl", "");
    assert_output_at(
        r#"
        IMPORT "data/reader.psl"
        DISPLAY(readvalue() * 6)
        "#,
        &main,
        "42",
    );
}

#[test]
fn test_module_builtins_take_no_arguments() {
    let scratch = Scratch::new();
    let main = scratch.write("main.psl", "");
    for call in ["SCRIPTPATH(1)", "ISMAIN(1)", "MODULES(1)"] {
        let err = get_error_at(&format!("DISPLAY({})", call), &main);
        assert!(err.contains("takes no arguments"), "{}: {}", call, err);
    }
}

#[test]
fn test_a_caught_import_failure_reports_the_plain_message() {
    // The error variable holds the failure, not a rendered multi-line diagnostic.
    let scratch = Scratch::new();
    scratch.write("bad.psl", "x <- 1 / 0");
    let main = scratch.write("main.psl", "");
    assert_output_at(
        r#"
        TRY
        {
            IMPORT "bad.psl"
        } CATCH (e)
        {
            DISPLAY(e)
        }
        "#,
        &main,
        "Division by zero",
    );
}

#[test]
fn test_retrying_a_failed_import_reruns_its_top_level() {
    // The cost of allowing a retry at all, and what Python does: a module that
    // raised is forgotten, so importing it again runs it from the top.
    let scratch = Scratch::new();
    scratch.write("partial.psl", "APPEND(log, \"ran\")\nx <- 1 / 0");
    let main = scratch.write("main.psl", "");
    assert_output_at(
        r#"
        log <- []
        TRY
        {
            IMPORT "partial.psl"
        } CATCH (e)
        {
            DISPLAY("first failed")
        }
        TRY
        {
            IMPORT "partial.psl"
        } CATCH (e)
        {
            DISPLAY("second failed")
        }
        DISPLAY(LENGTH(log))
        "#,
        &main,
        "first failed\nsecond failed\n2",
    );
}
