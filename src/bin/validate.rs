//! Lale Compiler Validation Tool
//!
//! This tool validates the completeness of the Lale compiler by checking:
//! 1. Grammar → AST: All grammar rules have AST Builder handlers
//! 2. AST → IR: All AST nodes have IR generation handlers
//! 3. IR → Execution: All IR instructions have interpreter handlers
//!
//! It also detects orphaned handlers (code that handles constructs that no longer exist).
//!
//! # Usage
//!
//! ```bash
//! cargo run --bin lale-validate
//! cargo run --bin lale-validate -- --stage ast
//! cargo run --bin lale-validate -- --stage ir
//! cargo run --bin lale-validate -- --stage interpreter
//! cargo run --bin lale-validate -- --output json
//! ```

use clap::Parser as ClapParser;
use colored::Colorize;
use std::collections::{HashMap, HashSet};
use std::fs;
use syn::spanned::Spanned;
use syn::visit::Visit;

/// Lale Compiler Validation Tool
#[derive(clap_derive::Parser, Debug)]
#[command(name = "lale-validate")]
#[command(about = "Validates completeness of the Lale compiler stages")]
#[command(version)]
struct Args {
  /// Which stage to validate (default: all)
  #[arg(short = 'S', long, value_enum)]
  stage: Option<Stage>,

  /// Output format
  #[arg(short, long, value_enum, default_value = "text")]
  output: OutputFormat,

  /// Show only missing handlers (hide complete items)
  #[arg(long)]
  missing_only: bool,

  /// Show only summary counts, hide detailed listings
  #[arg(short = 's', long)]
  summary: bool,

  /// Path to the lale source directory (default: src/)
  #[arg(long, default_value = "src")]
  src_dir: String,

  /// Path to the grammar file (default: src/grammar/lale.pest)
  #[arg(long, default_value = "src/grammar/lale.pest")]
  grammar_file: String,
}

#[derive(Debug, Clone, Copy, clap_derive::ValueEnum)]
enum Stage {
  /// Validate Grammar → AST stage
  Ast,
  /// Validate AST → IR stage
  Ir,
  /// Validate IR → Execution stage
  Interpreter,
  /// Validate all stages
  All,
  /// Scan for silently ignored errors
  Silent,
}

#[derive(Debug, Clone, Copy, clap_derive::ValueEnum)]
enum OutputFormat {
  /// Human-readable text output
  Text,
  /// JSON output for programmatic use
  Json,
  /// Markdown output for documentation
  Markdown,
}

/// Result of validating a single item
#[derive(Debug, Clone)]
struct ValidationItem {
  name: String,
  category: String,
  has_handler: bool,
  is_orphan: bool,
  notes: String,
}

/// Result of validating a stage
#[derive(Debug, Clone)]
struct StageValidation {
  stage_name: String,
  items: Vec<ValidationItem>,
  source_constructs: HashSet<String>,
  handled_constructs: HashSet<String>,
}

impl StageValidation {
  fn missing_handlers(&self) -> Vec<&ValidationItem> {
    self
      .items
      .iter()
      .filter(|i| !i.has_handler && !i.is_orphan)
      .collect()
  }

  fn orphaned_handlers(&self) -> Vec<&ValidationItem> {
    self.items.iter().filter(|i| i.is_orphan).collect()
  }

  fn coverage_percent(&self) -> f64 {
    if self.source_constructs.is_empty() {
      return 100.0;
    }
    let handled = self
      .source_constructs
      .intersection(&self.handled_constructs)
      .count();
    (handled as f64 / self.source_constructs.len() as f64) * 100.0
  }
}

/// Type of grammar rule based on Pest syntax
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RuleType {
  /// Normal rule: `rule = { ... }` - produces parse tree node
  Normal,
  /// Silent rule: `rule = _{ ... }` - no parse tree node produced
  Silent,
  /// Atomic rule: `rule = @{ ... }` - captured as single string
  Atomic,
  /// Compound atomic: `rule = ${ ... }` - atomic with inner structure
  CompoundAtomic,
  /// Non-atomic: `rule = !{ ... }` - opposite of atomic
  NonAtomic,
}

impl RuleType {
  fn needs_handler(&self) -> bool {
    match self {
      RuleType::Normal => true,
      RuleType::Silent => false, // Never in parse tree
      RuleType::Atomic => true,  // In parse tree as string
      RuleType::CompoundAtomic => true,
      RuleType::NonAtomic => true,
    }
  }

  /// Whether child rules inside this rule type produce parse tree nodes
  fn children_produce_nodes(&self) -> bool {
    match self {
      RuleType::Normal => true,
      RuleType::Silent => false, // Children inside silent rules don't appear
      RuleType::Atomic => false, // Children inside atomic rules are captured as string
      RuleType::CompoundAtomic => false,
      RuleType::NonAtomic => true,
    }
  }

  fn description(&self) -> &'static str {
    match self {
      RuleType::Normal => "normal",
      RuleType::Silent => "silent (no node)",
      RuleType::Atomic => "atomic (string)",
      RuleType::CompoundAtomic => "compound atomic",
      RuleType::NonAtomic => "non-atomic",
    }
  }
}

/// A grammar rule with its metadata
#[derive(Debug, Clone)]
struct GrammarRule {
  name: String,
  rule_type: RuleType,
  section: String,
  #[allow(dead_code)]
  line_number: usize,
  /// Whether this rule is an internal sub-rule (consumed by parent atomic/silent rules)
  is_internal: bool,
  /// The raw content of the rule definition (for dependency analysis)
  content: String,
}

/// Categorized grammar rules
#[derive(Debug, Clone, Default)]
struct GrammarCatalog {
  rules: Vec<GrammarRule>,
  sections: Vec<String>,
}

/// Type of AST node
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum AstNodeType {
  /// Statement type (Stmt enum variant)
  Statement,
  /// Expression type (Expr enum variant or literal)
  Expression,
  /// Supporting type (not directly used in IR gen)
  Supporting,
  /// Enum definition
  Enum,
}

impl AstNodeType {
  fn needs_ir_handler(&self) -> bool {
    match self {
      AstNodeType::Statement => true,
      AstNodeType::Expression => true,
      AstNodeType::Supporting => false,
      AstNodeType::Enum => false,
    }
  }

  fn description(&self) -> &'static str {
    match self {
      AstNodeType::Statement => "statement",
      AstNodeType::Expression => "expression",
      AstNodeType::Supporting => "supporting",
      AstNodeType::Enum => "enum",
    }
  }
}

/// An AST node with its metadata
#[derive(Debug, Clone)]
struct AstNode {
  name: String,
  node_type: AstNodeType,
  section: String,
}

/// Categorized AST nodes
#[derive(Debug, Clone, Default)]
struct AstCatalog {
  nodes: Vec<AstNode>,
  sections: Vec<String>,
}

/// Type of IR instruction
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum IrInstructionType {
  Arithmetic,
  Bitwise,
  Comparison,
  Logical,
  Memory,
  ControlFlow,
  FunctionCall,
  TypeConversion,
  Constant,
  GlobalAccess,
  UnitAssertion,
  StringOp,
  Vector,
  Struct,
  Optional,
  ErrorStack,
  Safety,
  Uncategorized,
}

