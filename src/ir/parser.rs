//! IR Parser
//!
//! Parses the canonical line-oriented IR text format produced by
//! [`crate::ir::printer::print_module`] back into an in-memory [`Module`].
//!
//! The grammar is documented in `doc/ARCHITECTURE.md` §4.7 ("Stable
//! Serialisation Format").

use super::blocks::BasicBlock;
use super::function::{ExternFunc, Linkage, Parameter};
use super::instructions::{FuncRef, Instruction};
use super::module::{Constant, Module, StructDef};
use super::types::IrType;
use super::values::{BlockId, ValueId, ValueInfo};
use crate::types::NormalizedUnit;
use std::collections::HashMap;

type SourceLoc = (Option<String>, Option<i64>, Option<i64>);

/// A single token in one IR line.
#[derive(Debug, Clone, PartialEq)]
enum Tok {
  Ident(String),
  Str(String),
  Punct(char),
}

/// Parse an IR module from its canonical text representation.
pub fn parse_module(text: &str) -> Result<Module, String> {
  let lines: Vec<&str> = text.lines().collect();
  let mut idx = 0usize;

  let mut version_seen = false;
  let mut module_name = String::new();
  while idx < lines.len() {
    let line = lines[idx].trim();
    idx += 1;
    if line.is_empty() {
      continue;
    }
    if let Some(rest) = line.strip_prefix("; ir-version:") {
      let version = rest.trim();
      let major = version
        .split('.')
        .next()
        .ok_or_else(|| format!("invalid ir-version line: '{}'", line))?;
      let major: u32 = major
        .parse()
        .map_err(|_| format!("invalid ir-version major: '{}'", version))?;
      if major != crate::ir::IR_FORMAT_MAJOR {
        return Err(format!(
          "unsupported IR format version {}.x (expected {}.x)",
          major,
          crate::ir::IR_FORMAT_MAJOR
        ));
      }
      version_seen = true;
      continue;
    }
    if let Some(rest) = line.strip_prefix("; Module:") {
      module_name = rest.trim().to_string();
      break;
    }
    if line.starts_with(';') {
      continue;
    }
    return Err(format!("expected version/module header, found: '{}'", line));
  }

  if !version_seen {
    return Err("missing '; ir-version:' header".to_string());
  }

  let mut module = Module::new_empty(module_name);
  let func_names = collect_function_names(&lines[idx..]);

  let mut current_fn: Option<FnBuilder> = None;

  while idx < lines.len() {
    let line = lines[idx].trim();
    idx += 1;

    if line.is_empty() || line.starts_with(';') {
      continue;
    }

    if line == "}" {
      let builder = current_fn
        .take()
        .ok_or_else(|| "unexpected '}' outside a function".to_string())?;
      finalize_function(&mut module, builder)?;
      continue;
    }

    if let Some(builder) = &mut current_fn {
      if is_block_header(line) {
        begin_block(builder, line)?;
      } else {
        let inst = parse_instruction_line(line, &module, &func_names, builder)?;
        builder.current_block_mut()?.instructions.push(inst);
      }
      continue;
    }

    let tokens = tokenize(line)?;
    if tokens.is_empty() {
      continue;
    }

    if is_function_header(&tokens) {
      let block_names = collect_block_names(&lines[idx..]);
      current_fn = Some(start_function(&mut module, &tokens, &block_names)?);
    } else if looks_like_struct(&tokens) {
      parse_struct(&mut module, &tokens)?;
    } else if looks_like_extern(&tokens) {
      parse_extern(&mut module, &tokens)?;
    } else {
      parse_global(&mut module, &tokens)?;
    }
  }

  if let Some(builder) = current_fn {
    return Err(format!("unterminated function '{}'", builder.name));
  }

  Ok(module)
}

// ---------------------------------------------------------------------------
// Function-name pre-pass
// ---------------------------------------------------------------------------

fn collect_block_names(lines: &[&str]) -> Vec<String> {
  let mut names = Vec::new();
  for line in lines {
    let line = line.trim();
    if line == "}" {
      break;
    }
    if is_block_header(line)
      && let Some(name) = line.strip_suffix(':')
    {
      names.push(name.trim().to_string());
    }
  }
  names
}

fn collect_function_names(lines: &[&str]) -> HashMap<String, super::values::FuncId> {
  let mut names = HashMap::new();
  let mut next_id = 0u32;
  for line in lines {
    let line = line.trim();
    if line.contains("func @")
      && line.trim_end().ends_with('{')
      && let Some(rest) = line.split("func @").nth(1)
      && let Some(name) = rest.split('(').next()
    {
      let name = name.trim();
      if !name.is_empty() && !names.contains_key(name) {
        names.insert(name.to_string(), super::values::FuncId::new(next_id));
        next_id += 1;
      }
    }
  }
  names
}

// ---------------------------------------------------------------------------
// Tokenizer
// ---------------------------------------------------------------------------

fn tokenize(line: &str) -> Result<Vec<Tok>, String> {
  let mut tokens = Vec::new();
  let chars: Vec<char> = line.chars().collect();
  let mut i = 0usize;

  while i < chars.len() {
    let c = chars[i];
    if c.is_whitespace() {
      i += 1;
      continue;
    }
    if c == ';' {
      break;
    }
    if c == '"' {
      let (s, next) = parse_string(&chars, i)?;
      tokens.push(Tok::Str(s));
      i = next;
      continue;
    }
    if c == '-' && chars.get(i + 1) == Some(&'>') {
      tokens.push(Tok::Ident("->".to_string()));
      i += 2;
      continue;
    }
    if matches!(
      c,
      '=' | ':' | ',' | '(' | ')' | '{' | '}' | '[' | ']' | '<' | '>'
    ) {
      tokens.push(Tok::Punct(c));
      i += 1;
      continue;
    }

    let start = i;
    while i < chars.len()
      && !chars[i].is_whitespace()
      && !matches!(
        chars[i],
        '=' | ':' | ',' | '(' | ')' | '{' | '}' | '[' | ']' | '<' | '>'
      )
      && chars[i] != ';'
      && chars[i] != '"'
    {
      i += 1;
    }
    let s: String = chars[start..i].iter().collect();
    if s.is_empty() {
      return Err(format!("unexpected character '{}' in line: '{}'", c, line));
    }
    tokens.push(Tok::Ident(s));
  }

  Ok(tokens)
}

