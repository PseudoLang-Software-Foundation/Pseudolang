//! Fixture runner: every program under `tests/programs/` is executed and its
//! output compared against a recorded expectation.
//!
//! Adding one needs no Rust: drop two files in `tests/programs/`. Use a
//! hand-written test in the other modules when you need stdin, timing, arguments,
//! a particular working directory, or to inspect the filesystem afterwards.
//!
//! # Layouts
//!
//! A single-file fixture is a pair:
//!
//! ```text
//! tests/programs/sorting.psl        <- the program
//! tests/programs/sorting.expected   <- exactly what it should print
//! ```
//!
//! A multi-file fixture is a directory containing `main.psl`, an `expected` file,
//! and whatever else the program imports or reads:
//!
//! ```text
//! tests/programs/library/main.psl
//! tests/programs/library/helper.psl
//! tests/programs/library/expected
//! ```
//!
//! # Directives
//!
//! Anything the program needs beyond "run it" goes in `#` comment lines at the top
//! of the entry file. `#` is a PseudoLang line comment, so a fixture with
//! directives is still a runnable program.
//!
//! ```text
//! # ARGS: --mode fast input.txt
//! # STDIN: first line
//! # STDIN: second line
//! # EXIT: 3
//! # STDERR: Division by zero
//! ```
//!
//! * `ARGS`   -- whitespace-separated program arguments. May appear once.
//! * `STDIN`  -- one line of standard input. May appear any number of times.
//! * `EXIT`   -- the expected exit status. Defaults to 0.
//! * `STDERR` -- a substring that must appear on standard error.
//!
//! # Reporting
//!
//! One `#[test]` runs every fixture and reports *all* failures together, so a
//! change that breaks several is diagnosed in one pass rather than one per run.

use crate::harness::Program;
use std::path::{Path, PathBuf};

/// Everything a fixture says about how it should be run and what it should do.
struct Expectation {
    args: Vec<String>,
    stdin: Option<String>,
    exit: i32,
    stderr_contains: Option<String>,
    stdout: String,
}

fn programs_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/programs")
}

/// Pull the `#` directives out of a fixture's source.
fn parse_directives(source: &str) -> (Vec<String>, Option<String>, i32, Option<String>) {
    let mut args = Vec::new();
    let mut stdin_lines: Vec<String> = Vec::new();
    let mut exit = 0;
    let mut stderr = None;

    for line in source.lines() {
        let trimmed = line.trim();
        let Some(body) = trimmed.strip_prefix('#') else {
            // Directives are a header block; stop at the first line of real code
            // so a `#` comment further down is not mistaken for one.
            if trimmed.is_empty() {
                continue;
            }
            break;
        };
        let body = body.trim();
        if let Some(rest) = body.strip_prefix("ARGS:") {
            args = rest.split_whitespace().map(str::to_string).collect();
        } else if let Some(rest) = body.strip_prefix("STDIN:") {
            // The single leading space after the colon is separator, not data.
            stdin_lines.push(rest.strip_prefix(' ').unwrap_or(rest).to_string());
        } else if let Some(rest) = body.strip_prefix("EXIT:") {
            exit = rest
                .trim()
                .parse()
                .unwrap_or_else(|_| panic!("fixture EXIT directive is not a number: {:?}", rest));
        } else if let Some(rest) = body.strip_prefix("STDERR:") {
            stderr = Some(rest.trim().to_string());
        }
    }

    let stdin = if stdin_lines.is_empty() {
        None
    } else {
        Some(format!("{}\n", stdin_lines.join("\n")))
    };
    (args, stdin, exit, stderr)
}

/// A fixture, resolved to the files it needs.
struct Fixture {
    name: String,
    /// Entry file source.
    source: String,
    /// Extra files, as (path relative to the program root, contents).
    extra: Vec<(String, String)>,
    expectation: Expectation,
}

fn collect_fixtures() -> Vec<Fixture> {
    let dir = programs_dir();
    let entries = std::fs::read_dir(&dir)
        .unwrap_or_else(|e| panic!("could not read {}: {}", dir.display(), e));

    let mut fixtures = Vec::new();
    for entry in entries {
        let entry = entry.expect("directory entry");
        let path = entry.path();
        let file_type = entry.file_type().expect("file type");

        if file_type.is_dir() {
            fixtures.push(load_directory_fixture(&path));
        } else if path.extension().and_then(|e| e.to_str()) == Some("psl") {
            fixtures.push(load_single_file_fixture(&path));
        }
    }
    fixtures.sort_by(|a, b| a.name.cmp(&b.name));
    fixtures
}

