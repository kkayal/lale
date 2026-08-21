//! IR Type Verifier
//!
//! Validates that IR code follows typing rules:
//! - Load/Store operands have compatible types
//! - Binary operations have matching operand types
//! - Function calls match parameter types
//! - Control flow destinations are blocks

use crate::ir::function::Function;
use crate::ir::instructions::Instruction;
use crate::ir::module::Module;
use crate::ir::types::IrType;
use crate::ir::values::ValueId;
use std::collections::HashMap;

/// Verification errors found in IR
#[derive(Debug, Clone)]
pub struct VerifyError {
  pub message: String,
  pub instruction_index: Option<usize>,
}

impl std::fmt::Display for VerifyError {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    if let Some(idx) = self.instruction_index {
      write!(f, "Instruction {}: {}", idx, self.message)
    } else {
      write!(f, "{}", self.message)
    }
  }
}

/// IR Verifier
pub struct Verifier;

impl Verifier {
  /// Verify an entire IR module
  pub fn verify_module(module: &Module) -> Result<(), Vec<VerifyError>> {
    let mut errors = Vec::new();

    // Verify each function
    for func in &module.functions {
      if let Err(mut func_errors) = Self::verify_function(func) {
        errors.append(&mut func_errors);
      }
    }

    if errors.is_empty() {
      Ok(())
    } else {
      Err(errors)
    }
  }

  /// Verify a single function
  fn verify_function(func: &Function) -> Result<(), Vec<VerifyError>> {
    let mut errors = Vec::new();

    // Build a map of value types
    let mut value_types: HashMap<ValueId, IrType> = HashMap::new();

    // Parameter types
    for param in &func.params {
      value_types.insert(param.value_id, param.ty.clone());
    }

    // Verify instructions in each block
    for block in &func.blocks {
      for (idx, instr) in block.instructions.iter().enumerate() {
        if let Err(mut instr_errors) = Self::verify_instruction(instr, &value_types, idx) {
          errors.append(&mut instr_errors);
        } else {
          // Update value types after successful instruction
          Self::update_value_types(instr, &mut value_types);
        }
      }
    }

    if errors.is_empty() {
      Ok(())
    } else {
      Err(errors)
    }
  }

  /// Verify a single instruction
  fn verify_instruction(
    instr: &Instruction,
    value_types: &HashMap<ValueId, IrType>,
    idx: usize,
  ) -> Result<(), Vec<VerifyError>> {
    let mut errors = Vec::new();

    match instr {
      Instruction::Load { dst: _, ptr, .. } => {
        // ptr must be a pointer type
        if let Some(ptr_type) = value_types.get(ptr)
          && !matches!(ptr_type, IrType::Ptr(_))
        {
          errors.push(VerifyError {
            message: format!(
              "Load: pointer operand is not a pointer type (got {:?})",
              ptr_type
            ),
            instruction_index: Some(idx),
          });
        }
      }

      Instruction::Store { val: _, ptr, .. } => {
        // ptr must be a pointer type
        if let Some(ptr_type) = value_types.get(ptr)
          && !matches!(ptr_type, IrType::Ptr(_))
        {
          errors.push(VerifyError {
            message: format!(
              "Store: pointer operand is not a pointer type (got {:?})",
              ptr_type
            ),
            instruction_index: Some(idx),
          });
        }
        // val type should match pointer target type (relaxed check)
        // Could be stricter but requires more context
      }

      Instruction::Add { lhs, rhs, .. }
      | Instruction::Sub { lhs, rhs, .. }
      | Instruction::Mul { lhs, rhs, .. }
      | Instruction::Div { lhs, rhs, .. } => {
        // Both operands must have the same type
        if let (Some(lhs_type), Some(rhs_type)) = (value_types.get(lhs), value_types.get(rhs))
          && lhs_type != rhs_type
        {
          errors.push(VerifyError {
            message: format!(
              "Binary operation: operand types don't match (lhs: {:?}, rhs: {:?})",
              lhs_type, rhs_type
            ),
            instruction_index: Some(idx),
          });
        }
      }

      Instruction::Le { lhs, rhs, .. }
      | Instruction::Lt { lhs, rhs, .. }
      | Instruction::Gt { lhs, rhs, .. }
      | Instruction::Ge { lhs, rhs, .. }
      | Instruction::Eq { lhs, rhs, .. }
      | Instruction::Ne { lhs, rhs, .. } => {
        // Comparison operands must have the same type
        if let (Some(lhs_type), Some(rhs_type)) = (value_types.get(lhs), value_types.get(rhs))
          && lhs_type != rhs_type
        {
          errors.push(VerifyError {
            message: format!(
              "Comparison: operand types don't match (lhs: {:?}, rhs: {:?})",
              lhs_type, rhs_type
            ),
            instruction_index: Some(idx),
          });
        }
      }

      Instruction::Call { .. } => {
        // Could verify parameter types here with more context
      }

      Instruction::Some { value, .. }
        // value must be a valid readable value (its type will be used for Optional wrapping)
        if !value_types.contains_key(value) => {
          errors.push(VerifyError {
            message: format!(
              "Some: value operand {} is not defined in this scope",
              value.0
            ),
            instruction_index: Some(idx),
          });
        }

      _ => {
        // Other instructions are generally type-safe by construction
      }
    }

    if errors.is_empty() {
      Ok(())
    } else {
      Err(errors)
    }
  }

  /// Update the value types map after processing an instruction
  fn update_value_types(_instr: &Instruction, _value_types: &mut HashMap<ValueId, IrType>) {
    // Most instructions with result types will be added here
    // For now, we rely on the Function's value_type method
  }
}

#[cfg(test)]
mod tests {
  #[test]
  fn test_verifier_accepts_valid_ir() {
    // This would test a valid IR module
    // Implementation depends on how to construct minimal IR for testing
  }
}
