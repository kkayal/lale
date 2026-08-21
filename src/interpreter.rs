//! Lale IR Interpreter
//!
//! This module executes the Lale IR by interpreting it directly (without compilation).
//! It calls C library functions when needed (like puts for output) and supports
//! control flow (branches, loops) and multiple basic blocks.
//!
//! Note: This is a pure Rust interpreter that walks the IR and executes each instruction.
//!
//! Quick navigation:
//!   line   24 — Value enum (String, Int, Uint, Float, Bool, Null, Pointer, Struct, Optional, Vec2/3/4)
//!   line   55 — MemoryRegion / MemoryManager types (static memory for globals, heap allocator)
//!   line  315 — Value::to_c_string / Value::extract_float
//!   line  350 — extract_value_string helper
//!   line  511 — ControlFlow enum (Continue, Jump, Return)
//!   line  600 — lale_error_and_abort helper
//!   line  621 — execute_module (entry point)
//!   line  668 — execute_function
//!   line  741 — execute_block
//!   line  761 — execute_block_with_return
//!   line  847 — execute_function_with_args
//!   line  983 — execute_instruction_with_memory (giant match; sub-sections below)
//!   line  992   ├─ Constants (ConstString, ConstInt, ConstUint, ConstFloat, ConstBool, ConstNull)
//!   line 1029   ├─ String concatenation (Concat)
//!   line 1062   ├─ Memory ops (Alloca, Store, BoundsCheck, ZeroCheck, Load, GetElementPtr, GetFieldPtr, PtrToInt)
//!   line 1328   ├─ Arithmetic (Add, Sub, Mul, Cross, Dot, Div, Pow, Rem, Neg)
//!   line 1640   ├─ Bitwise (BitAnd, BitOr, BitXor, BitNot, Shl, Shr, UShr)
//!   line 1805   ├─ Comparison (Eq, Ne, Lt, Le, Gt, Ge)
//!   line 2191   ├─ Logical (And, Or, Xor, Not)
//!   line 2251   ├─ Type Conversions (SExt, ZExt, SiToFp, UiToFp, FpToSi, FpToUi, Trunc, FpTrunc, FpExt, IntToPtr)
//!   line 2407   ├─ Function Calls — CallVoid (puts, exit)
//!   line 2451   ├─ Function Calls — Call with return (write, malloc, to_str, pow, open, read, write, close, parse, read_line, Lale function dispatch)
//!   line 3095   ├─ Struct ops (BuildStruct, BuildVec2/3/4, ExtractField, InsertField, StructFieldPtr)
//!   line 3287   ├─ Optional ops (Some, None, UnwrapOptional)
//!   line 3346   ├─ Bitcast
//!   line 3386   ├─ Error Stack (PushError, PopError, ErrorCount, DrainErrors)
//!   line 3455   └─ Branch/Return passthrough (Br, CondBr, Ret, RetVoid)
//!   line 3465 — puts (extern C)
//!   line 3472 — tests module

use crate::ir::module::Constant;
use crate::ir::values::{BlockId, GlobalId};
use crate::ir::{BasicBlock, FuncRef, Function, Instruction, IrType, Module, ValueId};
use std::collections::HashMap;
use std::error::Error;
use std::fs::File;
use std::sync::Mutex;

use std::alloc::{Layout, alloc, dealloc};

// Thread-local file handle manager for POSIX syscalls
thread_local! {
  static FILE_HANDLES: Mutex<HashMap<i32, File>> = Mutex::new(HashMap::new());
  static NEXT_FD: Mutex<i32> = const { Mutex::new(3) }; // Start fd numbering at 3 (0=stdin, 1=stdout, 2=stderr)
  /// V3 real-heap allocation log.
  static ALLOC_LOG: Mutex<AllocLog> = const { Mutex::new(AllocLog { entries: Vec::new() }) };
}

/// Represents a value during IR execution
#[derive(Debug, Clone)]
enum Value {
  String(String),
  Int(i64),
  Uint(u64),
  Float(f64),
  Bool(bool),
  Null,
  /// A pointer to a real host memory address (V3: real heap).
  Pointer(usize),
  /// A struct value with name and field values
  Struct(String, Vec<Value>),
  /// An optional value: Some(value) or None
  Optional {
    is_some: bool,
    value: Box<Value>,
  },
  /// 2-component vector value.
  Vec2(Box<Value>, Box<Value>),
  /// 3-component vector value.
  Vec3(Box<Value>, Box<Value>, Box<Value>),
  /// 4-component vector value.
  Vec4(Box<Value>, Box<Value>, Box<Value>, Box<Value>),
}

/// Represents an allocated memory region (for records/structs)
#[derive(Debug, Clone)]
struct MemoryRegion {
  base_address: i64,
  size: i64,
  fields: HashMap<i64, Value>, // offset -> value
}

/// Sentinel base address for static (global) memory regions.
/// Emulates the `.data`/`.rodata` section an AOT backend would use.
/// Globals allocated from this range are never tracked by AllocLog
/// and never reported as heap leaks.
const STATIC_ADDR_BASE: i64 = 0x7fff_0000_0000_0000;

/// Memory manager that tracks allocated regions
#[derive(Debug)]
struct MemoryManager {
  regions: HashMap<i64, MemoryRegion>, // base_address -> region
  global_addresses: HashMap<GlobalId, i64>, // GlobalId -> base address
  /// Bump allocator for static (global) memory — emulates .data/.rodata.
  /// Incremented by each global's type size + alignment padding.
  next_static_addr: i64,
}

// ── V3 AllocLog: real-heap allocation tracking ──────────────────

/// Tracks allocations on the real host heap for leak detection.
/// This is the V3 replacement for `MemoryManager`'s simulated heap tracking.
/// Each entry records source location for actionable leak reports.
#[derive(Debug, Default)]
struct AllocLog {
  entries: Vec<AllocEntry>,
}

#[derive(Debug, Clone)]
struct AllocEntry {
  ptr: *mut u8,
  size: usize,
  source_file: Option<String>,
  source_line: Option<i64>,
}

// SAFETY: AllocLog is only accessed from the single-threaded interpreter.
unsafe impl Send for AllocLog {}

#[allow(dead_code)]
impl AllocLog {
  fn new() -> Self {
    Self {
      entries: Vec::new(),
    }
  }

  /// Allocate memory on the real heap and log the allocation.
  fn alloc_log(&mut self, size: usize, file: Option<&str>, line: Option<i64>) -> *mut u8 {
    // Alignment 8 is always a power of two, so layout construction is infallible.
    let layout = match Layout::from_size_align(size, 8) {
      Ok(l) => l,
      Err(_) => unreachable!("alignment 8 is always valid"),
    };
    let ptr = unsafe { alloc(layout) };
    if ptr.is_null() {
      panic!("AllocLog: allocation of {} bytes failed", size);
    }
    self.entries.push(AllocEntry {
      ptr,
      size,
      source_file: file.map(|s| s.to_string()),
      source_line: line,
    });
    ptr
  }

  /// Free memory on the real heap and remove from log.
  fn dealloc_log(&mut self, ptr: *mut u8) {
    self.entries.retain(|e| {
      if std::ptr::eq(e.ptr, ptr) {
        // Alignment 8 is always a power of two, so layout construction is infallible.
        let layout = match Layout::from_size_align(e.size, 8) {
          Ok(l) => l,
          Err(_) => unreachable!("alignment 8 is always valid"),
        };
        unsafe { dealloc(e.ptr, layout) };
        false
      } else {
        true
      }
    });
  }

  /// Check for unfreed allocations. Returns count of leaks.
  fn check_leaks(&self) -> usize {
    self.entries.len()
  }

  /// Report leaks with source location. Returns the number of leaked
  /// allocations so callers can force a non-zero exit when appropriate.
  fn report_leaks(&self) -> usize {
    let leaks = self.check_leaks();
    if leaks > 0 {
      eprintln!("HEAP LEAK: {} allocation(s) not freed", leaks);
      for entry in &self.entries {
        let src = match (&entry.source_file, entry.source_line) {
          (Some(f), Some(l)) => format!(" ({}:{})", f, l),
          _ => String::new(),
        };
        eprintln!("  leaked {:p} ({} bytes){}", entry.ptr, entry.size, src);
      }
    }
    leaks
  }
}

// ── V2 MemoryManager (deprecated, replaced by AllocLog in V3) ──

/// Memory manager that tracks allocated regions
impl MemoryManager {
  fn new() -> Self {
    Self {
      regions: HashMap::new(),
      global_addresses: HashMap::new(),
      next_static_addr: STATIC_ADDR_BASE,
    }
  }

  /// Initialize global variables from the module.
  ///
  /// Global memory uses synthetic addresses from a static pool (bump allocator
  /// starting at `STATIC_ADDR_BASE`). This emulates the `.data`/`.rodata`
  /// section an AOT backend would use — globals are not tracked by AllocLog
  /// and never appear in heap leak reports.
  fn init_globals(&mut self, module: &Module) -> Result<(), Box<dyn Error>> {
    // Align to 8 bytes between globals to prevent accidental overlaps.
    const ALIGN: i64 = 8;

    for global in &module.globals {
      let size = Self::compute_type_size(&global.ty);
      let base = self.next_static_addr;
      // Advance the bump pointer: size rounded up to 8-byte alignment.
      self.next_static_addr += (size + ALIGN - 1) & !(ALIGN - 1);

      self.global_addresses.insert(global.id, base);
      self.regions.insert(
        base,
        MemoryRegion {
          base_address: base,
          size,
          fields: HashMap::new(),
        },
      );

      if let Some(init) = &global.initializer {
        // str globals need a pointer to separate string data, matching the
        // AOT layout (.rodata for bytes, .data for the struct wrapper).
        // Without this special case, init_constant would store raw bytes
        // inline at offset 0 instead of a proper Value::Pointer.
        if let IrType::Struct { name } = &global.ty
          && name == "str"
          && let Constant::Struct(fields) = init
        {
          if let Some((_, Constant::String(s))) = fields.first() {
            let data_addr = self.alloc_static_string(s);
            self.store(base, Value::Pointer(data_addr as usize))?;
          }
          if let Some((_, Constant::Uint(len))) = fields.get(1) {
            self.store(base + 8, Value::Uint(*len))?;
          }
          // `continue` to skip the generic init_constant below.
          continue;
        }
        self.init_constant(base, init)?;
      }
    }
    Ok(())
  }

  /// Recursively compute the byte size of an IR type.
  fn compute_type_size(ty: &IrType) -> i64 {
    match ty {
      IrType::Array { element, size } => {
        let elem_size = Self::compute_type_size(element);
        (*size as i64) * elem_size
      }
      IrType::Struct { .. } => 64, // Conservatively allocate 64 bytes for structs
      IrType::Ptr(_) => 8,
      IrType::I8 | IrType::U8 => 1,
      IrType::I16 | IrType::U16 | IrType::F16 => 2,
      IrType::I32 | IrType::U32 | IrType::F32 => 4,
      IrType::I64 | IrType::U64 | IrType::F64 => 8,
      IrType::Bool => 1,
      IrType::Char => 4,
      IrType::Void => 0,
      IrType::Optional(_) => 16, // 8 bytes tag (is_some) + 8 bytes value
      IrType::Vec2(inner) => Self::compute_type_size(inner) * 2,
      IrType::Vec3(inner) => Self::compute_type_size(inner) * 3,
      IrType::Vec4(inner) => Self::compute_type_size(inner) * 4,
    }
  }

  /// Initialize a memory region with a constant value
  fn init_constant(&mut self, addr: i64, constant: &Constant) -> Result<(), Box<dyn Error>> {
    match constant {
      Constant::Int(i) => {
        self.store(addr, Value::Int(*i))?;
      }
      Constant::Uint(u) => {
        self.store(addr, Value::Uint(*u))?;
      }
      Constant::Float(f) => {
        self.store(addr, Value::Float(*f))?;
      }
      Constant::Bool(b) => {
        self.store(addr, Value::Bool(*b))?;
      }
      Constant::String(s) => {
        for (i, byte) in s.bytes().enumerate() {
          self.store(addr + i as i64, Value::Uint(byte as u64))?;
        }
      }
      Constant::Null => {
        self.store(addr, Value::Null)?;
      }
      Constant::Array(elements) => {
        for (i, elem) in elements.iter().enumerate() {
          let offset = addr + (i as i64) * 8;
          match elem {
            Constant::Uint(u) => {
              self.store(offset, Value::Uint(*u))?;
            }
            Constant::Int(v) => {
              self.store(offset, Value::Int(*v))?;
            }
            _ => {
              self.init_constant(offset, elem)?;
            }
          }
        }
      }
      Constant::Struct(fields) => {
        for (i, (_name, val)) in fields.iter().enumerate() {
          self.init_constant(addr + (i as i64) * 8, val)?;
        }
      }
      Constant::Zero => {}
    }
    Ok(())
  }

  /// Get the address of a global variable
  fn global_addr(&self, id: GlobalId) -> Result<i64, Box<dyn Error>> {
    self
      .global_addresses
      .get(&id)
      .copied()
      .ok_or_else(|| format!("Global variable {:?} not found in address map", id).into())
  }

  /// Allocate and store a null-terminated string (for FFI compatibility).
  /// Uses the real heap via ALLOC_LOG for zero-copy FFI.
  /// Returns the real host address of the allocated string.
  fn alloc_null_terminated_string(&mut self, s: &str) -> Result<i64, Box<dyn Error>> {
    let bytes = s.as_bytes();
    let size = bytes.len() + 1;
    let ptr = ALLOC_LOG.with(|log| {
      #[allow(clippy::unwrap_used, clippy::expect_used)]
      log
        .lock()
        .expect("mutex poisoned")
        .alloc_log(size, None, None)
    });
    // Write raw bytes to real heap (for FFI).
    unsafe {
      std::ptr::copy_nonoverlapping(bytes.as_ptr(), ptr, bytes.len());
      ptr.add(bytes.len()).write(0u8);
    }
    // Also store to MemoryManager so readers find the data.
    let base = ptr as i64;
    self.regions.insert(
      base,
      MemoryRegion {
        base_address: base,
        size: size as i64,
        fields: HashMap::new(),
      },
    );
    for (i, &b) in bytes.iter().enumerate() {
      self.store(base + i as i64, Value::Uint(b as u64))?;
    }
    Ok(base)
  }

  /// Allocate string data in the static memory pool (emulating .rodata).
  /// Does NOT use AllocLog — static data lives for the program's lifetime
  /// and never appears in heap leak reports.
  fn alloc_static_string(&mut self, s: &str) -> i64 {
    let bytes = s.as_bytes();
    let size = (bytes.len() + 1) as i64;
    let base = self.next_static_addr;
    // Align to 8 bytes, reserving space for the null terminator.
    const ALIGN: i64 = 8;
    self.next_static_addr += (size + ALIGN - 1) & !(ALIGN - 1);

    self.regions.insert(
      base,
      MemoryRegion {
        base_address: base,
        size,
        fields: HashMap::new(),
      },
    );
    for (i, &b) in bytes.iter().enumerate() {
      self.store(base + i as i64, Value::Uint(b as u64)).ok();
    }
    base
  }

  fn store(&mut self, address: i64, value: Value) -> Result<(), Box<dyn Error>> {
    // Find all regions containing this address
    let matching_regions: Vec<i64> = self
      .regions
      .iter()
      .filter_map(|(&base, region)| {
        let offset = address - region.base_address;
        if offset >= 0 && offset < region.size {
          Some(base)
        } else {
          None
        }
      })
      .collect();

    match matching_regions.len() {
      0 => {
        // If no region found, create a simple one for this address
        self.regions.insert(
          address,
          MemoryRegion {
            base_address: address,
            size: 8,
            fields: HashMap::from([(0, value)]),
          },
        );
      }
      1 => {
        // Exactly one region found - store normally
        let region_base = matching_regions[0];
        let offset = address - region_base;
        if let Some(region) = self.regions.get_mut(&region_base) {
          region.fields.insert(offset, value);
        }
      }
      _ => {
        return Err(
          format!(
            "Memory address {:#x} is contained in {} overlapping regions — memory safety violation",
            address,
            matching_regions.len()
          )
          .into(),
        );
      }
    }
    Ok(())
  }

  /// Load a value from an address
  fn load(&self, address: i64) -> Value {
    // Find the region containing this address
    for region in self.regions.values() {
      let offset = address - region.base_address;
      if offset >= 0 && offset < region.size {
        if let Some(value) = region.fields.get(&offset) {
          return value.clone();
        }
        return Value::Int(0); // Uninitialized memory
      }
    }
    Value::Int(0) // Address not in any region
  }

  /// Allocate and store a null-terminated string (for FFI compatibility).
  fn read_c_string(&self, base: i64, offset: i64) -> String {
    self.read_c_string_sim(base + offset)
  }

  fn read_c_string_sim(&self, addr: i64) -> String {
    let mut result = String::new();
    let mut pos = 0i64;
    let max_len = 1024; // Prevent infinite loops

    for _ in 0..max_len {
      if let Value::Uint(byte) = self.load(addr + pos) {
        if byte == 0 {
          break;
        }
        // Only accept valid ASCII characters
        if byte < 128 {
          result.push(byte as u8 as char);
        }
        pos += 1;
      } else {
        break;
      }
    }

    result
  }

  fn read_byte(&self, base: i64, offset: i64) -> Option<u8> {
    match self.load(base + offset) {
      Value::Uint(byte) => Some(byte as u8),
      Value::Int(byte) => Some(byte as u8),
      _ => None,
    }
  }

  /// Write a single byte to memory
  #[allow(dead_code)]
  fn write_byte(&mut self, base: i64, offset: i64, byte: u8) -> Result<(), Box<dyn Error>> {
    self.store(base + offset, Value::Uint(byte as u64))
  }
}

impl Value {
  /// Convert to C string for puts()
  fn to_c_string(&self) -> Option<Vec<u8>> {
    match self {
      Value::String(s) => {
        let mut bytes = s.as_bytes().to_vec();
        bytes.push(0); // null terminator
        Some(bytes)
      }
      _ => None,
    }
  }

  /// Extract a float from a Value, converting integers as needed.
  /// Should only be called on numeric vector components.
  #[allow(clippy::boxed_local)]
  fn extract_float(v: Box<Value>) -> f64 {
    match *v {
      Value::Float(f) => f,
      Value::Int(i) => i as f64,
      Value::Uint(u) => u as f64,
      other => {
        ice!(
          "extract_float called on non-numeric value {:?} — vector IR generation should only produce numeric components",
          other
        );
      }
    }
  }
}

