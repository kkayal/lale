//! Tests for IR generation
//!
//! These tests verify that the IR generator correctly converts the AST to IR.

use lale::ast::builder::build_program;
use lale::ir_gen::IrGenerator;
use lale::semantic_analysis::{AnalyzerResults, analyze_ast};
use lale::{LaleParser, Rule};
use pest::Parser;

/// Helper to compile source and generate IR
fn generate_ir_for_source(source: &str) -> Result<lale::ir::Module, String> {
  let pairs =
    LaleParser::parse(Rule::program, source).map_err(|e| format!("Parse error: {}", e))?;

  let program = build_program(pairs, "test.lale").map_err(|e| format!("Build error: {}", e))?;

  let analyzer = analyze_ast(&program);
  if !analyzer.is_valid() {
    let errors: Vec<_> = analyzer
      .get_errors()
      .iter()
      .map(|e| e.message.clone())
      .collect();
    return Err(format!("Semantic errors: {}", errors.join("; ")));
  }

  let ir_gen = IrGenerator::new("test");
  ir_gen
    .try_generate_owned(&program, &analyzer, true) // include_stdlib = true
    .map_err(|e| format!("IR generation error: {}", e))
}

#[test]
fn test_ir_gen_simple_def() {
  let result = generate_ir_for_source("var x as i32 = 42");
  assert!(
    result.is_ok(),
    "Simple definition should generate IR without errors"
  );
}

#[test]
fn test_ir_gen_literal_left_of_scalar_dot() {
  // Regression: `4 ⋅ c` in a condition (no resolved type hint) used to ICE
  // because the integer literal's type comes from the right operand (c, an f64
  // identifier) but IR generation only threaded operand types left-to-right.
  let source = r#"
var c as f64 = 3.141592653589793
if (4 ⋅ c) != 0
    write "nonzero"
else
    write "zero"
end if
"#;
  let result = generate_ir_for_source(source);
  assert!(
    result.is_ok(),
    "literal-left scalar dot in a condition should not ICE: {:?}",
    result.err()
  );
}

#[test]
fn test_ir_gen_multiple_defs() {
  let source = r#"
        var x as i32 = 42
        var y as i32 = 99
    "#;
  let result = generate_ir_for_source(source);
  assert!(
    result.is_ok(),
    "Multiple definitions should generate IR without errors"
  );
}

#[test]
fn test_ir_gen_with_expressions() {
  let source = r#"
        var a as i32 = 10
        var b as i32 = 20
        var c as i32 = a + b
    "#;
  let result = generate_ir_for_source(source);
  assert!(
    result.is_ok(),
    "Definitions with expressions should generate IR"
  );
}

#[test]
fn test_ir_gen_write_statement() {
  let source = r#"
        var msg as text = "Hello"
        write msg
    "#;
  let result = generate_ir_for_source(source);
  assert!(result.is_ok(), "Write statement should generate IR");
}

#[test]
fn test_ir_gen_string_literal() {
  let source = r#"write "Hello, World!""#;
  let result = generate_ir_for_source(source);
  assert!(result.is_ok(), "String literal should generate IR");
}

#[test]
fn test_ir_gen_arithmetic() {
  let source = r#"
        var a as i32 = 10
        var b as i32 = 5
        var sum as i32 = a + b
        var diff as i32 = a - b
        var product as i32 = a * b
        var quotient as i32 = a / b
    "#;
  let result = generate_ir_for_source(source);
  assert!(result.is_ok(), "Arithmetic operations should generate IR");
}

#[test]
fn test_ir_gen_logical_operators() {
  let source = r#"
        var a as bool = true
        var b as bool = false
        var result1 as bool = a and b
        var result2 as bool = a or b
    "#;
  let result = generate_ir_for_source(source);
  assert!(result.is_ok(), "Logical operators should generate IR");
}

// Note: Comparison operators with variables in expression position may have
// some grammar precedence issues. Basic comparison should work, but complex
// expressions may need grammar refinement.

// Note: Unary operator parsing has some grammar issues with prefix minus
// and logical not. These will be addressed in grammar refinement.
// The IR generation for unary operators is implemented, but testing
// requires fixing the parser first.

#[test]
fn test_ir_gen_hex_literals() {
  let source = r#"
        var hex1 as i32 = 0xFF
        var hex2 as i32 = 0x10
    "#;
  let result = generate_ir_for_source(source);
  assert!(result.is_ok(), "Hex literals should generate IR");
}

#[test]
fn test_ir_gen_float_literals() {
  let source = r#"
        var pi as f64 = 3.14159
        var e as f32 = 2.71828
    "#;
  let result = generate_ir_for_source(source);
  assert!(result.is_ok(), "Float literals should generate IR");
}

