//! IR Builder
//!
//! Provides a convenient API for constructing IR. The builder tracks the
//! current insertion point and handles value allocation automatically.

use super::function::Linkage;
use super::instructions::{FuncRef, Instruction, RenderPart};
use super::module::{Constant, Module, StructDef};
use super::types::IrType;
use super::values::{BlockId, FuncId, GlobalId, ValueId};
use crate::error::{CompileResult, IrError};
use crate::types::NormalizedUnit;

/// Build the color-free render spec for a runtime-error trap message.
/// Produces a single `Text` part: `ERROR at <file>:<line>:<col>: <message>`
/// (no trailing newline — the output layer owns presentation).
fn trap_parts(file: &str, line: i64, column: i64, message: &str) -> Vec<RenderPart> {
  vec![RenderPart::Text(format!(
    "ERROR at {}:{}:{}: {}",
    file, line, column, message
  ))]
}

/// Build the message for a checked-arithmetic overflow trap. The sign and
/// overflow/underflow kind are determined from the declared `ty` and `op`.
fn checked_trap_message(signed: bool, op: &str, ty: &IrType, underflow: bool) -> String {
  let sign = if signed { "signed" } else { "unsigned" };
  let kind = if underflow { "underflow" } else { "overflow" };
  format!("integer {} in {} {} ({})", kind, sign, op, ty)
}

/// Builder for constructing IR modules.
pub struct IrBuilder {
  /// The module being built
  module: Module,
  /// Current function being built
  current_func: Option<FuncId>,
  /// Current block being built
  current_block: Option<BlockId>,
}

impl IrBuilder {
  /// Create a new IR builder with an empty module.
  pub fn new(module_name: impl Into<String>) -> Self {
    IrBuilder {
      module: Module::new(module_name),
      current_func: None,
      current_block: None,
    }
  }

  /// Create an IR builder with a pre-existing module.
  /// Used when the module has been pre-populated (e.g., with builtins types)
  /// before user code generation begins.
  pub fn with_module(module: Module) -> Self {
    IrBuilder {
      module,
      current_func: None,
      current_block: None,
    }
  }

  /// Consume the builder and return the module.
  pub fn build(self) -> Module {
    self.module
  }

  /// Get a reference to the module.
  pub fn module(&self) -> &Module {
    &self.module
  }

  /// Get a mutable reference to the module.
  pub fn module_mut(&mut self) -> &mut Module {
    &mut self.module
  }

  /// Get the current function being built.
  pub fn get_current_func(&self) -> Option<FuncId> {
    self.current_func
  }

  /// Get the current function, returning an error if not set.
  pub fn try_current_func(&self) -> CompileResult<FuncId> {
    self
      .current_func
      .ok_or_else(|| IrError::NoCurrentFunction.into())
  }

  /// Set the current function being built.
  pub fn set_current_func(&mut self, func: Option<FuncId>) {
    self.current_func = func;
  }

  /// Set the current function by ID, validating it exists in the module.
  /// Returns an error if the function doesn't exist.
  pub fn set_current_function(&mut self, id: FuncId) -> CompileResult<()> {
    // Validate function exists
    self.module.try_function(id)?;
    self.current_func = Some(id);
    Ok(())
  }

  /// Get the current block being built.
  pub fn get_current_block(&self) -> Option<BlockId> {
    self.current_block
  }

  /// Get the current block, returning an error if not set.
  pub fn try_current_block(&self) -> CompileResult<BlockId> {
    self
      .current_block
      .ok_or_else(|| IrError::NoCurrentBlock.into())
  }

  /// Set the current block being built.
  pub fn set_current_block(&mut self, block: Option<BlockId>) {
    self.current_block = block;
  }

  /// Check if the current block is already terminated.
  pub fn is_current_block_terminated(&self) -> bool {
    if let (Some(func_id), Some(block_id)) = (self.current_func, self.current_block) {
      if let Ok(func) = self.module.try_function(func_id) {
        if let Some(block) = func.block(block_id) {
          block.is_terminated()
        } else {
          false
        }
      } else {
        false
      }
    } else {
      false
    }
  }

  // ========== Module-Level Operations ==========

  /// Add standard library external functions.
  pub fn add_stdlib_externs(&mut self) {
    self.module.add_stdlib_externs();
  }

  /// Add a struct definition.
  pub fn add_struct(&mut self, def: StructDef) {
    self.module.add_struct(def);
  }

  /// Add a global variable.
  pub fn add_global(&mut self, name: impl Into<String>, ty: IrType, linkage: Linkage) -> GlobalId {
    self.module.add_global(name, ty, linkage)
  }

  /// Set a global's initializer.
  pub fn set_global_init(&mut self, id: GlobalId, init: Constant) {
    if let Some(global) = self.module.global_mut(id) {
      global.initializer = Some(init);
    }
  }

  /// Set a global's physical unit.
  pub fn set_global_unit(&mut self, id: GlobalId, unit: NormalizedUnit) {
    if let Some(global) = self.module.global_mut(id) {
      global.unit = Some(unit);
    }
  }

  // ========== Function Operations ==========

  /// Start building a new function.
  ///
  /// # Panics
  /// Panics if the function cannot be retrieved after adding (should never happen).
  #[allow(clippy::expect_used)]
  pub fn start_function(
    &mut self,
    name: impl Into<String>,
    return_type: IrType,
    linkage: Linkage,
  ) -> FuncId {
    self
      .try_start_function(name, return_type, linkage)
      .expect("Invariant: function must exist after add_function")
  }

  /// Start building a new function, returning a Result.
  pub fn try_start_function(
    &mut self,
    name: impl Into<String>,
    return_type: IrType,
    linkage: Linkage,
  ) -> CompileResult<FuncId> {
    let id = self.module.add_function(name, return_type, linkage);
    self.current_func = Some(id);
    let entry_block = self.module.try_function(id)?.entry_block;
    self.current_block = Some(entry_block);
    Ok(id)
  }

  /// Add a parameter to the current function.
  ///
  /// # Panics
  /// Panics if no current function is set or if function lookup fails.
  #[allow(clippy::expect_used)]
  pub fn add_param(
    &mut self,
    name: impl Into<String>,
    ty: IrType,
    unit: Option<NormalizedUnit>,
  ) -> ValueId {
    self
      .try_add_param(name, ty, unit)
      .expect("Invariant: add_param requires valid current function")
  }