fn parse_string(chars: &[char], start: usize) -> Result<(String, usize), String> {
  let mut out = String::new();
  let mut i = start + 1;
  while i < chars.len() {
    let c = chars[i];
    if c == '"' {
      return Ok((out, i + 1));
    }
    if c == '\\' {
      if i + 1 >= chars.len() {
        return Err("unterminated escape in string".to_string());
      }
      let next = chars[i + 1];
      match next {
        'n' => out.push('\n'),
        't' => out.push('\t'),
        'r' => out.push('\r'),
        '\\' => out.push('\\'),
        '"' => out.push('"'),
        'u' => {
          if i + 2 >= chars.len() || chars[i + 2] != '{' {
            return Err("invalid unicode escape".to_string());
          }
          let close = chars[i + 3..]
            .iter()
            .position(|&x| x == '}')
            .ok_or_else(|| "unterminated unicode escape".to_string())?;
          let hex: String = chars[i + 3..i + 3 + close].iter().collect();
          let code = u32::from_str_radix(&hex, 16)
            .map_err(|_| format!("invalid unicode escape '{}'", hex))?;
          let ch =
            char::from_u32(code).ok_or_else(|| format!("invalid unicode scalar '{}'", hex))?;
          out.push(ch);
          i += 3 + close + 1;
          continue;
        }
        other => {
          out.push('\\');
          out.push(other);
          i += 2;
          continue;
        }
      }
      i += 2;
      continue;
    }
    out.push(c);
    i += 1;
  }
  Err("unterminated string".to_string())
}

// ---------------------------------------------------------------------------
// Cursor helpers
// ---------------------------------------------------------------------------

struct Cursor<'a> {
  tokens: &'a [Tok],
  pos: usize,
}

impl<'a> Cursor<'a> {
  fn new(tokens: &'a [Tok]) -> Self {
    Cursor { tokens, pos: 0 }
  }

  fn peek(&self) -> Option<&Tok> {
    self.tokens.get(self.pos)
  }

  fn next(&mut self) -> Option<&Tok> {
    let t = self.tokens.get(self.pos);
    if t.is_some() {
      self.pos += 1;
    }
    t
  }

  fn expect_punct(&mut self, c: char) -> Result<(), String> {
    match self.next() {
      Some(Tok::Punct(actual)) if *actual == c => Ok(()),
      other => Err(format!("expected '{}', found {:?}", c, other)),
    }
  }

  fn expect_ident(&mut self) -> Result<String, String> {
    match self.next() {
      Some(Tok::Ident(s)) => Ok(s.clone()),
      other => Err(format!("expected identifier, found {:?}", other)),
    }
  }

  fn expect_str(&mut self) -> Result<String, String> {
    match self.next() {
      Some(Tok::Str(s)) => Ok(s.clone()),
      other => Err(format!("expected string, found {:?}", other)),
    }
  }

  fn at_end(&self) -> bool {
    self.pos >= self.tokens.len()
  }
}

// ---------------------------------------------------------------------------
// Type parsing
// ---------------------------------------------------------------------------

fn parse_type(cursor: &mut Cursor) -> Result<IrType, String> {
  let tok = cursor
    .next()
    .ok_or_else(|| "expected type, found end of line".to_string())?;
  let ident = match tok {
    Tok::Ident(s) => s.clone(),
    Tok::Punct('[') => {
      let size = parse_u64_ident(&cursor.expect_ident()?)?;
      let x = cursor.expect_ident()?;
      if x != "x" {
        return Err(format!("expected 'x' in array type, found '{}'", x));
      }
      let element = parse_type(cursor)?;
      cursor.expect_punct(']')?;
      return Ok(IrType::array(element, size));
    }
    other => return Err(format!("expected type, found {:?}", other)),
  };

  match ident.as_str() {
    "void" => Ok(IrType::Void),
    "bool" => Ok(IrType::Bool),
    "i8" => Ok(IrType::I8),
    "i16" => Ok(IrType::I16),
    "i32" => Ok(IrType::I32),
    "i64" => Ok(IrType::I64),
    "u8" => Ok(IrType::U8),
    "u16" => Ok(IrType::U16),
    "u32" => Ok(IrType::U32),
    "u64" => Ok(IrType::U64),
    "f16" => Ok(IrType::F16),
    "f32" => Ok(IrType::F32),
    "f64" => Ok(IrType::F64),
    "char" => Ok(IrType::Char),
    "ptr" => {
      if matches!(cursor.peek(), Some(Tok::Punct('<'))) {
        cursor.next();
        let inner = parse_type(cursor)?;
        cursor.expect_punct('>')?;
        Ok(IrType::ptr(inner))
      } else {
        Ok(IrType::raw_ptr())
      }
    }
    "vec2" | "vec3" | "vec4" => {
      cursor.expect_punct('<')?;
      let inner = parse_type(cursor)?;
      cursor.expect_punct('>')?;
      match ident.as_str() {
        "vec2" => Ok(IrType::vec2(inner)),
        "vec3" => Ok(IrType::vec3(inner)),
        "vec4" => Ok(IrType::vec4(inner)),
        _ => unreachable!(),
      }
    }
    other => {
      if let Some(base) = other.strip_suffix('?') {
        return parse_type_from_ident(base).map(IrType::optional);
      }
      if let Some(name) = other.strip_prefix('%') {
        return Ok(IrType::struct_ref(name.to_string()));
      }
      Err(format!("unknown type '{}'", other))
    }
  }
}

