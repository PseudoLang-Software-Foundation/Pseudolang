//! Runs the real `fpli` binary as a child process, one process per test.
//!
//! The in-process suite under `src/tests/` calls the interpreter as a library and
//! cannot reach:
//!
//! * `EXIT` -- `std::process::exit` would end the test binary.
//! * `INPUT` -- a library call has no stdin to read.
//! * `SLEEP` and flushing -- real time and real file descriptors.
//! * `CHDIR` and `SETENV` -- process-global, and `cargo test` uses parallel threads.
//! * Exit statuses, stderr format, CLI arguments -- all in `main.rs`.
//! * `IMPORT` resolution -- defined as differing from the working directory.
//! * `OutputMode::Stdout` -- the library tests always capture.
//!
//! ```ignore
//! Program::new("DISPLAY(INPUT())").stdin("hello\n").run().success().stdout_is("hello");
//! ```
//!
//! For a plain "this program prints that" case, use a fixture instead. See
//! `programs.rs`.

#![allow(dead_code)] // Each test module uses a different part of the builder.

use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::{Duration, Instant};

/// Path to the binary under test. Cargo builds it for us and hands over the path
/// at compile time, so this is always the current code and never a stale install.
const FPLI: &str = env!("CARGO_BIN_EXE_fpli");

/// How long a test program may run before it is killed and the test fails.
///
/// A hung child would otherwise hang CI until the job timeout. Generous enough
/// that a debug-build program doing real work is never cut off by accident.
const DEFAULT_TIMEOUT: Duration = Duration::from_secs(30);

/// A temporary directory that deletes itself when dropped.
///
/// Named from the process id and a counter, so it is unique both between the
/// parallel threads of one run and between concurrent runs of the suite.
pub struct ScratchDir {
    path: PathBuf,
}

impl ScratchDir {
    pub fn new() -> Self {
        static COUNTER: AtomicUsize = AtomicUsize::new(0);
        let n = COUNTER.fetch_add(1, Ordering::Relaxed);
        let path = std::env::temp_dir().join(format!("psl-it-{}-{}", std::process::id(), n));
        std::fs::create_dir_all(&path).expect("create scratch dir");
        ScratchDir { path }
    }

    pub fn path(&self) -> &Path {
        &self.path
    }
}

impl Drop for ScratchDir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.path);
    }
}

/// A program to run, plus the world to run it in.
pub struct Program {
    entry: String,
    source: String,
    files: Vec<(String, String)>,
    /// Flags for `fpli` itself, placed before the `run` subcommand.
    flags: Vec<String>,
    /// Arguments for the program, placed after the `.psl` path.
    args: Vec<String>,
    stdin: Option<String>,
    env_set: Vec<(String, std::ffi::OsString)>,
    env_clear: Vec<String>,
    /// Working directory, relative to the scratch root. `None` means the root.
    working_dir: Option<String>,
    /// Pass the entry file to `fpli` as an absolute path rather than a relative
    /// one, which is how a program launched from elsewhere sees itself.
    absolute_entry: bool,
    timeout: Duration,
}

impl Program {
    /// A program whose source is `source`, written to `main.psl`.
    pub fn new(source: &str) -> Self {
        Program {
            entry: "main.psl".to_string(),
            source: source.to_string(),
            files: Vec::new(),
            flags: Vec::new(),
            args: Vec::new(),
            stdin: None,
            env_set: Vec::new(),
            env_clear: Vec::new(),
            working_dir: None,
            absolute_entry: false,
            timeout: DEFAULT_TIMEOUT,
        }
    }

    /// Put the entry file somewhere other than the scratch root. The path is
    /// relative to the root and may name directories, which are created.
    pub fn entry_at(mut self, relative_path: &str) -> Self {
        self.entry = relative_path.to_string();
        self
    }

    /// Write an extra file, relative to the scratch root. Parent directories are
    /// created. Use this for the libraries an `IMPORT` is meant to find.
    pub fn file(mut self, relative_path: &str, contents: &str) -> Self {
        self.files
            .push((relative_path.to_string(), contents.to_string()));
        self
    }

    /// Run from this directory instead of the scratch root, so that a test can
    /// prove something resolves against the script rather than the cwd.
    pub fn working_dir(mut self, relative_path: &str) -> Self {
        self.working_dir = Some(relative_path.to_string());
        self
    }

    /// Name the entry file by absolute path on the command line.
    pub fn absolute_entry(mut self) -> Self {
        self.absolute_entry = true;
        self
    }

    /// An argument for the *program*, after the `.psl` path.
    pub fn arg(mut self, arg: &str) -> Self {
        self.args.push(arg.to_string());
        self
    }

    /// A flag for `fpli` itself, before the subcommand.
    pub fn flag(mut self, flag: &str) -> Self {
        self.flags.push(flag.to_string());
        self
    }