#[test]
fn test_ir_gen_char_literals() {
  let source = r#"
        var c as char = 'A'
    "#;
  let result = generate_ir_for_source(source);
  assert!(result.is_ok(), "Char literals should generate IR");
}

#[test]
fn test_ir_gen_boolean_literals() {
  let source = r#"
        var t as bool = true
        var f as bool = false
    "#;
  let result = generate_ir_for_source(source);
  assert!(result.is_ok(), "Boolean literals should generate IR");
}

// Note: Bitwise operations with variables in expression position may have
// grammar precedence issues. The IR generation is implemented, but testing
// requires grammar refinement.

#[test]
fn test_ir_gen_unsigned_integers() {
  let source = r#"
        var a as u8 = 255
        var b as u16 = 65535
        var c as u32 = 4294967295
        var d as u64 = 18446744073709551615
    "#;
  let result = generate_ir_for_source(source);
  assert!(
    result.is_ok(),
    "Unsigned integer literals should generate IR"
  );
}

#[test]
fn test_ir_gen_signed_integers() {
  let source = r#"
        var a as i8 = -128
        var b as i16 = -32768
        var c as i32 = -2147483648
        var d as i64 = -9223372036854775808
    "#;
  let result = generate_ir_for_source(source);
  assert!(result.is_ok(), "Signed integer literals should generate IR");
}

#[test]
fn test_ir_gen_module_info() {
  let result = generate_ir_for_source("var x as i32 = 1");
  assert!(result.is_ok());
  let module = result.unwrap();

  // Verify that the module is created with a name
  assert!(!module.name.is_empty(), "Module should have a name");
}

// ==================== PHASE 1 TESTS ====================

#[test]
fn test_phase1_simple_assignment() {
  let source = r#"
        var x as i32 = 5
        x = 10
    "#;
  let result = generate_ir_for_source(source);
  assert!(result.is_ok(), "Simple assignment should generate IR");
}

#[test]
fn test_phase1_multiple_reassignments() {
  let source = r#"
        var x as i32 = 5
        x = 10
        x = 15
        x = 20
    "#;
  let result = generate_ir_for_source(source);
  assert!(result.is_ok(), "Multiple reassignments should generate IR");
}

#[test]
fn test_phase1_reassignment_with_expression() {
  let source = r#"
        var x as i32 = 5
        x = x + (1 as i32)
    "#;
  let result = generate_ir_for_source(source);
  assert!(
    result.is_ok(),
    "Reassignment with expression should generate IR"
  );
}

#[test]
fn test_phase1_reassignment_across_types() {
  let source = r#"
        var x as f64 = 3.14
        x = 2.0
    "#;
  let result = generate_ir_for_source(source);
  assert!(
    result.is_ok(),
    "Reassignment across types should generate IR"
  );
}

#[test]
fn test_phase1_compound_assign_add() {
  let source = r#"
        var x as i32 = 10
        x += 5
    "#;
  let result = generate_ir_for_source(source);
  assert!(result.is_ok(), "+= should generate IR");
}

#[test]
fn test_phase1_compound_assign_sub() {
  let source = r#"
        var x as i32 = 10
        x -= 3
    "#;
  let result = generate_ir_for_source(source);
  assert!(result.is_ok(), "-= should generate IR");
}

#[test]
fn test_phase1_compound_assign_mul() {
  let source = r#"
        var x as i32 = 10
        x *= 2
    "#;
  let result = generate_ir_for_source(source);
  assert!(result.is_ok(), "*= should generate IR");
}

#[test]
fn test_phase1_compound_assign_div() {
  let source = r#"
        var x as i32 = 10
        x /= 2
    "#;
  let result = generate_ir_for_source(source);
  assert!(result.is_ok(), "/= should generate IR");
}

#[test]
fn test_phase1_compound_assign_mod() {
  let source = r#"
        var x as i32 = 10
        x %= 3
    "#;
  let result = generate_ir_for_source(source);
  assert!(result.is_ok(), "%= should generate IR");
}

#[test]
fn test_phase1_compound_assign_chain() {
  let source = r#"
        var x as i32 = 10
        x += 1
        x *= 2
        x -= 3
    "#;
  let result = generate_ir_for_source(source);
  assert!(
    result.is_ok(),
    "Chained compound assignments should generate IR"
  );
}

#[test]
fn test_phase1_compound_assign_with_expression() {
  let source = r#"
        var x as i32 = 10
        var y as i32 = 5
        x += y + (1 as i32)
    "#;
  let result = generate_ir_for_source(source);
  assert!(
    result.is_ok(),
    "Compound assign with expression should generate IR"
  );
}

