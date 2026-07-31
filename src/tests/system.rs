//! System integration: environment variables, child processes, process
//! management, paths and machine facts. Read-only against the host, and asserting
//! only what holds on all three platforms.
//!
//! Every test that *writes* the process environment or the working directory is in
//! `tests/integration/process_state.rs` instead, one process each:
//! `std::env::set_var` is unsafe because of concurrent readers, and this binary runs
//! its tests on parallel threads.

use super::{Scratch, assert_output, get_error, run_test};

// ---------------------------------------------------------------------------
// Environment variables
// ---------------------------------------------------------------------------

#[test]
fn test_getenv_default_may_be_any_value() {
    assert_output(
        r#"
        DISPLAY(GETENV("PSL_TEST_NEVER_SET_ANYDEFAULT", 0))
        DISPLAY(TYPEOF(GETENV("PSL_TEST_NEVER_SET_ANYDEFAULT", NULL)))
        "#,
        "0\nnull",
    );
}

#[test]
fn test_getenv_without_a_default_errors_when_missing() {
    let err = get_error("DISPLAY(GETENV(\"PSL_TEST_DEFINITELY_MISSING\"))");
    assert!(err.contains("is not set"), "{}", err);
    assert!(err.contains("default"), "{}", err);
}

#[test]
fn test_env_var_names_are_validated() {
    let err = get_error("SETENV(\"BAD=NAME\", \"x\")");
    assert!(
        err.contains("not a usable environment variable name"),
        "{}",
        err
    );

    let err = get_error("SETENV(\"\", \"x\")");
    assert!(
        err.contains("not a usable environment variable name"),
        "{}",
        err
    );
}

#[test]
fn test_exec_returns_stdout_and_exit_code() {
    // The point is the *shape* of the result, which is the same whichever program
    // runs, so pick one that exists on the platform.
    let program = if cfg!(target_os = "windows") {
        r#"EXEC("cmd", ["/C", "echo hi"])"#
    } else {
        r#"EXEC("echo", ["hi"])"#
    };
    assert_output(
        &format!(
            r#"
            r <- {p}
            DISPLAY(TYPEOF(r))
            DISPLAY(TRIM(r["stdout"]))
            DISPLAY(r["exitcode"])
            DISPLAY(TYPEOF(r["stderr"]))
            "#,
            p = program
        ),
        "dictionary\nhi\n0\nstring",
    );
}

#[test]
fn test_exec_passes_arguments_without_shell_reparsing() {
    // A single argument containing a space and a quote must arrive intact rather
    // than being split -- the reason EXEC exists alongside SHELL.
    if cfg!(target_os = "windows") {
        return;
    }
    assert_output(
        r#"
        r <- EXEC("printf", ["%s", "one two"])
        DISPLAY(r["stdout"])
        "#,
        "one two",
    );
}

#[test]
fn test_exec_without_arguments() {
    if cfg!(target_os = "windows") {
        return;
    }
    assert_output(
        r#"
        r <- EXEC("true")
        DISPLAY(r["exitcode"])
        "#,
        "0",
    );
}

#[test]
fn test_exec_reports_a_nonzero_exit_code() {
    if cfg!(target_os = "windows") {
        return;
    }
    assert_output(
        r#"
        r <- EXEC("false")
        DISPLAY(r["exitcode"] = 0)
        "#,
        "false",
    );
}

#[test]
fn test_exec_captures_stderr_separately() {
    if cfg!(target_os = "windows") {
        return;
    }
    assert_output(
        r#"
        r <- SHELL("echo oops 1>&2")
        DISPLAY(TRIM(r["stderr"]))
        DISPLAY(CONCAT("stdout=", r["stdout"]))
        "#,
        "oops\nstdout=",
    );
}

#[test]
fn test_exec_on_a_missing_program_is_a_catchable_error() {
    let output = run_test(
        r#"
        TRY
        {
            r <- EXEC("psl-no-such-program-xyz")
            DISPLAY("should not get here")
        } CATCH (err)
        {
            DISPLAY("recovered")
        }
        "#,
    )
    .expect("TRY/CATCH should swallow the spawn failure");
    assert_eq!(output, "recovered");
}

#[test]
fn test_exec_rejects_non_string_arguments() {
    let err = get_error("DISPLAY(EXEC(\"echo\", [1]))");
    assert!(
        err.contains("EXEC arguments must all be strings"),
        "{}",
        err
    );

    let err = get_error("DISPLAY(EXEC(\"echo\", \"notalist\"))");
    assert!(err.contains("list of strings"), "{}", err);
}

