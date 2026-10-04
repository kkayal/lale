//! Lale Intermediate Representation (IR)
//!
//! This module defines the IR that serves as the bridge between semantic analysis
//! and backend code generation. The IR is in SSA (Static Single Assignment) form.
//!
//! # Architecture
//!
//! ```text
//! Semantic AST
//!     ↓
//! IR Generation (AST → IR)
//!     ↓
//! ┌─────────────────────────────────────┐
//! │            Lale IR Module           │
//! │  ┌─────────────────────────────┐    │
//! │  │      Global Variables       │    │
//! │  └─────────────────────────────┘    │
//! │  ┌─────────────────────────────┐    │
//! │  │        Functions            │    │
//! │  │  ┌───────────────────────┐  │    │
//! │  │  │    Basic Blocks       │  │    │
//! │  │  │  ┌─────────────────┐  │  │    │
//! │  │  │  │  Instructions   │  │  │    │
//! │  │  │  └─────────────────┘  │  │    │
//! │  │  └───────────────────────┘  │    │
//! │  └─────────────────────────────┘    │
//! └─────────────────────────────────────┘
//!     ↓
//! Backend Lowering (IR → LLVM)
//!     ↓
//! Machine Code
//! ```
//!
//! # Key Types
//!
//! - [`Module`] - Top-level container for an IR program
//! - [`Function`] - A function definition with basic blocks
//! - [`BasicBlock`] - A sequence of instructions with single entry/exit
//! - [`Instruction`] - An atomic IR operation
//! - [`IrType`] - IR type system
//! - [`ValueId`] - SSA value identifier
//! - [`IrBuilder`] - Convenient API for constructing IR
//!
//! # Example
//!
//! ```rust
//! use lale::ir::{IrBuilder, IrType, Linkage, print_module};
//!
//! // Build a simple "main" function that prints "Hello, World!"
//! let mut builder = IrBuilder::new("hello");
//! builder.add_stdlib_externs();
//!
//! builder.start_function("main", IrType::Void, Linkage::Export);
//! let msg = builder.const_string("Hello, World!");
//! builder.call_void_named("puts", vec![msg]);
//! builder.ret_void();
//!
//! let module = builder.build();
//! println!("{}", print_module(&module));
//! ```
//!
//! # Module Organization
//!
//! | Module | Purpose |
//! |--------|---------|
//! | `types` | IR type definitions |
//! | `values` | Value IDs and metadata |
//! | `instructions` | All IR instruction types |
//! | `blocks` | Basic block representation |
//! | `function` | Function and parameter types |
//! | `module` | Top-level module, globals, structs |
//! | `builder` | Convenient IR construction API |
//! | `printer` | Human-readable IR output |
//!
//! # Design Goals
//!
//! 1. **SSA Form**: All values defined exactly once
//! 2. **Backend Agnostic**: Maps cleanly to LLVM
//! 3. **Type Safe**: IR types mirror Lale's type system
//! 4. **Unit Aware**: Physical units preserved for runtime checks
//! 5. **Minimal**: Only necessary constructs
//!
//! See `doc/IR_SPECIFICATION.md` for the full specification.

/// IR text-format major version. Increment on breaking changes.
pub const IR_FORMAT_MAJOR: u32 = 1;
/// IR text-format minor version. Increment on additive, backward-compatible changes.
pub const IR_FORMAT_MINOR: u32 = 0;

pub mod blocks;
pub mod builder;
pub mod function;
pub mod instructions;
pub mod module;
pub mod parser;
pub mod printer;
pub mod types;
pub mod values;
pub mod verifier;

// Re-export commonly used types
pub use blocks::BasicBlock;
pub use builder::IrBuilder;
pub use function::{ExternFunc, Function, Linkage, Parameter};
pub use instructions::{FuncRef, Instruction, RenderPart};
pub use module::{Constant, Global, Module, StructDef};
pub use parser::parse_module;
pub use printer::print_module;
pub use types::IrType;
pub use values::{BlockId, FuncId, GlobalId, Operand, ValueId, ValueInfo};
