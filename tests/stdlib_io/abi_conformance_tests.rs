use lale::ir::{ExternFunc, IrType, Module};
use lale::ir_gen::IrGenerator;

fn hook<'a>(module: &'a Module, name: &str) -> &'a ExternFunc {
  module
    .extern_func(name)
    .unwrap_or_else(|| panic!("missing extern '{}'", name))
}

#[test]
fn primitive_sizes_match_abi() {
  assert_eq!(IrType::Bool.size_bytes(), Some(1));
  assert_eq!(IrType::I8.size_bytes(), Some(1));
  assert_eq!(IrType::U8.size_bytes(), Some(1));
  assert_eq!(IrType::I16.size_bytes(), Some(2));
  assert_eq!(IrType::U16.size_bytes(), Some(2));
  assert_eq!(IrType::F16.size_bytes(), Some(2));
  assert_eq!(IrType::I32.size_bytes(), Some(4));
  assert_eq!(IrType::U32.size_bytes(), Some(4));
  assert_eq!(IrType::F32.size_bytes(), Some(4));
  assert_eq!(IrType::Char.size_bytes(), Some(4));
  assert_eq!(IrType::I64.size_bytes(), Some(8));
  assert_eq!(IrType::U64.size_bytes(), Some(8));
  assert_eq!(IrType::F64.size_bytes(), Some(8));
  assert_eq!(IrType::raw_ptr().size_bytes(), Some(8));
  assert_eq!(IrType::ptr(IrType::I32).size_bytes(), Some(8));
}

#[test]
fn text_fat_pointer_layout() {
  let mut module = Module::new("abi");
  IrGenerator::load_builtins_into_module(&mut module, true)
    .expect("embedded builtins.lale must parse and compile");

  let text_def = module
    .struct_def("text")
    .expect("builtins.lale defines the text type");

  assert_eq!(text_def.name, "text");
  assert_eq!(text_def.fields.len(), 3);

  assert_eq!(text_def.fields[0].0, "ptr");
  assert_eq!(text_def.fields[0].1, IrType::raw_ptr());
  assert_eq!(text_def.fields[1].0, "bytes");
  assert_eq!(text_def.fields[1].1, IrType::U64);

  assert_eq!(text_def.field_offset_by_name("ptr"), Some(0));
  assert_eq!(text_def.field_offset_by_name("bytes"), Some(8));
  assert_eq!(text_def.field_offset_by_name("chars"), Some(16));
}

#[test]
fn extern_ffi_signatures_match_abi() {
  let mut module = Module::new_empty("abi");
  module.add_stdlib_externs();

  // C runtime boundary (platform C ABI).
  assert_eq!(
    hook(&module, "write").params,
    vec![IrType::I32, IrType::ptr(IrType::I8), IrType::I64]
  );
  assert_eq!(hook(&module, "write").return_type, IrType::I64);
  assert_eq!(hook(&module, "pow").params, vec![IrType::F64, IrType::F64]);
  assert_eq!(hook(&module, "pow").return_type, IrType::F64);

  // __lale_* runtime hooks.
  assert_eq!(hook(&module, "__lale_exit").params, vec![IrType::I32]);
  assert_eq!(hook(&module, "__lale_exit").return_type, IrType::Void);

  assert_eq!(hook(&module, "__lale_malloc_u64").params, vec![IrType::U64]);
  assert_eq!(
    hook(&module, "__lale_malloc_u64").return_type,
    IrType::ptr(IrType::I8)
  );

  assert_eq!(
    hook(&module, "__lale_free_pointer").params,
    vec![IrType::ptr(IrType::I8)]
  );
  assert_eq!(
    hook(&module, "__lale_free_pointer").return_type,
    IrType::Void
  );

  // The stale pre-freeze hooks must not be part of the ABI.
  assert!(!module.has_extern_func("__lale_write_stdout"));
  assert!(!module.has_extern_func("__lale_write_stderr"));
  assert!(!module.has_extern_func("__lale_error"));
  assert!(!module.has_extern_func("__lale_pow"));
  assert!(!module.has_extern_func("__lale_malloc"));
  assert!(!module.has_extern_func("__lale_free"));
}
