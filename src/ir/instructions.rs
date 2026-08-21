//! IR Instructions
//!
//! Defines all instruction types in the Lale IR. Instructions are the atomic
//! operations that make up the IR.

use super::types::IrType;
use super::values::{BlockId, FuncId, GlobalId, ValueId};
use crate::types::NormalizedUnit;
use std::fmt;

/// All IR instructions.
///
/// Each instruction produces zero or one value (SSA form).
/// Instructions that produce a value have a `dst` field.
#[derive(Debug, Clone)]
pub enum Instruction {
  // ========== Arithmetic Operations ==========
  /// Integer/float addition: dst = lhs + rhs
  Add {
    dst: ValueId,
    lhs: ValueId,
    rhs: ValueId,
  },
  /// Integer/float subtraction: dst = lhs - rhs
  Sub {
    dst: ValueId,
    lhs: ValueId,
    rhs: ValueId,
  },
  /// Integer/float multiplication: dst = lhs * rhs
  Mul {
    dst: ValueId,
    lhs: ValueId,
    rhs: ValueId,
  },
  /// Integer/float division: dst = lhs / rhs
  Div {
    dst: ValueId,
    lhs: ValueId,
    rhs: ValueId,
  },
  /// Integer remainder: dst = lhs % rhs
  Rem {
    dst: ValueId,
    lhs: ValueId,
    rhs: ValueId,
  },
  /// Negation: dst = -src
  Neg { dst: ValueId, src: ValueId },
  /// Checked integer addition: dst = lhs + rhs, trap on overflow/underflow.
  /// The `ty` field is the declared integer width of the result, which the
  /// interpreter needs because SSA registers always hold i64/u64 values.
  CheckedAdd {
    dst: ValueId,
    lhs: ValueId,
    rhs: ValueId,
    ty: IrType,
    file: String,
    line: i64,
    column: i64,
  },
  /// Checked integer subtraction: dst = lhs - rhs, trap on overflow/underflow.
  CheckedSub {
    dst: ValueId,
    lhs: ValueId,
    rhs: ValueId,
    ty: IrType,
    file: String,
    line: i64,
    column: i64,
  },
  /// Checked integer multiplication: dst = lhs * rhs, trap on overflow/underflow.
  CheckedMul {
    dst: ValueId,
    lhs: ValueId,
    rhs: ValueId,
    ty: IrType,
    file: String,
    line: i64,
    column: i64,
  },
  /// Checked integer negation: dst = -src, trap on overflow/underflow.
  CheckedNeg {
    dst: ValueId,
    src: ValueId,
    ty: IrType,
    file: String,
    line: i64,
    column: i64,
  },
  /// Pointer difference: dst = ptr1 - ptr2 (returns signed i64 byte offset)
  PtrDiff {
    dst: ValueId,
    lhs: ValueId,
    rhs: ValueId,
  },
  /// Exponentiation: dst = base ^ exp
  Pow {
    dst: ValueId,
    base: ValueId,
    exp: ValueId,
  },
  /// Vector cross product: dst = lhs × rhs
  Cross {
    dst: ValueId,
    lhs: ValueId,
    rhs: ValueId,
  },
  /// Vector dot product: dst = lhs · rhs
  Dot {
    dst: ValueId,
    lhs: ValueId,
    rhs: ValueId,
  },
  /// Build a 2-component vector from element values
  BuildVec2 {
    dst: ValueId,
    elements: [ValueId; 2],
  },
  /// Build a 3-component vector from element values
  BuildVec3 {
    dst: ValueId,
    elements: [ValueId; 3],
  },
  /// Build a 4-component vector from element values
  BuildVec4 {
    dst: ValueId,
    elements: [ValueId; 4],
  },
  /// Extract an element from a vector value
  ExtractVecElement {
    dst: ValueId,
    src: ValueId,
    index: u32,
    inner_ty: IrType,
  },

