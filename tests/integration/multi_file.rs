//! Real multi-file programs on a real filesystem. A relative `IMPORT` follows the
//! importing file and not the working directory, which needs a process whose
//! working directory is somewhere else.

use crate::harness::Program;

#[test]
fn a_program_split_across_directories_runs_from_anywhere() {
    // Launched from `run-from-here/`, importing `../lib/...`. The libraries know
    // nothing about the working directory.
    Program::new(
        r#"
        IMPORT "lib/greet.psl"
        DISPLAY(greet("world"))
        "#,
    )
    .file(
        "lib/greet.psl",
        r#"
        IMPORT "shout.psl"
        PROCEDURE greet(name)
        {
            RETURN CONCAT("hello, ", shout(name))
        }
        "#,
    )
    .file(
        "lib/shout.psl",
        r#"
        PROCEDURE shout(s)
        {
            RETURN UPPERCASE(s)
        }
        "#,
    )
    .working_dir("run-from-here")
    .run()
    .success()
    .stdout_is("hello, WORLD");
}

#[test]
fn an_absolute_entry_path_from_an_unrelated_working_directory() {
    Program::new(
        r#"
        IMPORT "lib/util.psl"
        DISPLAY(util())
        "#,
    )
    .file(
        "lib/util.psl",
        r#"
        PROCEDURE util()
        {
            RETURN "resolved"
        }
        "#,
    )
    .working_dir("somewhere/else/entirely")
    .absolute_entry()
    .run()
    .success()
    .stdout_is("resolved");
}

#[test]
fn a_library_reads_a_data_file_sitting_beside_itself() {
    // The combination the lexical SCRIPTPATH exists for. The working directory is
    // two levels away from both the library and its data.
    Program::new(
        r#"
        IMPORT "lib/table.psl"
        DISPLAY(rows())
        DISPLAY(LENGTH(rows()))
        "#,
    )
    .file(
        "lib/table.psl",
        r#"
        PROCEDURE rows()
        {
            RETURN READLINES(JOINPATH(DIRNAME(SCRIPTPATH()), "rows.txt"))
        }
        "#,
    )
    .file("lib/rows.txt", "alpha\nbeta\ngamma\n")
    .working_dir("deep/nested/cwd")
    .run()
    .success()
    .stdout_is("[alpha, beta, gamma]\n3");
}

#[test]
fn the_ismain_guard_is_quiet_when_imported_and_fires_when_run_directly() {
    let library = r#"
        PROCEDURE useful()
        {
            RETURN "useful"
        }
        IF ISMAIN()
        {
            DISPLAY("self-test running")
        }
        "#;

    // Imported: quiet.
    Program::new(
        r#"
        IMPORT "lib.psl"
        DISPLAY(useful())
        "#,
    )
    .file("lib.psl", library)
    .run()
    .success()
    .stdout_is("useful")
    .stdout_excludes("self-test");

    // Run directly: fires.
    Program::new(library)
        .entry_at("lib.psl")
        .run()
        .success()
        .stdout_is("self-test running");
}

#[test]
fn a_library_that_imports_the_entry_script_does_not_restart_it() {
    // Regression: the entry file used to be re-run from the top, half-way through
    // its own first run.
    Program::new(
        r#"
        IMPORT "helper.psl"
        DISPLAY("entry ran")
        DISPLAY(fromhelper())
        "#,
    )
    .file(
        "helper.psl",
        r#"
        IMPORT "main.psl"
        PROCEDURE fromhelper()
        {
            RETURN "helper"
        }
        "#,
    )
    .run()
    .success()
    .stdout_is("entry ran\nhelper");
}

#[test]
fn a_circular_import_between_two_real_files_terminates() {
    let run = Program::new(
        r#"
        IMPORT "a.psl"
        DISPLAY(fromA())
        "#,
    )
    .file(
        "a.psl",
        r#"
        IMPORT "b.psl"
        PROCEDURE fromA()
        {
            RETURN CONCAT("A+", fromB())
        }
        "#,
    )
    .file(
        "b.psl",
        r#"
        IMPORT "a.psl"
        PROCEDURE fromB()
        {
            RETURN "B"
        }
        "#,
    )
    .run();
    assert!(!run.timed_out, "a circular import did not terminate");
    run.success().stdout_is("A+B");
}

