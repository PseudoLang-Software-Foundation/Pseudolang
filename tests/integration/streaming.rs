//! The streaming output sink, and the file built-ins against a filesystem the
//! test inspects afterwards. The library tests capture into a `String` and cannot
//! distinguish written from buffered.

use crate::harness::Program;

#[test]
fn interleaved_display_and_displayinline_keep_their_order() {
    Program::new(
        r#"
        DISPLAYINLINE("a")
        DISPLAY("b")
        DISPLAYINLINE("c")
        DISPLAYINLINE("d")
        DISPLAY("e")
        "#,
    )
    .run()
    .success()
    .stdout_is("ab\ncde");
}

#[test]
fn output_written_before_a_failure_is_not_lost() {
    Program::new(
        r#"
        DISPLAY("first")
        DISPLAYINLINE("second, unterminated")
        x <- 1 / 0
        "#,
    )
    .run()
    .code(1)
    .stdout_contains("first")
    .stdout_contains("second, unterminated");
}

#[test]
fn output_from_a_caught_error_path_reaches_the_terminal() {
    // A CATCH block that displays and then the program continues: the writes
    // happen inside a scope that is unwound, and used to be dropped.
    Program::new(
        r#"
        TRY
        {
            x <- 1 / 0
        } CATCH (err)
        {
            DISPLAY("caught it")
        }
        DISPLAY("carried on")
        "#,
    )
    .run()
    .success()
    .stdout_is("caught it\ncarried on");
}

#[test]
fn output_from_a_catch_block_that_returns_is_not_dropped() {
    Program::new(
        r#"
        PROCEDURE attempt()
        {
            TRY
            {
                x <- 1 / 0
            } CATCH (err)
            {
                DISPLAY("inside catch")
                RETURN "returned from catch"
            }
            RETURN "not reached"
        }
        DISPLAY(attempt())
        "#,
    )
    .run()
    .success()
    .stdout_is("inside catch\nreturned from catch");
}

// ---------------------------------------------------------------------------
// File IO against a filesystem the test inspects itself
// ---------------------------------------------------------------------------

#[test]
fn a_written_file_really_exists_on_disk_afterwards() {
    // The in-process tests check what the program can read back. This checks the
    // bytes from outside the interpreter entirely.
    let run = Program::new(
        r#"
        WRITEFILE("out.txt", "written by the program\n")
        APPENDFILE("out.txt", "and appended\n")
        "#,
    )
    .run();
    run.success();
    assert_eq!(
        run.file("out.txt"),
        "written by the program\nand appended\n"
    );
}

#[test]
fn a_deleted_file_is_really_gone() {
    let run = Program::new(
        r#"
        WRITEFILE("temp.txt", "x")
        DELETEFILE("temp.txt")
        "#,
    )
    .run();
    run.success();
    assert!(!run.file_exists("temp.txt"));
}

#[test]
fn makedir_creates_the_tree_on_disk() {
    let run = Program::new(
        r#"
        MAKEDIR(JOINPATH("a", JOINPATH("b", "c")))
        WRITEFILE(JOINPATH("a", JOINPATH("b", JOINPATH("c", "deep.txt"))), "deep")
        "#,
    )
    .run();
    run.success();
    assert!(run.path("a/b/c").is_dir());
    assert_eq!(run.file("a/b/c/deep.txt"), "deep");
}

#[test]
fn rename_and_copyfile_move_real_bytes() {
    let run = Program::new(
        r#"
        WRITEFILE("original.txt", "12345")
        n <- COPYFILE("original.txt", "copy.txt")
        DISPLAY(n)
        RENAME("original.txt", "moved.txt")
        "#,
    )
    .run();
    run.success().stdout_is("5");
    assert!(!run.file_exists("original.txt"));
    assert_eq!(run.file("moved.txt"), "12345");
    assert_eq!(run.file("copy.txt"), "12345");
}

#[test]
fn a_file_written_then_read_by_the_same_program_round_trips_non_ascii() {
    let run = Program::new(
        r#"
        WRITEFILE("utf8.txt", "héllo — wörld")
        back <- READFILE("utf8.txt")
        DISPLAY(back)
        DISPLAY(LENGTH(back))
        DISPLAY(FILESIZE("utf8.txt"))
        "#,
    )
    .run();
    run.success();
    let lines = run.lines();
    assert_eq!(lines[0], "héllo — wörld");
    // Characters, then bytes: they must differ for this string.
    assert_eq!(lines[1], "13");
    assert_ne!(lines[1], lines[2], "FILESIZE should be a byte count");
}

#[test]
fn listdir_sees_files_the_program_just_created() {
    Program::new(
        r#"
        WRITEFILE("c.txt", "")
        WRITEFILE("a.txt", "")
        WRITEFILE("b.txt", "")
        names <- LISTDIR(".")
        DISPLAY(CONTAINS(names, "a.txt"))
        DISPLAY(CONTAINS(names, "b.txt"))
        DISPLAY(CONTAINS(names, "c.txt"))
        DISPLAY(CONTAINS(names, "main.psl"))
        "#,
    )
    .run()
    .success()
    .stdout_is("true\ntrue\ntrue\ntrue");
}

#[test]
fn a_read_of_a_missing_file_fails_with_the_path_and_exits_one() {
    Program::new(r#"DISPLAY(READFILE("absent.txt"))"#)
        .run()
        .code(1)
        .stderr_contains("READFILE failed for")
        .stderr_contains("absent.txt");
}

#[test]
fn a_program_can_recover_from_a_missing_file_and_carry_on() {
    Program::new(
        r#"
        TRY
        {
            config <- READFILE("config.txt")
        } CATCH (err)
        {
            config <- "defaults"
        }
        DISPLAY(config)
        "#,
    )
    .run()
    .success()
    .stdout_is("defaults");
}
