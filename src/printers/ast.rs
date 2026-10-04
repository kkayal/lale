//! AST Printer - Pretty-prints the Lale AST.
//!
//! This module implements the `AstVisitor` trait to produce a formatted,
//! hierarchical view of the AST. Output includes source locations and
//! optional color coding for different node types.

use crate::ast::{AstVisitor, *};
use colored::*;

/// A visitor implementation for pretty-printing the AST.
pub struct AstPrinter<'a> {
  file_name: &'a str,
  use_colors: bool,
  indent_level: usize,
}

impl<'a> AstPrinter<'a> {
  /// Create a new AST printer.
  pub fn new(file_name: &'a str, use_colors: bool) -> Self {
    AstPrinter {
      file_name,
      use_colors,
      indent_level: 0,
    }
  }

  /// Generate indentation string based on current level.
  fn indent(&self) -> String {
    if self.indent_level == 0 {
      String::new()
    } else {
      format!(
        "L{:02} {}",
        self.indent_level,
        "│ ".repeat(self.indent_level - 1)
      )
    }
  }

  /// Print an internal AST node with location info.
  fn print_node(&mut self, name: &str, loc: &SourceLocation) {
    let colored_name = self.colorize_name(name);
    let prefix = if self.indent_level > 0 { "├─ " } else { "" };
    let span = format!("[{}-{}]", loc.start_pos, loc.end_pos);
    let location = format!("{}:{}:{}", self.file_name, loc.line, loc.col);

    println!(
      "{}{}{} {} {}",
      self.indent(),
      prefix,
      colored_name,
      span,
      location
    );
  }

  /// Print an organizational/structural label without location (for grouping children).
  fn print_label(&mut self, name: &str) {
    let colored_name = self.colorize_name(name);
    let prefix = if self.indent_level > 0 { "├─ " } else { "" };
    println!("{}{}{}", self.indent(), prefix, colored_name);
  }

  /// Colorize a node name based on its type.
  fn colorize_name(&self, name: &str) -> String {
    if self.use_colors {
      if name.starts_with("#") {
        name.red().to_string()
      } else if name.starts_with("Fn") || name.starts_with("fn") {
        name.cyan().to_string()
      } else if name.contains("Literal") || name.contains("literal") {
        name.green().to_string()
      } else if name == "Identifier" || name == "identifier" {
        name.yellow().to_string()
      } else if name.contains("doc") || name.contains("comment") {
        name.dimmed().to_string()
      } else {
        name.blue().to_string()
      }
    } else {
      name.to_string()
    }
  }

  /// Print a leaf node (terminal) with value.
  fn print_leaf(&mut self, name: &str, value: &str, loc: &SourceLocation) {
    let colored_name = if self.use_colors {
      if name == "identifier" {
        format!("{}: \"{}\"", name, value).yellow().to_string()
      } else if name.contains("literal") {
        format!("{}: \"{}\"", name, value).green().to_string()
      } else if name.contains("doc") || name.contains("comment") {
        format!("{}: \"{}\"", name, value).dimmed().to_string()
      } else {
        format!("{}: \"{}\"", name, value).normal().to_string()
      }
    } else {
      format!("{}: \"{}\"", name, value)
    };

    let prefix = if self.indent_level > 0 { "├─ " } else { "" };
    let span = format!("[{}-{}]", loc.start_pos, loc.end_pos);
    let location = format!("{}:{}:{}", self.file_name, loc.line, loc.col);

    println!(
      "{}{}{} {} {}",
      self.indent(),
      prefix,
      colored_name,
      span,
      location
    );
  }

  /// Execute closure with increased indentation level.
  fn with_indent<F, T>(&mut self, f: F) -> T
  where
    F: FnOnce(&mut Self) -> T,
  {
    self.indent_level += 1;
    let result = f(self);
    self.indent_level -= 1;
    result
  }

  /// Print attached comments (leading and trailing).
  fn print_attached_comments(&mut self, comments: &AttachedComments) {
    for comment in &comments.leading {
      match comment {
        AttachedComment::Doc(doc) => {
          self.print_leaf("leading_doc", &doc.content, &doc.location);
        }
        AttachedComment::Comment(c) => {
          self.print_leaf("leading_comment", &c.content, &c.location);
        }
      }
    }
    if let Some(trailing) = &comments.trailing {
      match trailing {
        AttachedComment::Doc(doc) => {
          self.print_leaf("trailing_doc", &doc.content, &doc.location);
        }
        AttachedComment::Comment(c) => {
          self.print_leaf("trailing_comment", &c.content, &c.location);
        }
      }
    }
  }
}

