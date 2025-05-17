
# SageGuard

[![Rust](https://img.shields.io/badge/language-Rust-orange)](https://www.rust-lang.org/)  
[![Crates.io](https://img.shields.io/crates/v/sageguard)]()  
[![License](https://img.shields.io/badge/license-MIT-blue)]()

---

## What is SageGuard?

**SageGuard** is a Rust-based static analysis tool designed to help Solana developers identify potential security issues and best practice violations in Anchor smart contracts.  
Inspired by tools like Slither (for Ethereum), SageGuard focuses on analyzing the Rust code of Anchor programs, providing helpful warnings and insights about your accounts and program structure.

---

## Features

- Detect missing `signer` constraints on account fields  
- Identify `#[derive(Accounts)]` structs and analyze their account attributes  
- Find and report on `#[program]` modules and their functions  
- Color-coded CLI output for better readability  
- Reports filename and precise line numbers with clickable terminal links  
- Designed for extensibility — easy to add new static checks

---

## Installation

Currently, SageGuard is a Rust project. To build and run:

```bash
git clone https://github.com/yourusername/sageguard.git
cd sageguard
cargo build --release
```

---

## Usage

Run SageGuard against your Anchor project directory:

```bash
./target/release/sageguard /path/to/your/anchor/project
```

Example output:

```
[INFO] Found #[derive(Accounts)] struct: Transfer (programs/my_program/src/lib.rs:10)
[WARNING] Account `authority` may be missing `signer` constraint. (programs/my_program/src/lib.rs:12)
[INFO] Found program : my_program (programs/my_program/src/lib.rs:5)
[INFO] Found function: initialize (programs/my_program/src/lib.rs:25)
```

---

## Example Checks

- Missing `signer` constraint in accounts struct fields  
- Presence of `#[program]` attribute on modules  
- Listing of all functions inside `#[program]` modules

---

## How it works

- Parses Rust source files using [`syn`](https://crates.io/crates/syn)  
- Walks project directory recursively with [`walkdir`](https://crates.io/crates/walkdir)  
- Uses Rust's procedural macro parsing utilities to analyze attributes and types  
- Prints colorized warnings and info with [`colored`](https://crates.io/crates/colored)

---

## Contributing

Contributions are welcome! Feel free to open issues or submit pull requests to add new checks or improve existing ones.

---

## License

This project is licensed under the MIT License.

---

## Contact

Created by Farman Shaik — feel free to reach out on x.com/x0rc1ph3r.