#[test]
fn test_shell_runs_a_command_line() {
    // `echo` is a builtin of both cmd.exe and sh, so one script covers all three
    // platforms.
    assert_output(
        r#"
        r <- SHELL("echo shell works")
        DISPLAY(TRIM(r["stdout"]))
        "#,
        "shell works",
    );
}

#[test]
fn test_which_finds_a_real_program_and_reports_null_otherwise() {
    let present = if cfg!(target_os = "windows") {
        "cmd"
    } else {
        "sh"
    };
    assert_output(
        &format!(
            r#"
            DISPLAY(TYPEOF(WHICH("{p}")))
            DISPLAY(WHICH("psl-definitely-not-installed-xyz"))
            "#,
            p = present
        ),
        "string\nNULL",
    );
}

// ---------------------------------------------------------------------------
// Processes
// ---------------------------------------------------------------------------

#[test]
fn test_pid_is_a_positive_integer() {
    assert_output(
        r#"
        DISPLAY(TYPEOF(PID()))
        DISPLAY(PID() > 0)
        "#,
        "integer\ntrue",
    );
}

#[test]
fn test_processinfo_describes_our_own_process() {
    assert_output(
        r#"
        me <- PROCESSINFO(PID())
        DISPLAY(TYPEOF(me))
        DISPLAY(me["pid"] = PID())
        DISPLAY(TYPEOF(me["name"]))
        DISPLAY(me["memory"] > 0)
        "#,
        "dictionary\ntrue\nstring\ntrue",
    );
}

#[test]
fn test_processinfo_on_a_missing_pid_is_null() {
    // Pid 0 is never an ordinary user process on any of the three platforms.
    assert_output("DISPLAY(PROCESSINFO(999999999))", "NULL");
}

#[test]
fn test_processes_lists_at_least_our_own() {
    assert_output(
        r#"
        all <- PROCESSES()
        DISPLAY(TYPEOF(all))
        DISPLAY(LENGTH(all) > 0)
        DISPLAY(TYPEOF(all[1]))
        "#,
        "list\ntrue\ndictionary",
    );
}

#[test]
fn test_kill_refuses_to_kill_the_interpreter() {
    let err = get_error("DISPLAY(KILL(PID()))");
    assert!(
        err.contains("refuses to terminate the interpreter"),
        "{}",
        err
    );
}

#[test]
fn test_kill_on_a_missing_pid_errors() {
    let err = get_error("DISPLAY(KILL(999999999))");
    assert!(err.contains("No process is running with pid"), "{}", err);
}

#[test]
fn test_pid_arguments_are_validated() {
    let err = get_error("DISPLAY(PROCESSINFO(\"x\"))");
    assert!(err.contains("integer process id"), "{}", err);

    let err = get_error("DISPLAY(PROCESSINFO(-1))");
    assert!(err.contains("fits in 32 bits"), "{}", err);
}

// ---------------------------------------------------------------------------
// Working directory and paths
// ---------------------------------------------------------------------------

#[test]
fn test_cwd_is_an_existing_directory() {
    assert_output(
        r#"
        here <- CWD()
        DISPLAY(TYPEOF(here))
        DISPLAY(ISDIR(here))
        DISPLAY(ISFILE(here))
        "#,
        "string\ntrue\nfalse",
    );
}

#[test]
fn test_joinpath_uses_the_host_separator() {
    // Asserted through BASENAME/DIRNAME rather than against a literal separator,
    // so the test is true on Windows as well.
    assert_output(
        r#"
        p <- JOINPATH("a", "b", "c.txt")
        DISPLAY(BASENAME(p))
        DISPLAY(BASENAME(DIRNAME(p)))
        DISPLAY(EXTENSION(p))
        "#,
        "c.txt\nb\ntxt",
    );
}

#[test]
fn test_joinpath_with_one_segment() {
    assert_output("DISPLAY(JOINPATH(\"solo.txt\"))", "solo.txt");
}

#[test]
fn test_joinpath_requires_at_least_one_argument() {
    let err = get_error("DISPLAY(JOINPATH())");
    assert!(
        err.contains("JOINPATH requires at least one argument"),
        "{}",
        err
    );
}

#[test]
fn test_path_parts_of_a_plain_name() {
    assert_output(
        r#"
        DISPLAY(BASENAME("file.txt"))
        DISPLAY(DIRNAME("file.txt"))
        DISPLAY(EXTENSION("file.txt"))
        DISPLAY(EXTENSION("noextension"))
        DISPLAY(EXTENSION("archive.tar.gz"))
        "#,
        "file.txt\n\ntxt\n\ngz",
    );
}

