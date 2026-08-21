//! AST Builder - Converts pest parse tree to Lale AST.
//!
//! This module transforms pest's `Pairs<Rule>` into the semantic AST types
//! defined in `definitions`. Statement-level parsing is handled here, while
//! expression parsing (with Pratt precedence climbing) is delegated to `expr_parser`.
//!
//! # Usage
//!
//! ```rust
//! # use lale::{LaleParser, Rule};
//! # use pest::Parser;
//! # use lale::ast::builder::build_program;
//! let source_code = "var x as i32 = 5";
//! let pairs = LaleParser::parse(Rule::program, source_code).unwrap();
//! let program = build_program(pairs, "<test>").unwrap();
//! ```

use std::cell::RefCell;

use pest::iterators::{Pair, Pairs};

use super::definitions::*;
use super::expr_parser::{self, build_fn_call};
use super::{SourceLocation, Spanned};
use crate::Rule;

// Thread-local current source file name for setting in AST locations
thread_local! {
  static CURRENT_SOURCE_FILE: RefCell<String> = RefCell::new("<unknown>".to_string());
}

/// Macro for reporting unexpected grammar rules with Lale-style compiler errors.
/// Two-arg form: direct match arm → `return Err(msg)`
/// Three-arg form: nested match arm with interstitial check → `return Err(msg)`
macro_rules! unexpected {
  // Category 1: direct match arm in a function returning Result
  ($pair:expr, $context:expr) => {{
    let loc = location_from_pair(&$pair);
    let msg = $crate::internal_error!(
      "unexpected grammar rule '{:?}' in {}\n  at {}:{}:{}",
      $pair.as_rule(),
      $context,
      loc.source_file,
      loc.line,
      loc.col
    );
    Err(msg)
  }};
  // Category 2: nested match arm — skips interstitials, crashes on new content
  ($inner:expr, $context:expr, $loc:expr) => {
    match $inner.as_rule() {
      crate::Rule::info => {}
      _ => {
        let msg = $crate::internal_error!(
          "unexpected child rule '{:?}' in {}\n  at {}:{}:{}",
          $inner.as_rule(),
          $context,
          $loc.source_file,
          $loc.line,
          $loc.col
        );
        return Err(msg);
      }
    }
  };
}

/// Create a SourceLocation from a pest pair with the current source file.
/// Used by both builder and expr_parser to ensure consistent source file tracking.
pub fn location_from_pair(pair: &Pair<Rule>) -> SourceLocation {
  let (line, col) = pair.line_col();
  let mut loc = SourceLocation {
    line,
    col,
    start_pos: pair.as_span().start_pos().pos(),
    end_pos: pair.as_span().end_pos().pos(),
    source_file: "<unknown>".to_string(),
  };
  CURRENT_SOURCE_FILE.with(|f| {
    loc.source_file = f.borrow().clone();
  });
  loc
}

/// Helper to extract attached comments from a pair's inner children.
/// Returns (comments, remaining_pairs, content_location).
/// The content_location is the location of the first non-comment element,
/// which represents where the actual statement content starts.
fn extract_comments(
  pair: Pair<Rule>,
) -> (AttachedComments, Vec<Pair<Rule>>, Option<SourceLocation>) {
  let mut leading = Vec::new();
  let mut trailing = None;
  let mut remaining = Vec::new();
  let mut in_leading = true;
  let mut content_location = None;

  for inner in pair.into_inner() {
    match inner.as_rule() {
      Rule::info => {
        // Extract doc or comment from inside the info node
        for info_child in inner.clone().into_inner() {
          match info_child.as_rule() {
            Rule::doc => {
              let content_pair = info_child
                .clone()
                .into_inner()
                .find(|p| p.as_rule() == Rule::content);
              let content_text = content_pair
                .map(|p| p.as_str().to_string())
                .unwrap_or_default();
              if in_leading {
                leading.push(AttachedComment::Doc(DocStmt {
                  content: content_text,
                  location: location_from_pair(&info_child),
                }));
              } else {
                trailing = Some(AttachedComment::Doc(DocStmt {
                  content: content_text,
                  location: location_from_pair(&info_child),
                }));
              }
            }
            Rule::comment => {
              let content_pair = info_child
                .clone()
                .into_inner()
                .find(|p| p.as_rule() == Rule::content);
              let content_text = content_pair
                .map(|p| p.as_str().to_string())
                .unwrap_or_default();
              if in_leading {
                leading.push(AttachedComment::Comment(CommentStmt {
                  content: content_text,
                  location: location_from_pair(&info_child),
                }));
              } else {
                trailing = Some(AttachedComment::Comment(CommentStmt {
                  content: content_text,
                  location: location_from_pair(&info_child),
                }));
              }
            }
            _ => ice!(
              "extract_comments: unexpected child rule '{:?}' of 'info' (grammar change without builder update?)",
              inner.as_rule()
            ),
          }
        }
      }
      Rule::doc if in_leading => {
        let content_pair = inner
          .clone()
          .into_inner()
          .find(|p| p.as_rule() == Rule::content);
        let content_text = content_pair
          .map(|p| p.as_str().to_string())
          .unwrap_or_default();
        leading.push(AttachedComment::Doc(DocStmt {
          content: content_text,
          location: location_from_pair(&inner),
        }));
      }
      Rule::comment if in_leading => {
        let content_pair = inner
          .clone()
          .into_inner()
          .find(|p| p.as_rule() == Rule::content);
        let content_text = content_pair
          .map(|p| p.as_str().to_string())
          .unwrap_or_default();
        leading.push(AttachedComment::Comment(CommentStmt {
          content: content_text,
          location: location_from_pair(&inner),
        }));
      }
      Rule::doc => {
        let content_pair = inner
          .clone()
          .into_inner()
          .find(|p| p.as_rule() == Rule::content);
        let content_text = content_pair
          .map(|p| p.as_str().to_string())
          .unwrap_or_default();
        trailing = Some(AttachedComment::Doc(DocStmt {
          content: content_text,
          location: location_from_pair(&inner),
        }));
      }
      Rule::comment => {
        let content_pair = inner
          .clone()
          .into_inner()
          .find(|p| p.as_rule() == Rule::content);
        let content_text = content_pair
          .map(|p| p.as_str().to_string())
          .unwrap_or_default();
        trailing = Some(AttachedComment::Comment(CommentStmt {
          content: content_text,
          location: location_from_pair(&inner),
        }));
      }
      _ => {
        if in_leading {
          content_location = Some(location_from_pair(&inner));
        }
        in_leading = false;
        remaining.push(inner);
      }
    }
  }

  (
    AttachedComments { leading, trailing },
    remaining,
    content_location,
  )
}

/// Build the program AST from a pest parse tree.
///
/// This is the main entry point for AST construction. It transforms the raw
/// `Pairs<Rule>` produced by pest into a [`Program`] — the root node of Lale's
/// Abstract Syntax Tree.
///
/// # The AST Structure
///
/// The AST is not a single struct but a tree of typed nodes:
///
/// ```text
/// Program (root)              ← this function builds and returns this
///   └── Vec<Statement>
///         ├── Statement::Let { ... }
///         │     └── Expr::Binary { ... }
///         │           ├── Expr::Identifier
///         │           └── Expr::IntLiteral
///         ├── Statement::Function { ... }
///         └── ...
/// ```
///
/// The [`Program`] struct *is* the AST — specifically, it's the root that owns
/// the entire tree. All other AST types (`Statement`, `Expr`, `Type`, etc.)
/// are defined in the [`crate::ast`] module.
/// ```rust
/// # use lale::{LaleParser, Rule};
/// # use pest::Parser;
/// # use lale::ast::builder::build_program;
/// let source_code = "var x as i32 = 5";
/// let pairs = LaleParser::parse(Rule::program, source_code).unwrap();
/// let program = build_program(pairs, "<test>").unwrap();
///
/// for statement in &program.statements {
///     // traverse the AST...
/// }
/// ```
pub fn build_program(pairs: Pairs<Rule>, source_file: &str) -> Result<Program, String> {
  // Set the thread-local source file name
  CURRENT_SOURCE_FILE.with(|f| {
    *f.borrow_mut() = source_file.to_string();
  });

  let mut statements = Vec::new();
  let mut location = None;

  for pair in pairs {
    match pair.as_rule() {
      Rule::program => {
        let mut loc = location_from_pair(&pair);
        loc.source_file = source_file.to_string();
        location = Some(loc);
        for inner in pair.into_inner() {
          for stmt in build_statement(inner)? {
            statements.push(stmt);
          }
        }
      }
      Rule::EOI => {}
      _ => {
        return Err(format!(
          "Unexpected rule at top level: {:?}",
          pair.as_rule()
        ));
      }
    }
  }

  Ok(Program {
    statements,
    location: location.unwrap_or(SourceLocation {
      line: 1,
      col: 1,
      start_pos: 0,
      end_pos: 0,
      source_file: "<unknown>".to_string(),
    }),
    global_symbol_table: RefCell::new(None),
  })
}

