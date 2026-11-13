//! Tests for type definition syntax.
//!
//! Types are composite types that group multiple fields together.
//! Initial implementation: data only (no methods or inheritance yet).

use lale::{LaleParser, Rule};
use pest::Parser;

fn parse_program(code: &str) -> Result<(), String> {
  LaleParser::parse(Rule::program, code)
    .map(|_| ())
    .map_err(|e| e.to_string())
}

#[test]
fn test_simple_type_definition() {
  let code = r#"
type Point
    x as f64
    y as f64
end type
"#;

  let result = parse_program(code);
  assert!(result.is_ok(), "Failed to parse simple type: {:?}", result);
}

#[test]
fn test_type_empty_rejected() {
  let result = LaleParser::parse(Rule::type_def, "type Empty end type");
  assert!(result.is_err(), "type without any field must fail to parse");
}

#[test]
fn test_type_with_multiple_field_types() {
  let code = r#"
type Person
    name as text
    age as i32
    height as f64
end type
"#;

  let result = parse_program(code);
  assert!(
    result.is_ok(),
    "Failed to parse type with mixed types: {:?}",
    result
  );
}

#[test]
fn test_exported_type() {
  let code = r#"
export type Vehicle
    make as text
    model as text
    year as i32
end type
"#;

  let result = parse_program(code);
  assert!(
    result.is_ok(),
    "Failed to parse exported type: {:?}",
    result
  );
}

#[test]
fn test_type_with_array_field() {
  let code = r#"
type Matrix
    data as f64[3][3]
    rows as i32
end type
"#;

  let result = parse_program(code);
  assert!(
    result.is_ok(),
    "Failed to parse type with array field: {:?}",
    result
  );
}

#[test]
fn test_empty_type_rejected() {
  let code = r#"
type Empty
end type
"#;

  let result = parse_program(code);
  assert!(result.is_err(), "Empty type should be rejected");
}

#[test]
fn test_type_with_doc_comment() {
  let code = r#"
/// A 2D point in space
type Point
    x as f64
    y as f64
end type
"#;

  let result = parse_program(code);
  assert!(
    result.is_ok(),
    "Failed to parse type with doc comment: {:?}",
    result
  );
}

#[test]
fn test_multiple_types_in_program() {
  let code = r#"
type Point
    x as f64
    y as f64
end type

type Color
    r as u8
    g as u8
    b as u8
    a as u8
end type

type Circle
    center as Point
    radius as f64
    color as Color
end type
"#;

  let result = parse_program(code);
  assert!(
    result.is_ok(),
    "Failed to parse multiple types: {:?}",
    result
  );
}

#[test]
fn test_type_with_newlines_in_fields() {
  let code = r#"
type Data
    field1 as i32

    field2 as text

    field3 as f64
end type
"#;

  let result = parse_program(code);
  assert!(
    result.is_ok(),
    "Failed to parse type with newlines: {:?}",
    result
  );
}

#[test]
fn test_type_field_with_inline_comment() {
  let code = r#"
type Config
    timeout as i32 // timeout in seconds
    retries as u8 // number of retries
    enabled as bool // enable this feature
end type
"#;

  let result = parse_program(code);
  assert!(
    result.is_ok(),
    "Failed to parse type with field comments: {:?}",
    result
  );
}

#[test]
fn test_type_field_with_doc_comment() {
  let code = r#"
type Settings
    name as text /// The name of the setting
    value as i32 /// The numeric value
end type
"#;

  let result = parse_program(code);
  assert!(
    result.is_ok(),
    "Failed to parse type with doc comments: {:?}",
    result
  );
}

#[test]
fn test_type_mixed_comments() {
  let code = r#"
type User
    id as u32 // unique identifier
    username as text /// The login name
    email as text // contact email
end type
"#;

  let result = parse_program(code);
  assert!(
    result.is_ok(),
    "Failed to parse type with mixed comments: {:?}",
    result
  );
}

#[test]
fn test_type_field_with_unit() {
  let code = r#"
type Measurement
    value as f64 in <m>
    tolerance as f64 in <mm>
end type
"#;
  let result = parse_program(code);
  assert!(
    result.is_ok(),
    "Failed to parse type with unit fields: {:?}",
    result
  );
}

#[test]
fn test_type_field_mixed_units() {
  let code = r#"
type Physics
    distance as f64 in <m>
    time as f64 in <s>
    mass as f64 in <kg>
    count as i32
end type
"#;
  let result = parse_program(code);
  assert!(
    result.is_ok(),
    "Failed to parse type with mixed unit/non-unit fields: {:?}",
    result
  );
}

#[test]
fn test_type_field_unit_with_array() {
  let code = r#"
type DataSeries
    samples as f64[10] in <V>
end type
"#;
  let result = parse_program(code);
  assert!(
    result.is_ok(),
    "Failed to parse type with array and unit: {:?}",
    result
  );
}

// ==================== PRIVATE FIELD TESTS ====================

#[test]
fn test_private_field_simple() {
  let code = r#"
type Config
    private api_key as text
    timeout as i32
end type
"#;
  let result = parse_program(code);
  assert!(
    result.is_ok(),
    "Failed to parse type with private field: {:?}",
    result
  );
}

#[test]
fn test_private_field_multiple() {
  let code = r#"
type Internal
    private secret as text
    private key as u64
    name as text
end type
"#;
  let result = parse_program(code);
  assert!(
    result.is_ok(),
    "Failed to parse type with multiple private fields: {:?}",
    result
  );
}

#[test]
fn test_private_field_with_unit() {
  let code = r#"
type Sensor
    private raw_value as f64 in <V>
    calibrated as f64 in <V>
end type
"#;
  let result = parse_program(code);
  assert!(
    result.is_ok(),
    "Failed to parse private field with unit: {:?}",
    result
  );
}

#[test]
fn test_private_field_with_doc_comment() {
  let code = r#"
type Database
    private handle as pointer /// Internal connection handle — do not access directly
    url as text
end type
"#;
  let result = parse_program(code);
  assert!(
    result.is_ok(),
    "Failed to parse private field with doc comment: {:?}",
    result
  );
}
