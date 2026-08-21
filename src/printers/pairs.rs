//! Raw Parse Tree Printer
//!
//! This module prints the raw pest parse tree directly from `Pairs<Rule>`,
//! before AST conversion. Useful for debugging the grammar.
//!
//! For printing the converted AST, use `ast::print_ast()` instead.

use crate::Rule;
use colored::*;
use pest::iterators::Pairs;

/// Display raw parse tree from pest Pairs recursively, with optional colors.
/// Useful for debugging grammar rules before AST conversion.
pub fn print_pairs(pairs: Pairs<Rule>, file_name: &str, level: usize, use_colors: bool) {
  let mut indent: String = "".to_string();
  if level > 1 {
    indent = "│ ".repeat(level - 1);
  }
  let prefix = if level > 0 { "├─ " } else { "" };

  for pair in pairs {
    let rule = pair.as_rule();
    let start = pair.as_span().start_pos().pos();
    let end = pair.as_span().end_pos().pos();
    let line = pair.line_col().0;
    let col = pair.line_col().1;
    let text = pair.as_span().as_str();
    let inner = pair.into_inner();

    let rule_str = format!("{:?}", rule);
    let rule_text = if use_colors {
      if rule_str.starts_with("ct_") {
        rule_str.red()
      } else if rule_str == "identifier" {
        rule_str.yellow()
      } else if rule_str == "comment" || rule_str == "doc" || rule_str == "content" {
        rule_str.dimmed()
      } else {
        rule_str.green()
      }
    } else {
      rule_str.normal()
    };

    let content_text = if use_colors {
      if rule_str == "comment" || rule_str == "doc" || rule_str == "content" {
        format!("\"{}\"", text).dimmed()
      } else if rule_str == "identifier" {
        format!("\"{}\"", text).yellow()
      } else {
        format!("\"{}\"", text).green()
      }
    } else {
      format!("\"{}\"", text).normal()
    };

    if inner.peek().is_none() {
      println!(
        "{}{}{}: {} [{} - {}] {}:{}:{}",
        indent, prefix, rule_text, content_text, start, end, file_name, line, col
      );
    } else {
      print!(
        "{}{}{} [{} - {}] {}:{}:{}",
        indent, prefix, rule_text, start, end, file_name, line, col
      );
      println!();
      print_pairs(inner, file_name, level + 1, use_colors);
    }
  }
}
