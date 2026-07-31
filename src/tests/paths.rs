//! Unit tests for the path layer in [`crate::system`], calling the Rust functions
//! directly. The interesting cases are shapes from a platform the developer is not
//! on: a drive letter, a UNC share, a verbatim `\\?\` prefix. Anything
//! OS-dependent is asserted per platform with `cfg!`.

use crate::system;
use std::path::Path;

// ---------------------------------------------------------------------------
// Decomposition
// ---------------------------------------------------------------------------

#[test]
fn basename_takes_the_final_component() {
    assert_eq!(system::basename("file.txt"), "file.txt");
    assert_eq!(system::basename("a/b/c.txt"), "c.txt");
    assert_eq!(system::basename("a/b/c"), "c");
    // A trailing separator has no final component to name.
    assert_eq!(system::basename("a/b/"), "b");
    assert_eq!(system::basename(""), "");
    assert_eq!(system::basename("/"), "");
}

#[test]
fn dirname_drops_the_final_component() {
    assert_eq!(system::dirname("a/b/c.txt"), "a/b");
    // A bare name has no directory part.
    assert_eq!(system::dirname("file.txt"), "");
    assert_eq!(system::dirname(""), "");
    assert_eq!(system::dirname("/"), "");
}

#[test]
fn extension_is_the_last_suffix_without_its_dot() {
    assert_eq!(system::extension("file.txt"), "txt");
    assert_eq!(system::extension("archive.tar.gz"), "gz");
    assert_eq!(system::extension("noextension"), "");
    assert_eq!(system::extension("a/b.c/d"), "");
    // A leading dot makes a hidden file, not an extension -- the same rule
    // `Path::extension` and `os.path.splitext` both use.
    assert_eq!(system::extension(".hidden"), "");
    assert_eq!(system::extension(".hidden.txt"), "txt");
    assert_eq!(system::extension("trailing."), "");
}

#[test]
fn decomposition_round_trips_through_join() {
    let joined = system::join_paths(&["one".into(), "two".into(), "three.txt".into()]);
    assert_eq!(system::basename(&joined), "three.txt");
    assert_eq!(system::extension(&joined), "txt");
    assert_eq!(system::basename(&system::dirname(&joined)), "two");
}

// ---------------------------------------------------------------------------
// Joining
// ---------------------------------------------------------------------------

#[test]
fn join_uses_the_host_separator() {
    let joined = system::join_paths(&["a".into(), "b".into()]);
    if cfg!(target_os = "windows") {
        assert_eq!(joined, "a\\b");
    } else {
        assert_eq!(joined, "a/b");
    }
}

#[test]
fn join_of_one_segment_is_that_segment() {
    assert_eq!(system::join_paths(&["solo.txt".into()]), "solo.txt");
}

#[test]
fn join_of_nothing_is_empty() {
    assert_eq!(system::join_paths(&[]), "");
}

#[test]
fn an_absolute_later_segment_replaces_what_came_before() {
    // `PathBuf::push` semantics, shared with `os.path.join`: an absolute segment
    // discards the prefix. Worth pinning down, because a program building a path
    // from user input can hit it.
    let joined = if cfg!(target_os = "windows") {
        system::join_paths(&["a".into(), "C:\\b".into()])
    } else {
        system::join_paths(&["a".into(), "/b".into()])
    };
    assert!(
        Path::new(&joined).is_absolute(),
        "expected an absolute path, got {:?}",
        joined
    );
    assert!(
        !joined.starts_with('a'),
        "the earlier segment should have been discarded, got {:?}",
        joined
    );
}

#[test]
fn join_keeps_an_empty_segment_harmless() {
    assert_eq!(
        system::basename(&system::join_paths(&["a".into(), "".into(), "b".into()])),
        "b"
    );
}

// ---------------------------------------------------------------------------
// Absolute paths
// ---------------------------------------------------------------------------

#[test]
fn abspath_leaves_an_absolute_path_alone() {
    let absolute = std::env::temp_dir().join("psl-abspath-check.txt");
    let text = absolute.to_string_lossy().into_owned();
    assert_eq!(system::abspath(&text).expect("abspath"), text);
}

