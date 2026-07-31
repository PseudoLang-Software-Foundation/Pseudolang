//! Process-global state: the working directory, the environment, and real child
//! processes. `CHDIR` and `SETENV` mutate state the whole process shares, and
//! `cargo test` uses parallel threads, so each test owns its own process.

use crate::harness::Program;

// ---------------------------------------------------------------------------
// Working directory
// ---------------------------------------------------------------------------

#[test]
fn chdir_changes_where_relative_paths_resolve() {
    let run = Program::new(
        r#"
        MAKEDIR("sub")
        CHDIR("sub")
        WRITEFILE("inside.txt", "written after chdir")
        DISPLAY(BASENAME(CWD()))
        DISPLAY(ISFILE("inside.txt"))
        DISPLAY(ISFILE(JOINPATH("..", "inside.txt")))
        "#,
    )
    .run();
    run.success().stdout_is("sub\ntrue\nfalse");
    // The file really landed in the subdirectory, not next to the script.
    assert_eq!(run.file("sub/inside.txt"), "written after chdir");
    assert!(!run.file_exists("inside.txt"));
}

#[test]
fn cwd_starts_at_the_directory_fpli_was_run_from() {
    Program::new(
        r#"
        DISPLAY(ISFILE("main.psl"))
        DISPLAY(ISDIR(CWD()))
        "#,
    )
    .run()
    .success()
    .stdout_is("true\ntrue");
}

#[test]
fn chdir_to_a_missing_directory_is_an_error_and_leaves_the_cwd_alone() {
    Program::new(
        r#"
        before <- CWD()
        TRY
        {
            CHDIR("no-such-directory")
            DISPLAY("should not reach")
        } CATCH (err)
        {
            DISPLAY("refused")
        }
        DISPLAY(CWD() = before)
        "#,
    )
    .run()
    .success()
    .stdout_is("refused\ntrue");
}

#[test]
fn chdir_does_not_move_scriptpath() {
    // SCRIPTPATH is absolute, so changing directory must not invalidate it.
    Program::new(
        r#"
        MAKEDIR("elsewhere")
        before <- SCRIPTPATH()
        CHDIR("elsewhere")
        DISPLAY(SCRIPTPATH() = before)
        DISPLAY(ISFILE(SCRIPTPATH()))
        "#,
    )
    .run()
    .success()
    .stdout_is("true\ntrue");
}

#[test]
fn an_import_after_a_chdir_still_resolves_against_the_script() {
    // Resolution is relative to the importing *file*, so moving the working
    // directory out from under the program must not break it.
    Program::new(
        r#"
        MAKEDIR("elsewhere")
        CHDIR("elsewhere")
        IMPORT "lib.psl"
        DISPLAY(fromlib())
        "#,
    )
    .file(
        "lib.psl",
        r#"
        PROCEDURE fromlib()
        {
            RETURN "found after chdir"
        }
        "#,
    )
    .run()
    .success()
    .stdout_is("found after chdir");
}

// ---------------------------------------------------------------------------
// Environment
// ---------------------------------------------------------------------------