  // ========== Bitwise Operations ==========
  /// Bitwise AND: dst = lhs & rhs
  BitAnd {
    dst: ValueId,
    lhs: ValueId,
    rhs: ValueId,
  },
  /// Bitwise OR: dst = lhs | rhs
  BitOr {
    dst: ValueId,
    lhs: ValueId,
    rhs: ValueId,
  },
  /// Bitwise XOR: dst = lhs ^ rhs
  BitXor {
    dst: ValueId,
    lhs: ValueId,
    rhs: ValueId,
  },
  /// Bitwise NOT: dst = ~src
  BitNot { dst: ValueId, src: ValueId },
  /// Shift left: dst = lhs << rhs
  Shl {
    dst: ValueId,
    lhs: ValueId,
    rhs: ValueId,
  },
  /// Arithmetic shift right (signed): dst = lhs >> rhs
  Shr {
    dst: ValueId,
    lhs: ValueId,
    rhs: ValueId,
  },
  /// Logical shift right (unsigned): dst = lhs >>> rhs
  UShr {
    dst: ValueId,
    lhs: ValueId,
    rhs: ValueId,
  },

  // ========== Comparison Operations ==========
  /// Equal: dst = (lhs == rhs)
  Eq {
    dst: ValueId,
    lhs: ValueId,
    rhs: ValueId,
  },
  /// Not equal: dst = (lhs != rhs)
  Ne {
    dst: ValueId,
    lhs: ValueId,
    rhs: ValueId,
  },
  /// Less than: dst = (lhs < rhs)
  Lt {
    dst: ValueId,
    lhs: ValueId,
    rhs: ValueId,
  },
  /// Less or equal: dst = (lhs <= rhs)
  Le {
    dst: ValueId,
    lhs: ValueId,
    rhs: ValueId,
  },
  /// Greater than: dst = (lhs > rhs)
  Gt {
    dst: ValueId,
    lhs: ValueId,
    rhs: ValueId,
  },
  /// Greater or equal: dst = (lhs >= rhs)
  Ge {
    dst: ValueId,
    lhs: ValueId,
    rhs: ValueId,
  },

  // ========== Logical Operations ==========
  /// Logical AND: dst = lhs && rhs
  And {
    dst: ValueId,
    lhs: ValueId,
    rhs: ValueId,
  },
  /// Logical OR: dst = lhs || rhs
  Or {
    dst: ValueId,
    lhs: ValueId,
    rhs: ValueId,
  },
  /// Logical XOR: dst = lhs ^^ rhs
  Xor {
    dst: ValueId,
    lhs: ValueId,
    rhs: ValueId,
  },
  /// Logical NOT: dst = !src
  Not { dst: ValueId, src: ValueId },

  // ========== Memory Operations ==========
  /// Stack allocation: dst = alloca ty
  Alloca { dst: ValueId, ty: IrType },
  /// Load from memory: dst = *ptr (ty is the type being loaded)
  Load {
    dst: ValueId,
    ptr: ValueId,
    ty: IrType,
  },
  /// Store to memory: *ptr = val (ty is the type being stored, for serialization)
  Store {
    val: ValueId,
    ptr: ValueId,
    ty: IrType,
  },
  /// Get element pointer: dst = &base[indices...]
  GetElementPtr {
    dst: ValueId,
    base: ValueId,
    indices: Vec<ValueId>,
  },
  /// Get field pointer: dst = &base + byte_offset
  GetFieldPtr {
    dst: ValueId,
    base: ValueId,
    struct_name: String,
    byte_offset: u64,
  },
  /// Pointer to integer: dst = ptrtoint ptr
  PtrToInt { dst: ValueId, src: ValueId },
  /// Integer to pointer: dst = inttoptr src
  IntToPtr { dst: ValueId, src: ValueId },
  /// Bounds check: verify index < length, call __lale_error if violated
  BoundsCheck {
    index: ValueId,
    length: ValueId,
    message: String,
    file: String,
    line: i64,
    column: i64,
  },
  /// Zero check: verify operand != 0, call __lale_error if violated (for div/mod by zero)
  ZeroCheck {
    operand: ValueId,
    message: String,
    file: String,
    line: i64,
    column: i64,
  },

