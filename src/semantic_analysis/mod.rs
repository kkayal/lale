//! Semantic Analysis Module
//!
//! This module implements semantic analysis for the Lale programming language.
//! It validates the AST by building symbol tables and detecting errors such as:
//! - Undefined variable usage
//! - Scope violations (e.g., nested function/type definitions)
//! - Escaping references to local variables
//! - Type incompatibilities
//! - Unit mismatches
//! - Unused variables and functions
//!
//! # Scope Rules
//!
//! Lale enforces strict scope rules:
//! - **Global Scope**: Functions and types can ONLY be defined here
//! - **Function Scope**: Variables and parameters; NO nested function/type definitions
//!
//! Violations result in semantic errors:
//! - "Function definitions are only allowed at global scope"
//! - "Type definitions are only allowed at global scope"
//!
//! # Architecture
//!
//! The semantic analysis is split into focused submodules:
//!
//! - **analyzer**: Main semantic analyzer implementation with scope validation
//! - **sqlite_symbol_management**: SQLite-backed symbol table and type definition registry management
//! - **type_system**: Type inference and type compatibility checking
//! - **expression_analysis**: Expression utilities and pointer escape detection
//! - **error_types**: Semantic error reporting with source locations
//!
//! Physical unit computation and verification is handled by the separate `unit_analysis` module.

pub mod analyzer;
pub mod const_eval;
pub mod const_value;
pub mod error_types;
pub mod expression_analysis;
pub mod memory_safety;
pub mod module_resolver;
pub mod qualified_names;
pub mod sqlite_symbol_management;
pub mod type_compatibility;
pub mod type_conversion;
pub mod unit_computer;

// Re-export unit_analysis from top-level module for public re-export
pub use crate::unit_analysis;

// Re-export public API
pub use crate::ast::{Linkage, StorageClass, Symbol, SymbolKind, SymbolTable, Visibility};
pub use analyzer::{
  AnalyzerResults, OwnedAnalyzer, SemanticAnalyzer, analyze_ast, analyze_ast_with_options,
  analyze_ast_with_stdlib, process_ct_directives,
};
pub use const_value::ConstValue;
pub use error_types::SemanticError;
pub use module_resolver::{
  ModuleError, ModuleGraph, ModuleId, ModuleResolver, ResolvedModule, resolve_stdlib_archive_path,
  resolve_stdlib_path,
};
pub use qualified_names::{QualifiedFunctionName, QualifiedTypeName, VariableKey};
pub use sqlite_symbol_management::{
  FnInfo, FnScopeKey, SqliteSymbolManager, TypeDefInfo, TypeFieldInfo, VarScope,
};
pub use type_conversion::{NumericTypeInfo, TypeCategory, TypeValidator};
