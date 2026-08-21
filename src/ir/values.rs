//! IR Values
//!
//! Defines SSA values and their identifiers. In SSA form, each value is
//! defined exactly once and identified by a unique ValueId.

use super::types::IrType;
use crate::types::NormalizedUnit;
use std::fmt;

/// Unique identifier for a value within a function.
///
/// ValueIds are local to each function - the same ID can appear in different
/// functions without conflict.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ValueId(pub u32);

impl ValueId {
  /// Create a new ValueId.
  pub fn new(id: u32) -> Self {
    ValueId(id)
  }

  /// Get the raw ID value.
  pub fn raw(&self) -> u32 {
    self.0
  }
}

impl fmt::Display for ValueId {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    write!(f, "%{}", self.0)
  }
}

/// Unique identifier for a basic block within a function.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct BlockId(pub u32);

impl BlockId {
  pub fn new(id: u32) -> Self {
    BlockId(id)
  }

  pub fn raw(&self) -> u32 {
    self.0
  }
}

impl fmt::Display for BlockId {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    write!(f, "%block{}", self.0)
  }
}

/// Unique identifier for a function within a module.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct FuncId(pub u32);

impl FuncId {
  pub fn new(id: u32) -> Self {
    FuncId(id)
  }

  pub fn raw(&self) -> u32 {
    self.0
  }
}

impl fmt::Display for FuncId {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    // Generic display - actual name lookup should be done at module level
    write!(f, "@func{}", self.0)
  }
}

/// Unique identifier for a global variable within a module.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct GlobalId(pub u32);

impl GlobalId {
  pub fn new(id: u32) -> Self {
    GlobalId(id)
  }

  pub fn raw(&self) -> u32 {
    self.0
  }
}

impl fmt::Display for GlobalId {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    write!(f, "@global{}", self.0)
  }
}

/// Metadata about a value in the IR.
///
/// This is used by the builder to track value types and units during IR construction.
#[derive(Debug, Clone)]
pub struct ValueInfo {
  /// The type of this value
  pub ty: IrType,
  /// Optional physical unit (for runtime assertions)
  pub unit: Option<NormalizedUnit>,
  /// Optional debug name
  pub name: Option<String>,
}

impl ValueInfo {
  /// Create a new ValueInfo with just a type.
  pub fn new(ty: IrType) -> Self {
    ValueInfo {
      ty,
      unit: None,
      name: None,
    }
  }

  /// Create a ValueInfo with type and unit.
  pub fn with_unit(ty: IrType, unit: NormalizedUnit) -> Self {
    ValueInfo {
      ty,
      unit: Some(unit),
      name: None,
    }
  }

  /// Create a ValueInfo with type and name.
  pub fn with_name(ty: IrType, name: impl Into<String>) -> Self {
    ValueInfo {
      ty,
      unit: None,
      name: Some(name.into()),
    }
  }
}

/// An operand in an instruction - either a value reference or an immediate.
#[derive(Debug, Clone, PartialEq)]
pub enum Operand {
  /// Reference to an SSA value
  Value(ValueId),
  /// Immediate integer constant
  ImmInt(i64),
  /// Immediate unsigned constant
  ImmUint(u64),
  /// Immediate float constant
  ImmFloat(f64),
  /// Immediate boolean constant
  ImmBool(bool),
}

impl fmt::Display for Operand {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    match self {
      Operand::Value(id) => write!(f, "{}", id),
      Operand::ImmInt(v) => write!(f, "{}", v),
      Operand::ImmUint(v) => write!(f, "{}u", v),
      Operand::ImmFloat(v) => write!(f, "{}", v),
      Operand::ImmBool(v) => write!(f, "{}", v),
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_value_id_display() {
    assert_eq!(format!("{}", ValueId::new(0)), "%0");
    assert_eq!(format!("{}", ValueId::new(42)), "%42");
  }

  #[test]
  fn test_block_id_display() {
    assert_eq!(format!("{}", BlockId::new(0)), "%block0");
  }

  #[test]
  fn test_func_id_display() {
    assert_eq!(format!("{}", FuncId::new(0)), "@func0");
  }

  #[test]
  fn test_operand_display() {
    assert_eq!(format!("{}", Operand::Value(ValueId::new(5))), "%5");
    assert_eq!(format!("{}", Operand::ImmInt(42)), "42");
    assert_eq!(format!("{}", Operand::ImmBool(true)), "true");
  }
}