/// Convert a parsed statement pair into an AST statement node.
fn build_statement(pair: Pair<Rule>) -> Result<Vec<Stmt>, String> {
  let rule = pair.as_rule();
  match rule {
    Rule::use_stmt => Ok(vec![Stmt::Use(build_use_stmt(pair)?)]),
    Rule::type_def => Ok(vec![Stmt::TypeDef(build_type_def(pair)?)]),
    Rule::enum_def => Ok(vec![Stmt::EnumDef(build_enum_def(pair)?)]),
    Rule::var => Ok(vec![Stmt::VarDef(build_var_def(pair)?)]),
    Rule::unsafe_decl => Ok(vec![Stmt::UnsafeDecl(build_unsafe_decl(pair)?)]),
    Rule::assign => Ok(vec![Stmt::Assign(build_assign(pair)?)]),
    Rule::compound_assign => Ok(vec![Stmt::CompoundAssign(build_compound_assign(pair)?)]),
    Rule::value_at_assign => Ok(vec![Stmt::ValueAtAssign(build_value_at_assign(pair)?)]),
    Rule::fn_def => Ok(vec![Stmt::FnDef(build_fn_def(pair)?)]),
    Rule::fn_signature => Ok(vec![Stmt::FnSignature(build_fn_signature(pair)?)]),
    Rule::fn_call => Ok(vec![Stmt::FnCall(build_fn_call(pair)?)]),
    Rule::if_stmt => Ok(vec![Stmt::If(build_if(pair)?)]),
    Rule::when_stmt => Ok(vec![Stmt::When(build_when(pair)?)]),
    Rule::match_stmt => Ok(vec![Stmt::Match(build_match(pair)?)]),
    Rule::switch_stmt => Ok(vec![Stmt::Switch(build_switch(pair)?)]),
    Rule::loop_stmt => {
      let loop_stmt = build_loop(pair)?;
      let mut stmts: Vec<Stmt> = Vec::new();
      // If loop has a range, prepend a synthetic VarDefStmt for the loop variable.
      // This lets visit_var_def / try_generate_var_def handle allocation and
      // initialization, eliminating duplicated code in the analyzer and IR generator.
      if let Some(range) = &loop_stmt.range {
        let var_def = VarDefStmt {
          is_export: false,
          is_import: false,
          is_loop_var: true,
          name: Spanned::new(range.variable.node.clone(), range.variable.span.clone()),
          type_annotation: Some(range.var_type.clone()),
          unit: None,
          value: range.from.clone(),
          location: range.variable.span.clone(),
          comments: AttachedComments::default(),
        };
        stmts.push(Stmt::VarDef(var_def));
      }
      stmts.push(Stmt::Loop(Box::new(loop_stmt)));
      Ok(stmts)
    }
    Rule::return_stmt => Ok(vec![Stmt::Return(build_return(pair)?)]),
    Rule::exit_program => Ok(vec![Stmt::ExitProgram(build_exit_program(pair)?)]),
    Rule::exit_loop => Ok(vec![Stmt::ExitLoop(build_exit_loop(pair)?)]),
    Rule::rewind => Ok(vec![Stmt::Rewind(build_rewind(pair)?)]),
    Rule::stdout => Ok(vec![Stmt::Stdout(build_stdout(pair)?)]),
    Rule::stderr => Ok(vec![Stmt::Stderr(build_stderr(pair)?)]),
    Rule::stdin => {
      let stdin = build_stdin(pair)?;
      let var_name = stdin.target.node.join(".");
      let var_def = VarDefStmt {
        is_export: false,
        is_import: false,
        is_loop_var: false,
        name: Spanned::new(var_name.clone(), stdin.target.span.clone()),
        type_annotation: Some(TypeName {
          base_type: BaseType::Str,
          inner_type: None,
          array_dimensions: vec![],
          is_optional: false,
          location: stdin.location.clone(),
        }),
        unit: None,
        value: Expr::StringLiteral(StringLiteral {
          parts: vec![StringPart::Text(Spanned::new(
            "".to_string(),
            stdin.location.clone(),
          ))],
          location: stdin.location.clone(),
        }),
        location: stdin.location.clone(),
        comments: AttachedComments::default(),
      };
      Ok(vec![Stmt::VarDef(var_def), Stmt::Stdin(stdin)])
    }
    Rule::debug_output => Ok(vec![Stmt::Debug(build_debug(pair)?)]),
    Rule::ct_if => Ok(vec![Stmt::CtIf(build_ct_if(pair)?)]),
    Rule::ct_fail => Ok(vec![Stmt::CtFail(build_ct_fail(pair)?)]),
    Rule::ct_warn => Ok(vec![Stmt::CtWarn(build_ct_warn(pair)?)]),
    Rule::ct_when => Ok(vec![Stmt::CtWhen(build_ct_when(pair)?)]),
    Rule::ct_match => Ok(vec![Stmt::CtMatch(build_ct_match(pair)?)]),
    Rule::ct_switch => Ok(vec![Stmt::CtSwitch(build_ct_switch(pair)?)]),
    Rule::info => {
      // Standalone info node (contains doc or comment)
      for info_child in pair.clone().into_inner() {
        match info_child.as_rule() {
          Rule::doc => {
            let content_pair = info_child
              .clone()
              .into_inner()
              .find(|p| p.as_rule() == Rule::content);
            let content_text = content_pair
              .map(|p| p.as_str().to_string())
              .unwrap_or_default();
            return Ok(vec![Stmt::Doc(DocStmt {
              content: content_text,
              location: location_from_pair(&info_child),
            })]);
          }
          Rule::comment => {
            let content_pair = info_child
              .clone()
              .into_inner()
              .find(|p| p.as_rule() == Rule::content);
            let content_text = content_pair
              .map(|p| p.as_str().to_string())
              .unwrap_or_default();
            return Ok(vec![Stmt::Comment(CommentStmt {
              content: content_text,
              location: location_from_pair(&info_child),
            })]);
          }
          _ => unexpected!(info_child, "build_statement", &location_from_pair(&pair)),
        }
      }
      Ok(vec![])
    }
    Rule::doc => {
      let content_pair = pair
        .clone()
        .into_inner()
        .find(|p| p.as_rule() == Rule::content);
      let content_text = content_pair
        .map(|p| p.as_str().to_string())
        .unwrap_or_default();
      Ok(vec![Stmt::Doc(DocStmt {
        content: content_text,
        location: location_from_pair(&pair),
      })])
    }
    Rule::comment => {
      let content_pair = pair
        .clone()
        .into_inner()
        .find(|p| p.as_rule() == Rule::content);
      let content_text = content_pair
        .map(|p| p.as_str().to_string())
        .unwrap_or_default();
      Ok(vec![Stmt::Comment(CommentStmt {
        content: content_text,
        location: location_from_pair(&pair),
      })])
    }
    Rule::EOI => Ok(vec![]),
    Rule::error_push_stmt => {
      let location = location_from_pair(&pair);
      let value = expr_parser::build_expression(
        pair
          .into_inner()
          .find(|p| p.as_rule() == Rule::expression)
          .ok_or("error_push_stmt requires an expression")?,
      )?;
      Ok(vec![Stmt::AddError(AddErrorStmt {
        value,
        location,
        comments: AttachedComments::default(),
      })])
    }
    Rule::error_write_stmt => Ok(vec![Stmt::WriteErrors(WriteErrorsStmt {
      location: location_from_pair(&pair),
      comments: AttachedComments::default(),
    })]),
    Rule::error_warn_stmt => Ok(vec![Stmt::WarnErrors(WarnErrorsStmt {
      location: location_from_pair(&pair),
      comments: AttachedComments::default(),
    })]),
    Rule::error_alert_stmt => Ok(vec![Stmt::AlertErrors(AlertErrorsStmt {
      location: location_from_pair(&pair),
      comments: AttachedComments::default(),
    })]),
    Rule::alert_stmt => {
      let location = location_from_pair(&pair);
      let (comments, remaining, _content_location) = extract_comments(pair);
      let mut value = None;
      let mut inline = false;
      for inner in remaining {
        match inner.as_rule() {
          Rule::alert_inline => inline = true,
          Rule::alert_line => {} // inline = false (default)
          Rule::expression => value = Some(expr_parser::build_expression(inner)?),
          _ => unexpected!(inner, "build_statement", &location),
        }
      }
      Ok(vec![Stmt::Alert(AlertStmt {
        inline,
        value: value.ok_or("missing expression in alert statement")?,
        location,
        comments,
      })])
    }
    Rule::move_on_stmt => Ok(vec![Stmt::MoveOn(build_move_on(pair)?)]),
    Rule::missing_code_stmt => Ok(vec![Stmt::MissingCode(build_missing_code(pair)?)]),
    Rule::release_stmt => {
      let location = location_from_pair(&pair);
      let (comments, remaining, _) = extract_comments(pair);
      let pointer = expr_parser::build_expression(
        remaining
          .into_iter()
          .find(|p| p.as_rule() == Rule::expression)
          .ok_or("release requires a pointer expression")?,
      )?;
      Ok(vec![Stmt::Release(ReleaseStmt {
        pointer,
        location,
        comments,
      })])
    }
    Rule::on_exit_stmt => {
      let location = location_from_pair(&pair);
      let (_, remaining, _) = extract_comments(pair);
      // The inner statement is a deferrable_statement. Find and parse it.
      for p in remaining.into_iter() {
        if p.as_rule() == Rule::info {
          continue;
        }
        let stmts = build_statement(p)?;
        if let Some(body) = stmts.into_iter().next() {
          return Ok(vec![Stmt::OnExit(OnExitStmt {
            body: Box::new(body),
            location,
          })]);
        }
      }
      Err("on exit requires a statement".to_string())
    }
    Rule::test_suite => Ok(vec![Stmt::TestSuite(build_test_suite(pair)?)]),
    Rule::assert_stmt => {
      let location = location_from_pair(&pair);
      let (comments, remaining, _) = extract_comments(pair);
      let condition = expr_parser::build_expression(
        remaining
          .into_iter()
          .find(|p| p.as_rule() == Rule::expression)
          .ok_or("assert requires a condition")?,
      )?;
      Ok(vec![Stmt::Assert(AssertStmt {
        condition,
        location,
        comments,
      })])
    }
    _ => unexpected!(pair, "build_statement"),
  }
}