#[test]
fn test_abspath_leaves_an_absolute_path_alone_and_does_not_require_existence() {
    let scratch = Scratch::new();
    let absolute = scratch.psl_path("nothing-here.txt");
    assert_output(
        &format!(
            r#"
            DISPLAY(ABSPATH("{p}") = "{p}")
            DISPLAY(FILEEXISTS(ABSPATH("{p}")))
            "#,
            p = absolute
        ),
        "true\nfalse",
    );
}

#[test]
fn test_abspath_resolves_a_relative_path_against_the_working_directory() {
    assert_output(
        r#"
        p <- ABSPATH("relative.txt")
        DISPLAY(BASENAME(p))
        DISPLAY(DIRNAME(p) = CWD())
        "#,
        "relative.txt\ntrue",
    );
}

#[test]
fn test_realpath_resolves_an_existing_file() {
    let scratch = Scratch::new();
    let path = scratch.psl_path("real.txt");
    assert_output(
        &format!(
            r#"
            WRITEFILE("{p}", "x")
            DISPLAY(BASENAME(REALPATH("{p}")))
            DISPLAY(ISFILE(REALPATH("{p}")))
            "#,
            p = path
        ),
        "real.txt\ntrue",
    );
}

#[test]
fn test_realpath_errors_on_a_missing_path() {
    let scratch = Scratch::new();
    let err = get_error(&format!(
        "DISPLAY(REALPATH(\"{}\"))",
        scratch.psl_path("ghost.txt")
    ));
    assert!(err.contains("Could not resolve"), "{}", err);
}

#[test]
fn test_isfile_and_isdir_distinguish_the_two() {
    let scratch = Scratch::new();
    assert_output(
        &format!(
            r#"
            WRITEFILE("{f}", "x")
            DISPLAY(ISFILE("{f}"))
            DISPLAY(ISDIR("{f}"))
            DISPLAY(ISFILE("{d}"))
            DISPLAY(ISDIR("{d}"))
            DISPLAY(ISFILE("{missing}"))
            DISPLAY(ISDIR("{missing}"))
            "#,
            f = scratch.psl_path("a.txt"),
            d = scratch.psl_path(""),
            missing = scratch.psl_path("gone")
        ),
        "true\nfalse\nfalse\ntrue\nfalse\nfalse",
    );
}

#[test]
fn test_tempdir_and_the_user_directories_are_paths() {
    assert_output(
        r#"
        DISPLAY(ISDIR(TEMPDIR()))
        DISPLAY(TYPEOF(HOMEDIR()))
        DISPLAY(TYPEOF(CONFIGDIR()))
        DISPLAY(TYPEOF(CACHEDIR()))
        DISPLAY(TYPEOF(DATADIR()))
        "#,
        "true\nstring\nstring\nstring\nstring",
    );
}

#[test]
fn test_homedir_exists_on_a_developer_machine() {
    assert_output("DISPLAY(ISDIR(HOMEDIR()))", "true");
}

// ---------------------------------------------------------------------------
// Filesystem operations beyond read and write
// ---------------------------------------------------------------------------

#[test]
fn test_rename_moves_a_file() {
    let scratch = Scratch::new();
    assert_output(
        &format!(
            r#"
            WRITEFILE("{a}", "contents")
            RENAME("{a}", "{b}")
            DISPLAY(FILEEXISTS("{a}"))
            DISPLAY(READFILE("{b}"))
            "#,
            a = scratch.psl_path("before.txt"),
            b = scratch.psl_path("after.txt")
        ),
        "false\ncontents",
    );
}

#[test]
fn test_rename_of_a_missing_file_errors() {
    let scratch = Scratch::new();
    let err = get_error(&format!(
        "RENAME(\"{}\", \"{}\")",
        scratch.psl_path("nope.txt"),
        scratch.psl_path("dest.txt")
    ));
    assert!(err.contains("RENAME failed for"), "{}", err);
}

#[test]
fn test_copyfile_duplicates_and_reports_the_byte_count() {
    let scratch = Scratch::new();
    assert_output(
        &format!(
            r#"
            WRITEFILE("{a}", "12345")
            n <- COPYFILE("{a}", "{b}")
            DISPLAY(n)
            DISPLAY(READFILE("{a}"))
            DISPLAY(READFILE("{b}"))
            "#,
            a = scratch.psl_path("src.txt"),
            b = scratch.psl_path("dst.txt")
        ),
        "5\n12345\n12345",
    );
}