fn parse_type_from_ident(ident: &str) -> Result<IrType, String> {
  let tokens = vec![Tok::Ident(ident.to_string())];
  let mut cursor = Cursor::new(&tokens);
  parse_type(&mut cursor)
}

fn parse_u64_ident(ident: &str) -> Result<u64, String> {
  ident
    .parse()
    .map_err(|_| format!("expected unsigned integer, found '{}'", ident))
}

// ---------------------------------------------------------------------------
// Top-level declarations
// ---------------------------------------------------------------------------

fn looks_like_struct(tokens: &[Tok]) -> bool {
  matches!(tokens.first(), Some(Tok::Ident(s)) if s.starts_with('%'))
    && tokens
      .iter()
      .any(|t| matches!(t, Tok::Ident(s) if s == "type"))
}

fn looks_like_extern(tokens: &[Tok]) -> bool {
  matches!(tokens.first(), Some(Tok::Ident(s)) if s == "declare")
}

fn is_function_header(tokens: &[Tok]) -> bool {
  tokens
    .iter()
    .any(|t| matches!(t, Tok::Ident(s) if s == "func"))
    && tokens.last() == Some(&Tok::Punct('{'))
}

fn is_block_header(line: &str) -> bool {
  line.ends_with(':') && !line.contains('=')
}

fn parse_struct(module: &mut Module, tokens: &[Tok]) -> Result<(), String> {
  let mut cursor = Cursor::new(tokens);
  let name = cursor.expect_ident()?;
  let name = name
    .strip_prefix('%')
    .ok_or_else(|| format!("expected struct name, found '{}'", name))?
    .to_string();
  cursor.expect_punct('=')?;
  let kind = cursor.expect_ident()?;
  if kind != "type" {
    return Err(format!("expected 'type', found '{}'", kind));
  }
  cursor.expect_punct('{')?;

  let mut def = StructDef::new(name);
  while !matches!(cursor.peek(), Some(Tok::Punct('}'))) {
    let field_name = cursor.expect_ident()?;
    cursor.expect_punct(':')?;
    let ty = parse_type(&mut cursor)?;
    def.add_field(field_name, ty);
    if matches!(cursor.peek(), Some(Tok::Punct(','))) {
      cursor.next();
    }
  }
  cursor.expect_punct('}')?;
  module.add_struct(def);
  Ok(())
}

fn parse_extern(module: &mut Module, tokens: &[Tok]) -> Result<(), String> {
  let mut cursor = Cursor::new(tokens);
  cursor.expect_ident()?; // "declare"
  let name = cursor.expect_ident()?;
  let name = name
    .strip_prefix('@')
    .ok_or_else(|| format!("expected extern name, found '{}'", name))?
    .to_string();
  cursor.expect_punct('(')?;

  let mut params = Vec::new();
  let mut variadic = false;
  while !matches!(cursor.peek(), Some(Tok::Punct(')'))) {
    if matches!(cursor.peek(), Some(Tok::Ident(s)) if s == "...") {
      cursor.next();
      variadic = true;
      break;
    }
    params.push(parse_type(&mut cursor)?);
    if matches!(cursor.peek(), Some(Tok::Punct(','))) {
      cursor.next();
    } else {
      break;
    }
  }
  cursor.expect_punct(')')?;
  let arrow = cursor.expect_ident()?;
  if arrow != "->" {
    return Err(format!("expected '->', found '{}'", arrow));
  }
  let ret = parse_type(&mut cursor)?;

  module.add_extern_func(ExternFunc {
    name,
    params,
    return_type: ret,
    variadic,
  });
  Ok(())
}

fn parse_global(module: &mut Module, tokens: &[Tok]) -> Result<(), String> {
  let mut cursor = Cursor::new(tokens);
  let mut linkage = Linkage::Internal;

  if let Some(Tok::Ident(first)) = cursor.peek() {
    match first.as_str() {
      "export" => {
        linkage = Linkage::Export;
        cursor.next();
      }
      "import" => {
        linkage = Linkage::Import;
        cursor.next();
      }
      _ => {}
    }
  }

  let name = cursor.expect_ident()?;
  let name = name
    .strip_prefix('@')
    .ok_or_else(|| format!("expected global name, found '{}'", name))?
    .to_string();
  cursor.expect_punct(':')?;
  let ty = parse_type(&mut cursor)?;

  let mut init = None;
  if matches!(cursor.peek(), Some(Tok::Punct('='))) {
    cursor.next();
    init = Some(parse_constant(&mut cursor)?);
  }

  let id = module.add_global(name, ty, linkage);
  if let Some(init) = init
    && let Some(global) = module.global_mut(id)
  {
    global.initializer = Some(init);
  }
  Ok(())
}

// ---------------------------------------------------------------------------
// Function parsing
// ---------------------------------------------------------------------------

struct FnBuilder {
  func_id: super::values::FuncId,
  name: String,
  return_type: IrType,
  return_unit: Option<NormalizedUnit>,
  linkage: Linkage,
  params: Vec<Parameter>,
  blocks: Vec<BasicBlock>,
  block_map: HashMap<String, BlockId>,
  values: HashMap<ValueId, ValueInfo>,
}

impl FnBuilder {
  fn current_block_mut(&mut self) -> Result<&mut BasicBlock, String> {
    self
      .blocks
      .last_mut()
      .ok_or_else(|| "no current block".to_string())
  }
}