impl<'a> AstVisitor<()> for AstPrinter<'a> {
  fn visit_program(&mut self, program: &Program) {
    self.print_node("Program", &program.location);
    self.with_indent(|printer| {
      for stmt in &program.statements {
        printer.visit_stmt(stmt);
      }
    });
  }

  fn visit_use(&mut self, use_stmt: &UseStmt) {
    self.print_node("Use", &use_stmt.location);
    self.with_indent(|printer| {
      printer.print_attached_comments(&use_stmt.comments);
      let origin_str = match use_stmt.origin.node {
        ModuleOrigin::Std => "std",
        ModuleOrigin::Local => "local",
      };
      printer.print_label(&format!("origin: {}", origin_str));
      let path_str: Vec<_> = use_stmt.path.iter().map(|s| s.node.as_str()).collect();
      printer.print_label(&format!("path: {}", path_str.join(".")));
      match &use_stmt.imports {
        UseImports::All => {
          printer.print_label("imports: [all]");
        }
        UseImports::Named(names) => {
          let names_str: Vec<_> = names.iter().map(|s| s.node.as_str()).collect();
          printer.print_label(&format!("imports: {}", names_str.join(", ")));
        }
      }
    });
  }

  fn visit_type_def(&mut self, type_def: &TypeDefStmt) {
    let mut desc = "TypeDef".to_string();
    if type_def.is_export {
      desc = format!("export {}", desc);
    }
    self.print_node(&desc, &type_def.location);

    self.with_indent(|printer| {
      printer.print_attached_comments(&type_def.comments);
      printer.print_leaf("identifier", &type_def.name.node, &type_def.name.span);

      if !type_def.fields.is_empty() {
        printer.print_label("fields");
        printer.with_indent(|printer| {
          for field in &type_def.fields {
            printer.print_node("Field", &field.location);
            printer.with_indent(|printer| {
              printer.print_attached_comments(&field.comments);
              printer.print_leaf("name", &field.name.node, &field.name.span);
              printer.visit_type_name(&field.field_type);
            });
          }
        });
      }
    });
  }

  fn visit_enum_def(&mut self, enum_def: &EnumDefStmt) {
    let mut desc = "EnumDef".to_string();
    if enum_def.is_export {
      desc = format!("export {}", desc);
    }
    self.print_node(&desc, &enum_def.location);

    self.with_indent(|printer| {
      printer.print_attached_comments(&enum_def.comments);
      printer.print_leaf("identifier", &enum_def.name.node, &enum_def.name.span);

      if !enum_def.variants.is_empty() {
        printer.print_label("variants");
        printer.with_indent(|printer| {
          for variant in &enum_def.variants {
            printer.print_node("Variant", &variant.location);
            printer.with_indent(|printer| {
              printer.print_leaf("name", &variant.name.node, &variant.name.span);
              if !variant.fields.is_empty() {
                printer.print_label("fields");
                printer.with_indent(|p| {
                  for field in &variant.fields {
                    p.print_leaf("field", &field.name.node, &field.name.span);
                    p.visit_type_name(&field.type_annotation);
                  }
                });
              }
            });
          }
        });
      }
    });
  }

  fn visit_var_def(&mut self, var_def: &VarDefStmt) {
    let mut desc = "VarDef".to_string();
    if var_def.is_export {
      desc = format!("export {}", desc);
    }
    if var_def.is_import {
      desc = format!("import {}", desc);
    }
    self.print_node(&desc, &var_def.location);

    self.with_indent(|printer| {
      printer.print_attached_comments(&var_def.comments);
      printer.print_leaf("identifier", &var_def.name.node, &var_def.name.span);

      if let Some(type_ann) = &var_def.type_annotation {
        printer.visit_type_name(type_ann);
      }

      if let Some(unit) = &var_def.unit {
        printer.print_leaf("unit", &unit.raw, &unit.location);
      }

      printer.print_label("value");
      printer.with_indent(|p| p.visit_expr(&var_def.value));
    });
  }