    pub fn stdin(mut self, text: &str) -> Self {
        self.stdin = Some(text.to_string());
        self
    }

    pub fn env(mut self, name: &str, value: &str) -> Self {
        self.env_set
            .push((name.into(), std::ffi::OsString::from(value)));
        self
    }

    /// Set a variable to bytes that need not be valid Unicode, for the paths that
    /// have to survive an undecodable environment.
    pub fn raw_env(mut self, name: &str, value: std::ffi::OsString) -> Self {
        self.env_set.push((name.into(), value));
        self
    }

    /// Remove a variable from the child's environment, so a test can assert on
    /// what happens when it is genuinely unset whatever the developer's shell has.
    pub fn without_env(mut self, name: &str) -> Self {
        self.env_clear.push(name.to_string());
        self
    }

    pub fn timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }

    /// Write the files, run the program, and collect everything it did.
    pub fn run(self) -> Run {
        let dir = ScratchDir::new();
        write_file(&dir.path().join(&self.entry), &self.source);
        for (name, contents) in &self.files {
            write_file(&dir.path().join(name), contents);
        }

        let cwd = match &self.working_dir {
            Some(relative) => {
                let path = dir.path().join(relative);
                std::fs::create_dir_all(&path).expect("create working dir");
                path
            }
            None => dir.path().to_path_buf(),
        };

        let entry_arg = if self.absolute_entry {
            dir.path().join(&self.entry).to_string_lossy().into_owned()
        } else {
            // Relative to the working directory, which is what a user typing
            // `fpli run main.psl` gets.
            relative_from(&cwd, &dir.path().join(&self.entry))
        };

        let mut command = Command::new(FPLI);
        command.current_dir(&cwd);
        command.args(&self.flags);
        command.arg("run").arg(&entry_arg);
        command.args(&self.args);
        for (name, value) in &self.env_set {
            command.env(name, value);
        }
        for name in &self.env_clear {
            command.env_remove(name);
        }
        // stdin is always a pipe, never inherited: a program that reads INPUT must
        // see a controlled stream (or a clean EOF), not the terminal running the
        // test suite.
        command
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        let child = command
            .spawn()
            .unwrap_or_else(|e| panic!("could not start {}: {}", FPLI, e));
        let raw = wait_with_timeout(child, self.stdin, self.timeout);

        Run {
            dir,
            command_line: format!("{} run {}", "fpli", entry_arg),
            status: raw.status,
            timed_out: raw.timed_out,
            stdout: normalise(&raw.stdout),
            stderr: normalise(&raw.stderr),
            elapsed: raw.elapsed,
        }
    }
}

fn write_file(path: &Path, contents: &str) {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).expect("create parent dir");
    }
    std::fs::write(path, contents).expect("write test file");
}

/// `target` expressed relative to `base` when it is underneath it, else absolute.
fn relative_from(base: &Path, target: &Path) -> String {
    match target.strip_prefix(base) {
        Ok(relative) => relative.to_string_lossy().into_owned(),
        Err(_) => target.to_string_lossy().into_owned(),
    }
}

/// Windows tools may emit `\r\n`; assertions are written against `\n`.
fn normalise(text: &str) -> String {
    text.replace("\r\n", "\n")
}

struct RawOutput {
    status: Option<i32>,
    timed_out: bool,
    stdout: String,
    stderr: String,
    elapsed: Duration,
}

/// Feed stdin, drain both output pipes, and wait -- killing the child if it
/// outstays `timeout`.
///
/// stdin is written and the pipes are drained on their own threads. Doing any of
/// the three on this thread would deadlock as soon as a program's output filled
/// the pipe buffer while we were still blocked writing its input.
fn wait_with_timeout(mut child: Child, stdin_data: Option<String>, timeout: Duration) -> RawOutput {
    let mut stdin = child.stdin.take().expect("piped stdin");
    let mut stdout = child.stdout.take().expect("piped stdout");
    let mut stderr = child.stderr.take().expect("piped stderr");

    let writer = std::thread::spawn(move || {
        if let Some(data) = stdin_data {
            let _ = stdin.write_all(data.as_bytes());
            let _ = stdin.flush();
        }
        // Closing the pipe is what turns a further INPUT into EOF instead of a
        // wait that never ends.
        drop(stdin);
    });
    let out_reader = std::thread::spawn(move || read_lossy(&mut stdout));
    let err_reader = std::thread::spawn(move || read_lossy(&mut stderr));

    let start = Instant::now();
    let mut timed_out = false;
    let status = loop {
        match child.try_wait().expect("try_wait on child") {
            Some(status) => break status.code(),
            None => {
                if start.elapsed() >= timeout {
                    let _ = child.kill();
                    let _ = child.wait();
                    timed_out = true;
                    break None;
                }
                std::thread::sleep(Duration::from_millis(10));
            }
        }
    };
    let elapsed = start.elapsed();

    let _ = writer.join();
    let stdout = out_reader.join().unwrap_or_default();
    let stderr = err_reader.join().unwrap_or_default();

    RawOutput {
        status,
        timed_out,
        stdout,
        stderr,
        elapsed,
    }
}