  /// Add a parameter to the current function, returning a Result.
  pub fn try_add_param(
    &mut self,
    name: impl Into<String>,
    ty: IrType,
    unit: Option<NormalizedUnit>,
  ) -> CompileResult<ValueId> {
    let func_id = self.try_current_func()?;
    let func = self.module.try_function_mut(func_id)?;
    Ok(func.add_param(name, ty, unit))
  }

  /// Set the return unit for the current function.
  ///
  /// # Panics
  /// Panics if no current function is set or if function lookup fails.
  #[allow(clippy::expect_used)]
  pub fn set_return_unit(&mut self, unit: NormalizedUnit) {
    self
      .try_set_return_unit(unit)
      .expect("Invariant: set_return_unit requires valid current function")
  }

  /// Set the return unit for the current function, returning a Result.
  pub fn try_set_return_unit(&mut self, unit: NormalizedUnit) -> CompileResult<()> {
    let func_id = self.try_current_func()?;
    let func = self.module.try_function_mut(func_id)?;
    func.set_return_unit(unit);
    Ok(())
  }

  /// Create a new basic block in the current function.
  ///
  /// # Panics
  /// Panics if no current function is set or if function lookup fails.
  #[allow(clippy::expect_used)]
  pub fn create_block(&mut self, name: impl Into<String>) -> BlockId {
    self
      .try_create_block(name)
      .expect("Invariant: create_block requires valid current function")
  }

  /// Create a new basic block in the current function, returning a Result.
  pub fn try_create_block(&mut self, name: impl Into<String>) -> CompileResult<BlockId> {
    let func_id = self.try_current_func()?;
    let func = self.module.try_function_mut(func_id)?;
    Ok(func.create_block(name))
  }

  /// Position the builder at a specific block.
  pub fn position_at(&mut self, block: BlockId) {
    self.current_block = Some(block);
  }

  /// Allocate a new value with the given type.
  ///
  /// # Panics
  /// Panics if no current function is set or if function lookup fails.
  #[allow(clippy::expect_used)]
  fn alloc_value(&mut self, ty: IrType) -> ValueId {
    self
      .try_alloc_value(ty)
      .expect("Invariant: alloc_value requires valid current function")
  }

  /// Allocate a new value with the given type, returning a Result.
  fn try_alloc_value(&mut self, ty: IrType) -> CompileResult<ValueId> {
    let func_id = self.try_current_func()?;
    let func = self.module.try_function_mut(func_id)?;
    Ok(func.alloc_value(ty))
  }

  /// Allocate a new value with type and unit.
  ///
  /// # Panics
  /// Panics if no current function is set or if function lookup fails.
  #[allow(dead_code)]
  #[allow(clippy::expect_used)]
  fn alloc_value_with_unit(&mut self, ty: IrType, unit: NormalizedUnit) -> ValueId {
    self
      .try_alloc_value_with_unit(ty, unit)
      .expect("Invariant: alloc_value_with_unit requires valid current function")
  }

  /// Allocate a new value with type and unit, returning a Result.
  #[allow(dead_code)]
  fn try_alloc_value_with_unit(
    &mut self,
    ty: IrType,
    unit: NormalizedUnit,
  ) -> CompileResult<ValueId> {
    let func_id = self.try_current_func()?;
    let func = self.module.try_function_mut(func_id)?;
    Ok(func.alloc_value_with_unit(ty, unit))
  }

  /// Allocate a new value with type and debug name.
  ///
  /// # Panics
  /// Panics if no current function is set or if function lookup fails.
  #[allow(clippy::expect_used)]
  pub fn alloc_value_with_name(&mut self, ty: IrType, name: impl Into<String>) -> ValueId {
    self
      .try_alloc_value_with_name(ty, name)
      .expect("Invariant: alloc_value_with_name requires valid current function")
  }

  /// Allocate a new value with type and debug name, returning a Result.
  pub fn try_alloc_value_with_name(
    &mut self,
    ty: IrType,
    name: impl Into<String>,
  ) -> CompileResult<ValueId> {
    let func_id = self.try_current_func()?;
    let func = self.module.try_function_mut(func_id)?;
    let id = func.alloc_value(ty);
    if let Some(info) = func.values.get_mut(&id) {
      info.name = Some(name.into());
    }
    Ok(id)
  }

  /// Emit an instruction to the current block.
  ///
  /// # Panics
  /// Panics if no current function/block is set or if lookups fail.
  #[allow(clippy::expect_used)]
  fn emit(&mut self, inst: Instruction) {
    self
      .try_emit(inst)
      .expect("Invariant: emit requires valid current function and block")
  }

  /// Emit an instruction to the current block, returning a Result.
  fn try_emit(&mut self, inst: Instruction) -> CompileResult<()> {
    let func_id = self.try_current_func()?;
    let block_id = self.try_current_block()?;
    let func = self.module.try_function_mut(func_id)?;
    let block = func
      .block_mut(block_id)
      .ok_or_else(|| IrError::BlockNotFound {
        id: format!("{:?}", block_id),
      })?;
    block.push(inst);
    Ok(())
  }

  /// Emit an instruction into the function's **entry block**, before its terminator.
  ///
  /// ALLOCA HOISTING:
  /// In LLVM, `alloca` instructions inside a loop body cause the stack pointer to be
  /// decremented on every iteration without reclamation, leading to stack overflow.
  /// The standard fix (used by Clang, rustc, and all major LLVM frontends) is to hoist
  /// all `alloca`s to the function's entry block, where they execute exactly once.
  /// LLVM's `mem2reg` pass can then promote them to SSA registers.
  ///
  /// This method inserts the instruction just before the entry block's terminator
  /// (if it has one), so the alloca is placed after existing entry-block setup code
  /// but before the branch to the next block.
  #[allow(clippy::expect_used)]
  fn emit_to_entry(&mut self, inst: Instruction) {
    self
      .try_emit_to_entry(inst)
      .expect("Invariant: emit_to_entry requires valid current function")
  }

  /// Emit an instruction into the entry block, returning a Result.
  fn try_emit_to_entry(&mut self, inst: Instruction) -> CompileResult<()> {
    let func_id = self.try_current_func()?;
    let func = self.module.try_function_mut(func_id)?;
    let entry_id = func.entry_block;
    let entry = func
      .block_mut(entry_id)
      .ok_or_else(|| IrError::BlockNotFound {
        id: format!("{:?}", entry_id),
      })?;

    // Insert before the terminator if the entry block already has one (e.g., a branch
    // to the first real block). If unterminated, just append.
    if entry.is_terminated() {
      let insert_pos = entry.instructions.len() - 1;
      entry.instructions.insert(insert_pos, inst);
    } else {
      entry.instructions.push(inst);
    }
    Ok(())
  }

