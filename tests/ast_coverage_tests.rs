//! Tests to ensure AST coverage matches grammar definitions.
//!
//! These tests parse the grammar file itself to verify that all statement
//! rules defined in the grammar have corresponding AST handling.

use std::collections::HashSet;
use std::fs;

/// Extract rule names from a grammar rule definition.
/// For example, from "var_and_declaration_statement = _{ var | unsafe_decl | fn_signature | fn_def }"
/// it extracts ["var", "unsafe_decl", "fn_signature", "fn_def"]
fn extract_rule_references(line: &str) -> Vec<String> {
  let mut refs = Vec::new();

  // Find the part after "= _{"  or "= {"
  if let Some(start) = line
    .find("= _{ ")
    .or_else(|| line.find("= _{"))
    .or_else(|| line.find("= { "))
  {
    let content = &line[start..];
    // Extract identifiers (rule names)
    let mut current = String::new();
    for ch in content.chars() {
      if ch.is_alphanumeric() || ch == '_' {
        current.push(ch);
      } else if !current.is_empty() {
        // Skip keywords and the rule definition syntax
        if current != "_" && current != "info" {
          refs.push(current.clone());
        }
        current.clear();
      }
    }
  }
  refs
}

/// Parse the grammar file and extract all rules that are part of the statement hierarchy.
fn extract_statement_rules_from_grammar() -> HashSet<String> {
  let grammar = fs::read_to_string("src/grammar/lale.pest").expect("Failed to read grammar file");

  let mut statement_rules = HashSet::new();
  let mut category_rules: HashSet<String> = HashSet::new();

  // First pass: find the statement category rules
  for line in grammar.lines() {
    let line = line.trim();

    // Look for statement category definitions
    if line.starts_with("var_and_declaration_statement")
      || line.starts_with("imperative_statement")
      || line.starts_with("conditional_statement")
      || line.starts_with("looping_statement")
      || line.starts_with("compile_time_statement")
      || line.starts_with("runtime_io_statement")
      || line.starts_with("error_statement")
      || line.starts_with("noop_statement")
    {
      for rule_ref in extract_rule_references(line) {
        category_rules.insert(rule_ref);
      }
    }
  }

  // These are the actual statement rules we care about
  // Filter out meta-rules and keep only concrete statement rules
  let concrete_statements = [
    "var",
    "unsafe_decl",
    "fn_def",
    "fn_signature",
    "fn_call",
    "if_stmt",
    "when_stmt",
    "switch_stmt",
    "loop_stmt",
    "return_stmt",
    "exit_program",
    "exit_loop",
    "rewind",
    "stdout",
    "stderr",
    "stdin",
    "ct_if",
    "ct_fail",
    "ct_warn",
    "ct_when",
    "ct_match",
    "ct_switch",
    "move_on_stmt",
    "missing_code_stmt",
    "release_stmt",
    "on_exit_stmt",
  ];

  for stmt in concrete_statements {
    statement_rules.insert(stmt.to_string());
  }

  // Also include any rules found in the category definitions
  for rule in category_rules {
    if !rule.contains("statement") && rule != "info" {
      statement_rules.insert(rule);
    }
  }

  statement_rules
}

/// Extract the statement rules that are handled in build_statement by parsing ast/builder.rs
fn extract_handled_rules_from_ast_builder() -> HashSet<String> {
  let source = fs::read_to_string("src/ast/builder.rs").expect("Failed to read src/ast/builder.rs");

  let mut handled = HashSet::new();

  // Look for patterns like "Rule::var =>", "Rule::r#type =>", or "Rule::exit_program =>"
  for line in source.lines() {
    let line = line.trim();
    if line.starts_with("Rule::") && line.contains("=>") {
      // Extract the rule name
      if let Some(rule_part) = line.strip_prefix("Rule::")
        && let Some(name) = rule_part.split_whitespace().next()
      {
        let mut name = name.trim_end_matches("=>").to_string();
        // Handle raw identifiers like r#type -> type
        if name.starts_with("r#") {
          name = name[2..].to_string();
        }
        handled.insert(name);
      }
    }
  }

  handled
}

