// =============================================================================
// Comment and doc attachment rules
// =============================================================================
// Comments are represented two ways (the "attached vs. first-class" split):
//   - Attached: `comments.leading` / `comments.trailing` on a statement, for
//     comments adjacent to that statement.
//   - First-class: `Stmt::Comment` / `Stmt::Doc`, for standalone comments.
//
// "Adjacent" is defined as:
//   - leading  = comment/doc lines directly above the statement (no blank line).
//   - trailing = a comment on the same line, after the statement.
//   - standalone (first-class) = everything else: a comment separated from the
//     next statement by a blank line, or at the end of a block/file with no
//     following statement.

use lale::ast::{AttachedComment, Stmt};
use lale::ast_builder::build_program;
use lale::{LaleParser, Rule};
use pest::Parser;

fn build(source: &str) -> lale::ast::Program {
  let pairs = LaleParser::parse(Rule::program, source).expect("parse failed");
  build_program(pairs, "test.lale").expect("build failed")
}

#[test]
fn leading_and_trailing_attach_after_statement_is_first_class() {
  let source = "// Comment 1\nvar foo as i32 = 4 // Comment 2\n// Comment 3\n";
  let program = build(source);

  assert_eq!(
    program.statements.len(),
    2,
    "statements: {:?}",
    program.statements
  );

  // First statement: VarDef with leading "Comment 1" and trailing "Comment 2".
  match &program.statements[0] {
    Stmt::VarDef(var) => {
      assert_eq!(var.comments.leading.len(), 1);
      match &var.comments.leading[0] {
        AttachedComment::Comment(c) => assert_eq!(c.content, "Comment 1"),
        other => panic!("expected a leading Comment, got {:?}", other),
      }
      match &var.comments.trailing {
        Some(AttachedComment::Comment(c)) => assert_eq!(c.content, "Comment 2"),
        other => panic!("expected a trailing Comment, got {:?}", other),
      }
    }
    other => panic!("expected VarDef, got {:?}", other),
  }

  // Second statement: standalone first-class comment "Comment 3".
  match &program.statements[1] {
    Stmt::Comment(c) => assert_eq!(c.content, "Comment 3"),
    other => panic!("expected a standalone Comment, got {:?}", other),
  }
}

#[test]
fn multiple_leading_comments_preserve_order() {
  let source = "// A\n// B\nvar x as i32 = 1\n";
  let program = build(source);

  assert_eq!(program.statements.len(), 1);
  match &program.statements[0] {
    Stmt::VarDef(var) => {
      assert_eq!(var.comments.leading.len(), 2);
      let contents: Vec<&str> = var
        .comments
        .leading
        .iter()
        .map(|c| match c {
          AttachedComment::Comment(c) => c.content.as_str(),
          other => panic!("expected Comment, got {:?}", other),
        })
        .collect();
      assert_eq!(contents, vec!["A", "B"]);
    }
    other => panic!("expected VarDef, got {:?}", other),
  }
}

#[test]
fn doc_comment_is_leading_and_distinct() {
  let source = "/// The answer.\nvar answer as i32 = 42\n";
  let program = build(source);

  assert_eq!(program.statements.len(), 1);
  match &program.statements[0] {
    Stmt::VarDef(var) => {
      assert_eq!(var.comments.leading.len(), 1);
      match &var.comments.leading[0] {
        AttachedComment::Doc(d) => assert_eq!(d.content, "The answer."),
        other => panic!("expected a leading Doc, got {:?}", other),
      }
    }
    other => panic!("expected VarDef, got {:?}", other),
  }
}

