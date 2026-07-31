use super::{Scratch, assert_output, get_error, run_test};
use std::path::Path;

#[test]
fn test_write_then_read_roundtrip() {
    let scratch = Scratch::new();
    let path = scratch.psl_path("hello.txt");
    assert_output(
        &format!(
            "WRITEFILE(\"{}\", \"hello\")\nDISPLAY(READFILE(\"{}\"))",
            path, path
        ),
        "hello",
    );
}

#[test]
fn test_write_truncates_existing_contents() {
    let scratch = Scratch::new();
    let path = scratch.psl_path("truncate.txt");
    assert_output(
        &format!(
            r#"
            WRITEFILE("{p}", "first pass")
            WRITEFILE("{p}", "second")
            DISPLAY(READFILE("{p}"))
            "#,
            p = path
        ),
        "second",
    );
}

#[test]
fn test_append_creates_then_extends() {
    let scratch = Scratch::new();
    let path = scratch.psl_path("append.txt");
    // APPENDFILE on a missing path creates it, so a log-style program does not
    // have to special-case its first write.
    assert_output(
        &format!(
            r#"
            APPENDFILE("{p}", "a")
            APPENDFILE("{p}", "b")
            APPENDFILE("{p}", "c")
            DISPLAY(READFILE("{p}"))
            "#,
            p = path
        ),
        "abc",
    );
}

#[test]
fn test_readlines_splits_and_strips_terminators() {
    let scratch = Scratch::new();
    let path = scratch.psl_path("lines.txt");
    assert_output(
        &format!(
            r#"
            WRITEFILE("{p}", "one\ntwo\nthree\n")
            lines <- READLINES("{p}")
            DISPLAY(LENGTH(lines))
            DISPLAY(lines[1])
            DISPLAY(lines[3])
            DISPLAY(lines)
            "#,
            p = path
        ),
        "3\none\nthree\n[one, two, three]",
    );
}

#[test]
fn test_readlines_handles_crlf_and_no_trailing_newline() {
    let scratch = Scratch::new();
    let path = scratch.psl_path("crlf.txt");
    assert_output(
        &format!(
            r#"
            WRITEFILE("{p}", "one\r\ntwo")
            DISPLAY(READLINES("{p}"))
            "#,
            p = path
        ),
        "[one, two]",
    );
}

#[test]
fn test_readlines_on_empty_file_is_empty_list() {
    let scratch = Scratch::new();
    let path = scratch.psl_path("empty.txt");
    assert_output(
        &format!(
            r#"
            WRITEFILE("{p}", "")
            DISPLAY(LENGTH(READLINES("{p}")))
            DISPLAY(READLINES("{p}"))
            "#,
            p = path
        ),
        "0\n[]",
    );
}

#[test]
fn test_readlines_iterates_with_for_each() {
    let scratch = Scratch::new();
    let path = scratch.psl_path("iterate.txt");
    assert_output(
        &format!(
            r#"
            WRITEFILE("{p}", "10\n20\n30\n")
            total <- 0
            FOR EACH line IN READLINES("{p}")
            {{
                total <- total + TONUM(line)
            }}
            DISPLAY(total)
            "#,
            p = path
        ),
        "60",
    );
}

#[test]
fn test_fileexists() {
    let scratch = Scratch::new();
    let present = scratch.psl_path("present.txt");
    let missing = scratch.psl_path("missing.txt");
    assert_output(
        &format!(
            r#"
            WRITEFILE("{present}", "x")
            DISPLAY(FILEEXISTS("{present}"))
            DISPLAY(FILEEXISTS("{missing}"))
            "#,
            present = present,
            missing = missing
        ),
        "true\nfalse",
    );
}

#[test]
fn test_filesize_counts_bytes_not_characters() {
    let scratch = Scratch::new();
    let path = scratch.psl_path("size.txt");
    // "héllo" is 5 characters but 6 bytes, which is the distinction between
    // FILESIZE and LENGTH(READFILE(...)).
    assert_output(
        &format!(
            r#"
            WRITEFILE("{p}", "héllo")
            DISPLAY(FILESIZE("{p}"))
            DISPLAY(LENGTH(READFILE("{p}")))
            "#,
            p = path
        ),
        "6\n5",
    );
}