impl IrInstructionType {
  fn description(&self) -> &'static str {
    match self {
      IrInstructionType::Arithmetic => "Arithmetic",
      IrInstructionType::Bitwise => "Bitwise",
      IrInstructionType::Comparison => "Comparison",
      IrInstructionType::Logical => "Logical",
      IrInstructionType::Memory => "Memory",
      IrInstructionType::ControlFlow => "Control Flow",
      IrInstructionType::FunctionCall => "Function Call",
      IrInstructionType::TypeConversion => "Type Conversion",
      IrInstructionType::Constant => "Constant",
      IrInstructionType::GlobalAccess => "Global Access",
      IrInstructionType::UnitAssertion => "Unit Assertion",
      IrInstructionType::StringOp => "String Operation",
      IrInstructionType::Vector => "Vector",
      IrInstructionType::Struct => "Struct",
      IrInstructionType::Optional => "Optional",
      IrInstructionType::ErrorStack => "Error Stack",
      IrInstructionType::Safety => "Safety",
      IrInstructionType::Uncategorized => "Uncategorized",
    }
  }
}

/// An IR instruction with its metadata
#[derive(Debug, Clone)]
struct IrInstruction {
  name: String,
  instr_type: IrInstructionType,
}

/// Categorized IR instructions
#[derive(Debug, Clone, Default)]
struct IrCatalog {
  instructions: Vec<IrInstruction>,
}

fn main() {
  let args = Args::parse();

  println!(
    "{}",
    "═══════════════════════════════════════════════════════════════".cyan()
  );
  println!(
    "{}",
    "                 Lale Compiler Validation Tool                  "
      .cyan()
      .bold()
  );
  println!(
    "{}",
    "═══════════════════════════════════════════════════════════════".cyan()
  );
  println!();

  let stage = args.stage.unwrap_or(Stage::All);

  match stage {
    Stage::Ast | Stage::All => {
      let result = validate_ast_stage(&args);
      output_stage_result(&result, &args);
    }
    _ => {}
  }

  match stage {
    Stage::Ir | Stage::All => {
      let result = validate_ir_stage(&args);
      output_stage_result(&result, &args);
    }
    _ => {}
  }

  match stage {
    Stage::Interpreter | Stage::All => {
      let result = validate_interpreter_stage(&args);
      output_stage_result(&result, &args);
    }
    _ => {}
  }

  match stage {
    Stage::Silent | Stage::All => {
      validate_silent_errors_stage(&args);
    }
    _ => {}
  }

  println!();
  println!(
    "{}",
    "═══════════════════════════════════════════════════════════════".cyan()
  );
  println!(
    "{}",
    "                      Validation Complete                       "
      .cyan()
      .bold()
  );
  println!(
    "{}",
    "═══════════════════════════════════════════════════════════════".cyan()
  );
}

// ============================================================================
// Grammar → AST Validation
// ============================================================================

fn validate_ast_stage(args: &Args) -> StageValidation {
  println!(
    "{}",
    "┌─────────────────────────────────────────────────────────────┐".yellow()
  );
  println!(
    "{}",
    "│           Grammar → AST (AST Builder)              │"
      .yellow()
      .bold()
  );
  println!(
    "{}",
    "└─────────────────────────────────────────────────────────────┘".yellow()
  );

  let catalog = extract_grammar_catalog(&args.grammar_file);

  // Extract Rule:: references from both builder.rs and expr_parser.rs
  let mut rule_references = extract_rule_references(&format!("{}/ast/builder.rs", args.src_dir));
  let expr_parser_refs = extract_rule_references(&format!("{}/ast/expr_parser.rs", args.src_dir));
  rule_references.extend(expr_parser_refs);

  // Print grammar summary
  println!();
  println!("  {}", "Grammar Summary:".bold());
  let total_rules = catalog.rules.len();
  let silent_rules: Vec<_> = catalog
    .rules
    .iter()
    .filter(|r| r.rule_type == RuleType::Silent)
    .collect();
  let atomic_rules: Vec<_> = catalog
    .rules
    .iter()
    .filter(|r| r.rule_type == RuleType::Atomic)
    .collect();
  let normal_rules: Vec<_> = catalog
    .rules
    .iter()
    .filter(|r| r.rule_type == RuleType::Normal)
    .collect();
  let internal_rules: Vec<_> = catalog
    .rules
    .iter()
    .filter(|r| r.is_internal && r.rule_type.needs_handler())
    .collect();

  println!("    Total rules: {}", total_rules);
  println!("    Normal rules (need handlers): {}", normal_rules.len());
  println!("    Atomic rules (string capture): {}", atomic_rules.len());
  println!(
    "    Silent rules (no parse node): {} {}",
    silent_rules.len(),
    "(excluded from coverage)".dimmed()
  );
  println!(
    "    Internal sub-rules: {} {}",
    internal_rules.len(),
    "(consumed by parent rules, no handler needed)".dimmed()
  );
  println!();

  // Print sections - only count non-internal rules that need handlers
  println!("  {}", "Sections:".bold());
  for section in &catalog.sections {
    let section_rules: Vec<_> = catalog
      .rules
      .iter()
      .filter(|r| &r.section == section && r.rule_type.needs_handler() && !r.is_internal)
      .collect();
    let handled: Vec<_> = section_rules
      .iter()
      .filter(|r| rule_references.contains(&r.name))
      .collect();
    let pct = if section_rules.is_empty() {
      100.0
    } else {
      (handled.len() as f64 / section_rules.len() as f64) * 100.0
    };
    let status = if pct >= 100.0 {
      "✓".green()
    } else if pct >= 50.0 {
      "◐".yellow()
    } else {
      "✗".red()
    };
    println!(
      "    {} {} ({}/{} = {:.0}%)",
      status,
      section,
      handled.len(),
      section_rules.len(),
      pct
    );
  }
  println!();

  let mut items = Vec::new();

  // Only count non-internal rules that need handlers
  let rules_needing_handlers: HashSet<String> = catalog
    .rules
    .iter()
    .filter(|r| r.rule_type.needs_handler() && !r.is_internal)
    .map(|r| r.name.clone())
    .collect();

  // Check each grammar rule for a handler, grouped by section
  for rule in &catalog.rules {
    if !rule.rule_type.needs_handler() {
      continue; // Skip silent rules
    }

    // For internal rules, mark as handled (they don't need explicit handlers)
    let has_handler = rule_references.contains(&rule.name) || rule.is_internal;
    let note = if rule.is_internal {
      format!(
        "{} ({}, internal sub-rule)",
        rule.section,
        rule.rule_type.description()
      )
    } else {
      format!("{} ({})", rule.section, rule.rule_type.description())
    };
    items.push(ValidationItem {
      name: rule.name.clone(),
      category: rule.section.clone(),
      has_handler,
      is_orphan: false,
      notes: note,
    });
  }

  // Check for orphaned handlers (handlers for rules that don't exist or are silent)
  let all_rule_names: HashSet<String> = catalog.rules.iter().map(|r| r.name.clone()).collect();
  for handler in &rule_references {
    if !all_rule_names.contains(handler) {
      items.push(ValidationItem {
        name: handler.clone(),
        category: "Orphaned Handler".to_string(),
        has_handler: true,
        is_orphan: true,
        notes: "Handler exists but grammar rule was removed".to_string(),
      });
    }
  }

  // Internal rules count as handled in coverage calculation
  let mut handled_with_internal = rule_references.clone();
  for rule in &catalog.rules {
    if rule.is_internal {
      handled_with_internal.insert(rule.name.clone());
    }
  }

  StageValidation {
    stage_name: "Grammar → AST".to_string(),
    items,
    source_constructs: rules_needing_handlers,
    handled_constructs: handled_with_internal,
  }
}