/// Build a UseStmt (module use statement).
fn build_use_stmt(pair: Pair<Rule>) -> Result<UseStmt, String> {
  let fallback_location = location_from_pair(&pair);
  let (comments, remaining, content_location) = extract_comments(pair);
  let location = content_location.unwrap_or(fallback_location);
  let mut module_path: Vec<Spanned<String>> = Vec::new();
  let mut imports = UseImports::All; // Default: import all symbols

  for inner in remaining {
    match inner.as_rule() {
      Rule::qualified_identifier => {
        // Parse module path segments separated by link (->)
        // Grammar: single_identifier ~ (ms_ln ~ link ~ ms_ln ~ single_identifier)*
        // All paths are relative to current file's directory
        for segment in inner.into_inner() {
          match segment.as_rule() {
            Rule::single_identifier => {
              module_path.push(Spanned::new(
                segment.as_str().to_string(),
                location_from_pair(&segment),
              ));
            }
            Rule::link => {
              // Skip the separator itself
            }
            _ => {
              // Skip ms_ln and other whitespace/separator tokens
            }
          }
        }
      }
      Rule::use_import_list => {
        // Import list is present: parse specific symbols to import
        let mut names = Vec::new();
        for import_inner in inner.into_inner() {
          if import_inner.as_rule() == Rule::single_identifier {
            names.push(Spanned::new(
              import_inner.as_str().to_string(),
              location_from_pair(&import_inner),
            ));
          }
        }
        imports = UseImports::Named(names);
      }
      _ => unexpected!(inner, "build_use_stmt", &location),
    }
  }

  let is_relative = module_path
    .first()
    .is_some_and(|s| s.node == ".." || s.node == ".");

  Ok(UseStmt {
    module_path,
    is_relative,
    imports,
    location,
    comments,
  })
}

/// Build a TypeDef (type definition) statement.
fn build_type_def(pair: Pair<Rule>) -> Result<TypeDefStmt, String> {
  let fallback_location = location_from_pair(&pair);
  let (comments, remaining, content_location) = extract_comments(pair);
  let location = content_location.unwrap_or(fallback_location);
  let mut is_export = false;
  let mut name: Option<Spanned<String>> = None;
  let mut fields = Vec::new();

  for inner in remaining {
    match inner.as_rule() {
      Rule::kw_export => is_export = true,
      Rule::single_identifier => {
        name = Some(Spanned::new(
          inner.as_str().to_string(),
          location_from_pair(&inner),
        ));
      }
      Rule::type_field => {
        let field = build_type_field(inner)?;
        fields.push(field);
      }
      _ => unexpected!(inner, "build_type_def", &location),
    }
  }

  Ok(TypeDefStmt {
    is_export,
    name: name.ok_or("Type definition requires a name")?,
    fields,
    location,
    comments,
  })
}

/// Build an EnumDef (enum definition) statement.
fn build_enum_def(pair: Pair<Rule>) -> Result<EnumDefStmt, String> {
  let fallback_location = location_from_pair(&pair);
  let (comments, remaining, content_location) = extract_comments(pair);
  let location = content_location.unwrap_or(fallback_location);
  let mut is_export = false;
  let mut name: Option<Spanned<String>> = None;
  let mut variants = Vec::new();

  for inner in remaining {
    match inner.as_rule() {
      Rule::kw_export => is_export = true,
      Rule::single_identifier => {
        name = Some(Spanned::new(
          inner.as_str().to_string(),
          location_from_pair(&inner),
        ));
      }
      Rule::enum_variant => {
        let variant = build_enum_variant(inner)?;
        variants.push(variant);
      }
      _ => unexpected!(inner, "build_enum_def", &location),
    }
  }

  Ok(EnumDefStmt {
    is_export,
    name: name.ok_or("Enum definition requires a name")?,
    variants,
    location,
    comments,
  })
}

/// Build a single enum variant.
fn build_enum_variant(pair: Pair<Rule>) -> Result<EnumVariant, String> {
  let location = location_from_pair(&pair);
  let mut name: Option<Spanned<String>> = None;
  let mut fields = Vec::new();
  let mut field_idx = 0u32;

  for inner in pair.into_inner() {
    match inner.as_rule() {
      Rule::single_identifier => {
        name = Some(Spanned::new(
          inner.as_str().to_string(),
          location_from_pair(&inner),
        ));
      }
      Rule::enum_variant_field => {
        // Build a Parameter from the variant field (just a type_name with optional unit).
        // Auto-name: field0, field1, ...
        let field_location = location_from_pair(&inner);
        let mut field_type: Option<TypeName> = None;
        let mut field_unit: Option<Unit> = None;

        for field_inner in inner.into_inner() {
          match field_inner.as_rule() {
            Rule::type_name => {
              field_type = Some(expr_parser::build_type_name(field_inner)?);
            }
            Rule::unit => {
              field_unit = Some(build_unit(field_inner));
            }
            _ => unexpected!(field_inner, "build_enum_variant", &location),
          }
        }

        if let Some(ft) = field_type {
          fields.push(Parameter {
            is_copy: false,
            name: Spanned::new(format!("field{}", field_idx), field_location.clone()),
            type_annotation: ft,
            unit: field_unit,
            location: field_location,
          });
          field_idx += 1;
        }
      }
      _ => unexpected!(inner, "build_enum_variant", &location),
    }
  }

  Ok(EnumVariant {
    name: name.ok_or("Enum variant requires a name")?,
    fields,
    location,
  })
}

