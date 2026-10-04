//! IR Module
//!
//! The module is the top-level container for an IR program. It holds all
//! functions, global variables, type definitions, and external declarations.

use super::function::{ExternFunc, Function, Linkage};
use super::types::IrType;
use super::values::{FuncId, GlobalId};
use crate::error::{CompileResult, IrError};
use crate::types::NormalizedUnit;
use std::collections::HashMap;

/// A constant value for global variable initialization.
#[derive(Debug, Clone)]
pub enum Constant {
  /// Signed integer
  Int(i64),
  /// Unsigned integer
  Uint(u64),
  /// Floating point
  Float(f64),
  /// Boolean
  Bool(bool),
  /// String literal
  String(String),
  /// Null pointer
  Null,
  /// Array of constants
  Array(Vec<Constant>),
  /// Struct literal
  Struct(Vec<(String, Constant)>),
  /// Zero-initialized (all bytes zero)
  Zero,
}

/// A global variable.
#[derive(Debug, Clone)]
pub struct Global {
  /// Unique identifier
  pub id: GlobalId,
  /// Variable name
  pub name: String,
  /// Type
  pub ty: IrType,
  /// Optional physical unit
  pub unit: Option<NormalizedUnit>,
  /// Optional initializer
  pub initializer: Option<Constant>,
  /// Linkage
  pub linkage: Linkage,
}

impl Global {
  /// Create a new global variable.
  pub fn new(id: GlobalId, name: impl Into<String>, ty: IrType, linkage: Linkage) -> Self {
    Global {
      id,
      name: name.into(),
      ty,
      unit: None,
      initializer: None,
      linkage,
    }
  }

  /// Set the initializer.
  pub fn with_initializer(mut self, init: Constant) -> Self {
    self.initializer = Some(init);
    self
  }

  /// Set the physical unit.
  pub fn with_unit(mut self, unit: NormalizedUnit) -> Self {
    self.unit = Some(unit);
    self
  }
}

/// A struct type definition.
#[derive(Debug, Clone)]
pub struct StructDef {
  /// Struct name
  pub name: String,
  /// Fields: (name, type)
  pub fields: Vec<(String, IrType)>,
  /// Physical unit of each field (parallel to `fields`; `None` if unitless).
  pub field_units: Vec<Option<String>>,
}

impl StructDef {
  /// Create a new struct definition.
  pub fn new(name: impl Into<String>) -> Self {
    StructDef {
      name: name.into(),
      fields: Vec::new(),
      field_units: Vec::new(),
    }
  }

  /// Add a unitless field to this struct.
  pub fn add_field(&mut self, name: impl Into<String>, ty: IrType) {
    self.fields.push((name.into(), ty));
    self.field_units.push(None);
  }

  /// Get the index of a field by name.
  pub fn field_index(&self, name: &str) -> Option<usize> {
    self.fields.iter().position(|(n, _)| n == name)
  }

  /// Get the type of a field by name.
  pub fn field_type(&self, name: &str) -> Option<&IrType> {
    self.fields.iter().find(|(n, _)| n == name).map(|(_, t)| t)
  }
  /// Get the byte offset of a field by name.
  /// Returns `None` if the field is not found, or if any preceding field
  /// has an unknown size (e.g., nested structs requiring layout context).
  pub fn field_offset_by_name(&self, field_name: &str) -> Option<u64> {
    let mut offset = 0u64;
    for (name, field_type) in &self.fields {
      if name == field_name {
        return Some(offset);
      }
      offset += field_type.size_bytes()? as u64;
    }
    None
  }

  /// Get the byte offset of a field by index (0-based).
  /// Returns `None` if the index is out of bounds, or if any preceding field
  /// has an unknown size (e.g., nested structs requiring layout context).
  pub fn field_offset_by_index(&self, index: usize) -> Option<u64> {
    let mut offset = 0u64;
    for (i, (_, field_type)) in self.fields.iter().enumerate() {
      if i == index {
        return Some(offset);
      }
      offset += field_type.size_bytes()? as u64;
    }
    None
  }
}