  fn visit_unsafe_decl(&mut self, decl: &UnsafeDeclStmt) {
    self.print_node("UnsafeDecl", &decl.location);

    self.with_indent(|printer| {
      printer.print_attached_comments(&decl.comments);
      printer.print_leaf("identifier", &decl.name.node, &decl.name.span);

      if let Some(type_ann) = &decl.type_annotation {
        printer.visit_type_name(type_ann);
      }
    });
  }

  fn visit_assign(&mut self, assign: &AssignStmt) {
    self.print_node("Assign", &assign.location);

    self.with_indent(|printer| {
      printer.print_attached_comments(&assign.comments);
      printer.print_label("target");
      printer.with_indent(|p| {
        for part in &assign.target.node {
          p.print_leaf("identifier", part, &assign.target.span);
        }
      });

      if !assign.indices.is_empty() {
        printer.print_label("indices");
        printer.with_indent(|p| {
          for idx in &assign.indices {
            p.visit_expr(idx);
          }
        });
      }

      printer.print_label("value");
      printer.with_indent(|p| p.visit_expr(&assign.value));
    });
  }

  fn visit_compound_assign(&mut self, compound: &CompoundAssignStmt) {
    self.print_node("CompoundAssign", &compound.location);

    self.with_indent(|printer| {
      printer.print_attached_comments(&compound.comments);

      printer.print_label("target");
      printer.with_indent(|p| {
        for part in &compound.target.node {
          p.print_leaf("identifier", part, &compound.target.span);
        }
      });

      if !compound.indices.is_empty() {
        printer.print_label("indices");
        printer.with_indent(|p| {
          for idx in &compound.indices {
            p.visit_expr(idx);
          }
        });
      }

      let op_str = match &compound.operator.node {
        CompoundOp::AddAssign => "+=",
        CompoundOp::SubAssign => "-=",
        CompoundOp::MulAssign => "*=",
        CompoundOp::DivAssign => "/=",
        CompoundOp::ModAssign => "%=",
      };
      printer.print_leaf("operator", op_str, &compound.operator.span);

      printer.print_label("value");
      printer.with_indent(|p| p.visit_expr(&compound.value));
    });
  }

  fn visit_value_at_assign(&mut self, value_at: &ValueAtAssignStmt) {
    self.print_node("ValueAtAssign", &value_at.location);

    self.with_indent(|printer| {
      printer.print_attached_comments(&value_at.comments);

      printer.print_label("pointer");
      printer.with_indent(|p| p.visit_expr(&value_at.pointer));

      printer.print_label("value");
      printer.with_indent(|p| p.visit_expr(&value_at.value));
    });
  }

  fn visit_fn_def(&mut self, fn_def: &FnDefStmt) {
    let mut desc = format!("FnDef({})", fn_def.name.node);
    if fn_def.is_export {
      desc = format!("export {}", desc);
    }
    self.print_node(&desc, &fn_def.location);

    self.with_indent(|printer| {
      printer.print_attached_comments(&fn_def.comments);
      if !fn_def.parameters.is_empty() {
        printer.print_label("parameters");
        printer.with_indent(|p| {
          for param in &fn_def.parameters {
            p.visit_parameter(param);
          }
        });
      }

      printer.print_node(
        &format!("returns {:?}", fn_def.return_type.kind),
        &fn_def.return_type.location,
      );

      if !fn_def.body.is_empty() {
        printer.print_label("body");
        printer.with_indent(|p| {
          for stmt in &fn_def.body {
            p.visit_stmt(stmt);
          }
        });
      }
    });
  }

  fn visit_fn_signature(&mut self, fn_signature: &FnSignatureStmt) {
    let modifier = if fn_signature.is_import {
      "import "
    } else {
      ""
    };
    let desc = format!("FnSignature({}{})", modifier, fn_signature.name.node);
    self.print_node(&desc, &fn_signature.location);

    self.with_indent(|printer| {
      printer.print_attached_comments(&fn_signature.comments);
      if !fn_signature.parameters.is_empty() {
        printer.print_label("parameters");
        printer.with_indent(|p| {
          for param in &fn_signature.parameters {
            p.visit_parameter(param);
          }
        });
      }
    });
  }

