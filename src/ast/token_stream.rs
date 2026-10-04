//! Token stream extraction from the Pest parse tree.
//!
//! This module walks the Pest parse tree and produces a flat list of classified
//! tokens with exact source positions. Unlike the AST (which discards keyword
//! positions since Lale has no reserved words), this module preserves every
//! token's role as determined by the parser — keywords are keywords, identifiers
//! are identifiers, regardless of whether the same text could be either.
//!
//! The key insight: keywords in Lale's grammar are matched as literal strings
//! inside silent (`_{ }`) or compound (`{ }`) rules. They don't produce their
//! own Pest pairs, so we extract them from the **gaps** between child pairs
//! in compound rules.

use pest::Parser;
use pest::iterators::{Pair, Pairs};

use crate::{LaleParser, Rule};

/// What kind of token this is.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TokenKind {
  /// A language keyword: `var`, `fn`, `loop`, `if`, `else`, `returns`, `as`, etc.
  /// Determined by parser context, not text matching.
  Keyword,
  /// An identifier (variable name, function name, type name, etc.)
  Identifier,
  /// A numeric literal: integer, float, hex, unsigned
  Number,
  /// A string literal: `"hello"` or character literal: `'x'`
  String,
  /// A comment: `// ...` or `/// ...`
  Comment,
  /// An operator: `+`, `-`, `*`, `/`, `and`, `or`, `dot`, `cross`, etc.
  Operator,
  /// Punctuation: `(`, `)`, `[`, `]`, `{`, `}`, `,`, `.`, `:`, `;`
  Punctuation,
  /// A physical unit expression: `<kg⋅m/s²>`
  Unit,
  /// A boolean literal: `true`, `false`
  BoolLiteral,
  /// A compiler constant: `#compiler_version`, `#source_line`, etc.
  CompilerConst,
  /// The `nothing` keyword/expression
  Nothing,
}

/// A single classified token with exact source position.
#[derive(Debug, Clone)]
pub struct Token {
  pub kind: TokenKind,
  /// The text of this token.
  pub text: String,
  /// Byte range in the source.
  pub start_pos: usize,
  pub end_pos: usize,
}

// ---------------------------------------------------------------------------
// Main entry point
// ---------------------------------------------------------------------------

/// Parse source code and extract a flat, classified token list.
///
/// Returns tokens sorted by their byte position in the source.
/// Each token's classification comes from the parser's context.
pub fn tokenize(source: &str) -> Result<Vec<Token>, String> {
  let pairs =
    LaleParser::parse(Rule::program, source).map_err(|e| format!("Parse error: {}", e))?;
  let mut tokens = Vec::new();
  walk_pairs(pairs, source, &mut tokens);
  tokens.sort_by_key(|t| t.start_pos);
  Ok(tokens)
}

// ---------------------------------------------------------------------------
// Pair walking
// ---------------------------------------------------------------------------

fn walk_pairs(pairs: Pairs<Rule>, source: &str, out: &mut Vec<Token>) {
  for pair in pairs {
    walk_pair(pair, source, out);
  }
}