/// Extract comprehensive grammar catalog with rule types and sections
fn extract_grammar_catalog(grammar_path: &str) -> GrammarCatalog {
  let mut catalog = GrammarCatalog::default();

  let content = match fs::read_to_string(grammar_path) {
    Ok(c) => c,
    Err(e) => {
      eprintln!("{}: Could not read grammar file: {}", "Error".red(), e);
      return catalog;
    }
  };

  let mut current_section = "Uncategorized".to_string();
  let lines: Vec<&str> = content.lines().collect();
  let mut i = 0;

  while i < lines.len() {
    let line_number = i + 1;
    let trimmed = lines[i].trim();

    // Detect section headers: // ==================== SECTION NAME ====================
    if trimmed.starts_with("// ==") && trimmed.contains("==") {
      let section_text = trimmed
        .trim_start_matches("//")
        .trim()
        .trim_matches('=')
        .trim();
      if !section_text.is_empty() {
        current_section = section_text.to_string();
        if !catalog.sections.contains(&current_section) {
          catalog.sections.push(current_section.clone());
        }
      }
      i += 1;
      continue;
    }

    // Skip other comments and empty lines
    if trimmed.starts_with("//") || trimmed.is_empty() {
      i += 1;
      continue;
    }

    // Match rule definitions: identifier = { or = _{ or = @{ or = ${ or = !{
    if let Some(equals_pos) = trimmed.find('=') {
      let rule_name = trimmed[..equals_pos].trim();

      // Validate it looks like a rule name (alphanumeric + underscore, starts with letter)
      if rule_name.is_empty()
        || !rule_name.chars().all(|c| c.is_alphanumeric() || c == '_')
        || !rule_name
          .chars()
          .next()
          .map(|c| c.is_alphabetic() || c == '_')
          .unwrap_or(false)
      {
        i += 1;
        continue;
      }

      // Determine rule type from what follows the =
      let after_equals = trimmed[equals_pos + 1..].trim();
      let rule_type = if after_equals.starts_with("_{") {
        RuleType::Silent
      } else if after_equals.starts_with("@{") {
        RuleType::Atomic
      } else if after_equals.starts_with("${") {
        RuleType::CompoundAtomic
      } else if after_equals.starts_with("!{") {
        RuleType::NonAtomic
      } else {
        RuleType::Normal
      };

      // Collect the full rule content (may span multiple lines)
      let mut rule_content = trimmed.to_string();
      let mut brace_count =
        rule_content.matches('{').count() as i32 - rule_content.matches('}').count() as i32;

      while brace_count > 0 && i + 1 < lines.len() {
        let next_line = lines[i + 1].trim();

        // Stop if we hit a section header or new rule definition
        if next_line.starts_with("// ==") {
          break;
        }

        // Stop if we hit a new rule definition (identifier = {)
        if !next_line.starts_with("//")
          && !next_line.is_empty()
          && let Some(eq_pos) = next_line.find('=')
        {
          let potential_name = next_line[..eq_pos].trim();
          let after_eq = next_line[eq_pos + 1..].trim();
          if !potential_name.is_empty()
            && potential_name
              .chars()
              .all(|c| c.is_alphanumeric() || c == '_')
            && (after_eq.starts_with('{')
              || after_eq.starts_with("_{")
              || after_eq.starts_with("@{")
              || after_eq.starts_with("${")
              || after_eq.starts_with("!{"))
          {
            break;
          }
        }

        i += 1;
        rule_content.push(' ');
        rule_content.push_str(next_line);
        brace_count +=
          next_line.matches('{').count() as i32 - next_line.matches('}').count() as i32;
      }

      catalog.rules.push(GrammarRule {
        name: rule_name.to_string(),
        rule_type,
        section: current_section.clone(),
        line_number,
        is_internal: false, // Will be computed later
        content: rule_content,
      });
    }
    i += 1;
  }

  // Now analyze dependencies to find internal rules
  mark_internal_rules(&mut catalog);

  catalog
}

/// Analyze rule dependencies to mark internal sub-rules.
///
/// A rule is considered "internal" if:
/// 1. It starts with underscore (naming convention for internal rules)
/// 2. It is ONLY referenced from atomic or silent rules (its nodes never appear in parse tree)
/// 3. It's a low-level character class (alpha, digit, etc.) only used in atomic rules
fn mark_internal_rules(catalog: &mut GrammarCatalog) {
  // Build a map of which rules reference which other rules
  let mut referenced_from_normal: HashSet<String> = HashSet::new();
  let mut referenced_from_atomic_or_silent: HashSet<String> = HashSet::new();

  for rule in &catalog.rules {
    let references = extract_rule_refs_from_content(&rule.content);

    let produces_nodes = rule.rule_type.children_produce_nodes();

    for ref_name in references {
      if produces_nodes {
        referenced_from_normal.insert(ref_name);
      } else {
        referenced_from_atomic_or_silent.insert(ref_name);
      }
    }
  }

  // Mark rules as internal
  for rule in &mut catalog.rules {
    // Rule starts with underscore - internal by convention
    if rule.name.starts_with('_') {
      rule.is_internal = true;
      continue;
    }

    // kw_ prefix: keyword rules consumed by parent rules — never have standalone handlers
    if rule.name.starts_with("kw_") {
      rule.is_internal = true;
      continue;
    }

    // Rule is only referenced from atomic/silent rules (never produces nodes)
    if referenced_from_atomic_or_silent.contains(&rule.name)
      && !referenced_from_normal.contains(&rule.name)
    {
      rule.is_internal = true;
      continue;
    }

    // Special case: low-level building blocks consumed by parent rules.
    // These rules are referenced from normal rules but never produce standalone
    // parse tree nodes — they exist only as sub-components.
    //
    // MAINTENANCE: When adding new internal sub-rules to the grammar that are
    // referenced from normal (non-silent, non-atomic) rules, add them here.
    // Rules starting with `_` are auto-detected and don't need to be listed.
    let internal_building_blocks = [
      // Character classes
      "alpha",
      "digit",
      "subscript_digits",
      "latin1_safe",
      "combining_marks",
      "letter_ranges",
      "identifier_continue",
      "hex",
      "character",
      "sign",
      "plus",
      "minus",
      // Number literal components
      "decimal_with_exp",
      "decimal_only",
      "integer_with_exp",
      // Physical unit components
      "dividend",
      "divisor",
      "atomic_unit",
      "exponent",
      "superscript",
      // Keyword aliases consumed by parent statement rules
      "input",
      "write_line",
      "warn_line",
      "alert_line",
      "write_inline",
      "warn_inline",
      "alert_inline",
    ];
    if internal_building_blocks.contains(&rule.name.as_str()) {
      rule.is_internal = true;
    }
  }
}

