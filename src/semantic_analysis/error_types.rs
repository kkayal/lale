//! Semantic Error Types and Reporting
//!
//! This module defines the `SemanticError` struct for structured error reporting
//! with source location information.

use crate::ast::SourceLocation;
use std::fmt;

/// Semantic error with structured source location information.
///
/// This type provides rich error reporting with precise source locations,
/// enabling IDE integration and user-friendly error messages.
#[derive(Debug, Clone)]
pub struct SemanticError {
  /// The error message describing what went wrong.
  pub message: String,
  /// The source location where the error occurred.
  pub location: SourceLocation,
  /// Optional hint for how to fix the error.
  pub hint: Option<String>,
}

impl SemanticError {
  /// Create a new semantic error at the given location.
  pub fn new(message: impl Into<String>, location: SourceLocation) -> Self {
    SemanticError {
      message: message.into(),
      location,
      hint: None,
    }
  }

  /// Create a new semantic error with a hint.
  pub fn with_hint(
    message: impl Into<String>,
    location: SourceLocation,
    hint: impl Into<String>,
  ) -> Self {
    SemanticError {
      message: message.into(),
      location,
      hint: Some(hint.into()),
    }
  }

  /// Get the line number (1-indexed).
  pub fn line(&self) -> usize {
    self.location.line
  }

  /// Get the column number (1-indexed).
  pub fn col(&self) -> usize {
    self.location.col
  }

  /// Check if the error message contains a substring.
  pub fn contains(&self, substring: &str) -> bool {
    self.message.contains(substring)
  }
}

impl fmt::Display for SemanticError {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    // Format: file:line:col message
    // This format allows terminal click-to-open functionality
    write!(
      f,
      "{}:{}:{} {}",
      self.location.source_file, self.location.line, self.location.col, self.message
    )?;
    if let Some(ref hint) = self.hint {
      write!(f, " (hint: {})", hint)?;
    }
    Ok(())
  }
}

impl std::error::Error for SemanticError {}