  // ========== Constant Instructions ==========

  /// Emit a signed integer constant.
  pub fn const_int(&mut self, ty: IrType, val: i64) -> ValueId {
    let dst = self.alloc_value(ty.clone());
    self.emit(Instruction::ConstInt { dst, ty, val });
    dst
  }

  /// Emit an unsigned integer constant.
  pub fn const_uint(&mut self, ty: IrType, val: u64) -> ValueId {
    let dst = self.alloc_value(ty.clone());
    self.emit(Instruction::ConstUint { dst, ty, val });
    dst
  }

  /// Emit a float constant.
  pub fn const_float(&mut self, ty: IrType, val: f64) -> ValueId {
    let dst = self.alloc_value(ty.clone());
    self.emit(Instruction::ConstFloat { dst, ty, val });
    dst
  }

  /// Emit a boolean constant.
  pub fn const_bool(&mut self, val: bool) -> ValueId {
    let dst = self.alloc_value(IrType::Bool);
    self.emit(Instruction::ConstBool { dst, val });
    dst
  }

  /// Emit a string constant as a text struct.
  ///
  /// This creates a ConstString instruction that produces a pointer to the
  /// string data, then builds a text struct value with that pointer, the byte
  /// length, and the code point count.
  /// Returns a text struct value (not a pointer).
  ///
  /// The buffer includes +1 byte reserved space for potential newline append
  /// (used by write statements).
  pub fn const_string(&mut self, val: impl Into<String>) -> ValueId {
    let s: String = val.into();
    let byte_len = s.len() as i64; // UTF-8 byte count
    let char_len = s.chars().count() as i64; // Unicode code point count

    // Create the string constant (produces ptr to string data - i8 bytes)
    let ptr = self.alloc_value(IrType::ptr(IrType::I8));
    self.emit(Instruction::ConstString { dst: ptr, val: s });

    // Create the byte length and code point count constants
    let byte_len_val = self.const_int(IrType::I64, byte_len);
    let char_len_val = self.const_int(IrType::I64, char_len);

    // Build and return the text struct value (not a pointer)
    self.build_text_value(ptr, byte_len_val, char_len_val)
  }

  /// Emit a string constant that returns just the raw pointer (for internal use).
  /// Used when we need a pointer to string data without the text struct wrapper.
  pub fn const_string_ptr(&mut self, val: impl Into<String>) -> ValueId {
    let dst = self.alloc_value(IrType::ptr(IrType::I8));
    self.emit(Instruction::ConstString {
      dst,
      val: val.into(),
    });
    dst
  }

  /// Emit a null pointer constant.
  pub fn const_null(&mut self) -> ValueId {
    let dst = self.alloc_value(IrType::raw_ptr());
    self.emit(Instruction::ConstNull { dst });
    dst
  }

  // ========== Arithmetic Instructions ==========

  /// Emit an add instruction.
  pub fn add(&mut self, lhs: ValueId, rhs: ValueId, ty: IrType) -> ValueId {
    let dst = self.alloc_value(ty);
    self.emit(Instruction::Add { dst, lhs, rhs });
    dst
  }

  /// Emit a subtract instruction.
  pub fn sub(&mut self, lhs: ValueId, rhs: ValueId, ty: IrType) -> ValueId {
    let dst = self.alloc_value(ty);
    self.emit(Instruction::Sub { dst, lhs, rhs });
    dst
  }

  /// Emit a pointer difference instruction (ptr - ptr → i64).
  pub fn ptr_diff(&mut self, lhs: ValueId, rhs: ValueId) -> ValueId {
    let dst = self.alloc_value(IrType::I64);
    self.emit(Instruction::PtrDiff { dst, lhs, rhs });
    dst
  }

  /// Emit a multiply instruction.
  pub fn mul(&mut self, lhs: ValueId, rhs: ValueId, ty: IrType) -> ValueId {
    let dst = self.alloc_value(ty);
    self.emit(Instruction::Mul { dst, lhs, rhs });
    dst
  }

  /// Emit a divide instruction.
  pub fn div(&mut self, lhs: ValueId, rhs: ValueId, ty: IrType) -> ValueId {
    let dst = self.alloc_value(ty);
    self.emit(Instruction::Div { dst, lhs, rhs });
    dst
  }

  /// Emit a remainder instruction.
  pub fn rem(&mut self, lhs: ValueId, rhs: ValueId, ty: IrType) -> ValueId {
    let dst = self.alloc_value(ty);
    self.emit(Instruction::Rem { dst, lhs, rhs });
    dst
  }

  /// Emit a negation instruction.
  pub fn neg(&mut self, src: ValueId, ty: IrType) -> ValueId {
    let dst = self.alloc_value(ty);
    self.emit(Instruction::Neg { dst, src });
    dst
  }

  /// Emit a checked integer addition instruction (traps on overflow).
  #[allow(clippy::too_many_arguments)]
  pub fn checked_add(
    &mut self,
    lhs: ValueId,
    rhs: ValueId,
    ty: IrType,
    file: impl Into<String>,
    line: i64,
    column: i64,
  ) -> ValueId {
    let dst = self.alloc_value(ty.clone());
    let message = checked_trap_message(ty.is_signed_int(), "addition", &ty, false);
    let parts = trap_parts(&file.into(), line, column, &message);
    self.emit(Instruction::CheckedAdd {
      dst,
      lhs,
      rhs,
      ty,
      parts,
    });
    dst
  }

  /// Emit a checked integer subtraction instruction (traps on overflow).
  #[allow(clippy::too_many_arguments)]
  pub fn checked_sub(
    &mut self,
    lhs: ValueId,
    rhs: ValueId,
    ty: IrType,
    file: impl Into<String>,
    line: i64,
    column: i64,
  ) -> ValueId {
    let dst = self.alloc_value(ty.clone());
    let message = checked_trap_message(ty.is_signed_int(), "subtraction", &ty, !ty.is_signed_int());
    let parts = trap_parts(&file.into(), line, column, &message);
    self.emit(Instruction::CheckedSub {
      dst,
      lhs,
      rhs,
      ty,
      parts,
    });
    dst
  }

  /// Emit a checked integer multiplication instruction (traps on overflow).
  #[allow(clippy::too_many_arguments)]
  pub fn checked_mul(
    &mut self,
    lhs: ValueId,
    rhs: ValueId,
    ty: IrType,
    file: impl Into<String>,
    line: i64,
    column: i64,
  ) -> ValueId {
    let dst = self.alloc_value(ty.clone());
    let message = checked_trap_message(ty.is_signed_int(), "multiplication", &ty, false);
    let parts = trap_parts(&file.into(), line, column, &message);
    self.emit(Instruction::CheckedMul {
      dst,
      lhs,
      rhs,
      ty,
      parts,
    });
    dst
  }