/// Extract rule references from a rule's content
fn extract_rule_refs_from_content(content: &str) -> Vec<String> {
  let mut refs = Vec::new();

  // Simple extraction: find identifiers that could be rule references
  // Skip keywords, operators, and string literals
  let mut chars = content.chars().peekable();

  while let Some(c) = chars.next() {
    // Skip string literals
    if c == '"' {
      while let Some(nc) = chars.next() {
        if nc == '"' {
          break;
        }
        if nc == '\\' {
          chars.next();
        }
      }
      continue;
    }

    // Skip character classes like '0'..'9'
    if c == '\'' {
      for nc in chars.by_ref() {
        if nc == '\'' {
          break;
        }
      }
      continue;
    }

    // Extract identifiers
    if c.is_alphabetic() || c == '_' {
      let mut ident = String::new();
      ident.push(c);
      while let Some(&nc) = chars.peek() {
        if nc.is_alphanumeric() || nc == '_' {
          ident.push(nc);
          chars.next();
        } else {
          break;
        }
      }

      // Skip Pest keywords and operators
      let keywords = [
        "SOI",
        "EOI",
        "ANY",
        "PEEK",
        "POP",
        "PUSH",
        "ASCII_ALPHA",
        "ASCII_DIGIT",
        "NEWLINE",
        "true",
        "false",
      ];
      if !keywords.contains(&ident.as_str()) && !ident.is_empty() {
        refs.push(ident);
      }
    }
  }

  refs
}

/// Extract Rule:: references from the AST builder
fn extract_rule_references(builder_path: &str) -> HashSet<String> {
  let mut references = HashSet::new();

  let content = match fs::read_to_string(builder_path) {
    Ok(c) => c,
    Err(e) => {
      eprintln!("{}: Could not read AST builder: {}", "Error".red(), e);
      return references;
    }
  };

  // Find all Rule::something patterns
  let rule_pattern = "Rule::";
  for line in content.lines() {
    let mut remaining = line;
    while let Some(pos) = remaining.find(rule_pattern) {
      remaining = &remaining[pos + rule_pattern.len()..];

      // Handle Rust raw identifiers: Rule::r#var should extract "var"
      let remaining_after_raw = if let Some(stripped) = remaining.strip_prefix("r#") {
        stripped
      } else {
        remaining
      };

      // Extract the rule name (alphanumeric + underscore)
      let rule_name: String = remaining_after_raw
        .chars()
        .take_while(|c| c.is_alphanumeric() || *c == '_')
        .collect();

      if !rule_name.is_empty() {
        references.insert(rule_name);
      }
    }
  }

  // Exclude built-in pest rules that don't appear in grammar
  let builtin_rules = ["EOI", "SOI", "ANY", "PEEK", "POP", "PUSH"];
  for builtin in builtin_rules {
    references.remove(builtin);
  }

  references
}

// ============================================================================
// AST → IR Validation
// ============================================================================

fn validate_ir_stage(args: &Args) -> StageValidation {
  println!();
  println!(
    "{}",
    "┌─────────────────────────────────────────────────────────────┐".green()
  );
  println!(
    "{}",
    "│              AST → IR (IR Generator)               │"
      .green()
      .bold()
  );
  println!(
    "{}",
    "└─────────────────────────────────────────────────────────────┘".green()
  );

  let catalog = extract_ast_catalog(&format!("{}/ast/definitions.rs", args.src_dir));
  let ir_handlers = extract_ir_gen_handlers(
    &format!("{}/ast/definitions.rs", args.src_dir),
    &format!("{}/ir_gen.rs", args.src_dir),
  );

  // Print AST summary
  println!();
  println!("  {}", "AST Summary:".bold());
  let total_nodes = catalog.nodes.len();
  let stmt_nodes: Vec<_> = catalog
    .nodes
    .iter()
    .filter(|n| n.node_type == AstNodeType::Statement)
    .collect();
  let expr_nodes: Vec<_> = catalog
    .nodes
    .iter()
    .filter(|n| n.node_type == AstNodeType::Expression)
    .collect();
  let supporting_nodes: Vec<_> = catalog
    .nodes
    .iter()
    .filter(|n| n.node_type == AstNodeType::Supporting)
    .collect();
  let enum_nodes: Vec<_> = catalog
    .nodes
    .iter()
    .filter(|n| n.node_type == AstNodeType::Enum)
    .collect();

  println!("    Total AST types: {}", total_nodes);
  println!("    Statement types: {}", stmt_nodes.len());
  println!("    Expression types: {}", expr_nodes.len());
  println!(
    "    Supporting types: {} {}",
    supporting_nodes.len(),
    "(not directly handled)".dimmed()
  );
  println!(
    "    Enum definitions: {} {}",
    enum_nodes.len(),
    "(container types)".dimmed()
  );
  println!();

  // Print sections
  println!("  {}", "Sections:".bold());
  for section in &catalog.sections {
    let section_nodes: Vec<_> = catalog
      .nodes
      .iter()
      .filter(|n| &n.section == section && n.node_type.needs_ir_handler())
      .collect();
    let handled: Vec<_> = section_nodes
      .iter()
      .filter(|n| ir_handlers.contains(&n.name))
      .collect();
    let pct = if section_nodes.is_empty() {
      100.0
    } else {
      (handled.len() as f64 / section_nodes.len() as f64) * 100.0
    };
    let status = if pct >= 100.0 {
      "✓".green()
    } else if pct >= 50.0 {
      "◐".yellow()
    } else {
      "✗".red()
    };
    println!(
      "    {} {} ({}/{} = {:.0}%)",
      status,
      section,
      handled.len(),
      section_nodes.len(),
      pct
    );
  }
  println!();

  let mut items = Vec::new();

  // Only count nodes that need IR handlers
  let nodes_needing_handlers: HashSet<String> = catalog
    .nodes
    .iter()
    .filter(|n| n.node_type.needs_ir_handler())
    .map(|n| n.name.clone())
    .collect();

  // Check each AST node for a handler
  for node in &catalog.nodes {
    if !node.node_type.needs_ir_handler() {
      continue;
    }

    let has_handler = ir_handlers.contains(&node.name);
    let note = format!("{} ({})", node.section, node.node_type.description());
    items.push(ValidationItem {
      name: node.name.clone(),
      category: node.section.clone(),
      has_handler,
      is_orphan: false,
      notes: note,
    });
  }

  // Check for orphaned handlers
  let all_node_names: HashSet<String> = catalog.nodes.iter().map(|n| n.name.clone()).collect();
  for handler in &ir_handlers {
    if !all_node_names.contains(handler) {
      items.push(ValidationItem {
        name: handler.clone(),
        category: "Orphaned Handler".to_string(),
        has_handler: true,
        is_orphan: true,
        notes: "Handler exists but AST node type was removed".to_string(),
      });
    }
  }

  StageValidation {
    stage_name: "AST → IR".to_string(),
    items,
    source_constructs: nodes_needing_handlers,
    handled_constructs: ir_handlers,
  }
}