fn walk_pair(pair: Pair<Rule>, source: &str, out: &mut Vec<Token>) {
  let rule = pair.as_rule();
  let span = pair.as_span();
  let start = span.start();
  let end = span.end();

  // Collect children and their spans
  let children: Vec<(Rule, usize, usize)> = pair
    .clone()
    .into_inner()
    .map(|p| {
      let s = p.as_span();
      (p.as_rule(), s.start(), s.end())
    })
    .collect();

  match rule {
    // ---- Leaf rules: these ARE the tokens ----
    Rule::single_identifier | Rule::qualified_identifier => {
      add_token(out, TokenKind::Identifier, source, start, end);
    }
    Rule::int | Rule::u_int | Rule::float | Rule::hex | Rule::h_literal => {
      add_token(out, TokenKind::Number, source, start, end);
    }
    Rule::s_literal => {
      // String literal: add as whole, then recurse for embedding parts
      add_token(out, TokenKind::String, source, start, end);
      // Also recurse to capture tokens inside embedding expressions
      for child_pair in pair.into_inner() {
        if child_pair.as_rule() == Rule::embedded_value || child_pair.as_rule() == Rule::expression
        {
          walk_pair(child_pair, source, out);
        }
      }
    }
    Rule::c_literal => {
      add_token(out, TokenKind::String, source, start, end);
    }
    Rule::b_literal => {
      add_token(out, TokenKind::BoolLiteral, source, start, end);
    }
    Rule::n_literal => {
      // n_literal wraps a number with optional unit — extract the number part
      for child in pair.into_inner() {
        match child.as_rule() {
          Rule::int | Rule::u_int | Rule::float | Rule::hex => {
            add_token(
              out,
              TokenKind::Number,
              source,
              child.as_span().start(),
              child.as_span().end(),
            );
          }
          Rule::unit => {
            add_token(
              out,
              TokenKind::Unit,
              source,
              child.as_span().start(),
              child.as_span().end(),
            );
          }
          _ => walk_pair(child, source, out),
        }
      }
    }
    Rule::unit => {
      add_token(out, TokenKind::Unit, source, start, end);
    }
    Rule::comment | Rule::doc => {
      // These might be inside `info` wrappers; add as comment
      add_token(out, TokenKind::Comment, source, start, end);
    }
    Rule::content => {
      // comment/doc content — skip, the parent comment/doc covers it
    }
    Rule::comp_const => {
      add_token(out, TokenKind::CompilerConst, source, start, end);
    }
    Rule::nothing_expr => {
      add_token(out, TokenKind::Nothing, source, start, end);
    }
    Rule::identifier_continue => {
      // Part of an identifier — already covered by identifier/base_identifier
    }

    // ---- Top-level rules: just recurse (keywords are inside children) ----
    Rule::program | Rule::statement => {
      for child_pair in pair.into_inner() {
        walk_pair(child_pair, source, out);
      }
    }

    // ---- Compound rules: recurse into children AND extract keywords from gaps ----
    _ => {
      // Extract keywords/operators/punctuation from gaps between children
      extract_gap_tokens(rule, start, end, &children, source, out);

      // Recurse into all children
      for child_pair in pair.into_inner() {
        walk_pair(child_pair, source, out);
      }
    }
  }
}

// ---------------------------------------------------------------------------
// Gap token extraction
// ---------------------------------------------------------------------------

/// Extract tokens from the source text gaps between child pairs.
/// Gap 0: [parent_start, first_child_start)
/// Gap i: [child_i_end, child_i+1_start) for i in 0..children.len()-1
/// Final gap: [last_child_end, parent_end)
fn extract_gap_tokens(
  _parent_rule: Rule,
  parent_start: usize,
  parent_end: usize,
  children: &[(Rule, usize, usize)],
  source: &str,
  out: &mut Vec<Token>,
) {
  // No children: classify the entire parent span as gap text
  if children.is_empty() {
    let gap_start = snap_char_start(source, parent_start);
    let gap_end = snap_char_end(source, parent_end);
    if gap_start < gap_end {
      let gap_text = &source[gap_start..gap_end];
      classify_gap_text(gap_text, gap_start, source, out);
    }
    return;
  }
  // Gap before the first child
  if let Some(&(_, first_start, _)) = children.first() {
    let gap_start = snap_char_start(source, parent_start);
    let gap_end = snap_char_end(source, first_start);
    if gap_start < gap_end {
      let gap_text = &source[gap_start..gap_end];
      classify_gap_text(gap_text, gap_start, source, out);
    }
  }
  // Gaps between consecutive children
  for window in children.windows(2) {
    let (_, _, prev_end) = window[0];
    let (_, next_start, _) = window[1];
    let gap_start = snap_char_start(source, prev_end);
    let gap_end = snap_char_end(source, next_start);
    if gap_start < gap_end {
      let gap_text = &source[gap_start..gap_end];
      classify_gap_text(gap_text, gap_start, source, out);
    }
  }
  // Gap after the last child
  if let Some(&(_, _, last_end)) = children.last() {
    let gap_start = snap_char_start(source, last_end);
    let gap_end = snap_char_end(source, parent_end);
    if gap_start < gap_end {
      let gap_text = &source[gap_start..gap_end];
      classify_gap_text(gap_text, gap_start, source, out);
    }
  }
}