  // ========== Test Operations ==========
  /// Begin a test case: set the current suite/case context for reporting.
  TestBegin { suite: String, case: String },
  /// Record a test failure: print `Fail: suite / case` and continue. When the
  /// assert condition was an equality comparison (`a == b`), `expected` carries
  /// the right operand and `found` the left operand for a richer message.
  TestFail {
    file: String,
    line: i64,
    column: i64,
    expected: Option<ValueId>,
    found: Option<ValueId>,
  },
  /// End a test case: print `Pass: suite / case` if no failure was recorded.
  TestEnd,

  // ========== Control Flow ==========
  /// Unconditional branch: br target
  Br { target: BlockId },
  /// Conditional branch: br cond, then_block, else_block
  CondBr {
    cond: ValueId,
    then_block: BlockId,
    else_block: BlockId,
  },
  /// Return value: ret val
  Ret { val: ValueId },
  /// Return void: ret void
  RetVoid,

  // ========== Function Calls ==========
  /// Call function with return value: dst = call func(args...)
  Call {
    dst: ValueId,
    func: FuncRef,
    args: Vec<ValueId>,
    /// Source location of the call, for stack traces.
    source_file: Option<String>,
    source_line: Option<i64>,
    source_col: Option<i64>,
  },
  /// Call void function: call func(args...)
  CallVoid {
    func: FuncRef,
    args: Vec<ValueId>,
    /// Source location of the call, for stack traces.
    source_file: Option<String>,
    source_line: Option<i64>,
    source_col: Option<i64>,
  },

  // ========== Type Conversions ==========
  /// Truncate to smaller integer: dst = trunc src to ty
  Trunc {
    dst: ValueId,
    src: ValueId,
    to_ty: IrType,
  },
  /// Sign-extend to larger integer: dst = sext src to ty
  SExt {
    dst: ValueId,
    src: ValueId,
    to_ty: IrType,
  },
  /// Zero-extend to larger integer: dst = zext src to ty
  ZExt {
    dst: ValueId,
    src: ValueId,
    to_ty: IrType,
  },
  /// Float to signed integer: dst = fptosi src to ty
  FpToSi {
    dst: ValueId,
    src: ValueId,
    to_ty: IrType,
  },
  /// Float to unsigned integer: dst = fptoui src to ty
  FpToUi {
    dst: ValueId,
    src: ValueId,
    to_ty: IrType,
  },
  /// Signed integer to float: dst = sitofp src to ty
  SiToFp {
    dst: ValueId,
    src: ValueId,
    to_ty: IrType,
  },
  /// Unsigned integer to float: dst = uitofp src to ty
  UiToFp {
    dst: ValueId,
    src: ValueId,
    to_ty: IrType,
  },
  /// Truncate float: dst = fptrunc src to ty
  FpTrunc {
    dst: ValueId,
    src: ValueId,
    to_ty: IrType,
  },
  /// Extend float: dst = fpext src to ty
  FpExt {
    dst: ValueId,
    src: ValueId,
    to_ty: IrType,
  },
  /// Bitcast: reinterpret bits as different type (same size required)
  /// dst = bitcast src to ty
  Bitcast {
    dst: ValueId,
    src: ValueId,
    to_ty: IrType,
  },

  // ========== Constants ==========
  /// Signed integer constant: dst = const ty val
  ConstInt { dst: ValueId, ty: IrType, val: i64 },
  /// Unsigned integer constant: dst = const ty val
  ConstUint { dst: ValueId, ty: IrType, val: u64 },
  /// Float constant: dst = const ty val
  ConstFloat { dst: ValueId, ty: IrType, val: f64 },
  /// Boolean constant: dst = const bool val
  ConstBool { dst: ValueId, val: bool },
  /// String constant: dst = const str "val"
  ConstString { dst: ValueId, val: String },
  /// Null pointer constant: dst = const ptr null
  ConstNull { dst: ValueId },

  // ========== Global Access ==========
  /// Get address of global variable: dst = global @name
  GlobalAddr { dst: ValueId, global: GlobalId },

  // ========== Unit Assertions ==========
  /// Runtime unit assertion: assertunit val, expected_unit
  AssertUnit {
    val: ValueId,
    expected: NormalizedUnit,
  },

  // ========== String Operations ==========
  /// Concatenate strings: dst = concat lhs, rhs
  Concat {
    dst: ValueId,
    lhs: ValueId,
    rhs: ValueId,
  },
  /// Deep-copy a string's data without freeing the source.
  /// Used by string embedding to preserve live variables.
  StrCopy { dst: ValueId, src: ValueId },