  /// Emit a checked integer negation instruction (traps on overflow).
  pub fn checked_neg(
    &mut self,
    src: ValueId,
    ty: IrType,
    file: impl Into<String>,
    line: i64,
    column: i64,
  ) -> ValueId {
    let dst = self.alloc_value(ty.clone());
    let message = checked_trap_message(true, "negation", &ty, false);
    let parts = trap_parts(&file.into(), line, column, &message);
    self.emit(Instruction::CheckedNeg {
      dst,
      src,
      ty,
      parts,
    });
    dst
  }

  /// Emit a power instruction.
  pub fn pow(&mut self, base: ValueId, exp: ValueId, ty: IrType) -> ValueId {
    let dst = self.alloc_value(ty);
    self.emit(Instruction::Pow { dst, base, exp });
    dst
  }

  /// Emit a cross product instruction (vector cross product).
  pub fn cross(&mut self, lhs: ValueId, rhs: ValueId, ty: IrType) -> ValueId {
    let dst = self.alloc_value(ty);
    self.emit(Instruction::Cross { dst, lhs, rhs });
    dst
  }

  /// Emit a vector dot product instruction.
  /// Result type is the scalar element type of the vectors.
  pub fn dot(&mut self, lhs: ValueId, rhs: ValueId, ty: IrType) -> ValueId {
    let dst = self.alloc_value(ty);
    self.emit(Instruction::Dot { dst, lhs, rhs });
    dst
  }

  // ========== Bitwise Instructions ==========

  /// Emit a bitwise AND instruction.
  pub fn bit_and(&mut self, lhs: ValueId, rhs: ValueId, ty: IrType) -> ValueId {
    let dst = self.alloc_value(ty);
    self.emit(Instruction::BitAnd { dst, lhs, rhs });
    dst
  }

  /// Emit a bitwise OR instruction.
  pub fn bit_or(&mut self, lhs: ValueId, rhs: ValueId, ty: IrType) -> ValueId {
    let dst = self.alloc_value(ty);
    self.emit(Instruction::BitOr { dst, lhs, rhs });
    dst
  }

  /// Emit a bitwise XOR instruction.
  pub fn bit_xor(&mut self, lhs: ValueId, rhs: ValueId, ty: IrType) -> ValueId {
    let dst = self.alloc_value(ty);
    self.emit(Instruction::BitXor { dst, lhs, rhs });
    dst
  }

  /// Emit a bitwise NOT instruction.
  pub fn bit_not(&mut self, src: ValueId, ty: IrType) -> ValueId {
    let dst = self.alloc_value(ty);
    self.emit(Instruction::BitNot { dst, src });
    dst
  }

  /// Emit a shift left instruction.
  pub fn shl(&mut self, lhs: ValueId, rhs: ValueId, ty: IrType) -> ValueId {
    let dst = self.alloc_value(ty);
    self.emit(Instruction::Shl { dst, lhs, rhs });
    dst
  }

  /// Emit a signed shift right instruction.
  pub fn shr(&mut self, lhs: ValueId, rhs: ValueId, ty: IrType) -> ValueId {
    let dst = self.alloc_value(ty);
    self.emit(Instruction::Shr { dst, lhs, rhs });
    dst
  }

  /// Emit an unsigned shift right instruction.
  pub fn ushr(&mut self, lhs: ValueId, rhs: ValueId, ty: IrType) -> ValueId {
    let dst = self.alloc_value(ty);
    self.emit(Instruction::UShr { dst, lhs, rhs });
    dst
  }

  // ========== Comparison Instructions ==========

  /// Emit an equality comparison.
  pub fn eq(&mut self, lhs: ValueId, rhs: ValueId) -> ValueId {
    let dst = self.alloc_value(IrType::Bool);
    self.emit(Instruction::Eq { dst, lhs, rhs });
    dst
  }

  /// Emit a not-equal comparison.
  pub fn ne(&mut self, lhs: ValueId, rhs: ValueId) -> ValueId {
    let dst = self.alloc_value(IrType::Bool);
    self.emit(Instruction::Ne { dst, lhs, rhs });
    dst
  }

  /// Emit a less-than comparison.
  pub fn lt(&mut self, lhs: ValueId, rhs: ValueId) -> ValueId {
    let dst = self.alloc_value(IrType::Bool);
    self.emit(Instruction::Lt { dst, lhs, rhs });
    dst
  }

  /// Emit a less-or-equal comparison.
  pub fn le(&mut self, lhs: ValueId, rhs: ValueId) -> ValueId {
    let dst = self.alloc_value(IrType::Bool);
    self.emit(Instruction::Le { dst, lhs, rhs });
    dst
  }

  /// Emit a greater-than comparison.
  pub fn gt(&mut self, lhs: ValueId, rhs: ValueId) -> ValueId {
    let dst = self.alloc_value(IrType::Bool);
    self.emit(Instruction::Gt { dst, lhs, rhs });
    dst
  }

  /// Emit a greater-or-equal comparison.
  pub fn ge(&mut self, lhs: ValueId, rhs: ValueId) -> ValueId {
    let dst = self.alloc_value(IrType::Bool);
    self.emit(Instruction::Ge { dst, lhs, rhs });
    dst
  }

  // ========== Logical Instructions ==========

  /// Emit a logical AND.
  pub fn and(&mut self, lhs: ValueId, rhs: ValueId) -> ValueId {
    let dst = self.alloc_value(IrType::Bool);
    self.emit(Instruction::And { dst, lhs, rhs });
    dst
  }

  /// Emit a logical OR.
  pub fn or(&mut self, lhs: ValueId, rhs: ValueId) -> ValueId {
    let dst = self.alloc_value(IrType::Bool);
    self.emit(Instruction::Or { dst, lhs, rhs });
    dst
  }

  /// Emit a logical XOR.
  pub fn xor(&mut self, lhs: ValueId, rhs: ValueId) -> ValueId {
    let dst = self.alloc_value(IrType::Bool);
    self.emit(Instruction::Xor { dst, lhs, rhs });
    dst
  }

  /// Emit a logical NOT.
  pub fn not(&mut self, src: ValueId) -> ValueId {
    let dst = self.alloc_value(IrType::Bool);
    self.emit(Instruction::Not { dst, src });
    dst
  }

  // ========== Memory Instructions ==========

