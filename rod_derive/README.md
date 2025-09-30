# Rod Derive

[Crates.io](https://crates.io/crates/rod_derive)
[Documentation](https://docs.rs/rod_derive)
[![License: GPL v3](https://img.shields.io/badge/License-GPLv3-blue.svg)](https://github.com/kommade/rod/blob/master/LICENSE)

Procedural macro for the Rod Validation library. This crate provides the `#[derive(RodValidate)]` macro that generates validation code based on field attributes.

## Overview

`rod_derive` is the procedural macro component of the Rod Validation framework. It automatically generates validation implementations for structs and enumerations based on declarative attributes.

**This crate is typically used through the main `rod_validation` crate and not directly.**

## Usage

Add Rod Validation to your `Cargo.toml`:

```toml
[dependencies]
rod_validation = "0.2.2"
```

The derive macro is available through the prelude:

```rust
use rod_validation::prelude::*;

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

For complete documentation, examples, technical details, and all supported validation attributes, see the main **[Rod Validation crate documentation](https://docs.rs/rod_validation)** and **[README](../README.md)**.

This includes:

- All validation types and attributes
- Generated code details
- Compilation guarantees
- Performance considerations
- Advanced features
- Per-validation custom error messages using the `? "message"` syntax
- Limitations and troubleshooting

## Dependencies

- `proc-macro2` - Token manipulation
- `quote` - Code generation  
- `syn` - Rust syntax parsing
- `proc-macro-error` - Enhanced error reporting

## License

This project is licensed under the GNU General Public License v3.0 - see the [LICENSE](./LICENSE) file for details.