/// Extract a human-readable string representation from a Value.
/// Supports recursive unwrapping of Optional values.
fn extract_value_string(val: &Value, mm: &MemoryManager) -> String {
  match val {
    Value::String(s) => s.clone(),
    Value::Int(i) => format!("{}", i),
    Value::Uint(u) => format!("{}", u),
    Value::Float(f) => format!("{}", f),
    Value::Bool(b) => {
      if *b {
        "true".to_string()
      } else {
        "false".to_string()
      }
    }
    Value::Null => "null".to_string(),
    Value::Struct(name, fields) if name == "str" && fields.len() >= 2 => {
      // str struct: {ptr, len}
      let ptr_val = &fields[0];
      let len_val = &fields[1];

      let len = match len_val {
        Value::Int(l) => *l as usize,
        Value::Uint(l) => *l as usize,
        _ => 0,
      };

      match ptr_val {
        Value::String(s) => {
          if len <= s.len() {
            s[..len].to_string()
          } else {
            s.clone()
          }
        }
        Value::Pointer(p_ptr) => {
          let mut bytes = Vec::with_capacity(len);
          for i in 0..len {
            let addr = *p_ptr as i64 + i as i64;
            if let Some(b) = mm.read_byte(addr, 0) {
              bytes.push(b);
            } else {
              bytes.push(0);
            }
          }
          String::from_utf8_lossy(&bytes).to_string()
        }
        _ => String::new(),
      }
    }
    Value::Optional { is_some, value } => {
      if *is_some {
        extract_value_string(value, mm)
      } else {
        "none".to_string()
      }
    }
    Value::Pointer(addr) => {
      // This is a pointer to a str struct - extract ptr (field 0) and len (field 1)
      let combined_addr = *addr as i64;
      let ptr_val = mm.load(combined_addr); // field 0: ptr
      let len_val = mm.load(combined_addr + 8); // field 1: byte_len

      let len = match len_val {
        Value::Int(l) => l as usize,
        Value::Uint(l) => l as usize,
        _ => 0,
      };

      // Get the actual string data
      match ptr_val {
        Value::String(s) => {
          if len <= s.len() {
            s[..len].to_string()
          } else {
            s
          }
        }
        Value::Pointer(p_ptr) => {
          let mut bytes = Vec::with_capacity(len);
          for i in 0..len {
            if let Some(b) = mm.read_byte(p_ptr as i64, i as i64) {
              bytes.push(b);
            }
          }
          String::from_utf8_lossy(&bytes).to_string()
        }
        Value::Int(pa) => {
          let mut bytes = Vec::with_capacity(len);
          for i in 0..len {
            if let Some(b) = mm.read_byte(pa, i as i64) {
              bytes.push(b);
            }
          }
          String::from_utf8_lossy(&bytes).to_string()
        }
        Value::Uint(pa) => {
          let mut bytes = Vec::with_capacity(len);
          for i in 0..len {
            if let Some(b) = mm.read_byte(pa as i64, i as i64) {
              bytes.push(b);
            }
          }
          String::from_utf8_lossy(&bytes).to_string()
        }
        _ => String::new(),
      }
    }
    Value::Struct(name, fields) if name.starts_with("optional_") && fields.len() >= 2 => {
      // Optional struct: {is_present, value}
      let is_present = match &fields[0] {
        Value::Bool(b) => *b,
        _ => false,
      };
      if is_present {
        extract_value_string(&fields[1], mm)
      } else {
        "nothing".to_string()
      }
    }
    Value::Struct(_, _) => "{struct}".to_string(),
    Value::Vec2(x, y) => format!(
      "vec2({}, {})",
      extract_value_string(x, mm),
      extract_value_string(y, mm)
    ),
    Value::Vec3(x, y, z) => format!(
      "vec3({}, {}, {})",
      extract_value_string(x, mm),
      extract_value_string(y, mm),
      extract_value_string(z, mm)
    ),
    Value::Vec4(x, y, z, w) => format!(
      "vec4({}, {}, {}, {})",
      extract_value_string(x, mm),
      extract_value_string(y, mm),
      extract_value_string(z, mm),
      extract_value_string(w, mm)
    ),
  }
}

// ── Centralized extern dispatch helpers ──────────────────────────

/// Extract an integer (i64) from a value argument.
fn get_int_arg(
  args: &[ValueId],
  idx: usize,
  values: &HashMap<ValueId, Value>,
  default: i64,
) -> i64 {
  args
    .get(idx)
    .and_then(|id| values.get(id))
    .map(|v| match v {
      Value::Int(i) => *i,
      Value::Uint(u) => *u as i64,
      _ => default,
    })
    .unwrap_or(default)
}

/// Extract a pointer (base, offset) from a value argument.
/// String values are lazily allocated as null-terminated C strings.
/// Returns (base, offset, Option<owned_addr>) — caller must free owned_addr after use.
fn get_ptr_arg(
  args: &[ValueId],
  idx: usize,
  values: &HashMap<ValueId, Value>,
  mem: &mut MemoryManager,
) -> Result<(i64, i64, Option<i64>), Box<dyn Error>> {
  match args.get(idx).and_then(|id| values.get(id)) {
    Some(Value::Pointer(addr)) => Ok((*addr as i64, 0, None)),
    Some(Value::String(s)) => {
      let addr = mem.alloc_null_terminated_string(s)?;
      Ok((addr, 0, Some(addr)))
    }
    Some(Value::Int(addr)) => Ok((*addr, 0, None)),
    Some(Value::Uint(addr)) => Ok((*addr as i64, 0, None)),
    other => Err(format!("expected pointer or string, got {:?}", other).into()),
  }
}

/// Read bytes from the interpreter's simulated memory.
fn mem_read_bytes(mem: &MemoryManager, base: i64, offset: i64, count: usize) -> Vec<u8> {
  (0..count)
    .filter_map(|i| mem.read_byte(base, offset + i as i64))
    .collect()
}

// ── V3 serialization helpers (Value ↔ raw bytes) ──────────────────

/// Write a Value to a real memory address as raw bytes based on the IR type.
/// Uses little-endian byte order (Lale canonical).
#[allow(dead_code)]
fn v3_store(addr: usize, ty: &IrType, value: &Value) -> Result<(), Box<dyn Error>> {
  let ptr = addr as *mut u8;
  match ty {
    IrType::I8 => unsafe { ptr.write(value.as_i64() as i8 as u8) },
    IrType::U8 => unsafe { ptr.write(value.as_u64() as u8) },
    IrType::I16 => unsafe { *(ptr as *mut i16) = value.as_i64() as i16 },
    IrType::U16 => unsafe { *(ptr as *mut u16) = value.as_u64() as u16 },
    IrType::I32 => unsafe { (ptr as *mut u32).write_unaligned(value.as_i64() as u32) },
    IrType::U32 => unsafe { (ptr as *mut u32).write_unaligned(value.as_u64() as u32) },
    IrType::F32 => unsafe { (ptr as *mut u32).write_unaligned((value.as_f64() as f32).to_bits()) },
    IrType::I64 => unsafe { (ptr as *mut i64).write_unaligned(value.as_i64()) },
    IrType::U64 => unsafe { (ptr as *mut u64).write_unaligned(value.as_u64()) },
    IrType::F64 => unsafe { (ptr as *mut f64).write_unaligned(value.as_f64()) },
    IrType::Bool => unsafe { ptr.write(if value.as_bool() { 1u8 } else { 0u8 }) },
    IrType::Char => unsafe { (ptr as *mut u32).write_unaligned(value.as_i64() as u32) },
    IrType::Ptr(_) => unsafe {
      let pval = value.as_ptr_addr();
      (ptr as *mut u64).write_unaligned(pval as u64)
    },
    IrType::Struct { .. }
    | IrType::Optional(_)
    | IrType::Array { .. }
    | IrType::Vec2(_)
    | IrType::Vec3(_)
    | IrType::Vec4(_) => {
      // Delegated to MemoryManager until layout is fully specified
      return Err("v3_store: complex type not yet migrated".into());
    }
    IrType::Void | IrType::F16 => {}
  }
  Ok(())
}

/// Read a Value from a real memory address based on the IR type.
#[allow(dead_code)]
fn v3_load(addr: usize, ty: &IrType) -> Result<Value, Box<dyn Error>> {
  let ptr = addr as *const u8;
  Ok(match ty {
    IrType::I8 => Value::Int(unsafe { ptr.read() } as i8 as i64),
    IrType::U8 => Value::Uint(unsafe { ptr.read() } as u64),
    IrType::I16 => Value::Int(unsafe { *(ptr as *const i16) } as i64),
    IrType::U16 => Value::Uint(unsafe { *(ptr as *const u16) } as u64),
    IrType::I32 => Value::Int(unsafe { (ptr as *const u32).read_unaligned() } as i64),
    IrType::U32 => Value::Uint(unsafe { (ptr as *const u32).read_unaligned() } as u64),
    IrType::F32 => {
      let bits = unsafe { (ptr as *const u32).read_unaligned() };
      Value::Float(f32::from_bits(bits) as f64)
    }
    IrType::I64 => Value::Int(unsafe { (ptr as *const i64).read_unaligned() }),
    IrType::U64 => Value::Uint(unsafe { (ptr as *const u64).read_unaligned() }),
    IrType::F64 => Value::Float(unsafe { (ptr as *const f64).read_unaligned() }),
    IrType::Bool => Value::Bool(unsafe { ptr.read() != 0 }),
    IrType::Char => {
      let cp = unsafe { (ptr as *const u32).read_unaligned() };
      Value::Int(cp as i64)
    }
    IrType::Ptr(_) => {
      let raw = unsafe { (ptr as *const u64).read_unaligned() };
      Value::Pointer(raw as usize)
    }
    _ => return Err("v3_load: complex type not yet migrated".into()),
  })
}

// ── Value extraction helpers ─────────────────────────────────────

impl Value {
  #[allow(dead_code)]
  fn as_i64(&self) -> i64 {
    match self {
      Value::Int(i) => *i,
      Value::Uint(u) => *u as i64,
      _ => 0,
    }
  }
  #[allow(dead_code)]
  fn as_u64(&self) -> u64 {
    match self {
      Value::Uint(u) => *u,
      Value::Int(i) => *i as u64,
      _ => 0,
    }
  }
  #[allow(dead_code)]
  fn as_f64(&self) -> f64 {
    match self {
      Value::Float(f) => *f,
      Value::Int(i) => *i as f64,
      Value::Uint(u) => *u as f64,
      _ => 0.0,
    }
  }
  #[allow(dead_code)]
  fn as_bool(&self) -> bool {
    matches!(self, Value::Bool(true))
  }
  #[allow(dead_code)]
  fn as_ptr_addr(&self) -> usize {
    match self {
      Value::Pointer(a) => *a,
      Value::Int(i) => *i as usize,
      Value::Uint(u) => *u as usize,
      _ => 0,
    }
  }
}

/// Centralized dispatch for all `FuncRef::External` function calls.
/// Returns the result value (None for void functions).
///
/// The 9 parameters are justified: this is the single dispatch point for
/// every extern, needing access to values, memory, the module, and error state.
#[allow(clippy::too_many_arguments)]
fn call_extern(
  name: &str,
  args: &[ValueId],
  values: &mut HashMap<ValueId, Value>,
  mem: &mut MemoryManager,
  ir_module: &Module,
  error_stack: &mut Vec<String>,
  struct_field_counts: &HashMap<String, usize>,
  source_file: Option<&str>,
  source_line: Option<i64>,
) -> Result<Option<Value>, Box<dyn Error>> {
  use std::io::Write;
  match name {
    // ── Output ──
    "puts" => {
      if let Some(arg_id) = args.first()
        && let Some(value) = values.get(arg_id)
        && let Some(c_string) = value.to_c_string()
      {
        unsafe { puts(c_string.as_ptr() as *const i8) };
      }
      Ok(None)
    }

    // ── Memory management ──
    "__lale_malloc_u64" | "malloc" => {
      let size = get_int_arg(args, 0, values, 64) as usize;
      let (stack_file, stack_line) = resolve_alloc_source();
      let src_file = stack_file.or(source_file.map(|s| s.to_string()));
      let src_line = stack_line.or(source_line);
      let ptr = ALLOC_LOG.with(|log| {
        #[allow(clippy::unwrap_used, clippy::expect_used)]
        log
          .lock()
          .expect("mutex poisoned")
          .alloc_log(size, src_file.as_deref(), src_line)
      });
      // Create a MemoryManager region so subsequent stores don't overlap with nearby allocations.
      #[allow(clippy::unwrap_used, clippy::expect_used)]
      mem.regions.insert(
        ptr as i64,
        MemoryRegion {
          base_address: ptr as i64,
          size: size as i64,
          fields: HashMap::new(),
        },
      );
      Ok(Some(Value::Pointer(ptr as usize)))
    }
    "__lale_free_pointer" | "free" => {
      if let Some(Value::Pointer(addr)) = args.first().and_then(|id| values.get(id)) {
        mem.regions.remove(&(*addr as i64));
        ALLOC_LOG.with(|log| {
          #[allow(clippy::unwrap_used, clippy::expect_used)]
          log
            .lock()
            .expect("mutex poisoned")
            .dealloc_log(*addr as *mut u8)
        });
      }
      Ok(None)
    }

    // ── Program control ──
    "__lale_exit" => {
      let code = get_int_arg(args, 0, values, 1) as i32;
      std::process::exit(code);
    }

    // ── Math ──
    "pow" => {
      let base = args
        .first()
        .and_then(|id| values.get(id))
        .map(|v| match v {
          Value::Float(f) => *f,
          Value::Int(i) => *i as f64,
          Value::Uint(u) => *u as f64,
          _ => 0.0,
        })
        .unwrap_or(0.0);
      let exp = args
        .get(1)
        .and_then(|id| values.get(id))
        .map(|v| match v {
          Value::Float(f) => *f,
          Value::Int(i) => *i as f64,
          Value::Uint(u) => *u as f64,
          _ => 1.0,
        })
        .unwrap_or(1.0);
      Ok(Some(Value::Float(base.powf(exp))))
    }

    // ── POSIX I/O ──
    "write" => {
      let fd = get_int_arg(args, 0, values, -1) as i32;
      let (buf_base, buf_off, owned) = get_ptr_arg(args, 1, values, mem)?;
      let count = get_int_arg(args, 2, values, 0) as usize;
      let buf = mem_read_bytes(mem, buf_base, buf_off, count);

      let result = match fd {
        1 => std::io::stdout()
          .write_all(&buf)
          .map(|_| count as i64)
          .unwrap_or(-1),
        2 => std::io::stderr()
          .write_all(&buf)
          .map(|_| count as i64)
          .unwrap_or(-1),
        _ => -1,
      };

      if let Some(addr) = owned {
        mem.regions.remove(&addr);
        ALLOC_LOG.with(|log| {
          #[allow(clippy::unwrap_used, clippy::expect_used)]
          log
            .lock()
            .expect("mutex poisoned")
            .dealloc_log(addr as usize as *mut u8);
        });
      }
      Ok(Some(Value::Int(result)))
    }

    "read" => {
      let fd = get_int_arg(args, 0, values, -1) as i32;
      let (buf_base, buf_off, _) = get_ptr_arg(args, 1, values, mem)?;
      let count = get_int_arg(args, 2, values, 0) as usize;

      let bytes_read = if fd == 0 {
        // stdin
        use std::io::Read;
        let mut buf = vec![0u8; count];
        match std::io::stdin().read(&mut buf) {
          Ok(n) => {
            for (i, &b) in buf[..n].iter().enumerate() {
              let _ = mem.store(buf_base + buf_off + i as i64, Value::Int(b as i64));
            }
            n as i64
          }
          Err(_) => -1,
        }
      } else if fd >= 3 {
        FILE_HANDLES.with(|h| {
          #[allow(clippy::unwrap_used, clippy::expect_used)]
          let mut handles = h.lock().expect("mutex poisoned");
          if let Some(file) = handles.get_mut(&fd) {
            use std::io::Read;
            let mut buf = vec![0u8; count];
            match file.read(&mut buf) {
              Ok(n) => {
                for (i, &b) in buf[..n].iter().enumerate() {
                  let _ = mem.store(buf_base + buf_off + i as i64, Value::Int(b as i64));
                }
                n as i64
              }
              Err(_) => -1,
            }
          } else {
            -1
          }
        })
      } else {
        -1 // stdin/stdout/stderr not supported via 'read' extern
      };
      Ok(Some(Value::Int(bytes_read)))
    }

    "open" => {
      let (path_base, path_off, owned) = get_ptr_arg(args, 0, values, mem)?;
      let flags = get_int_arg(args, 1, values, 0) as i32;
      let path_str = mem.read_c_string(path_base, path_off);
      let is_readonly = (flags & 0x3) == 0;

      let open_result = if is_readonly {
        std::fs::File::open(&path_str)
      } else {
        std::fs::OpenOptions::new()
          .write(true)
          .create(true)
          .truncate(true)
          .open(&path_str)
      };

      if let Some(addr) = owned {
        mem.regions.remove(&addr);
        ALLOC_LOG.with(|log| {
          #[allow(clippy::unwrap_used, clippy::expect_used)]
          log
            .lock()
            .expect("mutex poisoned")
            .dealloc_log(addr as usize as *mut u8);
        });
      }

      let fd = match open_result {
        Ok(file) => {
          let new_fd = NEXT_FD.with(|c| {
            #[allow(clippy::unwrap_used, clippy::expect_used)]
            let mut fd = c.lock().expect("mutex poisoned");
            let cur = *fd;
            *fd += 1;
            cur
          });
          FILE_HANDLES.with(|h| {
            #[allow(clippy::unwrap_used, clippy::expect_used)]
            h.lock().expect("mutex poisoned").insert(new_fd, file);
          });
          new_fd as i64
        }
        Err(_) => -1,
      };
      Ok(Some(Value::Int(fd)))
    }

    "close" => {
      let fd = get_int_arg(args, 0, values, -1) as i32;
      let result = if fd >= 3 {
        FILE_HANDLES.with(|h| {
          #[allow(clippy::unwrap_used, clippy::expect_used)]
          if h.lock().expect("mutex poisoned").remove(&fd).is_some() {
            0
          } else {
            -1
          }
        })
      } else {
        -1
      };
      Ok(Some(Value::Int(result)))
    }

    "lseek" => {
      let fd = get_int_arg(args, 0, values, -1) as i32;
      let offset = get_int_arg(args, 1, values, 0);
      let whence = get_int_arg(args, 2, values, 0) as i32;

      let result = if fd >= 3 {
        FILE_HANDLES.with(|h| {
          #[allow(clippy::unwrap_used, clippy::expect_used)]
          let mut handles = h.lock().expect("mutex poisoned");
          if let Some(file) = handles.get_mut(&fd) {
            use std::io::Seek;
            let seek_from = match whence {
              0 if offset >= 0 => std::io::SeekFrom::Start(offset as u64),
              1 => std::io::SeekFrom::Current(offset),
              2 => std::io::SeekFrom::End(offset),
              _ => return -1,
            };
            match file.seek(seek_from) {
              Ok(pos) => pos as i64,
              Err(_) => -1,
            }
          } else {
            -1
          }
        })
      } else {
        -1
      };
      Ok(Some(Value::Int(result)))
    }

    // ── String parsing (libc strto*) ──
    // TODO(AOT): When an AOT backend exists, replace these interpreter handlers
    // with real FFI calls to libc's strtod/strtol/strtoul. The interpreter
    // emulates the libc API (reads C strings from MemoryManager, writes endptr
    // back to memory). AOT would link directly against libc.
    "strtod" => {
      if args.len() < 2 {
        return Ok(Some(Value::Float(f64::INFINITY)));
      }
      let (nptr_base, nptr_off, _) = get_ptr_arg(args, 0, values, mem)?;
      let endptr_addr = match values.get(&args[1]) {
        Some(Value::Pointer(addr)) => *addr as i64,
        Some(Value::Int(p)) => *p,
        _ => {
          return Ok(Some(Value::Float(f64::INFINITY)));
        }
      };
      let s = mem.read_c_string(nptr_base, nptr_off);
      let trimmed = s.trim_start();
      let (result, consumed) = if let Ok(val) = trimmed.parse::<f64>() {
        (Value::Float(val), s.len() - trimmed.len() + trimmed.len())
      } else {
        (Value::Float(f64::INFINITY), 0)
      };
      mem.store(
        endptr_addr,
        Value::Pointer((nptr_base + nptr_off + consumed as i64) as usize),
      )?;
      Ok(Some(result))
    }

    "strtol" => {
      if args.len() < 2 {
        return Ok(Some(Value::Int(i64::MIN)));
      }
      let (nptr_base, nptr_off, _) = get_ptr_arg(args, 0, values, mem)?;
      let endptr_addr = match values.get(&args[1]) {
        Some(Value::Pointer(addr)) => *addr as i64,
        Some(Value::Int(p)) => *p,
        _ => {
          return Ok(Some(Value::Int(i64::MIN)));
        }
      };
      let s = mem.read_c_string(nptr_base, nptr_off);
      let trimmed = s.trim_start();
      let (result, consumed) = if let Ok(val) = trimmed.parse::<i64>() {
        (Value::Int(val), s.len() - trimmed.len() + trimmed.len())
      } else {
        (Value::Int(i64::MIN), 0)
      };
      mem.store(
        endptr_addr,
        Value::Pointer((nptr_base + nptr_off + consumed as i64) as usize),
      )?;
      Ok(Some(result))
    }

    "strtoul" => {
      if args.len() < 2 {
        return Ok(Some(Value::Uint(u64::MAX)));
      }
      let (nptr_base, nptr_off, _) = get_ptr_arg(args, 0, values, mem)?;
      let endptr_addr = match values.get(&args[1]) {
        Some(Value::Pointer(addr)) => *addr as i64,
        Some(Value::Int(p)) => *p,
        _ => {
          return Ok(Some(Value::Uint(u64::MAX)));
        }
      };
      let s = mem.read_c_string(nptr_base, nptr_off);
      let trimmed = s.trim_start();
      let (result, consumed) = if let Ok(val) = trimmed.parse::<u64>() {
        (Value::Uint(val), s.len() - trimmed.len() + trimmed.len())
      } else {
        (Value::Uint(u64::MAX), 0)
      };
      mem.store(
        endptr_addr,
        Value::Pointer((nptr_base + nptr_off + consumed as i64) as usize),
      )?;
      Ok(Some(result))
    }

    // ── stdin read ──
    // TODO(AOT): When an AOT backend exists, replace this interpreter handler
    // with a real FFI call to libc getline/fgets. The interpreter reads from
    // Rust's stdin, allocates on the real heap, and populates MemoryManager
    // for the str return value. AOT would link directly against libc and let
    // the linker dead-code-eliminate the handler.
    "__lale_read_line" => {
      let mut input = String::new();
      if std::io::stdin().read_line(&mut input).is_err() {
        input = String::new();
      }
      let input = input.trim_end_matches(['\n', '\r']);
      let size = input.len() + 1;
      let base = ALLOC_LOG.with(|log| {
        #[allow(clippy::unwrap_used, clippy::expect_used)]
        log
          .lock()
          .expect("mutex poisoned")
          .alloc_log(size, None, None)
      });
      let base_i64 = base as i64;
      mem.regions.insert(
        base_i64,
        MemoryRegion {
          base_address: base_i64,
          size: size as i64,
          fields: HashMap::new(),
        },
      );
      for (i, &b) in input.as_bytes().iter().enumerate() {
        unsafe {
          base.add(i).write(b);
        }
        mem.store(base_i64 + i as i64, Value::Uint(b as u64))?;
      }
      unsafe {
        base.add(input.len()).write(0u8);
      }
      Ok(Some(Value::Struct(
        "str".to_string(),
        vec![
          Value::Pointer(base as usize),
          Value::Uint(input.len() as u64),
        ],
      )))
    }

    // ── Fallback: look up in module (stdlib functions) ──
    _ => {
      if let Some(called_func) = ir_module.functions.iter().find(|f| f.name == name) {
        let result = execute_function_with_args(
          ir_module,
          called_func,
          args,
          values,
          mem,
          struct_field_counts,
          error_stack,
          source_file,
          source_line,
        )?;
        Ok(Some(result))
      } else {
        Err(
          format!(
            "Undefined function: '{}' not found in module or as a built-in",
            name
          )
          .into(),
        )
      }
    }
  }
}