  // ========== Struct Operations ==========
  /// Build a struct value from individual field values: dst = struct { fields... }
  /// The struct_name identifies which struct type is being constructed.
  BuildStruct {
    dst: ValueId,
    struct_name: String,
    fields: Vec<ValueId>,
  },
  /// Extract a field from a struct value: dst = src.field_index
  /// Unlike GetFieldPtr which returns a pointer to the field,
  /// ExtractField extracts the actual value from an in-register struct.
  ExtractField {
    dst: ValueId,
    src: ValueId,
    struct_name: String,
    field_index: u32,
  },
  /// Insert a value into a struct field: dst = insertfield src, field_index, val
  /// Creates a new struct value with the specified field replaced.
  InsertField {
    dst: ValueId,
    src: ValueId,
    struct_name: String,
    field_index: u32,
    val: ValueId,
  },
  /// Compute a pointer to a named field within a struct.
  /// dst = struct_ptr + byte_offset_of(field_name) in the given struct definition.
  StructFieldPtr {
    dst: ValueId,
    struct_ptr: ValueId,
    struct_name: String,
    field_name: String,
  },

  // ========== Optional Operations ==========
  /// Wrap a value as Some(value): dst = some value
  Some { dst: ValueId, value: ValueId },
  /// Create a None optional: dst = none  (inner type determined by context)
  None { dst: ValueId },
  /// Unwrap an optional struct: check is_present, extract value or call __lale_error.
  /// This operates on the struct-based optional representation ({is_present: bool, value: T}).
  UnwrapOptional {
    dst: ValueId,
    src: ValueId,
    struct_name: String,
    message: String,
    file: String,
    line: i64,
    col: i64,
  },

  // ========== Error Stack Operations ==========
  /// Push a string message onto the error stack.
  PushError { message: ValueId },
  /// Pop the most recent error message from the stack. Aborts if empty.
  PopError { dst: ValueId },
  /// Get the number of messages on the error stack.
  ErrorCount { dst: ValueId },
  /// Drain all error messages to the output (stdout or stderr).
  /// Optional prefix is prepended to each message (e.g. red "Error: " for alert).
  DrainErrors {
    to_stderr: bool,
    prefix: Option<String>,
  },
}

/// Reference to a function - either by ID or by name (for externals).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FuncRef {
  /// Internal function reference
  Id(FuncId),
  /// External function reference by name
  External(String),
}

impl fmt::Display for FuncRef {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    match self {
      FuncRef::Id(id) => write!(f, "{}", id),
      FuncRef::External(name) => write!(f, "@{}", name),
    }
  }
}

impl Instruction {
  /// Returns true if this instruction is a terminator (ends a basic block).
  pub fn is_terminator(&self) -> bool {
    matches!(
      self,
      Instruction::Br { .. }
        | Instruction::CondBr { .. }
        | Instruction::Ret { .. }
        | Instruction::RetVoid
    )
  }

