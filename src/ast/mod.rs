//! Abstract Syntax Tree (AST) for the Lale programming language.
//!
//! This module defines the semantic structure of Lale programs with strongly-typed nodes
//! and provides the infrastructure for traversing and building the AST.
//!
//! # Module Structure
//!
//! - **definitions**: AST type definitions (statements, expressions, types, location tracking)
//! - **builder**: Conversion from pest parse tree to semantic AST (includes Pratt parser)
//! - **visitor**: Visitor trait for traversing the AST
//!
//! # Overview
//!
//! The AST is built from the pest parse tree by `builder::build_program()` and can be
//! traversed using the `AstVisitor` trait.
//!
//! The struct `Program` **is** the root of the AST. We call it Program instead of AST,
//! because it matches the program rule in the grammar, which is the top-level rule.
//!
//! ## Location Tracking with `Spanned<T>`
//!
//! The AST uses the [`Spanned<T>`] wrapper type to associate source locations
//! with individual tokens and values. This provides:
//!
//! - **Type safety**: Location information is part of the type, making it
//!   impossible to forget tracking a location.
//! - **Consistency**: All located values use the same pattern.
//! - **Clarity**: `Spanned<String>` clearly indicates a string with location.
//!
//! ### When to use Spanned vs plain location field
//!
//! - Use `Spanned<T>` for individual tokens: identifiers, operators, literals
//! - Use plain `location: SourceLocation` for compound nodes that span multiple
//!   tokens (like the overall statement location)
//!
//! ## Symbol Table Linkage
//!
//! Function definitions (`FnDefStmt`) contain a `symbol_table` field that is
//! populated during semantic analysis. This links AST nodes directly to their
//! corresponding symbol information, enabling code generation without needing
//! to carry the semantic analyzer around.

pub mod builder;
pub mod definitions;
pub mod expr_parser;
pub mod token_stream;
pub mod visitor;

// Re-export commonly used types at the module level
pub use builder::build_program;
pub use definitions::*;
pub use expr_parser::build_expression;
pub use token_stream::{Token, TokenKind, tokenize};
pub use visitor::AstVisitor;

// ==================== LOCATION TYPES ====================

/// Source location information for error reporting and IDE integration.
///
/// This struct captures the exact position of a syntax element in the source
/// file, including both line/column for human readability and byte offsets
/// for programmatic use.
///
/// # Fields
///
/// - `line`: 1-indexed line number
/// - `col`: 1-indexed column number (in characters, not bytes)
/// - `start_pos`: Byte offset from the start of the file
/// - `end_pos`: Byte offset of the end of this element
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceLocation {
  pub line: usize,
  pub col: usize,
  pub start_pos: usize,
  pub end_pos: usize,
  pub source_file: String,
}

impl SourceLocation {
  /// Create source location from pest parse tree pair.
  ///
  /// This extracts location information from a pest `Pair`, which represents
  /// a matched grammar rule in the parse tree.
  pub fn from_pair(pair: &pest::iterators::Pair<crate::Rule>) -> Self {
    let (line, col) = pair.line_col();
    SourceLocation {
      line,
      col,
      start_pos: pair.as_span().start_pos().pos(),
      end_pos: pair.as_span().end_pos().pos(),
      source_file: "<unknown>".to_string(),
    }
  }

  /// Create a dummy location for synthetic nodes or testing.
  ///
  /// This should only be used when a real location is not available,
  /// such as for compiler-generated code or in test fixtures.
  pub fn dummy() -> Self {
    SourceLocation {
      line: 0,
      col: 0,
      start_pos: 0,
      end_pos: 0,
      source_file: "<synthetic>".to_string(),
    }
  }
}

/// A value paired with its source location.
///
/// `Spanned<T>` is a generic wrapper that associates any value with the
/// location in the source code where it was defined. This is the primary
/// mechanism for tracking source locations throughout the AST.
///
/// # Design Rationale
///
/// Instead of having separate fields like:
/// ```text
/// pub name: String,
/// pub name_location: SourceLocation,
/// ```
///
/// We use:
/// ```text
/// pub name: Spanned<String>,
/// ```
///
/// This provides several benefits:
/// - **Type safety**: Cannot forget to add location tracking
/// - **Consistency**: All located values use the same pattern
/// - **Self-documenting**: The type clearly shows this value has a location
///
/// # Examples
///
/// ```rust
/// # use lale::ast::{Spanned, SourceLocation};
/// let location = SourceLocation { line: 1, col: 1, start_pos: 0, end_pos: 3, source_file: "test".into() };
/// let name = Spanned::new("foo".to_string(), location);
///
/// // Accessing the value
/// assert_eq!(name.node, "foo");
///
/// // Accessing the location
/// assert_eq!(name.span.line, 1);
///
/// // Pattern matching
/// let Spanned { node, span } = name;
/// assert_eq!(node, "foo");
/// ```
///
/// # When to Use
///
/// Use `Spanned<T>` for individual tokens that have a specific source location:
/// - Identifiers (variable names, function names)
/// - Operators
/// - Individual literal values
///
/// For compound nodes that span multiple tokens, use a plain `location` field
/// on the struct instead.
#[derive(Debug, Clone)]
pub struct Spanned<T> {
  /// The wrapped value.
  pub node: T,
  /// The source location where this value appears.
  pub span: SourceLocation,
}

impl<T> Spanned<T> {
  /// Create a new spanned value.
  ///
  /// # Arguments
  ///
  /// * `node` - The value to wrap
  /// * `span` - The source location of this value
  ///
  /// # Examples
  ///
  /// ```rust
  /// # use lale::ast::{Spanned, SourceLocation};
  /// let location = SourceLocation { line: 1, col: 1, start_pos: 0, end_pos: 13, source_file: "test".into() };
  /// let name = Spanned::new("variable_name".to_string(), location);
  /// assert_eq!(name.node, "variable_name");
  /// ```
  pub fn new(node: T, span: SourceLocation) -> Self {
    Self { node, span }
  }

  /// Transform the inner value while preserving the source location.
  ///
  /// This is useful when you need to convert the wrapped type but want
  /// to keep the same location information.
  ///
  /// # Examples
  ///
  /// ```rust
  /// # use lale::ast::{Spanned, SourceLocation};
  /// let location = SourceLocation { line: 1, col: 1, start_pos: 0, end_pos: 2, source_file: "test".into() };
  /// let spanned_str: Spanned<&str> = Spanned::new("42", location);
  /// let spanned_int: Spanned<i32> = spanned_str.map(|s| s.parse().unwrap());
  /// assert_eq!(spanned_int.node, 42);
  /// ```
  pub fn map<U, F>(self, f: F) -> Spanned<U>
  where
    F: FnOnce(T) -> U,
  {
    Spanned {
      node: f(self.node),
      span: self.span,
    }
  }

  /// Get a reference to the inner value.
  ///
  /// This is a convenience method equivalent to `&spanned.node`.
  pub fn value(&self) -> &T {
    &self.node
  }

  /// Get a reference to the source location.
  ///
  /// This is a convenience method equivalent to `&spanned.span`.
  pub fn location(&self) -> &SourceLocation {
    &self.span
  }
}

impl<T: PartialEq> PartialEq for Spanned<T> {
  /// Compare spanned values by their inner value only, ignoring location.
  ///
  /// This allows semantic comparison of AST nodes without worrying about
  /// whether they came from the same source location.
  fn eq(&self, other: &Self) -> bool {
    self.node == other.node
  }
}

impl<T: Eq> Eq for Spanned<T> {}
