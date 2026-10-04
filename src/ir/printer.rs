//! IR Printer
//!
//! Pretty-prints IR modules in a human-readable format for debugging
//! and verification.

use super::function::{ExternFunc, Function, Linkage};
use super::instructions::{Instruction, RenderPart};
use super::module::{Constant, Global, Module, StructDef};
use super::values::BlockId;

/// Pretty-print an IR module to a string.
pub fn print_module(module: &Module) -> String {
  let mut printer = IrPrinter::new();
  printer.print_module(module);
  printer.output
}

/// IR printer state.
struct IrPrinter {
  output: String,
  indent: usize,
  module: Option<*const Module>, // Reference to module for function name lookups
}

impl IrPrinter {
  fn new() -> Self {
    IrPrinter {
      output: String::new(),
      indent: 0,
      module: None,
    }
  }

  fn line(&mut self, s: &str) {
    for _ in 0..self.indent {
      self.output.push_str("    ");
    }
    self.output.push_str(s);
    self.output.push('\n');
  }

  /// Format a function reference with proper name lookup
  fn format_func_ref(&self, func_ref: &super::instructions::FuncRef, module: &Module) -> String {
    match func_ref {
      super::instructions::FuncRef::Id(id) => {
        // Look up the actual function name from the module.
        match module.function(*id) {
          Some(func) => format!("@{}", func.name),
          None => crate::ice!(
            "FuncRef::Id({}) does not reference a function in the module",
            id
          ),
        }
      }
      super::instructions::FuncRef::External(name) => format!("@{}", name),
    }
  }

  /// Check if a function call is a struct constructor
  fn get_constructor_comment(
    &self,
    func_ref: &super::instructions::FuncRef,
    module: &Module,
  ) -> Option<String> {
    match func_ref {
      super::instructions::FuncRef::Id(id) => {
        // Look up the function name
        if let Some(func) = module.function(*id) {
          // Check if it's registered as a struct constructor
          if module.is_struct_constructor(&func.name) {
            return Some(format!("struct constructor: {}", func.name));
          }

          // Heuristic: if function has no body and capitalized name, likely a constructor
          // (This helps until structs are fully integrated with IR)
          if func.blocks.is_empty() && func.name.chars().next().is_some_and(|c| c.is_uppercase()) {
            return Some(format!("struct constructor: {}", func.name));
          }
        }
        None
      }
      super::instructions::FuncRef::External(name) => {
        // Check if external function name looks like a constructor (capitalized)
        // This works for struct constructors that haven't been fully integrated
        if name.chars().next().is_some_and(|c| c.is_uppercase()) {
          return Some(format!("struct constructor: {}", name));
        }
        None
      }
    }
  }

  fn print_module(&mut self, module: &Module) {
    self.module = Some(module as *const Module); // Store module reference for function name lookups
    self.line(&format!(
      "; ir-version: {}.{}",
      crate::ir::IR_FORMAT_MAJOR,
      crate::ir::IR_FORMAT_MINOR
    ));
    self.line(&format!("; Module: {}", module.name));
    self.output.push('\n');

    // Print struct definitions
    if !module.structs.is_empty() {
      self.line("; Struct definitions");
      for s in &module.structs {
        self.print_struct(s);
      }
      self.output.push('\n');
    }

    // Print external function declarations
    if !module.extern_funcs.is_empty() {
      self.line("; External functions");
      for ext in &module.extern_funcs {
        self.print_extern_func(ext);
      }
      self.output.push('\n');
    }

    // Print global variables
    if !module.globals.is_empty() {
      self.line("; Global variables");
      for global in &module.globals {
        self.print_global(global);
      }
      self.output.push('\n');
    }

    // Print function definitions
    for func in &module.functions {
      self.print_function(func, module);
      self.output.push('\n');
    }
  }

  fn print_struct(&mut self, s: &StructDef) {
    let fields: Vec<String> = s
      .fields
      .iter()
      .map(|(name, ty)| format!("{}: {}", name, ty))
      .collect();
    self.line(&format!("%{} = type {{ {} }}", s.name, fields.join(", ")));
  }