#[test]
fn abspath_makes_a_relative_path_absolute_without_requiring_it_to_exist() {
    let resolved = system::abspath("definitely-not-here-xyz.txt").expect("abspath");
    assert!(
        Path::new(&resolved).is_absolute(),
        "not absolute: {}",
        resolved
    );
    assert_eq!(system::basename(&resolved), "definitely-not-here-xyz.txt");
    assert!(!Path::new(&resolved).exists());
}

#[test]
fn realpath_resolves_an_existing_file_and_refuses_a_missing_one() {
    let dir = std::env::temp_dir().join(format!("psl-realpath-{}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("create dir");
    let file = dir.join("real.txt");
    std::fs::write(&file, "x").expect("write");

    let resolved = system::realpath(&file.to_string_lossy()).expect("realpath");
    assert_eq!(system::basename(&resolved), "real.txt");
    assert!(Path::new(&resolved).is_absolute());
    // Whatever canonicalisation did, the result must still name the same file.
    assert!(Path::new(&resolved).is_file());

    let missing = dir.join("ghost.txt");
    assert!(system::realpath(&missing.to_string_lossy()).is_err());

    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn realpath_output_carries_no_verbatim_prefix() {
    // On Windows `canonicalize` returns `\\?\C:\...`, which is correct but leaks
    // into program output. The path handed back to a program must be the ordinary
    // spelling.
    let dir = std::env::temp_dir();
    let resolved = system::realpath(&dir.to_string_lossy()).expect("realpath of temp dir");
    assert!(
        !resolved.starts_with(r"\\?\"),
        "verbatim prefix leaked: {}",
        resolved
    );
}

// ---------------------------------------------------------------------------
// strip_unc, exercised on all three platforms
// ---------------------------------------------------------------------------

#[test]
fn strip_unc_removes_a_verbatim_drive_prefix() {
    assert_eq!(
        system::strip_unc(Path::new(r"\\?\C:\Users\test\file.txt")),
        r"C:\Users\test\file.txt"
    );
}

#[test]
fn strip_unc_rewrites_a_share_to_its_ordinary_spelling() {
    // Dropping the prefix would give `UNC\server\share`, a relative path naming
    // nothing. The ordinary form of a share is `\\server\share`.
    assert_eq!(
        system::strip_unc(Path::new(r"\\?\UNC\server\share\file.txt")),
        r"\\server\share\file.txt"
    );
    assert_eq!(
        system::strip_unc(Path::new(r"\\?\UNC\server\share")),
        r"\\server\share"
    );
}

#[test]
fn strip_unc_leaves_an_ordinary_path_untouched() {
    assert_eq!(
        system::strip_unc(Path::new("/usr/local/bin")),
        "/usr/local/bin"
    );
    assert_eq!(
        system::strip_unc(Path::new(r"C:\Users\test")),
        r"C:\Users\test"
    );
    assert_eq!(
        system::strip_unc(Path::new("relative/path")),
        "relative/path"
    );
    assert_eq!(system::strip_unc(Path::new("")), "");
}

// ---------------------------------------------------------------------------
// Predicates
// ---------------------------------------------------------------------------

#[test]
fn is_file_and_is_dir_distinguish_the_two_and_agree_on_absence() {
    let dir = std::env::temp_dir().join(format!("psl-predicates-{}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("create dir");
    let file = dir.join("f.txt");
    std::fs::write(&file, "x").expect("write");

    assert!(system::is_file(&file.to_string_lossy()));
    assert!(!system::is_dir(&file.to_string_lossy()));
    assert!(system::is_dir(&dir.to_string_lossy()));
    assert!(!system::is_file(&dir.to_string_lossy()));

    let missing = dir.join("nope");
    assert!(!system::is_file(&missing.to_string_lossy()));
    assert!(!system::is_dir(&missing.to_string_lossy()));

    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn the_cheap_cpu_and_uptime_accessors_report_plausible_values() {
    // Not compared against `machine_info`: that delegates to these, so the
    // comparison would be a function against itself.
    assert!(system::logical_cpus() >= 1);
    assert!(system::logical_cpus() <= 4096);
    if let Some(physical) = system::physical_cpus() {
        assert!(physical >= 1 && physical <= system::logical_cpus().max(physical));
    }
    assert!(system::uptime_seconds() > 0);
}

#[test]
fn temp_dir_is_a_real_directory() {
    assert!(system::is_dir(&system::temp_dir()));
}

// ---------------------------------------------------------------------------
// Compile-time machine facts
// ---------------------------------------------------------------------------

#[test]
fn platform_arch_and_family_match_the_build_target() {
    assert_eq!(system::platform(), std::env::consts::OS);
    assert_eq!(system::arch(), std::env::consts::ARCH);
    assert_eq!(system::family(), std::env::consts::FAMILY);
    // The value PseudoLang programs branch on must be one of the three.
    assert!(
        matches!(system::family(), "unix" | "windows" | "wasm"),
        "unexpected family: {}",
        system::family()
    );
}

#[test]
fn the_interpreter_version_is_the_crate_version() {
    assert_eq!(system::interpreter_version(), env!("CARGO_PKG_VERSION"));
}

// ---------------------------------------------------------------------------
// Environment variable name validation
// ---------------------------------------------------------------------------

#[test]
fn an_unusable_env_var_name_is_rejected_rather_than_passed_to_the_os() {
    // An empty name, or one containing `=` or NUL, makes `std::env::set_var`
    // panic. It has to be refused before it gets there.
    for bad in ["", "HAS=EQUALS", "HAS\0NUL"] {
        assert!(
            system::set_env_var(bad, "value").is_err(),
            "should have rejected {:?}",
            bad
        );
        assert!(
            system::unset_env_var(bad).is_err(),
            "should have rejected {:?}",
            bad
        );
    }
}

#[test]
fn a_value_containing_nul_is_rejected() {
    assert!(system::set_env_var("PSL_NUL_TEST", "has\0nul").is_err());
}

#[test]
fn env_vars_is_sorted_by_name() {
    let vars = system::env_vars();
    let mut sorted = vars.clone();
    sorted.sort_by(|a, b| a.0.cmp(&b.0));
    assert_eq!(
        vars.iter().map(|(k, _)| k).collect::<Vec<_>>(),
        sorted.iter().map(|(k, _)| k).collect::<Vec<_>>()
    );
}

// ---------------------------------------------------------------------------
// Machine probes: shape only, since the values are the machine's
// ---------------------------------------------------------------------------

#[test]
fn machine_info_reports_plausible_numbers() {
    let info = system::machine_info();
    assert!(info.logical_cpus >= 1);
    assert!(info.total_memory > 0, "total memory should be known");
    assert!(
        info.used_memory <= info.total_memory,
        "used ({}) exceeded total ({})",
        info.used_memory,
        info.total_memory
    );
    assert!(info.uptime_seconds > 0);
    if let Some(physical) = info.physical_cpus {
        assert!(physical >= 1);
    }
}

#[test]
fn current_pid_is_this_process() {
    assert_eq!(system::current_pid(), std::process::id());
}

#[test]
fn process_info_describes_this_process_and_nothing_for_an_absent_pid() {
    let me = system::process_info(std::process::id()).expect("our own process should be visible");
    assert_eq!(me.pid, std::process::id());
    assert!(!me.name.is_empty());
    assert!(me.memory_bytes > 0);

    assert!(system::process_info(u32::MAX - 1).is_none());
}

#[test]
fn killing_a_pid_that_does_not_exist_is_an_error_not_a_silent_false() {
    assert!(system::kill(u32::MAX - 1).is_err());
}

#[test]
fn user_dir_rejects_an_unknown_kind() {
    assert!(system::user_dir("not-a-kind").is_err());
}

#[test]
fn the_home_directory_is_a_real_directory() {
    let home = system::user_dir("home").expect("a home directory");
    assert!(system::is_dir(&home), "not a directory: {}", home);
}