  fn visit_test_suite(&mut self, test_suite: &TestSuiteStmt) {
    let desc = format!("TestSuite({})", test_suite.name);
    self.print_node(&desc, &test_suite.location);

    self.with_indent(|printer| {
      printer.print_attached_comments(&test_suite.comments);
      for item in &test_suite.items {
        match item {
          TestSuiteItem::Case(case) => printer.visit_test_case(case),
          TestSuiteItem::Declaration(stmt) => printer.visit_stmt(stmt.as_ref()),
          TestSuiteItem::Comment(comment) => printer.visit_comment(comment),
          TestSuiteItem::Doc(doc) => printer.visit_doc(doc),
        }
      }
    });
  }

  fn visit_test_case(&mut self, test_case: &TestCaseStmt) {
    let desc = format!("TestCase({})", test_case.name);
    self.print_node(&desc, &test_case.location);

    self.with_indent(|printer| {
      printer.print_attached_comments(&test_case.comments);
      if !test_case.body.is_empty() {
        printer.print_label("body");
        printer.with_indent(|p| {
          for stmt in &test_case.body {
            p.visit_stmt(stmt);
          }
        });
      }
    });
  }

  fn visit_fn_call_stmt(&mut self, fn_call: &FnCall) {
    let target = fn_call.target.node.join(".");
    self.print_node(&format!("FnCall({})", target), &fn_call.location);

    self.with_indent(|printer| {
      printer.print_attached_comments(&fn_call.comments);
      for comment in &fn_call.inline_comments {
        match comment {
          AttachedComment::Doc(doc) => {
            printer.print_leaf("inline_doc", &doc.content, &doc.location);
          }
          AttachedComment::Comment(c) => {
            printer.print_leaf("inline_comment", &c.content, &c.location);
          }
        }
      }
      for arg in &fn_call.arguments {
        printer.visit_expr(arg);
      }
    });
  }

  fn visit_if(&mut self, if_stmt: &IfStmt) {
    self.print_node("If", &if_stmt.location);

    self.with_indent(|printer| {
      printer.print_attached_comments(&if_stmt.comments);
      printer.print_node("condition", &if_stmt.condition.location);
      printer.with_indent(|p| p.visit_condition(&if_stmt.condition));

      printer.print_label("then");
      printer.with_indent(|p| {
        for stmt in &if_stmt.then_branch {
          p.visit_stmt(stmt);
        }
      });

      for (cond, branch) in &if_stmt.else_if_branches {
        printer.print_node("else if", &cond.location);
        printer.with_indent(|p| {
          p.visit_condition(cond);
          for stmt in branch {
            p.visit_stmt(stmt);
          }
        });
      }

      if let Some(else_branch) = &if_stmt.else_branch {
        printer.print_label("else");
        printer.with_indent(|p| {
          for stmt in else_branch {
            p.visit_stmt(stmt);
          }
        });
      }
    });
  }

  fn visit_when(&mut self, when_stmt: &WhenStmt) {
    self.print_node("When", &when_stmt.location);

    self.with_indent(|printer| {
      printer.print_attached_comments(&when_stmt.comments);
      printer.print_node("condition", &when_stmt.condition.location);
      printer.with_indent(|p| p.visit_condition(&when_stmt.condition));

      printer.print_label("body");
      printer.with_indent(|p| {
        for stmt in &when_stmt.body {
          p.visit_stmt(stmt);
        }
      });

      if let Some(else_body) = &when_stmt.else_branch {
        printer.print_label("else (error — when has no else)");
        printer.with_indent(|p| {
          for stmt in else_body {
            p.visit_stmt(stmt);
          }
        });
      }
    });
  }

  fn visit_move_on(&mut self, stmt: &MoveOnStmt) {
    self.print_node("MoveOn", &stmt.location);
    self.with_indent(|printer| {
      printer.print_attached_comments(&stmt.comments);
    });
  }

  fn visit_missing_code(&mut self, stmt: &MissingCodeStmt) {
    self.print_node("MissingCode", &stmt.location);
    self.with_indent(|printer| {
      printer.print_attached_comments(&stmt.comments);
    });
  }

