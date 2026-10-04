//! IR Type System
//!
//! Defines the type system for the Lale IR. These types map directly from
//! Lale's source-level types and are used throughout the IR.

use std::fmt;

/// IR types representing all possible value types in the IR.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum IrType {
  /// No value (for void functions)
  Void,
  /// Boolean (1 bit logical)
  Bool,
  /// Signed 8-bit integer
  I8,
  /// Signed 16-bit integer
  I16,
  /// Signed 32-bit integer
  I32,
  /// Signed 64-bit integer
  I64,
  /// Unsigned 8-bit integer
  U8,
  /// Unsigned 16-bit integer
  U16,
  /// Unsigned 32-bit integer
  U32,
  /// Unsigned 64-bit integer
  U64,
  /// Raw byte (8-bit octet). Unlike `u8`, this is not a number: it has no
  /// arithmetic or ordering operators, only bitwise operations and explicit
  /// conversions to/from integer types.
  Byte,
  /// 16-bit floating point (IEEE 754 half)
  F16,
  /// 32-bit floating point (IEEE 754 single)
  F32,
  /// 64-bit floating point (IEEE 754 double)
  F64,
  /// Unicode character (32-bit codepoint)
  Char,
  /// Typed pointer (platform-sized) pointing to a value of the specified type.
  /// Use `IrType::ptr(ty)` to create, or `IrType::raw_ptr()` for opaque pointers.
  Ptr(Box<IrType>),
  /// Fixed-size array
  Array { element: Box<IrType>, size: u64 },
  /// User-defined struct (type)
  Struct { name: String },
  /// 2-component vector with inner element type
  Vec2(Box<IrType>),
  /// 3-component vector with inner element type
  Vec3(Box<IrType>),
  /// 4-component vector with inner element type
  Vec4(Box<IrType>),
  /// Optional type (T?)
  Optional(Box<IrType>),
}

impl IrType {
  /// Returns true if this is a signed integer type.
  pub fn is_signed_int(&self) -> bool {
    matches!(self, IrType::I8 | IrType::I16 | IrType::I32 | IrType::I64)
  }

  /// Returns true if this is an unsigned integer type.
  pub fn is_unsigned_int(&self) -> bool {
    matches!(self, IrType::U8 | IrType::U16 | IrType::U32 | IrType::U64)
  }

  /// Returns true if this is the raw `byte` type (an octet, not a number).
  pub fn is_byte(&self) -> bool {
    matches!(self, IrType::Byte)
  }

  /// Returns true if this is any integer type.
  pub fn is_integer(&self) -> bool {
    self.is_signed_int() || self.is_unsigned_int()
  }

  /// Returns true if this is a floating-point type.
  pub fn is_float(&self) -> bool {
    matches!(self, IrType::F16 | IrType::F32 | IrType::F64)
  }

  /// Returns true if this is a numeric type (integer or float).
  pub fn is_numeric(&self) -> bool {
    self.is_integer() || self.is_float()
  }

  /// Returns true if this is a pointer type.
  pub fn is_ptr(&self) -> bool {
    matches!(self, IrType::Ptr(_))
  }

  /// Returns the pointee type if this is a pointer, None otherwise.
  pub fn pointee(&self) -> Option<&IrType> {
    match self {
      IrType::Ptr(inner) => Some(inner),
      _ => None,
    }
  }

  /// Returns the size in bits for primitive types.
  pub fn bit_size(&self) -> Option<u32> {
    match self {
      IrType::Bool => Some(1),
      IrType::I8 | IrType::U8 | IrType::Byte => Some(8),
      IrType::I16 | IrType::U16 | IrType::F16 => Some(16),
      IrType::I32 | IrType::U32 | IrType::F32 | IrType::Char => Some(32),
      IrType::I64 | IrType::U64 | IrType::F64 | IrType::Ptr(_) => Some(64),
      IrType::Void => Some(0),
      IrType::Array { .. }
      | IrType::Struct { .. }
      | IrType::Optional(_)
      | IrType::Vec2(_)
      | IrType::Vec3(_)
      | IrType::Vec4(_) => None,
    }
  }

  /// Returns the size in bytes, or `None` if the size cannot be determined
  /// without external context (e.g., struct layouts).
  pub fn size_bytes(&self) -> Option<u32> {
    match self {
      IrType::Void => Some(0),
      IrType::Bool => Some(1),
      IrType::I8 | IrType::U8 | IrType::Byte => Some(1),
      IrType::I16 | IrType::U16 | IrType::F16 => Some(2),
      IrType::I32 | IrType::U32 | IrType::F32 | IrType::Char => Some(4),
      IrType::I64 | IrType::U64 | IrType::F64 | IrType::Ptr(_) => Some(8),
      IrType::Array { element, size } => element.size_bytes().map(|s| s * (*size as u32)),
      IrType::Struct { .. } => None, // requires struct layout from Module
      IrType::Optional(inner) => {
        // 1 byte discriminant + inner type size
        inner.size_bytes().map(|s| 1 + s)
      }
      IrType::Vec2(inner) => inner.size_bytes().map(|s| 2 * s),
      IrType::Vec3(inner) => inner.size_bytes().map(|s| 3 * s),
      IrType::Vec4(inner) => inner.size_bytes().map(|s| 4 * s),
    }
  }

