//! Built-in compiler-provided source code embedded at compile time.
//!
//! This module contains the Lale compiler's builtins module as a
//! compile-time string constant, eliminating the need to read it from the
//! filesystem during compilation.

/// The builtins.lale source code embedded at compile time.
///
/// This is the compiler-provided builtins module, containing:
/// - The `str` type definition
/// - FFI function declarations (malloc, exit)
/// - Memory operations (memcpy)
/// - String construction utilities
/// - Type conversion functions (bool, char, u64, i64, f64 to string)
pub const BUILTINS_SOURCE: &str = include_str!("builtins.lale");