  fn visit_switch(&mut self, switch_stmt: &SwitchStmt) {
    self.print_node("Switch", &switch_stmt.location);

    self.with_indent(|printer| {
      printer.print_attached_comments(&switch_stmt.comments);
      printer.print_label("match");
      printer.with_indent(|p| p.visit_expr(&switch_stmt.value));

      for case in &switch_stmt.cases {
        let desc = match &case.pattern {
          SwitchPattern::Enum {
            variant_name,
            fields,
            ..
          } => {
            if fields.is_empty() {
              variant_name.node.clone()
            } else {
              let fields: Vec<String> = fields
                .iter()
                .map(|f| match f {
                  SwitchPatternField::Bind(name) => name.clone(),
                  SwitchPatternField::Discard => "_".to_string(),
                })
                .collect();
              format!("{}({})", variant_name.node, fields.join(", "))
            }
          }
          SwitchPattern::Literal { value, .. } => format!("{:?}", value),
        };
        printer.print_node(&format!("Case: {}", desc), &case.location);
        printer.with_indent(|p| {
          for stmt in &case.body {
            p.visit_stmt(stmt);
          }
        });
      }

      if let Some(default_body) = &switch_stmt.default_case {
        printer.print_label("default");
        printer.with_indent(|p| {
          for stmt in default_body {
            p.visit_stmt(stmt);
          }
        });
      }
    });
  }

  fn visit_loop(&mut self, loop_stmt: &LoopStmt) {
    self.print_node("Loop", &loop_stmt.location);

    self.with_indent(|printer| {
      printer.print_attached_comments(&loop_stmt.comments);
      if let Some(range) = &loop_stmt.range {
        printer.print_node("range", &range.location);
        printer.with_indent(|p| p.visit_range(range));
      }

      if let Some(cond) = &loop_stmt.pre_condition {
        printer.print_node("pre_condition", &cond.location);
        printer.with_indent(|p| p.visit_condition(cond));
      }

      printer.print_label("body");
      printer.with_indent(|p| {
        for stmt in &loop_stmt.body {
          p.visit_stmt(stmt);
        }
      });

      if let Some(cond) = &loop_stmt.post_condition {
        printer.print_node("post_condition", &cond.location);
        printer.with_indent(|p| p.visit_condition(cond));
      }
    });
  }

  fn visit_return(&mut self, ret: &ReturnStmt) {
    self.print_node("Return", &ret.location);

    self.with_indent(|printer| {
      printer.print_attached_comments(&ret.comments);
      if let Some(value) = &ret.value {
        printer.visit_expr(value);
      }
    });
  }

  fn visit_exit_program(&mut self, exit_program: &ExitProgramStmt) {
    let desc = match exit_program.code {
      Some(code) => format!("ExitProgram({})", code),
      None => "ExitProgram".to_string(),
    };
    self.print_node(&desc, &exit_program.location);
    self.with_indent(|printer| {
      printer.print_attached_comments(&exit_program.comments);
    });
  }

  fn visit_exit_loop(&mut self, exit_loop: &ExitLoopStmt) {
    self.print_node("ExitLoop", &exit_loop.location);
    self.with_indent(|printer| {
      printer.print_attached_comments(&exit_loop.comments);
    });
  }

  fn visit_rewind(&mut self, rewind: &RewindStmt) {
    self.print_node("Rewind", &rewind.location);
    self.with_indent(|printer| {
      printer.print_attached_comments(&rewind.comments);
    });
  }

  fn visit_stdout(&mut self, stdout: &StdoutStmt) {
    let desc = if stdout.inline {
      "WriteInline"
    } else {
      "Write"
    };
    self.print_node(desc, &stdout.location);

    self.with_indent(|printer| {
      printer.print_attached_comments(&stdout.comments);
      printer.print_label("value");
      printer.with_indent(|p| p.visit_expr(&stdout.value));
    });
  }

  fn visit_stderr(&mut self, stderr: &StderrStmt) {
    self.print_node("Warn", &stderr.location);

    self.with_indent(|printer| {
      printer.print_attached_comments(&stderr.comments);
      printer.print_label("value");
      printer.with_indent(|p| p.visit_expr(&stderr.value));
    });
  }

  fn visit_log(&mut self, log: &LogStmt) {
    self.print_node("Log", &log.location);
    self.with_indent(|printer| {
      printer.print_attached_comments(&log.comments);
      printer.print_label("value");
      printer.with_indent(|p| p.visit_expr(&log.value));
    });
  }

  fn visit_debug(&mut self, debug: &DebugStmt) {
    self.print_node("Debug", &debug.location);
    self.with_indent(|printer| {
      printer.print_attached_comments(&debug.comments);
      printer.print_leaf("expr_text", &debug.expr_text, &debug.location);
      printer.print_label("value");
      printer.with_indent(|p| p.visit_expr(&debug.value));
    });
  }