/// Extract AST catalog with node types and sections
fn extract_ast_catalog(definitions_path: &str) -> AstCatalog {
  let mut catalog = AstCatalog::default();

  let content = match fs::read_to_string(definitions_path) {
    Ok(c) => c,
    Err(e) => {
      eprintln!("{}: Could not read AST definitions: {}", "Error".red(), e);
      return catalog;
    }
  };

  // Extract enum variants dynamically from the source code
  let stmt_variants = extract_enum_variants(&content, "Stmt");
  let expr_variants = extract_enum_variants(&content, "Expr");

  let statement_types: HashSet<String> = stmt_variants.values().cloned().collect();
  let expression_types: HashSet<String> = expr_variants.values().cloned().collect();

  let mut current_section = "General".to_string();

  for line in content.lines() {
    let trimmed = line.trim();

    // Detect section headers: // ==================== SECTION NAME ====================
    if trimmed.starts_with("// ==") && trimmed.contains("==") {
      let section_text = trimmed
        .trim_start_matches("//")
        .trim()
        .trim_matches('=')
        .trim();
      if !section_text.is_empty() {
        current_section = section_text.to_string();
        if !catalog.sections.contains(&current_section) {
          catalog.sections.push(current_section.clone());
        }
      }
      continue;
    }

    // Match: pub struct SomeName or struct SomeName
    if trimmed.starts_with("pub struct ") || trimmed.starts_with("struct ") {
      let start = if trimmed.starts_with("pub struct ") {
        11
      } else {
        7
      };
      let rest = &trimmed[start..];
      let name: String = rest
        .chars()
        .take_while(|c| c.is_alphanumeric() || *c == '_')
        .collect();
      if !name.is_empty() {
        let node_type = if statement_types.contains(&name) {
          AstNodeType::Statement
        } else if expression_types.contains(&name) {
          AstNodeType::Expression
        } else {
          AstNodeType::Supporting
        };
        catalog.nodes.push(AstNode {
          name,
          node_type,
          section: current_section.clone(),
        });
      }
    }

    // Match: pub enum SomeName or enum SomeName
    if trimmed.starts_with("pub enum ") || trimmed.starts_with("enum ") {
      let start = if trimmed.starts_with("pub enum ") {
        9
      } else {
        5
      };
      let rest = &trimmed[start..];
      let name: String = rest
        .chars()
        .take_while(|c| c.is_alphanumeric() || *c == '_')
        .collect();
      if !name.is_empty() {
        catalog.nodes.push(AstNode {
          name,
          node_type: AstNodeType::Enum,
          section: current_section.clone(),
        });
      }
    }
  }

  // Add Expr variants that don't have separate struct definitions
  for variant_name in expr_variants.keys() {
    let struct_name = &expr_variants[variant_name];
    if !catalog.nodes.iter().any(|n| n.name == *struct_name) {
      catalog.nodes.push(AstNode {
        name: struct_name.clone(),
        node_type: AstNodeType::Expression,
        section: "AST TYPES".to_string(),
      });
    }
  }

  catalog
}

/// Extract enum variants from source code.
/// Returns a map of variant_name → struct_name.
/// Parses patterns like: `VariantName(StructName),` from `pub enum EnumName { ... }`
fn extract_enum_variants(source: &str, enum_name: &str) -> HashMap<String, String> {
  let mut variants = HashMap::new();

  // Find the enum block: `pub enum EnumName {` or `enum EnumName {`
  let enum_start = match source.find(&format!("pub enum {} {{", enum_name)) {
    Some(pos) => pos,
    None => match source.find(&format!("enum {} {{", enum_name)) {
      Some(pos) => pos,
      None => return variants,
    },
  };

  // Extract everything between the first `{` and the matching `}`
  let after_brace = &source[enum_start..];
  let brace_pos = after_brace.find('{').unwrap_or(0);
  let body = &after_brace[brace_pos + 1..];

  // Find matching closing brace by counting
  let mut depth = 1u32;
  let mut end_pos = 0usize;
  for (i, ch) in body.char_indices() {
    match ch {
      '{' => depth += 1,
      '}' => {
        depth -= 1;
        if depth == 0 {
          end_pos = i;
          break;
        }
      }
      _ => {}
    }
  }

  let enum_body = &body[..end_pos];

  // Parse variant lines like: `    VariantName(StructName),`
  for line in enum_body.lines() {
    let trimmed = line.trim();
    // Skip doc comments, attributes, and empty lines
    if trimmed.starts_with("///")
      || trimmed.starts_with("//")
      || trimmed.starts_with("#[")
      || trimmed.is_empty()
    {
      continue;
    }

    // Look for patterns like `VariantName(StructName),` or `VariantName(Box<StructName>),`
    if let Some(paren_idx) = trimmed.find('(') {
      let variant_name = trimmed[..paren_idx].to_string();
      let after_paren = &trimmed[paren_idx + 1..];

      // Extract the struct name, handling `Box<StructName>` wrappers
      let struct_name: String = if after_paren.starts_with("Box<") {
        after_paren
          .chars()
          .skip(4) // skip "Box<"
          .take_while(|c| c.is_alphanumeric() || *c == '_')
          .collect()
      } else {
        after_paren
          .chars()
          .take_while(|c| c.is_alphanumeric() || *c == '_')
          .collect()
      };

      if !struct_name.is_empty() {
        variants.insert(variant_name, struct_name);
      }
    } else {
      // Unit variant (no associated struct): VariantName, or VariantName
      // Strip trailing comma and any whitespace
      let variant_name = trimmed.trim_end_matches(',').trim().to_string();
      if !variant_name.is_empty() && !variant_name.starts_with("//") {
        // Unit variants map to themselves (they are their own "type" for catalog purposes)
        variants.insert(variant_name.clone(), variant_name);
      }
    }
  }

  variants
}

/// Extract references to AST types in IR generator
fn extract_ir_gen_handlers(definitions_path: &str, ir_gen_path: &str) -> HashSet<String> {
  let mut handlers = HashSet::new();

  // Build the variant → struct mapping from the AST definitions
  let defs_content = match fs::read_to_string(definitions_path) {
    Ok(c) => c,
    Err(_) => return handlers,
  };
  let stmt_map = extract_enum_variants(&defs_content, "Stmt");
  let expr_map = extract_enum_variants(&defs_content, "Expr");

  let content = match fs::read_to_string(ir_gen_path) {
    Ok(c) => c,
    Err(e) => {
      eprintln!("{}: Could not read IR generator: {}", "Error".red(), e);
      return handlers;
    }
  };

  // Find patterns like Stmt::SomeName, Expr::SomeName, etc.
  let patterns = [("Stmt::", &stmt_map), ("Expr::", &expr_map)];

  for line in content.lines() {
    for (pattern, name_map) in &patterns {
      let mut remaining = line;
      while let Some(pos) = remaining.find(pattern) {
        remaining = &remaining[pos + pattern.len()..];
        let name: String = remaining
          .chars()
          .take_while(|c| c.is_alphanumeric() || *c == '_')
          .collect();
        if !name.is_empty() {
          // Look up the variant name in the enum map to get the struct name
          let full_name = name_map.get(&name).cloned().unwrap_or(name);
          handlers.insert(full_name);
        }
      }
    }
  }

  handlers
}

// ============================================================================
// IR → Execution Validation
// ============================================================================

