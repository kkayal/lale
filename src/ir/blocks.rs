//! Basic Blocks
//!
//! A basic block is a sequence of instructions with a single entry point
//! (the first instruction) and a single exit point (the terminator).

use super::instructions::Instruction;
use super::values::BlockId;

/// A basic block in the control flow graph.
#[derive(Debug, Clone)]
pub struct BasicBlock {
  /// Unique identifier within the function
  pub id: BlockId,
  /// Human-readable name (e.g., "entry", "then", "loop_body")
  pub name: String,
  /// Instructions in this block (last must be a terminator)
  pub instructions: Vec<Instruction>,
}

impl BasicBlock {
  /// Create a new empty basic block.
  pub fn new(id: BlockId, name: impl Into<String>) -> Self {
    BasicBlock {
      id,
      name: name.into(),
      instructions: Vec::new(),
    }
  }

  /// Add an instruction to this block.
  pub fn push(&mut self, inst: Instruction) {
    self.instructions.push(inst);
  }

  /// Returns true if this block has a terminator instruction.
  pub fn is_terminated(&self) -> bool {
    self
      .instructions
      .last()
      .map(|i| i.is_terminator())
      .unwrap_or(false)
  }

  /// Returns the terminator instruction if present.
  pub fn terminator(&self) -> Option<&Instruction> {
    self.instructions.last().filter(|i| i.is_terminator())
  }

  /// Returns the successor block IDs (blocks this block can jump to).
  pub fn successors(&self) -> Vec<BlockId> {
    match self.terminator() {
      Some(Instruction::Br { target }) => vec![*target],
      Some(Instruction::CondBr {
        then_block,
        else_block,
        ..
      }) => vec![*then_block, *else_block],
      Some(Instruction::Ret { .. }) | Some(Instruction::RetVoid) => vec![],
      _ => vec![],
    }
  }

  /// Returns true if this block is empty (no instructions).
  pub fn is_empty(&self) -> bool {
    self.instructions.is_empty()
  }

  /// Returns the number of instructions in this block.
  pub fn len(&self) -> usize {
    self.instructions.len()
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::ir::values::ValueId;

  #[test]
  fn test_new_block() {
    let block = BasicBlock::new(BlockId::new(0), "entry");
    assert_eq!(block.name, "entry");
    assert!(block.is_empty());
    assert!(!block.is_terminated());
  }

  #[test]
  fn test_terminated_block() {
    let mut block = BasicBlock::new(BlockId::new(0), "entry");
    block.push(Instruction::RetVoid);
    assert!(block.is_terminated());
    assert_eq!(block.successors(), vec![]);
  }

  #[test]
  fn test_branch_successors() {
    let mut block = BasicBlock::new(BlockId::new(0), "entry");
    block.push(Instruction::CondBr {
      cond: ValueId::new(0),
      then_block: BlockId::new(1),
      else_block: BlockId::new(2),
    });
    assert_eq!(block.successors(), vec![BlockId::new(1), BlockId::new(2)]);
  }
}