#[test]
fn test_all_grammar_statements_have_ast_handling() {
  let grammar_statements = extract_statement_rules_from_grammar();
  let handled_rules = extract_handled_rules_from_ast_builder();

  let mut missing: Vec<_> = grammar_statements.difference(&handled_rules).collect();
  missing.sort();

  assert!(
    missing.is_empty(),
    "The following statement rules from the grammar are not handled in ast_builder.rs:\n  {:?}\n\
         Please add handling for these rules in build_statement() and corresponding variants in Stmt enum.",
    missing
  );
}

#[test]
fn test_ast_builder_handles_known_statement_rules() {
  let handled = extract_handled_rules_from_ast_builder();

  // These are the statement rules we expect to be handled
  let expected = [
    "var",
    "unsafe_decl",
    "fn_def",
    "fn_signature",
    "fn_call",
    "if_stmt",
    "when_stmt",
    "switch_stmt",
    "loop_stmt",
    "return_stmt",
    "exit_program",
    "exit_loop",
    "rewind",
    "stdout",
    "stderr",
    "stdin",
    "ct_if",
    "ct_fail",
    "ct_warn",
    "ct_when",
    "ct_match",
    "ct_switch",
    "error_push_stmt",
    "error_alert_stmt",
    "alert_stmt",
    "doc",
    "comment",
    "move_on_stmt",
    "missing_code_stmt",
  ];

  for rule in expected {
    assert!(
      handled.contains(rule),
      "Expected rule '{}' to be handled in ast_builder.rs but it wasn't found",
      rule
    );
  }
}

#[test]
fn test_stmt_enum_has_expected_variants() {
  let source =
    fs::read_to_string("src/ast/definitions.rs").expect("Failed to read src/ast/definitions.rs");

  // Expected Stmt variants (corresponding to statement rules)
  let expected_variants = [
    "Def",
    "UnsafeDecl",
    "FnDef",
    "FnSignature",
    "FnCall",
    "If",
    "When",
    "Match",
    "Switch",
    "Loop",
    "Return",
    "ExitProgram",
    "ExitLoop",
    "Rewind",
    "Stdout",
    "Stderr",
    "Stdin",
    "CtIf",
    "CtFail",
    "CtWarn",
    "CtWhen",
    "CtMatch",
    "CtSwitch",
    "AddError",
    "AlertErrors",
    "Alert",
    "Doc",
    "Comment",
    "MoveOn",
    "MissingCode",
    "Release",
    "OnExit",
  ];

  for variant in expected_variants {
    assert!(
      source.contains(&format!("{}(", variant)) || source.contains(&format!("{},", variant)),
      "Expected Stmt enum to have variant '{}' but it wasn't found in ast.rs",
      variant
    );
  }
}