/// Build a single field in a type definition.
fn build_type_field(pair: Pair<Rule>) -> Result<TypeField, String> {
  let location = location_from_pair(&pair);
  let (comments, remaining, _) = extract_comments(pair);
  let mut name: Option<Spanned<String>> = None;
  let mut field_type = None;

  let mut unit = None;
  let mut is_private = false;

  for inner in remaining {
    match inner.as_rule() {
      Rule::single_identifier => {
        name = Some(Spanned::new(
          inner.as_str().to_string(),
          location_from_pair(&inner),
        ));
      }
      Rule::type_name => {
        field_type = Some(expr_parser::build_type_name(inner)?);
      }
      Rule::kw_private => is_private = true,
      Rule::unit => unit = Some(build_unit(inner)),
      _ => unexpected!(inner, "build_type_field", &location),
    }
  }

  Ok(TypeField {
    is_private,
    name: name.ok_or("Type field requires a name")?,
    field_type: field_type.ok_or("Type field requires a type")?,
    unit,
    location,
    comments,
  })
}

/// Build a VarDef (variable definition) statement.
fn build_var_def(pair: Pair<Rule>) -> Result<VarDefStmt, String> {
  let fallback_location = location_from_pair(&pair);
  let (comments, remaining, content_location) = extract_comments(pair);
  let location = content_location.unwrap_or(fallback_location);
  let mut is_export = false;
  let mut is_import = false;
  let mut name: Option<Spanned<String>> = None;
  let mut type_annotation = None;
  let mut unit = None;
  let mut value = None;

  for inner in remaining {
    match inner.as_rule() {
      Rule::kw_export => is_export = true,
      Rule::kw_import => is_import = true,
      Rule::var_symbol => {
        for def_inner in inner.into_inner() {
          match def_inner.as_rule() {
            Rule::single_identifier => {
              name = Some(Spanned::new(
                def_inner.as_str().to_string(),
                location_from_pair(&def_inner),
              ));
            }
            Rule::type_name => type_annotation = Some(expr_parser::build_type_name(def_inner)?),
            Rule::unit => unit = Some(build_unit(def_inner)),
            _ => unexpected!(def_inner, "build_var_def", &location),
          }
        }
      }
      Rule::expression => value = Some(expr_parser::build_expression(inner)?),
      _ => unexpected!(inner, "build_var_def", &location),
    }
  }

  Ok(VarDefStmt {
    is_export,
    is_import,
    is_loop_var: false,
    name: name.ok_or("Definition requires a name")?,
    type_annotation,
    unit,
    value: value.ok_or("Definition requires a value expression")?,
    location,
    comments,
  })
}

/// Build an UnsafeDecl (unsafe variable declaration) statement.
fn build_unsafe_decl(pair: Pair<Rule>) -> Result<UnsafeDeclStmt, String> {
  let fallback_location = location_from_pair(&pair);
  let (comments, remaining, content_location) = extract_comments(pair);
  let location = content_location.unwrap_or(fallback_location);
  let mut is_export = false;
  let mut is_import = false;
  let mut name: Option<Spanned<String>> = None;
  let mut type_annotation = None;
  let mut unit = None;

  for inner in remaining {
    match inner.as_rule() {
      Rule::kw_export => is_export = true,
      Rule::kw_import => is_import = true,
      Rule::decl_symbol => {
        for decl_inner in inner.into_inner() {
          match decl_inner.as_rule() {
            Rule::single_identifier => {
              name = Some(Spanned::new(
                decl_inner.as_str().to_string(),
                location_from_pair(&decl_inner),
              ));
            }
            Rule::type_name => type_annotation = Some(expr_parser::build_type_name(decl_inner)?),
            Rule::unit => unit = Some(build_unit(decl_inner)),
            _ => unexpected!(decl_inner, "build_unsafe_decl", &location),
          }
        }
      }
      _ => unexpected!(inner, "build_unsafe_decl", &location),
    }
  }

  Ok(UnsafeDeclStmt {
    is_export,
    is_import,
    name: name.ok_or("Declaration requires a name")?,
    type_annotation,
    unit,
    location,
    comments,
  })
}

/// Build an assignment statement.
fn build_assign(pair: Pair<Rule>) -> Result<AssignStmt, String> {
  let fallback_location = location_from_pair(&pair);
  let (comments, remaining, content_location) = extract_comments(pair);
  let location = content_location.unwrap_or(fallback_location);
  let mut target: Option<Spanned<Vec<String>>> = None;
  let mut indices = Vec::new();
  let mut value = None;

  for inner in remaining {
    match inner.as_rule() {
      Rule::single_identifier => {
        let id_location = location_from_pair(&inner);
        let id_str = inner.as_str().to_string();
        match &mut target {
          None => target = Some(Spanned::new(vec![id_str], id_location)),
          Some(t) => t.node.push(id_str),
        }
      }
      Rule::arr_index => {
        for arr_inner in inner.into_inner() {
          if arr_inner.as_rule() == Rule::expression {
            indices.push(expr_parser::build_expression(arr_inner)?);
          }
        }
      }
      Rule::expression => value = Some(expr_parser::build_expression(inner)?),
      _ => unexpected!(inner, "build_assign", &location),
    }
  }

  Ok(AssignStmt {
    target: target.ok_or("Assignment requires a target")?,
    indices,
    value: value.ok_or("Assignment requires a value expression")?,
    location,
    comments,
  })
}

/// Build a compound assignment statement.
fn build_compound_assign(pair: Pair<Rule>) -> Result<CompoundAssignStmt, String> {
  let fallback_location = location_from_pair(&pair);
  let (comments, remaining, content_location) = extract_comments(pair);
  let location = content_location.unwrap_or(fallback_location);
  let mut target: Option<Spanned<Vec<String>>> = None;
  let mut indices = Vec::new();
  let mut operator: Option<Spanned<CompoundOp>> = None;
  let mut value = None;

  for inner in remaining {
    match inner.as_rule() {
      Rule::single_identifier => {
        let id_location = location_from_pair(&inner);
        let id_str = inner.as_str().to_string();
        match &mut target {
          None => target = Some(Spanned::new(vec![id_str], id_location)),
          Some(t) => t.node.push(id_str),
        }
      }
      Rule::arr_index => {
        for arr_inner in inner.into_inner() {
          if arr_inner.as_rule() == Rule::expression {
            indices.push(expr_parser::build_expression(arr_inner)?);
          }
        }
      }
      Rule::compound_op => {
        let op_location = location_from_pair(&inner);
        let op = match inner.as_str() {
          "+=" => CompoundOp::AddAssign,
          "-=" => CompoundOp::SubAssign,
          "*=" => CompoundOp::MulAssign,
          "/=" => CompoundOp::DivAssign,
          "%=" => CompoundOp::ModAssign,
          _ => return Err(format!("Unknown compound operator: {}", inner.as_str())),
        };
        operator = Some(Spanned::new(op, op_location));
      }
      Rule::expression => value = Some(expr_parser::build_expression(inner)?),
      _ => unexpected!(inner, "build_compound_assign", &location),
    }
  }

  Ok(CompoundAssignStmt {
    target: target.ok_or("Compound assignment requires a target")?,
    indices,
    operator: operator.ok_or("Compound assignment requires an operator")?,
    value: value.ok_or("Compound assignment requires a value expression")?,
    location,
    comments,
  })
}

fn build_value_at_assign(pair: Pair<Rule>) -> Result<ValueAtAssignStmt, String> {
  let fallback_location = location_from_pair(&pair);
  let (comments, remaining, content_location) = extract_comments(pair);
  let location = content_location.unwrap_or(fallback_location);
  let mut pointer: Option<Expr> = None;
  let mut value: Option<Expr> = None;

  for inner in remaining {
    if inner.as_rule() == Rule::expression {
      if pointer.is_none() {
        pointer = Some(expr_parser::build_expression(inner)?);
      } else if value.is_none() {
        value = Some(expr_parser::build_expression(inner)?);
      }
    }
  }

  Ok(ValueAtAssignStmt {
    pointer: pointer.ok_or("value at assign requires a pointer expression")?,
    value: value.ok_or("value at assign requires a value expression")?,
    location,
    comments,
  })
}