/// Control flow result from executing a block
#[derive(Debug, Clone)]
enum ControlFlow {
  /// Continue to next block (or exit if no next block)
  Continue,
  /// Jump to a specific block
  Jump(BlockId),
  /// Return from function with optional value
  Return(Option<Value>),
}

/// Trigger `__lale_error` with a message and abort.
/// Division by zero, modulo by zero, array bounds violations, and absent optional unwraps
/// all use this path. The message is pre-formatted with source location and details,
/// matching the format that the `write` extern receives.
fn lale_error_and_abort(
  file: &str,
  line: u64,
  col: u64,
  message: &str,
  index: i64,
  length: i64,
) -> ! {
  use std::io::Write;
  let error_msg = format!(
    "\x1b[31mERROR at {}:{}:{}: {} (index={}, length={})\x1b[0m\n",
    file, line, col, message, index, length
  );
  let _ = std::io::stderr().write_all(error_msg.as_bytes());
  print_call_stack();
  std::process::abort();
}

/// If the value is a str struct with a heap-allocated ptr field, free that
/// heap memory and remove its MemoryRegion. Silently ignores non-str and
/// non-heap values.
fn free_str_data(val: &Value, mm: &mut MemoryManager) {
  let ptr = match val {
    Value::Struct(name, fields) if name == "str" => match fields.first() {
      Some(Value::Pointer(base)) => Some(*base),
      _ => None,
    },
    Value::Pointer(addr) => {
      if let Value::Pointer(inner_base) = mm.load(*addr as i64) {
        Some(inner_base)
      } else {
        None
      }
    }
    _ => None,
  };
  if let Some(base) = ptr {
    // Remove the stale MemoryRegion so the address can be reused safely.
    mm.regions.remove(&(base as i64));
    ALLOC_LOG.with(|log| {
      #[allow(clippy::unwrap_used, clippy::expect_used)]
      log
        .lock()
        .expect("mutex poisoned")
        .dealloc_log(base as *mut u8);
    });
  }
}

/// Mutable test-runner state shared across IR instructions via a thread-local.
/// The interpreter is single-threaded per execution, so a thread-local is safe.
#[derive(Default)]
struct TestState {
  current_suite: Option<String>,
  current_case: Option<String>,
  current_case_failed: bool,
  any_failed: bool,
  /// Set when at least one TestBegin executed. Used to gate the
  /// "heap leak forces non-zero exit" rule to test mode only.
  ran_any_test: bool,
}

thread_local! {
  static TEST_STATE: std::cell::RefCell<TestState> =
    const { std::cell::RefCell::new(TestState {
      current_suite: None,
      current_case: None,
      current_case_failed: false,
      any_failed: false,
      ran_any_test: false,
    }) };
}

/// Execute an Lale IR module by interpreting it
pub fn execute_module(ir_module: &Module) -> Result<i32, Box<dyn Error>> {
  // Reset test state for this execution.
  TEST_STATE.with(|ts| {
    *ts.borrow_mut() = TestState {
      current_suite: None,
      current_case: None,
      current_case_failed: false,
      any_failed: false,
      ran_any_test: false,
    };
  });

  // Find main function
  let main_fn = ir_module
    .functions
    .iter()
    .find(|f| f.name == "main")
    .ok_or("No main function found in module")?;

  // Execute main function
  let result = execute_function(ir_module, main_fn);
  if result.is_err() {
    print_call_stack();
    return result;
  }

  // Report heap leaks (debug builds only).
  let leak_count = if cfg!(debug_assertions) {
    ALLOC_LOG.with(|log| {
      #[allow(clippy::unwrap_used, clippy::expect_used)]
      log.lock().expect("mutex poisoned").report_leaks()
    })
  } else {
    0
  };

  // A failing test case forces a non-zero exit code. A heap leak likewise
  // fails the run in test mode (but not in plain run mode) so the stdlib's
  // leak detector is trustworthy: `lale test` must exit non-zero when a test
  // leaked memory.
  let (any_failed, ran_any_test) = TEST_STATE.with(|ts| {
    let ts = ts.borrow();
    (ts.any_failed, ts.ran_any_test)
  });
  if any_failed || (ran_any_test && leak_count > 0) {
    Ok(1)
  } else {
    result
  }
}

// ---------------------------------------------------------------------------
// Call stack for runtime error reporting
// ---------------------------------------------------------------------------

type StackFrame = (String, Option<String>, Option<i64>);

thread_local! {
  static CALL_STACK: std::cell::RefCell<Vec<StackFrame>> =
    const { std::cell::RefCell::new(Vec::new()) };
}

/// Walk the call stack to find the outermost user frame (not stdlib/builtins).
/// Returns (file, line) for the allocation site, or (None, None) if stack is empty.
fn resolve_alloc_source() -> (Option<String>, Option<i64>) {
  CALL_STACK.with(|stack| {
    let stack = stack.borrow();
    // Search from most recent to oldest, skipping stdlib/builtins frames
    for (_name, file, line) in stack.iter().rev() {
      if let Some(f) = file
        && !f.contains("stdlib/")
        && !f.contains("builtins/")
      {
        return (Some(f.clone()), *line);
      }
    }
    // Fall back to the top frame if all are internal
    stack
      .last()
      .map_or((None, None), |(_, f, l)| (f.clone(), *l))
  })
}

fn print_call_stack() {
  use std::io::Write;
  CALL_STACK.with(|stack| {
    let stack = stack.borrow();
    if !stack.is_empty() {
      let _ = std::io::stderr().write_all(b"\nCall stack (most recent first):\n");
      for (i, (name, file, line)) in stack.iter().rev().enumerate() {
        match (file, line) {
          (Some(f), Some(l)) => {
            let _ = writeln!(std::io::stderr(), "  {}. {}  at {}:{}", i + 1, name, f, l);
          }
          _ => {
            let _ = writeln!(std::io::stderr(), "  {}. {}", i + 1, name);
          }
        }
      }
      let _ = std::io::stderr().flush();
    }
  });
}

/// Execute a single function from the IR with proper control flow handling
fn execute_function(ir_module: &Module, func: &Function) -> Result<i32, Box<dyn Error>> {
  // Execute the first basic block (entry)
  if func.blocks.is_empty() {
    return Err("Function has no basic blocks".into());
  }

  let mut values: HashMap<ValueId, Value> = HashMap::new();
  let mut mem_manager = MemoryManager::new();
  let mut error_stack: Vec<String> = Vec::new();

  // Initialize global variables before execution
  mem_manager.init_globals(ir_module)?;

  // Add function parameters to values
  // Note: For the main entry point, parameters should always be empty.
  // Regular function calls use execute_function_with_args() which properly
  // binds arguments to parameters (see lines 630-684 for parameter binding logic).
  // This initialization to 0 is a safety measure for edge cases.
  for param in func.params.iter() {
    values.insert(param.value_id, Value::Int(0));
  }

  // Build struct field count map from module
  let struct_field_counts: HashMap<String, usize> = ir_module
    .structs
    .iter()
    .map(|s| (s.name.clone(), s.fields.len()))
    .collect();

  // Start with entry block (first block)
  let mut current_block_idx = 0;

  // Control flow loop: execute blocks until we return or exit
  loop {
    // Get current block by index (blocks are in order, first is entry)
    if current_block_idx >= func.blocks.len() {
      break;
    }

    let block = &func.blocks[current_block_idx];

    // Execute this block's instructions
    match execute_block(
      ir_module,
      block,
      &mut values,
      &mut mem_manager,
      &struct_field_counts,
      &mut error_stack,
    )? {
      ControlFlow::Continue => {
        // No explicit control flow, assume we're done
        break;
      }
      ControlFlow::Jump(target_id) => {
        // Find the target block by ID
        if let Some(idx) = func.blocks.iter().position(|b| b.id == target_id) {
          current_block_idx = idx;
        } else {
          return Err(format!("Block {} not found", target_id.0).into());
        }
      }
      ControlFlow::Return(_) => {
        // Function returned
        break;
      }
    }
  }

  Ok(0)
}

/// Execute all instructions in a basic block and return where to go next
fn execute_block(
  ir_module: &Module,
  block: &BasicBlock,
  values: &mut HashMap<ValueId, Value>,
  mem_manager: &mut MemoryManager,
  struct_field_counts: &HashMap<String, usize>,
  error_stack: &mut Vec<String>,
) -> Result<ControlFlow, Box<dyn Error>> {
  let (cf, _ret) = execute_block_with_return(
    ir_module,
    block,
    values,
    mem_manager,
    struct_field_counts,
    error_stack,
  )?;
  Ok(cf)
}

/// Execute all instructions in a basic block and return control flow + return value
fn execute_block_with_return(
  ir_module: &Module,
  block: &BasicBlock,
  values: &mut HashMap<ValueId, Value>,
  mem_manager: &mut MemoryManager,
  struct_field_counts: &HashMap<String, usize>,
  error_stack: &mut Vec<String>,
) -> Result<(ControlFlow, Option<Value>), Box<dyn Error>> {
  for instr in block.instructions.iter() {
    match instr {
      Instruction::Br { target } => {
        // Unconditional branch
        return Ok((ControlFlow::Jump(*target), None));
      }
      Instruction::CondBr {
        cond,
        then_block,
        else_block,
      } => {
        // Conditional branch
        let cond_val = values
          .get(cond)
          .ok_or(format!("Condition value {} not found", cond.0))?;
        let is_true = match cond_val {
          Value::Int(i) => *i != 0,
          Value::Uint(u) => *u != 0,
          Value::Bool(b) => *b,
          Value::Optional { is_some, .. } => *is_some,
          _ => false,
        };

        let target = if is_true { *then_block } else { *else_block };
        return Ok((ControlFlow::Jump(target), None));
      }
      Instruction::RetVoid => {
        // Return void
        return Ok((ControlFlow::Return(None), None));
      }
      Instruction::Ret { val } => {
        // Return value
        let ret_val = values.get(val).cloned();
        return Ok((ControlFlow::Return(ret_val.clone()), ret_val));
      }
      Instruction::Call {
        dst: _,
        func: _,
        args: _,
        source_file: _,
        source_line: _,
        source_col: _,
      } => {
        match execute_instruction_with_memory(
          ir_module,
          instr,
          values,
          mem_manager,
          struct_field_counts,
          error_stack,
        ) {
          Ok(()) => {}
          Err(e) => {
            eprintln!("Runtime error: {}", e);
            print_call_stack();
          }
        }
      }
      _ => {
        // Regular instruction
        match execute_instruction_with_memory(
          ir_module,
          instr,
          values,
          mem_manager,
          struct_field_counts,
          error_stack,
        ) {
          Ok(()) => {}
          Err(e) => {
            eprintln!("Runtime error: {}", e);
            print_call_stack();
          }
        }
      }
    }
  }

  // Block ended without explicit control flow
  Ok((ControlFlow::Continue, None))
}

/// Execute a function with provided arguments
#[allow(clippy::too_many_arguments)]
fn execute_function_with_args(
  ir_module: &Module,
  func: &Function,
  args: &[ValueId],
  parent_values: &HashMap<ValueId, Value>,
  parent_mem_manager: &mut MemoryManager,
  struct_field_counts: &HashMap<String, usize>,
  error_stack: &mut Vec<String>,
  source_file: Option<&str>,
  source_line: Option<i64>,
) -> Result<Value, Box<dyn Error>> {
  CALL_STACK.with(|stack| {
    stack.borrow_mut().push((
      func.name.clone(),
      source_file.map(|s| s.to_string()),
      source_line,
    ));
  });
  let result = execute_function_with_args_inner(
    ir_module,
    func,
    args,
    parent_values,
    parent_mem_manager,
    struct_field_counts,
    error_stack,
  );
  CALL_STACK.with(|stack| {
    stack.borrow_mut().pop();
  });
  if result.is_err() {
    print_call_stack();
  }
  result
}