fn start_function(
  module: &mut Module,
  tokens: &[Tok],
  block_names: &[String],
) -> Result<FnBuilder, String> {
  let mut cursor = Cursor::new(tokens);
  let mut linkage = Linkage::Internal;
  if let Some(Tok::Ident(first)) = cursor.peek() {
    match first.as_str() {
      "export" => {
        linkage = Linkage::Export;
        cursor.next();
      }
      "import" => {
        linkage = Linkage::Import;
        cursor.next();
      }
      _ => {}
    }
  }

  let kw = cursor.expect_ident()?;
  if kw != "func" {
    return Err(format!("expected 'func', found '{}'", kw));
  }
  let name = cursor.expect_ident()?;
  let name = name
    .strip_prefix('@')
    .ok_or_else(|| format!("expected function name, found '{}'", name))?
    .to_string();

  cursor.expect_punct('(')?;
  let mut params = Vec::new();
  while !matches!(cursor.peek(), Some(Tok::Punct(')'))) {
    let vid = parse_value_id(&cursor.expect_ident()?)?;
    let pname = cursor.expect_ident()?;
    cursor.expect_punct(':')?;
    let ty = parse_type(&mut cursor)?;
    let unit = parse_optional_unit(&mut cursor)?;
    params.push(Parameter {
      name: pname.clone(),
      ty: ty.clone(),
      unit: unit.clone(),
      value_id: vid,
    });
    if matches!(cursor.peek(), Some(Tok::Punct(','))) {
      cursor.next();
    }
  }
  cursor.expect_punct(')')?;
  let arrow = cursor.expect_ident()?;
  if arrow != "->" {
    return Err(format!("expected '->', found '{}'", arrow));
  }
  let return_type = parse_type(&mut cursor)?;
  let return_unit = parse_optional_unit(&mut cursor)?;
  cursor.expect_punct('{')?;

  let func_id = module.add_function(name.clone(), return_type.clone(), linkage);

  let mut values = HashMap::new();
  for p in &params {
    values.insert(
      p.value_id,
      ValueInfo {
        ty: p.ty.clone(),
        unit: p.unit.clone(),
        name: Some(p.name.clone()),
      },
    );
  }

  let mut block_map = HashMap::new();
  for (i, block_name) in block_names.iter().enumerate() {
    block_map.insert(block_name.clone(), BlockId::new(i as u32));
  }

  Ok(FnBuilder {
    func_id,
    name,
    return_type,
    return_unit,
    linkage,
    params,
    blocks: Vec::new(),
    block_map,
    values,
  })
}

fn parse_optional_unit(cursor: &mut Cursor) -> Result<Option<NormalizedUnit>, String> {
  if matches!(cursor.peek(), Some(Tok::Punct('<'))) {
    cursor.next();
    let unit = cursor.expect_ident()?;
    cursor.expect_punct('>')?;
    Ok(Some(NormalizedUnit::parse(&unit)))
  } else {
    Ok(None)
  }
}

fn begin_block(builder: &mut FnBuilder, line: &str) -> Result<(), String> {
  let name = line
    .strip_suffix(':')
    .ok_or_else(|| format!("expected block header, found '{}'", line))?
    .trim()
    .to_string();
  let id = *builder
    .block_map
    .get(&name)
    .ok_or_else(|| format!("block '{}' not declared in function pre-scan", name))?;
  builder.blocks.push(BasicBlock::new(id, name));
  Ok(())
}

fn finalize_function(module: &mut Module, builder: FnBuilder) -> Result<(), String> {
  let func = module
    .function_mut(builder.func_id)
    .ok_or_else(|| format!("function id {} not found", builder.func_id))?;
  func.params = builder.params;
  func.blocks = builder.blocks;
  func.values = builder.values;
  func.return_unit = builder.return_unit;
  func.return_type = builder.return_type;
  func.linkage = builder.linkage;
  func.entry_block = BlockId::new(0);
  Ok(())
}

// ---------------------------------------------------------------------------
// Instruction parsing
// ---------------------------------------------------------------------------

fn parse_instruction_line(
  line: &str,
  module: &Module,
  func_names: &HashMap<String, super::values::FuncId>,
  builder: &FnBuilder,
) -> Result<Instruction, String> {
  let tokens = tokenize(line)?;
  let mut cursor = Cursor::new(&tokens);
  parse_instruction(&mut cursor, module, func_names, builder)
}

fn parse_instruction(
  cursor: &mut Cursor,
  module: &Module,
  func_names: &HashMap<String, super::values::FuncId>,
  builder: &FnBuilder,
) -> Result<Instruction, String> {
  let tokens = cursor.tokens;
  if tokens.len() >= 2 && matches!(tokens[1], Tok::Punct('=')) {
    let dst = match &tokens[0] {
      Tok::Ident(s) => parse_value_id(s)?,
      other => return Err(format!("expected value id, found {:?}", other)),
    };
    cursor.pos = 2;
    let op = cursor.expect_ident()?;
    parse_value_instruction(&op, dst, cursor, module, func_names, builder)
  } else {
    let op = cursor.expect_ident()?;
    parse_effect_instruction(&op, cursor, module, func_names, builder)
  }
}