/// The top-level IR module.
#[derive(Debug)]
pub struct Module {
  /// Module name
  pub name: String,
  /// Global variables
  pub globals: Vec<Global>,
  /// Function definitions
  pub functions: Vec<Function>,
  /// Struct type definitions
  pub structs: Vec<StructDef>,
  /// External function declarations
  pub extern_funcs: Vec<ExternFunc>,
  /// Function name to ID mapping
  func_map: HashMap<String, FuncId>,
  /// Global name to ID mapping
  global_map: HashMap<String, GlobalId>,
  /// Struct name to index mapping
  struct_map: HashMap<String, usize>,
  /// Next function ID
  next_func_id: u32,
  /// Next global ID
  next_global_id: u32,
}

impl Module {
  /// Create a new module with no pre-registered definitions.
  pub fn new_empty(name: impl Into<String>) -> Self {
    Module {
      name: name.into(),
      globals: Vec::new(),
      functions: Vec::new(),
      structs: Vec::new(),
      extern_funcs: Vec::new(),
      func_map: HashMap::new(),
      global_map: HashMap::new(),
      struct_map: HashMap::new(),
      next_func_id: 0,
      next_global_id: 0,
    }
  }

  /// Create a new module with no pre-registered definitions.
  ///
  /// `str` and any other builtin types are **not** registered here; they come
  /// from `builtins.lale` (loaded via `IrGenerator::load_builtins_into_module`),
  /// which is the single source of truth for the `str` layout.
  pub fn new(name: impl Into<String>) -> Self {
    Module::new_empty(name)
  }

  /// Add a function to this module.
  pub fn add_function(
    &mut self,
    name: impl Into<String>,
    return_type: IrType,
    linkage: Linkage,
  ) -> FuncId {
    let name = name.into();
    let id = FuncId::new(self.next_func_id);
    self.next_func_id += 1;

    let func = Function::new(id, name.clone(), return_type, linkage);
    self.func_map.insert(name, id);
    self.functions.push(func);
    id
  }

  /// Get a function by ID.
  pub fn function(&self, id: FuncId) -> Option<&Function> {
    self.functions.iter().find(|f| f.id == id)
  }

  /// Get a mutable function by ID.
  pub fn function_mut(&mut self, id: FuncId) -> Option<&mut Function> {
    self.functions.iter_mut().find(|f| f.id == id)
  }

  /// Get a function by ID, returning a Result for error propagation.
  pub fn try_function(&self, id: FuncId) -> CompileResult<&Function> {
    self.function(id).ok_or_else(|| {
      IrError::FunctionNotFound {
        id: format!("{:?}", id),
      }
      .into()
    })
  }

  /// Get a mutable function by ID, returning a Result for error propagation.
  pub fn try_function_mut(&mut self, id: FuncId) -> CompileResult<&mut Function> {
    self.function_mut(id).ok_or_else(|| {
      IrError::FunctionNotFound {
        id: format!("{:?}", id),
      }
      .into()
    })
  }

  /// Get a function by name.
  pub fn function_by_name(&self, name: &str) -> Option<&Function> {
    self.func_map.get(name).and_then(|id| self.function(*id))
  }

  /// Get a mutable function by name.
  pub fn function_by_name_mut(&mut self, name: &str) -> Option<&mut Function> {
    if let Some(&id) = self.func_map.get(name) {
      self.function_mut(id)
    } else {
      None
    }
  }

  /// Check if a function name is a struct constructor.
  pub fn is_struct_constructor(&self, func_name: &str) -> bool {
    self.struct_map.contains_key(func_name)
  }

  /// Add a global variable.
  pub fn add_global(&mut self, name: impl Into<String>, ty: IrType, linkage: Linkage) -> GlobalId {
    let name = name.into();
    let id = GlobalId::new(self.next_global_id);
    self.next_global_id += 1;

    let global = Global::new(id, name.clone(), ty, linkage);
    self.global_map.insert(name, id);
    self.globals.push(global);
    id
  }

  /// Get a global by ID.
  pub fn global(&self, id: GlobalId) -> Option<&Global> {
    self.globals.iter().find(|g| g.id == id)
  }

  /// Get a mutable global by ID.
  pub fn global_mut(&mut self, id: GlobalId) -> Option<&mut Global> {
    self.globals.iter_mut().find(|g| g.id == id)
  }

  /// Get a global by name.
  pub fn global_by_name(&self, name: &str) -> Option<&Global> {
    self.global_map.get(name).and_then(|id| self.global(*id))
  }