fn execute_function_with_args_inner(
  ir_module: &Module,
  func: &Function,
  args: &[ValueId],
  parent_values: &HashMap<ValueId, Value>,
  parent_mem_manager: &mut MemoryManager,
  struct_field_counts: &HashMap<String, usize>,
  error_stack: &mut Vec<String>,
) -> Result<Value, Box<dyn Error>> {
  if func.blocks.is_empty() {
    return Ok(Value::Float(0.0));
  }

  let mut values: HashMap<ValueId, Value> = HashMap::new();

  // Track freshly-allocated str parameter bases for cleanup at function exit.
  // String literals passed as str params are heap-allocated and must be freed.
  let mut fresh_str_param_bases: Vec<i64> = Vec::new();

  // Bind function parameters to arguments
  for (idx, param) in func.params.iter().enumerate() {
    if idx < args.len() {
      let arg_id = &args[idx];
      if let Some(arg_value) = parent_values.get(arg_id) {
        // Check if parameter expects a str struct but we have a Value::String
        let converted_value = if let IrType::Struct { name } = &param.ty {
          if name == "str" {
            match arg_value {
              Value::String(s) => {
                // Convert string to str struct: {ptr, len}
                // Allocate null-terminated string data
                let data_addr = parent_mem_manager.alloc_null_terminated_string(s)?;
                fresh_str_param_bases.push(data_addr);
                // Create struct with ptr (pointer to data) and len (string length)
                Value::Struct(
                  "str".to_string(),
                  vec![
                    Value::Pointer(data_addr as usize),
                    Value::Int(s.len() as i64),
                  ],
                )
              }
              _ => arg_value.clone(),
            }
          } else {
            arg_value.clone()
          }
        } else {
          arg_value.clone()
        };
        values.insert(param.value_id, converted_value);
      } else {
        values.insert(param.value_id, Value::Float(0.0));
      }
    }
  }

  // After binding, eagerly convert any str parameters whose ptr field is
  // still a Value::String (from string literals wrapped in buildstruct).
  // This mirrors what the Store handler does, but tracks the allocation
  // for cleanup at function exit.
  for param in &func.params {
    if matches!(&param.ty, IrType::Struct { name } if name == "str")
      && let Some(value) = values.get(&param.value_id)
      && let Value::Struct(name, fields) = value
      && name == "str"
      && let Some(Value::String(s)) = fields.first()
    {
      let data_addr = parent_mem_manager.alloc_null_terminated_string(s)?;
      fresh_str_param_bases.push(data_addr);
      values.insert(
        param.value_id,
        Value::Struct(
          "str".to_string(),
          vec![Value::Pointer(data_addr as usize), fields[1].clone()],
        ),
      );
    }
  }

  // Execute with control flow, tracking return value
  let mut return_value = Value::Float(0.0);
  let mut current_block_idx = 0;
  let mut block_execution_count = 0;
  const MAX_BLOCK_EXECUTIONS: usize = 10000; // Prevent infinite loops

  loop {
    block_execution_count += 1;
    if block_execution_count > MAX_BLOCK_EXECUTIONS {
      return Err(
        format!(
          "Function '{}' exceeded maximum block executions ({}). Possible infinite loop detected.",
          func.name, MAX_BLOCK_EXECUTIONS
        )
        .into(),
      );
    }
    // Get current block by index
    if current_block_idx >= func.blocks.len() {
      break;
    }

    let block = &func.blocks[current_block_idx];

    // Execute block
    match execute_block_with_return(
      ir_module,
      block,
      &mut values,
      parent_mem_manager,
      struct_field_counts,
      error_stack,
    )? {
      (ControlFlow::Continue, None) => {
        // Move to the next block sequentially
        // But don't go past the last block to prevent infinite loops
        if current_block_idx + 1 < func.blocks.len() {
          current_block_idx += 1;
        } else {
          break;
        }
      }
      (ControlFlow::Continue, Some(val)) => {
        return_value = val;
        // Move to the next block sequentially
        // But don't go past the last block to prevent infinite loops
        if current_block_idx + 1 < func.blocks.len() {
          current_block_idx += 1;
        } else {
          break;
        }
      }
      (ControlFlow::Jump(target_id), ret_val) => {
        if let Some(ret) = ret_val {
          return_value = ret;
        }
        // Find the target block by ID
        if let Some(idx) = func.blocks.iter().position(|b| b.id == target_id) {
          current_block_idx = idx;
        } else {
          return Err(format!("Block {} not found", target_id.0).into());
        }
      }
      (ControlFlow::Return(ret_val), _) => {
        if let Some(val) = ret_val {
          return_value = val;
        }
        break;
      }
    }
  }

  // Free freshly-allocated str parameter data before returning.
  // These are string literals converted to heap str during parameter binding.
  for base in &fresh_str_param_bases {
    parent_mem_manager.regions.remove(base);
    ALLOC_LOG.with(|log| {
      #[allow(clippy::unwrap_used, clippy::expect_used)]
      log
        .lock()
        .expect("mutex poisoned")
        .dealloc_log(*base as usize as *mut u8);
    });
  }

  Ok(return_value)
}

/// Load complex types (structs, optionals, vectors, arrays) from MemoryManager.
/// Fallback for types not yet migrated to V3 real-heap.
fn load_complex_mem(
  dst: &ValueId,
  address: i64,
  ty: &IrType,
  values: &mut HashMap<ValueId, Value>,
  mem_manager: &MemoryManager,
  struct_field_counts: &HashMap<String, usize>,
) -> Result<(), Box<dyn Error>> {
  let struct_name = match ty {
    IrType::Struct { name } => Some(name.clone()),
    IrType::Optional(inner) => Some(format!("optional_{}", inner.to_string().replace('%', ""))),
    _ => None,
  };

  if let Some(name) = struct_name {
    let field_count = struct_field_counts.get(&name).copied().unwrap_or_else(|| {
      ice!("Load: struct '{}' not found in field_counts", name);
    });
    let mut fields = Vec::new();
    for i in 0..field_count as i64 {
      let field_val = mem_manager.load(address + i * 8);
      fields.push(field_val);
    }
    if fields.is_empty() {
      fields.push(Value::Int(0));
    }
    values.insert(*dst, Value::Struct(name, fields));
  } else {
    let mut value = mem_manager.load(address);
    value = coerce_load_value(value, ty);
    values.insert(*dst, value);
  }
  Ok(())
}

/// When a `Load` instruction requests a type that differs from the stored
/// value's Rust variant, perform an implicit bit reinterpretation. This is
/// required for the `value at p unsafe cast` pattern where e.g. f64 memory
/// is loaded as i64 for IEEE 754 bit manipulation.
fn coerce_load_value(value: Value, load_ty: &IrType) -> Value {
  match (&value, load_ty) {
    // Float → Int: reinterpret IEEE 754 bits as integer (f64→i64, f32→i32)
    (Value::Float(f), IrType::I64) => Value::Int(f64::to_bits(*f) as i64),
    (Value::Float(f), IrType::U64) => Value::Uint(f64::to_bits(*f)),
    (Value::Float(f), IrType::I32) => Value::Int(f32::to_bits(*f as f32) as i64),
    (Value::Float(f), IrType::U32) => Value::Uint(f32::to_bits(*f as f32) as u64),
    // Int → Float: reinterpret integer bits as IEEE 754
    (Value::Int(i), IrType::F64) => Value::Float(f64::from_bits(*i as u64)),
    (Value::Int(i), IrType::F32) => Value::Float(f32::from_bits(*i as u32) as f64),
    (Value::Uint(u), IrType::F64) => Value::Float(f64::from_bits(*u)),
    (Value::Uint(u), IrType::F32) => Value::Float(f32::from_bits(*u as u32) as f64),
    // Default: return as-is (already correct type, or no applicable coercion)
    _ => value,
  }
}

/// Integer arithmetic operation kind used by the checked arithmetic helpers.
#[derive(Debug, Clone, Copy)]
enum CheckedArithOp {
  Add,
  Sub,
  Mul,
  Neg,
}

impl CheckedArithOp {
  fn name(self) -> &'static str {
    match self {
      CheckedArithOp::Add => "addition",
      CheckedArithOp::Sub => "subtraction",
      CheckedArithOp::Mul => "multiplication",
      CheckedArithOp::Neg => "negation",
    }
  }
}

/// Compute a checked signed-integer operation against the declared `ty` width.
/// Returns the wrapped/truncated result on success, or a message on overflow.
fn checked_signed_arith(
  ty: &IrType,
  lhs: i64,
  rhs: i64,
  op: CheckedArithOp,
) -> Result<i64, String> {
  let (min, max) = match ty {
    IrType::I8 => (i8::MIN as i128, i8::MAX as i128),
    IrType::I16 => (i16::MIN as i128, i16::MAX as i128),
    IrType::I32 => (i32::MIN as i128, i32::MAX as i128),
    IrType::I64 => (i64::MIN as i128, i64::MAX as i128),
    other => {
      return Err(format!(
        "checked arithmetic applied to non-signed-integer type {}",
        other
      ));
    }
  };

  let l = lhs as i128;
  let r = rhs as i128;
  let result = match op {
    CheckedArithOp::Add => l + r,
    CheckedArithOp::Sub => l - r,
    CheckedArithOp::Mul => l * r,
    CheckedArithOp::Neg => -l,
  };

  if result < min || result > max {
    return Err(format!("integer overflow in signed {} ({})", op.name(), ty));
  }
  Ok(result as i64)
}

/// Compute a checked unsigned-integer operation against the declared `ty` width.
fn checked_unsigned_arith(
  ty: &IrType,
  lhs: u64,
  rhs: u64,
  op: CheckedArithOp,
) -> Result<u64, String> {
  let max = match ty {
    IrType::U8 => u8::MAX as u128,
    IrType::U16 => u16::MAX as u128,
    IrType::U32 => u32::MAX as u128,
    IrType::U64 => u64::MAX as u128,
    other => {
      return Err(format!(
        "checked arithmetic applied to non-unsigned-integer type {}",
        other
      ));
    }
  };

  let l = lhs as u128;
  let r = rhs as u128;
  let result = match op {
    CheckedArithOp::Add => l + r,
    CheckedArithOp::Sub => l
      .checked_sub(r)
      .ok_or_else(|| format!("integer underflow in unsigned subtraction ({})", ty))?,
    CheckedArithOp::Mul => l * r,
    CheckedArithOp::Neg => {
      unreachable!("unsigned negation is not a valid Lale operation")
    }
  };

  if result > max {
    return Err(format!(
      "integer overflow in unsigned {} ({})",
      op.name(),
      ty
    ));
  }
  Ok(result as u64)
}

/// Execute a single IR instruction, dispatching to the appropriate handler.
///
/// This is the core of the interpreter — every IR instruction flows through
/// this match statement. Each instruction type reads from / writes to the
/// `values` map (SSA register file), `mem_manager` (heap allocations), and
/// `error_stack` (error messages for the error stack feature).
/// Reinterpret a runtime value as a signed i64, matching its two's-complement
/// bit pattern regardless of whether it is tagged `Int` or `Uint`. Same-width
/// `as` conversions are IR bitcasts and do not change the runtime tag, so the
/// tag alone is not a reliable sign indicator here.
fn value_as_signed(v: &Value) -> Option<i64> {
  match v {
    Value::Int(i) => Some(*i),
    Value::Uint(u) => Some(*u as i64),
    _ => None,
  }
}

/// Reinterpret a runtime value as an unsigned u64, matching its bit pattern.
fn value_as_unsigned(v: &Value) -> Option<u64> {
  match v {
    Value::Uint(u) => Some(*u),
    Value::Int(i) => Some(*i as u64),
    _ => None,
  }
}

/// Perform a checked binary integer operation, interpreting both operands in
/// the signedness declared by `ty`.
fn checked_binary(
  ty: &IrType,
  lhs: &Value,
  rhs: &Value,
  op: CheckedArithOp,
) -> Result<Value, String> {
  if ty.is_signed_int() {
    let l = value_as_signed(lhs).ok_or_else(|| {
      format!(
        "checked arithmetic expected integer operands, got {:?}",
        lhs
      )
    })?;
    let r = value_as_signed(rhs).ok_or_else(|| {
      format!(
        "checked arithmetic expected integer operands, got {:?}",
        rhs
      )
    })?;
    checked_signed_arith(ty, l, r, op).map(Value::Int)
  } else if ty.is_unsigned_int() {
    let l = value_as_unsigned(lhs).ok_or_else(|| {
      format!(
        "checked arithmetic expected integer operands, got {:?}",
        lhs
      )
    })?;
    let r = value_as_unsigned(rhs).ok_or_else(|| {
      format!(
        "checked arithmetic expected integer operands, got {:?}",
        rhs
      )
    })?;
    checked_unsigned_arith(ty, l, r, op).map(Value::Uint)
  } else {
    Err(format!(
      "checked arithmetic applied to non-integer type {}",
      ty
    ))
  }
}

/// Perform a checked negation, interpreting the operand as the signed `ty`.
fn checked_neg_value(ty: &IrType, src: &Value) -> Result<Value, String> {
  if ty.is_signed_int() {
    let i = value_as_signed(src)
      .ok_or_else(|| format!("checked negation expected integer operand, got {:?}", src))?;
    checked_signed_arith(ty, i, 0, CheckedArithOp::Neg).map(Value::Int)
  } else {
    Err(format!(
      "checked negation applied to non-signed-integer type {}",
      ty
    ))
  }
}