  fn visit_stdin(&mut self, stdin: &StdinStmt) {
    let target = stdin.target.node.join(".");
    self.print_node(&format!("Read({})", target), &stdin.location);
    self.with_indent(|printer| {
      printer.print_attached_comments(&stdin.comments);
    });
  }

  fn visit_ct_if(&mut self, ct_if: &CtIfStmt) {
    self.print_node("#if", &ct_if.location);

    self.with_indent(|printer| {
      printer.print_attached_comments(&ct_if.comments);
      printer.visit_condition(&ct_if.condition);
      for stmt in &ct_if.then_branch {
        printer.visit_stmt(stmt);
      }
      for (cond, branch) in &ct_if.else_if_branches {
        printer.print_node("else if", &cond.location);
        printer.with_indent(|p| {
          p.visit_condition(cond);
          for stmt in branch {
            p.visit_stmt(stmt);
          }
        });
      }
      if let Some(else_stmts) = &ct_if.else_branch {
        printer.print_node("else", &ct_if.location);
        printer.with_indent(|p| {
          for stmt in else_stmts {
            p.visit_stmt(stmt);
          }
        });
      }
    });
  }

  fn visit_ct_fail(&mut self, ct_fail: &CtFailStmt) {
    self.print_node("#fail", &ct_fail.location);
    self.with_indent(|printer| {
      printer.print_attached_comments(&ct_fail.comments);
      printer.print_leaf("message", &ct_fail.message, &ct_fail.location);
    });
  }

  fn visit_assert(&mut self, assert: &AssertStmt) {
    self.print_node("Assert", &assert.location);
    self.with_indent(|printer| {
      printer.visit_expr(&assert.condition);
    });
  }

  fn visit_ct_warn(&mut self, ct_warn: &CtWarnStmt) {
    self.print_node("#warn", &ct_warn.location);
    self.with_indent(|printer| {
      printer.print_attached_comments(&ct_warn.comments);
      printer.print_leaf("message", &ct_warn.message, &ct_warn.location);
    });
  }

  fn visit_ct_when(&mut self, ct_when: &CtWhenStmt) {
    self.print_node("#when", &ct_when.location);
    self.with_indent(|printer| {
      printer.print_attached_comments(&ct_when.comments);
      printer.print_node("condition", &ct_when.condition.location);
      printer.with_indent(|p| p.visit_condition(&ct_when.condition));
      printer.print_label("body");
      printer.with_indent(|p| {
        for stmt in &ct_when.body {
          p.visit_stmt(stmt);
        }
      });
    });
  }

  fn visit_ct_match(&mut self, ct_match: &CtMatchStmt) {
    self.print_node("#match", &ct_match.location);
    self.with_indent(|printer| {
      printer.print_attached_comments(&ct_match.comments);
      for arm in &ct_match.arms {
        printer.print_node("arm", &arm.location);
        printer.with_indent(|p| {
          p.visit_condition(&arm.guard);
          for stmt in &arm.body {
            p.visit_stmt(stmt);
          }
        });
      }
      if !ct_match.else_arm.is_empty() {
        printer.print_label("else");
        printer.with_indent(|p| {
          for stmt in &ct_match.else_arm {
            p.visit_stmt(stmt);
          }
        });
      }
    });
  }

  fn visit_ct_switch(&mut self, ct_switch: &CtSwitchStmt) {
    self.print_node("#switch", &ct_switch.location);
    self.with_indent(|printer| {
      printer.print_attached_comments(&ct_switch.comments);
      printer.print_label("match");
      printer.with_indent(|p| p.visit_expr(&ct_switch.value));
      for case in &ct_switch.cases {
        printer.print_node("case", &case.location);
        printer.with_indent(|p| {
          p.visit_expr(&case.value);
          for stmt in &case.body {
            p.visit_stmt(stmt);
          }
        });
      }
      if !ct_switch.default_case.is_empty() {
        printer.print_label("default");
        printer.with_indent(|p| {
          for stmt in &ct_switch.default_case {
            p.visit_stmt(stmt);
          }
        });
      }
    });
  }

