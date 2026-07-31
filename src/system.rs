//! The host-facing half of the standard library: the process environment, child
//! processes, filesystem paths and machine facts.
//!
//! Windows, macOS and Linux are all first-class. The work is delegated to `std`,
//! [`sysinfo`], [`which`] and [`dirs`]; the only `cfg` split is between "there is
//! a host" and WebAssembly.
//!
//! Errors come back as `String`, and the caller turns them into PseudoLang runtime
//! errors that TRY/CATCH can catch.

use std::path::{Path, PathBuf};

/// Message used by every operation that needs a host the target does not have.
#[cfg(target_arch = "wasm32")]
const NO_HOST: &str = "not supported in WebAssembly: there is no host process to reach";

// ---------------------------------------------------------------------------
// Environment variables
// ---------------------------------------------------------------------------
//
// Plain `std::env`, which compiles for every target. Under WASI these work
// against the real environment; in the browser, against an empty one.

pub fn env_var(name: &str) -> Option<String> {
    std::env::var(name).ok()
}

/// Set a variable for this process and everything it later spawns.
///
/// `std::env::set_var` is `unsafe` as of the 2024 edition because a concurrent
/// reader in another thread observes a torn environment. One evaluation is
/// single-threaded by construction -- values are held in `Rc<RefCell<..>>`, which is
/// not `Send` -- so nothing inside the interpreter races here. What this crate cannot
/// promise is the rest of the *process*: an embedder that runs a program on one
/// thread while another reads the environment is the unsound case, and it is the
/// embedder's to avoid. `fpli` itself is single-threaded, and the test suite keeps
/// every program that writes the environment in its own process for this reason.
pub fn set_env_var(name: &str, value: &str) -> Result<(), String> {
    if name.is_empty() || name.contains('=') || name.contains('\0') {
        return Err(format!(
            "'{}' is not a usable environment variable name (it must be non-empty and contain no '=' or NUL)",
            name
        ));
    }
    if value.contains('\0') {
        return Err("An environment variable value may not contain NUL".to_string());
    }
    // SAFETY: see the doc comment -- the interpreter is single-threaded.
    unsafe { std::env::set_var(name, value) };
    Ok(())
}

/// Remove a variable. Removing one that was never set is not an error, matching
/// the way `os.environ.pop(name, None)` is normally used.
pub fn unset_env_var(name: &str) -> Result<(), String> {
    if name.is_empty() || name.contains('=') || name.contains('\0') {
        return Err(format!(
            "'{}' is not a usable environment variable name (it must be non-empty and contain no '=' or NUL)",
            name
        ));
    }
    // SAFETY: as for `set_env_var` -- single-threaded interpreter.
    unsafe { std::env::remove_var(name) };
    Ok(())
}

/// Every variable, sorted by name so a program that lists the environment is
/// reproducible. Variables whose name or value is not UTF-8 are skipped: they
/// cannot be represented as a PseudoLang string.
pub fn env_vars() -> Vec<(String, String)> {
    // `env::vars` panics on a variable that is not valid Unicode, which would take
    // the whole interpreter down over a variable the program never asked for.
    // `vars_os` hands back the raw pairs so the undecodable ones can be dropped.
    let mut vars: Vec<(String, String)> = std::env::vars_os()
        .filter_map(|(name, value)| Some((name.into_string().ok()?, value.into_string().ok()?)))
        .collect();
    vars.sort_by(|a, b| a.0.cmp(&b.0));
    vars
}

// ---------------------------------------------------------------------------
// Working directory
// ---------------------------------------------------------------------------

pub fn cwd() -> Result<String, String> {
    std::env::current_dir()
        .map_err(|e| format!("Could not read the current directory: {}", e))
        .map(|p| p.to_string_lossy().into_owned())
}

pub fn chdir(path: &str) -> Result<(), String> {
    std::env::set_current_dir(path)
        .map_err(|e| format!("Could not change directory to '{}': {}", path, e))
}

