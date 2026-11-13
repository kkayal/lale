use lale::ast::builder::build_program;
use lale::semantic_analysis::{AnalyzerResults, OwnedAnalyzer, SemanticError, analyze_ast};
use lale::{LaleParser, Rule};
use pest::Parser;
use pest::iterators::Pair;

/// Grammar tests for the array type notation proposed in `doc/array_type.md`.
///
/// Two properties are the point of this file:
///
/// 1. The nested spelling `array of array of T` **parses** — the grammar accepts it on purpose —
///    and is then **rejected by semantic analysis** with a message that names the fix, while
///    `array of T` is accepted (proposal §6).
/// 2. `2D` / `3D` / `4D` are read as a **rank token**, not as the literal `2` followed by
///    the identifier `D` (§6, and the lexical note in §8.2).
///
/// The bracketed forms `i32[5]` and `[i32]` are still accepted on this branch; they are
/// removed at sign-off (proposal §8.2), so they appear here as regression tests.
///
/// ## Why `parses_fully` and not `is_ok()`
///
/// `LaleParser::parse` succeeds when the rule matches a **prefix** of the input — trailing
/// text is not rejected. So `assert!(parse(...).is_ok())` can pass on a partial match, and
/// `assert!(parse(...).is_err())` would not notice that `array of i32 of 5 elementsX` matched just
/// `array of i32`. Every assertion below therefore compares the matched span with the
/// input, which is what makes "accepted" and "rejected" mean something.

// ==================== HELPERS ====================

/// Does `src` parse **in full** as `rule`?
fn parses_fully(rule: Rule, src: &str) -> bool {
  match LaleParser::parse(rule, src) {
    Ok(mut pairs) => pairs
      .next()
      .map(|pair| pair.as_str() == src)
      .unwrap_or(false),
    Err(_) => false,
  }
}

/// The text of the first `rule` anywhere in the tree, if present.
fn text_of(pair: Pair<'_, Rule>, rule: Rule) -> Option<String> {
  if pair.as_rule() == rule {
    return Some(pair.as_str().to_string());
  }
  for inner in pair.into_inner() {
    if let Some(found) = text_of(inner, rule) {
      return Some(found);
    }
  }
  None
}

/// The text of the first `rule` in the tree of `src` parsed as `rule_at_top`.
fn inner_text(rule_at_top: Rule, src: &str, rule: Rule) -> Option<String> {
  let mut pairs = match LaleParser::parse(rule_at_top, src) {
    Ok(p) => p,
    Err(e) => panic!("`{src}` should parse as {rule_at_top:?}, but failed: {e}"),
  };
  match pairs.next() {
    Some(top) => text_of(top, rule),
    None => panic!("`{src}` produced no parse tree for {rule_at_top:?}"),
  }
}

/// Does any error message contain `needle`?
fn has_error(errors: &[SemanticError], needle: &str) -> bool {
  errors.iter().any(|e| e.message.contains(needle))
}

/// Semantic errors for a whole program, through the real parse → AST → analyze pipeline.
fn analyze(code: &str) -> OwnedAnalyzer {
  let pairs = match LaleParser::parse(Rule::program, code) {
    Ok(p) => p,
    Err(e) => panic!("`{code}` should parse, but failed: {e}"),
  };
  let program = match build_program(pairs, "test.lale") {
    Ok(p) => p,
    Err(e) => panic!("`{code}` should build an AST, but failed: {e:?}"),
  };
  analyze_ast(&program)
}

// ==================== 1. THE ARRAY TYPE ====================

#[test]
fn test_array_of_scalar_type_parses_fully() {
  assert!(parses_fully(Rule::type_name, "array of i32"));
}

#[test]
fn test_array_type_parses_in_a_var_symbol() {
  assert!(parses_fully(Rule::var_symbol, "var xs as array of i32"));
}

#[test]
fn test_array_of_user_type_parses_fully() {
  assert!(parses_fully(Rule::type_name, "array of Sample"));
}

#[test]
fn test_array_of_vector_parses_fully() {
  // Not expressible with the bracketed form: `[vec2 of f64]` is a grammar error.
  assert!(parses_fully(Rule::type_name, "array of vec2 of f64"));
}