fn load_single_file_fixture(path: &Path) -> Fixture {
    let name = path
        .file_stem()
        .expect("fixture stem")
        .to_string_lossy()
        .into_owned();
    let source = read(path);
    let expected_path = path.with_extension("expected");
    let stdout = std::fs::read_to_string(&expected_path).unwrap_or_else(|e| {
        panic!(
            "fixture {} has no .expected file ({}): {}",
            name,
            expected_path.display(),
            e
        )
    });
    let (args, stdin, exit, stderr_contains) = parse_directives(&source);
    Fixture {
        name,
        source,
        extra: Vec::new(),
        expectation: Expectation {
            args,
            stdin,
            exit,
            stderr_contains,
            stdout,
        },
    }
}

fn load_directory_fixture(dir: &Path) -> Fixture {
    let name = dir
        .file_name()
        .expect("fixture dir name")
        .to_string_lossy()
        .into_owned();
    let entry_path = dir.join("main.psl");
    let source = std::fs::read_to_string(&entry_path)
        .unwrap_or_else(|e| panic!("fixture directory {} needs a main.psl: {}", name, e));
    let stdout = std::fs::read_to_string(dir.join("expected"))
        .unwrap_or_else(|e| panic!("fixture directory {} needs an `expected` file: {}", name, e));

    // Everything else in the tree travels with the program.
    let mut extra = Vec::new();
    collect_tree(dir, dir, &mut extra);
    extra.retain(|(relative, _)| relative != "main.psl" && relative != "expected");

    let (args, stdin, exit, stderr_contains) = parse_directives(&source);
    Fixture {
        name,
        source,
        extra,
        expectation: Expectation {
            args,
            stdin,
            exit,
            stderr_contains,
            stdout,
        },
    }
}

fn collect_tree(root: &Path, dir: &Path, out: &mut Vec<(String, String)>) {
    let entries = std::fs::read_dir(dir)
        .unwrap_or_else(|e| panic!("could not read fixture directory {}: {}", dir.display(), e));
    for entry in entries {
        let entry = entry.expect("directory entry");
        let path = entry.path();
        if entry.file_type().expect("file type").is_dir() {
            collect_tree(root, &path, out);
        } else {
            let relative = path
                .strip_prefix(root)
                .expect("under root")
                .to_string_lossy()
                .replace('\\', "/");
            out.push((relative, read(&path)));
        }
    }
}

fn read(path: &Path) -> String {
    std::fs::read_to_string(path)
        .unwrap_or_else(|e| panic!("could not read {}: {}", path.display(), e))
}

#[test]
fn every_fixture_produces_its_recorded_output() {
    let fixtures = collect_fixtures();
    assert!(
        !fixtures.is_empty(),
        "no fixtures found in {} -- the runner would pass vacuously",
        programs_dir().display()
    );

    let mut failures: Vec<String> = Vec::new();
    for fixture in &fixtures {
        let mut program = Program::new(&fixture.source);
        for (relative, contents) in &fixture.extra {
            program = program.file(relative, contents);
        }
        for arg in &fixture.expectation.args {
            program = program.arg(arg);
        }
        if let Some(stdin) = &fixture.expectation.stdin {
            program = program.stdin(stdin);
        }
        let run = program.run();

        let mut problems: Vec<String> = Vec::new();
        if run.timed_out {
            problems.push("timed out".to_string());
        }
        if run.status != Some(fixture.expectation.exit) {
            problems.push(format!(
                "exit {:?}, expected {}",
                run.status, fixture.expectation.exit
            ));
        }
        // Both sides normalised: `Run` already converts CRLF, and a `.expected` file
        // checked out on Windows may carry CRLF of its own.
        let expected = fixture.expectation.stdout.replace("\r\n", "\n");
        if run.stdout.trim_end() != expected.trim_end() {
            problems.push(format!(
                "stdout mismatch\n      actual:   {:?}\n      expected: {:?}",
                run.stdout.trim_end(),
                expected.trim_end()
            ));
        }
        if let Some(needle) = &fixture.expectation.stderr_contains
            && !run.stderr.contains(needle)
        {
            problems.push(format!(
                "stderr should contain {:?} but was {:?}",
                needle, run.stderr
            ));
        }

        if !problems.is_empty() {
            failures.push(format!("  {}: {}", fixture.name, problems.join("; ")));
        }
    }

    assert!(
        failures.is_empty(),
        "{} of {} fixtures failed:\n{}",
        failures.len(),
        fixtures.len(),
        failures.join("\n")
    );
}