  /// Emit a stack allocation, hoisted to the function's entry block.
  ///
  /// All allocas are placed in the entry block so they execute exactly once,
  /// regardless of whether the source-level variable was declared inside a loop.
  /// This prevents unbounded stack growth from loop-body allocas and enables
  /// LLVM's mem2reg pass to promote them to SSA registers.
  pub fn alloca(&mut self, ty: IrType) -> ValueId {
    let dst = self.alloc_value(IrType::ptr(ty.clone()));
    self.emit_to_entry(Instruction::Alloca { dst, ty });
    dst
  }

  /// Allocate memory with a debug name, hoisted to the function's entry block.
  /// See [`alloca`](Self::alloca) for rationale on entry-block hoisting.
  pub fn alloca_named(&mut self, ty: IrType, name: &str) -> ValueId {
    let dst = self.alloc_value_with_name(IrType::ptr(ty.clone()), name);
    self.emit_to_entry(Instruction::Alloca { dst, ty });
    dst
  }

  /// Emit a load from memory.
  pub fn load(&mut self, ptr: ValueId, ty: IrType) -> ValueId {
    let dst = self.alloc_value(ty.clone());
    self.emit(Instruction::Load { dst, ptr, ty });
    dst
  }

  /// Emit a store to memory. Looks up the value type from the current function.
  /// Falls back to `IrType::I64` with a diagnostic if the type cannot be determined.
  pub fn store(&mut self, val: ValueId, ptr: ValueId) {
    let ty = match self
      .current_func
      .and_then(|fid| self.module.try_function(fid).ok())
      .and_then(|f| f.value_type(val).cloned())
    {
      Some(ty) => ty,
      None => {
        eprintln!(
          "WARNING: store value type lookup failed; defaulting to I64. \
           This may emit wrong-typed IR."
        );
        IrType::I64
      }
    };
    self.emit(Instruction::Store { val, ptr, ty });
  }

  /// Emit a bounds check for array access.
  /// Verifies that index < length; calls __lale_error if violated.
  pub fn bounds_check(
    &mut self,
    index: ValueId,
    length: ValueId,
    message: impl Into<String>,
    file: impl Into<String>,
    line: i64,
    column: i64,
  ) {
    let prefix = format!(
      "ERROR at {}:{}:{}: {} (index=",
      file.into(),
      line,
      column,
      message.into()
    );
    self.emit(Instruction::BoundsCheck {
      index,
      length,
      parts: vec![
        RenderPart::Text(prefix),
        RenderPart::Value(index),
        RenderPart::Text(", length=".to_string()),
        RenderPart::Value(length),
        RenderPart::Text(")".to_string()),
      ],
    });
  }

  /// Emit a zero check before division or modulo.
  /// Verifies that operand != 0; calls __lale_error if violated.
  pub fn zero_check(
    &mut self,
    operand: ValueId,
    message: impl Into<String>,
    file: impl Into<String>,
    line: i64,
    column: i64,
  ) {
    let parts = trap_parts(&file.into(), line, column, &message.into());
    self.emit(Instruction::ZeroCheck { operand, parts });
  }

  /// Emit the start of a test case (sets suite/case reporting context).
  pub fn test_begin(&mut self, suite: impl Into<String>, case: impl Into<String>) {
    self.emit(Instruction::TestBegin {
      suite: suite.into(),
      case: case.into(),
    });
  }

  /// Emit a test failure marker (prints `Fail: suite / case` and continues).
  /// `expected`/`found` are optional SSA values formatted into the message.
  pub fn test_fail(
    &mut self,
    file: impl Into<String>,
    line: i64,
    column: i64,
    expected: Option<ValueId>,
    found: Option<ValueId>,
  ) {
    self.emit(Instruction::TestFail {
      file: file.into(),
      line,
      column,
      expected,
      found,
    });
  }

  /// Emit the end of a test case (prints `Pass: suite / case` if no failure).
  pub fn test_end(&mut self) {
    self.emit(Instruction::TestEnd);
  }

  /// Unwrap an optional struct value: check is_present, extract value or call __lale_error.
  /// src must be a struct value of type struct_name with fields (is_present: bool, value: T).
  #[allow(clippy::too_many_arguments)]
  pub fn unwrap_optional(
    &mut self,
    src: ValueId,
    struct_name: impl Into<String>,
    inner_type: IrType,
    message: impl Into<String>,
    file: impl Into<String>,
    line: i64,
    col: i64,
  ) -> ValueId {
    let dst = self.alloc_value(inner_type);
    let parts = trap_parts(&file.into(), line, col, &message.into());
    self.emit(Instruction::UnwrapOptional {
      dst,
      src,
      struct_name: struct_name.into(),
      parts,
    });
    dst
  }

  // ========== Error Stack Operations ==========

  /// Push a string message onto the error stack.
  pub fn push_error(&mut self, message: ValueId) {
    self.emit(Instruction::PushError { message });
  }

  /// Pop the most recent error message from the stack.
  /// Returns a text value. Caller must ensure stack is non-empty.
  pub fn pop_error(&mut self) -> ValueId {
    let dst = self.alloc_value(IrType::struct_ref("text"));
    self.emit(Instruction::PopError { dst });
    dst
  }

  /// Get the number of messages on the error stack as an i64.
  pub fn error_count(&mut self) -> ValueId {
    let dst = self.alloc_value(IrType::I64);
    self.emit(Instruction::ErrorCount { dst });
    dst
  }

  /// Drain all error messages to stdout (to_stderr = false) or stderr (to_stderr = true).
  /// Optional prefix is prepended to each message; when `timestamp` is set, a
  /// runtime UTC timestamp is inserted between the prefix and the message.
  pub fn drain_errors(&mut self, to_stderr: bool, prefix: Option<String>, timestamp: bool) {
    self.emit(Instruction::DrainErrors {
      to_stderr,
      prefix,
      timestamp,
    });
  }

  /// Emit a get-element-pointer for arrays.
  /// Note: Returns a raw pointer; caller should track element type.
  pub fn gep(&mut self, base: ValueId, indices: Vec<ValueId>) -> ValueId {
    let dst = self.alloc_value(IrType::raw_ptr());
    self.emit(Instruction::GetElementPtr { dst, base, indices });
    dst
  }