#[test]
fn test_phase1_if_simple() {
  let source = r#"
        var x as i32 = 5
        if x > (0 as i32)
            write "positive"
        else
            move on
        end if
    "#;
  let result = generate_ir_for_source(source);
  assert!(result.is_ok(), "Simple if statement should generate IR");
}

#[test]
fn test_phase1_if_else() {
  let source = r#"
        var x as i32 = 5
        if x > (0 as i32)
            write "positive"
        else
            write "non-positive"
        end if
    "#;
  let result = generate_ir_for_source(source);
  assert!(result.is_ok(), "If-else statement should generate IR");
}

#[test]
fn test_phase1_if_else_if() {
  let source = r#"
        var x as i32 = 5
        match
            when x > (0 as i32):
                write "positive"
            when x < (0 as i32):
                write "negative"
            else:
                write "zero"
        end match
    "#;
  let result = generate_ir_for_source(source);
  assert!(result.is_ok(), "Match statement should generate IR");
}

#[test]
fn test_phase1_if_multiple_else_if() {
  let source = r#"
        var x as i32 = 5
        match
            when x > (10 as i32):
                write "greater than 10"
            when x > (5 as i32):
                write "greater than 5"
            when x > (0 as i32):
                write "greater than 0"
            else:
                write "zero or negative"
        end match
    "#;
  let result = generate_ir_for_source(source);
  assert!(
    result.is_ok(),
    "Match with multiple when arms should generate IR"
  );
}

#[test]
fn test_phase1_if_nested() {
  let source = r#"
        var x as i32 = 5
        var y as i32 = 10
        if x > (0 as i32)
            if y > (0 as i32)
                write "both positive"
            else
                move on
            end if
        else
            move on
        end if
    "#;
  let result = generate_ir_for_source(source);
  assert!(result.is_ok(), "Nested if statements should generate IR");
}

#[test]
fn test_phase1_if_with_multiple_statements() {
  let source = r#"
        var x as i32 = 5
        if x > (0 as i32)
            var y as i32 = 10
            write "positive"
            write y
        else
            move on
        end if
    "#;
  let result = generate_ir_for_source(source);
  assert!(
    result.is_ok(),
    "If with multiple statements should generate IR"
  );
}

#[test]
fn test_phase1_loop_range_simple() {
  let source = r#"
        loop var i as i32 from (0 as i32) to (5 as i32)
            write i
        end loop
    "#;
  let result = generate_ir_for_source(source);
  assert!(result.is_ok(), "Simple range loop should generate IR");
}

#[test]
fn test_phase1_loop_range_with_body() {
  let source = r#"
        var sum as i32 = (0 as i32)
        loop var i as i32 from (1 as i32) to (10 as i32)
            sum = sum + i
        end loop
        write sum
    "#;
  let result = generate_ir_for_source(source);
  assert!(result.is_ok(), "Range loop with body should generate IR");
}

#[test]
fn test_phase1_loop_infinite_with_break() {
  let source = r#"
        var x as i32 = (0 as i32)
        loop
            x = x + (1 as i32)
            when x > (5 as i32)
                exit loop
            end when
        end loop
    "#;
  let result = generate_ir_for_source(source);
  assert!(
    result.is_ok(),
    "Infinite loop with break should generate IR"
  );
}

#[test]
fn test_phase1_loop_with_condition() {
  let source = r#"
        var x as i32 = (0 as i32)
        loop when x < (10 as i32)
            x = x + (1 as i32)
        end loop when not (x < (10 as i32))
    "#;
  let result = generate_ir_for_source(source);
  assert!(result.is_ok(), "Loop with condition should generate IR");
}

#[test]
fn test_phase1_loop_nested() {
  let source = r#"
        loop var i as i32 from (0 as i32) to (3 as i32)
            loop var j as i32 from (0 as i32) to (3 as i32)
                write i
                write j
            end loop
        end loop
    "#;
  let result = generate_ir_for_source(source);
  assert!(result.is_ok(), "Nested loops should generate IR");
}

#[test]
fn test_phase1_loop_with_break_in_if() {
  let source = r#"
        loop var i as i32 from (0 as i32) to (10 as i32)
            when i > (5 as i32)
                exit loop
            end when
            write i
        end loop
    "#;
  let result = generate_ir_for_source(source);
  assert!(
    result.is_ok(),
    "Loop with break inside if should generate IR"
  );
}

#[test]
fn test_phase1_integration_assignment_in_if() {
  let source = r#"
        var x as i32 = 5
        if x > (0 as i32)
            x = (10 as i32)
            write x
        else
            move on
        end if
    "#;
  let result = generate_ir_for_source(source);
  assert!(
    result.is_ok(),
    "Assignment inside if statement should generate IR"
  );
}