#[test]
fn test_deletefile_removes_the_file() {
    let scratch = Scratch::new();
    let path = scratch.psl_path("doomed.txt");
    assert_output(
        &format!(
            r#"
            WRITEFILE("{p}", "x")
            DISPLAY(FILEEXISTS("{p}"))
            DELETEFILE("{p}")
            DISPLAY(FILEEXISTS("{p}"))
            "#,
            p = path
        ),
        "true\nfalse",
    );
    assert!(!scratch.path("doomed.txt").exists());
}

#[test]
fn test_listdir_is_sorted_by_name() {
    let scratch = Scratch::new();
    let dir = scratch.psl_path("");
    assert_output(
        &format!(
            r#"
            WRITEFILE("{c}", "")
            WRITEFILE("{a}", "")
            WRITEFILE("{b}", "")
            DISPLAY(LISTDIR("{dir}"))
            "#,
            a = scratch.psl_path("a.txt"),
            b = scratch.psl_path("b.txt"),
            c = scratch.psl_path("c.txt"),
            dir = dir
        ),
        "[a.txt, b.txt, c.txt]",
    );
}

#[test]
fn test_listdir_returns_names_not_paths() {
    let scratch = Scratch::new();
    assert_output(
        &format!(
            r#"
            WRITEFILE("{f}", "")
            names <- LISTDIR("{dir}")
            DISPLAY(names[1])
            "#,
            f = scratch.psl_path("only.txt"),
            dir = scratch.psl_path("")
        ),
        "only.txt",
    );
}

#[test]
fn test_makedir_is_recursive_and_idempotent() {
    let scratch = Scratch::new();
    let nested = scratch.psl_path("a/b/c");
    assert_output(
        &format!(
            r#"
            MAKEDIR("{d}")
            MAKEDIR("{d}")
            DISPLAY(FILEEXISTS("{d}"))
            WRITEFILE("{f}", "deep")
            DISPLAY(READFILE("{f}"))
            "#,
            d = nested,
            f = scratch.psl_path("a/b/c/deep.txt")
        ),
        "true\ndeep",
    );
    assert!(scratch.path("a/b/c").is_dir());
}

#[test]
fn test_deletedir_removes_an_empty_directory() {
    let scratch = Scratch::new();
    assert_output(
        &format!(
            r#"
            MAKEDIR("{d}")
            DISPLAY(ISDIR("{d}"))
            DELETEDIR("{d}")
            DISPLAY(ISDIR("{d}"))
            "#,
            d = scratch.psl_path("empty-dir")
        ),
        "true\nfalse",
    );
    assert!(!scratch.path("empty-dir").exists());
}

#[test]
fn test_deletedir_refuses_a_directory_that_still_has_contents() {
    // Safe by construction: the one operation that cannot destroy work.
    let scratch = Scratch::new();
    let err = get_error(&format!(
        r#"
        MAKEDIR("{d}")
        WRITEFILE("{f}", "x")
        DELETEDIR("{d}")
        "#,
        d = scratch.psl_path("full-dir"),
        f = scratch.psl_path("full-dir/keep.txt")
    ));
    assert!(err.contains("DELETEDIR failed for"), "{}", err);
    assert!(err.contains("DELETETREE"), "{}", err);
    assert!(scratch.path("full-dir/keep.txt").is_file());
}

#[test]
fn test_deletetree_removes_a_directory_and_everything_in_it() {
    let scratch = Scratch::new();
    assert_output(
        &format!(
            r#"
            MAKEDIR("{deep}")
            WRITEFILE("{f}", "x")
            DELETETREE("{root}")
            DISPLAY(ISDIR("{root}"))
            "#,
            root = scratch.psl_path("tree"),
            deep = scratch.psl_path("tree/a/b"),
            f = scratch.psl_path("tree/a/b/leaf.txt")
        ),
        "false",
    );
    assert!(!scratch.path("tree").exists());
}

