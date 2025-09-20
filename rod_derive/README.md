# Rod Derive

[![Crates.io](https://img.shields.io/crates/v/rod_derive.svg)](https://crates.io/crates/rod_derive)
[![Documentation](https://docs.rs/rod_derive/badge.svg)](https://docs.rs/rod_derive)
[![License: GPL v3](https://img.shields.io/badge/License-GPLv3-blue.svg)](https://github.com/kommade/rod/blob/master/LICENSE)

Procedural macro for the Rod validation library. This crate provides the `#[derive(RodValidate)]` macro that generates validation code based on field attributes.

## Overview

`rod_derive` is the procedural macro component of the Rod validation framework. It automatically generates validation implementations for structs and enums based on declarative attributes.

**This crate is typically used through the main `rod` crate and not directly.**

## Usage

Add Rod to your `Cargo.toml`:

```toml
[dependencies]
rod = "0.2.0"
```

The derive macro is available through the prelude:

```rust
use rod::prelude::*;

#[derive(RodValidate)]
struct User {
    #[rod(String {
        length: 3..=50,
        format: Email,
    })]
    email: String,
}
```

## Documentation

For complete documentation, examples, technical details, and all supported validation attributes, see the main **[Rod crate documentation](https://docs.rs/rod)** and **[README](../README.md)**.

This includes:

- All validation types and attributes
- Generated code details
- Compilation guarantees
- Performance considerations
- Advanced features
- Limitations and troubleshooting

## Dependencies

- `proc-macro2` - Token manipulation
- `quote` - Code generation  
- `syn` - Rust syntax parsing
- `proc-macro-error` - Enhanced error reporting

## License

This project is licensed under the GNU General Public License v3.0 - see the [LICENSE](./LICENSE) file for details.