#[test]
fn test_phase1_integration_assignment_in_loop() {
  let source = r#"
        var x as i32 = (0 as i32)
        loop var i as i32 from (1 as i32) to (5 as i32)
            x = x + i
        end loop
        write x
    "#;
  let result = generate_ir_for_source(source);
  assert!(result.is_ok(), "Assignment inside loop should generate IR");
}

#[test]
fn test_phase1_integration_complex_control_flow() {
  let source = r#"
        var x as i32 = (0 as i32)
        loop var i as i32 from (0 as i32) to (10 as i32)
            when i > (5 as i32)
                exit loop
            end when
            x = x + i
        end loop
        if x > (20 as i32)
            write "large sum"
        else
            write "small sum"
        end if
    "#;
  let result = generate_ir_for_source(source);
  assert!(result.is_ok(), "Complex control flow should generate IR");
}

// ========== Phase 2 Tests: Data Structure Access ==========

#[test]
fn test_phase2_type_conversion_int_to_int() {
  let source = r#"
        var x as i32 = 42
        var y as i64 = (x as i64)
    "#;
  let result = generate_ir_for_source(source);
  assert!(
    result.is_ok(),
    "Type conversion (int->int) should generate IR: {:?}",
    result.err()
  );
}

#[test]
fn test_phase2_type_conversion_int_to_float() {
  let source = r#"
        var x as i32 = 42
        var y as f64 = (x as f64)
    "#;
  let result = generate_ir_for_source(source);
  assert!(
    result.is_ok(),
    "Type conversion (int->float) should generate IR: {:?}",
    result.err()
  );
}

#[test]
fn test_phase2_type_conversion_widening_chain() {
  let source = r#"
        var x as i32 = 10
        var y as i64 = (x as i64)
        var z as f64 = (y as f64)
    "#;
  let result = generate_ir_for_source(source);
  assert!(
    result.is_ok(),
    "Type conversion widening chain should generate IR: {:?}",
    result.err()
  );
}

#[test]
fn test_phase2_type_conversion_with_literals() {
  let source = r#"
        var x as f64 = (42 as f64)
        var y as f64 = (3.14 as f64)
    "#;
  let result = generate_ir_for_source(source);
  assert!(
    result.is_ok(),
    "Type conversion with literals should generate IR: {:?}",
    result.err()
  );
}

// Note: Array bounds checking for literal indices is implemented in the semantic analyzer.
// The SemanticAnalyzer::visit_array_index() method checks for literal 0 or negative indices
// and reports an error when found. This ensures Lale's 1-based indexing is enforced at
// compile time, catching off-by-one errors early.

// ==================== FUNCTION PARAMETER PASSING TESTS ====================

#[test]
fn test_function_parameter_passing_simple_addition() {
  let source = r#"
        fn add(x as i32, y as i32) returns i32
            return x + y
        end fn

        var result as i32 = add(5, 3)
    "#;
  let result = generate_ir_for_source(source);
  assert!(
    result.is_ok(),
    "Function parameter passing should generate IR: {:?}",
    result.err()
  );
}

#[test]
fn test_function_parameter_passing_with_floats() {
  let source = r#"
        fn multiply(a as f64, b as f64) returns f64
            return a * b
        end fn

        var product as f64 = multiply(2.5, 4.0)
    "#;
  let result = generate_ir_for_source(source);
  assert!(
    result.is_ok(),
    "Float parameter passing should generate IR: {:?}",
    result.err()
  );
}

#[test]
fn test_function_parameter_passing_multiple_calls() {
  let source = r#"
        fn double(x as i32) returns i32
            return x + x
        end fn

        var first as i32 = double(10)
        var second as i32 = double(20)
        var third as i32 = double(30)
    "#;
  let result = generate_ir_for_source(source);
  assert!(
    result.is_ok(),
    "Multiple function calls should generate IR: {:?}",
    result.err()
  );
}

#[test]
fn test_function_parameter_passing_nested_calls() {
  let source = r#"
        fn add(x as i32, y as i32) returns i32
            return x + y
        end fn

        fn double_add(a as i32, b as i32) returns i32
            var sum as i32 = add(a, b)
            return add(sum, sum)
        end fn

        var result as i32 = double_add(5, 3)
    "#;
  let result = generate_ir_for_source(source);
  assert!(
    result.is_ok(),
    "Nested function calls should generate IR: {:?}",
    result.err()
  );
}

#[test]
fn test_function_parameter_passing_with_expressions() {
  let source = r#"
        fn subtract(x as i32, y as i32) returns i32
            return x - y
        end fn

        var a as i32 = 100
        var b as i32 = 25
        var result as i32 = subtract(a + b, a - b)
    "#;
  let result = generate_ir_for_source(source);
  assert!(
    result.is_ok(),
    "Function calls with expression arguments should generate IR: {:?}",
    result.err()
  );
}