  fn visit_doc(&mut self, doc: &DocStmt) {
    self.print_leaf("doc", &doc.content, &doc.location);
  }

  fn visit_comment(&mut self, comment: &CommentStmt) {
    self.print_leaf("comment", &comment.content, &comment.location);
  }

  fn visit_binary(&mut self, bin: &BinaryExpr) {
    let op_str = format!("{:?}", bin.operator);
    self.print_node(&format!("BinaryOp({})", op_str), &bin.location);

    self.with_indent(|printer| {
      printer.visit_expr(&bin.left);
      printer.visit_expr(&bin.right);
    });
  }

  fn visit_unary(&mut self, un: &UnaryExpr) {
    let op_str = format!("{:?}", un.operator);
    self.print_node(&format!("UnaryOp({})", op_str), &un.location);

    self.with_indent(|p| p.visit_expr(&un.operand));
  }

  fn visit_conversion(&mut self, conv: &ConversionExpr) {
    self.print_node(
      &format!("Conversion(as {:?})", conv.target_type.base_type),
      &conv.location,
    );

    self.with_indent(|p| p.visit_expr(&conv.operand));
  }

  fn visit_identifier(&mut self, id: &IdentifierExpr) {
    self.print_leaf("identifier", &id.full_path(), &id.location);
  }

  fn visit_int_literal(&mut self, lit: &IntLiteral) {
    let desc = match &lit.unit {
      Some(u) => format!("{} {}", lit.value, u.raw),
      None => lit.value.to_string(),
    };
    self.print_leaf("int_literal", &desc, &lit.location);
  }

  fn visit_uint_literal(&mut self, lit: &UintLiteral) {
    let desc = match &lit.unit {
      Some(u) => format!("{} {}", lit.value, u.raw),
      None => lit.value.to_string(),
    };
    self.print_leaf("uint_literal", &desc, &lit.location);
  }

  fn visit_float_literal(&mut self, lit: &FloatLiteral) {
    let desc = match &lit.unit {
      Some(u) => format!("{} {}", lit.value, u.raw),
      None => lit.value.to_string(),
    };
    self.print_leaf("float_literal", &desc, &lit.location);
  }

  fn visit_hex_literal(&mut self, lit: &HexLiteral) {
    self.print_leaf("hex_literal", &lit.value, &lit.location);
  }

  fn visit_char_literal(&mut self, lit: &CharLiteral) {
    self.print_leaf("char_literal", &lit.value.to_string(), &lit.location);
  }

  fn visit_bool_literal(&mut self, lit: &BoolLiteral) {
    self.print_leaf("bool_literal", &lit.value.to_string(), &lit.location);
  }

  fn visit_string_literal(&mut self, lit: &StringLiteral) {
    self.print_node("StringLiteral", &lit.location);

    self.with_indent(|printer| {
      for part in &lit.parts {
        match part {
          StringPart::Text(spanned) => {
            printer.print_leaf("text", &spanned.node, &spanned.span);
          }
          StringPart::EmbeddedValue(expr) => {
            printer.print_label("embedded_value");
            printer.with_indent(|p| p.visit_expr(expr));
          }
        }
      }
    });
  }

  fn visit_array_literal(&mut self, lit: &ArrayLiteral) {
    self.print_node("ArrayLiteral", &lit.location);

    self.with_indent(|p| {
      if let Some(value) = &lit.fill {
        p.print_node("fill", &lit.location);
        p.with_indent(|pp| pp.visit_expr(value));
      }
      for elem in &lit.elements {
        p.visit_expr(elem);
      }
    });
  }

  fn visit_fn_call_expr(&mut self, fn_call: &FnCall) {
    let target = fn_call.target.node.join(".");
    self.print_node(&format!("FnCallExpr({})", target), &fn_call.location);

    self.with_indent(|printer| {
      printer.print_attached_comments(&fn_call.comments);
      for comment in &fn_call.inline_comments {
        match comment {
          AttachedComment::Doc(doc) => {
            printer.print_leaf("inline_doc", &doc.content, &doc.location);
          }
          AttachedComment::Comment(c) => {
            printer.print_leaf("inline_comment", &c.content, &c.location);
          }
        }
      }
      for arg in &fn_call.arguments {
        printer.visit_expr(arg);
      }
    });
  }