#[test]
fn test_array_type_with_unit_clause_parses_fully() {
  assert!(parses_fully(
    Rule::var_symbol,
    "var xs as array of f64 in <m>"
  ));
}

// ==================== 2. THE EXTENT CLAUSE ====================

#[test]
fn test_extent_clause_parses_fully() {
  assert!(parses_fully(Rule::type_name, "array of i32 of 5 elements"));
}

#[test]
fn test_variable_extent_parses_fully() {
  assert!(parses_fully(Rule::type_name, "array of i32 of n elements"));
}

#[test]
fn test_extent_may_be_an_expression() {
  assert!(parses_fully(
    Rule::type_name,
    "array of i32 of n + 1 elements"
  ));
}

#[test]
fn test_two_dimensional_extents_parse_fully() {
  assert!(parses_fully(
    Rule::type_name,
    "2D array of i32 of 5 by 3 elements"
  ));
}

#[test]
fn test_unsafe_decl_with_extent_clause_parses_fully() {
  assert!(parses_fully(
    Rule::unsafe_decl,
    "unsafe decl buffer as array of i32 of 5 elements"
  ));
}

// ==================== 3. THE RANK TOKEN ====================

#[test]
fn test_two_dimensional_array_parses_fully() {
  assert!(parses_fully(Rule::type_name, "2D array of i32"));
}

#[test]
fn test_rank_is_a_rank_token_not_a_literal_plus_identifier() {
  // If `2D` were read as the literal `2` followed by the identifier `D`, the type position
  // could not continue after the `2`, and neither of these would hold.
  assert!(parses_fully(Rule::type_name, "2D array of i32"));
  assert_eq!(
    inner_text(Rule::type_name, "2D array of i32", Rule::rank).as_deref(),
    Some("2D"),
    "`2D` must be a single `rank` token, not `2` plus an identifier"
  );
}

#[test]
fn test_rank_3d_parses_fully() {
  assert_eq!(
    inner_text(Rule::type_name, "3D array of i32", Rule::rank).as_deref(),
    Some("3D")
  );
}

#[test]
fn test_rank_4d_parses_fully() {
  assert_eq!(
    inner_text(Rule::type_name, "4D array of f64", Rule::rank).as_deref(),
    Some("4D")
  );
}

#[test]
fn test_multi_digit_rank_parses_fully() {
  assert_eq!(
    inner_text(Rule::type_name, "12D array of i32", Rule::rank).as_deref(),
    Some("12D")
  );
}

#[test]
fn test_rank_guard_rejects_a_glued_name() {
  // `2Dfoo` is neither a rank (`2D` glued to `foo`) nor an identifier (digit-led).
  assert!(!parses_fully(Rule::type_name, "2Dfoo array of i32"));
}

// ============ 4. THE NESTED SPELLING PARSES, THEN IS REJECTED IN SEMANTIC ANALYSIS ============

#[test]
fn test_nested_array_of_array_parses_in_the_grammar() {
  // The grammar accepts the nested spelling on purpose: the ban is enforced in semantic analysis,
  // so that the diagnostic can name the fix instead of reporting an unexpected token
  // (doc/array_type.md §6).
  assert!(parses_fully(Rule::type_name, "array of array of i32"));
}

#[test]
fn test_nested_spelling_with_extents_parses_in_the_grammar() {
  assert!(parses_fully(
    Rule::type_name,
    "array of array of i32 of 5 by 3 elements"
  ));
}

#[test]
fn test_three_level_nesting_parses_in_the_grammar() {
  assert!(parses_fully(
    Rule::type_name,
    "array of array of array of i32"
  ));
}

#[test]
fn test_ranked_nested_spelling_parses_in_the_grammar() {
  assert!(parses_fully(Rule::type_name, "2D array of array of i32"));
}

#[test]
fn test_nested_spelling_reaches_semantic_analysis() {
  // It must get past the AST build, which is what makes the semantic diagnostic possible.
  let analyzer = analyze("var m as array of array of i32 = [[1, 2], [3, 4]]\n");
  assert!(!analyzer.get_errors().is_empty());
}