fn validate_interpreter_stage(args: &Args) -> StageValidation {
  println!();
  println!(
    "{}",
    "┌─────────────────────────────────────────────────────────────┐".magenta()
  );
  println!(
    "{}",
    "│          IR → Execution (Interpreter)              │"
      .magenta()
      .bold()
  );
  println!(
    "{}",
    "└─────────────────────────────────────────────────────────────┘".magenta()
  );

  let catalog = extract_ir_catalog(&format!("{}/ir/instructions.rs", args.src_dir));
  let interpreter_handlers =
    extract_interpreter_handlers(&format!("{}/interpreter.rs", args.src_dir));

  // Print IR summary
  println!();
  println!("  {}", "IR Instruction Summary:".bold());
  let total_instructions = catalog.instructions.len();

  // Count by category — derive unique categories from the catalog itself
  let mut categories: Vec<IrInstructionType> = catalog
    .instructions
    .iter()
    .map(|i| i.instr_type)
    .collect::<HashSet<_>>()
    .into_iter()
    .collect();
  categories.sort_by_key(|c| c.description().to_string());

  println!("    Total instructions: {}", total_instructions);
  println!();

  // Print categories
  println!("  {}", "Categories:".bold());
  for cat in &categories {
    let cat_instrs: Vec<_> = catalog
      .instructions
      .iter()
      .filter(|i| i.instr_type == *cat)
      .collect();
    if cat_instrs.is_empty() {
      continue;
    }
    let handled: Vec<_> = cat_instrs
      .iter()
      .filter(|i| interpreter_handlers.contains(&i.name))
      .collect();
    let pct = (handled.len() as f64 / cat_instrs.len() as f64) * 100.0;
    let status = if pct >= 100.0 {
      "✓".green()
    } else if pct >= 50.0 {
      "◐".yellow()
    } else {
      "✗".red()
    };
    println!(
      "    {} {} ({}/{} = {:.0}%)",
      status,
      cat.description(),
      handled.len(),
      cat_instrs.len(),
      pct
    );
  }
  println!();

  let mut items = Vec::new();
  let all_instruction_names: HashSet<String> = catalog
    .instructions
    .iter()
    .map(|i| i.name.clone())
    .collect();

  // Check each IR instruction for a handler
  for instr in &catalog.instructions {
    let has_handler = interpreter_handlers.contains(&instr.name);
    items.push(ValidationItem {
      name: instr.name.clone(),
      category: instr.instr_type.description().to_string(),
      has_handler,
      is_orphan: false,
      notes: String::new(),
    });
  }

  // Check for orphaned handlers
  for handler in &interpreter_handlers {
    if !all_instruction_names.contains(handler) {
      items.push(ValidationItem {
        name: handler.clone(),
        category: "Orphaned Handler".to_string(),
        has_handler: true,
        is_orphan: true,
        notes: "Handler exists but IR instruction was removed".to_string(),
      });
    }
  }

  StageValidation {
    stage_name: "IR → Execution".to_string(),
    items,
    source_constructs: all_instruction_names,
    handled_constructs: interpreter_handlers,
  }
}

/// Extract IR catalog with instruction types and categories.
/// Categories are parsed from `// ========== Category Name ==========` comments in the source.
fn extract_ir_catalog(instructions_path: &str) -> IrCatalog {
  let mut catalog = IrCatalog::default();

  let content = match fs::read_to_string(instructions_path) {
    Ok(c) => c,
    Err(e) => {
      eprintln!("{}: Could not read IR instructions: {}", "Error".red(), e);
      return catalog;
    }
  };

  // Map section header text to IrInstructionType
  fn category_from_comment(comment: &str) -> IrInstructionType {
    let comment = comment.trim();
    match comment {
      "Arithmetic Operations" => IrInstructionType::Arithmetic,
      "Bitwise Operations" => IrInstructionType::Bitwise,
      "Comparison Operations" => IrInstructionType::Comparison,
      "Logical Operations" => IrInstructionType::Logical,
      "Memory Operations" => IrInstructionType::Memory,
      "Control Flow" => IrInstructionType::ControlFlow,
      "Function Calls" => IrInstructionType::FunctionCall,
      "Type Conversions" => IrInstructionType::TypeConversion,
      "Constants" => IrInstructionType::Constant,
      "Global Access" => IrInstructionType::GlobalAccess,
      "Unit Assertions" => IrInstructionType::UnitAssertion,
      "String Operations" => IrInstructionType::StringOp,
      "Struct Operations" => IrInstructionType::Struct,
      "Optional Operations" => IrInstructionType::Optional,
      "Error Stack Operations" => IrInstructionType::ErrorStack,
      _ => {
        if comment.contains("Vector") || comment.contains("vec") {
          IrInstructionType::Vector
        } else if comment.contains("Bounds")
          || comment.contains("Zero")
          || comment.contains("Safety")
        {
          IrInstructionType::Safety
        } else {
          IrInstructionType::Uncategorized
        }
      }
    }
  }

  let mut in_instruction_enum = false;
  let mut brace_depth = 0;
  let mut current_category = IrInstructionType::Uncategorized;

  for line in content.lines() {
    let line = line.trim();

    // Detect start of Instruction enum
    if line.contains("enum Instruction") || line.contains("pub enum Instruction") {
      in_instruction_enum = true;
      if line.contains('{') {
        brace_depth = 1;
      }
      continue;
    }

    if in_instruction_enum {
      // Track brace depth
      brace_depth += line.matches('{').count() as i32;
      brace_depth -= line.matches('}').count() as i32;

      if brace_depth <= 0 {
        in_instruction_enum = false;
        continue;
      }

      // Detect category section comments: // ========== Name ==========
      if line.starts_with("//") && line.contains("===") {
        let category_text = line
          .trim_start_matches("//")
          .trim()
          .trim_matches('=')
          .trim();
        if !category_text.is_empty() {
          current_category = category_from_comment(category_text);
        }
        continue;
      }

      // Extract variant name (before { or ,)
      let variant_line = line.trim_start();
      if !variant_line.is_empty()
        && !variant_line.starts_with("//")
        && !variant_line.starts_with('#')
      {
        let name: String = variant_line
          .chars()
          .take_while(|c| c.is_alphanumeric() || *c == '_')
          .collect();
        if !name.is_empty()
          && name
            .chars()
            .next()
            .map(|c| c.is_uppercase())
            .unwrap_or(false)
        {
          catalog.instructions.push(IrInstruction {
            name,
            instr_type: current_category,
          });
        }
      }
    }
  }

  catalog
}

/// Extract Instruction:: references from interpreter
fn extract_interpreter_handlers(interpreter_path: &str) -> HashSet<String> {
  let mut handlers = HashSet::new();

  let content = match fs::read_to_string(interpreter_path) {
    Ok(c) => c,
    Err(e) => {
      eprintln!("{}: Could not read interpreter: {}", "Error".red(), e);
      return handlers;
    }
  };

  // Find all Instruction::SomeName patterns
  let pattern = "Instruction::";
  for line in content.lines() {
    let mut remaining = line;
    while let Some(pos) = remaining.find(pattern) {
      remaining = &remaining[pos + pattern.len()..];
      let name: String = remaining
        .chars()
        .take_while(|c| c.is_alphanumeric() || *c == '_')
        .collect();
      if !name.is_empty() {
        handlers.insert(name);
      }
    }
  }

  handlers
}

// ============================================================================
// Output Functions
// ============================================================================

fn output_stage_result(result: &StageValidation, args: &Args) {
  match args.output {
    OutputFormat::Text => output_text(result, args),
    OutputFormat::Json => output_json(result),
    OutputFormat::Markdown => output_markdown(result, args),
  }
}

fn output_text(result: &StageValidation, args: &Args) {
  println!();
  println!(
    "  {} Coverage: {:.1}%",
    result.stage_name,
    result.coverage_percent()
  );
  println!("  Source constructs: {}", result.source_constructs.len());
  println!("  Handled constructs: {}", result.handled_constructs.len());
  println!();

  // Group items by category
  let mut by_category: HashMap<String, Vec<&ValidationItem>> = HashMap::new();
  for item in &result.items {
    if args.missing_only && item.has_handler && !item.is_orphan {
      continue;
    }
    by_category
      .entry(item.category.clone())
      .or_default()
      .push(item);
  }

  // Per-category details (shown by default, hidden with --summary)
  if !args.summary {
    for (category, items) in &by_category {
      println!("  {}:", category.bold());
      for item in items {
        let status = if item.is_orphan {
          "⚠ ORPHAN".yellow()
        } else if item.has_handler {
          "✓".green()
        } else {
          "✗ MISSING".red()
        };
        println!("    {} {}", status, item.name);
        if !item.notes.is_empty() {
          println!("      {}", item.notes.dimmed());
        }
      }
      println!();
    }
  }

  // Summary
  let missing = result.missing_handlers();
  let orphans = result.orphaned_handlers();

  if !missing.is_empty() {
    println!("  {} Missing handlers: {}", "⚠".yellow(), missing.len());
  }
  if !orphans.is_empty() && !args.summary {
    println!("  {} Orphaned handlers: {}", "⚠".yellow(), orphans.len());
  }
}