#[test]
fn a_shared_library_runs_its_top_level_once_across_a_diamond() {
    Program::new(
        r#"
        IMPORT "left.psl"
        IMPORT "right.psl"
        DISPLAY("done")
        "#,
    )
    .file("base.psl", r#"DISPLAY("base ran")"#)
    .file("left.psl", r#"IMPORT "base.psl""#)
    .file("right.psl", r#"IMPORT "base.psl""#)
    .run()
    .success()
    .stdout_is("base ran\ndone");
}

#[test]
fn modules_reports_absolute_paths_of_what_was_imported() {
    let run = Program::new(
        r#"
        IMPORT "lib/one.psl"
        IMPORT "lib/two.psl"
        loaded <- MODULES()
        DISPLAY(LENGTH(loaded))
        DISPLAY(BASENAME(loaded[1]))
        DISPLAY(BASENAME(loaded[2]))
        DISPLAY(ISFILE(loaded[1]))
        "#,
    )
    .file("lib/one.psl", "")
    .file("lib/two.psl", "")
    .working_dir("elsewhere")
    .run();
    run.success().stdout_is("2\none.psl\ntwo.psl\ntrue");
}

#[test]
fn a_missing_import_fails_with_the_locations_it_tried() {
    Program::new(r#"IMPORT "nowhere""#)
        .run()
        .code(1)
        .stderr_contains("Could not find imported file 'nowhere'")
        .stderr_contains("Tried:");
}

#[test]
fn a_parse_error_in_a_library_names_that_library_not_the_entry() {
    Program::new(r#"IMPORT "broken.psl""#)
        .file("broken.psl", "PROCEDURE oops(\n")
        .run()
        .code(1)
        .stderr_contains("Failed to parse imported file")
        .stderr_contains("broken.psl");
}

#[test]
fn a_three_deep_import_chain_resolves_each_link_against_its_own_directory() {
    Program::new(
        r#"
        IMPORT "one/a.psl"
        DISPLAY(a())
        "#,
    )
    .file(
        "one/a.psl",
        r#"
        IMPORT "two/b.psl"
        PROCEDURE a()
        {
            RETURN CONCAT("a>", b())
        }
        "#,
    )
    .file(
        "one/two/b.psl",
        r#"
        IMPORT "three/c.psl"
        PROCEDURE b()
        {
            RETURN CONCAT("b>", c())
        }
        "#,
    )
    .file(
        "one/two/three/c.psl",
        r#"
        PROCEDURE c()
        {
            RETURN "c"
        }
        "#,
    )
    .working_dir("unrelated")
    .run()
    .success()
    .stdout_is("a>b>c");
}

#[test]
fn scriptpath_is_the_defining_file_through_a_cross_file_call_chain() {
    Program::new(
        r#"
        IMPORT "mid.psl"
        DISPLAY(mid())
        DISPLAY(BASENAME(SCRIPTPATH()))
        "#,
    )
    .file(
        "mid.psl",
        r#"
        IMPORT "leaf.psl"
        PROCEDURE mid()
        {
            RETURN CONCAT(leaf(), CONCAT(" via ", BASENAME(SCRIPTPATH())))
        }
        "#,
    )
    .file(
        "leaf.psl",
        r#"
        PROCEDURE leaf()
        {
            RETURN BASENAME(SCRIPTPATH())
        }
        "#,
    )
    .run()
    .success()
    .stdout_is("leaf.psl via mid.psl\nmain.psl");
}

#[test]
fn a_library_can_be_imported_by_bare_name_without_the_extension() {
    Program::new(
        r#"
        IMPORT shapes
        DISPLAY(area(3, 4))
        "#,
    )
    .file(
        "shapes.psl",
        r#"
        PROCEDURE area(w, h)
        {
            RETURN w * h
        }
        "#,
    )
    .run()
    .success()
    .stdout_is("12");
}

#[test]
fn program_arguments_reach_a_multi_file_program() {
    Program::new(
        r#"
        IMPORT "args.psl"
        DISPLAY(describe())
        "#,
    )
    .file(
        "args.psl",
        r#"
        PROCEDURE describe()
        {
            RETURN CONCAT(TOSTRING(ARGCOUNT), CONCAT(" ", GETARG("mode", "none")))
        }
        "#,
    )
    .arg("--mode")
    .arg("fast")
    .run()
    .success()
    .stdout_is("2 fast");
}
