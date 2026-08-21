//! IR Functions
//!
//! Functions are the primary code containers in the IR. Each function has
//! a signature, entry block, and zero or more additional blocks.

use super::blocks::BasicBlock;
use super::types::IrType;
use super::values::{BlockId, FuncId, ValueId, ValueInfo};
use crate::types::NormalizedUnit;
use std::collections::HashMap;

/// Linkage type for functions and globals.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Linkage {
  /// Internal to this module (not visible externally)
  Internal,
  /// Exported from this module (visible externally)
  Export,
  /// Imported from another module
  Import,
}

/// A function parameter.
#[derive(Debug, Clone)]
pub struct Parameter {
  /// Parameter name (for debugging)
  pub name: String,
  /// Parameter type
  pub ty: IrType,
  /// Optional physical unit
  pub unit: Option<NormalizedUnit>,
  /// ValueId assigned to this parameter in the entry block
  pub value_id: ValueId,
}

/// An IR function definition.
#[derive(Debug, Clone)]
pub struct Function {
  /// Unique identifier
  pub id: FuncId,
  /// Function name
  pub name: String,
  /// Function parameters
  pub params: Vec<Parameter>,
  /// Return type (Void for procedures)
  pub return_type: IrType,
  /// Optional return unit
  pub return_unit: Option<NormalizedUnit>,
  /// Basic blocks (first is entry)
  pub blocks: Vec<BasicBlock>,
  /// Entry block ID
  pub entry_block: BlockId,
  /// Linkage (internal/export/import)
  pub linkage: Linkage,
  /// Value metadata (types, units, names)
  pub values: HashMap<ValueId, ValueInfo>,
  /// Next available ValueId
  next_value_id: u32,
  /// Next available BlockId
  next_block_id: u32,
}

impl Function {
  /// Create a new function with an empty entry block.
  pub fn new(id: FuncId, name: impl Into<String>, return_type: IrType, linkage: Linkage) -> Self {
    let entry_block = BlockId::new(0);
    let entry = BasicBlock::new(entry_block, "entry");

    Function {
      id,
      name: name.into(),
      params: Vec::new(),
      return_type,
      return_unit: None,
      blocks: vec![entry],
      entry_block,
      linkage,
      values: HashMap::new(),
      next_value_id: 0,
      next_block_id: 1,
    }
  }

  /// Add a parameter to this function.
  pub fn add_param(
    &mut self,
    name: impl Into<String>,
    ty: IrType,
    unit: Option<NormalizedUnit>,
  ) -> ValueId {
    let value_id = self.alloc_value(ty.clone());
    let name = name.into();

    if let Some(info) = self.values.get_mut(&value_id) {
      info.name = Some(name.clone());
      info.unit = unit.clone();
    }

    self.params.push(Parameter {
      name,
      ty,
      unit,
      value_id,
    });

    value_id
  }

  /// Set the return unit.
  pub fn set_return_unit(&mut self, unit: NormalizedUnit) {
    self.return_unit = Some(unit);
  }

  /// Allocate a new ValueId with the given type.
  pub fn alloc_value(&mut self, ty: IrType) -> ValueId {
    let id = ValueId::new(self.next_value_id);
    self.next_value_id += 1;
    self.values.insert(id, ValueInfo::new(ty));
    id
  }

  /// Allocate a new ValueId with type and unit.
  pub fn alloc_value_with_unit(&mut self, ty: IrType, unit: NormalizedUnit) -> ValueId {
    let id = ValueId::new(self.next_value_id);
    self.next_value_id += 1;
    self.values.insert(id, ValueInfo::with_unit(ty, unit));
    id
  }

  /// Create a new basic block and return its ID.
  pub fn create_block(&mut self, name: impl Into<String>) -> BlockId {
    let id = BlockId::new(self.next_block_id);
    self.next_block_id += 1;
    self.blocks.push(BasicBlock::new(id, name));
    id
  }

  /// Get a mutable reference to a block by ID.
  pub fn block_mut(&mut self, id: BlockId) -> Option<&mut BasicBlock> {
    self.blocks.iter_mut().find(|b| b.id == id)
  }

  /// Get a reference to a block by ID.
  pub fn block(&self, id: BlockId) -> Option<&BasicBlock> {
    self.blocks.iter().find(|b| b.id == id)
  }