#[test]
fn test_deletetree_refuses_a_plain_file() {
    let scratch = Scratch::new();
    let err = get_error(&format!(
        r#"
        WRITEFILE("{f}", "x")
        DELETETREE("{f}")
        "#,
        f = scratch.psl_path("just-a-file.txt")
    ));
    assert!(err.contains("is a file"), "{}", err);
    assert!(err.contains("DELETEFILE"), "{}", err);
    assert!(scratch.path("just-a-file.txt").is_file());
}

#[test]
fn test_deletefile_on_a_directory_names_the_reason_and_the_alternatives() {
    // The OS reports EPERM on macOS and EISDIR on Linux for this, neither of which
    // mentions directories, so the message is our own.
    let scratch = Scratch::new();
    let err = get_error(&format!(
        r#"
        MAKEDIR("{d}")
        DELETEFILE("{d}")
        "#,
        d = scratch.psl_path("a-directory")
    ));
    assert!(err.contains("will not remove a directory"), "{}", err);
    assert!(err.contains("DELETEDIR"), "{}", err);
    assert!(err.contains("DELETETREE"), "{}", err);
    assert!(scratch.path("a-directory").is_dir());
}

#[test]
fn test_deletedir_and_deletetree_on_a_missing_path_error() {
    let scratch = Scratch::new();
    let err = get_error(&format!("DELETEDIR(\"{}\")", scratch.psl_path("nope")));
    assert!(err.contains("DELETEDIR failed for"), "{}", err);

    let err = get_error(&format!("DELETETREE(\"{}\")", scratch.psl_path("nope")));
    assert!(err.contains("DELETETREE failed for"), "{}", err);
}

#[test]
fn test_makedir_and_deletetree_round_trip() {
    // The pairing the surface had been missing: create a tree, then remove it.
    let scratch = Scratch::new();
    assert_output(
        &format!(
            r#"
            MAKEDIR("{deep}")
            DISPLAY(ISDIR("{deep}"))
            DELETETREE("{root}")
            DISPLAY(ISDIR("{root}"))
            DISPLAY(FILEEXISTS("{root}"))
            "#,
            root = scratch.psl_path("scratch-tree"),
            deep = scratch.psl_path("scratch-tree/x/y/z")
        ),
        "true\nfalse\nfalse",
    );
}

#[test]
fn test_filemtime_is_unix_seconds_and_feeds_the_time_builtins() {
    let scratch = Scratch::new();
    assert_output(
        &format!(
            r#"
            WRITEFILE("{f}", "x")
            stamp <- FILEMTIME("{f}")
            DISPLAY(TYPEOF(stamp))
            DISPLAY(stamp > 1700000000)
            DISPLAY(stamp <= TIMESTAMP())
            DISPLAY(TYPEOF(TIME(stamp)))
            "#,
            f = scratch.psl_path("stamped.txt")
        ),
        "integer\ntrue\ntrue\nstring",
    );
}

#[test]
fn test_filemtime_orders_two_files_by_age() {
    // The question FILEMTIME exists to answer: is my output older than my input?
    let scratch = Scratch::new();
    assert_output(
        &format!(
            r#"
            WRITEFILE("{old}", "first")
            SLEEP(1.1)
            WRITEFILE("{new}", "second")
            COMMENT strictly greater: a full second apart, so equal stamps mean broken
            DISPLAY(FILEMTIME("{new}") > FILEMTIME("{old}"))
            DISPLAY(FILEMTIME("{old}") < FILEMTIME("{new}"))
            "#,
            old = scratch.psl_path("older.txt"),
            new = scratch.psl_path("newer.txt")
        ),
        "true\ntrue",
    );
}