// ---------------------------------------------------------------------------
// Paths
// ---------------------------------------------------------------------------
//
// Pure `std::path`: correct for Windows separators and POSIX ones without
// branching, and needing no host.

/// Join path segments with the host's separator.
pub fn join_paths(segments: &[String]) -> String {
    let mut joined = PathBuf::new();
    for segment in segments {
        joined.push(segment);
    }
    joined.to_string_lossy().into_owned()
}

/// The final component of a path (`"a/b/c.txt"` -> `"c.txt"`). A path that ends
/// in a separator or has no final component yields `""`.
pub fn basename(path: &str) -> String {
    Path::new(path)
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default()
}

/// Everything but the final component (`"a/b/c.txt"` -> `"a/b"`).
pub fn dirname(path: &str) -> String {
    Path::new(path)
        .parent()
        .map(|p| p.to_string_lossy().into_owned())
        .unwrap_or_default()
}

/// The extension without its dot (`"a/b.tar.gz"` -> `"gz"`), or `""` if there is
/// none.
pub fn extension(path: &str) -> String {
    Path::new(path)
        .extension()
        .map(|e| e.to_string_lossy().into_owned())
        .unwrap_or_default()
}

/// Resolve a path against the working directory. This does *not* require the
/// path to exist, so it can be used to build a destination before creating it;
/// it therefore also does not resolve symlinks or `..`.
pub fn abspath(path: &str) -> Result<String, String> {
    let p = Path::new(path);
    if p.is_absolute() {
        return Ok(p.to_string_lossy().into_owned());
    }
    let base = std::env::current_dir()
        .map_err(|e| format!("Could not read the current directory: {}", e))?;
    Ok(base.join(p).to_string_lossy().into_owned())
}

/// Resolve a path all the way to a real location, following symlinks. Requires
/// the path to exist.
pub fn realpath(path: &str) -> Result<String, String> {
    std::fs::canonicalize(path)
        .map_err(|e| format!("Could not resolve '{}': {}", path, e))
        .map(|p| strip_unc(&p))
}

/// Strip Windows' `\\?\` verbatim prefix from a canonicalised path.
///
/// `fs::canonicalize` returns verbatim paths on Windows, which are correct but
/// leak into program output and are rejected by some other tools. The prefix is
/// only removed for ordinary drive paths, where dropping it cannot change which
/// file is meant.
pub fn strip_unc(path: &Path) -> String {
    let text = path.to_string_lossy();
    let Some(rest) = text.strip_prefix(r"\\?\") else {
        return text.into_owned();
    };
    // A share is spelled `\\?\UNC\server\share` in verbatim form and
    // `\\server\share` in ordinary form, so the prefix is replaced rather than
    // dropped. Dropping it would give `UNC\server\share`, a relative path naming
    // nothing.
    match rest.strip_prefix(r"UNC\") {
        Some(share) => format!(r"\\{}", share),
        None => rest.to_string(),
    }
}

pub fn is_file(path: &str) -> bool {
    Path::new(path).is_file()
}

pub fn is_dir(path: &str) -> bool {
    Path::new(path).is_dir()
}

/// Whether the path is a symbolic link, without following it.
///
/// `is_file` and `is_dir` both follow links, so a link to a directory looks like a
/// directory to them. Removing one is a *file* operation, so the difference matters.
pub fn is_symlink(path: &str) -> bool {
    std::fs::symlink_metadata(path).is_ok_and(|meta| meta.file_type().is_symlink())
}

/// Whether two paths name the same file on disk.
///
/// Compared after resolving both, so a relative and an absolute spelling, or a path
/// reached through a symlink, are recognised as one file. `false` when either path
/// cannot be resolved, which for a copy means the destination does not exist yet.
pub fn is_same_file(left: &str, right: &str) -> bool {
    match (std::fs::canonicalize(left), std::fs::canonicalize(right)) {
        (Ok(left), Ok(right)) => left == right,
        _ => false,
    }
}

pub fn temp_dir() -> String {
    std::env::temp_dir().to_string_lossy().into_owned()
}