  /// Create a typed pointer to the given type.
  pub fn ptr(pointee: IrType) -> Self {
    IrType::Ptr(Box::new(pointee))
  }

  /// Create a raw/opaque pointer (pointer to Void).
  /// Use this when the pointee type is unknown or irrelevant.
  pub fn raw_ptr() -> Self {
    IrType::Ptr(Box::new(IrType::Void))
  }

  /// Create an array type.
  pub fn array(element: IrType, size: u64) -> Self {
    IrType::Array {
      element: Box::new(element),
      size,
    }
  }

  /// Create a struct type reference.
  pub fn struct_ref(name: impl Into<String>) -> Self {
    IrType::Struct { name: name.into() }
  }

  /// Create an optional type wrapping the given inner type.
  pub fn optional(inner: IrType) -> Self {
    IrType::Optional(Box::new(inner))
  }

  /// Create a 2-component vector type.
  pub fn vec2(inner: IrType) -> Self {
    IrType::Vec2(Box::new(inner))
  }

  /// Create a 3-component vector type.
  pub fn vec3(inner: IrType) -> Self {
    IrType::Vec3(Box::new(inner))
  }

  /// Create a 4-component vector type.
  pub fn vec4(inner: IrType) -> Self {
    IrType::Vec4(Box::new(inner))
  }
}

impl fmt::Display for IrType {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    match self {
      IrType::Void => write!(f, "void"),
      IrType::Bool => write!(f, "bool"),
      IrType::I8 => write!(f, "i8"),
      IrType::I16 => write!(f, "i16"),
      IrType::I32 => write!(f, "i32"),
      IrType::I64 => write!(f, "i64"),
      IrType::U8 => write!(f, "u8"),
      IrType::U16 => write!(f, "u16"),
      IrType::U32 => write!(f, "u32"),
      IrType::U64 => write!(f, "u64"),
      IrType::Byte => write!(f, "byte"),
      IrType::F16 => write!(f, "f16"),
      IrType::F32 => write!(f, "f32"),
      IrType::F64 => write!(f, "f64"),
      IrType::Char => write!(f, "char"),
      IrType::Ptr(pointee) => {
        if **pointee == IrType::Void {
          write!(f, "ptr")
        } else {
          write!(f, "ptr<{}>", pointee)
        }
      }
      IrType::Array { element, size } => write!(f, "[{} x {}]", size, element),
      IrType::Struct { name } => write!(f, "%{}", name),
      IrType::Optional(inner) => write!(f, "{}?", inner),
      IrType::Vec2(inner) => write!(f, "vec2<{}>", inner),
      IrType::Vec3(inner) => write!(f, "vec3<{}>", inner),
      IrType::Vec4(inner) => write!(f, "vec4<{}>", inner),
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_integer_classification() {
    assert!(IrType::I32.is_signed_int());
    assert!(IrType::U32.is_unsigned_int());
    assert!(IrType::I64.is_integer());
    assert!(!IrType::F64.is_integer());
  }

  #[test]
  fn test_float_classification() {
    assert!(IrType::F32.is_float());
    assert!(IrType::F64.is_float());
    assert!(!IrType::I32.is_float());
  }

  #[test]
  fn test_bit_sizes() {
    assert_eq!(IrType::I8.bit_size(), Some(8));
    assert_eq!(IrType::I32.bit_size(), Some(32));
    assert_eq!(IrType::F64.bit_size(), Some(64));
    assert_eq!(IrType::raw_ptr().bit_size(), Some(64));
    assert_eq!(IrType::ptr(IrType::I32).bit_size(), Some(64));
  }

  #[test]
  fn test_display() {
    assert_eq!(format!("{}", IrType::I32), "i32");
    assert_eq!(format!("{}", IrType::array(IrType::I32, 10)), "[10 x i32]");
    assert_eq!(format!("{}", IrType::struct_ref("Point")), "%Point");
    assert_eq!(format!("{}", IrType::optional(IrType::I32)), "i32?");
    assert_eq!(format!("{}", IrType::optional(IrType::F64)), "f64?");
  }

  #[test]
  fn test_typed_pointers() {
    // Raw pointer (opaque)
    let raw = IrType::raw_ptr();
    assert!(raw.is_ptr());
    assert_eq!(raw.pointee(), Some(&IrType::Void));
    assert_eq!(format!("{}", raw), "ptr");

    // Typed pointer to i32
    let ptr_i32 = IrType::ptr(IrType::I32);
    assert!(ptr_i32.is_ptr());
    assert_eq!(ptr_i32.pointee(), Some(&IrType::I32));
    assert_eq!(format!("{}", ptr_i32), "ptr<i32>");

    // Pointer to text struct
    let ptr_text = IrType::ptr(IrType::struct_ref("text"));
    assert!(ptr_text.is_ptr());
    assert_eq!(ptr_text.pointee(), Some(&IrType::struct_ref("text")));
    assert_eq!(format!("{}", ptr_text), "ptr<%text>");

    // Pointer to struct
    let ptr_textuct = IrType::ptr(IrType::struct_ref("Person"));
    assert!(ptr_textuct.is_ptr());
    assert_eq!(format!("{}", ptr_textuct), "ptr<%Person>");

    // Nested pointer (pointer to pointer)
    let ptr_ptr = IrType::ptr(IrType::ptr(IrType::I32));
    assert_eq!(format!("{}", ptr_ptr), "ptr<ptr<i32>>");
  }
}