#[test]
fn comment_between_statements_is_leading_of_following() {
  let source =
    "//comment 1\nvar foo as i32 = 5 // comment 2\n//comment 3\nvar bar as i32 = 6 // comment 2\n";
  let program = build(source);

  assert_eq!(program.statements.len(), 2);

  // `foo`: leading [comment 1], trailing [comment 2].
  match &program.statements[0] {
    Stmt::VarDef(var) => {
      assert_eq!(var.comments.leading.len(), 1);
      match &var.comments.leading[0] {
        AttachedComment::Comment(c) => assert_eq!(c.content, "comment 1"),
        other => panic!("expected leading Comment, got {:?}", other),
      }
      match &var.comments.trailing {
        Some(AttachedComment::Comment(c)) => assert_eq!(c.content, "comment 2"),
        other => panic!("expected trailing Comment, got {:?}", other),
      }
    }
    other => panic!("expected VarDef, got {:?}", other),
  }

  // `bar`: leading [comment 3] (NOT trailing of `foo`), trailing [comment 2].
  match &program.statements[1] {
    Stmt::VarDef(var) => {
      assert_eq!(var.comments.leading.len(), 1);
      match &var.comments.leading[0] {
        AttachedComment::Comment(c) => assert_eq!(c.content, "comment 3"),
        other => panic!("expected leading Comment, got {:?}", other),
      }
      match &var.comments.trailing {
        Some(AttachedComment::Comment(c)) => assert_eq!(c.content, "comment 2"),
        other => panic!("expected trailing Comment, got {:?}", other),
      }
    }
    other => panic!("expected VarDef, got {:?}", other),
  }
}

#[test]
fn blank_line_separated_comments_are_standalone() {
  let source = "// Comment 1\n\n//Comment 2\nvar foo as i32 = 5 // comment 3\n\n// Comment 4\n\n// Comment 5\nvar bar as i32 = 6 // comment 6\n";
  let program = build(source);

  assert_eq!(program.statements.len(), 4);

  // [0] standalone "Comment 1"
  match &program.statements[0] {
    Stmt::Comment(c) => assert_eq!(c.content, "Comment 1"),
    other => panic!("expected standalone Comment, got {:?}", other),
  }

  // [1] `foo`: leading [Comment 2], trailing [comment 3].
  match &program.statements[1] {
    Stmt::VarDef(var) => {
      assert_eq!(var.comments.leading.len(), 1);
      match &var.comments.leading[0] {
        AttachedComment::Comment(c) => assert_eq!(c.content, "Comment 2"),
        other => panic!("expected leading Comment, got {:?}", other),
      }
      match &var.comments.trailing {
        Some(AttachedComment::Comment(c)) => assert_eq!(c.content, "comment 3"),
        other => panic!("expected trailing Comment, got {:?}", other),
      }
    }
    other => panic!("expected VarDef, got {:?}", other),
  }

  // [2] standalone "Comment 4"
  match &program.statements[2] {
    Stmt::Comment(c) => assert_eq!(c.content, "Comment 4"),
    other => panic!("expected standalone Comment, got {:?}", other),
  }

  // [3] `bar`: leading [Comment 5], trailing [comment 6].
  match &program.statements[3] {
    Stmt::VarDef(var) => {
      assert_eq!(var.comments.leading.len(), 1);
      match &var.comments.leading[0] {
        AttachedComment::Comment(c) => assert_eq!(c.content, "Comment 5"),
        other => panic!("expected leading Comment, got {:?}", other),
      }
      match &var.comments.trailing {
        Some(AttachedComment::Comment(c)) => assert_eq!(c.content, "comment 6"),
        other => panic!("expected trailing Comment, got {:?}", other),
      }
    }
    other => panic!("expected VarDef, got {:?}", other),
  }
}

#[test]
fn comment_inside_function_call_arguments_parses() {
  // A `//` comment between the arguments of a multi-line function call must
  // not break parsing, and the call must still carry both arguments.
  let source = "var energy as f64 = kinetic_energy(m, // mass\nv // velocity\n)\n";
  let program = build(source);

  assert_eq!(program.statements.len(), 1);
  match &program.statements[0] {
    Stmt::VarDef(var) => match &var.value {
      lale::Expr::FnCallExpr(call) => {
        assert_eq!(
          call.arguments.len(),
          2,
          "expected 2 arguments, got {:?}",
          call.arguments
        );
      }
      other => panic!("expected FnCallExpr, got {:?}", other),
    },
    other => panic!("expected VarDef, got {:?}", other),
  }
}