// ---------------------------------------------------------------------------
// Child processes
// ---------------------------------------------------------------------------

/// What a finished child process left behind.
pub struct CommandOutput {
    /// `None` when the child was killed by a signal instead of exiting.
    pub exit_code: Option<i32>,
    pub stdout: String,
    pub stderr: String,
}

/// Facts about one running process.
pub struct ProcessInfo {
    pub pid: u32,
    pub name: String,
    pub memory_bytes: u64,
    pub parent_pid: Option<u32>,
}

/// Everything [`machine_info`] can find out about the host.
///
/// Each field is optional where the platform may genuinely not know: `sysinfo`
/// reports `None` rather than guessing, and that distinction is worth keeping
/// all the way out to the language.
pub struct MachineInfo {
    pub platform: &'static str,
    pub arch: &'static str,
    pub family: &'static str,
    pub os_name: Option<String>,
    pub os_version: Option<String>,
    pub kernel_version: Option<String>,
    pub hostname: Option<String>,
    pub logical_cpus: usize,
    pub physical_cpus: Option<usize>,
    pub total_memory: u64,
    pub used_memory: u64,
    pub uptime_seconds: u64,
    pub username: Option<String>,
}

/// The OS name as PseudoLang reports it: `std`'s own constant, which is
/// `"windows"`, `"macos"`, `"linux"`, `"wasi"` and so on.
pub fn platform() -> &'static str {
    std::env::consts::OS
}

/// `"unix"`, `"windows"` or `"wasm"`.
pub fn family() -> &'static str {
    std::env::consts::FAMILY
}

pub fn arch() -> &'static str {
    std::env::consts::ARCH
}

/// The interpreter's own version.
pub fn interpreter_version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

/// The login name of the user running the program, read from the environment.
///
/// `USER` is the POSIX spelling and `USERNAME` the Windows one; checking both
/// covers all three platforms without a `cfg`.
pub fn username() -> Option<String> {
    env_var("USER").or_else(|| env_var("USERNAME"))
}

// --- Native: a real host is present -----------------------------------------

#[cfg(not(target_arch = "wasm32"))]
mod host {
    use super::{CommandOutput, MachineInfo, ProcessInfo};
    use std::process::Command;

    /// Turn a finished `std::process::Output` into our own shape.
    ///
    /// Output is decoded lossily on purpose: a command that emits a stray
    /// non-UTF-8 byte should still hand back the rest of its output instead of
    /// failing the whole call.
    fn collect(output: std::process::Output) -> CommandOutput {
        CommandOutput {
            exit_code: output.status.code(),
            stdout: String::from_utf8_lossy(&output.stdout).into_owned(),
            stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
        }
    }

    /// Run `program` with `args` directly, with no shell involved.
    ///
    /// Because the argument vector is passed through as-is, nothing in it is
    /// ever re-parsed: a filename containing a space, a quote or a `;` reaches
    /// the program exactly as written. This is the form to reach for by default;
    /// [`shell`] exists for when shell syntax is actually wanted.
    pub fn exec(program: &str, args: &[String]) -> Result<CommandOutput, String> {
        Command::new(program)
            .args(args)
            .output()
            .map(collect)
            .map_err(|e| format!("Could not run '{}': {}", program, e))
    }

    /// Run a command line through the platform's shell.
    ///
    /// `cmd.exe /C` on Windows and `sh -c` elsewhere, which is the same choice
    /// Python's `subprocess(shell=True)` makes.
    pub fn shell(command: &str) -> Result<CommandOutput, String> {
        let mut child = shell_command(command);
        child
            .output()
            .map(collect)
            .map_err(|e| format!("Could not run the shell command '{}': {}", command, e))
    }

    /// `sh -c <command>`.
    #[cfg(not(target_os = "windows"))]
    fn shell_command(command: &str) -> Command {
        let mut child = Command::new("sh");
        child.arg("-c").arg(command);
        child
    }