/// Build a function definition.
fn build_fn_def(pair: Pair<Rule>) -> Result<FnDefStmt, String> {
  let location = location_from_pair(&pair);
  let (comments, remaining, _content_location) = extract_comments(pair);
  let mut is_export = false;
  let mut name: Option<Spanned<String>> = None;
  let mut parameters = Vec::new();
  let mut return_type = ReturnType {
    kind: ReturnTypeKind::Nothing,
    location: location.clone(),
  };
  let mut return_unit = None;
  let mut body = Vec::new();

  for inner in remaining {
    match inner.as_rule() {
      Rule::kw_export => is_export = true,
      Rule::single_identifier => {
        if name.is_none() {
          name = Some(Spanned::new(
            inner.as_str().to_string(),
            location_from_pair(&inner),
          ));
        }
      }
      Rule::parameters => parameters = build_parameters(inner)?,
      Rule::return_type => return_type = build_return_type(inner)?,
      Rule::unit => return_unit = Some(build_unit(inner)),
      _ => {
        for stmt in build_statement(inner)? {
          body.push(stmt);
        }
      }
    }
  }

  Ok(FnDefStmt {
    is_export,
    name: name.ok_or("Function definition requires a name")?,
    parameters,
    return_type,
    return_unit,
    body,
    location,
    symbol_table: RefCell::new(None),
    comments,
  })
}

/// Build a function signature.
fn build_fn_signature(pair: Pair<Rule>) -> Result<FnSignatureStmt, String> {
  let location = location_from_pair(&pair);
  let (comments, remaining, _content_location) = extract_comments(pair);
  let mut is_import = false;
  let mut name: Option<Spanned<String>> = None;
  let mut parameters = Vec::new();
  let mut return_type = ReturnType {
    kind: ReturnTypeKind::Nothing,
    location: location.clone(),
  };
  let mut return_unit = None;

  for inner in remaining {
    match inner.as_rule() {
      Rule::kw_import => is_import = true,
      Rule::single_identifier => {
        if name.is_none() {
          name = Some(Spanned::new(
            inner.as_str().to_string(),
            location_from_pair(&inner),
          ));
        }
      }
      Rule::parameters => parameters = build_parameters(inner)?,
      Rule::return_type => return_type = build_return_type(inner)?,
      Rule::unit => return_unit = Some(build_unit(inner)),
      _ => unexpected!(inner, "build_fn_signature", &location),
    }
  }

  Ok(FnSignatureStmt {
    is_import,
    name: name.ok_or("Function signature requires a name")?,
    parameters,
    return_type,
    return_unit,
    location,
    comments,
  })
}

/// Build function parameters.
fn build_parameters(pair: Pair<Rule>) -> Result<Vec<Parameter>, String> {
  let mut params = Vec::new();

  for inner in pair.into_inner() {
    if inner.as_rule() == Rule::fn_parameter {
      params.push(build_parameter(inner)?);
    }
  }

  Ok(params)
}

/// Build a single parameter.
fn build_parameter(pair: Pair<Rule>) -> Result<Parameter, String> {
  let location = location_from_pair(&pair);
  let mut is_copy = false;
  let mut name: Option<Spanned<String>> = None;
  let mut type_annotation = None;
  let mut unit = None;

  for inner in pair.into_inner() {
    match inner.as_rule() {
      Rule::kw_copy => is_copy = true,
      Rule::single_identifier => {
        name = Some(Spanned::new(
          inner.as_str().to_string(),
          location_from_pair(&inner),
        ));
      }
      Rule::type_name => type_annotation = Some(expr_parser::build_type_name(inner)?),
      Rule::unit => unit = Some(build_unit(inner)),
      _ => unexpected!(inner, "build_parameter", &location),
    }
  }

  Ok(Parameter {
    is_copy,
    name: name.ok_or("Parameter requires a name")?,
    type_annotation: type_annotation.ok_or("Parameter requires a type")?,
    unit,
    location,
  })
}

/// Build a return type.
fn build_return_type(pair: Pair<Rule>) -> Result<ReturnType, String> {
  let location = location_from_pair(&pair);
  let text = pair.as_str().trim();
  if text == "nothing" {
    return Ok(ReturnType {
      kind: ReturnTypeKind::Nothing,
      location,
    });
  }

  for inner in pair.into_inner() {
    if inner.as_rule() == Rule::type_name {
      return Ok(ReturnType {
        kind: ReturnTypeKind::Type(expr_parser::build_type_name(inner)?),
        location,
      });
    }
  }

  Ok(ReturnType {
    kind: ReturnTypeKind::Nothing,
    location,
  })
}

/// Build a unit.
fn build_unit(pair: Pair<Rule>) -> Unit {
  Unit {
    raw: pair.as_str().to_string(),
    location: location_from_pair(&pair),
  }
}

/// Build an if statement.
fn build_if(pair: Pair<Rule>) -> Result<IfStmt, String> {
  let location = location_from_pair(&pair);
  let (comments, remaining, _content_location) = extract_comments(pair);
  let mut conditions = Vec::new();
  let mut branches: Vec<Vec<Stmt>> = Vec::new();
  let mut current_branch = Vec::new();
  let mut seen_first_branch = false;

  for inner in remaining {
    match inner.as_rule() {
      Rule::condition => {
        // Save the current branch when we see a new condition
        if !current_branch.is_empty() || !conditions.is_empty() {
          branches.push(std::mem::take(&mut current_branch));
        }
        conditions.push(build_condition(inner)?);
      }
      Rule::stmt_block => {
        // If we've already seen a branch and are seeing another without a condition between,
        // it means we're at the else branch (no condition before it). In this case,
        // we need to ensure the previous branch was saved.
        if seen_first_branch && !current_branch.is_empty() && conditions.len() > branches.len() {
          // We have accumulated statements from a previous branch but haven't saved it yet
          branches.push(std::mem::take(&mut current_branch));
        }
        seen_first_branch = true;

        // Accumulate statements for the current branch
        for stmt_pair in inner.into_inner() {
          for stmt in build_statement(stmt_pair)? {
            current_branch.push(stmt);
          }
        }
      }
      _ => {
        // Ignore info comments and other non-statement rules
        for stmt in build_statement(inner)? {
          current_branch.push(stmt);
        }
      }
    }
  }

  // Save any remaining branch statements
  if !current_branch.is_empty() {
    branches.push(current_branch);
  }

  // Determine the structure:
  // - If we have N conditions, we should have N or N+1 branches
  // - N branches: all conditions have branches, no else
  // - N+1 branches: all conditions have branches, plus an else branch
  let else_branch = if branches.len() > conditions.len() {
    branches.pop() // Safe because we just checked len > 0
  } else {
    None
  };

  let then_branch = branches.first().cloned().unwrap_or_default();

  // Extract the first condition for the main if, remaining conditions for else-if branches
  let first_condition = conditions
    .first()
    .cloned()
    .ok_or("If statement requires a condition")?;
  let else_if_branches: Vec<(Condition, Vec<Stmt>)> = conditions
    .into_iter()
    .skip(1)
    .zip(branches.into_iter().skip(1))
    .collect();

  Ok(IfStmt {
    condition: first_condition,
    then_branch,
    else_if_branches,
    else_branch,
    location,
    comments,
  })
}

/// Build a when statement: one-sided action (no else).
fn build_when(pair: Pair<Rule>) -> Result<WhenStmt, String> {
  let location = location_from_pair(&pair);
  let (comments, remaining, _content_location) = extract_comments(pair);
  let mut condition_opt: Option<Condition> = None;
  let mut body = Vec::new();
  let mut else_branch: Option<Vec<Stmt>> = None;
  let mut seen_first_body = false;

  for inner in remaining {
    match inner.as_rule() {
      Rule::condition => {
        condition_opt = Some(build_condition(inner)?);
      }
      Rule::stmt_block => {
        let mut stmts = Vec::new();
        for stmt_pair in inner.into_inner() {
          for stmt in build_statement(stmt_pair)? {
            stmts.push(stmt);
          }
        }
        if !seen_first_body {
          body = stmts;
          seen_first_body = true;
        } else {
          else_branch = Some(stmts);
        }
      }
      _ => {
        // Info comments, etc.
        for stmt in build_statement(inner)? {
          body.push(stmt);
        }
      }
    }
  }

  Ok(WhenStmt {
    condition: condition_opt.ok_or("When statement requires a condition")?,
    body,
    else_branch,
    location,
    comments,
  })
}