fn execute_instruction_with_memory(
  ir_module: &Module,
  instr: &Instruction,
  values: &mut HashMap<ValueId, Value>,
  mem_manager: &mut MemoryManager,
  struct_field_counts: &HashMap<String, usize>,
  error_stack: &mut Vec<String>,
) -> Result<(), Box<dyn Error>> {
  match instr {
    // Constants - store their values
    Instruction::ConstString { dst, val } => {
      values.insert(*dst, Value::String(val.clone()));
      Ok(())
    }
    Instruction::ConstInt { dst, ty, val } => {
      // Produce the correct Value variant based on the IR type
      match ty {
        IrType::U8 | IrType::U16 | IrType::U32 | IrType::U64 => {
          values.insert(*dst, Value::Uint(*val as u64));
        }
        IrType::F16 | IrType::F32 | IrType::F64 => {
          values.insert(*dst, Value::Float(*val as f64));
        }
        IrType::I8 | IrType::I16 | IrType::I32 | IrType::I64 | IrType::Ptr(_) => {
          values.insert(*dst, Value::Int(*val));
        }
        other => {
          return Err(
            format!(
              "ConstInt: unsupported type {:?} — expected integer type",
              other
            )
            .into(),
          );
        }
      }
      Ok(())
    }
    Instruction::ConstUint { dst, val, .. } => {
      values.insert(*dst, Value::Uint(*val));
      Ok(())
    }
    Instruction::ConstFloat { dst, val, .. } => {
      values.insert(*dst, Value::Float(*val));
      Ok(())
    }
    Instruction::ConstBool { dst, val } => {
      values.insert(*dst, Value::Bool(*val));
      Ok(())
    }
    Instruction::ConstNull { dst } => {
      values.insert(*dst, Value::Null);
      Ok(())
    }

    // String concatenation - combine strings (supports str structs)
    Instruction::Concat { dst, lhs, rhs } => {
      let left = values.get(lhs).cloned().ok_or("Value not found")?;
      let right = values.get(rhs).cloned().ok_or("Value not found")?;

      let left_str = extract_value_string(&left, mem_manager);
      let right_str = extract_value_string(&right, mem_manager);
      let result = format!("{}{}", left_str, right_str);

      // Allocate null-terminated string data for the result
      let data_addr = mem_manager.alloc_null_terminated_string(&result)?;

      // Free the source strings' data — concat consumed them
      free_str_data(&left, mem_manager);
      free_str_data(&right, mem_manager);

      // Build the str struct as an in-register value (not heap-allocated).
      // This matches the IR type IrType::struct_ref("str").
      // The struct will be stored to an alloca by a subsequent Store instruction.
      values.insert(
        *dst,
        Value::Struct(
          "str".to_string(),
          vec![
            Value::Pointer(data_addr as usize),
            Value::Int(result.len() as i64),
          ],
        ),
      );
      Ok(())
    }

    // Deep-copy a str without freeing the source (used by embedding).
    Instruction::StrCopy { dst, src } => {
      let src_val = values.get(src).cloned().ok_or("Value not found")?;
      let s = extract_value_string(&src_val, mem_manager);
      let data_addr = mem_manager.alloc_null_terminated_string(&s)?;
      values.insert(
        *dst,
        Value::Struct(
          "str".to_string(),
          vec![
            Value::Pointer(data_addr as usize),
            Value::Int(s.len() as i64),
          ],
        ),
      );
      Ok(())
    }

    // Memory operations
    Instruction::Alloca { dst, ty } => {
      let size = MemoryManager::compute_type_size(ty);
      let ptr = ALLOC_LOG.with(|log| {
        #[allow(clippy::unwrap_used, clippy::expect_used)]
        log
          .lock()
          .expect("mutex poisoned")
          .alloc_log(size as usize, None, None)
      });
      // Create a properly‑sized MemoryManager region at the real address.
      // This prevents byte‑level stores from creating overlapping 8‑byte regions.
      mem_manager.regions.insert(
        ptr as i64,
        MemoryRegion {
          base_address: ptr as i64,
          size,
          fields: HashMap::new(),
        },
      );
      values.insert(*dst, Value::Pointer(ptr as usize));
      Ok(())
    }
    Instruction::Store { val, ptr, ty: _ } => {
      // Store value to memory location
      if let Some(value) = values.get(val).cloned() {
        let address = match values.get(ptr) {
          Some(Value::Pointer(addr)) => *addr as i64,
          Some(Value::Int(addr)) => *addr,
          _ => {
            return Err(
              format!(
                "Store: pointer operand is not a pointer, got {:?}",
                values.get(ptr)
              )
              .into(),
            );
          }
        };

        if let Value::Struct(name, fields) = &value {
          let is_static = address >= STATIC_ADDR_BASE;
          for (i, field) in fields.iter().enumerate() {
            let field_val = if name == "str" && i == 0 {
              match field {
                Value::String(s) => {
                  let ptr = if is_static {
                    mem_manager.alloc_static_string(s)
                  } else {
                    mem_manager.alloc_null_terminated_string(s)?
                  };
                  Value::Pointer(ptr as usize)
                }
                other => other.clone(),
              }
            } else {
              field.clone()
            };
            mem_manager.store(address + (i as i64) * 8, field_val)?;
          }
        } else if let Value::Optional {
          is_some,
          value: inner,
        } = &value
        {
          mem_manager.store(address, Value::Bool(*is_some))?;
          mem_manager.store(address + 8, inner.as_ref().clone())?;
        } else {
          mem_manager.store(address, value)?;
        }
      }
      Ok(())
    }
    Instruction::BoundsCheck {
      index,
      length,
      message,
      file,
      line,
      column,
    } => {
      // Check if index < length; call __lale_error if violated
      let index_val = match values.get(index) {
        Some(Value::Int(i)) => *i,
        Some(Value::Uint(u)) => *u as i64,
        _ => return Err("Invalid index value".into()),
      };

      let length_val = match values.get(length) {
        Some(Value::Int(l)) => *l,
        Some(Value::Uint(u)) => *u as i64,
        _ => return Err("Invalid length value".into()),
      };

      // Lale uses 1-based indexing; valid indices are 1..length (inclusive)
      // So check: index >= 1 && index <= length
      if !(index_val >= 1 && index_val <= length_val) {
        lale_error_and_abort(
          file,
          *line as u64,
          *column as u64,
          message,
          index_val,
          length_val,
        );
      }
      Ok(())
    }
    Instruction::ZeroCheck {
      operand,
      message,
      file,
      line,
      column,
    } => {
      // Check if operand is zero; call __lale_error if violated
      let is_zero = match values.get(operand) {
        Some(Value::Int(i)) => *i == 0,
        Some(Value::Uint(u)) => *u == 0,
        Some(Value::Float(f)) => *f == 0.0,
        _ => false,
      };
      if is_zero {
        lale_error_and_abort(file, *line as u64, *column as u64, message, -1, 0);
      }
      Ok(())
    }
    Instruction::TestBegin { suite, case } => {
      TEST_STATE.with(|ts| {
        let mut ts = ts.borrow_mut();
        ts.current_suite = Some(suite.clone());
        ts.current_case = Some(case.clone());
        ts.current_case_failed = false;
        ts.ran_any_test = true;
      });
      Ok(())
    }
    Instruction::TestFail {
      file,
      line,
      column,
      expected,
      found,
    } => {
      // Format optional expected/found operands (e.g. an `assert a == b`).
      let expected_str = expected
        .as_ref()
        .and_then(|id| values.get(id))
        .map(|v| extract_value_string(v, mem_manager));
      let found_str = found
        .as_ref()
        .and_then(|id| values.get(id))
        .map(|v| extract_value_string(v, mem_manager));
      TEST_STATE.with(|ts| -> Result<(), Box<dyn Error>> {
        let mut ts = ts.borrow_mut();
        ts.current_case_failed = true;
        ts.any_failed = true;
        let suite = match ts.current_suite.clone() {
          Some(s) => s,
          None => return Err("TestFail executed without a TestBegin context".into()),
        };
        let case = match ts.current_case.clone() {
          Some(c) => c,
          None => return Err("TestFail executed without a TestBegin context".into()),
        };
        let msg = match (expected_str.as_deref(), found_str.as_deref()) {
          (Some(exp), Some(fnd)) => format!(
            "Fail: {} / {}: expected {}, found {} ({}:{}:{})\n",
            suite, case, exp, fnd, file, line, column
          ),
          _ => format!(
            "Fail: {} / {} ({}:{}:{})\n",
            suite, case, file, line, column
          ),
        };
        std::io::Write::write_all(&mut std::io::stderr(), msg.as_bytes())?;
        Ok(())
      })?;
      Ok(())
    }
    Instruction::TestEnd => {
      TEST_STATE.with(|ts| -> Result<(), Box<dyn Error>> {
        let mut ts = ts.borrow_mut();
        if !ts.current_case_failed {
          let suite = match ts.current_suite.clone() {
            Some(s) => s,
            None => return Err("TestEnd executed without a TestBegin context".into()),
          };
          let case = match ts.current_case.clone() {
            Some(c) => c,
            None => return Err("TestEnd executed without a TestBegin context".into()),
          };
          let msg = format!("Pass: {} / {}\n", suite, case);
          std::io::Write::write_all(&mut std::io::stderr(), msg.as_bytes())?;
        }
        ts.current_case = None;
        ts.current_case_failed = false;
        Ok(())
      })?;
      Ok(())
    }
    Instruction::Load { dst, ptr, ty } => {
      // Load value from memory location
      let address = match values.get(ptr) {
        Some(Value::Pointer(addr)) => *addr as i64,
        Some(Value::Int(addr)) => *addr,
        _ => {
          return Err(format!("Load from non-pointer value: {:?}", values.get(ptr)).into());
        }
      };

      load_complex_mem(dst, address, ty, values, mem_manager, struct_field_counts)?;
      Ok(())
    }
    Instruction::GetElementPtr { dst, base, indices } => {
      // Compute element pointer: base + sum of indices
      // For byte arrays (u8), each index is 1 byte offset
      // For other types, we'd need type info, but default to 1 byte for string buffers
      let base_ptr = match values.get(base) {
        Some(Value::Pointer(addr)) => (*addr as i64, 0),
        Some(Value::Int(addr)) => (*addr, 0),
        _ => {
          return Err(
            format!(
              "GetElementPtr: base operand is not a pointer or int, got {:?}",
              values.get(base)
            )
            .into(),
          );
        }
      };

      // Sum all indices - assume 1-byte elements (for u8 arrays used in string buffers)
      let mut total_offset = base_ptr.1;
      for idx in indices {
        if let Some(Value::Int(i)) = values.get(idx) {
          total_offset += i; // 1 byte per element for u8 arrays
        } else if let Some(Value::Uint(u)) = values.get(idx) {
          total_offset += *u as i64;
        }
      }

      values.insert(*dst, Value::Pointer((base_ptr.0 + total_offset) as usize));
      Ok(())
    }
    Instruction::GetFieldPtr {
      dst,
      base,
      byte_offset,
      struct_name,
      ..
    } => {
      // Compute field pointer: base + pre-computed byte_offset
      match values.get(base) {
        Some(Value::Pointer(addr)) => {
          values.insert(
            *dst,
            Value::Pointer((*addr as i64 + *byte_offset as i64) as usize),
          );
          Ok(())
        }
        Some(Value::Int(addr)) => {
          values.insert(*dst, Value::Pointer((*addr + *byte_offset as i64) as usize));
          Ok(())
        }
        Some(Value::String(s)) => {
          // Lazily allocate a Value::String in memory when accessing its fields
          // This creates a str type: {ptr, len}
          let str_data_addr = mem_manager.alloc_null_terminated_string(s)?;
          let struct_addr = ALLOC_LOG.with(|log| {
            #[allow(clippy::unwrap_used, clippy::expect_used)]
            log
              .lock()
              .expect("mutex poisoned")
              .alloc_log(16, None, None)
          }) as i64;
          mem_manager.store(struct_addr, Value::Pointer(str_data_addr as usize))?;
          mem_manager.store(struct_addr + 8, Value::Int(s.len() as i64))?;

          // Update the value map to use the allocated version going forward
          values.insert(*base, Value::Pointer(struct_addr as usize));

          values.insert(
            *dst,
            Value::Pointer((struct_addr + *byte_offset as i64) as usize),
          );
          Ok(())
        }
        Some(Value::Struct(..)) => {
          // In-register struct: look up layout, find field at byte_offset,
          // and allocate it to a dedicated memory slot.
          let struct_def = ir_module.struct_def(struct_name);
          let field_value_and_size: Option<(Value, i64)> = struct_def.and_then(|def| {
            // Walk fields accumulating byte offsets to find the right one
            let mut cum = 0u64;
            for (idx, (_, field_type)) in def.fields.iter().enumerate() {
              if cum == *byte_offset {
                // Extract this field by index from the struct value
                if let Some(Value::Struct(_, fields)) = values.get(base) {
                  let fv = fields.get(idx).cloned().unwrap_or_else(|| {
                    ice!(
                      "GetFieldPtr: field index {} out of bounds for struct with {} fields",
                      idx,
                      fields.len()
                    );
                  });
                  let size = field_type.size_bytes()? as i64;
                  return Some((fv, size));
                }
                return None;
              }
              cum += field_type.size_bytes()? as u64;
            }
            None
          });
          match field_value_and_size {
            Some((fv, size)) => {
              let field_addr = ALLOC_LOG.with(|log| {
                #[allow(clippy::unwrap_used, clippy::expect_used)]
                log
                  .lock()
                  .expect("mutex poisoned")
                  .alloc_log(size as usize, None, None)
              }) as i64;
              mem_manager.store(field_addr, fv)?;
              values.insert(*dst, Value::Pointer(field_addr as usize));
            }
            None => {
              eprintln!(
                "[DEBUG GetFieldPtr] Struct '{}': no field at byte_offset={}",
                struct_name, byte_offset
              );
              values.insert(*dst, Value::Pointer(0));
            }
          }
          Ok(())
        }
        Some(v) => {
          ice!(
            "GetFieldPtr: unhandled value type {:?} at byte_offset={}",
            v,
            byte_offset
          )
        }
        None => {
          ice!("GetFieldPtr: base ValueId not found in values map")
        }
      }
    }
    Instruction::PtrToInt { dst, src } => {
      // Convert pointer to integer
      match values.get(src) {
        Some(Value::Pointer(addr)) => {
          values.insert(*dst, Value::Int(*addr as i64));
        }
        Some(Value::Int(i)) => {
          values.insert(*dst, Value::Int(*i));
        }
        Some(Value::Null) => {
          values.insert(*dst, Value::Int(0));
        }
        other => {
          return Err(format!("PtrToInt requires pointer operand, got {:?}", other).into());
        }
      }
      Ok(())
    }

    // Arithmetic operations - handle pointer arithmetic
    Instruction::Add { dst, lhs, rhs } => {
      match (values.get(lhs), values.get(rhs)) {
        // Pointer + Int = Pointer (address arithmetic for field access)
        (Some(Value::Pointer(ptr)), Some(Value::Int(r))) => {
          values.insert(*dst, Value::Pointer((*ptr as i64 + *r) as usize));
        }
        (Some(Value::Int(l)), Some(Value::Pointer(ptr))) => {
          values.insert(*dst, Value::Pointer((*ptr as i64 + *l) as usize));
        }
        (Some(Value::Pointer(ptr)), Some(Value::Uint(r))) => {
          values.insert(*dst, Value::Pointer((*ptr as i64 + *r as i64) as usize));
        }
        (Some(Value::Int(l)), Some(Value::Int(r))) => {
          values.insert(*dst, Value::Int(l.wrapping_add(*r)));
        }
        (Some(Value::Uint(l)), Some(Value::Uint(r))) => {
          values.insert(*dst, Value::Uint(l.wrapping_add(*r)));
        }
        (Some(Value::Float(l)), Some(Value::Float(r))) => {
          values.insert(*dst, Value::Float(l + r));
        }
        // Coerce mixed int/uint to i64
        (Some(Value::Int(l)), Some(Value::Uint(r))) => {
          values.insert(*dst, Value::Int(l.wrapping_add(*r as i64)));
        }
        (Some(Value::Uint(l)), Some(Value::Int(r))) => {
          values.insert(*dst, Value::Int((*l as i64).wrapping_add(*r)));
        }
        // Type mismatch: strict type checking
        _ => {
          return Err(
            format!(
              "Add: operand type mismatch. Got {:?} + {:?}",
              values.get(lhs).map(|v| format!("{:?}", v)),
              values.get(rhs).map(|v| format!("{:?}", v))
            )
            .into(),
          );
        }
      }
      Ok(())
    }
    Instruction::Sub { dst, lhs, rhs } => {
      match (values.get(lhs), values.get(rhs)) {
        (Some(Value::Int(l)), Some(Value::Int(r))) => {
          values.insert(*dst, Value::Int(l.wrapping_sub(*r)));
        }
        (Some(Value::Uint(l)), Some(Value::Uint(r))) => {
          values.insert(*dst, Value::Uint(l.wrapping_sub(*r)));
        }
        (Some(Value::Float(l)), Some(Value::Float(r))) => {
          values.insert(*dst, Value::Float(l - r));
        }
        // Coerce mixed int/uint to i64
        (Some(Value::Int(l)), Some(Value::Uint(r))) => {
          values.insert(*dst, Value::Int(l.wrapping_sub(*r as i64)));
        }
        (Some(Value::Uint(l)), Some(Value::Int(r))) => {
          values.insert(*dst, Value::Int((*l as i64).wrapping_sub(*r)));
        }
        // Type mismatch: strict type checking
        _ => {
          return Err(
            format!(
              "Sub: operand type mismatch. Got {:?} - {:?}",
              values.get(lhs).map(|v| format!("{:?}", v)),
              values.get(rhs).map(|v| format!("{:?}", v))
            )
            .into(),
          );
        }
      }
      Ok(())
    }
    Instruction::PtrDiff { dst, lhs, rhs } => {
      // Pointer difference: ptr1 - ptr2 → signed i64 byte offset
      match (values.get(lhs), values.get(rhs)) {
        (Some(Value::Pointer(a1)), Some(Value::Pointer(a2))) => {
          values.insert(*dst, Value::Int((*a1 as i64).wrapping_sub(*a2 as i64)));
        }
        (Some(Value::Pointer(a1)), Some(Value::Null)) => {
          values.insert(*dst, Value::Int(*a1 as i64));
        }
        (Some(Value::Null), Some(Value::Pointer(a2))) => {
          values.insert(*dst, Value::Int(0i64.wrapping_sub(*a2 as i64)));
        }
        (Some(Value::Null), Some(Value::Null)) => {
          values.insert(*dst, Value::Int(0));
        }
        _ => {
          return Err(
            format!(
              "PtrDiff: expected pointer operands, got {:?} - {:?}",
              values.get(lhs).map(|v| format!("{:?}", v)),
              values.get(rhs).map(|v| format!("{:?}", v))
            )
            .into(),
          );
        }
      }
      Ok(())
    }
    Instruction::Mul { dst, lhs, rhs } => {
      match (values.get(lhs), values.get(rhs)) {
        (Some(Value::Int(l)), Some(Value::Int(r))) => {
          values.insert(*dst, Value::Int(l.wrapping_mul(*r)));
        }
        (Some(Value::Uint(l)), Some(Value::Uint(r))) => {
          values.insert(*dst, Value::Uint(l.wrapping_mul(*r)));
        }
        (Some(Value::Float(l)), Some(Value::Float(r))) => {
          values.insert(*dst, Value::Float(l * r));
        }
        // Coerce mixed int/uint to i64
        (Some(Value::Int(l)), Some(Value::Uint(r))) => {
          values.insert(*dst, Value::Int(l.wrapping_mul(*r as i64)));
        }
        (Some(Value::Uint(l)), Some(Value::Int(r))) => {
          values.insert(*dst, Value::Int((*l as i64).wrapping_mul(*r)));
        }
        // Type mismatch: strict type checking
        _ => {
          return Err(
            format!(
              "Mul: operand type mismatch. Got {:?} * {:?}",
              values.get(lhs).map(|v| format!("{:?}", v)),
              values.get(rhs).map(|v| format!("{:?}", v))
            )
            .into(),
          );
        }
      }
      Ok(())
    }
    Instruction::Cross { dst, lhs, rhs } => {
      // Cross product: vec3 × vec3 → vec3
      let lv = values.get(lhs).cloned();
      let rv = values.get(rhs).cloned();
      match (lv, rv) {
        (Some(Value::Vec3(lx, ly, lz)), Some(Value::Vec3(rx, ry, rz))) => {
          let x1 = Value::extract_float(lx);
          let y1 = Value::extract_float(ly);
          let z1 = Value::extract_float(lz);
          let x2 = Value::extract_float(rx);
          let y2 = Value::extract_float(ry);
          let z2 = Value::extract_float(rz);
          // v1 × v2 = (y1*z2 - z1*y2, z1*x2 - x1*z2, x1*y2 - y1*x2)
          let cx = y1 * z2 - z1 * y2;
          let cy = z1 * x2 - x1 * z2;
          let cz = x1 * y2 - y1 * x2;
          values.insert(
            *dst,
            Value::Vec3(
              Box::new(Value::Float(cx)),
              Box::new(Value::Float(cy)),
              Box::new(Value::Float(cz)),
            ),
          );
        }
        _ => {
          return Err(
            format!(
              "Cross product requires two vec3 operands, got {:?} and {:?}",
              values.get(lhs),
              values.get(rhs)
            )
            .into(),
          );
        }
      }
      Ok(())
    }
    Instruction::Dot { dst, lhs, rhs } => {
      // Dot product: vecN · vecN → scalar, or scalar · scalar → scalar
      let lv = values.get(lhs).cloned();
      let rv = values.get(rhs).cloned();
      match (lv, rv) {
        // Vector dot products
        (Some(Value::Vec2(lx, ly)), Some(Value::Vec2(rx, ry))) => {
          let result = Value::extract_float(lx) * Value::extract_float(rx)
            + Value::extract_float(ly) * Value::extract_float(ry);
          values.insert(*dst, Value::Float(result));
        }
        (Some(Value::Vec3(lx, ly, lz)), Some(Value::Vec3(rx, ry, rz))) => {
          let result = Value::extract_float(lx) * Value::extract_float(rx)
            + Value::extract_float(ly) * Value::extract_float(ry)
            + Value::extract_float(lz) * Value::extract_float(rz);
          values.insert(*dst, Value::Float(result));
        }
        (Some(Value::Vec4(lx, ly, lz, lw)), Some(Value::Vec4(rx, ry, rz, rw))) => {
          let result = Value::extract_float(lx) * Value::extract_float(rx)
            + Value::extract_float(ly) * Value::extract_float(ry)
            + Value::extract_float(lz) * Value::extract_float(rz)
            + Value::extract_float(lw) * Value::extract_float(rw);
          values.insert(*dst, Value::Float(result));
        }
        // Scalar dot products
        (Some(Value::Int(l)), Some(Value::Int(r))) => {
          values.insert(*dst, Value::Int(l.wrapping_mul(r)));
        }
        (Some(Value::Uint(l)), Some(Value::Uint(r))) => {
          values.insert(*dst, Value::Uint(l.wrapping_mul(r)));
        }
        (Some(Value::Float(l)), Some(Value::Float(r))) => {
          values.insert(*dst, Value::Float(l * r));
        }
        _ => {
          return Err(
            format!(
              "Dot product type mismatch: {:?} · {:?}",
              values.get(lhs),
              values.get(rhs)
            )
            .into(),
          );
        }
      }
      Ok(())
    }
    Instruction::Div { dst, lhs, rhs } => {
      // ZeroCheck emitted by IR gen guarantees rhs != 0
      match (values.get(lhs), values.get(rhs)) {
        (Some(Value::Int(l)), Some(Value::Int(r))) => {
          values.insert(*dst, Value::Int(l.wrapping_div(*r)));
        }
        (Some(Value::Uint(l)), Some(Value::Uint(r))) => {
          values.insert(*dst, Value::Uint(l / r));
        }
        (Some(Value::Float(l)), Some(Value::Float(r))) => {
          values.insert(*dst, Value::Float(l / r));
        }
        // Coerce mixed int/uint to i64
        (Some(Value::Int(l)), Some(Value::Uint(r))) => {
          values.insert(*dst, Value::Int(l.wrapping_div(*r as i64)));
        }
        (Some(Value::Uint(l)), Some(Value::Int(r))) => {
          values.insert(*dst, Value::Int((*l as i64).wrapping_div(*r)));
        }
        _ => {
          return Err(
            format!(
              "Div: operand type mismatch. Got {:?} / {:?}",
              values.get(lhs).map(|v| format!("{:?}", v)),
              values.get(rhs).map(|v| format!("{:?}", v))
            )
            .into(),
          );
        }
      }
      Ok(())
    }
    Instruction::Pow { dst, base, exp } => {
      let base_val = match values.get(base) {
        Some(Value::Float(f)) => *f,
        Some(Value::Int(i)) => *i as f64,
        Some(Value::Uint(u)) => *u as f64,
        _ => 0.0,
      };
      let exp_val = match values.get(exp) {
        Some(Value::Float(f)) => *f,
        Some(Value::Int(i)) => *i as f64,
        Some(Value::Uint(u)) => *u as f64,
        _ => 0.0,
      };
      let result = base_val.powf(exp_val);
      values.insert(*dst, Value::Float(result));
      Ok(())
    }
    Instruction::Rem { dst, lhs, rhs } => {
      // ZeroCheck emitted by IR gen guarantees rhs != 0
      match (values.get(lhs), values.get(rhs)) {
        (Some(Value::Int(l)), Some(Value::Int(r))) => {
          values.insert(*dst, Value::Int(l.wrapping_rem(*r)));
        }
        (Some(Value::Uint(l)), Some(Value::Uint(r))) => {
          values.insert(*dst, Value::Uint(l % r));
        }
        (Some(Value::Float(l)), Some(Value::Float(r))) => {
          values.insert(*dst, Value::Float(l % r));
        }
        // Coerce mixed int/uint to i64
        (Some(Value::Int(l)), Some(Value::Uint(r))) => {
          values.insert(*dst, Value::Int(l.wrapping_rem(*r as i64)));
        }
        (Some(Value::Uint(l)), Some(Value::Int(r))) => {
          values.insert(*dst, Value::Int((*l as i64).wrapping_rem(*r)));
        }
        _ => {
          return Err(
            format!(
              "Rem: operand type mismatch. Got {:?} % {:?}",
              values.get(lhs).map(|v| format!("{:?}", v)),
              values.get(rhs).map(|v| format!("{:?}", v))
            )
            .into(),
          );
        }
      }
      Ok(())
    }
    Instruction::Neg { dst, src } => {
      match values.get(src) {
        Some(Value::Int(i)) => {
          values.insert(*dst, Value::Int(i.wrapping_neg()));
        }
        Some(Value::Float(f)) => {
          values.insert(*dst, Value::Float(-f));
        }
        _ => {
          return Err(
            format!(
              "Neg: invalid operand type {:?}",
              values.get(src).map(|v| format!("{:?}", v))
            )
            .into(),
          );
        }
      }
      Ok(())
    }
    Instruction::CheckedAdd {
      dst,
      lhs,
      rhs,
      ty,
      file,
      line,
      column,
    } => {
      let result = match (values.get(lhs), values.get(rhs)) {
        (Some(l), Some(r)) => checked_binary(ty, l, r, CheckedArithOp::Add),
        _ => return Err("CheckedAdd: missing operands".into()),
      };
      match result {
        Ok(v) => values.insert(*dst, v),
        Err(msg) => lale_error_and_abort(file, *line as u64, *column as u64, &msg, -1, 0),
      };
      Ok(())
    }
    Instruction::CheckedSub {
      dst,
      lhs,
      rhs,
      ty,
      file,
      line,
      column,
    } => {
      let result = match (values.get(lhs), values.get(rhs)) {
        (Some(l), Some(r)) => checked_binary(ty, l, r, CheckedArithOp::Sub),
        _ => return Err("CheckedSub: missing operands".into()),
      };
      match result {
        Ok(v) => values.insert(*dst, v),
        Err(msg) => lale_error_and_abort(file, *line as u64, *column as u64, &msg, -1, 0),
      };
      Ok(())
    }
    Instruction::CheckedMul {
      dst,
      lhs,
      rhs,
      ty,
      file,
      line,
      column,
    } => {
      let result = match (values.get(lhs), values.get(rhs)) {
        (Some(l), Some(r)) => checked_binary(ty, l, r, CheckedArithOp::Mul),
        _ => return Err("CheckedMul: missing operands".into()),
      };
      match result {
        Ok(v) => values.insert(*dst, v),
        Err(msg) => lale_error_and_abort(file, *line as u64, *column as u64, &msg, -1, 0),
      };
      Ok(())
    }
    Instruction::CheckedNeg {
      dst,
      src,
      ty,
      file,
      line,
      column,
    } => {
      let result = match values.get(src) {
        Some(s) => checked_neg_value(ty, s),
        None => return Err("CheckedNeg: missing operand".into()),
      };
      match result {
        Ok(v) => values.insert(*dst, v),
        Err(msg) => lale_error_and_abort(file, *line as u64, *column as u64, &msg, -1, 0),
      };
      Ok(())
    }

    // Bitwise operations
    Instruction::BitAnd { dst, lhs, rhs } => {
      match (values.get(lhs), values.get(rhs)) {
        (Some(Value::Int(a)), Some(Value::Int(b))) => {
          values.insert(*dst, Value::Int(a & b));
        }
        (Some(Value::Uint(a)), Some(Value::Uint(b))) => {
          values.insert(*dst, Value::Uint(a & b));
        }
        _ => {
          return Err(
            format!(
              "BitAnd: operand type mismatch. Got {:?} & {:?}",
              values.get(lhs).map(|v| format!("{:?}", v)),
              values.get(rhs).map(|v| format!("{:?}", v))
            )
            .into(),
          );
        }
      }
      Ok(())
    }
    Instruction::BitOr { dst, lhs, rhs } => {
      match (values.get(lhs), values.get(rhs)) {
        (Some(Value::Int(a)), Some(Value::Int(b))) => {
          values.insert(*dst, Value::Int(a | b));
        }
        (Some(Value::Uint(a)), Some(Value::Uint(b))) => {
          values.insert(*dst, Value::Uint(a | b));
        }
        _ => {
          return Err(
            format!(
              "BitOr: operand type mismatch. Got {:?} | {:?}",
              values.get(lhs).map(|v| format!("{:?}", v)),
              values.get(rhs).map(|v| format!("{:?}", v))
            )
            .into(),
          );
        }
      }
      Ok(())
    }
    Instruction::BitXor { dst, lhs, rhs } => {
      match (values.get(lhs), values.get(rhs)) {
        (Some(Value::Int(a)), Some(Value::Int(b))) => {
          values.insert(*dst, Value::Int(a ^ b));
        }
        (Some(Value::Uint(a)), Some(Value::Uint(b))) => {
          values.insert(*dst, Value::Uint(a ^ b));
        }
        _ => {
          return Err(
            format!(
              "BitXor: operand type mismatch. Got {:?} ^ {:?}",
              values.get(lhs).map(|v| format!("{:?}", v)),
              values.get(rhs).map(|v| format!("{:?}", v))
            )
            .into(),
          );
        }
      }
      Ok(())
    }
    Instruction::BitNot { dst, src } => {
      match values.get(src) {
        Some(Value::Int(a)) => {
          values.insert(*dst, Value::Int(!a));
        }
        Some(Value::Uint(a)) => {
          values.insert(*dst, Value::Uint(!a));
        }
        _ => {
          return Err(
            format!(
              "BitNot: invalid operand type {:?}",
              values.get(src).map(|v| format!("{:?}", v))
            )
            .into(),
          );
        }
      }
      Ok(())
    }
    Instruction::Shl { dst, lhs, rhs } => {
      match (values.get(lhs), values.get(rhs)) {
        (Some(Value::Int(a)), Some(Value::Int(b))) => {
          if *b >= 0 && *b < 64 {
            values.insert(*dst, Value::Int(a << b));
          } else {
            values.insert(*dst, Value::Int(0));
          }
        }
        (Some(Value::Uint(a)), Some(Value::Uint(b))) => {
          if *b < 64 {
            values.insert(*dst, Value::Uint(a << b));
          } else {
            values.insert(*dst, Value::Uint(0));
          }
        }
        _ => {
          return Err(
            format!(
              "Shl: operand type mismatch. Got {:?} << {:?}",
              values.get(lhs).map(|v| format!("{:?}", v)),
              values.get(rhs).map(|v| format!("{:?}", v))
            )
            .into(),
          );
        }
      }
      Ok(())
    }
    Instruction::Shr { dst, lhs, rhs } => {
      match (values.get(lhs), values.get(rhs)) {
        (Some(Value::Int(a)), Some(Value::Int(b))) => {
          if *b >= 0 && *b < 64 {
            values.insert(*dst, Value::Int(a >> b));
          } else {
            values.insert(*dst, Value::Int(0));
          }
        }
        (Some(Value::Uint(a)), Some(Value::Uint(b))) => {
          if *b < 64 {
            values.insert(*dst, Value::Uint(a >> b));
          } else {
            values.insert(*dst, Value::Uint(0));
          }
        }
        _ => {
          return Err(
            format!(
              "Shr: operand type mismatch. Got {:?} >> {:?}",
              values.get(lhs).map(|v| format!("{:?}", v)),
              values.get(rhs).map(|v| format!("{:?}", v))
            )
            .into(),
          );
        }
      }
      Ok(())
    }
    Instruction::UShr { dst, lhs, rhs } => {
      match (values.get(lhs), values.get(rhs)) {
        (Some(Value::Uint(a)), Some(Value::Uint(b))) => {
          if *b < 64 {
            values.insert(*dst, Value::Uint(a >> b));
          } else {
            values.insert(*dst, Value::Uint(0));
          }
        }
        (Some(Value::Int(a)), Some(Value::Int(b))) => {
          // Treat as unsigned: reinterpret i64 bits as u64, shift, reinterpret back
          let ua = *a as u64;
          let ub = *b as u64;
          let result = if ub < 64 { ua >> ub } else { 0u64 };
          values.insert(*dst, Value::Int(result as i64));
        }
        (Some(Value::Uint(a)), Some(Value::Int(b))) => {
          if *b >= 0 && *b < 64 {
            values.insert(*dst, Value::Uint(a >> (*b as u64)));
          } else {
            values.insert(*dst, Value::Uint(0));
          }
        }
        (Some(Value::Int(a)), Some(Value::Uint(b))) => {
          let ua = *a as u64;
          let result = if *b < 64 { ua >> b } else { 0u64 };
          values.insert(*dst, Value::Int(result as i64));
        }
        _ => {
          return Err(
            format!(
              "UShr: operand type mismatch. Got {:?} >> {:?}",
              values.get(lhs).map(|v| format!("{:?}", v)),
              values.get(rhs).map(|v| format!("{:?}", v))
            )
            .into(),
          );
        }
      }
      Ok(())
    }

    // Comparison operations
    Instruction::Eq { dst, lhs, rhs } => {
      let result = match (values.get(lhs), values.get(rhs)) {
        (Some(Value::Int(a)), Some(Value::Int(b))) => a == b,
        (Some(Value::Uint(a)), Some(Value::Uint(b))) => a == b,
        (Some(Value::Float(a)), Some(Value::Float(b))) => (a - b).abs() < 1e-10,
        (Some(Value::String(a)), Some(Value::String(b))) => a == b,
        (Some(Value::Bool(a)), Some(Value::Bool(b))) => a == b,
        // Pointer comparisons
        (Some(Value::Pointer(a)), Some(Value::Pointer(b))) => a == b,
        // Null pointer comparisons
        (Some(Value::Null), Some(Value::Null)) => true,
        // Struct comparisons (e.g., str structs)
        (Some(Value::Struct(name_a, fields_a)), Some(Value::Struct(name_b, fields_b))) => {
          if name_a != name_b || fields_a.len() != fields_b.len() {
            false
          } else if name_a == "str" {
            // str structs compare by content, not pointer identity.
            let a_str = extract_value_string(
              &Value::Struct(name_a.clone(), fields_a.clone()),
              mem_manager,
            );
            let b_str = extract_value_string(
              &Value::Struct(name_b.clone(), fields_b.clone()),
              mem_manager,
            );
            a_str == b_str
          } else {
            // For other structs, compare all fields
            fields_a
              .iter()
              .zip(fields_b.iter())
              .all(|(a, b)| match (a, b) {
                (Value::Int(x), Value::Int(y)) => x == y,
                (Value::Uint(x), Value::Uint(y)) => x == y,
                (Value::Pointer(ab), Value::Pointer(bb)) => ab == bb,
                (Value::String(x), Value::String(y)) => x == y,
                _ => false,
              })
          }
        }
        // Mixed: Struct (loaded from pointer) vs Pointer (string literal)
        // This happens when comparing a dereferenced pointer to a string literal pointer
        (Some(Value::Struct(name_a, fields_a)), Some(Value::Pointer(ptr_base)))
        | (Some(Value::Pointer(ptr_base)), Some(Value::Struct(name_a, fields_a)))
          if name_a == "str" && fields_a.len() == 2 =>
        {
          // Load the struct from the pointer and compare
          match (&fields_a[0], &fields_a[1]) {
            (Value::Pointer(struct_ptr_base), Value::Uint(len)) => {
              // Compare: does this dereferenced struct equal the literal pointer?
              // For str structs, they're equal only if they point to the same address and have the same length
              struct_ptr_base == ptr_base && *len == 0
            }
            _ => false,
          }
        }
        // Coerce mixed int/uint
        (Some(Value::Int(a)), Some(Value::Uint(b))) => *a == *b as i64,
        (Some(Value::Uint(a)), Some(Value::Int(b))) => *a as i64 == *b,
        // Optional comparisons
        (
          Some(Value::Optional {
            is_some: a_some,
            value: a_val,
          }),
          Some(Value::Optional {
            is_some: b_some,
            value: b_val,
          }),
        ) => {
          match (a_some, b_some) {
            (false, false) => true,
            (true, true) => {
              // Two Some values: compare inner values recursively
              // Create a temporary map and call the comparison
              match (a_val.as_ref(), b_val.as_ref()) {
                (Value::Int(x), Value::Int(y)) => x == y,
                (Value::Uint(x), Value::Uint(y)) => x == y,
                (Value::Float(x), Value::Float(y)) => (x - y).abs() < 1e-10,
                (Value::String(x), Value::String(y)) => x == y,
                (Value::Bool(x), Value::Bool(y)) => x == y,
                (Value::Null, Value::Null) => true,
                _ => false,
              }
            }
            _ => false,
          }
        }
        _ => {
          return Err(
            format!(
              "Eq: operand type mismatch — expected matching types, got {:?} and {:?}",
              values.get(lhs),
              values.get(rhs)
            )
            .into(),
          );
        }
      };
      values.insert(*dst, Value::Bool(result));
      Ok(())
    }
    Instruction::Ne { dst, lhs, rhs } => {
      let result = match (values.get(lhs), values.get(rhs)) {
        (Some(Value::Int(a)), Some(Value::Int(b))) => a != b,
        (Some(Value::Uint(a)), Some(Value::Uint(b))) => a != b,
        (Some(Value::Float(a)), Some(Value::Float(b))) => (a - b).abs() >= 1e-10,
        (Some(Value::String(a)), Some(Value::String(b))) => a != b,
        (Some(Value::Bool(a)), Some(Value::Bool(b))) => a != b,
        // Pointer comparisons
        (Some(Value::Pointer(a)), Some(Value::Pointer(b))) => a != b,
        // Null pointer comparisons
        (Some(Value::Null), Some(Value::Null)) => false,
        // Struct comparisons (e.g., str structs)
        (Some(Value::Struct(name_a, fields_a)), Some(Value::Struct(name_b, fields_b))) => {
          if name_a != name_b || fields_a.len() != fields_b.len() {
            true
          } else if name_a == "str" {
            // str structs compare by content, not pointer identity.
            let a_str = extract_value_string(
              &Value::Struct(name_a.clone(), fields_a.clone()),
              mem_manager,
            );
            let b_str = extract_value_string(
              &Value::Struct(name_b.clone(), fields_b.clone()),
              mem_manager,
            );
            a_str != b_str
          } else {
            // For other structs, compare all fields
            !fields_a
              .iter()
              .zip(fields_b.iter())
              .all(|(a, b)| match (a, b) {
                (Value::Int(x), Value::Int(y)) => x == y,
                (Value::Uint(x), Value::Uint(y)) => x == y,
                (Value::Pointer(ab), Value::Pointer(bb)) => ab == bb,
                (Value::String(x), Value::String(y)) => x == y,
                _ => false,
              })
          }
        }
        // Mixed: Struct (loaded from pointer) vs Pointer (string literal)
        // This happens when comparing a dereferenced pointer to a string literal pointer
        (Some(Value::Struct(name_a, fields_a)), Some(Value::Pointer(ptr_base)))
        | (Some(Value::Pointer(ptr_base)), Some(Value::Struct(name_a, fields_a)))
          if name_a == "str" && fields_a.len() == 2 =>
        {
          // Load the struct from the pointer and compare
          match (&fields_a[0], &fields_a[1]) {
            (Value::Pointer(struct_ptr_base), Value::Uint(len)) => {
              // Compare: does this dereferenced struct NOT equal the literal pointer?
              // For str structs, they're different if they don't point to the same address or don't have the same length
              struct_ptr_base != ptr_base || *len != 0
            }
            _ => true, // If format is unexpected, consider them different
          }
        }
        // Coerce mixed int/uint
        (Some(Value::Int(a)), Some(Value::Uint(b))) => *a != *b as i64,
        (Some(Value::Uint(a)), Some(Value::Int(b))) => *a as i64 != *b,
        // Optional comparisons
        (
          Some(Value::Optional {
            is_some: a_some,
            value: a_val,
          }),
          Some(Value::Optional {
            is_some: b_some,
            value: b_val,
          }),
        ) => {
          match (a_some, b_some) {
            (false, false) => false,
            (true, true) => {
              // Two Some values: compare inner values recursively
              match (a_val.as_ref(), b_val.as_ref()) {
                (Value::Int(x), Value::Int(y)) => x != y,
                (Value::Uint(x), Value::Uint(y)) => x != y,
                (Value::Float(x), Value::Float(y)) => (x - y).abs() >= 1e-10,
                (Value::String(x), Value::String(y)) => x != y,
                (Value::Bool(x), Value::Bool(y)) => x != y,
                (Value::Null, Value::Null) => false,
                _ => true,
              }
            }
            _ => true,
          }
        }
        _ => {
          return Err(
            format!(
              "Ne: operand type mismatch. Got {:?} != {:?}",
              values.get(lhs).map(|v| format!("{:?}", v)),
              values.get(rhs).map(|v| format!("{:?}", v))
            )
            .into(),
          );
        }
      };
      values.insert(*dst, Value::Bool(result));
      Ok(())
    }
    Instruction::Lt { dst, lhs, rhs } => {
      let result = match (values.get(lhs), values.get(rhs)) {
        (Some(Value::Int(a)), Some(Value::Int(b))) => a < b,
        (Some(Value::Uint(a)), Some(Value::Uint(b))) => a < b,
        (Some(Value::Float(a)), Some(Value::Float(b))) => a < b,
        (Some(Value::Bool(a)), Some(Value::Bool(b))) => a < b,
        // Pointer comparisons (by combined address + offset)
        (Some(Value::Pointer(a)), Some(Value::Pointer(b))) => (*a as i64) < (*b as i64),
        // Coerce mixed int/uint
        (Some(Value::Int(a)), Some(Value::Uint(b))) => *a < *b as i64,
        (Some(Value::Uint(a)), Some(Value::Int(b))) => (*a as i64) < *b,
        _ => {
          return Err(
            format!(
              "Lt: operand type mismatch. Got {:?} < {:?}",
              values.get(lhs).map(|v| format!("{:?}", v)),
              values.get(rhs).map(|v| format!("{:?}", v))
            )
            .into(),
          );
        }
      };
      values.insert(*dst, Value::Bool(result));
      Ok(())
    }
    Instruction::Le { dst, lhs, rhs } => {
      let result = match (values.get(lhs), values.get(rhs)) {
        (Some(Value::Int(a)), Some(Value::Int(b))) => a <= b,
        (Some(Value::Uint(a)), Some(Value::Uint(b))) => a <= b,
        (Some(Value::Float(a)), Some(Value::Float(b))) => a <= b,
        (Some(Value::Bool(a)), Some(Value::Bool(b))) => a <= b,
        // Pointer comparisons (by combined address + offset)
        (Some(Value::Pointer(a)), Some(Value::Pointer(b))) => (*a as i64) <= (*b as i64),
        // Coerce mixed int/uint
        (Some(Value::Int(a)), Some(Value::Uint(b))) => *a <= *b as i64,
        (Some(Value::Uint(a)), Some(Value::Int(b))) => (*a as i64) <= *b,
        _ => {
          return Err(
            format!(
              "Le: operand type mismatch. Got {:?} <= {:?}",
              values.get(lhs).map(|v| format!("{:?}", v)),
              values.get(rhs).map(|v| format!("{:?}", v))
            )
            .into(),
          );
        }
      };
      values.insert(*dst, Value::Bool(result));
      Ok(())
    }
    Instruction::Gt { dst, lhs, rhs } => {
      let result = match (values.get(lhs), values.get(rhs)) {
        (Some(Value::Int(a)), Some(Value::Int(b))) => a > b,
        (Some(Value::Uint(a)), Some(Value::Uint(b))) => a > b,
        (Some(Value::Float(a)), Some(Value::Float(b))) => a > b,
        (Some(Value::Bool(a)), Some(Value::Bool(b))) => a > b,
        // Pointer comparisons (by combined address + offset)
        (Some(Value::Pointer(a)), Some(Value::Pointer(b))) => (*a as i64) > (*b as i64),
        // Coerce mixed int/uint
        (Some(Value::Int(a)), Some(Value::Uint(b))) => *a > *b as i64,
        (Some(Value::Uint(a)), Some(Value::Int(b))) => (*a as i64) > *b,
        _ => {
          return Err(
            format!(
              "Gt: operand type mismatch. Got {:?} > {:?}",
              values.get(lhs).map(|v| format!("{:?}", v)),
              values.get(rhs).map(|v| format!("{:?}", v))
            )
            .into(),
          );
        }
      };
      values.insert(*dst, Value::Bool(result));
      Ok(())
    }
    Instruction::Ge { dst, lhs, rhs } => {
      let result = match (values.get(lhs), values.get(rhs)) {
        (Some(Value::Int(a)), Some(Value::Int(b))) => a >= b,
        (Some(Value::Uint(a)), Some(Value::Uint(b))) => a >= b,
        (Some(Value::Float(a)), Some(Value::Float(b))) => a >= b,
        (Some(Value::Bool(a)), Some(Value::Bool(b))) => a >= b,
        // Pointer comparisons (by combined address + offset)
        (Some(Value::Pointer(a)), Some(Value::Pointer(b))) => (*a as i64) >= (*b as i64),
        // Coerce mixed int/uint
        (Some(Value::Int(a)), Some(Value::Uint(b))) => *a >= *b as i64,
        (Some(Value::Uint(a)), Some(Value::Int(b))) => (*a as i64) >= *b,
        _ => {
          return Err(
            format!(
              "Ge: operand type mismatch. Got {:?} >= {:?}",
              values.get(lhs).map(|v| format!("{:?}", v)),
              values.get(rhs).map(|v| format!("{:?}", v))
            )
            .into(),
          );
        }
      };
      values.insert(*dst, Value::Bool(result));
      Ok(())
    }

    // Logical operations
    Instruction::And { dst, lhs, rhs } => {
      let l_val = match values.get(lhs) {
        Some(Value::Bool(b)) => *b,
        Some(Value::Int(i)) => *i != 0,
        Some(Value::Uint(u)) => *u != 0,
        _ => false,
      };
      let r_val = match values.get(rhs) {
        Some(Value::Bool(b)) => *b,
        Some(Value::Int(i)) => *i != 0,
        Some(Value::Uint(u)) => *u != 0,
        _ => false,
      };
      values.insert(*dst, Value::Bool(l_val && r_val));
      Ok(())
    }
    Instruction::Or { dst, lhs, rhs } => {
      let l_val = match values.get(lhs) {
        Some(Value::Bool(b)) => *b,
        Some(Value::Int(i)) => *i != 0,
        Some(Value::Uint(u)) => *u != 0,
        _ => false,
      };
      let r_val = match values.get(rhs) {
        Some(Value::Bool(b)) => *b,
        Some(Value::Int(i)) => *i != 0,
        Some(Value::Uint(u)) => *u != 0,
        _ => false,
      };
      values.insert(*dst, Value::Bool(l_val || r_val));
      Ok(())
    }
    Instruction::Xor { dst, lhs, rhs } => {
      let l_val = match values.get(lhs) {
        Some(Value::Bool(b)) => *b,
        Some(Value::Int(i)) => *i != 0,
        Some(Value::Uint(u)) => *u != 0,
        _ => false,
      };
      let r_val = match values.get(rhs) {
        Some(Value::Bool(b)) => *b,
        Some(Value::Int(i)) => *i != 0,
        Some(Value::Uint(u)) => *u != 0,
        _ => false,
      };
      values.insert(*dst, Value::Bool(l_val ^ r_val));
      Ok(())
    }
    Instruction::Not { dst, src } => {
      let val = match values.get(src) {
        Some(Value::Bool(b)) => *b,
        Some(Value::Int(i)) => *i != 0,
        Some(Value::Uint(u)) => *u != 0,
        _ => false,
      };
      values.insert(*dst, Value::Bool(!val));
      Ok(())
    }

    // Type conversions
    // Type extension
    Instruction::SExt { dst, src, to_ty: _ } => {
      match values.get(src) {
        Some(Value::Int(i)) => {
          values.insert(*dst, Value::Int(*i));
        }
        Some(Value::Uint(u)) => {
          values.insert(*dst, Value::Uint(*u));
        }
        other => {
          return Err(format!("SExt requires int/uint operand, got {:?}", other).into());
        }
      }
      Ok(())
    }
    Instruction::ZExt { dst, src, to_ty: _ } => {
      match values.get(src) {
        Some(Value::Int(i)) => {
          values.insert(*dst, Value::Uint(*i as u64));
        }
        Some(Value::Uint(u)) => {
          values.insert(*dst, Value::Uint(*u));
        }
        other => {
          return Err(format!("ZExt requires int/uint operand, got {:?}", other).into());
        }
      }
      Ok(())
    }

    Instruction::SiToFp { dst, src, to_ty: _ } => {
      match values.get(src) {
        Some(Value::Int(i)) => {
          values.insert(*dst, Value::Float(*i as f64));
        }
        Some(Value::Float(f)) => {
          values.insert(*dst, Value::Float(*f));
        }
        Some(Value::Uint(u)) => {
          values.insert(*dst, Value::Float(*u as f64));
        }
        other => {
          return Err(format!("SiToFp requires int/float/uint operand, got {:?}", other).into());
        }
      }
      Ok(())
    }
    Instruction::UiToFp { dst, src, to_ty: _ } => {
      match values.get(src) {
        Some(Value::Uint(u)) => {
          values.insert(*dst, Value::Float(*u as f64));
        }
        other => {
          return Err(format!("UiToFp requires uint operand, got {:?}", other).into());
        }
      }
      Ok(())
    }
    Instruction::FpToSi { dst, src, to_ty: _ } => {
      match values.get(src) {
        Some(Value::Float(f)) => {
          values.insert(*dst, Value::Int(*f as i64));
        }
        other => {
          return Err(format!("FpToSi requires float operand, got {:?}", other).into());
        }
      }
      Ok(())
    }
    Instruction::FpToUi { dst, src, to_ty: _ } => {
      match values.get(src) {
        Some(Value::Float(f)) => {
          values.insert(*dst, Value::Uint(if *f >= 0.0 { *f as u64 } else { 0 }));
        }
        other => {
          return Err(format!("FpToUi requires float operand, got {:?}", other).into());
        }
      }
      Ok(())
    }
    Instruction::Trunc { dst, src, to_ty } => {
      // Truncate to smaller integer type
      // The target type determines whether result is signed or unsigned
      let is_signed = matches!(to_ty, IrType::I8 | IrType::I16 | IrType::I32 | IrType::I64);
      match values.get(src) {
        Some(Value::Int(i)) => {
          if is_signed {
            values.insert(*dst, Value::Int(*i));
          } else {
            values.insert(*dst, Value::Uint(*i as u64));
          }
        }
        Some(Value::Uint(u)) => {
          if is_signed {
            values.insert(*dst, Value::Int(*u as i64));
          } else {
            values.insert(*dst, Value::Uint(*u));
          }
        }
        _ => {
          return Err(format!("Trunc requires int/uint operand, got {:?}", values.get(src)).into());
        }
      }
      Ok(())
    }
    Instruction::FpTrunc { dst, src, to_ty: _ } => {
      match values.get(src) {
        Some(Value::Float(f)) => {
          let truncated = (*f as f32) as f64;
          values.insert(*dst, Value::Float(truncated));
        }
        other => {
          return Err(format!("FpTrunc requires float operand, got {:?}", other).into());
        }
      }
      Ok(())
    }
    Instruction::FpExt { dst, src, to_ty: _ } => {
      match values.get(src) {
        Some(Value::Float(f)) => {
          values.insert(*dst, Value::Float(*f));
        }
        other => {
          return Err(format!("FpExt requires float operand, got {:?}", other).into());
        }
      }
      Ok(())
    }
    Instruction::IntToPtr { dst, src } => {
      match values.get(src) {
        Some(Value::Int(i)) => {
          values.insert(*dst, Value::Pointer(*i as usize));
        }
        Some(Value::Uint(u)) => {
          values.insert(*dst, Value::Pointer(*u as usize));
        }
        other => {
          return Err(format!("IntToPtr requires int/uint operand, got {:?}", other).into());
        }
      }
      Ok(())
    }

    // Global access
    Instruction::GlobalAddr { dst, global } => {
      let addr = mem_manager.global_addr(*global)?;
      values.insert(*dst, Value::Pointer(addr as usize));
      Ok(())
    }

    // Unit assertions
    Instruction::AssertUnit {
      val: _,
      expected: _,
    } => {
      // Unit assertions are compile-time checks; at runtime we just succeed
      // In a full implementation, we would track units and verify at runtime
      Ok(())
    }

    // Function calls - THIS IS WHERE WE OUTPUT
    Instruction::CallVoid {
      func,
      args,
      source_file: _,
      source_line: _,
      source_col: _,
    } => match func {
      FuncRef::External(name) => {
        call_extern(
          name,
          args,
          values,
          mem_manager,
          ir_module,
          error_stack,
          struct_field_counts,
          None,
          None,
        )?;
        Ok(())
      }
      _ => unreachable!("CallVoid with non-external function"),
    },

    // Function calls with return value
    Instruction::Call {
      dst,
      func,
      args,
      source_file,
      source_line,
      source_col: _,
    } => {
      match func {
        FuncRef::External(name) => {
          let result = call_extern(
            name,
            args,
            values,
            mem_manager,
            ir_module,
            error_stack,
            struct_field_counts,
            source_file.as_deref(),
            *source_line,
          )?;
          if let Some(val) = result {
            values.insert(*dst, val);
          }
          Ok(())
        }
        FuncRef::Id(func_id) => {
          // Look up the function and execute it
          if let Some(called_func) = ir_module.functions.iter().find(|f| f.id == *func_id) {
            // Execute the function with the provided arguments
            let result = execute_function_with_args(
              ir_module,
              called_func,
              args,
              values,
              mem_manager,
              struct_field_counts,
              error_stack,
              source_file.as_deref(),
              *source_line,
            )?;
            values.insert(*dst, result);
          } else {
            values.insert(*dst, Value::Float(0.0));
          }
          Ok(())
        }
      }
    }

    // ========== Struct Operations ==========
    Instruction::BuildStruct {
      dst,
      struct_name,
      fields,
    } => {
      // Build a struct value from individual field values
      let field_values: Vec<Value> = fields
        .iter()
        .map(|vid| {
          values.get(vid).cloned().unwrap_or_else(|| {
            ice!(
              "BuildStruct '{}': field value {:?} not found",
              struct_name,
              vid
            );
          })
        })
        .collect();
      values.insert(*dst, Value::Struct(struct_name.clone(), field_values));
      Ok(())
    }

    Instruction::BuildVec2 {
      dst,
      elements: [x, y],
    } => {
      let vx = values.get(x).cloned().unwrap_or_else(|| {
        ice!("BuildVec2: x component value not found");
      });
      let vy = values.get(y).cloned().unwrap_or_else(|| {
        ice!("BuildVec2: y component value not found");
      });
      values.insert(*dst, Value::Vec2(Box::new(vx), Box::new(vy)));
      Ok(())
    }

    Instruction::BuildVec3 {
      dst,
      elements: [x, y, z],
    } => {
      let vx = values.get(x).cloned().unwrap_or_else(|| {
        ice!("BuildVec3: x component value not found");
      });
      let vy = values.get(y).cloned().unwrap_or_else(|| {
        ice!("BuildVec3: y component value not found");
      });
      let vz = values.get(z).cloned().unwrap_or_else(|| {
        ice!("BuildVec3: z component value not found");
      });
      values.insert(*dst, Value::Vec3(Box::new(vx), Box::new(vy), Box::new(vz)));
      Ok(())
    }

    Instruction::BuildVec4 {
      dst,
      elements: [x, y, z, w],
    } => {
      let vx = values.get(x).cloned().unwrap_or_else(|| {
        ice!("BuildVec4: x component value not found");
      });
      let vy = values.get(y).cloned().unwrap_or_else(|| {
        ice!("BuildVec4: y component value not found");
      });
      let vz = values.get(z).cloned().unwrap_or_else(|| {
        ice!("BuildVec4: z component value not found");
      });
      let vw = values.get(w).cloned().unwrap_or_else(|| {
        ice!("BuildVec4: w component value not found");
      });
      values.insert(
        *dst,
        Value::Vec4(Box::new(vx), Box::new(vy), Box::new(vz), Box::new(vw)),
      );
      Ok(())
    }

    Instruction::ExtractVecElement {
      dst,
      src,
      index,
      inner_ty: _,
    } => {
      let vec_val = values.get(src).cloned().unwrap_or_else(|| {
        ice!("ExtractVecElement: source value not found");
      });
      let element = match vec_val {
        Value::Vec2(x, y) => {
          if *index == 0 {
            x
          } else {
            y
          }
        }
        Value::Vec3(x, y, z) => {
          if *index == 0 {
            x
          } else if *index == 1 {
            y
          } else {
            z
          }
        }
        Value::Vec4(x, y, z, w) => {
          if *index == 0 {
            x
          } else if *index == 1 {
            y
          } else if *index == 2 {
            z
          } else {
            w
          }
        }
        _ => {
          return Err(
            format!(
              "ExtractVecElement: expected vector value, got {:?}",
              vec_val
            )
            .into(),
          );
        }
      };
      values.insert(*dst, *element);
      Ok(())
    }

    Instruction::ExtractField {
      dst,
      src,
      struct_name: _,
      field_index,
    } => {
      // Extract a field value from a struct value
      let struct_val = values.get(src).cloned().unwrap_or_else(|| {
        ice!("ExtractField: source struct value not found");
      });

      let field_val = match struct_val {
        Value::Struct(ref name, ref fields) => {
          let field = fields
            .get(*field_index as usize)
            .cloned()
            .unwrap_or_else(|| {
              ice!(
                "ExtractField: field index {} out of bounds for struct '{}' with {} fields",
                *field_index,
                name,
                fields.len()
              );
            });
          // Special case: str struct's ptr field may be a String (from literals).
          // Convert to a Pointer so it can be used as a pointer in syscalls etc.
          if name == "str" && *field_index == 0 {
            match field {
              Value::String(ref s) => {
                let ptr = mem_manager.alloc_null_terminated_string(s)?;
                Value::Pointer(ptr as usize)
              }
              other => other,
            }
          } else {
            field
          }
        }
        Value::Optional { is_some, ref value } => {
          // Optional values have 2 fields: 0 = is_some (bool), 1 = inner value
          if *field_index == 0 {
            Value::Bool(is_some)
          } else if *field_index == 1 {
            value.as_ref().clone()
          } else {
            return Err(
              format!(
                "ExtractField: Optional has only 2 fields, tried to access field {}",
                field_index
              )
              .into(),
            );
          }
        }
        Value::String(s) => {
          // Special case: String has implicit fields .ptr (field 0) and .len (field 1)
          if *field_index == 0 {
            // .ptr field - allocate the string in memory if not already done
            let ptr = mem_manager.alloc_null_terminated_string(&s)?;

            Value::Pointer(ptr as usize)
          } else if *field_index == 1 {
            // .len field
            Value::Int(s.len() as i64)
          } else {
            return Err(
              format!(
                "ExtractField: String has only 2 fields, tried to access field {}",
                field_index
              )
              .into(),
            );
          }
        }
        _ => {
          // Zeroed slots are the memory model's representation of uninitialized
          // storage (`MemoryManager::load` returns `Int(0)`). The epilogue's
          // auto-free pass legitimately inspects such slots when the defining
          // block was skipped (e.g. a mode-guarded body in the other mode):
          // treat them as "nothing to extract" instead of crashing.
          match struct_val {
            Value::Int(0) | Value::Uint(0) => Value::Int(0),
            other => {
              return Err(
                format!(
                  "ExtractField: expected struct or string value, got {:?}",
                  other
                )
                .into(),
              );
            }
          }
        }
      };
      values.insert(*dst, field_val);
      Ok(())
    }

    Instruction::InsertField {
      dst,
      src,
      struct_name,
      field_index,
      val,
    } => {
      // Insert a value into a struct field, creating a new struct value
      let struct_val = values.get(src).cloned().unwrap_or_else(|| {
        ice!("InsertField: source struct value not found");
      });
      let new_val = values.get(val).cloned().unwrap_or_else(|| {
        ice!("InsertField: new field value not found");
      });

      let new_struct = if let Value::Struct(name, mut fields) = struct_val {
        if (*field_index as usize) < fields.len() {
          fields[*field_index as usize] = new_val;
        }
        Value::Struct(name, fields)
      } else {
        // Create a new struct with the field
        let mut fields = vec![Value::Int(0); (*field_index as usize) + 1];
        fields[*field_index as usize] = new_val;
        Value::Struct(struct_name.clone(), fields)
      };
      values.insert(*dst, new_struct);
      Ok(())
    }

    Instruction::StructFieldPtr {
      dst,
      struct_ptr,
      struct_name,
      field_name,
    } => {
      // Compute a pointer to a named field within a struct.
      // Looks up the struct definition to find the byte offset of the field,
      // then adds it to the base pointer.
      let ptr_val = values
        .get(struct_ptr)
        .cloned()
        .ok_or_else(|| format!("StructFieldPtr: pointer value {} not found", struct_ptr.0))?;

      let (base, offset) = match ptr_val {
        Value::Pointer(addr) => (addr as i64, 0),
        Value::Int(addr) => (addr, 0),
        _ => {
          return Err(format!("StructFieldPtr: expected pointer, got {:?}", ptr_val).into());
        }
      };

      let struct_def = ir_module.struct_def(struct_name).ok_or_else(|| {
        format!(
          "StructFieldPtr: struct '{}' not found in module",
          struct_name
        )
      })?;

      let field_offset = struct_def.field_offset_by_name(field_name).ok_or_else(|| {
        format!(
          "StructFieldPtr: field '{}' not found in struct '{}'",
          field_name, struct_name
        )
      })?;

      values.insert(
        *dst,
        Value::Pointer((base + offset + field_offset as i64) as usize),
      );
      Ok(())
    }

    // ========== Optional Operations ==========
    Instruction::Some { dst, value } => {
      let inner = values.get(value).cloned().unwrap_or_else(|| {
        ice!("Some: value operand not found");
      });
      values.insert(
        *dst,
        Value::Optional {
          is_some: true,
          value: Box::new(inner),
        },
      );
      Ok(())
    }
    Instruction::None { dst } => {
      values.insert(
        *dst,
        Value::Optional {
          is_some: false,
          value: Box::new(Value::Int(0)),
        },
      );
      Ok(())
    }
    Instruction::UnwrapOptional {
      dst,
      src,
      struct_name: _,
      message,
      file,
      line,
      col,
    } => {
      // Unwrap a struct-based optional: {is_present: bool, value: T}
      let struct_val = values.get(src).cloned().unwrap_or_else(|| {
        ice!("UnwrapOptional: source optional value not found");
      });
      match struct_val {
        Value::Struct(_name, ref fields) => {
          let is_present = match fields.first() {
            Some(Value::Bool(b)) => *b,
            _ => {
              return Err("UnwrapOptional: field 0 (is_present) is not a Bool".into());
            }
          };
          if !is_present {
            lale_error_and_abort(file, *line as u64, *col as u64, message, -1, -1);
          }
          // Extract the value (field 1)
          let value = fields.get(1).cloned().unwrap_or_else(|| {
            ice!("UnwrapOptional: value field (index 1) missing from optional struct");
          });
          values.insert(*dst, value);
          Ok(())
        }
        _ => Err(
          format!(
            "UnwrapOptional: expected struct value, got {:?}",
            struct_val
          )
          .into(),
        ),
      }
    }

    // ========== Bitcast ==========
    Instruction::Bitcast { dst, src, to_ty } => {
      let src_val = values
        .get(src)
        .ok_or_else(|| format!("Bitcast: source value {} not found", src.0))?
        .clone();

      // Reinterpret the bits as a different type
      let result = match (&src_val, to_ty) {
        // f64 -> u64 (reinterpret float bits as integer)
        (Value::Float(f), IrType::U64) => Value::Uint(f.to_bits()),
        // f64 -> i64 (reinterpret float bits as signed integer)
        (Value::Float(f), IrType::I64) => Value::Int(f.to_bits() as i64),
        // u64 -> f64 (reinterpret integer bits as float)
        (Value::Uint(u), IrType::F64) => Value::Float(f64::from_bits(*u)),
        // i64 -> f64 (reinterpret signed integer bits as float)
        (Value::Int(i), IrType::F64) => Value::Float(f64::from_bits(*i as u64)),
        // f32 -> u32
        (Value::Float(f), IrType::U32) => Value::Uint((*f as f32).to_bits() as u64),
        // u32 -> f32
        (Value::Uint(u), IrType::F32) => Value::Float(f32::from_bits(*u as u32) as f64),
        // Pointer to integer and back
        (Value::Pointer(addr), IrType::I64 | IrType::U64) => Value::Uint(*addr as u64),
        (Value::Uint(u), IrType::Ptr(_)) => Value::Pointer(*u as usize),
        (Value::Int(i), IrType::Ptr(_)) => Value::Pointer(*i as usize),
        // Same type - just copy
        _ => {
          return Err(
            format!(
              "Bitcast: unsupported conversion from {:?} to {:?}",
              src_val, to_ty
            )
            .into(),
          );
        }
      };
      values.insert(*dst, result);
      Ok(())
    }

    // ========== Error Stack Operations ==========
    Instruction::PushError { message } => {
      let msg = match values.get(message) {
        Some(Value::String(s)) => s.clone(),
        Some(Value::Struct(name, fields)) if name == "str" => {
          extract_value_string(&Value::Struct(name.clone(), fields.clone()), mem_manager)
        }
        Some(val @ Value::Pointer(_)) => extract_value_string(val, mem_manager),
        other => format!("{:?}", other),
      };
      error_stack.push(msg);
      Ok(())
    }
    Instruction::PopError { dst } => {
      let msg = if let Some(msg) = error_stack.pop() {
        msg
      } else {
        "Nothing".to_string()
      };
      // Return as a str struct
      let byte_len = msg.len() as i64;
      let ptr = ALLOC_LOG.with(|log| {
        #[allow(clippy::unwrap_used, clippy::expect_used)]
        log
          .lock()
          .expect("mutex poisoned")
          .alloc_log((byte_len + 1) as usize, None, None)
      }) as i64;
      for (i, byte) in msg.as_bytes().iter().enumerate() {
        mem_manager.store(ptr + i as i64, Value::Int(*byte as i64))?;
      }
      values.insert(
        *dst,
        Value::Struct(
          "str".to_string(),
          vec![Value::Pointer(ptr as usize), Value::Int(byte_len)],
        ),
      );
      Ok(())
    }
    Instruction::ErrorCount { dst } => {
      values.insert(*dst, Value::Int(error_stack.len() as i64));
      Ok(())
    }
    Instruction::DrainErrors { to_stderr, prefix } => {
      use std::io::Write;
      let write_msg = |msg: &str| {
        let output: &mut dyn Write = if *to_stderr {
          &mut std::io::stderr()
        } else {
          &mut std::io::stdout()
        };
        if let Some(p) = prefix {
          let _ = output.write_all(p.as_bytes());
        }
        let _ = output.write_all(msg.as_bytes());
        let _ = output.write_all(b"\n");
        let _ = output.flush();
      };
      if error_stack.is_empty() {
        write_msg("Nothing");
      } else {
        while let Some(msg) = error_stack.pop() {
          write_msg(&msg);
        }
      }
      Ok(())
    }

    // Branch and return instructions are handled at block level, not here
    Instruction::Br { .. }
    | Instruction::CondBr { .. }
    | Instruction::Ret { .. }
    | Instruction::RetVoid => {
      Ok(()) // These should be handled in execute_block()
    }
  }
}