    /// `cmd /C "<command>"`, passed through without argument escaping.
    ///
    /// `Command::arg` applies the C runtime's quoting rules, which `cmd.exe` does
    /// not parse: a command containing a double quote arrives mangled. `raw_arg`
    /// appends the text verbatim, and the extra surrounding pair of quotes is what
    /// `cmd.exe` itself requires -- the recipe in the `raw_arg` documentation.
    #[cfg(target_os = "windows")]
    fn shell_command(command: &str) -> Command {
        use std::os::windows::process::CommandExt;
        // `Command::new("cmd")` searches the application directory before PATH, so a
        // `cmd.exe` sitting beside `fpli.exe` would be picked up instead of the real
        // one. `ComSpec` is where Windows records the command processor.
        let shell = std::env::var_os("ComSpec").unwrap_or_else(|| "cmd.exe".into());
        let mut child = Command::new(shell);
        child.arg("/C").raw_arg(format!("\"{}\"", command));
        child
    }

    /// Locate an executable on `PATH`, the way `shutil.which` does. Handles the
    /// `.exe`/`PATHEXT` lookup on Windows rather than assuming a bare name.
    pub fn which(program: &str) -> Option<String> {
        which::which(program)
            .ok()
            .map(|p| super::strip_unc(p.as_path()))
    }

    pub fn current_pid() -> u32 {
        std::process::id()
    }

    /// Build a `System` that knows about one process, or all of them.
    fn probe_processes(pid: Option<u32>) -> sysinfo::System {
        // Memory only. `refresh_processes` would use a kind that includes tasks,
        // which on Linux lists every *thread* as a separate process, plus cpu, disk
        // and executable data that no built-in exposes. Pid, parent, name and start
        // time come back regardless of the kind.
        let kind = sysinfo::ProcessRefreshKind::nothing().with_memory();
        let mut system = sysinfo::System::new();
        match pid {
            Some(pid) => {
                let pids = [sysinfo::Pid::from_u32(pid)];
                system.refresh_processes_specifics(
                    sysinfo::ProcessesToUpdate::Some(&pids),
                    true,
                    kind,
                );
            }
            None => {
                system.refresh_processes_specifics(sysinfo::ProcessesToUpdate::All, true, kind);
            }
        }
        system
    }

    fn describe(pid: sysinfo::Pid, process: &sysinfo::Process) -> ProcessInfo {
        ProcessInfo {
            pid: pid.as_u32(),
            name: process.name().to_string_lossy().into_owned(),
            memory_bytes: process.memory(),
            parent_pid: process.parent().map(|p| p.as_u32()),
        }
    }

    /// Facts about one process, or `None` if nothing is running under that pid.
    pub fn process_info(pid: u32) -> Option<ProcessInfo> {
        let system = probe_processes(Some(pid));
        let key = sysinfo::Pid::from_u32(pid);
        system.process(key).map(|p| describe(key, p))
    }

    /// Every process the current user can see, ordered by pid so the listing is
    /// reproducible.
    pub fn processes() -> Vec<ProcessInfo> {
        let system = probe_processes(None);
        let mut all: Vec<ProcessInfo> = system
            .processes()
            .iter()
            .map(|(pid, process)| describe(*pid, process))
            .collect();
        all.sort_by_key(|p| p.pid);
        all
    }

    /// Terminate a process. A force-kill, with no chance for the target to clean up:
    /// SIGKILL on Unix, and whatever `sysinfo` uses on Windows. `false` means the
    /// request was refused, usually because the process belongs to another user.
    pub fn kill(pid: u32) -> Result<bool, String> {
        let system = probe_processes(Some(pid));
        match system.process(sysinfo::Pid::from_u32(pid)) {
            Some(process) => Ok(process.kill()),
            None => Err(format!("No process is running with pid {}", pid)),
        }
    }

    /// Logical CPUs, without building a `System`.
    pub fn logical_cpus() -> usize {
        std::thread::available_parallelism()
            .map(|n| n.get())
            .unwrap_or(1)
    }