/// Build a match statement: conditional branching with multiple arms.
fn build_match(pair: Pair<Rule>) -> Result<MatchStmt, String> {
  let location = location_from_pair(&pair);
  let (comments, remaining, _content_location) = extract_comments(pair);
  let mut arms = Vec::new();
  let mut else_arm = Vec::new();

  for inner in remaining {
    match inner.as_rule() {
      Rule::match_arm => {
        arms.push(build_match_arm(inner)?);
      }
      Rule::match_default => {
        else_arm = build_match_default_body(inner)?;
      }
      _ => {
        return Err(format!(
          "Unexpected rule in match_stmt: {:?}",
          inner.as_rule()
        ));
      }
    }
  }

  Ok(MatchStmt {
    arms,
    else_arm,
    location,
    comments,
  })
}

/// Build a single match arm from a `match_arm` parse pair.
/// The arm contains a condition (guard) and an optional body.
fn build_match_arm(pair: Pair<Rule>) -> Result<MatchArm, String> {
  let location = location_from_pair(&pair);
  let mut guard: Option<Condition> = None;
  let mut body: Vec<Stmt> = Vec::new();

  for inner in pair.into_inner() {
    match inner.as_rule() {
      Rule::condition => {
        guard = Some(build_condition(inner)?);
      }
      Rule::stmt_block => {
        for stmt_pair in inner.into_inner() {
          for stmt in build_statement(stmt_pair)? {
            body.push(stmt);
          }
        }
      }
      _ => {
        return Err(format!(
          "Unexpected rule in match_arm: {:?}",
          inner.as_rule()
        ));
      }
    }
  }

  let guard = guard.ok_or_else(|| "match_arm missing condition".to_string())?;
  Ok(MatchArm {
    guard,
    body,
    location,
  })
}

/// Build the default arm body from a `match_default` parse pair.
/// Returns an empty Vec if the default has no body.
fn build_match_default_body(pair: Pair<Rule>) -> Result<Vec<Stmt>, String> {
  let mut body = Vec::new();
  for inner in pair.into_inner() {
    if inner.as_rule() == Rule::stmt_block {
      for stmt_pair in inner.into_inner() {
        for stmt in build_statement(stmt_pair)? {
          body.push(stmt);
        }
      }
    }
  }
  Ok(body)
}

/// Build a test suite statement: a named group of test cases.
/// Build an `AttachedComment` (Doc or Comment) from an `info` parse node.
fn build_attached_comment(info: &Pair<Rule>) -> Result<AttachedComment, String> {
  for info_child in info.clone().into_inner() {
    let content_text = info_child
      .clone()
      .into_inner()
      .find(|p| p.as_rule() == Rule::content)
      .map(|p| p.as_str().to_string())
      .unwrap_or_default();
    match info_child.as_rule() {
      Rule::doc => {
        return Ok(AttachedComment::Doc(DocStmt {
          content: content_text,
          location: location_from_pair(&info_child),
        }));
      }
      Rule::comment => {
        return Ok(AttachedComment::Comment(CommentStmt {
          content: content_text,
          location: location_from_pair(&info_child),
        }));
      }
      _ => {}
    }
  }
  Err("info node contained no doc or comment".to_string())
}

fn build_test_suite(pair: Pair<Rule>) -> Result<TestSuiteStmt, String> {
  let location = location_from_pair(&pair);
  let mut name = String::new();
  let mut items = Vec::new();
  let mut leading = Vec::new();
  let mut seen_name = false;

  // Walk the suite's children in source order. Comments before the suite name
  // are leading comments; after the name, standalone comments/docs become
  // `TestSuiteItem` nodes interleaved with cases (preserving source fidelity).
  for inner in pair.into_inner() {
    match inner.as_rule() {
      Rule::info => {
        let comment = build_attached_comment(&inner)?;
        if seen_name {
          match comment {
            AttachedComment::Doc(d) => items.push(TestSuiteItem::Doc(d)),
            AttachedComment::Comment(c) => items.push(TestSuiteItem::Comment(c)),
          }
        } else {
          leading.push(comment);
        }
      }
      Rule::single_identifier => {
        name = inner.as_str().to_string();
        seen_name = true;
      }
      Rule::test_case => {
        items.push(TestSuiteItem::Case(build_test_case(inner)?));
      }
      Rule::var | Rule::fn_def => {
        // Suite-level declaration. `build_statement` returns a single-element
        // Vec for these rules, but we iterate to be robust to future changes.
        for stmt in build_statement(inner)? {
          items.push(TestSuiteItem::Declaration(Box::new(stmt)));
        }
      }
      _ => {}
    }
  }

  Ok(TestSuiteStmt {
    name,
    items,
    location,
    comments: AttachedComments {
      leading,
      trailing: None,
    },
  })
}

/// Build a single test case: a named block of test statements.
fn build_test_case(pair: Pair<Rule>) -> Result<TestCaseStmt, String> {
  let location = location_from_pair(&pair);
  let (comments, remaining, _content_location) = extract_comments(pair);
  let mut name = String::new();
  let mut body = Vec::new();

  for inner in remaining {
    match inner.as_rule() {
      Rule::single_identifier => {
        name = inner.as_str().to_string();
      }
      Rule::stmt_block => {
        for stmt_pair in inner.into_inner() {
          for stmt in build_statement(stmt_pair)? {
            body.push(stmt);
          }
        }
      }
      Rule::info => {
        // Stray comment node — already captured by extract_comments.
      }
      other => return Err(format!("Unexpected rule in test_case: {:?}", other)),
    }
  }

  Ok(TestCaseStmt {
    name,
    body,
    location,
    comments,
  })
}

/// Build a move on statement: intentional no-op.
fn build_move_on(pair: Pair<Rule>) -> Result<MoveOnStmt, String> {
  let location = location_from_pair(&pair);
  let (comments, _remaining, _) = extract_comments(pair);
  Ok(MoveOnStmt { location, comments })
}

/// Build a missing code statement: deferred implementation placeholder.
fn build_missing_code(pair: Pair<Rule>) -> Result<MissingCodeStmt, String> {
  let location = location_from_pair(&pair);
  let (comments, _remaining, _) = extract_comments(pair);
  Ok(MissingCodeStmt { location, comments })
}

/// Build a switch statement: exhaustive pattern matching over enums.
fn build_switch(pair: Pair<Rule>) -> Result<SwitchStmt, String> {
  let fallback_location = location_from_pair(&pair);
  let (comments, remaining, content_location) = extract_comments(pair);
  let location = content_location.unwrap_or(fallback_location);
  let mut value: Option<Expr> = None;
  let mut cases = Vec::new();
  let mut default_case = None;
  let mut default_location = None;

  for inner in remaining {
    match inner.as_rule() {
      Rule::expression => {
        value = Some(expr_parser::build_expression(inner)?);
      }
      Rule::switch_case => {
        let case = build_switch_case(inner)?;
        cases.push(case);
      }
      Rule::switch_default => {
        default_location = Some(location_from_pair(&inner));
        default_case = Some(build_switch_default(inner)?);
      }
      _ => unexpected!(inner, "build_switch", &location),
    }
  }

  Ok(SwitchStmt {
    value: value.ok_or("Switch statement requires an expression to match")?,
    cases,
    default_case,
    default_location,
    location,
    comments,
  })
}

/// Build a single switch case: pattern -> stmt.
fn build_switch_case(pair: Pair<Rule>) -> Result<SwitchCase, String> {
  let location = location_from_pair(&pair);
  let mut pattern: Option<SwitchPattern> = None;
  let mut body = Vec::new();

  for inner in pair.into_inner() {
    match inner.as_rule() {
      Rule::switch_pattern => {
        pattern = Some(build_switch_pattern(inner)?);
      }
      Rule::stmt_block => {
        // Expand stmt_block: iterate over its statement children
        for child in inner.into_inner() {
          for stmt in build_statement(child)? {
            body.push(stmt);
          }
        }
      }
      _ => {
        for stmt in build_statement(inner)? {
          body.push(stmt);
        }
      }
    }
  }

  Ok(SwitchCase {
    pattern: pattern.ok_or("Switch case requires a pattern")?,
    body,
    location,
  })
}