fn parse_value_instruction(
  op: &str,
  dst: ValueId,
  cursor: &mut Cursor,
  module: &Module,
  func_names: &HashMap<String, super::values::FuncId>,
  _builder: &FnBuilder,
) -> Result<Instruction, String> {
  match op {
    "add" | "sub" | "mul" | "div" | "rem" | "ptrdiff" | "pow" | "cross" | "dot" | "bitand"
    | "bitor" | "bitxor" | "shl" | "shr" | "ushr" | "eq" | "ne" | "lt" | "le" | "gt" | "ge"
    | "and" | "or" | "xor" => {
      let lhs = parse_value_id(&cursor.expect_ident()?)?;
      cursor.expect_punct(',')?;
      let rhs = parse_value_id(&cursor.expect_ident()?)?;
      Ok(binary_instruction(op, dst, lhs, rhs))
    }
    "neg" | "bitnot" | "not" => {
      let src = parse_value_id(&cursor.expect_ident()?)?;
      Ok(unary_instruction(op, dst, src))
    }
    "checked_add" | "checked_sub" | "checked_mul" | "checked_neg" => {
      let ty = parse_type(cursor)?;
      if op == "checked_neg" {
        let src = parse_value_id(&cursor.expect_ident()?)?;
        let (file, line, column) = parse_required_source(cursor)?;
        Ok(Instruction::CheckedNeg {
          dst,
          src,
          ty,
          file,
          line,
          column,
        })
      } else {
        let lhs = parse_value_id(&cursor.expect_ident()?)?;
        cursor.expect_punct(',')?;
        let rhs = parse_value_id(&cursor.expect_ident()?)?;
        let (file, line, column) = parse_required_source(cursor)?;
        match op {
          "checked_add" => Ok(Instruction::CheckedAdd {
            dst,
            lhs,
            rhs,
            ty,
            file,
            line,
            column,
          }),
          "checked_sub" => Ok(Instruction::CheckedSub {
            dst,
            lhs,
            rhs,
            ty,
            file,
            line,
            column,
          }),
          "checked_mul" => Ok(Instruction::CheckedMul {
            dst,
            lhs,
            rhs,
            ty,
            file,
            line,
            column,
          }),
          _ => unreachable!(),
        }
      }
    }
    "alloca" => {
      let ty = parse_type(cursor)?;
      Ok(Instruction::Alloca { dst, ty })
    }
    "load" => {
      let ty = parse_type(cursor)?;
      let ptr = parse_value_id(&cursor.expect_ident()?)?;
      Ok(Instruction::Load { dst, ptr, ty })
    }
    "gep" => {
      let base = parse_value_id(&cursor.expect_ident()?)?;
      cursor.expect_punct(',')?;
      let mut indices = Vec::new();
      while !cursor.at_end() {
        indices.push(parse_value_id(&cursor.expect_ident()?)?);
        if matches!(cursor.peek(), Some(Tok::Punct(','))) {
          cursor.next();
        }
      }
      Ok(Instruction::GetElementPtr { dst, base, indices })
    }
    "getfieldptr" => {
      let struct_name = cursor.expect_ident()?.trim_start_matches('%').to_string();
      let base = parse_value_id(&cursor.expect_ident()?)?;
      cursor.expect_punct(',')?;
      let byte_offset = parse_u64_ident(&cursor.expect_ident()?)?;
      Ok(Instruction::GetFieldPtr {
        dst,
        base,
        struct_name,
        byte_offset,
      })
    }
    "ptrtoint" => {
      let src = parse_value_id(&cursor.expect_ident()?)?;
      Ok(Instruction::PtrToInt { dst, src })
    }
    "inttoptr" => {
      let src = parse_value_id(&cursor.expect_ident()?)?;
      Ok(Instruction::IntToPtr { dst, src })
    }
    "globaladdr" => {
      let name = cursor.expect_ident()?.trim_start_matches('@').to_string();
      let global = module
        .global_id(&name)
        .ok_or_else(|| format!("global '{}' not found", name))?;
      Ok(Instruction::GlobalAddr { dst, global })
    }
    "concat" => {
      let lhs = parse_value_id(&cursor.expect_ident()?)?;
      cursor.expect_punct(',')?;
      let rhs = parse_value_id(&cursor.expect_ident()?)?;
      Ok(Instruction::Concat { dst, lhs, rhs })
    }
    "strcopy" => {
      let src = parse_value_id(&cursor.expect_ident()?)?;
      Ok(Instruction::StrCopy { dst, src })
    }
    "buildstruct" => {
      let struct_name = cursor.expect_ident()?.trim_start_matches('%').to_string();
      cursor.expect_punct('{')?;
      let mut fields = Vec::new();
      while !matches!(cursor.peek(), Some(Tok::Punct('}'))) {
        fields.push(parse_value_id(&cursor.expect_ident()?)?);
        if matches!(cursor.peek(), Some(Tok::Punct(','))) {
          cursor.next();
        }
      }
      cursor.expect_punct('}')?;
      Ok(Instruction::BuildStruct {
        dst,
        struct_name,
        fields,
      })
    }
    "buildvec2" | "buildvec3" | "buildvec4" => {
      let mut elems = Vec::new();
      while !cursor.at_end() {
        elems.push(parse_value_id(&cursor.expect_ident()?)?);
        if matches!(cursor.peek(), Some(Tok::Punct(','))) {
          cursor.next();
        }
      }
      match op {
        "buildvec2" => Ok(Instruction::BuildVec2 {
          dst,
          elements: [elems[0], elems[1]],
        }),
        "buildvec3" => Ok(Instruction::BuildVec3 {
          dst,
          elements: [elems[0], elems[1], elems[2]],
        }),
        "buildvec4" => Ok(Instruction::BuildVec4 {
          dst,
          elements: [elems[0], elems[1], elems[2], elems[3]],
        }),
        _ => unreachable!(),
      }
    }
    "extractvecelem" => {
      let src = parse_value_id(&cursor.expect_ident()?)?;
      cursor.expect_punct(',')?;
      let index = parse_u32_ident(&cursor.expect_ident()?)?;
      cursor.expect_punct(',')?;
      let inner_ty = parse_type(cursor)?;
      Ok(Instruction::ExtractVecElement {
        dst,
        src,
        index,
        inner_ty,
      })
    }
    "extractfield" => {
      let struct_name = cursor.expect_ident()?.trim_start_matches('%').to_string();
      let src = parse_value_id(&cursor.expect_ident()?)?;
      cursor.expect_punct(',')?;
      let field_index = parse_u32_ident(&cursor.expect_ident()?)?;
      Ok(Instruction::ExtractField {
        dst,
        src,
        struct_name,
        field_index,
      })
    }
    "insertfield" => {
      let struct_name = cursor.expect_ident()?.trim_start_matches('%').to_string();
      let src = parse_value_id(&cursor.expect_ident()?)?;
      cursor.expect_punct(',')?;
      let field_index = parse_u32_ident(&cursor.expect_ident()?)?;
      cursor.expect_punct(',')?;
      let val = parse_value_id(&cursor.expect_ident()?)?;
      Ok(Instruction::InsertField {
        dst,
        src,
        struct_name,
        field_index,
        val,
      })
    }
    "fieldptr" => {
      let struct_name = cursor.expect_ident()?.trim_start_matches('%').to_string();
      let struct_ptr = parse_value_id(&cursor.expect_ident()?)?;
      cursor.expect_punct(',')?;
      let field_name = cursor.expect_ident()?;
      Ok(Instruction::StructFieldPtr {
        dst,
        struct_ptr,
        struct_name,
        field_name,
      })
    }
    "some" => {
      let value = parse_value_id(&cursor.expect_ident()?)?;
      Ok(Instruction::Some { dst, value })
    }
    "none" => Ok(Instruction::None { dst }),
    "unwrapoptional" => {
      let struct_name = cursor.expect_ident()?.trim_start_matches('%').to_string();
      let src = parse_value_id(&cursor.expect_ident()?)?;
      cursor.expect_punct(',')?;
      let message = cursor.expect_str()?;
      let (file, line, col) = parse_required_source(cursor)?;
      Ok(Instruction::UnwrapOptional {
        dst,
        src,
        struct_name,
        message,
        file,
        line,
        col,
      })
    }
    "poperror" => Ok(Instruction::PopError { dst }),
    "errorcount" => Ok(Instruction::ErrorCount { dst }),
    "trunc" | "sext" | "zext" | "fptosi" | "fptoui" | "sitofp" | "uitofp" | "fptrunc" | "fpext"
    | "bitcast" => {
      let src = parse_value_id(&cursor.expect_ident()?)?;
      let to = cursor.expect_ident()?;
      if to != "to" {
        return Err(format!("expected 'to', found '{}'", to));
      }
      let to_ty = parse_type(cursor)?;
      Ok(conversion_instruction(op, dst, src, to_ty))
    }
    "call" => {
      let func = parse_func_ref(&cursor.expect_ident()?, module, func_names)?;
      cursor.expect_punct('(')?;
      let args = parse_arg_list(cursor)?;
      cursor.expect_punct(')')?;
      let (source_file, source_line, source_col) = parse_optional_source(cursor)?;
      Ok(Instruction::Call {
        dst,
        func,
        args,
        source_file,
        source_line,
        source_col,
      })
    }
    "const" => parse_const(dst, cursor),
    other => Err(format!("unknown value-producing instruction '{}'", other)),
  }
}