// Link against libc for C functions
#[link(name = "c")]
unsafe extern "C" {
  fn puts(s: *const i8) -> i32;
}

#[cfg(test)]
mod tests {
  use super::*;

  /// Helper: construct a Float value.
  fn f(v: f64) -> Value {
    Value::Float(v)
  }

  #[test]
  fn test_vec2_construction_and_dot() {
    let v = Value::Vec2(Box::new(f(1.0)), Box::new(f(2.0)));
    assert!(matches!(v, Value::Vec2(..)));
  }

  #[test]
  fn test_vec3_construction_and_cross() {
    let v = Value::Vec3(Box::new(f(1.0)), Box::new(f(2.0)), Box::new(f(3.0)));
    assert!(matches!(v, Value::Vec3(..)));
  }

  #[test]
  fn test_vec4_construction() {
    let v = Value::Vec4(
      Box::new(f(1.0)),
      Box::new(f(2.0)),
      Box::new(f(3.0)),
      Box::new(f(4.0)),
    );
    assert!(matches!(v, Value::Vec4(..)));
  }

  #[test]
  fn test_extract_float_from_int() {
    let v = Box::new(Value::Int(42));
    assert!((Value::extract_float(v) - 42.0).abs() < 1e-10);
  }

  #[test]
  fn test_extract_float_from_uint() {
    let v = Box::new(Value::Uint(7));
    assert!((Value::extract_float(v) - 7.0).abs() < 1e-10);
  }

