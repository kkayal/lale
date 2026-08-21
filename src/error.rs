//! Unified Compilation Error Types
//!
//! This module provides a crate-wide error type system to replace panics with proper error handling.
//! It covers IR construction, semantic analysis, code generation, and I/O operations.

use std::fmt;
use std::io;

/// Unified compilation result type for the Lale compiler.
///
/// All fallible operations should return this type rather than panicking with `.unwrap()`.
pub type CompileResult<T> = Result<T, CompileError>;

/// Root compilation error enum covering all compiler subsystems.
///
/// This enum represents all recoverable errors in the compilation pipeline.
/// Each variant can be context-enriched with file/line information by callers.
#[derive(Debug, Clone)]
pub enum CompileError {
  /// IR (Intermediate Representation) construction error.
  Ir(IrError),
  /// Semantic analysis error (symbol resolution, type checking, etc.).
  Semantic(String),
  /// Code generation error (for future backends).
  Codegen(CodegenError),
  /// I/O and file system errors.
  Io(String),
  /// Module resolution or import errors.
  ModuleResolution(String),
  /// Generic compilation error with a message.
  Generic(String),
}

/// IR-specific errors (function, block, value lookups, builder state).
#[derive(Debug, Clone)]
pub enum IrError {
  /// Function with the given ID was not found in module.
  FunctionNotFound { id: String },
  /// Basic block with the given ID was not found in function.
  BlockNotFound { id: String },
  /// Value with the given ID was not found.
  ValueNotFound { id: String },
  /// No current function has been set in the builder.
  NoCurrentFunction,
  /// No current block has been set in the builder.
  NoCurrentBlock,
  /// Entry block is missing (indicates broken invariant in function construction).
  MissingEntryBlock,
  /// Generic IR error with a message.
  Other(String),
}

/// IR generation errors (AST to IR conversion failures).
#[derive(Debug, Clone)]
pub enum IrGenError {
  /// Reference to an undefined variable.
  UndefinedVariable { name: String },
  /// Reference to an undefined function.
  UndefinedFunction { name: String },
  /// Type mismatch during IR generation.
  TypeMismatch { expected: String, actual: String },
  /// Invalid builder state (e.g., no current function or block).
  InvalidBuilderState { reason: String },
  /// Internal error during IR generation.
  InternalError { message: String },
}

/// Code generation errors (LLVM, type mismatches, etc.).
#[derive(Debug, Clone)]
pub enum CodegenError {
  /// Symbol (function, value) not found during codegen.
  SymbolNotFound { name: String },
  /// LLVM type construction failed.
  TypeConstructionFailed { reason: String },
  /// Missing or malformed terminator in basic block.
  MissingTerminator { block_name: String },
  /// Generic code generation error.
  Other(String),
}

impl fmt::Display for CompileError {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    match self {
      CompileError::Ir(err) => write!(f, "compiler error: {}", err),
      CompileError::Semantic(msg) => write!(f, "semantic error: {}", msg),
      CompileError::Codegen(err) => write!(f, "compiler error: {}", err),
      CompileError::Io(msg) => write!(f, "compiler error: {}", msg),
      CompileError::ModuleResolution(msg) => write!(f, "module error: {}", msg),
      CompileError::Generic(msg) => write!(f, "compiler error: {}", msg),
    }
  }
}

impl fmt::Display for IrError {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    match self {
      IrError::FunctionNotFound { id } => write!(f, "function '{}' not found", id),
      IrError::BlockNotFound { id } => write!(f, "block '{}' not found", id),
      IrError::ValueNotFound { id } => write!(f, "value '{}' not found", id),
      IrError::NoCurrentFunction => {
        write!(f, "Invariant: no current function set in builder")
      }
      IrError::NoCurrentBlock => {
        write!(f, "Invariant: no current block set in builder")
      }
      IrError::MissingEntryBlock => {
        write!(f, "Invariant: entry block must exist after construction")
      }
      IrError::Other(msg) => write!(f, "{}", msg),
    }
  }
}

impl fmt::Display for IrGenError {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    match self {
      IrGenError::UndefinedVariable { name } => {
        write!(f, "undefined variable '{}'", name)
      }
      IrGenError::UndefinedFunction { name } => {
        write!(f, "undefined function '{}'", name)
      }
      IrGenError::TypeMismatch { expected, actual } => {
        write!(
          f,
          "type mismatch: expected '{}', got '{}'",
          expected, actual
        )
      }
      IrGenError::InvalidBuilderState { reason } => {
        write!(f, "invalid builder state: {}", reason)
      }
      IrGenError::InternalError { message } => {
        write!(f, "internal IR generation error: {}", message)
      }
    }
  }
}

impl fmt::Display for CodegenError {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    match self {
      CodegenError::SymbolNotFound { name } => {
        write!(f, "symbol '{}' not found during code generation", name)
      }
      CodegenError::TypeConstructionFailed { reason } => {
        write!(f, "failed to construct type: {}", reason)
      }
      CodegenError::MissingTerminator { block_name } => {
        write!(
          f,
          "block '{}' is missing a terminator instruction",
          block_name
        )
      }
      CodegenError::Other(msg) => write!(f, "{}", msg),
    }
  }
}

impl std::error::Error for CompileError {}
impl std::error::Error for IrError {}
impl std::error::Error for IrGenError {}
impl std::error::Error for CodegenError {}

// Convenience conversions for common cases

impl From<io::Error> for CompileError {
  fn from(err: io::Error) -> Self {
    CompileError::Io(format!("{}", err))
  }
}

impl From<IrError> for CompileError {
  fn from(err: IrError) -> Self {
    CompileError::Ir(err)
  }
}

impl From<IrGenError> for CompileError {
  fn from(err: IrGenError) -> Self {
    CompileError::Generic(err.to_string())
  }
}

impl From<CodegenError> for CompileError {
  fn from(err: CodegenError) -> Self {
    CompileError::Codegen(err)
  }
}

impl From<String> for CompileError {
  fn from(msg: String) -> Self {
    CompileError::Generic(msg)
  }
}

impl From<&str> for CompileError {
  fn from(msg: &str) -> Self {
    CompileError::Generic(msg.to_string())
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_ir_error_display() {
    let err = IrError::FunctionNotFound {
      id: "func_1".to_string(),
    };
    assert_eq!(err.to_string(), "function 'func_1' not found");
  }

  #[test]
  fn test_compile_error_from_string() {
    let err: CompileError = "test error".into();
    assert!(err.to_string().contains("test error"));
  }

  #[test]
  fn test_error_conversion() {
    let ir_err = IrError::NoCurrentFunction;
    let compile_err: CompileError = ir_err.into();
    assert!(compile_err.to_string().contains("no current function"));
  }
}