/// Build the default arm of a switch statement.
fn build_switch_default(pair: Pair<Rule>) -> Result<Vec<Stmt>, String> {
  let mut body = Vec::new();

  for inner in pair.into_inner() {
    match inner.as_rule() {
      Rule::stmt_block => {
        for child in inner.into_inner() {
          for stmt in build_statement(child)? {
            body.push(stmt);
          }
        }
      }
      _ => {
        for stmt in build_statement(inner)? {
          body.push(stmt);
        }
      }
    }
  }

  Ok(body)
}

/// Build a switch pattern: an enum variant pattern or a literal value pattern.
fn build_switch_pattern(pair: Pair<Rule>) -> Result<SwitchPattern, String> {
  for inner in pair.into_inner() {
    let location = location_from_pair(&inner);
    match inner.as_rule() {
      Rule::b_literal => {
        return Ok(SwitchPattern::Literal {
          value: Box::new(Expr::BoolLiteral(BoolLiteral {
            value: inner.as_str() == "true",
            location: location.clone(),
          })),
          location,
        });
      }
      Rule::switch_variant_pattern => {
        return build_switch_variant_pattern(inner, location);
      }
      Rule::expression => {
        return Ok(SwitchPattern::Literal {
          value: Box::new(expr_parser::build_expression(inner)?),
          location,
        });
      }
      _ => {}
    }
  }

  Err("Switch pattern requires a variant or literal".to_string())
}

/// Build an enum variant switch pattern: `Circle(r)`, `Point`, `Rectangle(_, _)`.
fn build_switch_variant_pattern(
  pair: Pair<Rule>,
  location: SourceLocation,
) -> Result<SwitchPattern, String> {
  let mut variant_name: Option<Spanned<String>> = None;
  let mut fields = Vec::new();

  for inner in pair.into_inner() {
    match inner.as_rule() {
      Rule::single_identifier => {
        variant_name = Some(Spanned::new(
          inner.as_str().to_string(),
          location_from_pair(&inner),
        ));
      }
      Rule::switch_pattern_field => {
        let text = inner.as_str();
        if text == "_" {
          fields.push(SwitchPatternField::Discard);
        } else {
          fields.push(SwitchPatternField::Bind(text.to_string()));
        }
      }
      _ => unexpected!(inner, "build_switch_variant_pattern", &location),
    }
  }

  Ok(SwitchPattern::Enum {
    variant_name: variant_name.ok_or("Switch pattern requires a variant name")?,
    fields,
    location,
  })
}

/// Build a condition.
fn build_condition(pair: Pair<Rule>) -> Result<Condition, String> {
  let location = location_from_pair(&pair);
  let mut negated = false;
  let mut expr = None;

  for inner in pair.into_inner() {
    match inner.as_rule() {
      Rule::kw_not => negated = true,
      Rule::expression => expr = Some(expr_parser::build_expression(inner)?),
      _ => unexpected!(inner, "build_condition", &location),
    }
  }

  Ok(Condition {
    negated,
    expr: expr.ok_or("Condition requires an expression")?,
    location,
  })
}

/// Build a loop statement.
fn build_loop(pair: Pair<Rule>) -> Result<LoopStmt, String> {
  let location = location_from_pair(&pair);
  let (comments, remaining, _content_location) = extract_comments(pair);
  let mut range = None;
  let mut pre_condition = None;
  let mut body = Vec::new();
  let mut post_condition = None;
  let mut seen_body = false;

  for inner in remaining {
    match inner.as_rule() {
      Rule::range => range = Some(build_range(inner)?),
      Rule::condition => {
        // First condition before any body statements is entry condition
        // Second condition (after body) is exit condition
        if seen_body {
          post_condition = Some(build_condition(inner)?);
        } else {
          pre_condition = Some(build_condition(inner)?);
        }
      }
      _ => {
        // Try to build as a statement
        for stmt in build_statement(inner)? {
          body.push(stmt);
          seen_body = true;
        }
      }
    }
  }

  Ok(LoopStmt {
    range,
    pre_condition,
    body,
    post_condition,
    location,
    comments,
  })
}

/// Build a range.
fn build_range(pair: Pair<Rule>) -> Result<Range, String> {
  let location = location_from_pair(&pair);
  let mut variable: Option<Spanned<String>> = None;
  let mut var_type = None;
  let mut from = None;
  let mut to = None;
  let mut step = None;
  let mut expr_count = 0;

  for inner in pair.into_inner() {
    match inner.as_rule() {
      Rule::single_identifier => {
        variable = Some(Spanned::new(
          inner.as_str().to_string(),
          location_from_pair(&inner),
        ));
      }
      Rule::type_name => var_type = Some(expr_parser::build_type_name(inner)?),
      Rule::expression => {
        let expr = expr_parser::build_expression(inner)?;
        match expr_count {
          0 => from = Some(expr),
          1 => to = Some(expr),
          2 => step = Some(expr),
          _ => {}
        }
        expr_count += 1;
      }
      _ => unexpected!(inner, "build_range", &location),
    }
  }

  Ok(Range {
    variable: variable.ok_or("Range requires a variable")?,
    var_type: var_type.ok_or("Range requires a type")?,
    from: from.ok_or("Range requires a from expression")?,
    to: to.ok_or("Range requires a to expression")?,
    step,
    location,
  })
}

/// Build a return statement.
fn build_return(pair: Pair<Rule>) -> Result<ReturnStmt, String> {
  let location = location_from_pair(&pair);
  let (comments, remaining, _content_location) = extract_comments(pair);
  let mut value = None;

  for inner in remaining {
    if inner.as_rule() == Rule::expression {
      value = Some(expr_parser::build_expression(inner)?);
    }
  }

  Ok(ReturnStmt {
    value,
    location,
    comments,
  })
}

/// Build an exit program statement.
fn build_exit_program(pair: Pair<Rule>) -> Result<ExitProgramStmt, String> {
  let location = location_from_pair(&pair);
  let (comments, remaining, _content_location) = extract_comments(pair);
  let mut code = None;

  for inner in remaining {
    if inner.as_rule() == Rule::int {
      code = inner.as_str().parse().ok();
    }
  }

  Ok(ExitProgramStmt {
    code,
    location,
    comments,
  })
}

/// Build an exit loop statement.
fn build_exit_loop(pair: Pair<Rule>) -> Result<ExitLoopStmt, String> {
  let location = location_from_pair(&pair);
  let (comments, _remaining, _content_location) = extract_comments(pair);
  Ok(ExitLoopStmt { location, comments })
}

/// Build a rewind statement.
fn build_rewind(pair: Pair<Rule>) -> Result<RewindStmt, String> {
  let location = location_from_pair(&pair);
  let (comments, _remaining, _content_location) = extract_comments(pair);
  Ok(RewindStmt { location, comments })
}

/// Build a stdout statement.
fn build_stdout(pair: Pair<Rule>) -> Result<StdoutStmt, String> {
  let location = location_from_pair(&pair);
  let (comments, remaining, _content_location) = extract_comments(pair);
  let mut inline = false;
  let mut value = None;
  let mut target = None;

  for inner in remaining {
    match inner.as_rule() {
      Rule::write_inline => inline = true,
      Rule::write_line => inline = false,
      Rule::expression => value = Some(expr_parser::build_expression(inner)?),
      Rule::single_identifier => {
        target = Some(Spanned::new(
          inner.as_str().to_string(),
          location_from_pair(&inner),
        ));
      }
      _ => unexpected!(inner, "build_stdout", &location),
    }
  }

  Ok(StdoutStmt {
    inline,
    value: value.ok_or("stdout requires a value")?,
    target,
    location,
    comments,
  })
}

