//! Element Type Integration and Multi-Dimensional Array Support
//!
//! Tests:
//! 1. Element type extraction from array variables
//! 2. Array dimension extraction from type strings
//! 3. Multi-dimensional array indexing computation
//! 4. Type string parsing and conversion

#[cfg(test)]
mod array_indexing_phase_2_5_tests {
  use lale::ir_gen::IrGenerator;

  #[test]
  fn test_extract_base_type_simple() {
    let ir = IrGenerator::new("test");
    assert_eq!(ir.extract_base_type_pub("i32"), "i32");
    assert_eq!(ir.extract_base_type_pub("f64"), "f64");
    assert_eq!(ir.extract_base_type_pub("u8"), "u8");
  }

  #[test]
  fn test_extract_base_type_single_dimension() {
    let ir = IrGenerator::new("test");
    assert_eq!(ir.extract_base_type_pub("i32[5]"), "i32");
    assert_eq!(ir.extract_base_type_pub("f64[10]"), "f64");
    assert_eq!(ir.extract_base_type_pub("u8[100]"), "u8");
  }

  #[test]
  fn test_extract_base_type_multi_dimension() {
    let ir = IrGenerator::new("test");
    assert_eq!(ir.extract_base_type_pub("i32[5][10]"), "i32");
    assert_eq!(ir.extract_base_type_pub("f64[3][4][5]"), "f64");
    assert_eq!(ir.extract_base_type_pub("u8[2][3][4][5]"), "u8");
  }

  #[test]
  fn test_extract_array_dimensions_single() {
    let ir = IrGenerator::new("test");
    assert_eq!(ir.extract_array_dimensions_pub("i32[5]"), vec![5]);
    assert_eq!(ir.extract_array_dimensions_pub("f64[10]"), vec![10]);
    assert_eq!(ir.extract_array_dimensions_pub("u8[100]"), vec![100]);
  }

  #[test]
  fn test_extract_array_dimensions_multi() {
    let ir = IrGenerator::new("test");
    assert_eq!(ir.extract_array_dimensions_pub("i32[5][10]"), vec![5, 10]);
    assert_eq!(
      ir.extract_array_dimensions_pub("f64[3][4][5]"),
      vec![3, 4, 5]
    );
    assert_eq!(
      ir.extract_array_dimensions_pub("u8[2][3][4][5]"),
      vec![2, 3, 4, 5]
    );
  }

  #[test]
  fn test_extract_array_dimensions_no_array() {
    let ir = IrGenerator::new("test");
    assert_eq!(ir.extract_array_dimensions_pub("i32"), vec![]);
    assert_eq!(ir.extract_array_dimensions_pub("f64"), vec![]);
  }

  #[test]
  fn test_type_string_to_ir_type_integers() {
    let ir = IrGenerator::new("test");
    // Test integer types
    assert_eq!(
      format!("{:?}", ir.type_string_to_ir_type_pub("u8")),
      format!("{:?}", lale::ir::types::IrType::U8)
    );
    assert_eq!(
      format!("{:?}", ir.type_string_to_ir_type_pub("i32")),
      format!("{:?}", lale::ir::types::IrType::I32)
    );
    assert_eq!(
      format!("{:?}", ir.type_string_to_ir_type_pub("u64")),
      format!("{:?}", lale::ir::types::IrType::U64)
    );
  }

  #[test]
  fn test_type_string_to_ir_type_floats() {
    let ir = IrGenerator::new("test");
    assert_eq!(
      format!("{:?}", ir.type_string_to_ir_type_pub("f32")),
      format!("{:?}", lale::ir::types::IrType::F32)
    );
    assert_eq!(
      format!("{:?}", ir.type_string_to_ir_type_pub("f64")),
      format!("{:?}", lale::ir::types::IrType::F64)
    );
  }

  #[test]
  fn test_type_string_to_ir_type_special() {
    let ir = IrGenerator::new("test");
    assert_eq!(
      format!("{:?}", ir.type_string_to_ir_type_pub("bool")),
      format!("{:?}", lale::ir::types::IrType::Bool)
    );
    assert_eq!(
      format!("{:?}", ir.type_string_to_ir_type_pub("char")),
      format!("{:?}", lale::ir::types::IrType::U32)
    );
  }

  // test_type_string_to_ir_type_unknown removed — unknown types now trigger
  // ice!() (hard crash), not a silent I64 default. Semantic analysis must
  // reject unknown types before IR generation begins.

  // Test stride computation for multi-dimensional arrays
  // For 2D array i32[3][4]:
  // - arr[1][1] should be at offset 0
  // - arr[1][2] should be at offset 1*4 = 4 (elements)
  // - arr[2][1] should be at offset 1*4 = 4 (elements)
  // - arr[2][2] should be at offset 1*4 + 1 = 5 (elements)
  #[test]
  fn test_multidimensional_stride_2d() {
    // Stride for first dimension of [3][4] should be 4
    // Stride for second dimension should be 1
    let dims = [3i64, 4i64];

    // For dimension 0, stride = product of dims[1..] = 4
    let stride_dim0 = {
      let mut s = 1i64;
      for dim in &dims[1..] {
        s *= dim;
      }
      s
    };
    assert_eq!(stride_dim0, 4);

    // For dimension 1, stride = product of dims[2..] = 1
    let stride_dim1 = {
      let mut s = 1i64;
      for dim in &dims[2..] {
        s *= dim;
      }
      s
    };
    assert_eq!(stride_dim1, 1);
  }

  #[test]
  fn test_multidimensional_stride_3d() {
    // Stride for [3][4][5]
    let dims = [3i64, 4i64, 5i64];

    // Dimension 0: stride = 4 * 5 = 20
    let stride_dim0 = {
      let mut s = 1i64;
      for dim in &dims[1..] {
        s *= dim;
      }
      s
    };
    assert_eq!(stride_dim0, 20);

    // Dimension 1: stride = 5
    let stride_dim1 = {
      let mut s = 1i64;
      for dim in &dims[2..] {
        s *= dim;
      }
      s
    };
    assert_eq!(stride_dim1, 5);

    // Dimension 2: stride = 1
    let stride_dim2 = {
      let mut s = 1i64;
      for dim in &dims[3..] {
        s *= dim;
      }
      s
    };
    assert_eq!(stride_dim2, 1);
  }
}