#[test]
fn test_filemtime_works_on_a_directory_and_errors_on_a_missing_path() {
    let scratch = Scratch::new();
    assert_output(
        &format!(
            r#"
            MAKEDIR("{d}")
            DISPLAY(TYPEOF(FILEMTIME("{d}")))
            "#,
            d = scratch.psl_path("timed-dir")
        ),
        "integer",
    );

    let err = get_error(&format!(
        "DISPLAY(FILEMTIME(\"{}\"))",
        scratch.psl_path("gone")
    ));
    assert!(err.contains("FILEMTIME failed for"), "{}", err);
}

#[test]
fn test_new_file_builtins_check_their_arity_and_argument_type() {
    for call in ["DELETEDIR()", "DELETETREE()", "FILEMTIME()"] {
        let err = get_error(&format!("DISPLAY({})", call));
        assert!(err.contains("requires one argument"), "{}: {}", call, err);
    }
    for call in ["DELETEDIR(1)", "DELETETREE(1)", "FILEMTIME(1)"] {
        let err = get_error(&format!("DISPLAY({})", call));
        assert!(err.contains("requires a string path"), "{}: {}", call, err);
    }
}

#[test]
fn test_write_accepts_formatted_and_multiline_strings() {
    let scratch = Scratch::new();
    let path = scratch.psl_path("formatted.txt");
    assert_output(
        &format!(
            r#"
            name <- "world"
            WRITEFILE("{p}", f"hello {{name}}")
            DISPLAY(READFILE("{p}"))
            "#,
            p = path
        ),
        "hello world",
    );
}

#[test]
fn test_read_failure_is_catchable() {
    let scratch = Scratch::new();
    let missing = scratch.psl_path("nope.txt");
    let output = run_test(&format!(
        r#"
        TRY
        {{
            DISPLAY(READFILE("{p}"))
        }} CATCH (err)
        {{
            DISPLAY("recovered")
        }}
        "#,
        p = missing
    ))
    .expect("TRY/CATCH should swallow the read error");
    assert_eq!(output, "recovered");
}

#[test]
fn test_read_missing_file_names_the_path() {
    let scratch = Scratch::new();
    let missing = scratch.psl_path("absent.txt");
    let err = get_error(&format!("DISPLAY(READFILE(\"{}\"))", missing));
    assert!(err.contains("READFILE failed for"), "{}", err);
    assert!(err.contains("absent.txt"), "{}", err);
}

#[test]
fn test_readfile_rejects_a_directory() {
    let scratch = Scratch::new();
    let err = get_error(&format!("DISPLAY(READFILE(\"{}\"))", scratch.psl_path("")));
    assert!(err.contains("READFILE failed for"), "{}", err);
}

#[test]
fn test_deletefile_on_missing_file_errors() {
    let scratch = Scratch::new();
    let err = get_error(&format!(
        "DELETEFILE(\"{}\")",
        scratch.psl_path("ghost.txt")
    ));
    assert!(err.contains("DELETEFILE failed for"), "{}", err);
}

#[test]
fn test_listdir_on_missing_directory_errors() {
    let scratch = Scratch::new();
    let err = get_error(&format!(
        "DISPLAY(LISTDIR(\"{}\"))",
        scratch.psl_path("nodir")
    ));
    assert!(err.contains("LISTDIR failed for"), "{}", err);
}

#[test]
fn test_wrong_argument_counts() {
    let err = get_error("DISPLAY(READFILE())");
    assert!(err.contains("READFILE requires one argument"), "{}", err);

    let err = get_error("DISPLAY(READFILE(\"a\", \"b\"))");
    assert!(err.contains("READFILE requires one argument"), "{}", err);

    let err = get_error("WRITEFILE(\"a\")");
    assert!(err.contains("WRITEFILE requires two arguments"), "{}", err);

    let err = get_error("APPENDFILE(\"a\", \"b\", \"c\")");
    assert!(err.contains("APPENDFILE requires two arguments"), "{}", err);

    let err = get_error("DISPLAY(LISTDIR())");
    assert!(err.contains("LISTDIR requires one argument"), "{}", err);
}

