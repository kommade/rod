# Rod Validation

[Crates.io](https://crates.io/crates/rod_validation)
[Documentation](https://docs.rs/rod_validation)
[![License: GPL v3](https://img.shields.io/badge/License-GPLv3-blue.svg)](https://github.com/kommade/rod/blob/master/LICENSE)

A powerful and flexible compile-time validation library for Rust structs and enums. Rod provides declarative validation through derive macros, allowing you to specify validation rules directly in your type definitions.

## Features

- **Compile-time validation**: Validation rules are checked at compile time for type safety
- **Declarative syntax**: Define validation rules using intuitive attribute macros
- **Comprehensive type support**: Validate strings, integers, floats, options, tuples, and custom types
- **Flexible error handling**: Choose between fail-fast or collect-all error reporting
- **Custom validation**: Add custom validation logic with closures
- **Regex support**: Built-in regex validation for strings (with optional `regex` feature)
- **Nested validation**: Support for complex nested data structures
- **Per-validation custom errors**: Attach tailored messages to individual validation rules for precise feedback

## Quick Start

Add Rod Validation to your `Cargo.toml`:

```toml
[dependencies]
rod_validation = "0.2.2"
```

For regex support:

```toml
[dependencies]
rod_validation = { version = "0.2.2", features = ["regex"] }
```

## Basic Usage

```rust
use rod_validation::prelude::*;

#[derive(RodValidate)]
struct User {
    #[rod(String {
        length: 3..=50,
        format: Email,
    })]
    email: String,
    
    #[rod(String {
        length: 8..=100,
        includes: "@",
    })]
    password: String,
    
    #[rod(i32 {
        size: 18..=120,
        sign: Positive,
    })]
    age: i32,
    
    #[rod(Option {
        String {
            length: 1..=100,
        }
    })]
    bio: Option<String>,
}

fn main() {
    let user = User {
        email: "user@example.com".to_string(),
        password: "secure@password123".to_string(),
        age: 25,
        bio: Some("Software developer".to_string()),
    };
    
    // Validate and get first error (fail-fast)
    match user.validate() {
        Ok(_) => println!("User is valid!"),
        Err(e) => println!("Validation error: {}", e),
    }
    
    // Validate and collect all errors
    match user.validate_all() {
        Ok(_) => println!("User is valid!"),
        Err(errors) => {
            println!("Found {} validation errors:", errors.len());
            for error in errors {
                println!("  - {}", error);
            }
        }
    }
}
```

## Validation Types

### String Validation

```rust
#[derive(RodValidate)]
struct StringExample {
    #[rod(String {
        length: 5..=20,           // Length between 5 and 20 characters
        format: Email,            // Built-in email format (requires regex feature)
        starts_with: "user_",     // Must start with "user_"
        ends_with: "@domain.com", // Must end with "@domain.com"
        includes: "test",         // Must contain "test"
    })]
    field: String,
}
```

Available string formats (with `regex` feature):

- `Email` - Email address validation
- `Url` - URL validation  
- `Uuid` - UUID validation
- `Ipv4` - IPv4 address validation
- `Ipv6` - IPv6 address validation
- `DateTime` - DateTime validation
- `Regex("pattern")` - Custom regex pattern

### Integer Validation

```rust
#[derive(RodValidate)]
struct IntegerExample {
    #[rod(i32 {
        size: 1..=100,    // Value between 1 and 100
        sign: Positive,   // Must be positive
        step: 5,          // Must be multiple of 5
    })]
    field: i32,
}
```

Number signs:

- `Positive` - Greater than 0
- `Negative` - Less than 0  
- `NonPositive` - Less than or equal to 0
- `NonNegative` - Greater than or equal to 0

### Float Validation

```rust
#[derive(RodValidate)]
struct FloatExample {
    #[rod(f64 {
        size: 0.0..=100.0,  // Value between 0.0 and 100.0
        sign: NonNegative,  // Must be non-negative
        type: Finite,       // Must be finite (not NaN or infinite)
    })]
    field: f64,
}
```

Float types:

- `Finite` - Not NaN or infinite
- `Infinite` - Must be infinite
- `Nan` - Must be NaN
- `Normal` - Must be normal
- `Subnormal` - Must be subnormal

### Option Validation

```rust
#[derive(RodValidate)]
struct OptionExample {
    // Validate the inner value if Some
    #[rod(Option {
        String {
            length: 5,
        }
    })]
    optional_field: Option<String>,
    
    // Require the field to be None
    #[rod(Option {})]
    must_be_none: Option<String>,
}
```

### Tuple Validation

```rust
#[derive(RodValidate)]
struct TupleExample {
    #[rod(Tuple (
        i32 {
            size: 1..=10,
            sign: Positive,
        },
        String {
            length: 5,
        },
        f64 {
            size: 0.0..=1.0,
        }
    ))]
    coordinates: (i32, String, f64),
}
```

### Literal Validation

```rust
#[derive(RodValidate)]
struct LiteralExample {
    #[rod(Literal {
        value: "expected_value",
    })]
    field: String,
}
```

### Custom Validation

```rust
#[derive(RodValidate)]
struct CustomExample {
    #[rod(
        i32 {
            size: 1..=100,
        },
        check = |x| x % 2 == 0  // Custom validation: must be even
    )]
    even_number: i32,
}
```

### Iterable Validation

```rust
#[derive(RodValidate)]
struct IterableExample {
    #[rod(Iterable {
        length: 1..=10,
        String {
            length: 3..=20,
        }
    })]
    tags: Vec<String>,
}
```

## Error Handling

Rod provides two validation methods:

- `validate()` - Returns the first validation error encountered (fail-fast)
- `validate_all()` - Collects and returns all validation errors

```rust
// Fail-fast validation
match user.validate() {
    Ok(_) => println!("Valid!"),
    Err(error) => println!("Error: {}", error),
}

// Collect all errors
match user.validate_all() {
    Ok(_) => println!("Valid!"),
    Err(errors) => {
        for error in errors.iter() {
            println!("Error: {}", error);
        }
    }
}
```

## Per-Validation Custom Errors

Attach bespoke error messages to any validation rule using the `? "<error>"` syntax. Messages placed immediately before a rule override its default error output.

```rust
#[derive(RodValidate)]
struct SignupForm {
    #[rod(String {
        ? "Password must be at least eight characters long.",
        length: 8..,
        ? "Password must include the @ symbol for legacy compatibility.",
        includes: "@",
    })]
    password: String,
}
```

The custom messages are surfaced by both `validate` and `validate_all`, making it straightforward to deliver user-friendly, context-aware feedback.

You may also override the error messages for all validation rules of a type with the `message: "<error>"` syntax.

```rust
#[derive(RodValidate)]
struct SignupForm {
    #[rod(
        String {
            length: 8..,
            includes: "@",
        },
        message: "Password must be 8 characters AND have an @"
    )]
    password: String,
}

If both error message syntaxes are attached, messages attached to specific rules will be preferred.

## Nested Structures

Rod supports validation of nested structures that implement `RodValidate`:

```rust
#[derive(RodValidate)]
struct Address {
    #[rod(String { length: 1..=100 })]
    street: String,
    
    #[rod(String { length: 2..=50 })]
    city: String,
}

#[derive(RodValidate)]
struct Person {
    #[rod(String { length: 1..=50 })]
    name: String,
    
    // No #[rod] attribute needed for custom types
    address: Address,
}
```

## Enums

Rod supports validation of enumeration variants:

```rust
#[derive(RodValidate)]
enum Status {
    #[rod(String { length: 1..=100 })]
    Active(String),
    
    Inactive,
    
    #[rod(i32 { size: 1..=30 })]
    Pending { days: i32 },
}
```

## The RodValidate Derive Macro

The `#[derive(RodValidate)]` macro generates two validation methods for your types:

- `validate(&self) -> Result<(), RodValidateError>` - Fail-fast validation (returns on first error)
- `validate_all(&self) -> Result<(), RodValidateErrorList>` - Collect all errors before returning

### Error Messages

The generated validation code produces detailed error messages with field paths:

```rust
// Example error: "Expected `user.email` to be a string with length 3..=50, got 2"
```

### Compilation Guarantees

The macro provides several compile-time guarantees:

1. **Type Safety**: Validation attributes must match the field type
2. **Nested Types**: Custom types must implement `RodValidate`
3. **Attribute Validation**: Invalid attribute combinations are caught at compile time

### Technical Details

#### Code Generation Strategy

The macro generates validation code that:

1. Traverses struct or enumeration fields recursively
2. Applies validation rules in the order they appear in attributes
3. Handles early return (fail-fast) or error collection (validate-all) modes
4. Provides detailed error context with complete field paths

#### Performance Considerations

- All validation logic is generated at compile time
- No runtime reflection or dynamic dispatch overhead
- Validation performance is equivalent to hand-written code
- Zero-cost abstractions for unused validation paths

#### Limitations

1. **Union types**: Not supported (will generate `todo!()` panic)
2. **Double references**: `&&T` types are not allowed
3. **Custom validation closures**: Must return `bool` type
4. **Regex features**: Require the `regex` crate feature to be enabled

## Rust Features

- **Default features**: `["regex"]`
- **`regex`**: Enables regex-based string format validation

## Documentation

For detailed documentation and examples, visit [docs.rs/rod](https://docs.rs/rod).

## License

This project is licensed under the GNU General Public License v3.0 - see the [LICENSE](LICENSE) file for details.

## Contributing

Contributions are welcome! Please feel free to submit a pull request.
