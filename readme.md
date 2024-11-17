<div align="center">
  <h1><a href="https://pseudo-lang.org/">PseudoLang</a></h1>
</div>

<p align="center">
    <img src="Pseudolang-Logo.png" alt="Pseudolang Logo" height="200px" width="auto">
</p>

<div align="center">
  <p>
    <img src="https://github.com/PseudoLang-Software-Foundation/Pseudolang/actions/workflows/build.yml/badge.svg" alt="Build and Test Pseudolang">
    <img src="https://img.shields.io/badge/Version-0.9.1-green" alt="Version">
    <a href="https://nightly.link/PseudoLang-Software-Foundation/Pseudolang/workflows/build/main">Nightly Link</a>
  </p>
</div>

Welcome to Pseudolang! Pseudolang is a simple programming language written in Rust, inspired by The College Board's Pseudocode

This project aims to fully support Windows and Linux.

## Releases

Goto [nightly releases](https://nightly.link/PseudoLang-Software-Foundation/Pseudolang/workflows/build/main) and download the binary for your operating system.

## Use

To use the compiled versions, run the executable and pass two parameters as the input and output file (pseudolang programs end with `.psl`). Ex: `./fplc main.psl main.exe`

It is highly recommended to add the executable to your PATH, so you can simply run `fplc`.

Free Pseudolang Compiler = fplc (like gcc :)

## Compiling

In order to compile the project yourself, you will need to have rust installed.

- Install [**rust**](https://www.rust-lang.org/tools/install), and make sure you have it added to PATH.
- Clone the repository `git clone https://github.com/Pseudolang-Software-Foundation/PseudoLang.git`
- - To build **release**, you will need bash (or you can translate the shell commands to your liking). Then run `./build_release.sh`. The binaries for each operating system will be in the `release` folder.
- - To build **debug**, simply run `cargo build`. The binary will be in the `target/debug` folder.
- In order to run the unit tests, simply run `cargo test`.

## Examples

[Pseudolang.md](Pseudolang.md) contains a full explanation of Pseudolang and features specific to PseudoLang.

The **examples** folder contains multiple example files for Pseudolang. It is also wowrth checking `mod.rs`, as it contains various unit tests for Pseudolang.

## Todo

- [ ] Web version
- [ ] Math functions
- [ ] Console input
- [ ] Compiler optimizations
- [ ] Installer
- [ ] Library support (remote procedures, calls, etc)
- [ ] Casting and more types (ex: doubles, n-dimensional arrays (matrices))
- [ ] Console input
- [ ] Networking
- [ ] Man page and and help menu on simple command run
- [ ] Examples
- [ ] VSCode syntax highlighter extension and runner
- [ ] GitHub issue template
- [ ] Better error handling (suggestions, etc)
- [ ] Code todos

## Issues

Feel free to make issues for any bugs or trouble that you experience with this! Especially since this is new, and there are going to be a lot of them!

## Contributing

We welcome contributions! If there are any bugs, or particularly pointing out limitations in [Pseudolang.md](Pseudolang.md) at the bottom, or adding things from the todo list, please make a pull request!

## License

This project is licensed under the MIT License - see the [LICENSE file](LICENSE) file for details.