  /// Returns the destination ValueId if this instruction produces a value.
  pub fn dest(&self) -> Option<ValueId> {
    match self {
      // Arithmetic
      Instruction::Add { dst, .. }
      | Instruction::Sub { dst, .. }
      | Instruction::Mul { dst, .. }
      | Instruction::Div { dst, .. }
      | Instruction::Rem { dst, .. }
      | Instruction::Neg { dst, .. }
      | Instruction::CheckedAdd { dst, .. }
      | Instruction::CheckedSub { dst, .. }
      | Instruction::CheckedMul { dst, .. }
      | Instruction::CheckedNeg { dst, .. }
      | Instruction::PtrDiff { dst, .. }
      | Instruction::Pow { dst, .. }
      | Instruction::Cross { dst, .. }
      | Instruction::Dot { dst, .. } => Some(*dst),

      // Bitwise
      Instruction::BitAnd { dst, .. }
      | Instruction::BitOr { dst, .. }
      | Instruction::BitXor { dst, .. }
      | Instruction::BitNot { dst, .. }
      | Instruction::Shl { dst, .. }
      | Instruction::Shr { dst, .. }
      | Instruction::UShr { dst, .. } => Some(*dst),

      // Comparison
      Instruction::Eq { dst, .. }
      | Instruction::Ne { dst, .. }
      | Instruction::Lt { dst, .. }
      | Instruction::Le { dst, .. }
      | Instruction::Gt { dst, .. }
      | Instruction::Ge { dst, .. } => Some(*dst),

      // Logical
      Instruction::And { dst, .. }
      | Instruction::Or { dst, .. }
      | Instruction::Xor { dst, .. }
      | Instruction::Not { dst, .. } => Some(*dst),

      // Memory
      Instruction::Alloca { dst, .. }
      | Instruction::Load { dst, .. }
      | Instruction::GetElementPtr { dst, .. }
      | Instruction::GetFieldPtr { dst, .. }
      | Instruction::PtrToInt { dst, .. }
      | Instruction::IntToPtr { dst, .. } => Some(*dst),

      // Calls with return
      Instruction::Call { dst, .. } => Some(*dst),

      // Type conversions
      Instruction::Trunc { dst, .. }
      | Instruction::SExt { dst, .. }
      | Instruction::ZExt { dst, .. }
      | Instruction::FpToSi { dst, .. }
      | Instruction::FpToUi { dst, .. }
      | Instruction::SiToFp { dst, .. }
      | Instruction::UiToFp { dst, .. }
      | Instruction::FpTrunc { dst, .. }
      | Instruction::FpExt { dst, .. }
      | Instruction::Bitcast { dst, .. } => Some(*dst),

      // Constants
      Instruction::ConstInt { dst, .. }
      | Instruction::ConstUint { dst, .. }
      | Instruction::ConstFloat { dst, .. }
      | Instruction::ConstBool { dst, .. }
      | Instruction::ConstString { dst, .. }
      | Instruction::ConstNull { dst } => Some(*dst),

      // Global access
      Instruction::GlobalAddr { dst, .. } => Some(*dst),

      // String ops
      Instruction::Concat { dst, .. } => Some(*dst),
      Instruction::StrCopy { dst, .. } => Some(*dst),

      // Struct ops
      Instruction::BuildStruct { dst, .. }
      | Instruction::ExtractField { dst, .. }
      | Instruction::InsertField { dst, .. }
      | Instruction::StructFieldPtr { dst, .. }
      // Vector constructors
      | Instruction::BuildVec2 { dst, .. }
      | Instruction::BuildVec3 { dst, .. }
      | Instruction::BuildVec4 { dst, .. }
      | Instruction::ExtractVecElement { dst, .. } => Some(*dst),

      // Optional ops
      Instruction::Some { dst, .. }
      | Instruction::None { dst }
      | Instruction::UnwrapOptional { dst, .. } => Some(*dst),

      // Error stack ops
      Instruction::PopError { dst, .. } | Instruction::ErrorCount { dst, .. } => Some(*dst),

      // No destination
      Instruction::Store { .. }
      | Instruction::BoundsCheck { .. }
      | Instruction::ZeroCheck { .. }
      | Instruction::TestBegin { .. }
      | Instruction::TestFail { .. }
      | Instruction::TestEnd
      | Instruction::DrainErrors { .. }
      | Instruction::PushError { .. }
      | Instruction::Br { .. }
      | Instruction::CondBr { .. }
      | Instruction::Ret { .. }
      | Instruction::RetVoid
      | Instruction::CallVoid { .. }
      | Instruction::AssertUnit { .. } => None,
    }
  }