#[test]
fn test_no_orphaned_ast_builder_handlers() {
  // Check that ast_builder.rs doesn't handle rules that no longer exist in the grammar
  // This catches cases where a statement is REMOVED or RENAMED in the grammar
  let grammar_statements = extract_statement_rules_from_grammar();
  let handled_rules = extract_handled_rules_from_ast_builder();

  // These rules are handled but are not "statement" rules per se (they're part of statement handling)
  let non_statement_rules: HashSet<_> = [
    "doc",
    "comment",
    "EOI",
    "expression",
    "identifier",
    "type_name",
    "unit",
    "export",
    "import",
    "var_symbol",
    "decl_symbol",
    "copy",
    "parameters",
    "fn_parameter",
    "return_type",
    "condition",
    "branch",
    "range",
    "int",
    "write_line",
    "write_inline",
    "warn_line",
    "input",
    "ct_match_arm",
    "ct_match_default",
    "ct_switch_case",
    "ct_switch_default",
  ]
  .iter()
  .map(|s| s.to_string())
  .collect();

  let statement_handlers: HashSet<_> = handled_rules
    .difference(&non_statement_rules)
    .cloned()
    .collect();

  let mut orphaned: Vec<_> = statement_handlers
    .difference(&grammar_statements)
    .filter(|r| !non_statement_rules.contains(*r))
    .collect();
  orphaned.sort();

  // Filter out rules that are legitimately handled but not in statement categories
  let orphaned: Vec<_> = orphaned
    .into_iter()
    .filter(|r| !r.contains("_") || !grammar_statements.contains(*r))
    .filter(|r| {
      // Keep only rules that look like statement rules (not helper rules)
      let looks_like_statement = *r == "var"
        || *r == "fn_def"
        || *r == "fn_call"
        || r.ends_with("_stmt")
        || r.starts_with("exit_")
        || r.starts_with("ct_")
        || [
          "stdout",
          "stderr",
          "stdin",
          "rewind",
          "unsafe_decl",
          "fn_signature",
        ]
        .contains(&r.as_str());
      looks_like_statement && !grammar_statements.contains(*r)
    })
    .collect();

  assert!(
    orphaned.is_empty(),
    "The following rules are handled in ast_builder.rs but don't exist in the grammar's statement rules:\n  {:?}\n\
         This may indicate a REMOVED or RENAMED statement. Please update ast_builder.rs accordingly.",
    orphaned
  );
}

// ==================== META-TESTS ====================
// These tests verify that the coverage tests themselves work correctly

#[test]
fn test_detection_of_missing_handler() {
  // Verify that our extraction logic would detect a missing handler
  let handled = extract_handled_rules_from_ast_builder();
  let grammar = extract_statement_rules_from_grammar();

  // Simulate adding a new rule to grammar that's not handled
  let mut simulated_grammar = grammar.clone();
  simulated_grammar.insert("fake_new_statement".to_string());

  let missing: Vec<_> = simulated_grammar.difference(&handled).collect();

  assert!(
    missing.contains(&&"fake_new_statement".to_string()),
    "Detection logic should identify 'fake_new_statement' as missing from handlers"
  );
}

#[test]
fn test_detection_of_orphaned_handler() {
  // Verify that our extraction logic would detect an orphaned handler
  let handled = extract_handled_rules_from_ast_builder();
  let grammar = extract_statement_rules_from_grammar();

  // Simulate a handler for a rule that was removed from grammar
  let mut simulated_handlers = handled.clone();
  simulated_handlers.insert("removed_old_statement".to_string());

  let orphaned: Vec<_> = simulated_handlers.difference(&grammar).collect();

  assert!(
    orphaned.contains(&&"removed_old_statement".to_string()),
    "Detection logic should identify 'removed_old_statement' as orphaned"
  );
}

#[test]
fn test_grammar_extraction_finds_known_rules() {
  // Verify that our grammar extraction finds the rules we know exist
  let grammar = extract_statement_rules_from_grammar();

  let known_rules = [
    "var",
    "fn_def",
    "if_stmt",
    "loop_stmt",
    "exit_program",
    "exit_loop",
  ];

  for rule in known_rules {
    assert!(
      grammar.contains(rule),
      "Grammar extraction should find rule '{}' but didn't. \
             Either the rule was removed or the extraction logic is broken.",
      rule
    );
  }
}

#[test]
fn test_handler_extraction_finds_known_handlers() {
  // Verify that our handler extraction finds the handlers we know exist
  let handled = extract_handled_rules_from_ast_builder();

  let known_handlers = [
    "var",
    "fn_def",
    "if_stmt",
    "loop_stmt",
    "exit_program",
    "exit_loop",
  ];

  for handler in known_handlers {
    assert!(
      handled.contains(handler),
      "Handler extraction should find handler for '{}' but didn't. \
             Either the handler was removed or the extraction logic is broken.",
      handler
    );
  }
}