  fn print_extern_func(&mut self, ext: &ExternFunc) {
    let params: Vec<String> = ext.params.iter().map(|t| format!("{}", t)).collect();
    let variadic = if ext.variadic { ", ..." } else { "" };
    self.line(&format!(
      "declare @{}({}{}) -> {}",
      ext.name,
      params.join(", "),
      variadic,
      ext.return_type
    ));
  }

  fn print_global(&mut self, global: &Global) {
    let linkage = match global.linkage {
      Linkage::Export => "export ",
      Linkage::Import => "import ",
      Linkage::Internal => "",
    };

    let init = match &global.initializer {
      Some(c) => format!(" = {}", Self::format_constant(c)),
      None => String::new(),
    };

    let unit = match &global.unit {
      Some(u) => format!(" ; unit: <{}>", u),
      None => String::new(),
    };

    self.line(&format!(
      "{}@{}: {}{}{}",
      linkage, global.name, global.ty, init, unit
    ));
  }

  fn format_constant(c: &Constant) -> String {
    match c {
      Constant::Int(v) => format!("{}", v),
      Constant::Uint(v) => format!("{}u", v),
      Constant::Float(v) => format!("{}", v),
      Constant::Bool(v) => format!("{}", v),
      Constant::String(s) => format!("\"{}\"", s.escape_default()),
      Constant::Null => "null".to_string(),
      Constant::Zero => "zeroinit".to_string(),
      Constant::Array(elems) => {
        let items: Vec<String> = elems.iter().map(Self::format_constant).collect();
        format!("[{}]", items.join(", "))
      }
      Constant::Struct(fields) => {
        let items: Vec<String> = fields
          .iter()
          .map(|(n, v)| format!("{}: {}", n, Self::format_constant(v)))
          .collect();
        format!("{{ {} }}", items.join(", "))
      }
    }
  }

  fn print_function(&mut self, func: &Function, module: &Module) {
    let linkage = match func.linkage {
      Linkage::Export => "export ",
      Linkage::Import => "import ",
      Linkage::Internal => "",
    };

    let params: Vec<String> = func
      .params
      .iter()
      .map(|p| {
        let unit = match &p.unit {
          Some(u) => format!(" <{}>", u),
          None => String::new(),
        };
        format!("{} {}: {}{}", p.value_id, p.name, p.ty, unit)
      })
      .collect();

    let ret_unit = match &func.return_unit {
      Some(u) => format!(" <{}>", u),
      None => String::new(),
    };

    self.line(&format!(
      "{}func @{}({}) -> {}{} {{",
      linkage,
      func.name,
      params.join(", "),
      func.return_type,
      ret_unit
    ));

    self.indent += 1;
    for block in &func.blocks {
      self.print_block(block, func, module);
    }
    self.indent -= 1;

    self.line("}");
  }

  fn print_block(&mut self, block: &super::blocks::BasicBlock, func: &Function, module: &Module) {
    self.line(&format!("{}:", block.name));
    self.indent += 1;

    for inst in &block.instructions {
      self.print_instruction(inst, func, module);
    }

    self.indent -= 1;
  }

  fn print_instruction(&mut self, inst: &Instruction, func: &Function, module: &Module) {
    let s = self.format_instruction(inst, func, module);

    // Add variable name comment if available
    let comment = self.get_instruction_comment(inst, func);
    let line = if let Some(c) = comment {
      format!("{} ; {}", s, c)
    } else {
      s
    };
    self.line(&line);
  }

  /// Get a debug comment for an instruction showing variable names.
  fn get_instruction_comment(&self, inst: &Instruction, func: &Function) -> Option<String> {
    match inst {
      Instruction::Alloca { dst, .. } => {
        if let Some(info) = func.values.get(dst) {
          info.name.as_ref().map(|n| format!("var: {}", n))
        } else {
          None
        }
      }
      _ => None,
    }
  }