fn output_json(result: &StageValidation) {
  let escape = |s: &str| -> String {
    s.replace('\\', "\\\\")
      .replace('"', "\\\"")
      .replace('\n', "\\n")
      .replace('\r', "\\r")
      .replace('\t', "\\t")
  };

  println!("{{");
  println!("  \"stage\": \"{}\",", result.stage_name);
  println!("  \"coverage_percent\": {:.1},", result.coverage_percent());
  println!("  \"source_count\": {},", result.source_constructs.len());
  println!("  \"handled_count\": {},", result.handled_constructs.len());
  println!("  \"missing_count\": {},", result.missing_handlers().len());
  println!("  \"orphan_count\": {},", result.orphaned_handlers().len());
  println!("  \"items\": [");

  let items: Vec<_> = result.items.iter().collect();
  for (i, item) in items.iter().enumerate() {
    let comma = if i < items.len() - 1 { "," } else { "" };
    println!(
      "    {{ \"name\": \"{}\", \"category\": \"{}\", \"has_handler\": {}, \"is_orphan\": {} }}{}",
      escape(&item.name),
      escape(&item.category),
      item.has_handler,
      item.is_orphan,
      comma
    );
  }

  println!("  ]");
  println!("}}");
}

fn output_markdown(result: &StageValidation, args: &Args) {
  println!("## {}", result.stage_name);
  println!();
  println!("**Coverage**: {:.1}%", result.coverage_percent());
  println!();
  println!("| Construct | Status | Notes |");
  println!("|-----------|--------|-------|");

  for item in &result.items {
    if args.missing_only && item.has_handler && !item.is_orphan {
      continue;
    }
    let status = if item.is_orphan {
      "⚠️ Orphan"
    } else if item.has_handler {
      "✅ Handled"
    } else {
      "❌ Missing"
    };
    println!("| `{}` | {} | {} |", item.name, status, item.notes);
  }
  println!();
}

// ============================================================================
// Silent Errors Scanner
// ============================================================================

fn validate_silent_errors_stage(args: &Args) {
  println!();
  println!(
    "{}",
    "┌─────────────────────────────────────────────────────────────┐".magenta()
  );
  println!(
    "{}",
    "│        Silent Error Patterns (Source Scan)         │"
      .magenta()
      .bold()
  );
  println!(
    "{}",
    "└─────────────────────────────────────────────────────────────┘".magenta()
  );
  println!();

  let src_dir = &args.src_dir;
  let mut findings: Vec<SilentFinding> = Vec::new();

  // Walk all .rs files in src_dir
  fn walk_dir(dir: &str, findings: &mut Vec<SilentFinding>) {
    if let Ok(entries) = fs::read_dir(dir) {
      for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
          walk_dir(&path.to_string_lossy(), findings);
        } else if path.extension().is_some_and(|e| e == "rs") {
          scan_file(&path, findings);
        }
      }
    }
  }

  walk_dir(src_dir, &mut findings);

  // Count by category
  let mut cats: HashMap<String, (usize, usize, usize)> = HashMap::new();
  for f in &findings {
    let entry = cats.entry(f.category.clone()).or_insert((0, 0, 0));
    match f.severity.as_str() {
      "HIGH" => entry.0 += 1,
      "MEDIUM" => entry.1 += 1,
      _ => entry.2 += 1,
    }
  }

  // Display summary table
  println!(
    "{:<50} {:>6} {:>6} {:>6} {:>6}",
    "Category", "HIGH", "MED", "LOW", "Total"
  );
  println!("{}", "─".repeat(78));
  let mut total = (0, 0, 0);
  for (cat, (h, m, l)) in &cats {
    println!("{:<50} {:>6} {:>6} {:>6} {:>6}", cat, h, m, l, h + m + l);
    total.0 += h;
    total.1 += m;
    total.2 += l;
  }
  println!("{}", "─".repeat(78));
  println!(
    "{:<50} {:>6} {:>6} {:>6} {:>6}",
    "TOTAL",
    total.0,
    total.1,
    total.2,
    total.0 + total.1 + total.2
  );
  println!();

  if total.0 > 0 {
    println!(
      "{} {:3} HIGH-severity silent fallbacks remaining",
      "🔴".red().bold(),
      total.0
    );
  }
  println!(
    "{} {:3} medium-severity patterns",
    "🟡".yellow().bold(),
    total.1
  );
  println!(
    "{} {:3} low-severity patterns",
    "🟢".green().bold(),
    total.2
  );

  if total.0 == 0 {
    println!();
    println!("{}", "✅ No HIGH-severity silent fallbacks!".green().bold());
  }

  // ── Grouped details by category (shown by default, hidden with --summary) ──
  if !args.summary {
    print_grouped_details(&findings, &cats);
  }
}

/// Print findings grouped by category with file:line references.
fn print_grouped_details(
  findings: &[SilentFinding],
  cats: &HashMap<String, (usize, usize, usize)>,
) {
  // Sort categories by name for stable output
  let mut cat_names: Vec<&String> = cats.keys().collect();
  cat_names.sort();

  for cat_name in &cat_names {
    let (h, m, l) = cats[*cat_name];
    let total = h + m + l;
    if total == 0 {
      continue;
    }

    println!();
    let mut sev_parts = Vec::new();
    if h > 0 {
      sev_parts.push(format!("🔴 {} High", h));
    }
    if m > 0 {
      sev_parts.push(format!("🟡 {} Medium", m));
    }
    if l > 0 {
      sev_parts.push(format!("🟢 {} Low", l));
    }
    let sev_str = sev_parts.join(" ");
    // Format: "3. Silent unwrap_or" → "Category 3: Silent unwrap_or"
    let label = if let Some((num, rest)) = cat_name.split_once(". ") {
      format!("Category {}: {}", num, rest)
    } else {
      format!("Category {}", cat_name)
    };
    println!("{} [{}]", label.bold(), sev_str);
    println!();

    // Collect findings for this category, sorted by file and line
    let mut cat_findings: Vec<&SilentFinding> = findings
      .iter()
      .filter(|f| &f.category == *cat_name)
      .collect();
    cat_findings.sort_by_key(|f| (&f.file, f.line));

    for f in &cat_findings {
      let sev_icon = match f.severity.as_str() {
        "HIGH" => "🔴",
        "MEDIUM" => "🟡",
        _ => "🟢",
      };
      // Truncate the file path to src/... for readability
      let short_file = if let Some(pos) = f.file.find("/src/") {
        &f.file[pos + 1..]
      } else if f.file.contains('/') {
        &f.file[f.file.rfind('/').map(|i| i + 1).unwrap_or(0)..]
      } else {
        &f.file
      };
      println!("  {} {}:{} — {}", sev_icon, short_file, f.line, f.pattern);
    }
  }
}