fn parse_effect_instruction(
  op: &str,
  cursor: &mut Cursor,
  module: &Module,
  func_names: &HashMap<String, super::values::FuncId>,
  builder: &FnBuilder,
) -> Result<Instruction, String> {
  match op {
    "store" => {
      let val = parse_value_id(&cursor.expect_ident()?)?;
      cursor.expect_punct(',')?;
      let ptr = parse_value_id(&cursor.expect_ident()?)?;
      cursor.expect_punct(',')?;
      let ty = parse_type(cursor)?;
      Ok(Instruction::Store { val, ptr, ty })
    }
    "boundscheck" => {
      let index = parse_value_id(&cursor.expect_ident()?)?;
      cursor.expect_punct(',')?;
      let length = parse_value_id(&cursor.expect_ident()?)?;
      cursor.expect_punct(',')?;
      let message = cursor.expect_str()?;
      cursor.expect_punct(',')?;
      let file = cursor.expect_str()?;
      cursor.expect_punct(',')?;
      let line = parse_i64_ident(&cursor.expect_ident()?)?;
      cursor.expect_punct(',')?;
      let column = parse_i64_ident(&cursor.expect_ident()?)?;
      Ok(Instruction::BoundsCheck {
        index,
        length,
        message,
        file,
        line,
        column,
      })
    }
    "zerocheck" => {
      let operand = parse_value_id(&cursor.expect_ident()?)?;
      cursor.expect_punct(',')?;
      let message = cursor.expect_str()?;
      cursor.expect_punct(',')?;
      let file = cursor.expect_str()?;
      cursor.expect_punct(',')?;
      let line = parse_i64_ident(&cursor.expect_ident()?)?;
      cursor.expect_punct(',')?;
      let column = parse_i64_ident(&cursor.expect_ident()?)?;
      Ok(Instruction::ZeroCheck {
        operand,
        message,
        file,
        line,
        column,
      })
    }
    "testbegin" => {
      let suite = cursor.expect_str()?;
      let case = cursor.expect_str()?;
      Ok(Instruction::TestBegin { suite, case })
    }
    "testfail" => {
      let file = cursor.expect_str()?;
      let line = parse_i64_ident(&cursor.expect_ident()?)?;
      let column = parse_i64_ident(&cursor.expect_ident()?)?;
      let expected = parse_opt_value_id(&cursor.expect_ident()?)?;
      let found = parse_opt_value_id(&cursor.expect_ident()?)?;
      Ok(Instruction::TestFail {
        file,
        line,
        column,
        expected,
        found,
      })
    }
    "testend" => Ok(Instruction::TestEnd),
    "br" => {
      let target = parse_block_ref(&cursor.expect_ident()?, builder)?;
      Ok(Instruction::Br { target })
    }
    "condbr" => {
      let cond = parse_value_id(&cursor.expect_ident()?)?;
      cursor.expect_punct(',')?;
      let then_block = parse_block_ref(&cursor.expect_ident()?, builder)?;
      cursor.expect_punct(',')?;
      let else_block = parse_block_ref(&cursor.expect_ident()?, builder)?;
      Ok(Instruction::CondBr {
        cond,
        then_block,
        else_block,
      })
    }
    "ret" => match cursor.peek() {
      Some(Tok::Ident(s)) if s == "void" => {
        cursor.next();
        Ok(Instruction::RetVoid)
      }
      _ => {
        let val = parse_value_id(&cursor.expect_ident()?)?;
        Ok(Instruction::Ret { val })
      }
    },
    "call" => {
      let func = parse_func_ref(&cursor.expect_ident()?, module, func_names)?;
      cursor.expect_punct('(')?;
      let args = parse_arg_list(cursor)?;
      cursor.expect_punct(')')?;
      let (source_file, source_line, source_col) = parse_optional_source(cursor)?;
      Ok(Instruction::CallVoid {
        func,
        args,
        source_file,
        source_line,
        source_col,
      })
    }
    "pusherror" => {
      let message = parse_value_id(&cursor.expect_ident()?)?;
      Ok(Instruction::PushError { message })
    }
    "drainerrors" => {
      let target = cursor.expect_ident()?;
      let to_stderr = target == "stderr";
      let mut prefix = None;
      if matches!(cursor.peek(), Some(Tok::Ident(s)) if s == "prefix") {
        cursor.next();
        prefix = Some(cursor.expect_str()?);
      }
      Ok(Instruction::DrainErrors { to_stderr, prefix })
    }
    "assertunit" => {
      let val = parse_value_id(&cursor.expect_ident()?)?;
      cursor.expect_punct(',')?;
      cursor.expect_punct('<')?;
      let unit = cursor.expect_ident()?;
      cursor.expect_punct('>')?;
      Ok(Instruction::AssertUnit {
        val,
        expected: NormalizedUnit::parse(&unit),
      })
    }
    other => Err(format!("unknown effect instruction '{}'", other)),
  }
}