#[test]
fn getenv_reads_a_variable_the_parent_set() {
    Program::new(r#"DISPLAY(GETENV("PSL_FROM_PARENT"))"#)
        .env("PSL_FROM_PARENT", "inherited value")
        .run()
        .success()
        .stdout_is("inherited value");
}

#[test]
fn getenv_on_a_variable_removed_from_the_environment_uses_the_default() {
    Program::new(r#"DISPLAY(GETENV("PSL_REMOVED", "fallback"))"#)
        .without_env("PSL_REMOVED")
        .run()
        .success()
        .stdout_is("fallback");
}

#[test]
fn setenv_is_visible_to_a_child_process() {
    // The whole point of writing the real process environment rather than a
    // private map: children inherit it.
    let read_it = if cfg!(target_os = "windows") {
        r#"SHELL("echo %PSL_TO_CHILD%")"#
    } else {
        r#"SHELL("printf %s \"$PSL_TO_CHILD\"")"#
    };
    Program::new(&format!(
        r#"
        SETENV("PSL_TO_CHILD", "passed down")
        r <- {}
        DISPLAY(TRIM(r["stdout"]))
        "#,
        read_it
    ))
    .run()
    .success()
    .stdout_is("passed down");
}

#[test]
fn unsetenv_removes_a_variable_from_a_child_too() {
    let read_it = if cfg!(target_os = "windows") {
        // cmd leaves an undefined variable as the literal text.
        r#"SHELL("echo %PSL_GONE%")"#
    } else {
        r#"SHELL("printf %s \"$PSL_GONE\"")"#
    };
    let run = Program::new(&format!(
        r#"
        SETENV("PSL_GONE", "here")
        UNSETENV("PSL_GONE")
        r <- {}
        DISPLAY(CONCAT("child saw: [", CONCAT(TRIM(r["stdout"]), "]")))
        DISPLAY(GETENV("PSL_GONE", "unset in parent"))
        "#,
        read_it
    ))
    .env("PSL_GONE", "set by the parent")
    .run();
    run.success();
    let lines = run.lines();
    if cfg!(target_os = "windows") {
        assert!(
            lines[0].contains("PSL_GONE") || lines[0] == "child saw: []",
            "unexpected: {:?}",
            lines
        );
    } else {
        assert_eq!(lines[0], "child saw: []");
    }
    assert_eq!(lines[1], "unset in parent");
}

#[test]
fn setenv_get_and_unsetenv_round_trip() {
    Program::new(
        r#"
        SETENV("PSL_RT", "value one")
        DISPLAY(GETENV("PSL_RT"))
        SETENV("PSL_RT", "value two")
        DISPLAY(GETENV("PSL_RT"))
        UNSETENV("PSL_RT")
        DISPLAY(GETENV("PSL_RT", "absent"))
        "#,
    )
    .without_env("PSL_RT")
    .run()
    .success()
    .stdout_is("value one\nvalue two\nabsent");
}

#[test]
fn a_getenv_default_is_used_only_while_the_variable_is_missing() {
    Program::new(
        r#"
        DISPLAY(GETENV("PSL_DEF", "fallback"))
        SETENV("PSL_DEF", "real")
        DISPLAY(GETENV("PSL_DEF", "fallback"))
        "#,
    )
    .without_env("PSL_DEF")
    .run()
    .success()
    .stdout_is("fallback\nreal");
}

#[test]
fn unsetenv_on_a_variable_that_was_never_set_is_not_an_error() {
    Program::new(
        r#"
        UNSETENV("PSL_NEVER_EXISTED")
        DISPLAY("ok")
        "#,
    )
    .run()
    .success()
    .stdout_is("ok");
}

#[test]
fn envvars_contains_what_the_program_set() {
    Program::new(
        r#"
        SETENV("PSL_IN_LISTING", "here")
        all <- ENVVARS()
        DISPLAY(TYPEOF(all))
        DISPLAY(CONTAINS(all, "PSL_IN_LISTING"))
        DISPLAY(all["PSL_IN_LISTING"])
        "#,
    )
    .run()
    .success()
    .stdout_is("dictionary\ntrue\nhere");
}

#[test]
fn envvars_survives_a_variable_that_is_not_valid_unicode() {
    // `std::env::vars` panics on one of these, which would abort the interpreter
    // over a variable the program never asked about. The undecodable entry is
    // skipped instead.
    let program = Program::new(
        r#"
        DISPLAY(TYPEOF(ENVVARS()))
        DISPLAY(LENGTH(ENVVARS()) > 0)
        DISPLAY(GETENV("PSL_FINE"))
        "#,
    )
    .env("PSL_FINE", "readable");
    // A lone 0xFF byte is not valid UTF-8 anywhere. Only reachable on unix, where an
    // environment variable is arbitrary bytes; Windows stores UTF-16.
    #[cfg(unix)]
    let program = {
        use std::ffi::OsString;
        use std::os::unix::ffi::OsStringExt;
        program.raw_env("PSL_NOT_UTF8", OsString::from_vec(vec![0xff, 0xfe]))
    };
    program
        .run()
        .success()
        .stdout_is("dictionary\ntrue\nreadable");
}

#[test]
fn envvars_reflects_a_variable_the_program_set_and_then_removed() {
    Program::new(
        r#"
        SETENV("PSL_LISTED", "yes")
        DISPLAY(CONTAINS(ENVVARS(), "PSL_LISTED"))
        UNSETENV("PSL_LISTED")
        DISPLAY(CONTAINS(ENVVARS(), "PSL_LISTED"))
        "#,
    )
    .run()
    .success()
    .stdout_is("true\nfalse");
}

#[test]
fn envvars_is_sorted_by_name() {
    Program::new(
        r#"
        SETENV("PSL_AAA", "1")
        SETENV("PSL_ZZZ", "2")
        names <- KEYS(ENVVARS())
        sorted <- TRUE
        i <- 1
        REPEAT UNTIL (i >= LENGTH(names))
        {
            IF names[i] > names[i + 1]
            {
                sorted <- FALSE
            }
            i <- i + 1
        }
        DISPLAY(sorted)
        "#,
    )
    .run()
    .success()
    .stdout_is("true");
}

// ---------------------------------------------------------------------------
// Child processes
// ---------------------------------------------------------------------------

#[test]
fn exec_runs_a_program_and_reports_its_status() {
    let program = if cfg!(target_os = "windows") {
        r#"EXEC("cmd", ["/C", "exit 3"])"#
    } else {
        r#"EXEC("sh", ["-c", "exit 3"])"#
    };
    Program::new(&format!(
        r#"
        r <- {}
        DISPLAY(r["exitcode"])
        "#,
        program
    ))
    .run()
    .success()
    .stdout_is("3");
}

#[test]
fn a_child_writing_a_lot_of_output_is_captured_whole() {
    // Well past a pipe buffer (~64 KiB), which is where an implementation that
    // waited for the child before draining its output would deadlock.
    //
    // The bulk is produced by PseudoLang itself and then echoed back by the one
    // command that means the same thing on all three platforms, so the test does
    // not depend on shell loop syntax.
    let dump = if cfg!(target_os = "windows") {
        r#"EXEC("cmd", ["/C", "type big.txt"])"#
    } else {
        r#"EXEC("cat", ["big.txt"])"#
    };
    Program::new(&format!(
        r#"
        row <- "0123456789012345678901234567890123456789"
        text <- ""
        REPEAT 5000 TIMES
        {{
            text <- text + row + "\n"
        }}
        WRITEFILE("big.txt", text)

        r <- {}
        DISPLAY(r["exitcode"])
        DISPLAY(LENGTH(SPLIT(TRIM(r["stdout"]), "\n")))
        "#,
        dump
    ))
    .run()
    .success()
    .stdout_is("0\n5000");
}

#[test]
fn a_child_reading_stdin_sees_a_closed_pipe_rather_than_hanging() {
    // `Command::output` gives the child a null stdin, so a program that reads must
    // get EOF. If it inherited ours it could block for ever.
    if cfg!(target_os = "windows") {
        return;
    }
    let run = Program::new(
        r#"
        r <- EXEC("cat")
        DISPLAY(CONCAT("[", CONCAT(r["stdout"], "]")))
        DISPLAY(r["exitcode"])
        "#,
    )
    .run();
    assert!(!run.timed_out, "EXEC hung on a child reading stdin");
    run.success().stdout_is("[]\n0");
}

#[test]
fn pid_is_this_process_and_processinfo_agrees() {
    Program::new(
        r#"
        me <- PROCESSINFO(PID())
        DISPLAY(me["pid"] = PID())
        DISPLAY(me["memory"] > 0)
        DISPLAY(TYPEOF(me["name"]))
        "#,
    )
    .run()
    .success()
    .stdout_is("true\ntrue\nstring");
}

#[test]
fn processinfo_reports_our_real_parent() {
    // The test binary is the parent, so the child must see a parent pid that is
    // not its own and that exists.
    Program::new(
        r#"
        me <- PROCESSINFO(PID())
        DISPLAY(me["parent"] NOT= NULL)
        DISPLAY(me["parent"] NOT= PID())
        "#,
    )
    .run()
    .success()
    .stdout_is("true\ntrue");
}

#[test]
fn kill_terminates_a_real_child_process() {
    // Needs a genuine long-running process to kill, which only a separate process
    // can safely provide. POSIX only: getting a background pid out of cmd.exe is
    // a different exercise and is covered by the refusal tests instead.
    if cfg!(target_os = "windows") {
        return;
    }
    // The background child's stdout must be redirected away from the pipe SHELL
    // is reading: an inherited descriptor keeps the pipe open, so SHELL would wait
    // out the whole `sleep` instead of returning as soon as `sh` exits.
    Program::new(
        r#"
        r <- SHELL("sleep 30 >/dev/null 2>&1 & echo $!")
        child <- TONUM(TRIM(r["stdout"]))
        DISPLAY(PROCESSINFO(child) NOT= NULL)
        DISPLAY(KILL(child))
        SLEEP(0.3)
        DISPLAY(PROCESSINFO(child) = NULL)
        "#,
    )
    .run()
    .success()
    .stdout_is("true\ntrue\ntrue");
}

#[test]
fn kill_refuses_to_kill_the_interpreter_itself() {
    // If this were ever allowed the exit status would be a signal death rather
    // than the clean error the test asserts.
    Program::new(
        r#"
        TRY
        {
            DISPLAY(KILL(PID()))
        } CATCH (err)
        {
            DISPLAY("refused")
        }
        DISPLAY("still alive")
        "#,
    )
    .run()
    .success()
    .stdout_is("refused\nstill alive");
}

#[test]
fn processes_includes_this_process() {
    Program::new(
        r#"
        found <- FALSE
        FOR EACH p IN PROCESSES()
        {
            IF p["pid"] = PID()
            {
                found <- TRUE
            }
        }
        DISPLAY(found)
        "#,
    )
    .run()
    .success()
    .stdout_is("true");
}

#[test]
fn which_finds_the_shell_that_shell_uses() {
    let shell = if cfg!(target_os = "windows") {
        "cmd"
    } else {
        "sh"
    };
    Program::new(&format!(
        r#"
        found <- WHICH("{}")
        DISPLAY(found NOT= NULL)
        DISPLAY(ISFILE(found))
        "#,
        shell
    ))
    .run()
    .success()
    .stdout_is("true\ntrue");
}
