use super::{assert_output, assert_output_with_args, run_test_with_args};

#[test]
fn test_args_empty() {
    assert_output("DISPLAY(ARGS)", "[]");
    assert_output("DISPLAY(ARGCOUNT)", "0");
    assert_output("DISPLAY(POSITIONALS)", "[]");
}

#[test]
fn test_args_raw_list() {
    assert_output_with_args(
        "DISPLAY(ARGS)",
        &["--verbose", "-n", "5", "output.txt"],
        "[--verbose, -n, 5, output.txt]",
    );
}

#[test]
fn test_argcount() {
    assert_output_with_args("DISPLAY(ARGCOUNT)", &["--a", "-b", "c"], "3");
    assert_output_with_args("DISPLAY(ARGCOUNT)", &[], "0");
}

#[test]
fn test_positionals() {
    assert_output_with_args(
        "DISPLAY(POSITIONALS)",
        &["--verbose", "-n", "5", "input.txt", "output.txt"],
        "[input.txt, output.txt]",
    );
}

#[test]
fn test_positionals_only() {
    assert_output_with_args(
        "DISPLAY(POSITIONALS)",
        &["foo", "bar", "baz"],
        "[foo, bar, baz]",
    );
}

#[test]
fn test_hasarg_long_flag() {
    assert_output_with_args("DISPLAY(HASARG(\"verbose\"))", &["--verbose"], "true");
}

#[test]
fn test_hasarg_short_flag() {
    assert_output_with_args("DISPLAY(HASARG(\"v\"))", &["-v"], "true");
}

#[test]
fn test_hasarg_missing() {
    assert_output_with_args("DISPLAY(HASARG(\"missing\"))", &["--other"], "false");
}

#[test]
fn test_hasarg_with_dashes() {
    assert_output_with_args("DISPLAY(HASARG(\"--verbose\"))", &["--verbose"], "true");
}

#[test]
fn test_hasarg_short_with_dash() {
    assert_output_with_args("DISPLAY(HASARG(\"-v\"))", &["-v"], "true");
}

#[test]
fn test_getarg_long_with_value() {
    assert_output_with_args(
        r#"DISPLAY(GETARG("output"))"#,
        &["--output", "file.txt"],
        "file.txt",
    );
}

#[test]
fn test_getarg_short_with_value() {
    assert_output_with_args(r#"DISPLAY(GETARG("n"))"#, &["-n", "42"], "42");
}

#[test]
fn test_getarg_boolean_flag() {
    assert_output_with_args(r#"DISPLAY(GETARG("verbose"))"#, &["--verbose"], "true");
}

#[test]
fn test_getarg_missing_with_default() {
    assert_output_with_args(
        r#"DISPLAY(GETARG("missing", "fallback"))"#,
        &["--other"],
        "fallback",
    );
}

#[test]
fn test_getarg_missing_no_default_errors() {
    let result = run_test_with_args(r#"DISPLAY(GETARG("missing"))"#, &["--other"]);
    assert!(result.is_err());
}

#[test]
fn test_getarg_with_dashes_in_query() {
    assert_output_with_args(
        r#"DISPLAY(GETARG("--output"))"#,
        &["--output", "file.txt"],
        "file.txt",
    );
}

#[test]
fn test_mixed_flags_and_positionals() {
    let code = r#"
DISPLAY(ARGCOUNT)
DISPLAY(HASARG("verbose"))
DISPLAY(GETARG("n"))
DISPLAY(POSITIONALS)
"#;
    assert_output_with_args(
        code,
        &["--verbose", "-n", "5", "input.txt", "output.txt"],
        "5\ntrue\n5\n[input.txt, output.txt]",
    );
}

#[test]
fn test_hasarg_type_error() {
    let result = run_test_with_args("DISPLAY(HASARG(42))", &["--flag"]);
    assert!(result.is_err());
}

#[test]
fn test_getarg_type_error() {
    let result = run_test_with_args("DISPLAY(GETARG(42))", &["--flag"]);
    assert!(result.is_err());
}

#[test]
fn test_args_iteration() {
    let code = r#"
FOR EACH arg IN ARGS {
    DISPLAY(arg)
}
"#;
    assert_output_with_args(code, &["--flag", "-n", "5"], "--flag\n-n\n5");
}

#[test]
fn test_getarg_default_not_evaluated_when_found() {
    let code = r#"DISPLAY(GETARG("n", 1 / 0))"#;
    assert_output_with_args(code, &["-n", "42"], "42");
}