fn read_lossy(source: &mut impl Read) -> String {
    let mut bytes = Vec::new();
    let _ = source.read_to_end(&mut bytes);
    String::from_utf8_lossy(&bytes).into_owned()
}

/// What a run produced. Every assertion returns `&Self` so they chain, and every
/// failure message carries the whole run so a red test explains itself.
pub struct Run {
    /// Held so the scratch directory outlives the assertions that read from it.
    dir: ScratchDir,
    command_line: String,
    /// `None` when the program was killed by a signal or by our own timeout.
    pub status: Option<i32>,
    pub timed_out: bool,
    pub stdout: String,
    pub stderr: String,
    pub elapsed: Duration,
}

impl Run {
    fn report(&self) -> String {
        format!(
            "\n  command: {}\n  exit:    {}\n  stdout:  {:?}\n  stderr:  {:?}\n  elapsed: {:?}",
            self.command_line,
            match self.status {
                Some(code) => code.to_string(),
                None if self.timed_out => "killed after timeout".to_string(),
                None => "terminated by signal".to_string(),
            },
            self.stdout,
            self.stderr,
            self.elapsed
        )
    }

    fn assert_not_timed_out(&self) {
        assert!(
            !self.timed_out,
            "program did not finish within the timeout{}",
            self.report()
        );
    }

    /// Exited 0.
    pub fn success(&self) -> &Self {
        self.assert_not_timed_out();
        assert_eq!(self.status, Some(0), "expected success{}", self.report());
        self
    }

    /// Exited with a specific status.
    pub fn code(&self, expected: i32) -> &Self {
        self.assert_not_timed_out();
        assert_eq!(
            self.status,
            Some(expected),
            "expected exit code {}{}",
            expected,
            self.report()
        );
        self
    }

    /// Exited non-zero.
    pub fn failed(&self) -> &Self {
        self.assert_not_timed_out();
        assert!(
            self.status != Some(0),
            "expected a failure exit{}",
            self.report()
        );
        self
    }

    /// stdout equals `expected`, ignoring trailing blank space on either side.
    pub fn stdout_is(&self, expected: &str) -> &Self {
        self.assert_not_timed_out();
        assert_eq!(
            self.stdout.trim_end(),
            expected.trim_end(),
            "stdout mismatch{}",
            self.report()
        );
        self
    }

    pub fn stdout_contains(&self, needle: &str) -> &Self {
        assert!(
            self.stdout.contains(needle),
            "stdout should contain {:?}{}",
            needle,
            self.report()
        );
        self
    }

    pub fn stdout_excludes(&self, needle: &str) -> &Self {
        assert!(
            !self.stdout.contains(needle),
            "stdout should not contain {:?}{}",
            needle,
            self.report()
        );
        self
    }

    pub fn stdout_is_empty(&self) -> &Self {
        assert!(
            self.stdout.trim().is_empty(),
            "expected no stdout{}",
            self.report()
        );
        self
    }

    pub fn stderr_contains(&self, needle: &str) -> &Self {
        assert!(
            self.stderr.contains(needle),
            "stderr should contain {:?}{}",
            needle,
            self.report()
        );
        self
    }

    pub fn stderr_is_empty(&self) -> &Self {
        assert!(
            self.stderr.trim().is_empty(),
            "expected no stderr{}",
            self.report()
        );
        self
    }

    pub fn lines(&self) -> Vec<&str> {
        self.stdout.trim_end().lines().collect()
    }

    /// The program really did take at least this long -- for `SLEEP`.
    pub fn took_at_least(&self, minimum: Duration) -> &Self {
        assert!(
            self.elapsed >= minimum,
            "expected to take at least {:?}{}",
            minimum,
            self.report()
        );
        self
    }

    pub fn took_less_than(&self, maximum: Duration) -> &Self {
        assert!(
            self.elapsed < maximum,
            "expected to take less than {:?}{}",
            maximum,
            self.report()
        );
        self
    }

    /// A path inside the scratch directory, for inspecting what the program wrote.
    pub fn path(&self, relative: &str) -> PathBuf {
        self.dir.path().join(relative)
    }

    /// The contents of a file the program wrote.
    pub fn file(&self, relative: &str) -> String {
        std::fs::read_to_string(self.path(relative))
            .unwrap_or_else(|e| panic!("could not read {}: {}{}", relative, e, self.report()))
    }

    pub fn file_exists(&self, relative: &str) -> bool {
        self.path(relative).exists()
    }
}
