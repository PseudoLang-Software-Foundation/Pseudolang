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

## Build / CI Pipeline

CI is defined in `.github/workflows/build.yml`. On every push it runs tests on Linux and macOS, then builds these targets in parallel:

1. **Windows** (`x86_64-pc-windows-gnu`) -- cross-compiled, plus NSIS installer
2. **Linux** (`x86_64-unknown-linux-gnu` and `aarch64-unknown-linux-gnu`) -- cross-compiled, plus `.deb` packages
3. **macOS** (`aarch64-apple-darwin` and `x86_64-apple-darwin`) -- built on a macOS runner, then combined into a universal binary with `lipo`
4. **WASM** (`wasm32-unknown-unknown`) -- wasm-pack/wasm-bindgen bundle for browser embedding
5. **WASI** (`wasm32-wasip1`) -- standalone CLI binary for runtimes like webassembly.sh

macOS cannot be cross-compiled with `cross` (there are no darwin images), so it builds natively on a `macos-15` runner.

Pushing a `vX.Y.Z` tag triggers a GitHub Release with all artifacts attached.

## Examples

[Pseudolang.md](Pseudolang.md) contains a full explanation of Collegeboard's Pseudocode and many features specific to PseudoLang.

The file `src/tests/mod.rs` also contains various unit tests (examples of code) for PseudoLang.

## To-do

- [ ] Proper documentation (integrate into web IDE site via Astro)

<details>
<summary>Functionality</summary>

- [x] Dictionaries
- [ ] Networking
- [ ] File IO
- [ ] System integration (terminal commands, process management, environment variables)
- [ ] Library support (remote procedures)
- [ ] Graphics
- [ ] Meta programming
- [ ] Multithreading
- [ ] Bundled compiler
- [ ] Dunder info like python
- [ ] Env information

<details>
<summary>Misc</summary>

- [ ] Testing for INPUT and SLEEP (mocking framework)
- [ ] More escape characters

</details>
</details>

## Issues

Feel free to make issues for any bugs or trouble that you experience with this! Especially since this is new, and there are going to be a lot of them!

## Contributing

We welcome contributions! If there are any bugs, or particularly pointing out limitations in [Pseudolang.md](Pseudolang.md) at the bottom, or adding things from the to-do list, please make a pull request!

## License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.