/// Build a stderr statement.
fn build_stderr(pair: Pair<Rule>) -> Result<StderrStmt, String> {
  let location = location_from_pair(&pair);
  let (comments, remaining, _content_location) = extract_comments(pair);
  let mut inline = false;
  let mut value = None;
  let mut target = None;

  for inner in remaining {
    match inner.as_rule() {
      Rule::warn_inline => inline = true,
      Rule::warn_line => inline = false,
      Rule::expression => value = Some(expr_parser::build_expression(inner)?),
      Rule::single_identifier => {
        target = Some(Spanned::new(
          inner.as_str().to_string(),
          location_from_pair(&inner),
        ));
      }
      _ => unexpected!(inner, "build_stderr", &location),
    }
  }

  Ok(StderrStmt {
    inline,
    value: value.ok_or("stderr requires a value")?,
    target,
    location,
    comments,
  })
}

/// Build a debug statement.
fn build_debug(pair: Pair<Rule>) -> Result<DebugStmt, String> {
  let location = location_from_pair(&pair);
  let (comments, remaining, _content_location) = extract_comments(pair);
  let mut value = None;
  let mut expr_text = String::new();

  for inner in remaining {
    if inner.as_rule() == Rule::expression {
      expr_text = inner.as_str().to_string();
      value = Some(expr_parser::build_expression(inner)?);
    }
  }

  Ok(DebugStmt {
    value: value.ok_or("debug requires an expression")?,
    expr_text,
    location,
    comments,
  })
}

/// Build a stdin statement.
fn build_stdin(pair: Pair<Rule>) -> Result<StdinStmt, String> {
  let location = location_from_pair(&pair);
  let (comments, remaining, _content_location) = extract_comments(pair);
  let mut target: Option<Spanned<Vec<String>>> = None;

  for inner in remaining {
    if inner.as_rule() == Rule::single_identifier {
      let id_location = location_from_pair(&inner);
      let id_str = inner.as_str().to_string();
      match &mut target {
        None => target = Some(Spanned::new(vec![id_str], id_location)),
        Some(t) => t.node.push(id_str),
      }
    }
  }

  Ok(StdinStmt {
    target: target.ok_or("stdin requires a target variable")?,
    location,
    comments,
  })
}

/// Build a compile-time if statement.
fn build_ct_if(pair: Pair<Rule>) -> Result<CtIfStmt, String> {
  let location = location_from_pair(&pair);
  let if_stmt = build_if(pair)?;

  Ok(CtIfStmt {
    condition: if_stmt.condition,
    then_branch: if_stmt.then_branch,
    else_if_branches: if_stmt.else_if_branches,
    else_branch: if_stmt.else_branch,
    location,
    comments: if_stmt.comments,
  })
}

/// Build a compile-time fail statement.
fn build_ct_fail(pair: Pair<Rule>) -> Result<CtFailStmt, String> {
  let location = location_from_pair(&pair);
  let (comments, remaining, _content_location) = extract_comments(pair);
  let mut message = String::new();

  for inner in remaining {
    if inner.as_rule() == Rule::s_literal {
      message = extract_string_content(inner);
    }
  }

  Ok(CtFailStmt {
    message,
    location,
    comments,
  })
}

/// Build a compile-time warn statement.
fn build_ct_warn(pair: Pair<Rule>) -> Result<CtWarnStmt, String> {
  let location = location_from_pair(&pair);
  let (comments, remaining, _content_location) = extract_comments(pair);
  let mut message = String::new();

  for inner in remaining {
    if inner.as_rule() == Rule::s_literal {
      message = extract_string_content(inner);
    }
  }

  Ok(CtWarnStmt {
    message,
    location,
    comments,
  })
}

/// Build a compile-time when statement (#when ... #end when).
fn build_ct_when(pair: Pair<Rule>) -> Result<CtWhenStmt, String> {
  let location = location_from_pair(&pair);
  let (comments, remaining, _content_location) = extract_comments(pair);
  let mut condition_opt: Option<Condition> = None;
  let mut body = Vec::new();

  for inner in remaining {
    match inner.as_rule() {
      Rule::condition => {
        condition_opt = Some(build_condition(inner)?);
      }
      Rule::stmt_block => {
        for stmt_pair in inner.into_inner() {
          for stmt in build_statement(stmt_pair)? {
            body.push(stmt);
          }
        }
      }
      _ => {
        for stmt in build_statement(inner)? {
          body.push(stmt);
        }
      }
    }
  }

  Ok(CtWhenStmt {
    condition: condition_opt.ok_or("Compile-time when requires a condition")?,
    body,
    location,
    comments,
  })
}

/// Build a compile-time match statement (#match ... #end match).
fn build_ct_match(pair: Pair<Rule>) -> Result<CtMatchStmt, String> {
  let location = location_from_pair(&pair);
  let (comments, remaining, _content_location) = extract_comments(pair);
  let mut arms = Vec::new();
  let mut else_arm = Vec::new();

  for inner in remaining {
    match inner.as_rule() {
      Rule::ct_match_arm => {
        arms.push(build_ct_match_arm(inner)?);
      }
      Rule::ct_match_default => {
        else_arm = build_match_default_body(inner)?;
      }
      _ => {
        return Err(format!(
          "Unexpected rule in ct_match: {:?}",
          inner.as_rule()
        ));
      }
    }
  }

  Ok(CtMatchStmt {
    arms,
    else_arm,
    location,
    comments,
  })
}

/// Build a compile-time match arm (reuses MatchArm).
fn build_ct_match_arm(pair: Pair<Rule>) -> Result<MatchArm, String> {
  let location = location_from_pair(&pair);
  let mut guard: Option<Condition> = None;
  let mut body = Vec::new();

  for inner in pair.into_inner() {
    match inner.as_rule() {
      Rule::condition => {
        guard = Some(build_condition(inner)?);
      }
      Rule::stmt_block => {
        for stmt_pair in inner.into_inner() {
          for stmt in build_statement(stmt_pair)? {
            body.push(stmt);
          }
        }
      }
      _ => {
        return Err(format!(
          "Unexpected rule in ct_match_arm: {:?}",
          inner.as_rule()
        ));
      }
    }
  }

  let guard = guard.ok_or_else(|| "ct_match_arm missing condition".to_string())?;
  Ok(MatchArm {
    guard,
    body,
    location,
  })
}

/// Build a compile-time switch statement (#switch ... #end switch).
fn build_ct_switch(pair: Pair<Rule>) -> Result<CtSwitchStmt, String> {
  let location = location_from_pair(&pair);
  let (comments, remaining, _content_location) = extract_comments(pair);
  let mut value: Option<Expr> = None;
  let mut cases = Vec::new();
  let mut default_case = Vec::new();

  for inner in remaining {
    match inner.as_rule() {
      Rule::expression => {
        value = Some(expr_parser::build_expression(inner)?);
      }
      Rule::ct_switch_case => {
        cases.push(build_ct_switch_case(inner)?);
      }
      Rule::ct_switch_default => {
        default_case = build_match_default_body(inner)?;
      }
      _ => unexpected!(inner, "build_ct_switch", &location),
    }
  }

  Ok(CtSwitchStmt {
    value: value.ok_or("Compile-time switch requires an expression to match")?,
    cases,
    default_case,
    location,
    comments,
  })
}

/// Build a compile-time switch case (#case <literal>: <body>).
fn build_ct_switch_case(pair: Pair<Rule>) -> Result<CtSwitchCase, String> {
  let location = location_from_pair(&pair);
  let mut value: Option<Expr> = None;
  let mut body = Vec::new();

  for inner in pair.into_inner() {
    match inner.as_rule() {
      Rule::expression => {
        value = Some(expr_parser::build_expression(inner)?);
      }
      Rule::stmt_block => {
        for child in inner.into_inner() {
          for stmt in build_statement(child)? {
            body.push(stmt);
          }
        }
      }
      _ => {
        for stmt in build_statement(inner)? {
          body.push(stmt);
        }
      }
    }
  }

  Ok(CtSwitchCase {
    value: value.ok_or("Compile-time switch case requires a literal")?,
    body,
    location,
  })
}

/// Extract string content from a string pair, expanding escape sequences.
fn extract_string_content(pair: Pair<Rule>) -> String {
  let s = pair.as_str();
  let raw = if s.starts_with('"') && s.ends_with('"') {
    s[1..s.len() - 1].to_string()
  } else {
    s.to_string()
  };
  crate::ast::expr_parser::expand_escapes_str(&raw)
}