  /// Returns all ValueIds used as operands by this instruction.
  pub fn operands(&self) -> Vec<ValueId> {
    match self {
      // Binary ops
      Instruction::Add { lhs, rhs, .. }
      | Instruction::Sub { lhs, rhs, .. }
      | Instruction::Mul { lhs, rhs, .. }
      | Instruction::Div { lhs, rhs, .. }
      | Instruction::Rem { lhs, rhs, .. }
      | Instruction::CheckedAdd { lhs, rhs, .. }
      | Instruction::CheckedSub { lhs, rhs, .. }
      | Instruction::CheckedMul { lhs, rhs, .. }
      | Instruction::Cross { lhs, rhs, .. }
      | Instruction::Dot { lhs, rhs, .. }
      | Instruction::PtrDiff { lhs, rhs, .. }
      | Instruction::Pow {
        base: lhs,
        exp: rhs,
        ..
      }
      | Instruction::BitAnd { lhs, rhs, .. }
      | Instruction::BitOr { lhs, rhs, .. }
      | Instruction::BitXor { lhs, rhs, .. }
      | Instruction::Shl { lhs, rhs, .. }
      | Instruction::Shr { lhs, rhs, .. }
      | Instruction::UShr { lhs, rhs, .. }
      | Instruction::Eq { lhs, rhs, .. }
      | Instruction::Ne { lhs, rhs, .. }
      | Instruction::Lt { lhs, rhs, .. }
      | Instruction::Le { lhs, rhs, .. }
      | Instruction::Gt { lhs, rhs, .. }
      | Instruction::Ge { lhs, rhs, .. }
      | Instruction::And { lhs, rhs, .. }
      | Instruction::Or { lhs, rhs, .. }
      | Instruction::Xor { lhs, rhs, .. }
      | Instruction::Concat { lhs, rhs, .. } => vec![*lhs, *rhs],
      Instruction::StrCopy { src, .. } => vec![*src],

      // Unary ops
      Instruction::Neg { src, .. }
      | Instruction::CheckedNeg { src, .. }
      | Instruction::BitNot { src, .. }
      | Instruction::Not { src, .. }
      | Instruction::Load { ptr: src, .. }
      | Instruction::PtrToInt { src, .. }
      | Instruction::IntToPtr { src, .. }
      | Instruction::Trunc { src, .. }
      | Instruction::SExt { src, .. }
      | Instruction::ZExt { src, .. }
      | Instruction::FpToSi { src, .. }
      | Instruction::FpToUi { src, .. }
      | Instruction::SiToFp { src, .. }
      | Instruction::UiToFp { src, .. }
      | Instruction::FpTrunc { src, .. }
      | Instruction::FpExt { src, .. }
      | Instruction::Bitcast { src, .. } => vec![*src],

      // Store
      Instruction::Store { val, ptr, .. } => vec![*val, *ptr],

      // GEP
      Instruction::GetElementPtr { base, indices, .. } => {
        let mut ops = vec![*base];
        ops.extend(indices.iter().copied());
        ops
      }
      Instruction::GetFieldPtr { base, .. } => vec![*base],

      // Control flow
      Instruction::Ret { val } => vec![*val],
      Instruction::CondBr { cond, .. } => vec![*cond],

      // Calls
      Instruction::Call { args, .. } | Instruction::CallVoid { args, .. } => args.clone(),

      // Unit assertion
      Instruction::AssertUnit { val, .. } => vec![*val],

      // Bounds check
      Instruction::BoundsCheck { index, length, .. } => vec![*index, *length],

      // Zero check
      Instruction::ZeroCheck { operand, .. } => vec![*operand],

      // Struct ops
      Instruction::BuildStruct { fields, .. } => fields.clone(),
      Instruction::BuildVec2 { elements, .. } => elements.to_vec(),
      Instruction::BuildVec3 { elements, .. } => elements.to_vec(),
      Instruction::BuildVec4 { elements, .. } => elements.to_vec(),
      Instruction::ExtractField { src, .. } => vec![*src],
      Instruction::ExtractVecElement { src, .. } => vec![*src],
      Instruction::InsertField { src, val, .. } => vec![*src, *val],
      Instruction::StructFieldPtr { struct_ptr, .. } => vec![*struct_ptr],

      // Optional ops
      Instruction::Some { value, .. } => vec![*value],
      Instruction::UnwrapOptional { src, .. } => vec![*src],

      // Error stack ops
      Instruction::PushError { message } => vec![*message],
      Instruction::PopError { .. } | Instruction::ErrorCount { .. } => vec![],

      // Test failure carries optional expected/found operands for the message.
      Instruction::TestFail {
        expected, found, ..
      } => {
        let mut ops = Vec::new();
        if let Some(id) = expected {
          ops.push(*id);
        }
        if let Some(id) = found {
          ops.push(*id);
        }
        ops
      }

      // No operands
      Instruction::Alloca { .. }
      | Instruction::Br { .. }
      | Instruction::DrainErrors { .. }
      | Instruction::TestBegin { .. }
      | Instruction::TestEnd
      | Instruction::RetVoid
      | Instruction::ConstInt { .. }
      | Instruction::ConstUint { .. }
      | Instruction::ConstFloat { .. }
      | Instruction::ConstBool { .. }
      | Instruction::ConstString { .. }
      | Instruction::ConstNull { .. }
      | Instruction::None { .. }
      | Instruction::GlobalAddr { .. } => vec![],
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_is_terminator() {
    assert!(
      Instruction::Br {
        target: BlockId::new(0)
      }
      .is_terminator()
    );
    assert!(Instruction::RetVoid.is_terminator());
    assert!(
      !Instruction::Add {
        dst: ValueId::new(0),
        lhs: ValueId::new(1),
        rhs: ValueId::new(2)
      }
      .is_terminator()
    );
  }

  #[test]
  fn test_dest() {
    let add = Instruction::Add {
      dst: ValueId::new(5),
      lhs: ValueId::new(1),
      rhs: ValueId::new(2),
    };
    assert_eq!(add.dest(), Some(ValueId::new(5)));
    assert_eq!(Instruction::RetVoid.dest(), None);
  }

  #[test]
  fn test_operands() {
    let add = Instruction::Add {
      dst: ValueId::new(0),
      lhs: ValueId::new(1),
      rhs: ValueId::new(2),
    };
    assert_eq!(add.operands(), vec![ValueId::new(1), ValueId::new(2)]);
  }

  #[test]
  fn test_build_struct() {
    let inst = Instruction::BuildStruct {
      dst: ValueId::new(0),
      struct_name: "Point".to_string(),
      fields: vec![ValueId::new(1), ValueId::new(2)],
    };
    assert_eq!(inst.dest(), Some(ValueId::new(0)));
    assert_eq!(inst.operands(), vec![ValueId::new(1), ValueId::new(2)]);
    assert!(!inst.is_terminator());
  }

  #[test]
  fn test_extract_field() {
    let inst = Instruction::ExtractField {
      dst: ValueId::new(0),
      src: ValueId::new(1),
      struct_name: "Point".to_string(),
      field_index: 0,
    };
    assert_eq!(inst.dest(), Some(ValueId::new(0)));
    assert_eq!(inst.operands(), vec![ValueId::new(1)]);
    assert!(!inst.is_terminator());
  }

  #[test]
  fn test_insert_field() {
    let inst = Instruction::InsertField {
      dst: ValueId::new(0),
      src: ValueId::new(1),
      struct_name: "Point".to_string(),
      field_index: 0,
      val: ValueId::new(2),
    };
    assert_eq!(inst.dest(), Some(ValueId::new(0)));
    assert_eq!(inst.operands(), vec![ValueId::new(1), ValueId::new(2)]);
    assert!(!inst.is_terminator());
  }

  #[test]
  fn test_struct_field_ptr() {
    let inst = Instruction::StructFieldPtr {
      dst: ValueId::new(0),
      struct_ptr: ValueId::new(1),
      struct_name: "Point".to_string(),
      field_name: "y".to_string(),
    };
    assert_eq!(inst.dest(), Some(ValueId::new(0)));
    assert_eq!(inst.operands(), vec![ValueId::new(1)]);
    assert!(!inst.is_terminator());
  }

  #[test]
  fn test_some_instruction() {
    let inst = Instruction::Some {
      dst: ValueId::new(0),
      value: ValueId::new(1),
    };
    assert_eq!(inst.dest(), Some(ValueId::new(0)));
    assert_eq!(inst.operands(), vec![ValueId::new(1)]);
    assert!(!inst.is_terminator());
  }

  #[test]
  fn test_none_instruction() {
    let inst = Instruction::None {
      dst: ValueId::new(0),
    };
    assert_eq!(inst.dest(), Some(ValueId::new(0)));
    assert!(inst.operands().is_empty());
    assert!(!inst.is_terminator());
  }
}
