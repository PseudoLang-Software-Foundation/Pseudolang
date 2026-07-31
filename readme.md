<div align="center">
  <h1><a href="https://pseudo-lang.org/">Pseudolang</a></h1>
</div>

<p align="center">
    <img src="https://raw.githubusercontent.com/PseudoLang-Software-Foundation/Pseudolang/main/assets/Pseudolang-Logo.png" alt="Pseudolang Logo" height="200px" width="auto">
</p>

<div align="center">
  <p>
    <img src="https://github.com/PseudoLang-Software-Foundation/Pseudolang/actions/workflows/build.yml/badge.svg" alt="Build and Test Pseudolang">
    <img src="https://img.shields.io/github/v/release/PseudoLang-Software-Foundation/Pseudolang?color=green&label=Version" alt="Version">
    <a href="https://nightly.link/PseudoLang-Software-Foundation/Pseudolang/workflows/build/main"><img src="https://img.shields.io/badge/Nightly-Releases-purple" alt="Nightly Releases"></a>
  </p>
</div>

Welcome to Pseudolang! Pseudolang is a simple programming language written in Rust, inspired by College Board's Pseudocode.

This project aims to fully support 64-bit Windows, Linux, macOS (Apple Silicon and Intel), and WebAssembly (WASI CLI, wasm-bindgen for browser).

## Screenshots

<p align="center">
  <img src="https://raw.githubusercontent.com/PseudoLang-Software-Foundation/Pseudolang/main/assets/fib_psl.png" alt="Fibonacci Example in Pseudolang" height="auto" width="auto">
</p>

<p align="center">
  <img src="https://raw.githubusercontent.com/PseudoLang-Software-Foundation/Pseudolang/main/assets/web.png" alt="Pseudolang Web Interpreter" height="auto" width="auto">
</p>

## Install