    pub fn physical_cpus() -> Option<usize> {
        sysinfo::System::physical_core_count()
    }

    pub fn uptime_seconds() -> u64 {
        sysinfo::System::uptime()
    }

    pub fn machine_info() -> MachineInfo {
        let mut system = sysinfo::System::new();
        system.refresh_memory();
        MachineInfo {
            platform: super::platform(),
            arch: super::arch(),
            family: super::family(),
            os_name: sysinfo::System::name(),
            os_version: sysinfo::System::long_os_version(),
            kernel_version: sysinfo::System::kernel_version(),
            hostname: sysinfo::System::host_name(),
            // `available_parallelism` is what a program should size a workload
            // against, and unlike a cpu list it needs no refresh pass.
            logical_cpus: logical_cpus(),
            physical_cpus: physical_cpus(),
            total_memory: system.total_memory(),
            used_memory: system.used_memory(),
            uptime_seconds: uptime_seconds(),
            username: super::username(),
        }
    }

    /// One of the well-known per-user directories, by role.
    ///
    /// `dirs` implements the actual platform conventions -- `%APPDATA%` on
    /// Windows, `~/Library/Application Support` on macOS, `$XDG_CONFIG_HOME` on
    /// Linux -- so PseudoLang does not have to.
    pub fn user_dir(kind: &str) -> Result<String, String> {
        let path = match kind {
            "home" => dirs::home_dir(),
            "config" => dirs::config_dir(),
            "cache" => dirs::cache_dir(),
            "data" => dirs::data_dir(),
            "document" => dirs::document_dir(),
            "download" => dirs::download_dir(),
            "desktop" => dirs::desktop_dir(),
            _ => return Err(format!("Unknown directory kind '{}'", kind)),
        };
        path.map(|p| p.to_string_lossy().into_owned())
            .ok_or_else(|| {
                format!(
                    "This platform has no {} directory for the current user",
                    kind
                )
            })
    }
}

// --- WebAssembly: no host to reach ------------------------------------------
//
// The signatures mirror the native module exactly. The interpreter has one code
// path, and the difference appears as a runtime error naming the reason.

#[cfg(target_arch = "wasm32")]
mod host {
    use super::{CommandOutput, MachineInfo, NO_HOST, ProcessInfo};

    pub fn exec(program: &str, _args: &[String]) -> Result<CommandOutput, String> {
        Err(format!("Running '{}' is {}", program, NO_HOST))
    }

    pub fn shell(_command: &str) -> Result<CommandOutput, String> {
        Err(format!("Shell commands are {}", NO_HOST))
    }

    pub fn which(_program: &str) -> Option<String> {
        None
    }

    pub fn current_pid() -> u32 {
        0
    }

    pub fn process_info(_pid: u32) -> Option<ProcessInfo> {
        None
    }

    pub fn processes() -> Vec<ProcessInfo> {
        Vec::new()
    }

    pub fn kill(_pid: u32) -> Result<bool, String> {
        Err(format!("Process management is {}", NO_HOST))
    }

    pub fn logical_cpus() -> usize {
        1
    }

    pub fn physical_cpus() -> Option<usize> {
        None
    }

    pub fn uptime_seconds() -> u64 {
        0
    }

    pub fn machine_info() -> MachineInfo {
        MachineInfo {
            platform: super::platform(),
            arch: super::arch(),
            family: super::family(),
            os_name: None,
            os_version: None,
            kernel_version: None,
            hostname: None,
            logical_cpus: 1,
            physical_cpus: None,
            total_memory: 0,
            used_memory: 0,
            uptime_seconds: 0,
            username: None,
        }
    }

    pub fn user_dir(kind: &str) -> Result<String, String> {
        Err(format!("The {} directory is {}", kind, NO_HOST))
    }
}

pub use host::{
    current_pid, exec, kill, logical_cpus, machine_info, physical_cpus, process_info, processes,
    shell, uptime_seconds, user_dir, which,
};