// ---------------------------------------------------------------------------
// Instruction helpers
// ---------------------------------------------------------------------------

fn binary_instruction(op: &str, dst: ValueId, lhs: ValueId, rhs: ValueId) -> Instruction {
  match op {
    "add" => Instruction::Add { dst, lhs, rhs },
    "sub" => Instruction::Sub { dst, lhs, rhs },
    "mul" => Instruction::Mul { dst, lhs, rhs },
    "div" => Instruction::Div { dst, lhs, rhs },
    "rem" => Instruction::Rem { dst, lhs, rhs },
    "ptrdiff" => Instruction::PtrDiff { dst, lhs, rhs },
    "pow" => Instruction::Pow {
      dst,
      base: lhs,
      exp: rhs,
    },
    "cross" => Instruction::Cross { dst, lhs, rhs },
    "dot" => Instruction::Dot { dst, lhs, rhs },
    "bitand" => Instruction::BitAnd { dst, lhs, rhs },
    "bitor" => Instruction::BitOr { dst, lhs, rhs },
    "bitxor" => Instruction::BitXor { dst, lhs, rhs },
    "shl" => Instruction::Shl { dst, lhs, rhs },
    "shr" => Instruction::Shr { dst, lhs, rhs },
    "ushr" => Instruction::UShr { dst, lhs, rhs },
    "eq" => Instruction::Eq { dst, lhs, rhs },
    "ne" => Instruction::Ne { dst, lhs, rhs },
    "lt" => Instruction::Lt { dst, lhs, rhs },
    "le" => Instruction::Le { dst, lhs, rhs },
    "gt" => Instruction::Gt { dst, lhs, rhs },
    "ge" => Instruction::Ge { dst, lhs, rhs },
    "and" => Instruction::And { dst, lhs, rhs },
    "or" => Instruction::Or { dst, lhs, rhs },
    "xor" => Instruction::Xor { dst, lhs, rhs },
    _ => unreachable!("binary_instruction: {}", op),
  }
}

fn unary_instruction(op: &str, dst: ValueId, src: ValueId) -> Instruction {
  match op {
    "neg" => Instruction::Neg { dst, src },
    "bitnot" => Instruction::BitNot { dst, src },
    "not" => Instruction::Not { dst, src },
    _ => unreachable!("unary_instruction: {}", op),
  }
}

fn conversion_instruction(op: &str, dst: ValueId, src: ValueId, to_ty: IrType) -> Instruction {
  match op {
    "trunc" => Instruction::Trunc { dst, src, to_ty },
    "sext" => Instruction::SExt { dst, src, to_ty },
    "zext" => Instruction::ZExt { dst, src, to_ty },
    "fptosi" => Instruction::FpToSi { dst, src, to_ty },
    "fptoui" => Instruction::FpToUi { dst, src, to_ty },
    "sitofp" => Instruction::SiToFp { dst, src, to_ty },
    "uitofp" => Instruction::UiToFp { dst, src, to_ty },
    "fptrunc" => Instruction::FpTrunc { dst, src, to_ty },
    "fpext" => Instruction::FpExt { dst, src, to_ty },
    "bitcast" => Instruction::Bitcast { dst, src, to_ty },
    _ => unreachable!("conversion_instruction: {}", op),
  }
}

fn parse_const(dst: ValueId, cursor: &mut Cursor) -> Result<Instruction, String> {
  let ty = cursor.expect_ident()?;
  match ty.as_str() {
    "bool" => {
      let val = cursor.expect_ident()?;
      Ok(Instruction::ConstBool {
        dst,
        val: val == "true",
      })
    }
    "str" => {
      let val = cursor.expect_str()?;
      Ok(Instruction::ConstString { dst, val })
    }
    "ptr" => {
      if matches!(cursor.peek(), Some(Tok::Ident(s)) if s == "null") {
        cursor.next();
        Ok(Instruction::ConstNull { dst })
      } else {
        let mut ty = IrType::raw_ptr();
        if matches!(cursor.peek(), Some(Tok::Punct('<'))) {
          cursor.next();
          let inner = parse_type(cursor)?;
          cursor.expect_punct('>')?;
          ty = IrType::ptr(inner);
        }
        let val = cursor.expect_ident()?;
        make_const(dst, ty, &val)
      }
    }
    other => {
      let val = cursor.expect_ident()?;
      let ty = parse_type_from_ident(other)?;
      make_const(dst, ty, &val)
    }
  }
}

fn make_const(dst: ValueId, ty: IrType, val: &str) -> Result<Instruction, String> {
  if val.ends_with('u') {
    let v: u64 = val
      .trim_end_matches('u')
      .parse()
      .map_err(|_| format!("invalid uint constant '{}'", val))?;
    Ok(Instruction::ConstUint { dst, ty, val: v })
  } else if val.contains('.') || val.contains('e') || val.contains('E') {
    let v: f64 = val
      .parse()
      .map_err(|_| format!("invalid float constant '{}'", val))?;
    Ok(Instruction::ConstFloat { dst, ty, val: v })
  } else {
    let v: i64 = val
      .parse()
      .map_err(|_| format!("invalid int constant '{}'", val))?;
    Ok(Instruction::ConstInt { dst, ty, val: v })
  }
}