#[test]
fn test_semantic_analysis_rejects_the_nested_spelling() {
  let analyzer = analyze("var m as array of array of i32 = [[1, 2], [3, 4]]\n");
  assert!(
    has_error(analyzer.get_errors(), "may not itself be an array"),
    "errors: {:?}",
    analyzer.get_errors()
  );
  assert!(
    has_error(analyzer.get_errors(), "2D array of i32"),
    "the diagnostic must name the fix; errors: {:?}",
    analyzer.get_errors()
  );
}

#[test]
fn test_semantic_analysis_suggests_3d_for_three_level_nesting() {
  let analyzer = analyze("var m as array of array of array of i32 = []\n");
  assert!(
    has_error(analyzer.get_errors(), "3D array of i32"),
    "errors: {:?}",
    analyzer.get_errors()
  );
}

#[test]
fn test_nested_spelling_in_a_struct_field_is_rejected() {
  let analyzer = analyze("type S\n  rows as array of array of i32\nend type\n");
  assert!(
    has_error(analyzer.get_errors(), "may not itself be an array"),
    "errors: {:?}",
    analyzer.get_errors()
  );
}

#[test]
fn test_nested_spelling_in_a_parameter_is_rejected() {
  let code = "fn f(xs as array of array of i32) returns nothing\n  move on\nend fn\n";
  let analyzer = analyze(code);
  assert!(
    has_error(analyzer.get_errors(), "may not itself be an array"),
    "errors: {:?}",
    analyzer.get_errors()
  );
}

#[test]
fn test_valid_dynamic_array_is_reported_as_unimplemented() {
  // `array of i32` is valid notation but not implemented on this branch. It must be reported,
  // never silently treated as its element type.
  let analyzer = analyze("var xs as array of i32 = [1, 2, 3]\n");
  assert!(
    has_error(analyzer.get_errors(), "not implemented yet"),
    "errors: {:?}",
    analyzer.get_errors()
  );
}

// ==================== 5. PRECISION OF THE NEW RULES ====================

#[test]
fn test_identifier_named_array_is_not_reserved() {
  // Lale has no reserved words: `array` on its own is still a (user) type name.
  assert!(parses_fully(Rule::var_symbol, "var x as array"));
}

#[test]
fn test_identifier_named_array_is_allowed_as_an_element_type() {
  assert!(parses_fully(Rule::type_name, "array of array"));
}

#[test]
fn test_elements_word_needs_a_word_boundary() {
  // `elementsX` must not satisfy the extent clause's `elements`.
  assert!(!parses_fully(
    Rule::type_name,
    "array of i32 of 5 elementsX"
  ));
}

#[test]
fn test_array_word_needs_a_word_boundary() {
  // `arrayfoo` is a plain identifier, not the array constructor.
  assert!(!parses_fully(Rule::type_name, "arrayfoo of i32"));
  assert!(parses_fully(Rule::var_symbol, "var x as arrayfoo"));
}

// ==================== 6. REGRESSION: THE OLD FORMS STILL PARSE ====================

#[test]
fn test_bracketed_suffix_form_still_parses_fully() {
  assert!(parses_fully(Rule::type_name, "i32[3]"));
}

#[test]
fn test_bracketed_prefix_form_still_parses_fully() {
  assert!(parses_fully(Rule::type_name, "[i32]"));
}

#[test]
fn test_multi_dimensional_bracketed_form_still_parses_fully() {
  assert!(parses_fully(Rule::type_name, "u32[10][20]"));
}

#[test]
fn test_plain_and_vector_types_still_parse_fully() {
  for src in ["i32", "text", "pointer", "vec3 of f64", "MyType", "i32?"] {
    assert!(
      parses_fully(Rule::type_name, src),
      "`{src}` should still parse as a type"
    );
  }
}

// ==================== 7. A LITERAL *2* IS STILL A LITERAL ====================

#[test]
fn test_integer_literal_still_parses_fully_in_an_expression() {
  assert!(parses_fully(Rule::r#var, "var n as i32 = 2"));
}

#[test]
fn test_rank_token_is_not_an_expression() {
  // `2D` is a type-level token; it must not become a valid expression.
  assert!(!parses_fully(Rule::r#var, "var n as i32 = 2D"));
}