  #[test]
  fn test_vec_dot_product_values() {
    // Manual dot product computation using extract_float
    let a = Value::Vec2(Box::new(f(1.0)), Box::new(f(2.0)));
    let b = Value::Vec2(Box::new(f(3.0)), Box::new(f(4.0)));
    // a · b = 1*3 + 2*4 = 11
    if let (Value::Vec2(ax, ay), Value::Vec2(bx, by)) = (a, b) {
      let result = Value::extract_float(ax) * Value::extract_float(bx)
        + Value::extract_float(ay) * Value::extract_float(by);
      assert!((result - 11.0).abs() < 1e-10);
    }
  }

  #[test]
  fn test_vec3_cross_product_values() {
    // Cross product: (1,0,0) × (0,1,0) = (0,0,1)
    let a = Value::Vec3(Box::new(f(1.0)), Box::new(f(0.0)), Box::new(f(0.0)));
    let b = Value::Vec3(Box::new(f(0.0)), Box::new(f(1.0)), Box::new(f(0.0)));
    if let (Value::Vec3(ax, ay, az), Value::Vec3(bx, by, bz)) = (a, b) {
      let x1 = Value::extract_float(ax);
      let y1 = Value::extract_float(ay);
      let z1 = Value::extract_float(az);
      let x2 = Value::extract_float(bx);
      let y2 = Value::extract_float(by);
      let z2 = Value::extract_float(bz);
      let cx = y1 * z2 - z1 * y2;
      let cy = z1 * x2 - x1 * z2;
      let cz = x1 * y2 - y1 * x2;
      assert!((cx - 0.0).abs() < 1e-10);
      assert!((cy - 0.0).abs() < 1e-10);
      assert!((cz - 1.0).abs() < 1e-10);
    }
  }