#[test]
fn test_non_string_arguments_are_rejected() {
    let err = get_error("DISPLAY(READFILE(1))");
    assert!(err.contains("READFILE requires a string path"), "{}", err);

    let err = get_error("DISPLAY(FILESIZE([1, 2]))");
    assert!(err.contains("FILESIZE requires a string path"), "{}", err);

    let scratch = Scratch::new();
    let err = get_error(&format!("WRITEFILE(\"{}\", 42)", scratch.psl_path("n.txt")));
    assert!(
        err.contains("WRITEFILE requires a string as its second argument"),
        "{}",
        err
    );
    // The rejected write must not have created the file.
    assert!(!scratch.path("n.txt").exists());
}

#[test]
fn test_file_builtins_shadow_user_procedures() {
    let scratch = Scratch::new();
    let path = scratch.psl_path("shadow.txt");
    // Built-ins are resolved before user-defined procedures, the same rule the
    // dictionary and string built-ins follow.
    assert_output(
        &format!(
            r#"
            PROCEDURE READFILE(p)
            {{
                RETURN "from the procedure"
            }}
            WRITEFILE("{p}", "from the file")
            DISPLAY(READFILE("{p}"))
            "#,
            p = path
        ),
        "from the file",
    );
}

#[test]
fn test_scratch_paths_are_absolute() {
    // The suite's own assumption: every path handed to PSL is absolute, so no
    // test depends on the working directory cargo happens to run it in.
    let scratch = Scratch::new();
    assert!(Path::new(&scratch.path("x.txt")).is_absolute());
}

#[test]
fn test_copyfile_refuses_to_copy_a_file_onto_itself() {
    // `fs::copy` truncates the destination before reading the source, so this
    // destroyed the file and reported success.
    let scratch = Scratch::new();
    let path = scratch.psl_path("precious.txt");
    let err = get_error(&format!(
        r#"
        WRITEFILE("{p}", "important data")
        COPYFILE("{p}", "{p}")
        "#,
        p = path
    ));
    assert!(err.contains("onto itself"), "{}", err);
    assert_eq!(
        std::fs::read_to_string(scratch.path("precious.txt")).expect("still there"),
        "important data"
    );
}

#[test]
fn test_copyfile_refuses_two_spellings_of_the_same_file() {
    let scratch = Scratch::new();
    let err = get_error(&format!(
        r#"
        WRITEFILE("{p}", "data")
        COPYFILE("{p}", "{alias}")
        "#,
        p = scratch.psl_path("original.txt"),
        alias = scratch.psl_path("./original.txt")
    ));
    assert!(err.contains("onto itself"), "{}", err);
}

#[test]
#[cfg(unix)]
fn test_a_symlink_is_removed_as_a_file_whatever_it_points_at() {
    // `ISDIR` follows links, so a link to a directory was refused by DELETEFILE,
    // rejected by DELETEDIR as "not a directory", and left with no way to remove it.
    let scratch = Scratch::new();
    std::fs::create_dir_all(scratch.path("target-dir")).expect("create dir");
    std::os::unix::fs::symlink(scratch.path("target-dir"), scratch.path("link-to-dir"))
        .expect("create symlink");

    assert_output(
        &format!(
            r#"
            DISPLAY(ISDIR("{link}"))
            DELETEFILE("{link}")
            DISPLAY(FILEEXISTS("{link}"))
            DISPLAY(ISDIR("{target}"))
            "#,
            link = scratch.psl_path("link-to-dir"),
            target = scratch.psl_path("target-dir")
        ),
        // The link goes; what it pointed at stays.
        "true\nfalse\ntrue",
    );
}

#[test]
#[cfg(unix)]
fn test_deletetree_refuses_a_symlink_and_names_deletefile() {
    let scratch = Scratch::new();
    std::fs::create_dir_all(scratch.path("real")).expect("create dir");
    std::os::unix::fs::symlink(scratch.path("real"), scratch.path("alias")).expect("symlink");
    let err = get_error(&format!("DELETETREE(\"{}\")", scratch.psl_path("alias")));
    assert!(err.contains("DELETEFILE"), "{}", err);
    // Refusing must not have removed the directory it pointed at.
    assert!(scratch.path("real").is_dir());
}