  fn format_instruction(&self, inst: &Instruction, func: &Function, module: &Module) -> String {
    match inst {
      // Arithmetic
      Instruction::Add { dst, lhs, rhs } => {
        format!("{} = add {}, {}", dst, lhs, rhs)
      }
      Instruction::Sub { dst, lhs, rhs } => {
        format!("{} = sub {}, {}", dst, lhs, rhs)
      }
      Instruction::PtrDiff { dst, lhs, rhs } => {
        format!("{} = ptrdiff {}, {}", dst, lhs, rhs)
      }
      Instruction::Mul { dst, lhs, rhs } => {
        format!("{} = mul {}, {}", dst, lhs, rhs)
      }
      Instruction::Div { dst, lhs, rhs } => {
        format!("{} = div {}, {}", dst, lhs, rhs)
      }
      Instruction::Rem { dst, lhs, rhs } => {
        format!("{} = rem {}, {}", dst, lhs, rhs)
      }
      Instruction::Neg { dst, src } => {
        format!("{} = neg {}", dst, src)
      }
      Instruction::CheckedAdd {
        dst,
        lhs,
        rhs,
        ty,
        parts,
      } => {
        format!(
          "{} = checked_add {} {}, {}, {}",
          dst,
          ty,
          lhs,
          rhs,
          format_parts(parts)
        )
      }
      Instruction::CheckedSub {
        dst,
        lhs,
        rhs,
        ty,
        parts,
      } => {
        format!(
          "{} = checked_sub {} {}, {}, {}",
          dst,
          ty,
          lhs,
          rhs,
          format_parts(parts)
        )
      }
      Instruction::CheckedMul {
        dst,
        lhs,
        rhs,
        ty,
        parts,
      } => {
        format!(
          "{} = checked_mul {} {}, {}, {}",
          dst,
          ty,
          lhs,
          rhs,
          format_parts(parts)
        )
      }
      Instruction::CheckedNeg {
        dst,
        src,
        ty,
        parts,
      } => {
        format!(
          "{} = checked_neg {} {}, {}",
          dst,
          ty,
          src,
          format_parts(parts)
        )
      }
      Instruction::Pow { dst, base, exp } => {
        format!("{} = pow {}, {}", dst, base, exp)
      }
      Instruction::Cross { dst, lhs, rhs } => {
        format!("{} = cross {}, {}", dst, lhs, rhs)
      }

      Instruction::Dot { dst, lhs, rhs } => {
        format!("{} = dot {}, {}", dst, lhs, rhs)
      }

      // Bitwise
      Instruction::BitAnd { dst, lhs, rhs } => {
        format!("{} = bitand {}, {}", dst, lhs, rhs)
      }
      Instruction::BitOr { dst, lhs, rhs } => {
        format!("{} = bitor {}, {}", dst, lhs, rhs)
      }
      Instruction::BitXor { dst, lhs, rhs } => {
        format!("{} = bitxor {}, {}", dst, lhs, rhs)
      }
      Instruction::BitNot { dst, src } => {
        format!("{} = bitnot {}", dst, src)
      }
      Instruction::Shl { dst, lhs, rhs } => {
        format!("{} = shl {}, {}", dst, lhs, rhs)
      }
      Instruction::Shr { dst, lhs, rhs } => {
        format!("{} = shr {}, {}", dst, lhs, rhs)
      }
      Instruction::UShr { dst, lhs, rhs } => {
        format!("{} = ushr {}, {}", dst, lhs, rhs)
      }

      // Comparison
      Instruction::Eq { dst, lhs, rhs } => {
        format!("{} = eq {}, {}", dst, lhs, rhs)
      }
      Instruction::Ne { dst, lhs, rhs } => {
        format!("{} = ne {}, {}", dst, lhs, rhs)
      }
      Instruction::Lt { dst, lhs, rhs } => {
        format!("{} = lt {}, {}", dst, lhs, rhs)
      }
      Instruction::Le { dst, lhs, rhs } => {
        format!("{} = le {}, {}", dst, lhs, rhs)
      }
      Instruction::Gt { dst, lhs, rhs } => {
        format!("{} = gt {}, {}", dst, lhs, rhs)
      }
      Instruction::Ge { dst, lhs, rhs } => {
        format!("{} = ge {}, {}", dst, lhs, rhs)
      }

      // Logical
      Instruction::And { dst, lhs, rhs } => {
        format!("{} = and {}, {}", dst, lhs, rhs)
      }
      Instruction::Or { dst, lhs, rhs } => {
        format!("{} = or {}, {}", dst, lhs, rhs)
      }
      Instruction::Xor { dst, lhs, rhs } => {
        format!("{} = xor {}, {}", dst, lhs, rhs)
      }
      Instruction::Not { dst, src } => {
        format!("{} = not {}", dst, src)
      }

      // Memory
      Instruction::Alloca { dst, ty } => {
        format!("{} = alloca {}", dst, ty)
      }
      Instruction::Load { dst, ptr, ty } => {
        format!("{} = load {} {}", dst, ty, ptr)
      }
      Instruction::Store { val, ptr, ty } => {
        format!("store {}, {}, {}", val, ptr, ty)
      }
      Instruction::GetElementPtr { dst, base, indices } => {
        let idx_str: Vec<String> = indices.iter().map(|i| format!("{}", i)).collect();
        format!("{} = gep {}, {}", dst, base, idx_str.join(", "))
      }
      Instruction::GetFieldPtr {
        dst,
        base,
        struct_name,
        byte_offset,
      } => {
        format!(
          "{} = getfieldptr %{} {}, {}",
          dst, struct_name, base, byte_offset
        )
      }
      Instruction::PtrToInt { dst, src } => {
        format!("{} = ptrtoint {}", dst, src)
      }
      Instruction::IntToPtr { dst, src } => {
        format!("{} = inttoptr {}", dst, src)
      }
      Instruction::BoundsCheck {
        index,
        length,
        parts,
      } => {
        format!("boundscheck {}, {}, {}", index, length, format_parts(parts))
      }
      Instruction::ZeroCheck { operand, parts } => {
        format!("zerocheck {}, {}", operand, format_parts(parts))
      }
      Instruction::TestBegin { suite, case } => {
        format!("testbegin \"{}\" \"{}\"", suite, case)
      }
      Instruction::TestFail {
        file,
        line,
        column,
        expected,
        found,
      } => {
        let expected = match expected {
          Some(v) => v.to_string(),
          None => "-".to_string(),
        };
        let found = match found {
          Some(v) => v.to_string(),
          None => "-".to_string(),
        };
        format!(
          "testfail \"{}\" {} {} {} {}",
          file, line, column, expected, found
        )
      }
      Instruction::TestEnd => "testend".to_string(),

      // Control flow
      Instruction::Br { target } => {
        format!("br {}", self.format_block_id(*target, func))
      }
      Instruction::CondBr {
        cond,
        then_block,
        else_block,
      } => {
        format!(
          "condbr {}, {}, {}",
          cond,
          self.format_block_id(*then_block, func),
          self.format_block_id(*else_block, func)
        )
      }
      Instruction::Ret { val } => {
        format!("ret {}", val)
      }
      Instruction::RetVoid => "ret void".to_string(),

      // Calls
      Instruction::Call {
        dst,
        func: func_ref,
        args,
        source_file,
        source_line,
        source_col,
      } => {
        let args_str: Vec<String> = args.iter().map(|a| format!("{}", a)).collect();
        let func_name = self.format_func_ref(func_ref, module);
        let mut base = format!("{} = call {}({})", dst, func_name, args_str.join(", "));

        if let Some(source) = format_source_suffix(source_file, source_line, source_col) {
          base.push_str(&source);
        }
        if let Some(comment) = self.get_constructor_comment(func_ref, module) {
          base.push_str(&format!(" ; {}", comment));
        }
        base
      }
      Instruction::CallVoid {
        func: func_ref,
        args,
        source_file,
        source_line,
        source_col,
      } => {
        let args_str: Vec<String> = args.iter().map(|a| format!("{}", a)).collect();
        let func_name = self.format_func_ref(func_ref, module);
        let mut base = format!("call {}({})", func_name, args_str.join(", "));

        if let Some(source) = format_source_suffix(source_file, source_line, source_col) {
          base.push_str(&source);
        }
        if let Some(comment) = self.get_constructor_comment(func_ref, module) {
          base.push_str(&format!(" ; {}", comment));
        }
        base
      }

      // Type conversions
      Instruction::Trunc { dst, src, to_ty } => {
        format!("{} = trunc {} to {}", dst, src, to_ty)
      }
      Instruction::SExt { dst, src, to_ty } => {
        format!("{} = sext {} to {}", dst, src, to_ty)
      }
      Instruction::ZExt { dst, src, to_ty } => {
        format!("{} = zext {} to {}", dst, src, to_ty)
      }
      Instruction::FpToSi { dst, src, to_ty } => {
        format!("{} = fptosi {} to {}", dst, src, to_ty)
      }
      Instruction::FpToUi { dst, src, to_ty } => {
        format!("{} = fptoui {} to {}", dst, src, to_ty)
      }
      Instruction::SiToFp { dst, src, to_ty } => {
        format!("{} = sitofp {} to {}", dst, src, to_ty)
      }
      Instruction::UiToFp { dst, src, to_ty } => {
        format!("{} = uitofp {} to {}", dst, src, to_ty)
      }
      Instruction::FpTrunc { dst, src, to_ty } => {
        format!("{} = fptrunc {} to {}", dst, src, to_ty)
      }
      Instruction::FpExt { dst, src, to_ty } => {
        format!("{} = fpext {} to {}", dst, src, to_ty)
      }
      Instruction::Bitcast { dst, src, to_ty } => {
        format!("{} = bitcast {} to {}", dst, src, to_ty)
      }

      // Constants
      Instruction::ConstInt { dst, ty, val } => {
        format!("{} = const {} {}", dst, ty, val)
      }
      Instruction::ConstUint { dst, ty, val } => {
        format!("{} = const {} {}u", dst, ty, val)
      }
      Instruction::ConstFloat { dst, ty, val } => {
        format!("{} = const {} {}", dst, ty, val)
      }
      Instruction::ConstBool { dst, val } => {
        format!("{} = const bool {}", dst, val)
      }
      Instruction::ConstString { dst, val } => {
        format!("{} = const text \"{}\"", dst, val.escape_default())
      }
      Instruction::ConstNull { dst } => {
        format!("{} = const ptr null", dst)
      }

      // Global access
      Instruction::GlobalAddr { dst, global } => {
        let name = match module.global(*global) {
          Some(g) => g.name.as_str(),
          None => crate::ice!("GlobalAddr references unknown global id {}", global),
        };
        format!("{} = globaladdr @{}", dst, name)
      }

      // String ops
      Instruction::Concat { dst, lhs, rhs } => {
        format!("{} = concat {}, {}", dst, lhs, rhs)
      }
      Instruction::TextCopy { dst, src } => {
        format!("{} = textcopy {}", dst, src)
      }
      Instruction::DeepCopy { dst, src } => {
        format!("{} = deepcopy {}", dst, src)
      }

      // Struct ops
      Instruction::BuildStruct {
        dst,
        struct_name,
        fields,
      } => {
        let fields_str: Vec<String> = fields.iter().map(|f| format!("{}", f)).collect();
        format!(
          "{} = buildstruct %{} {{ {} }}",
          dst,
          struct_name,
          fields_str.join(", ")
        )
      }

      Instruction::BuildVec2 {
        dst,
        elements: [x, y],
      } => {
        format!("{} = buildvec2 {}, {}", dst, x, y)
      }
      Instruction::BuildVec3 {
        dst,
        elements: [x, y, z],
      } => {
        format!("{} = buildvec3 {}, {}, {}", dst, x, y, z)
      }
      Instruction::BuildVec4 {
        dst,
        elements: [x, y, z, w],
      } => {
        format!("{} = buildvec4 {}, {}, {}, {}", dst, x, y, z, w)
      }
      Instruction::ExtractVecElement {
        dst,
        src,
        index,
        inner_ty,
      } => {
        format!("{} = extractvecelem {}, {}, {}", dst, src, index, inner_ty)
      }
      Instruction::ExtractField {
        dst,
        src,
        struct_name,
        field_index,
      } => {
        format!(
          "{} = extractfield %{} {}, {}",
          dst, struct_name, src, field_index
        )
      }
      Instruction::InsertField {
        dst,
        src,
        struct_name,
        field_index,
        val,
      } => {
        format!(
          "{} = insertfield %{} {}, {}, {}",
          dst, struct_name, src, field_index, val
        )
      }
      Instruction::StructFieldPtr {
        dst,
        struct_ptr,
        struct_name,
        field_name,
      } => {
        format!(
          "{} = fieldptr %{} {}, {}",
          dst, struct_name, struct_ptr, field_name
        )
      }

      // Unit assertions
      Instruction::AssertUnit { val, expected } => {
        format!("assertunit {}, <{}>", val, expected)
      }

      // Optional ops
      Instruction::Some { dst, value } => {
        format!("{} = some {}", dst, value)
      }
      Instruction::None { dst } => {
        format!("{} = none", dst)
      }
      Instruction::UnwrapOptional {
        dst,
        src,
        struct_name,
        parts,
      } => {
        format!(
          "{} = unwrapoptional %{} {}, {}",
          dst,
          struct_name,
          src,
          format_parts(parts)
        )
      }

      // Error stack ops
      Instruction::PushError { message } => {
        format!("pusherror {}", message)
      }
      Instruction::PopError { dst } => {
        format!("{} = poperror", dst)
      }
      Instruction::ErrorCount { dst } => {
        format!("{} = errorcount", dst)
      }
      Instruction::DrainErrors {
        to_stderr,
        prefix,
        timestamp,
      } => {
        let target = if *to_stderr { "stderr" } else { "stdout" };
        let mut out = format!("drainerrors {}", target);
        if let Some(p) = prefix {
          out.push_str(&format!(" prefix \"{}\"", p.escape_default()));
        }
        if *timestamp {
          out.push_str(" ts");
        }
        out
      }
    }
  }