From [crates.io](https://crates.io/crates/fpli), on any supported platform:

```bash
cargo install fpli
```

Or download a prebuilt binary from **[nightly releases](https://nightly.link/PseudoLang-Software-Foundation/Pseudolang/workflows/build/main)** or [GitHub releases](https://github.com/PseudoLang-Software-Foundation/Pseudolang/releases). Windows additionally has an **installer**, and Debian/Ubuntu have a `.deb` package.

### macOS

Releases ship `fpli-macos-arm64` (Apple Silicon), `fpli-macos-amd64` (Intel), and `fpli-macos-universal` (both). Download the tarball rather than the bare binary — it preserves the executable bit:

```bash
curl -LO https://github.com/PseudoLang-Software-Foundation/Pseudolang/releases/latest/download/fpli-macos-universal.tar.gz
shasum -a 256 -c fpli-macos-universal.tar.gz.sha256   # optional
tar -xzf fpli-macos-universal.tar.gz
sudo mv fpli-macos-universal /usr/local/bin/fpli
```

The binaries are ad-hoc signed but not notarized, so Gatekeeper will quarantine a download made through a browser. Clear it with `xattr -d com.apple.quarantine /usr/local/bin/fpli`, or avoid it entirely by using `curl` or `cargo install`.

## Use

Pseudolang programs use the `.psl` extension. Run them with the `fpli` CLI:

```
fpli run program.psl
fpli run --debug program.psl
```

If `fpli` is not in your PATH, run the binary directly (e.g. `./fpli run program.psl`).

Free Pseudolang Interpreter = fpli

## Building

You will need [Rust](https://www.rust-lang.org/tools/install) installed and added to PATH.

```bash
git clone https://github.com/PseudoLang-Software-Foundation/Pseudolang.git
cd Pseudolang
```

- **Debug build**: `cargo build --features native`
- **Release build**: `cargo build --release --features native`
- **Full release** (native + WASM + WASI, with optional cross-compilation): `just build-all` from `jfiles/src/`
- **Run tests**: `cargo test`

Cross-compilation for other platforms requires [`cross`](https://github.com/cross-rs/cross) and Docker.

## Just Commands

All recipes live in `jfiles/src/` and are run with [`just`](https://github.com/casey/just) from that directory.

| Command                   | Description                                                    |
| ------------------------- | -------------------------------------------------------------- |
| `just install`            | Install toolchain deps (cross, wasm-pack, taplo, WASM targets) |
| `just build`              | Debug build (native)                                           |
| `just release`            | Release build (native)                                         |
| `just build-macos`        | macOS arm64 release (macOS host only)                          |
| `just build-macos-intel`  | macOS x86_64 release (macOS host only)                         |
| `just build-macos-universal` | macOS universal binary via lipo (macOS host only)           |
| `just build-wasm`         | Browser WASM via wasm-pack/wasm-bindgen                        |
| `just build-wasi`         | WASI CLI binary (`wasm32-wasip1`)                              |
| `just build-all`          | Native release + WASM + WASI + optional cross-compilation      |
| `just test`               | Run all tests                                                  |
| `just test-verbose`       | Run tests with stdout                                          |
| `just run <ARGS>`         | Run fpli in debug mode                                         |
| `just run-release <ARGS>` | Run fpli in release mode                                       |
| `just fmt`                | Clippy fix + rustfmt + taplo fmt                               |
| `just fmt-check`          | Lint/format check (no changes)                                 |
| `just check`              | `cargo check`                                                  |
| `just clean`              | Remove build artifacts                                         |
| `just tag-release`        | Tag current version and push (triggers CI release)             |

## Testing

`cargo test` runs both suites.

**`src/tests/`** drives the interpreter as a library. Most tests belong here.

```rust
// src/tests/<area>.rs, declared as `mod <area>;` in src/tests/mod.rs
use super::{assert_output, get_error};

#[test]
fn test_addition() {
    assert_output("DISPLAY(1 + 1)", "2");
    assert!(get_error("DISPLAY(1 / 0)").contains("Division by zero"));
}
```

Helpers in `src/tests/mod.rs`: `assert_output`, `get_error`,
`assert_output_with_args`, `assert_output_at` / `get_error_at` (run as though
loaded from a given path, for `IMPORT` / `SCRIPTPATH` / `ISMAIN`), and `Scratch`,
a self-deleting temporary directory.

**`tests/integration/`** runs the built `fpli` binary as a child process, one
process and one temporary directory per test. Use it for `EXIT` and exit codes,
`INPUT`, `SLEEP` and flushing, `CHDIR` and `SETENV`, CLI arguments and stderr,
`IMPORT` resolution against a different working directory, and streamed stdout.
A library call cannot reach any of those.

```rust
// tests/integration/<area>.rs, declared as `mod <area>;` in tests/integration/main.rs
use crate::harness::Program;

#[test]
fn input_reads_stdin() {
    Program::new(r#"DISPLAY(INPUT())"#)
        .stdin("hello\n")
        .run()
        .success()
        .stdout_is("hello");
}

#[test]
fn exit_code_and_files_are_observable() {
    let run = Program::new(r#"WRITEFILE("out.txt", "x")
EXIT(3)"#)
        .file("lib.psl", "PROCEDURE p()\n{\n RETURN 1\n}")
        .arg("--mode")
        .env("KEY", "value")
        .working_dir("elsewhere")
        .run();
    run.code(3).stderr_is_empty();
    assert_eq!(run.file("out.txt"), "x");
}
```

A program that hangs is killed by a watchdog and reported as a failure.

**`tests/programs/`** needs no Rust. Drop in `<name>.psl` and `<name>.expected`,
or a directory holding `main.psl`, `expected`, and whatever it imports. `#`
directives at the top of the entry file set everything else:

```psl
# ARGS: --mode fast input.txt
# STDIN: 10
# STDIN: 20
# EXIT: 1
# STDERR: Division by zero
DISPLAY(TONUM(INPUT()) + TONUM(INPUT()))
```

## Build / CI Pipeline

CI is defined in `.github/workflows/build.yml`. On every push:

1. **Lint** -- `cargo fmt --check`, then `cargo clippy --all-targets --all-features -- -D warnings`
2. **Test** -- `cargo test` on `ubuntu-latest`, `macos-15` and `windows-latest`
3. **Check (WebAssembly)** -- `cargo check` for `wasm32-unknown-unknown` and `wasm32-wasip1`

Then it builds these targets in parallel:

1. **Windows** (`x86_64-pc-windows-gnu`) -- cross-compiled, plus NSIS installer
2. **Linux** (`x86_64-unknown-linux-gnu` and `aarch64-unknown-linux-gnu`) -- cross-compiled, plus `.deb` packages
3. **macOS** (`aarch64-apple-darwin` and `x86_64-apple-darwin`) -- built on a macOS runner, then combined into a universal binary with `lipo`
4. **WASM** (`wasm32-unknown-unknown`) -- wasm-pack/wasm-bindgen bundle for browser embedding
5. **WASI** (`wasm32-wasip1`) -- standalone CLI binary for runtimes like webassembly.sh

macOS cannot be cross-compiled with `cross` (there are no darwin images), so it builds natively on a `macos-15` runner.

Pushing a `vX.Y.Z` tag triggers a GitHub Release with all artifacts attached.

## Examples

[Pseudolang.md](Pseudolang.md) contains a full explanation of Collegeboard's Pseudocode and many features specific to PseudoLang.

`src/tests/` holds the language test suite, and `tests/programs/` holds runnable
example programs with their expected output -- both double as worked examples. See
[Testing](#testing).

## To-do

- [ ] Proper documentation (integrate into web IDE site via Astro)

<details>
<summary>Functionality</summary>

- [x] Dictionaries
- [ ] Networking
- [x] File IO
- [x] System integration (terminal commands, process management, environment variables)
- [x] Library support (multi-file programs, procedures from other files)
- [ ] Graphics
- [x] Meta programming
- [ ] Multithreading
- [ ] Bundled compiler
- [ ] Dunder info like python
- [x] Env information

<details>
<summary>Misc</summary>

- [x] Testing for INPUT and SLEEP (process-level integration harness)
- [ ] More escape characters

</details>
</details>

## Issues

Feel free to make issues for any bugs or trouble that you experience with this! Especially since this is new, and there are going to be a lot of them!

## Contributing

We welcome contributions! If there are any bugs, or particularly pointing out limitations in [Pseudolang.md](Pseudolang.md) at the bottom, or adding things from the to-do list, please make a pull request!

## License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.