  /// Emit a get-field-pointer for structs.
  /// Note: Returns a raw pointer; caller should track field type.
  /// Get a pointer to a struct field using byte offset stored in IR.
  /// The byte offset is computed from the struct layout in the module.
  pub fn get_field_ptr(
    &mut self,
    base: ValueId,
    struct_name: impl Into<String>,
    field_index: u32,
  ) -> ValueId {
    let name = struct_name.into();
    let byte_offset = self
      .module
      .struct_def(&name)
      .and_then(|layout| layout.field_offset_by_index(field_index as usize))
      .unwrap_or_else(|| {
        ice!(
          "struct '{}' field {} offset unknown — struct must be registered before field access",
          name,
          field_index
        )
      });
    let dst = self.alloc_value(IrType::raw_ptr());
    self.emit(Instruction::GetFieldPtr {
      dst,
      base,
      struct_name: name,
      byte_offset,
    });
    dst
  }

  /// Emit a global address reference.
  /// Note: Returns a raw pointer; caller should track global type.
  pub fn global_addr(&mut self, global: GlobalId) -> ValueId {
    let dst = self.alloc_value(IrType::raw_ptr());
    self.emit(Instruction::GlobalAddr { dst, global });
    dst
  }

  /// Emit a global address reference by name.
  /// Panics if the global does not exist.
  pub fn global_addr_by_name(&mut self, name: &str) -> ValueId {
    let global = self
      .module
      .global_by_name(name)
      .unwrap_or_else(|| ice!("Global '{}' not found in module at IR gen time", name));
    let global_id = global.id;
    self.global_addr(global_id)
  }

  // ========== Control Flow Instructions ==========

  /// Emit an unconditional branch.
  pub fn br(&mut self, target: BlockId) {
    self.emit(Instruction::Br { target });
  }

  /// Emit a conditional branch.
  pub fn cond_br(&mut self, cond: ValueId, then_block: BlockId, else_block: BlockId) {
    self.emit(Instruction::CondBr {
      cond,
      then_block,
      else_block,
    });
  }

  /// Emit a return with value.
  pub fn ret(&mut self, val: ValueId) {
    self.emit(Instruction::Ret { val });
  }

  /// Emit a void return.
  pub fn ret_void(&mut self) {
    self.emit(Instruction::RetVoid);
  }

  // ========== Function Call Instructions ==========

  /// Emit a function call with return value.
  pub fn call(&mut self, func: FuncRef, args: Vec<ValueId>, ret_ty: IrType) -> ValueId {
    let dst = self.alloc_value(ret_ty);
    self.emit(Instruction::Call {
      dst,
      func,
      args,
      source_file: None,
      source_line: None,
      source_col: None,
    });
    dst
  }

  /// Emit a function call with source location for stack traces.
  pub fn call_at(
    &mut self,
    func: FuncRef,
    args: Vec<ValueId>,
    ret_ty: IrType,
    file: &str,
    line: i64,
    col: i64,
  ) -> ValueId {
    let dst = self.alloc_value(ret_ty);
    self.emit(Instruction::Call {
      dst,
      func,
      args,
      source_file: Some(file.to_string()),
      source_line: Some(line),
      source_col: Some(col),
    });
    dst
  }

  /// Emit a void function call.
  pub fn call_void(&mut self, func: FuncRef, args: Vec<ValueId>) {
    self.emit(Instruction::CallVoid {
      func,
      args,
      source_file: None,
      source_line: None,
      source_col: None,
    });
  }

  /// Emit a call to a named function.
  pub fn call_named(&mut self, name: &str, args: Vec<ValueId>, ret_ty: IrType) -> ValueId {
    let func_ref = if let Some(id) = self.module.func_id(name) {
      FuncRef::Id(id)
    } else {
      FuncRef::External(name.to_string())
    };
    self.call(func_ref, args, ret_ty)
  }

  /// Emit a call to a named function with source location for stack traces.
  pub fn call_named_at(
    &mut self,
    name: &str,
    args: Vec<ValueId>,
    ret_ty: IrType,
    file: &str,
    line: i64,
    col: i64,
  ) -> ValueId {
    let func_ref = if let Some(id) = self.module.func_id(name) {
      FuncRef::Id(id)
    } else {
      FuncRef::External(name.to_string())
    };
    self.call_at(func_ref, args, ret_ty, file, line, col)
  }

  /// Emit a void call to a named function.
  pub fn call_void_named(&mut self, name: &str, args: Vec<ValueId>) {
    let func_ref = if let Some(id) = self.module.func_id(name) {
      FuncRef::Id(id)
    } else {
      FuncRef::External(name.to_string())
    };
    self.call_void(func_ref, args);
  }

  // ========== Type Conversion Instructions ==========

  /// Emit a truncation.
  pub fn trunc(&mut self, src: ValueId, to_ty: IrType) -> ValueId {
    let dst = self.alloc_value(to_ty.clone());
    self.emit(Instruction::Trunc { dst, src, to_ty });
    dst
  }

  /// Emit a sign extension.
  pub fn sext(&mut self, src: ValueId, to_ty: IrType) -> ValueId {
    let dst = self.alloc_value(to_ty.clone());
    self.emit(Instruction::SExt { dst, src, to_ty });
    dst
  }

  /// Emit a zero extension.
  pub fn zext(&mut self, src: ValueId, to_ty: IrType) -> ValueId {
    let dst = self.alloc_value(to_ty.clone());
    self.emit(Instruction::ZExt { dst, src, to_ty });
    dst
  }

  /// Emit a float-to-signed-int conversion.
  pub fn fp_to_si(&mut self, src: ValueId, to_ty: IrType) -> ValueId {
    let dst = self.alloc_value(to_ty.clone());
    self.emit(Instruction::FpToSi { dst, src, to_ty });
    dst
  }

  /// Emit a float-to-unsigned-int conversion.
  pub fn fp_to_ui(&mut self, src: ValueId, to_ty: IrType) -> ValueId {
    let dst = self.alloc_value(to_ty.clone());
    self.emit(Instruction::FpToUi { dst, src, to_ty });
    dst
  }

  /// Emit a signed-int-to-float conversion.
  pub fn si_to_fp(&mut self, src: ValueId, to_ty: IrType) -> ValueId {
    let dst = self.alloc_value(to_ty.clone());
    self.emit(Instruction::SiToFp { dst, src, to_ty });
    dst
  }

  /// Emit an unsigned-int-to-float conversion.
  pub fn ui_to_fp(&mut self, src: ValueId, to_ty: IrType) -> ValueId {
    let dst = self.alloc_value(to_ty.clone());
    self.emit(Instruction::UiToFp { dst, src, to_ty });
    dst
  }

  /// Emit a float truncation.
  pub fn fp_trunc(&mut self, src: ValueId, to_ty: IrType) -> ValueId {
    let dst = self.alloc_value(to_ty.clone());
    self.emit(Instruction::FpTrunc { dst, src, to_ty });
    dst
  }

