//! Output Printers Module
//!
//! This module provides pretty-printing functionality for the compiler's internal structures.
//! It includes three specialized printers for different compilation stages:
//!
//! - **pairs**: Prints the raw parse tree from pest before AST conversion
//! - **ast**: Prints the semantic AST with hierarchical formatting and source locations
//! - **symbol_table**: Prints symbol tables generated during semantic analysis
//!
//! All printers support optional color output for improved readability in terminals.

pub mod ast;
pub mod pairs;
pub mod symbol_table;