  fn format_block_id(&self, id: BlockId, func: &Function) -> String {
    match func.block(id) {
      Some(block) => format!("%{}", block.name),
      None => crate::ice!("BlockId {} not found in function '{}'", id, func.name),
    }
  }
}

/// Format an optional call source location as ` at "file" line col`.
fn format_source_suffix(
  source_file: &Option<String>,
  source_line: &Option<i64>,
  source_col: &Option<i64>,
) -> Option<String> {
  match (source_file, source_line, source_col) {
    (Some(file), Some(line), Some(col)) => Some(format!(
      " at \"{}\" {} {}",
      file.escape_default(),
      line,
      col
    )),
    _ => None,
  }
}

/// Format a trap message's render spec as a bracketed part list.
/// `Text` parts become quoted strings; `Value` parts become `%N` operands.
fn format_parts(parts: &[RenderPart]) -> String {
  let items: Vec<String> = parts
    .iter()
    .map(|part| match part {
      RenderPart::Text(s) => format!("\"{}\"", s.escape_default()),
      RenderPart::Value(id) => format!("{}", id),
    })
    .collect();
  format!("[{}]", items.join(", "))
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::ir::builder::IrBuilder;
  use crate::ir::function::Linkage;
  use crate::ir::types::IrType;

  #[test]
  fn test_print_simple_function() {
    let mut builder = IrBuilder::new("test");
    builder.add_stdlib_externs();
    builder.start_function("main", IrType::Void, Linkage::Export);
    let s = builder.const_string("Hello, World!");
    builder.call_void_named("puts", vec![s]);
    builder.ret_void();

    let module = builder.build();
    let output = print_module(&module);

    assert!(output.contains("func @main"));
    assert!(output.contains("const text"));
    assert!(output.contains("call @puts"));
    assert!(output.contains("ret void"));
  }
}