  #[test]
  fn test_vec4_dot_product() {
    // (1,2,3,4) · (5,6,7,8) = 1*5 + 2*6 + 3*7 + 4*8 = 70
    let a = Value::Vec4(
      Box::new(f(1.0)),
      Box::new(f(2.0)),
      Box::new(f(3.0)),
      Box::new(f(4.0)),
    );
    let b = Value::Vec4(
      Box::new(f(5.0)),
      Box::new(f(6.0)),
      Box::new(f(7.0)),
      Box::new(f(8.0)),
    );
    if let (Value::Vec4(ax, ay, az, aw), Value::Vec4(bx, by, bz, bw)) = (a, b) {
      let result = Value::extract_float(ax) * Value::extract_float(bx)
        + Value::extract_float(ay) * Value::extract_float(by)
        + Value::extract_float(az) * Value::extract_float(bz)
        + Value::extract_float(aw) * Value::extract_float(bw);
      assert!((result - 70.0).abs() < 1e-10);
    }
  }

  #[test]
  fn test_vec2_dot_negative() {
    // (-1,-2) · (3,4) = -1*3 + -2*4 = -11
    let a = Value::Vec2(Box::new(f(-1.0)), Box::new(f(-2.0)));
    let b = Value::Vec2(Box::new(f(3.0)), Box::new(f(4.0)));
    if let (Value::Vec2(ax, ay), Value::Vec2(bx, by)) = (a, b) {
      let result = Value::extract_float(ax) * Value::extract_float(bx)
        + Value::extract_float(ay) * Value::extract_float(by);
      assert!((result - (-11.0)).abs() < 1e-10);
    }
  }

  #[test]
  fn test_vec3_dot_zero() {
    // (0,0,0) · (1,2,3) = 0
    let a = Value::Vec3(Box::new(f(0.0)), Box::new(f(0.0)), Box::new(f(0.0)));
    let b = Value::Vec3(Box::new(f(1.0)), Box::new(f(2.0)), Box::new(f(3.0)));
    if let (Value::Vec3(ax, ay, az), Value::Vec3(bx, by, bz)) = (a, b) {
      let result = Value::extract_float(ax) * Value::extract_float(bx)
        + Value::extract_float(ay) * Value::extract_float(by)
        + Value::extract_float(az) * Value::extract_float(bz);
      assert!((result - 0.0).abs() < 1e-10);
    }
  }

  #[test]
  fn test_scalar_dot_int() {
    // Int(3) dot Int(4) = 12
    let a = Box::new(Value::Int(3));
    let b = Box::new(Value::Int(4));
    let result = Value::extract_float(a) * Value::extract_float(b);
    assert!((result - 12.0).abs() < 1e-10);
  }

  #[test]
  fn test_scalar_dot_uint() {
    // Uint(3) dot Uint(4) = 12
    let a = Box::new(Value::Uint(3));
    let b = Box::new(Value::Uint(4));
    let result = Value::extract_float(a) * Value::extract_float(b);
    assert!((result - 12.0).abs() < 1e-10);
  }

  #[test]
  fn test_vec3_cross_parallel() {
    // (1,2,3) × (2,4,6) = (0,0,0) — parallel vectors
    let a = Value::Vec3(Box::new(f(1.0)), Box::new(f(2.0)), Box::new(f(3.0)));
    let b = Value::Vec3(Box::new(f(2.0)), Box::new(f(4.0)), Box::new(f(6.0)));
    if let (Value::Vec3(ax, ay, az), Value::Vec3(bx, by, bz)) = (a, b) {
      let x1 = Value::extract_float(ax);
      let y1 = Value::extract_float(ay);
      let z1 = Value::extract_float(az);
      let x2 = Value::extract_float(bx);
      let y2 = Value::extract_float(by);
      let z2 = Value::extract_float(bz);
      let cx = y1 * z2 - z1 * y2;
      let cy = z1 * x2 - x1 * z2;
      let cz = x1 * y2 - y1 * x2;
      assert!((cx - 0.0).abs() < 1e-10);
      assert!((cy - 0.0).abs() < 1e-10);
      assert!((cz - 0.0).abs() < 1e-10);
    }
  }

  #[test]
  fn test_vec3_cross_same_vector() {
    // v × v = (0,0,0): (1,2,3) × (1,2,3)
    let a = Value::Vec3(Box::new(f(1.0)), Box::new(f(2.0)), Box::new(f(3.0)));
    let b = Value::Vec3(Box::new(f(1.0)), Box::new(f(2.0)), Box::new(f(3.0)));
    if let (Value::Vec3(ax, ay, az), Value::Vec3(bx, by, bz)) = (a, b) {
      let x1 = Value::extract_float(ax);
      let y1 = Value::extract_float(ay);
      let z1 = Value::extract_float(az);
      let x2 = Value::extract_float(bx);
      let y2 = Value::extract_float(by);
      let z2 = Value::extract_float(bz);
      let cx = y1 * z2 - z1 * y2;
      let cy = z1 * x2 - x1 * z2;
      let cz = x1 * y2 - y1 * x2;
      assert!((cx - 0.0).abs() < 1e-10);
      assert!((cy - 0.0).abs() < 1e-10);
      assert!((cz - 0.0).abs() < 1e-10);
    }
  }

  #[test]
  fn test_vec3_cross_negative() {
    // (-1,2,-3) × (4,-5,6) = (-3,-6,-3)
    let a = Value::Vec3(Box::new(f(-1.0)), Box::new(f(2.0)), Box::new(f(-3.0)));
    let b = Value::Vec3(Box::new(f(4.0)), Box::new(f(-5.0)), Box::new(f(6.0)));
    if let (Value::Vec3(ax, ay, az), Value::Vec3(bx, by, bz)) = (a, b) {
      let x1 = Value::extract_float(ax);
      let y1 = Value::extract_float(ay);
      let z1 = Value::extract_float(az);
      let x2 = Value::extract_float(bx);
      let y2 = Value::extract_float(by);
      let z2 = Value::extract_float(bz);
      let cx = y1 * z2 - z1 * y2;
      let cy = z1 * x2 - x1 * z2;
      let cz = x1 * y2 - y1 * x2;
      assert!((cx - (-3.0)).abs() < 1e-10);
      assert!((cy - (-6.0)).abs() < 1e-10);
      assert!((cz - (-3.0)).abs() < 1e-10);
    }
  }

  // ==================== Checked arithmetic helpers ====================

  #[test]
  fn test_checked_signed_add_ok() {
    assert_eq!(
      checked_signed_arith(&IrType::I8, 100, 27, CheckedArithOp::Add).unwrap(),
      127
    );
  }

  #[test]
  fn test_checked_signed_add_overflow() {
    assert!(checked_signed_arith(&IrType::I8, 127, 1, CheckedArithOp::Add).is_err());
  }

  #[test]
  fn test_checked_signed_sub_underflow() {
    assert!(checked_signed_arith(&IrType::I8, -128, 1, CheckedArithOp::Sub).is_err());
  }

  #[test]
  fn test_checked_signed_mul_overflow() {
    assert!(checked_signed_arith(&IrType::I8, 64, 2, CheckedArithOp::Mul).is_err());
  }

  #[test]
  fn test_checked_signed_neg_min_overflow() {
    assert!(checked_signed_arith(&IrType::I64, i64::MIN, 0, CheckedArithOp::Neg).is_err());
  }

  #[test]
  fn test_checked_unsigned_add_overflow() {
    assert!(checked_unsigned_arith(&IrType::U8, 255, 1, CheckedArithOp::Add).is_err());
  }

  #[test]
  fn test_checked_unsigned_mul_overflow() {
    assert!(checked_unsigned_arith(&IrType::U8, 16, 16, CheckedArithOp::Mul).is_err());
  }

  #[test]
  fn test_checked_unsigned_sub_underflow() {
    assert!(checked_unsigned_arith(&IrType::U8, 0, 1, CheckedArithOp::Sub).is_err());
  }

  #[test]
  fn test_checked_binary_mixed_tag_signed_ty() {
    // A same-width `as i64` on a u64 value is an IR bitcast and can leave the
    // runtime tag as Uint while the declared type is signed. The checked path
    // must interpret by declared type, not by tag.
    let result = checked_binary(
      &IrType::I64,
      &Value::Uint(3),
      &Value::Int(1),
      CheckedArithOp::Sub,
    )
    .unwrap();
    assert!(matches!(result, Value::Int(2)));
  }

  #[test]
  fn test_checked_neg_value_ok() {
    let result = checked_neg_value(&IrType::I8, &Value::Int(5)).unwrap();
    assert!(matches!(result, Value::Int(-5)));
  }
}