  /// Look up a global ID by name.
  pub fn global_id(&self, name: &str) -> Option<GlobalId> {
    self.global_map.get(name).copied()
  }

  /// Look up a function ID by name.
  pub fn func_id(&self, name: &str) -> Option<FuncId> {
    self.func_map.get(name).copied()
  }

  /// Get the next function ID counter value.
  pub fn get_next_func_id(&self) -> u32 {
    self.next_func_id
  }

  /// Set the next function ID counter value (used when merging modules).
  pub fn set_next_func_id(&mut self, id: u32) {
    self.next_func_id = id;
  }

  /// Increment and get the next function ID (returns current value, then increments).
  pub fn alloc_func_id(&mut self) -> FuncId {
    let id = FuncId::new(self.next_func_id);
    self.next_func_id += 1;
    id
  }

  /// Register a function in the func_map with a specific ID (used when merging modules).
  pub fn register_func(&mut self, name: String, id: FuncId) {
    self.func_map.insert(name, id);
  }

  /// Add a struct definition.
  pub fn add_struct(&mut self, def: StructDef) {
    let name = def.name.clone();
    let index = self.structs.len();
    self.struct_map.insert(name, index);
    self.structs.push(def);
  }

  /// Get a struct definition by name.
  pub fn struct_def(&self, name: &str) -> Option<&StructDef> {
    self.struct_map.get(name).map(|&i| &self.structs[i])
  }

  /// Add an external function declaration.
  pub fn add_extern_func(&mut self, func: ExternFunc) {
    self.extern_funcs.push(func);
  }

  /// Check if an external function exists.
  pub fn has_extern_func(&self, name: &str) -> bool {
    self.extern_funcs.iter().any(|f| f.name == name)
  }

  /// Get an external function by name.
  pub fn extern_func(&self, name: &str) -> Option<&ExternFunc> {
    self.extern_funcs.iter().find(|f| f.name == name)
  }

  /// Add standard library external functions (hooks).
  ///
  /// These are user-replaceable hooks that enable no-std support. In default mode,
  /// they are linked against libc. In no-std mode, the user must provide implementations.
  ///
  /// **I/O Hooks:**
  /// - `write(fd, ptr, count)` - POSIX write syscall (fd=1 stdout, fd=2 stderr)
  ///
  /// **Runtime Hooks:**
  /// - `write(fd, ptr, count)` - POSIX write syscall (fd=1 stdout, fd=2 stderr)
  /// - `__lale_exit(code)` - program termination
  /// - `pow(base, exp)` - floating-point exponentiation (libc pow)
  /// - `__lale_malloc_u64(size)` - dynamic memory allocation
  /// - `__lale_free_pointer(ptr)` - dynamic memory deallocation
  ///
  /// Names use qualified function naming (simple_name + "_" + param_types)
  /// to match the symbols exported by the stdlib archive (std.a).
  ///
  /// All type-to-string conversions are generated by builtins.lale.
  /// The extern list is manually maintained; it changes infrequently.
  pub fn add_stdlib_externs(&mut self) {
    // Note: The text struct is defined in builtins.lale (loaded separately),
    // not here. Externs only declare C ABI functions with raw pointers/scalars.

    // ========== I/O Hooks ==========

    // POSIX write syscall — used for stdout (fd=1), stderr (fd=2), and file I/O.
    // Replaces the 4 legacy handlers (__lale_write_stdout/stderr/error/alert_pointer_i64).
    self.add_extern_func(ExternFunc::new(
      "write",
      vec![IrType::I32, IrType::ptr(IrType::I8), IrType::I64],
      IrType::I64,
    ));

    // ========== Runtime Hooks ==========

    // Exit program hook
    self.add_extern_func(ExternFunc::new(
      "__lale_exit",
      vec![IrType::I32],
      IrType::Void,
    ));

    // Floating-point exponentiation hook
    self.add_extern_func(ExternFunc::new(
      "pow",
      vec![IrType::F64, IrType::F64],
      IrType::F64,
    ));

    // Memory allocation hook (u64 parameter to match stdlib export)
    self.add_extern_func(ExternFunc::new(
      "__lale_malloc_u64",
      vec![IrType::U64],
      IrType::ptr(IrType::I8),
    ));

    // Memory deallocation hook
    self.add_extern_func(ExternFunc::new(
      "__lale_free_pointer",
      vec![IrType::ptr(IrType::I8)],
      IrType::Void,
    ));

    // Runtime timestamp hook (ISO-8601 UTC)
    self.add_extern_func(ExternFunc::new(
      "__lale_timestamp",
      vec![],
      IrType::struct_ref("text"),
    ));

    // Runtime log-level threshold hook (0=off, 1=alert, 2=warn, 3=log)
    self.add_extern_func(ExternFunc::new("__lale_log_level", vec![], IrType::I32));

    // ========== Type-to-String Conversion Functions ==========
    // The integer/boolean/character conversions are exported from builtins.lale;
    // the f64 conversion is handled by the interpreter (src/interpreter.rs
    // call_extern) because shortest round-trip formatting (Ryu) needs 128-bit
    // arithmetic Lale cannot express. The list is manually maintained
    // and changes infrequently (only when new integer/float types are added).

    self.add_extern_func(ExternFunc::new(
      "__lale_i32_to_str_i32",
      vec![IrType::I32],
      IrType::struct_ref("text"),
    ));

    self.add_extern_func(ExternFunc::new(
      "__lale_i64_to_str_i64",
      vec![IrType::I64],
      IrType::struct_ref("text"),
    ));

    self.add_extern_func(ExternFunc::new(
      "__lale_u64_to_str_u64",
      vec![IrType::U64],
      IrType::struct_ref("text"),
    ));

    self.add_extern_func(ExternFunc::new(
      "__lale_f64_to_str_f64",
      vec![IrType::F64],
      IrType::struct_ref("text"),
    ));
  }