  /// Emit a float extension.
  pub fn fp_ext(&mut self, src: ValueId, to_ty: IrType) -> ValueId {
    let dst = self.alloc_value(to_ty.clone());
    self.emit(Instruction::FpExt { dst, src, to_ty });
    dst
  }

  /// Emit a bitcast (reinterpret bits as different type, same size required).
  pub fn bitcast(&mut self, src: ValueId, to_ty: IrType) -> ValueId {
    let dst = self.alloc_value(to_ty.clone());
    self.emit(Instruction::Bitcast { dst, src, to_ty });
    dst
  }

  /// Emit an integer-to-pointer conversion.
  pub fn int_to_ptr(&mut self, src: ValueId) -> ValueId {
    let dst = self.alloc_value(IrType::raw_ptr());
    self.emit(Instruction::IntToPtr { dst, src });
    dst
  }

  // ========== String Operations ==========

  /// Emit a string concatenation.
  /// Takes two text struct values and returns a text struct value.
  /// The data pointer field of the result is heap-allocated and must be
  /// freed by the caller (the text auto-free pass handles this automatically).
  pub fn concat(&mut self, lhs: ValueId, rhs: ValueId) -> ValueId {
    let dst = self.alloc_value(IrType::struct_ref("text"));
    self.emit(Instruction::Concat { dst, lhs, rhs });
    dst
  }

  /// Deep-copy a str's data without freeing the source.
  /// Returns a fresh text struct value whose data is independently allocated.
  pub fn copy_text(&mut self, src: ValueId) -> ValueId {
    let dst = self.alloc_value(IrType::struct_ref("text"));
    self.emit(Instruction::TextCopy { dst, src });
    dst
  }

  /// Recursively deep-copy an aggregate value. The caller supplies the value's
  /// type (nested `str` buffers are freshly allocated by the interpreter's
  /// `DeepCopy` handler).
  pub fn deep_copy(&mut self, src: ValueId, ty: IrType) -> ValueId {
    let dst = self.alloc_value(ty);
    self.emit(Instruction::DeepCopy { dst, src });
    dst
  }

  // ========== text Struct Operations ==========

  /// Allocate a text struct on the stack and initialize with ptr, bytes, chars.
  /// Returns a pointer to the allocated text struct.
  pub fn build_text(&mut self, ptr: ValueId, bytes: ValueId, chars: ValueId) -> ValueId {
    // Allocate space for %text struct on stack
    let text_alloc = self.alloca(IrType::struct_ref("text"));

    // Store ptr field (index 0)
    let ptr_field = self.get_field_ptr(text_alloc, "text", 0);
    self.store(ptr, ptr_field);

    // Store bytes field (index 1)
    let bytes_field = self.get_field_ptr(text_alloc, "text", 1);
    self.store(bytes, bytes_field);

    // Store chars field (index 2)
    let chars_field = self.get_field_ptr(text_alloc, "text", 2);
    self.store(chars, chars_field);

    text_alloc
  }

  /// Build a text struct value directly (not on stack).
  /// Returns a direct struct value that can be returned from functions.
  /// Fields: [ptr, bytes, chars]
  pub fn build_text_value(&mut self, ptr: ValueId, bytes: ValueId, chars: ValueId) -> ValueId {
    self.build_struct_value("text", vec![ptr, bytes, chars])
  }

  /// Extract the ptr field from a text struct.
  /// Takes a pointer to a text struct, returns the ptr value (pointer to i8 bytes).
  pub fn text_get_ptr(&mut self, text_ptr: ValueId) -> ValueId {
    let ptr_field = self.get_field_ptr(text_ptr, "text", 0);
    self.load(ptr_field, IrType::raw_ptr())
  }

  /// Extract the bytes field from a text struct.
  /// Takes a pointer to a text struct, returns the byte length (UTF-8 byte count).
  pub fn text_get_bytes(&mut self, text_ptr: ValueId) -> ValueId {
    let bytes_field = self.get_field_ptr(text_ptr, "text", 1);
    self.load(bytes_field, IrType::U64)
  }

  /// Extract the chars field from a text struct.
  /// Takes a pointer to a text struct, returns the code point count.
  pub fn text_get_chars(&mut self, text_ptr: ValueId) -> ValueId {
    let chars_field = self.get_field_ptr(text_ptr, "text", 2);
    self.load(chars_field, IrType::U64)
  }

  /// Call the write(POSIX) syscall with a text struct pointer.
  /// Takes a pointer to a text struct, extracts ptr and bytes, writes to stdout (fd=1).
  pub fn put_text(&mut self, text_ptr: ValueId) {
    let ptr = self.text_get_ptr(text_ptr);
    let bytes = self.text_get_bytes(text_ptr);
    let fd = self.const_int(IrType::I32, 1);
    self.call_named("write", vec![fd, ptr, bytes], IrType::I64);
  }

  // ========== Struct Value Operations ==========

  /// Build an in-register struct value from individual field values.
  /// This creates a struct value directly (not on the stack).
  /// For stack-allocated structs, use alloca + store instead.
  pub fn build_struct_value(&mut self, struct_name: &str, fields: Vec<ValueId>) -> ValueId {
    let dst = self.alloc_value(IrType::struct_ref(struct_name));
    self.emit(Instruction::BuildStruct {
      dst,
      struct_name: struct_name.to_string(),
      fields,
    });
    dst
  }

  /// Build a 2-component vector value from element values.
  pub fn build_vec2(&mut self, x: ValueId, y: ValueId, inner: IrType) -> ValueId {
    let dst = self.alloc_value(IrType::vec2(inner));
    self.emit(Instruction::BuildVec2 {
      dst,
      elements: [x, y],
    });
    dst
  }

  /// Build a 3-component vector value from element values.
  pub fn build_vec3(&mut self, x: ValueId, y: ValueId, z: ValueId, inner: IrType) -> ValueId {
    let dst = self.alloc_value(IrType::vec3(inner));
    self.emit(Instruction::BuildVec3 {
      dst,
      elements: [x, y, z],
    });
    dst
  }

  /// Build a 4-component vector value from element values.
  pub fn build_vec4(
    &mut self,
    x: ValueId,
    y: ValueId,
    z: ValueId,
    w: ValueId,
    inner: IrType,
  ) -> ValueId {
    let dst = self.alloc_value(IrType::vec4(inner));
    self.emit(Instruction::BuildVec4 {
      dst,
      elements: [x, y, z, w],
    });
    dst
  }

