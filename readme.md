<div align="center">
  <h1><a href="https://pseudo-lang.org/">Pseudolang</a></h1>
</div>

<p align="center">
    <img src="Pseudolang-Logo.png" alt="Pseudolang Logo" height="200px" width="auto">
</p>

<div align="center">
  <p>
    <img src="https://github.com/PseudoLang-Software-Foundation/Pseudolang/actions/workflows/build.yml/badge.svg" alt="Build and Test Pseudolang">
    <img src="https://img.shields.io/badge/Version-0.9.496-green" alt="Version">
    <a href="https://nightly.link/PseudoLang-Software-Foundation/Pseudolang/workflows/build/main"><img src="https://img.shields.io/badge/Nightly-Releases-purple" alt="Nightly Releases"></a>
  </p>
</div>

Welcome to Pseudolang! Pseudolang is a simple programming language written in Rust, inspired by College Board's Pseudocode.

This project aims to fully support Windows and Linux.

## Releases

Goto **[nightly releases](https://nightly.link/PseudoLang-Software-Foundation/Pseudolang/workflows/build/main)** and download the binary for your operating system.

There is also an **installer** you can download in [releases](https://github.com/PseudoLang-Software-Foundation/Pseudolang/releases).

## Use

To use the compiled versions, run the executable and pass two parameters as the input and output file (pseudolang programs end with `.psl`). Ex: `fplc run main.psl`

If `fplc` is not added to path or environment variables, make sure to execute the binary specifically.

Free Pseudolang Compiler = fplc (like gcc :)

## Compiling

In order to compile the project yourself, you will need to have rust installed.

- Install [**rust**](https://www.rust-lang.org/tools/install), and make sure you have it added to PATH.
- Clone the repository `git clone https://github.com/Pseudolang-Software-Foundation/PseudoLang.git`
- - To build **release**, you will need bash, cross (cargo install cross), and docker. Then run `./build_release.sh`. The binaries for each operating system will be in the `release` folder.
- - To build **debug**, simply run `cargo build`. The binary will be in the `target/debug` folder.

- In order to run the unit tests, simply run `cargo test`.
- For the NSIS installer, just compile `./installer/pseudolang.nsi`

## Examples

[Pseudolang.md](Pseudolang.md) contains a full explanation of Collegeboard's Pseudocode and many features specific to PseudoLang.

The file `src/tests/mod.rs` also contains various unit tests (examples of code) for Pseudolang.

## Todo

- [ ] Debian package
- [ ] GitHub issue template
- [ ] Proper documentation

<details>
<summary>Functionality</summary>

- [ ] Dictionaries
- [ ] Better error handling (line, column)
- [ ] Networking
- [ ] File IO
- [ ] System integration (terminal commands, process management, environment variables)
- [ ] Library support (remote procedures)
- [ ] Graphics
- [ ] Meta programming
- [ ] Multithreading
- [ ] Bundled compiler

<details>
<summary>Misc</summary>

- [ ] Testing for INPUT and SLEEP (mocking framework)
- [ ] More escape characters

</details>
</details>

## Issues

Feel free to make issues for any bugs or trouble that you experience with this! Especially since this is new, and there are going to be a lot of them!

## Contributing

We welcome contributions! If there are any bugs, or particularly pointing out limitations in [Pseudolang.md](Pseudolang.md) at the bottom, or adding things from the todo list, please make a pull request!

## License

This project is licensed under the MIT License - see the [LICENSE file](LICENSE) file for details.