#[test]
fn test_copyfile_overwrites_its_destination() {
    let scratch = Scratch::new();
    assert_output(
        &format!(
            r#"
            WRITEFILE("{a}", "new")
            WRITEFILE("{b}", "old and longer")
            COPYFILE("{a}", "{b}")
            DISPLAY(READFILE("{b}"))
            "#,
            a = scratch.psl_path("s.txt"),
            b = scratch.psl_path("d.txt")
        ),
        "new",
    );
}

// ---------------------------------------------------------------------------
// Machine facts
// ---------------------------------------------------------------------------

#[test]
fn test_platform_arch_and_family_are_known_values() {
    let output = run_test(
        r#"
        DISPLAY(PLATFORM())
        DISPLAY(ARCH())
        DISPLAY(OSFAMILY())
        "#,
    )
    .expect("machine facts");
    let lines: Vec<&str> = output.lines().collect();
    assert_eq!(lines[0], std::env::consts::OS);
    assert_eq!(lines[1], std::env::consts::ARCH);
    assert_eq!(lines[2], std::env::consts::FAMILY);
}

#[test]
fn test_version_matches_the_crate_version() {
    assert_output("DISPLAY(VERSION())", env!("CARGO_PKG_VERSION"));
}

#[test]
fn test_numeric_machine_facts_are_sane() {
    assert_output(
        r#"
        DISPLAY(CPUCOUNT() >= 1)
        DISPLAY(TOTALMEMORY() > 0)
        DISPLAY(USEDMEMORY() > 0)
        DISPLAY(USEDMEMORY() <= TOTALMEMORY())
        DISPLAY(UPTIME() > 0)
        "#,
        "true\ntrue\ntrue\ntrue\ntrue",
    );
}

#[test]
fn test_optional_machine_facts_are_a_string_or_null() {
    // A platform is allowed not to know these, and NULL is how it says so.
    let output = run_test(
        r#"
        DISPLAY(TYPEOF(HOSTNAME()))
        DISPLAY(TYPEOF(OSNAME()))
        DISPLAY(TYPEOF(OSVERSION()))
        DISPLAY(TYPEOF(KERNELVERSION()))
        DISPLAY(TYPEOF(USERNAME()))
        "#,
    )
    .expect("optional facts");
    for line in output.lines() {
        assert!(
            line == "string" || line == "null",
            "a text fact must be a string or NULL, got: {}",
            line
        );
    }

    // A count is an integer or NULL, never a string.
    let counts = run_test("DISPLAY(TYPEOF(PHYSICALCPUS()))").expect("physical cpus");
    assert!(
        counts == "integer" || counts == "null",
        "unexpected type: {}",
        counts
    );
}

#[test]
fn test_sysinfo_bundles_the_individual_facts() {
    assert_output(
        r#"
        info <- SYSINFO()
        DISPLAY(TYPEOF(info))
        DISPLAY(info["platform"] = PLATFORM())
        DISPLAY(info["arch"] = ARCH())
        DISPLAY(info["osfamily"] = OSFAMILY())
        DISPLAY(info["version"] = VERSION())
        DISPLAY(info["cpucount"] = CPUCOUNT())
        DISPLAY(CONTAINS(info, "totalmemory"))
        DISPLAY(CONTAINS(info, "hostname"))
        DISPLAY(CONTAINS(info, "uptime"))
        "#,
        "dictionary\ntrue\ntrue\ntrue\ntrue\ntrue\ntrue\ntrue\ntrue",
    );
}

#[test]
fn test_machine_facts_take_no_arguments() {
    for call in [
        "PLATFORM(1)",
        "ARCH(1)",
        "CPUCOUNT(1)",
        "SYSINFO(1)",
        "PID(1)",
        "CWD(1)",
        "TEMPDIR(1)",
        "ENVVARS(1)",
        "PROCESSES(1)",
    ] {
        let err = get_error(&format!("DISPLAY({})", call));
        assert!(err.contains("takes no arguments"), "{}: {}", call, err);
    }
}

#[test]
fn test_exit_rejects_an_out_of_range_code() {
    let err = get_error("EXIT(300)");
    assert!(err.contains("between 0 and 255"), "{}", err);

    let err = get_error("EXIT(-1)");
    assert!(err.contains("between 0 and 255"), "{}", err);

    let err = get_error("EXIT(\"x\")");
    assert!(err.contains("integer exit code"), "{}", err);

    let err = get_error("EXIT(1, 2)");
    assert!(err.contains("no arguments, or one exit code"), "{}", err);
}