  /// Is the module well-formed?
  pub fn validate(&self) -> Vec<String> {
    let mut errors = Vec::new();

    for func in &self.functions {
      if !func.is_well_formed() {
        errors.push(format!("Function '{}' has unterminated blocks", func.name));
      }
    }

    errors
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_new_module() {
    let module = Module::new("test");
    assert_eq!(module.name, "test");
    assert!(module.functions.is_empty());
    assert!(module.globals.is_empty());
    // str is NOT pre-registered — it comes from builtins.lale.
    assert!(module.struct_def("text").is_none());
  }

  #[test]
  fn test_add_function() {
    let mut module = Module::new("test");
    let id = module.add_function("main", IrType::Void, Linkage::Export);
    assert_eq!(id, FuncId::new(0));
    assert!(module.function(id).is_some());
    assert!(module.function_by_name("main").is_some());
  }

  #[test]
  fn test_add_global() {
    let mut module = Module::new("test");
    let id = module.add_global("counter", IrType::I32, Linkage::Internal);
    assert_eq!(id, GlobalId::new(0));
    assert!(module.global(id).is_some());
    assert!(module.global_by_name("counter").is_some());
  }

  #[test]
  fn test_add_struct() {
    let mut module = Module::new("test");
    let mut point = StructDef::new("Point");
    point.add_field("x", IrType::F64);
    point.add_field("y", IrType::F64);
    module.add_struct(point);

    let def = module.struct_def("Point").unwrap();
    assert_eq!(def.fields.len(), 2);
    assert_eq!(def.field_index("y"), Some(1));
    assert_eq!(def.field_units.len(), 2);
    assert!(def.field_units.iter().all(|u| u.is_none()));
  }

  #[test]
  fn test_stdlib_externs() {
    let mut module = Module::new("test");
    module.add_stdlib_externs();
    // I/O hooks
    assert!(module.has_extern_func("write"));
    // Runtime hooks
    assert!(module.has_extern_func("__lale_exit"));
    assert!(module.has_extern_func("pow"));
    assert!(module.has_extern_func("__lale_malloc_u64"));
    assert!(module.has_extern_func("__lale_free_pointer"));
    // Old function names should NOT be present
    assert!(!module.has_extern_func("puts"));
    assert!(!module.has_extern_func("exit"));
    assert!(!module.has_extern_func("__lale_pow_f64_f64"));
    assert!(!module.has_extern_func("malloc"));
    assert!(!module.has_extern_func("itoa"));
    assert!(!module.has_extern_func("__lale_write_stdout_pointer_i64"));
    assert!(!module.has_extern_func("__lale_write_stderr_pointer_i64"));
    assert!(!module.has_extern_func("__lale_error_pointer_i64"));
    assert!(!module.has_extern_func("__lale_alert_pointer_i64"));
  }
}