/// Classify text found in a gap between AST children.
///
/// Gaps contain keywords, operators, punctuation, and whitespace.
/// We scan for known patterns in order (longest match first).
fn classify_gap_text(gap: &str, gap_offset: usize, full_source: &str, out: &mut Vec<Token>) {
  let bytes = gap.as_bytes();
  let mut pos = 0usize;

  // All known keyword/operator/punctuation patterns to scan for.
  // Ordered: multi-word patterns first, then long single words, then short.
  // Patterns that contain whitespace must use `starts_with` on the remaining text.
  let patterns: &[(&str, TokenKind)] = &[
    // Multi-word keywords (longest first)
    ("fn signature", TokenKind::Keyword),
    ("end if", TokenKind::Keyword),
    ("end switch", TokenKind::Keyword),
    ("end match", TokenKind::Keyword),
    ("end when", TokenKind::Keyword),
    ("end loop", TokenKind::Keyword),
    ("end fn", TokenKind::Keyword),
    ("end type", TokenKind::Keyword),
    ("end enum", TokenKind::Keyword),
    ("end test suite", TokenKind::Keyword),
    ("end test case", TokenKind::Keyword),
    ("exit loop", TokenKind::Keyword),
    ("exit program", TokenKind::Keyword),
    ("unsafe bitcast", TokenKind::Keyword),
    ("test suite", TokenKind::Keyword),
    ("test case", TokenKind::Keyword),
    // Multi-word operators
    ("bitwise and", TokenKind::Operator),
    ("bitwise or", TokenKind::Operator),
    ("bitwise xor", TokenKind::Operator),
    ("unsigned left shift", TokenKind::Operator),
    ("unsigned right shift", TokenKind::Operator),
    ("signed left shift", TokenKind::Operator),
    ("signed right shift", TokenKind::Operator),
    ("pointer to", TokenKind::Operator),
    ("value of", TokenKind::Operator),
    ("value at", TokenKind::Operator),
    ("has value", TokenKind::Operator),
    ("has no value", TokenKind::Operator),
    ("#type of", TokenKind::Operator),
    ("#unit of", TokenKind::Operator),
    ("#size of", TokenKind::Operator),
    // Multi-word I/O
    ("write inline", TokenKind::Keyword),
    ("add error", TokenKind::Keyword),
    ("alert error messages", TokenKind::Keyword),
    ("errors has messages", TokenKind::Keyword),
    ("last error", TokenKind::Keyword),
    ("move on", TokenKind::Keyword),
    ("missing code", TokenKind::Keyword),
    // Compiler directives
    ("#else if", TokenKind::Keyword),
    ("#end if", TokenKind::Keyword),
    // Single-word keywords
    ("var", TokenKind::Keyword),
    ("fn", TokenKind::Keyword),
    ("type", TokenKind::Keyword),
    ("use", TokenKind::Keyword),
    ("unsafe", TokenKind::Keyword),
    ("if", TokenKind::Keyword),
    ("else", TokenKind::Keyword),
    ("loop", TokenKind::Keyword),
    ("return", TokenKind::Keyword),
    ("case", TokenKind::Keyword),
    ("switch", TokenKind::Keyword),
    ("match", TokenKind::Keyword),
    ("when", TokenKind::Keyword),
    ("default", TokenKind::Keyword),
    ("debug", TokenKind::Keyword),
    ("rewind", TokenKind::Keyword),
    ("export", TokenKind::Keyword),
    ("import", TokenKind::Keyword),
    ("copy", TokenKind::Keyword),
    ("private", TokenKind::Keyword),
    ("enum", TokenKind::Keyword),
    ("assert", TokenKind::Keyword),
    ("nothing", TokenKind::Nothing),
    ("returns", TokenKind::Keyword),
    ("as", TokenKind::Keyword),
    ("from", TokenKind::Keyword),
    ("to", TokenKind::Keyword),
    ("step", TokenKind::Keyword),
    ("in", TokenKind::Keyword),
    ("of", TokenKind::Keyword),
    ("end", TokenKind::Keyword),
    ("write", TokenKind::Keyword),
    ("warn", TokenKind::Keyword),
    ("alert", TokenKind::Keyword),
    ("read", TokenKind::Keyword),
    ("true", TokenKind::BoolLiteral),
    ("false", TokenKind::BoolLiteral),
    ("decl", TokenKind::Keyword),
    ("fill", TokenKind::Keyword),
    ("with", TokenKind::Keyword),
    // Operators (single-word)
    ("and", TokenKind::Operator),
    ("or", TokenKind::Operator),
    ("xor", TokenKind::Operator),
    ("not", TokenKind::Operator),
    ("invert", TokenKind::Operator),
    ("dot", TokenKind::Operator),
    ("cross", TokenKind::Operator),
    // Two-char operators
    ("==", TokenKind::Operator),
    ("!=", TokenKind::Operator),
    ("≠", TokenKind::Operator),
    (">=", TokenKind::Operator),
    ("<=", TokenKind::Operator),
    ("≥", TokenKind::Operator),
    ("≤", TokenKind::Operator),
    ("+=", TokenKind::Operator),
    ("-=", TokenKind::Operator),
    ("*=", TokenKind::Operator),
    ("/=", TokenKind::Operator),
    ("%=", TokenKind::Operator),
    ("//", TokenKind::Comment),
    ("///", TokenKind::Comment),
    ("÷", TokenKind::Operator),
    ("⨯", TokenKind::Operator),
    ("⋅", TokenKind::Operator),
    ("⊻", TokenKind::Operator),
    // Single-char operators and punctuation
    ("+", TokenKind::Operator),
    ("-", TokenKind::Operator),
    ("*", TokenKind::Operator),
    ("/", TokenKind::Operator),
    ("%", TokenKind::Operator),
    ("^", TokenKind::Operator),
    ("~", TokenKind::Operator),
    ("=", TokenKind::Operator),
    (">", TokenKind::Operator),
    ("<", TokenKind::Operator),
    ("!", TokenKind::Operator),
    ("(", TokenKind::Punctuation),
    (")", TokenKind::Punctuation),
    ("[", TokenKind::Punctuation),
    ("]", TokenKind::Punctuation),
    ("{", TokenKind::Punctuation),
    ("}", TokenKind::Punctuation),
    (",", TokenKind::Punctuation),
    (".", TokenKind::Punctuation),
    (":", TokenKind::Punctuation),
    (";", TokenKind::Punctuation),
    ("?", TokenKind::Operator),
    ("#", TokenKind::Operator),
  ];

  while pos < bytes.len() {
    // Skip whitespace and newlines (ASCII, so byte iteration is safe)
    if bytes[pos].is_ascii_whitespace() {
      pos += 1;
      continue;
    }

    // Ensure we're at a char boundary before slicing
    pos = snap_char_start(gap, pos);
    if pos >= bytes.len() {
      break;
    }

    let remaining = &gap[pos..];
    let mut matched = false;

    for &(pat, ref kind) in patterns {
      if remaining.starts_with(pat) {
        let pat_end = pos + pat.len();
        // Check boundary: pat must be followed by non-identifier char or EOF
        let after = bytes.get(pat_end).copied().unwrap_or(b' ');
        let is_boundary = !after.is_ascii_alphanumeric() && after != b'_';
        // Special case: punctuation and single-char operators don't need boundary checks
        let is_single_char_punct =
          pat.len() == 1 && matches!(kind, TokenKind::Punctuation | TokenKind::Operator);

        if is_boundary || is_single_char_punct {
          let abs_start = gap_offset + pos;
          let abs_end = gap_offset + pat_end;
          if abs_start < full_source.len() && abs_end <= full_source.len() {
            add_token(out, kind.clone(), full_source, abs_start, abs_end);
          }
          pos = pat_end;
          matched = true;
          break;
        }
      }
    }

    if !matched {
      // Advance by one char (UTF-8 safety)
      let next = gap[pos..].chars().next().map(|c| c.len_utf8()).unwrap_or(1);
      pos += next;
    }
  }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn add_token(out: &mut Vec<Token>, kind: TokenKind, source: &str, start: usize, end: usize) {
  let start = start.min(source.len());
  let end = end.min(source.len());
  if start >= end {
    return;
  }
  // Snap to char boundaries
  let start = snap_char_start(source, start);
  let end = snap_char_end(source, end);
  if start >= end {
    return;
  }
  let text = source[start..end].to_string();
  out.push(Token {
    kind,
    text,
    start_pos: start,
    end_pos: end,
  });
}

fn snap_char_start(source: &str, pos: usize) -> usize {
  let mut p = pos.min(source.len());
  while p > 0 && !source.is_char_boundary(p) {
    p -= 1;
  }
  p
}

fn snap_char_end(source: &str, pos: usize) -> usize {
  let mut p = pos.min(source.len());
  while p < source.len() && !source.is_char_boundary(p) {
    p += 1;
  }
  p
}

#[cfg(test)]
mod tests {
  use crate::ast::TokenKind;
  use crate::ast::token_stream::tokenize;

  #[test]
  fn test_simple_def() {
    let tokens = tokenize("var x as u32 = 5").unwrap();
    let kinds: Vec<String> = tokens.iter().map(|t| format!("{:?}", t.kind)).collect();
    assert!(
      kinds.contains(&"Keyword".to_string()),
      "Expected Keyword tokens, got: {:?}",
      kinds
    );
    assert!(
      kinds.contains(&"Identifier".to_string()),
      "Expected Identifier tokens"
    );
    assert!(
      kinds.contains(&"Number".to_string()),
      "Expected Number tokens"
    );
  }

  #[test]
  fn test_loop_var() {
    let tokens = tokenize("loop var i as i32 from 1 to 10\n  write \"hello\"\nend loop").unwrap();
    let var_tokens: Vec<_> = tokens.iter().filter(|t| t.text == "var").collect();
    assert!(!var_tokens.is_empty(), "Expected 'var' token to exist");
    for t in &var_tokens {
      assert_eq!(
        t.kind,
        TokenKind::Keyword,
        "Expected 'var' to be Keyword, got {:?} at pos {}",
        t.kind,
        t.start_pos
      );
    }
  }

  #[test]
  fn test_test_suite_and_case_keywords() {
    let tokens = tokenize(
      "test suite maths\n  test case addition\n    assert true\n  end test case\nend test suite",
    )
    .unwrap();
    let suite_tokens: Vec<_> = tokens.iter().filter(|t| t.text == "test suite").collect();
    let case_tokens: Vec<_> = tokens.iter().filter(|t| t.text == "test case").collect();
    assert!(
      !suite_tokens.is_empty(),
      "Expected 'test suite' keyword token"
    );
    for t in &suite_tokens {
      assert_eq!(
        t.kind,
        TokenKind::Keyword,
        "Expected 'test suite' to be Keyword, got {:?}",
        t.kind
      );
    }
    assert!(
      !case_tokens.is_empty(),
      "Expected 'test case' keyword token"
    );
    for t in &case_tokens {
      assert_eq!(
        t.kind,
        TokenKind::Keyword,
        "Expected 'test case' to be Keyword, got {:?}",
        t.kind
      );
    }
  }

  #[test]
  fn test_string_literal() {
    let tokens = tokenize("write \"hello world\"").unwrap();
    let strings: Vec<_> = tokens
      .iter()
      .filter(|t| t.kind == TokenKind::String)
      .collect();
    assert!(!strings.is_empty(), "Expected string token");
  }

  #[test]
  fn test_comment() {
    let tokens = tokenize("// this is a comment\nvar x as u32 = 5").unwrap();
    let comments: Vec<_> = tokens
      .iter()
      .filter(|t| t.kind == TokenKind::Comment)
      .collect();
    assert!(!comments.is_empty(), "Expected comment token");
  }

  #[test]
  fn test_embedding() {
    let tokens = tokenize("write \"value = {x * 2}\"").unwrap();
    let kinds: Vec<_> = tokens.iter().map(|t| format!("{:?}", t.kind)).collect();
    assert!(
      kinds.contains(&"String".to_string()),
      "Expected String token, got: {:?}",
      kinds
    );
    assert!(
      kinds.contains(&"Identifier".to_string()),
      "Expected Identifier token"
    );
    assert!(
      kinds.contains(&"Operator".to_string()),
      "Expected Operator token"
    );
    assert!(
      kinds.contains(&"Number".to_string()),
      "Expected Number token"
    );
  }

  #[test]
  fn test_returns_keyword() {
    let tokens = tokenize("fn foo() returns text\n  return \"hi\"\nend fn").unwrap();
    let returns_tokens: Vec<_> = tokens.iter().filter(|t| t.text == "returns").collect();
    assert!(!returns_tokens.is_empty(), "Expected 'returns' token");
    for t in &returns_tokens {
      assert_eq!(
        t.kind,
        TokenKind::Keyword,
        "Expected 'returns' to be Keyword, got {:?}",
        t.kind
      );
    }
  }

  #[test]
  fn test_over_as_identifier() {
    let tokens = tokenize("var over as u32 = 5").unwrap();
    let ident_tokens: Vec<_> = tokens
      .iter()
      .filter(|t| t.text == "over" && t.kind == TokenKind::Identifier)
      .collect();
    assert!(
      !ident_tokens.is_empty(),
      "Expected 'over' Identifier in var over, got tokens: {:?}",
      tokens
        .iter()
        .map(|t| format!("{}:{:?}", t.text, t.kind))
        .collect::<Vec<_>>()
    );
  }

  #[test]
  fn test_full_source() {
    let source = "// use std\nuse openFile, readFile from std.file_io_posix\n\nwarn \"~~~ BEGIN ~~~\"\n\nloop var i as i32 from 1 to 2\n  write \"döngü i = {i * 2}\"\nend loop";
    let tokens = tokenize(source).unwrap();
    println!("Got {} tokens", tokens.len());
    for t in &tokens {
      println!(
        "  {:?} at {}..{}: '{}'",
        t.kind, t.start_pos, t.end_pos, t.text
      );
    }
    assert!(
      tokens.len() > 10,
      "Expected many tokens, got {}",
      tokens.len()
    );
  }
}
