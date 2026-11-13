use lale::ir::{
  BasicBlock, BlockId, Constant, ExternFunc, FuncRef, Instruction, IrType, Linkage, Module,
  RenderPart, StructDef, ValueId, parse_module, print_module,
};

fn build_broad_module() -> Module {
  let mut m = Module::new_empty("roundtrip");

  let mut point = StructDef::new("Point");
  point.add_field("x", IrType::F64);
  point.add_field("y", IrType::F64);
  m.add_struct(point);

  m.add_extern_func(ExternFunc::new(
    "write",
    vec![IrType::I32, IrType::ptr(IrType::I8), IrType::I64],
    IrType::I64,
  ));
  m.add_extern_func(ExternFunc::variadic(
    "printf",
    vec![IrType::ptr(IrType::I8)],
    IrType::I32,
  ));

  m.add_global("answer", IrType::I64, Linkage::Internal);

  let fid = m.add_function("main", IrType::Void, Linkage::Export);
  let func = m.function_mut(fid).expect("function exists");

  let entry = BlockId::new(0);
  let then = BlockId::new(1);
  let else_block = BlockId::new(2);
  let merge = BlockId::new(3);

  let mut entry_block = BasicBlock::new(entry, "entry");
  entry_block.instructions = vec![
    Instruction::ConstInt {
      dst: ValueId::new(0),
      ty: IrType::I64,
      val: 5,
    },
    Instruction::ConstInt {
      dst: ValueId::new(1),
      ty: IrType::I64,
      val: 10,
    },
    Instruction::Add {
      dst: ValueId::new(2),
      lhs: ValueId::new(0),
      rhs: ValueId::new(1),
    },
    Instruction::Sub {
      dst: ValueId::new(3),
      lhs: ValueId::new(1),
      rhs: ValueId::new(0),
    },
    Instruction::Mul {
      dst: ValueId::new(4),
      lhs: ValueId::new(0),
      rhs: ValueId::new(1),
    },
    Instruction::Div {
      dst: ValueId::new(5),
      lhs: ValueId::new(1),
      rhs: ValueId::new(0),
    },
    Instruction::Rem {
      dst: ValueId::new(6),
      lhs: ValueId::new(1),
      rhs: ValueId::new(0),
    },
    Instruction::Neg {
      dst: ValueId::new(7),
      src: ValueId::new(0),
    },
    Instruction::CheckedAdd {
      dst: ValueId::new(8),
      lhs: ValueId::new(0),
      rhs: ValueId::new(1),
      ty: IrType::I64,
      parts: vec![RenderPart::Text(
        "ERROR at main.lale:10:4: integer overflow in signed addition (i64)".to_string(),
      )],
    },
    Instruction::BitAnd {
      dst: ValueId::new(9),
      lhs: ValueId::new(0),
      rhs: ValueId::new(1),
    },
    Instruction::BitNot {
      dst: ValueId::new(10),
      src: ValueId::new(0),
    },
    Instruction::Eq {
      dst: ValueId::new(11),
      lhs: ValueId::new(0),
      rhs: ValueId::new(1),
    },
    Instruction::And {
      dst: ValueId::new(12),
      lhs: ValueId::new(11),
      rhs: ValueId::new(11),
    },
    Instruction::Not {
      dst: ValueId::new(13),
      src: ValueId::new(11),
    },
    Instruction::Alloca {
      dst: ValueId::new(14),
      ty: IrType::I64,
    },
    Instruction::Store {
      val: ValueId::new(2),
      ptr: ValueId::new(14),
      ty: IrType::I64,
    },
    Instruction::Load {
      dst: ValueId::new(15),
      ptr: ValueId::new(14),
      ty: IrType::I64,
    },
    Instruction::GlobalAddr {
      dst: ValueId::new(16),
      global: lale::ir::GlobalId::new(0),
    },
    Instruction::ConstString {
      dst: ValueId::new(17),
      val: "hello \"world\"\n".to_string(),
    },
    Instruction::ConstFloat {
      dst: ValueId::new(18),
      ty: IrType::F64,
      val: 3.5,
    },
    Instruction::ConstBool {
      dst: ValueId::new(19),
      val: true,
    },
    Instruction::ConstNull {
      dst: ValueId::new(20),
    },
    Instruction::Call {
      dst: ValueId::new(21),
      func: FuncRef::External("puts".to_string()),
      args: vec![ValueId::new(17)],
      source_file: Some("main.lale".to_string()),
      source_line: Some(20),
      source_col: Some(5),
    },
    Instruction::CallVoid {
      func: FuncRef::External("puts".to_string()),
      args: vec![ValueId::new(17)],
      source_file: Some("main.lale".to_string()),
      source_line: Some(21),
      source_col: Some(5),
    },
    Instruction::Concat {
      dst: ValueId::new(22),
      lhs: ValueId::new(17),
      rhs: ValueId::new(17),
    },
    Instruction::TextCopy {
      dst: ValueId::new(23),
      src: ValueId::new(17),
    },
    Instruction::BuildStruct {
      dst: ValueId::new(24),
      struct_name: "Point".to_string(),
      fields: vec![ValueId::new(18), ValueId::new(18)],
    },
    Instruction::ExtractField {
      dst: ValueId::new(25),
      src: ValueId::new(24),
      struct_name: "Point".to_string(),
      field_index: 0,
    },
    Instruction::InsertField {
      dst: ValueId::new(26),
      src: ValueId::new(24),
      struct_name: "Point".to_string(),
      field_index: 1,
      val: ValueId::new(18),
    },
    Instruction::StructFieldPtr {
      dst: ValueId::new(27),
      struct_ptr: ValueId::new(14),
      struct_name: "Point".to_string(),
      field_name: "x".to_string(),
    },
    Instruction::GetFieldPtr {
      dst: ValueId::new(28),
      base: ValueId::new(14),
      struct_name: "Point".to_string(),
      byte_offset: 0,
    },
    Instruction::GetElementPtr {
      dst: ValueId::new(29),
      base: ValueId::new(14),
      indices: vec![ValueId::new(0)],
    },
    Instruction::BuildVec2 {
      dst: ValueId::new(30),
      elements: [ValueId::new(18), ValueId::new(18)],
    },
    Instruction::ExtractVecElement {
      dst: ValueId::new(31),
      src: ValueId::new(30),
      index: 0,
      inner_ty: IrType::F64,
    },
    Instruction::Some {
      dst: ValueId::new(32),
      value: ValueId::new(18),
    },
    Instruction::None {
      dst: ValueId::new(33),
    },
    Instruction::UnwrapOptional {
      dst: ValueId::new(34),
      src: ValueId::new(32),
      struct_name: "F64?".to_string(),
      parts: vec![RenderPart::Text(
        "ERROR at main.lale:40:9: unwrap failed".to_string(),
      )],
    },
    Instruction::PushError {
      message: ValueId::new(17),
    },
    Instruction::PopError {
      dst: ValueId::new(35),
    },
    Instruction::ErrorCount {
      dst: ValueId::new(36),
    },
    Instruction::DrainErrors {
      to_stderr: true,
      prefix: Some("Error: ".to_string()),
      timestamp: false,
    },
    Instruction::AssertUnit {
      val: ValueId::new(18),
      expected: lale::types::NormalizedUnit::parse("kg*m^2/s^2"),
    },
    Instruction::TestBegin {
      suite: "suite".to_string(),
      case: "case".to_string(),
    },
    Instruction::TestFail {
      file: "main.lale".to_string(),
      line: 50,
      column: 3,
      expected: None,
      found: None,
    },
    Instruction::TestEnd,
    Instruction::Pow {
      dst: ValueId::new(40),
      base: ValueId::new(0),
      exp: ValueId::new(1),
    },
    Instruction::Cross {
      dst: ValueId::new(41),
      lhs: ValueId::new(30),
      rhs: ValueId::new(30),
    },
    Instruction::Dot {
      dst: ValueId::new(42),
      lhs: ValueId::new(30),
      rhs: ValueId::new(30),
    },
    Instruction::PtrDiff {
      dst: ValueId::new(43),
      lhs: ValueId::new(16),
      rhs: ValueId::new(16),
    },
    Instruction::Shl {
      dst: ValueId::new(44),
      lhs: ValueId::new(0),
      rhs: ValueId::new(1),
    },
    Instruction::Shr {
      dst: ValueId::new(45),
      lhs: ValueId::new(0),
      rhs: ValueId::new(1),
    },
    Instruction::UShr {
      dst: ValueId::new(46),
      lhs: ValueId::new(0),
      rhs: ValueId::new(1),
    },
    Instruction::BitOr {
      dst: ValueId::new(47),
      lhs: ValueId::new(0),
      rhs: ValueId::new(1),
    },
    Instruction::BitXor {
      dst: ValueId::new(48),
      lhs: ValueId::new(0),
      rhs: ValueId::new(1),
    },
    Instruction::Ne {
      dst: ValueId::new(49),
      lhs: ValueId::new(0),
      rhs: ValueId::new(1),
    },
    Instruction::Lt {
      dst: ValueId::new(50),
      lhs: ValueId::new(0),
      rhs: ValueId::new(1),
    },
    Instruction::Le {
      dst: ValueId::new(51),
      lhs: ValueId::new(0),
      rhs: ValueId::new(1),
    },
    Instruction::Gt {
      dst: ValueId::new(52),
      lhs: ValueId::new(0),
      rhs: ValueId::new(1),
    },
    Instruction::Ge {
      dst: ValueId::new(53),
      lhs: ValueId::new(0),
      rhs: ValueId::new(1),
    },
    Instruction::Or {
      dst: ValueId::new(54),
      lhs: ValueId::new(11),
      rhs: ValueId::new(11),
    },
    Instruction::Xor {
      dst: ValueId::new(55),
      lhs: ValueId::new(11),
      rhs: ValueId::new(11),
    },
    Instruction::CheckedSub {
      dst: ValueId::new(56),
      lhs: ValueId::new(1),
      rhs: ValueId::new(0),
      ty: IrType::I64,
      parts: vec![RenderPart::Text(
        "ERROR at main.lale:60:3: integer overflow in signed subtraction (i64)".to_string(),
      )],
    },
    Instruction::CheckedMul {
      dst: ValueId::new(57),
      lhs: ValueId::new(0),
      rhs: ValueId::new(1),
      ty: IrType::I64,
      parts: vec![RenderPart::Text(
        "ERROR at main.lale:61:3: integer overflow in signed multiplication (i64)".to_string(),
      )],
    },
    Instruction::CheckedNeg {
      dst: ValueId::new(58),
      src: ValueId::new(0),
      ty: IrType::I64,
      parts: vec![RenderPart::Text(
        "ERROR at main.lale:62:3: integer overflow in signed negation (i64)".to_string(),
      )],
    },
    Instruction::ConstUint {
      dst: ValueId::new(59),
      ty: IrType::U64,
      val: 7,
    },
    Instruction::Trunc {
      dst: ValueId::new(60),
      src: ValueId::new(0),
      to_ty: IrType::I32,
    },
    Instruction::SExt {
      dst: ValueId::new(61),
      src: ValueId::new(0),
      to_ty: IrType::I64,
    },
    Instruction::ZExt {
      dst: ValueId::new(62),
      src: ValueId::new(59),
      to_ty: IrType::I64,
    },
    Instruction::FpToSi {
      dst: ValueId::new(63),
      src: ValueId::new(18),
      to_ty: IrType::I64,
    },
    Instruction::FpToUi {
      dst: ValueId::new(64),
      src: ValueId::new(18),
      to_ty: IrType::U64,
    },
    Instruction::SiToFp {
      dst: ValueId::new(65),
      src: ValueId::new(0),
      to_ty: IrType::F64,
    },
    Instruction::UiToFp {
      dst: ValueId::new(66),
      src: ValueId::new(59),
      to_ty: IrType::F64,
    },
    Instruction::FpTrunc {
      dst: ValueId::new(67),
      src: ValueId::new(18),
      to_ty: IrType::F32,
    },
    Instruction::FpExt {
      dst: ValueId::new(68),
      src: ValueId::new(18),
      to_ty: IrType::F64,
    },
    Instruction::Bitcast {
      dst: ValueId::new(69),
      src: ValueId::new(18),
      to_ty: IrType::I64,
    },
    Instruction::BoundsCheck {
      index: ValueId::new(0),
      length: ValueId::new(1),
      parts: vec![
        RenderPart::Text("ERROR at main.lale:70:3: out of bounds (index=".to_string()),
        RenderPart::Value(ValueId::new(0)),
        RenderPart::Text(", length=".to_string()),
        RenderPart::Value(ValueId::new(1)),
        RenderPart::Text(")".to_string()),
      ],
    },
    Instruction::ZeroCheck {
      operand: ValueId::new(0),
      parts: vec![RenderPart::Text(
        "ERROR at main.lale:71:3: division by zero".to_string(),
      )],
    },
    Instruction::CondBr {
      cond: ValueId::new(11),
      then_block: then,
      else_block,
    },
  ];

  let mut then_block = BasicBlock::new(then, "then");
  then_block.instructions.push(Instruction::Ret {
    val: ValueId::new(2),
  });

  let mut else_block2 = BasicBlock::new(else_block, "else");
  else_block2
    .instructions
    .push(Instruction::Br { target: merge });

  let mut merge_block = BasicBlock::new(merge, "merge");
  merge_block.instructions.push(Instruction::RetVoid);

  func.blocks = vec![entry_block, then_block, else_block2, merge_block];
  func.entry_block = BlockId::new(0);
  m
}

#[test]
fn broad_module_round_trips_idempotently() {
  let module = build_broad_module();
  let text = print_module(&module);
  let parsed = parse_module(&text).expect("parse printed module");
  let text2 = print_module(&parsed);
  assert_eq!(
    text, text2,
    "serialize -> deserialize -> serialize must be idempotent"
  );
}

#[test]
fn rejects_unsupported_major_version() {
  let text = "; ir-version: 99.0\n; Module: x\n";
  let err = parse_module(text).expect_err("must reject unsupported major");
  assert!(
    err.contains("unsupported IR format version"),
    "err: {}",
    err
  );
}

#[test]
fn parses_globals_and_structs() {
  let mut m = Module::new_empty("g");
  let mut s = StructDef::new("S");
  s.add_field("a", IrType::I32);
  m.add_struct(s);
  let gid = m.add_global("g", IrType::array(IrType::I8, 4), Linkage::Internal);
  m.global_mut(gid).expect("global").initializer = Some(Constant::Array(vec![
    Constant::Int(1),
    Constant::Int(2),
    Constant::Int(3),
    Constant::Int(4),
  ]));

  let text = print_module(&m);
  let parsed = parse_module(&text).expect("parse");
  assert_eq!(print_module(&parsed), text);
}