struct SilentFinding {
  file: String,
  line: usize,
  pattern: String,
  category: String,
  severity: String,
}

// ── AST-based silent error scanner ───────────────────────────────────────────

struct SilentErrorVisitor<'a> {
  findings: &'a mut Vec<SilentFinding>,
  file: String,
  source: String,
  in_test_context: bool,
}

impl<'a> SilentErrorVisitor<'a> {
  fn new(findings: &'a mut Vec<SilentFinding>, file: &str, source: &str) -> Self {
    Self {
      findings,
      file: file.to_string(),
      source: source.to_string(),
      in_test_context: false,
    }
  }

  fn add_finding(&mut self, line: usize, pattern: &str, category: &str, severity: &str) {
    self.findings.push(SilentFinding {
      file: self.file.clone(),
      line,
      pattern: pattern.to_string(),
      category: category.to_string(),
      severity: severity.to_string(),
    });
  }

  /// Check if the line before `span` contains an explanatory comment.
  fn has_explanatory_comment(&self, span: proc_macro2::Span) -> bool {
    let line = span.start().line;
    if line < 2 {
      return false;
    }
    let prev = self.source.lines().nth(line - 2).unwrap_or("");
    let t = prev.trim();
    t.starts_with("//")
      && (t.contains("intentional")
        || t.contains("reasonable")
        || t.contains("legitimate")
        || t.contains("fallback")
        || t.contains("acceptable")
        || t.contains("known")
        || t.contains("safe"))
  }

  /// Check if `eprintln!` at this span is followed by error propagation.
  fn has_error_propagation_after(&self, span: proc_macro2::Span) -> bool {
    let end = span.end().line;
    let lines: Vec<&str> = self.source.lines().collect();
    for i in end..(end + 6).min(lines.len()) {
      let l = lines.get(i).unwrap_or(&"").trim();
      if l.contains("process::exit(")
        || l.contains("return Err(")
        || l.contains("ice!(")
        || l.contains("panic!(")
      {
        return true;
      }
    }
    false
  }

  fn classify_unwrap_or(&self, call: &syn::ExprMethodCall) -> &'static str {
    if self.has_explanatory_comment(call.method.span()) {
      return "SKIP";
    }
    if self.file.contains("validate.rs") {
      return "SKIP";
    }
    let s = approx_source(call);
    // quote! adds spaces: "IrType :: I64" not "IrType::I64"
    if s.contains("IrType :: I64")
      || s.contains("IrType :: F64")
      || s.contains("IrType :: U64")
      || s.contains("IrType :: I32")
    {
      return "MEDIUM";
    }
    if s.contains("Value :: Int (0)")
      || s.contains("Value :: Float (0")
      || s.contains("Value :: Null")
    {
      return "MEDIUM";
    }
    if self.file.contains("expr_parser.rs")
      || self.file.contains("symbol_table.rs")
      || self.file.contains("printers")
    {
      return "LOW";
    }
    "LOW"
  }
}

impl<'ast, 'a> Visit<'ast> for SilentErrorVisitor<'a> {
  fn visit_item_mod(&mut self, node: &'ast syn::ItemMod) {
    let was = self.in_test_context;
    for attr in &node.attrs {
      if attr.path().is_ident("cfg") && quote_to_string(attr).contains("test") {
        self.in_test_context = true;
      }
    }
    syn::visit::visit_item_mod(self, node);
    self.in_test_context = was;
  }

  fn visit_item_fn(&mut self, node: &'ast syn::ItemFn) {
    let was = self.in_test_context;
    for attr in &node.attrs {
      if attr.path().is_ident("test") {
        self.in_test_context = true;
      }
    }
    syn::visit::visit_item_fn(self, node);
    self.in_test_context = was;
  }

  fn visit_local(&mut self, node: &'ast syn::Local) {
    if self.in_test_context || self.has_explanatory_comment(node.pat.span()) {
      return;
    }
    if matches!(node.pat, syn::Pat::Wild(_)) {
      self.add_finding(
        node.pat.span().start().line,
        "let _ = discard",
        "8. let _ = discard",
        "LOW",
      );
    }
    syn::visit::visit_local(self, node);
  }

  fn visit_expr_method_call(&mut self, node: &'ast syn::ExprMethodCall) {
    if self.in_test_context {
      return;
    }
    let line = node.method.span().start().line;
    match node.method.to_string().as_str() {
      "ok" => {
        if !self.has_explanatory_comment(node.method.span()) {
          self.add_finding(
            line,
            ".ok() discards error",
            "6. .ok() discards error",
            "MEDIUM",
          );
        }
      }
      "unwrap_or" | "unwrap_or_else" => {
        let sev = self.classify_unwrap_or(node);
        if sev != "SKIP" {
          self.add_finding(
            line,
            &format!("Silent unwrap_or — {}", approx_source(node)),
            "3. Silent unwrap_or",
            sev,
          );
        }
      }
      _ => {}
    }
    syn::visit::visit_expr_method_call(self, node);
  }

  fn visit_macro(&mut self, node: &'ast syn::Macro) {
    if self.in_test_context {
      return;
    }
    let name = node
      .path
      .segments
      .last()
      .map(|s| s.ident.to_string())
      .unwrap_or_default();
    if name == "eprintln" || name == "eprint" {
      let upper = node.tokens.to_string().to_uppercase();
      if (upper.contains("WARNING") || upper.contains("WARN"))
        && !self.has_error_propagation_after(node.span())
        && !self.has_explanatory_comment(node.span())
      {
        self.add_finding(
          node.path.span().start().line,
          "eprintln! WARNING",
          "9. eprintln! warning",
          "MEDIUM",
        );
      }
    }
    syn::visit::visit_macro(self, node);
  }

  fn visit_expr_match(&mut self, node: &'ast syn::ExprMatch) {
    if self.in_test_context {
      return;
    }
    for arm in &node.arms {
      if let syn::Pat::Wild(_) = &arm.pat {
        let body = approx_source(&arm.body);
        if body.contains("Ok(())") {
          self.add_finding(
            arm.pat.span().start().line,
            "Silent _ => Ok(())",
            "1. Catch-all Ok(())",
            "HIGH",
          );
        }
      }
    }
    syn::visit::visit_expr_match(self, node);
  }
}

fn approx_source(node: &impl quote::ToTokens) -> String {
  let s = quote::quote!(#node).to_string();
  if s.len() > 80 {
    format!("{}...", &s[..77])
  } else {
    s
  }
}

fn quote_to_string(attr: &syn::Attribute) -> String {
  quote::quote!(#attr).to_string()
}

fn scan_file(path: &std::path::Path, findings: &mut Vec<SilentFinding>) {
  let content = match fs::read_to_string(path) {
    Ok(c) => c,
    Err(_) => return,
  };
  let file = path.to_string_lossy().to_string();
  match syn::parse_file(&content) {
    Ok(ast) => {
      let mut visitor = SilentErrorVisitor::new(findings, &file, &content);
      visitor.visit_file(&ast);
    }
    Err(_) => {
      // Text fallback for unparseable files
      for (ln, line) in content.lines().enumerate() {
        let t = line.trim();
        if t == "_ => Ok(())," || t == "_ => Ok(())" {
          findings.push(SilentFinding {
            file: file.clone(),
            line: ln + 1,
            pattern: "Silent _ => Ok(())".into(),
            category: "1. Catch-all Ok(())".into(),
            severity: "HIGH".into(),
          });
        }
      }
    }
  }
}