  /// Extract an element from an in-register vector value.
  pub fn extract_vec_element(&mut self, src: ValueId, index: u32, inner_ty: IrType) -> ValueId {
    let dst = self.alloc_value(inner_ty.clone());
    self.emit(Instruction::ExtractVecElement {
      dst,
      src,
      index,
      inner_ty,
    });
    dst
  }

  /// Extract a field value from an in-register struct value.
  /// Takes a struct value and returns the value of the specified field.
  /// For struct pointers, use get_field_ptr + load instead.
  pub fn extract_field(
    &mut self,
    src: ValueId,
    struct_name: &str,
    field_index: u32,
    field_ty: IrType,
  ) -> ValueId {
    let dst = self.alloc_value(field_ty);
    self.emit(Instruction::ExtractField {
      dst,
      src,
      struct_name: struct_name.to_string(),
      field_index,
    });
    dst
  }

  /// Insert a value into a struct field, creating a new struct value.
  /// Takes an existing struct value and returns a new struct with the field replaced.
  pub fn insert_field(
    &mut self,
    src: ValueId,
    struct_name: &str,
    field_index: u32,
    val: ValueId,
  ) -> ValueId {
    let dst = self.alloc_value(IrType::struct_ref(struct_name));
    self.emit(Instruction::InsertField {
      dst,
      src,
      struct_name: struct_name.to_string(),
      field_index,
      val,
    });
    dst
  }

  // ========== Unit Assertions ==========

  /// Emit a runtime unit assertion.
  pub fn assert_unit(&mut self, val: ValueId, expected: NormalizedUnit) {
    self.emit(Instruction::AssertUnit { val, expected });
  }

  // ========== Optional Operations ==========

  /// Wrap a value as Some(value).
  /// Returns a new ValueId with Optional type wrapping the input value's type.
  pub fn some(&mut self, value: ValueId) -> ValueId {
    let inner_type = self
      .get_current_func()
      .and_then(|func_id| self.module.function(func_id))
      .and_then(|func| func.value_type(value).cloned())
      .unwrap_or_else(|| {
        ice!("IrBuilder::some(): cannot determine inner type for Optional — value type not found in current function");
      });
    let opt_type = IrType::optional(inner_type);
    let dst = self.alloc_value(opt_type);
    self.emit(Instruction::Some { dst, value });
    dst
  }

  /// Create a None optional value of the given inner type.
  pub fn none(&mut self, inner_type: IrType) -> ValueId {
    let opt_type = IrType::optional(inner_type);
    let dst = self.alloc_value(opt_type);
    self.emit(Instruction::None { dst });
    dst
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  /// Register the canonical `text` struct (as defined in builtins.lale) so that
  /// builder methods that access `text` fields can resolve field offsets.
  fn register_text_struct(builder: &mut IrBuilder) {
    let mut text_def = StructDef::new("text");
    text_def.add_field("ptr", IrType::raw_ptr());
    text_def.add_field("bytes", IrType::U64);
    text_def.add_field("chars", IrType::U64);
    builder.module_mut().add_struct(text_def);
  }

  #[test]
  fn test_simple_function() {
    let mut builder = IrBuilder::new("test");
    builder.start_function("add", IrType::I32, Linkage::Export);
    let a = builder.add_param("a", IrType::I32, None);
    let b = builder.add_param("b", IrType::I32, None);
    let result = builder.add(a, b, IrType::I32);
    builder.ret(result);

    let module = builder.build();
    let func = module.function_by_name("add").unwrap();
    assert_eq!(func.params.len(), 2);
    assert!(func.is_well_formed());
  }

  #[test]
  fn test_control_flow() {
    let mut builder = IrBuilder::new("test");
    builder.start_function("abs", IrType::I32, Linkage::Export);
    let x = builder.add_param("x", IrType::I32, None);
    let zero = builder.const_int(IrType::I32, 0);
    let cond = builder.lt(x, zero);

    let then_block = builder.create_block("then");
    let else_block = builder.create_block("else");
    let end_block = builder.create_block("end");

    builder.cond_br(cond, then_block, else_block);

    builder.position_at(then_block);
    let neg_x = builder.neg(x, IrType::I32);
    builder.br(end_block);

    builder.position_at(else_block);
    builder.br(end_block);

    builder.position_at(end_block);
    builder.ret(neg_x);

    let module = builder.build();
    let func = module.function_by_name("abs").unwrap();
    assert_eq!(func.blocks.len(), 4);
  }

  #[test]
  fn test_text_struct_build() {
    let mut builder = IrBuilder::new("test");
    register_text_struct(&mut builder);
    builder.start_function("test_text", IrType::Void, Linkage::Export);

    // Create a text struct manually
    let ptr = builder.const_string_ptr("hello");
    let bytes = builder.const_int(IrType::I64, 5); // 5 bytes in UTF-8
    let chars = builder.const_int(IrType::I64, 5); // 5 code points
    let text_ptr = builder.build_text(ptr, bytes, chars);

    // Extract components
    let _extracted_ptr = builder.text_get_ptr(text_ptr);
    let _extracted_bytes = builder.text_get_bytes(text_ptr);
    let _extracted_chars = builder.text_get_chars(text_ptr);

    builder.ret_void();

    let module = builder.build();
    let func = module.function_by_name("test_text").unwrap();
    assert!(func.is_well_formed());
  }

  #[test]
  fn test_const_string_struct() {
    let mut builder = IrBuilder::new("test");
    builder.start_function("test_const", IrType::Void, Linkage::Export);

    // const_string now returns a text struct value
    let text_val = builder.const_string("hello world");

    // For a struct value, we can extract fields directly
    let _ptr = builder.extract_field(text_val, "text", 0, IrType::ptr(IrType::I8));
    let _byte_len = builder.extract_field(text_val, "text", 1, IrType::I64);

    builder.ret_void();

    let module = builder.build();
    let func = module.function_by_name("test_const").unwrap();
    assert!(func.is_well_formed());
  }

  #[test]
  fn test_put_text() {
    let mut builder = IrBuilder::new("test");
    register_text_struct(&mut builder);
    builder.add_stdlib_externs();
    builder.start_function("test_put", IrType::Void, Linkage::Export);

    let text_ptr = builder.const_string("hello");
    builder.put_text(text_ptr);

    builder.ret_void();

    let module = builder.build();
    let func = module.function_by_name("test_put").unwrap();
    assert!(func.is_well_formed());
    // Verify write extern is registered (used by put_text for stdout output)
    assert!(module.has_extern_func("write"));
  }
}