  /// Get the entry block.
  ///
  /// # Panics
  /// Panics if entry block doesn't exist (invariant: always exists after construction).
  #[allow(clippy::expect_used)]
  pub fn entry(&self) -> &BasicBlock {
    self
      .block(self.entry_block)
      .expect("Invariant: entry block must exist after construction")
  }

  /// Get the entry block mutably.
  ///
  /// # Panics
  /// Panics if entry block doesn't exist (invariant: always exists after construction).
  #[allow(clippy::expect_used)]
  pub fn entry_mut(&mut self) -> &mut BasicBlock {
    let entry_id = self.entry_block;
    self
      .block_mut(entry_id)
      .expect("Invariant: entry block must exist after construction")
  }

  /// Get the entry block, returning None if it doesn't exist.
  pub fn try_entry(&self) -> Option<&BasicBlock> {
    self.block(self.entry_block)
  }

  /// Get the entry block mutably, returning None if it doesn't exist.
  pub fn try_entry_mut(&mut self) -> Option<&mut BasicBlock> {
    let entry_id = self.entry_block;
    self.block_mut(entry_id)
  }

  /// Get type information for a value.
  pub fn value_type(&self, id: ValueId) -> Option<&IrType> {
    self.values.get(&id).map(|v| &v.ty)
  }

  /// Get unit information for a value.
  pub fn value_unit(&self, id: ValueId) -> Option<&NormalizedUnit> {
    self.values.get(&id).and_then(|v| v.unit.as_ref())
  }

  /// Returns true if all blocks are properly terminated.
  pub fn is_well_formed(&self) -> bool {
    self.blocks.iter().all(|b| b.is_terminated())
  }
}

/// An external function declaration (imported from runtime/libc).
#[derive(Debug, Clone)]
pub struct ExternFunc {
  /// Function name
  pub name: String,
  /// Parameter types
  pub params: Vec<IrType>,
  /// Return type
  pub return_type: IrType,
  /// Whether this function is variadic
  pub variadic: bool,
}

impl ExternFunc {
  /// Create a new external function declaration.
  pub fn new(name: impl Into<String>, params: Vec<IrType>, return_type: IrType) -> Self {
    ExternFunc {
      name: name.into(),
      params,
      return_type,
      variadic: false,
    }
  }

  /// Create a variadic external function.
  pub fn variadic(name: impl Into<String>, params: Vec<IrType>, return_type: IrType) -> Self {
    ExternFunc {
      name: name.into(),
      params,
      return_type,
      variadic: true,
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_new_function() {
    let func = Function::new(FuncId::new(0), "main", IrType::Void, Linkage::Export);
    assert_eq!(func.name, "main");
    assert_eq!(func.return_type, IrType::Void);
    assert_eq!(func.blocks.len(), 1);
    assert_eq!(func.entry().name, "entry");
  }

  #[test]
  fn test_add_param() {
    let mut func = Function::new(FuncId::new(0), "add", IrType::I32, Linkage::Internal);
    let a = func.add_param("a", IrType::I32, None);
    let b = func.add_param("b", IrType::I32, None);
    assert_eq!(func.params.len(), 2);
    assert_eq!(a, ValueId::new(0));
    assert_eq!(b, ValueId::new(1));
  }

  #[test]
  fn test_create_block() {
    let mut func = Function::new(FuncId::new(0), "test", IrType::Void, Linkage::Internal);
    let then_block = func.create_block("then");
    let else_block = func.create_block("else");
    assert_eq!(func.blocks.len(), 3);
    assert_eq!(then_block, BlockId::new(1));
    assert_eq!(else_block, BlockId::new(2));
  }

  #[test]
  fn test_alloc_value() {
    let mut func = Function::new(FuncId::new(0), "test", IrType::Void, Linkage::Internal);
    let v1 = func.alloc_value(IrType::I32);
    let v2 = func.alloc_value(IrType::F64);
    assert_eq!(v1, ValueId::new(0));
    assert_eq!(v2, ValueId::new(1));
    assert_eq!(func.value_type(v1), Some(&IrType::I32));
    assert_eq!(func.value_type(v2), Some(&IrType::F64));
  }
}