  fn visit_member_access(&mut self, acc: &MemberAccess) {
    self.print_node(
      &format!("MemberAccess(.{})", acc.member.node),
      &acc.location,
    );

    self.with_indent(|p| p.visit_expr(&acc.object));
  }

  fn visit_array_index(&mut self, idx: &ArrayIndex) {
    self.print_node("ArrayIndex", &idx.location);

    self.with_indent(|printer| {
      printer.visit_expr(&idx.array);
      for index in &idx.indices {
        printer.visit_expr(index);
      }
    });
  }

  fn visit_compiler_const(&mut self, cc: &CompilerConst) {
    let name = match &cc.kind {
      CompilerConstKind::Main => "#main",
      CompilerConstKind::SourceFile => "#source_file",
      CompilerConstKind::SourceLine => "#source_line",
      CompilerConstKind::CompileTime => "#compile_time",
      CompilerConstKind::CompilerVersion => "#compiler_version",
      CompilerConstKind::Function => "#function_name",
      CompilerConstKind::Posix => "#posix",
      CompilerConstKind::Windows => "#windows",
      CompilerConstKind::Debug => "#debug",
      CompilerConstKind::Mode => "#mode",
    };
    self.print_node(name, &cc.location);
  }

  fn visit_nothing_expr(&mut self) {
    self.print_node("Nothing", &SourceLocation::dummy());
  }

  fn visit_has_errors(&mut self) {
    self.print_node("HasErrors", &SourceLocation::dummy());
  }

  fn visit_last_error(&mut self) {
    self.print_node("LastError", &SourceLocation::dummy());
  }

  fn visit_add_error(&mut self, _add_error: &AddErrorStmt) {
    self.print_node("AddError", &SourceLocation::dummy());
  }

  fn visit_alert_errors(&mut self, _alert_errors: &AlertErrorsStmt) {
    self.print_node("AlertErrors", &SourceLocation::dummy());
  }

  fn visit_alert_stmt(&mut self, _alert: &AlertStmt) {
    self.print_node("Alert", &SourceLocation::dummy());
  }

  fn visit_type_name(&mut self, type_name: &TypeName) {
    let mut desc = format!("{:?}", type_name.base_type);
    if !type_name.array_dimensions.is_empty() {
      // Build string representation of array dimensions from expression literals
      let dims = type_name
        .array_dimensions
        .iter()
        .map(|expr| {
          // Try to extract the literal value from the expression
          match expr {
            Expr::UintLiteral(val) => val.value.to_string(),
            Expr::IntLiteral(val) => val.value.to_string(),
            _ => "?".to_string(),
          }
        })
        .collect::<Vec<_>>()
        .join("][");
      desc = format!("{}[{}]", desc, dims);
    }
    self.print_node(&format!("type: {}", desc), &type_name.location);
  }

  fn visit_parameter(&mut self, param: &Parameter) {
    let mut desc = param.name.node.clone();
    match param.pass_mode {
      ParameterPassMode::ByValue => {}
      ParameterPassMode::ByValueExplicit => {
        desc = format!("copy {}", desc);
      }
      ParameterPassMode::ByRef => {
        desc = format!("ref {}", desc);
      }
    }
    self.print_node(&format!("param: {}", desc), &param.location);

    self.with_indent(|printer| {
      printer.visit_type_name(&param.type_annotation);
    });
  }

  fn visit_condition(&mut self, cond: &Condition) {
    if cond.negated {
      self.print_node("not", &cond.location);
      self.with_indent(|p| p.visit_expr(&cond.expr));
    } else {
      self.visit_expr(&cond.expr);
    }
  }

  fn visit_range(&mut self, range: &Range) {
    self.print_leaf("variable", &range.variable.node, &range.variable.span);
    self.visit_type_name(&range.var_type);
    self.print_label("from");
    self.with_indent(|p| p.visit_expr(&range.from));
    self.print_label("to");
    self.with_indent(|p| p.visit_expr(&range.to));
    if let Some(step) = &range.step {
      self.print_label("step");
      self.with_indent(|p| p.visit_expr(step));
    }
  }
}

/// Print an AST using the visitor pattern.
pub fn print_ast(program: &Program, file_name: &str, use_colors: bool) {
  println!("\n=== AST ({}) ===\n", file_name);
  let mut printer = AstPrinter::new(file_name, use_colors);
  printer.visit_program(program);
  println!("\n=== END AST ===\n");
}
