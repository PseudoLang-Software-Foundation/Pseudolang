# WARNING
# This project is still in development, and is not AT ALL yet ready for use.

# [PseudoLang](https://pseudo-lang.org/)

<p align="center">
    <img src="Pseudolang-Logo.png" alt="Pseudolang Logo" height="200px" width="auto">
</p>

Welcome to Pseudolang! Pseudolang is a simple programming language written in Rust, inspired by The College Board's AP Computer Science Pseudocode

This project aims to fully support Windows, Linux (Debian), and MacOS.

## Releases

Soon, under the releases section you will be able to find pre-compiled binaries for all the languages and operating systems.

## Use

To use the compiled versions, run the executable in the directory of the file ending with `.pc`, and pass the file as and argument. Ex: `./fplc main.pc`

It is highly recommended to add the executable to your PATH, so you can simply run `fplc`.

Free Pseudolang Compiler = fplc (like gcc :)

## Compiling

In order to compile the project yourself, you will need to have rust installed.

- Install [**rust**](https://www.rust-lang.org/tools/install), and make sure you have it added to PATH.
- Clone the repository `git clone https://github.com/Pseudolang-Software-Foundation/PseudoLang.git`
- - To build **release**, you will need bash. Then run `./build_release.sh`. The binaries will be in folders under `target` for their operating systems.
- - To build **debug**, simply run `cargo build`. The binary will be in the `target/debug` folder.

The output will be in the target directory in debug as an executable for the 3 supported operating systems.

## Examples

[Pseudolang.md](Pseudolang.md) contains a full explanation of Pseudolang and features specific to PseudoLang.

We will be adding examples of how to use PseudoLang. Check back soon!

## Todo

- [ ] Complete
- [ ] Web version
- [ ] Installer
- [ ] Library support (remote procedures, calls, etc)
- [ ] Casting and more types (ex: doubles, n-dimensional arrays)
- [ ] Console input
- [ ] Raise error
- [ ] Networking
- [ ] Create release build
- [ ] Man page and and help menu on simple command run
- [ ] Examples
- [ ] VSCode syntax highlighter extension and runner
- [ ] GitHub issue template
- [ ] Error handling (suggestions, etc)
- [ ] Code todos

## Issues

Feel free to make issues for any bugs or trouble that you experience with this! Especially since this is new, and there are going to be a lot of them!

## Contributing

We welcome contributions! If there are any bugs, or particularly pointing out limitations in [Pseudolang.md](Pseudolang.md) at the bottom, or adding things from the todo list, please make a pull request!

## License

This project is licensed under the MIT License - see the [LICENSE file](LICENSE) file for details.