fn parse_arg_list(cursor: &mut Cursor) -> Result<Vec<ValueId>, String> {
  let mut args = Vec::new();
  while !matches!(cursor.peek(), Some(Tok::Punct(')'))) {
    args.push(parse_value_id(&cursor.expect_ident()?)?);
    if matches!(cursor.peek(), Some(Tok::Punct(','))) {
      cursor.next();
    }
  }
  Ok(args)
}

fn parse_optional_source(cursor: &mut Cursor) -> Result<SourceLoc, String> {
  if matches!(cursor.peek(), Some(Tok::Ident(s)) if s == "at") {
    cursor.next();
    let file = cursor.expect_str()?;
    let line = parse_i64_ident(&cursor.expect_ident()?)?;
    let col = parse_i64_ident(&cursor.expect_ident()?)?;
    Ok((Some(file), Some(line), Some(col)))
  } else {
    Ok((None, None, None))
  }
}

fn parse_required_source(cursor: &mut Cursor) -> Result<(String, i64, i64), String> {
  if !matches!(cursor.peek(), Some(Tok::Ident(s)) if s == "at") {
    return Err("expected ' at \"file\" line col'".to_string());
  }
  cursor.next();
  let file = cursor.expect_str()?;
  let line = parse_i64_ident(&cursor.expect_ident()?)?;
  let col = parse_i64_ident(&cursor.expect_ident()?)?;
  Ok((file, line, col))
}

fn parse_func_ref(
  tok: &str,
  module: &Module,
  func_names: &HashMap<String, super::values::FuncId>,
) -> Result<FuncRef, String> {
  let name = tok.trim_start_matches('@').to_string();
  if let Some(id) = func_names.get(&name) {
    Ok(FuncRef::Id(*id))
  } else if let Some(id) = module.func_id(&name) {
    Ok(FuncRef::Id(id))
  } else {
    Ok(FuncRef::External(name))
  }
}

fn parse_value_id(tok: &str) -> Result<ValueId, String> {
  let n = tok
    .trim_start_matches('%')
    .parse()
    .map_err(|_| format!("invalid value id '{}'", tok))?;
  Ok(ValueId::new(n))
}

/// Parse an optional value id: `-` means `None`, otherwise a `%N` reference.
fn parse_opt_value_id(tok: &str) -> Result<Option<ValueId>, String> {
  if tok == "-" {
    Ok(None)
  } else {
    parse_value_id(tok).map(Some)
  }
}

fn parse_block_ref(tok: &str, builder: &FnBuilder) -> Result<BlockId, String> {
  let name = tok.trim_start_matches('%');
  builder
    .block_map
    .get(name)
    .copied()
    .ok_or_else(|| format!("unknown block '{}'", name))
}

fn parse_i64_ident(s: &str) -> Result<i64, String> {
  s.parse().map_err(|_| format!("invalid integer '{}'", s))
}

fn parse_u32_ident(s: &str) -> Result<u32, String> {
  s.parse()
    .map_err(|_| format!("invalid unsigned integer '{}'", s))
}

// ---------------------------------------------------------------------------
// Constant parsing
// ---------------------------------------------------------------------------

fn parse_constant(cursor: &mut Cursor) -> Result<Constant, String> {
  match cursor.next() {
    Some(Tok::Ident(s)) => {
      if s == "null" {
        Ok(Constant::Null)
      } else if s == "zeroinit" {
        Ok(Constant::Zero)
      } else if let Some(v) = s.strip_suffix('u') {
        let v: u64 = v.parse().map_err(|_| format!("invalid uint '{}'", s))?;
        Ok(Constant::Uint(v))
      } else if let Ok(v) = s.parse::<i64>() {
        Ok(Constant::Int(v))
      } else if let Ok(v) = s.parse::<f64>() {
        Ok(Constant::Float(v))
      } else if s == "true" || s == "false" {
        Ok(Constant::Bool(s == "true"))
      } else {
        Err(format!("invalid constant '{}'", s))
      }
    }
    Some(Tok::Str(s)) => Ok(Constant::String(s.clone())),
    Some(Tok::Punct('[')) => {
      let mut elems = Vec::new();
      while !matches!(cursor.peek(), Some(Tok::Punct(']'))) {
        elems.push(parse_constant(cursor)?);
        if matches!(cursor.peek(), Some(Tok::Punct(','))) {
          cursor.next();
        }
      }
      cursor.expect_punct(']')?;
      Ok(Constant::Array(elems))
    }
    Some(Tok::Punct('{')) => {
      let mut fields = Vec::new();
      while !matches!(cursor.peek(), Some(Tok::Punct('}'))) {
        let name = cursor.expect_ident()?;
        cursor.expect_punct(':')?;
        let value = parse_constant(cursor)?;
        fields.push((name, value));
        if matches!(cursor.peek(), Some(Tok::Punct(','))) {
          cursor.next();
        }
      }
      cursor.expect_punct('}')?;
      Ok(Constant::Struct(fields))
    }
    other => Err(format!("expected constant, found {:?}", other)),
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::ir::builder::IrBuilder;
  use crate::ir::printer::print_module;

  #[test]
  fn round_trip_simple_module() {
    let mut builder = IrBuilder::new("test");
    builder.start_function("main", IrType::I64, Linkage::Export);
    let a = builder.const_int(IrType::I64, 1);
    let b = builder.const_int(IrType::I64, 2);
    let c = builder.add(a, b, IrType::I64);
    builder.ret(c);

    let module = builder.build();
    let text = print_module(&module);
    let parsed = parse_module(&text).expect("parse printed module");
    let text2 = print_module(&parsed);
    assert_eq!(text, text2);
  }
}
