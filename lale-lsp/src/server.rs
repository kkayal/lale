//! LSP server backend implementation.
//!
//! Implements the full LSP protocol handlers for the Lale language, including
//! diagnostics, completion, hover, go-to-definition, document symbols, and
//! semantic tokens.

use dashmap::DashMap;
use lale::{
  ast::{self, builder::build_program, token_stream::tokenize, Expr, Stmt, StringPart, TokenKind},
  semantic_analysis::{
    ModuleId, ModuleResolver, SemanticAnalyzer, SemanticError, SqliteSymbolManager,
  },
  CompilerOptions, LaleParser, Rule, SourceLocation, StdlibLevel,
};
use pest::Parser;
use std::cell::RefCell;
use std::path::PathBuf;
use std::rc::Rc;
use std::sync::Arc;
use tower_lsp::{jsonrpc::Result as LspResult, lsp_types::*, Client, LanguageServer};

// ---------------------------------------------------------------------------
// Span map — AST-based token classification for hover
// ---------------------------------------------------------------------------

/// What kind of thing is at this source location.
#[derive(Debug, Clone, PartialEq)]
enum SpanKind {
  Keyword,
  TypeName,
  Identifier,
  Number,
  String,
  Comment,
  Operator,
  Unit,
}

/// A classified span from the AST or tokenizer.
#[derive(Debug, Clone)]
struct SpanEntry {
  kind: SpanKind,
  range: Range,
  /// Human-readable label for hover, or None for comments.
  label: Option<String>,
  /// Priority: higher values win in hover when spans overlap.
  /// 0 = tokenizer base classification, 1 = AST semantic enrichment.
  priority: u8,
}

// ---------------------------------------------------------------------------
// Document cache
// ---------------------------------------------------------------------------

struct DocState {
  source: String,
  line_starts: Vec<usize>,
  /// AST-derived span map, sorted by range.start. Built during diagnostics.
  spans: Vec<SpanEntry>,
  /// Definition locations for goto-definition: name → (start_pos, end_pos).
  definitions: std::collections::HashMap<String, (usize, usize)>,
}

impl DocState {
  fn new(source: String) -> Self {
    let line_starts = compute_line_starts(&source);
    Self {
      source,
      line_starts,
      spans: Vec::new(),
      definitions: std::collections::HashMap::new(),
    }
  }

  fn position_to_offset(&self, pos: Position) -> Option<usize> {
    let line = pos.line as usize;
    if line >= self.line_starts.len() {
      return None;
    }
    let line_start = self.line_starts[line];
    let line_end = self
      .line_starts
      .get(line + 1)
      .copied()
      .unwrap_or(self.source.len());
    let line_bytes = &self.source[line_start..line_end];
    let mut utf16_offset = 0usize;
    for (byte_idx, ch) in line_bytes.char_indices() {
      if utf16_offset >= pos.character as usize {
        return Some(line_start + byte_idx);
      }
      utf16_offset += ch.len_utf16();
    }
    Some(line_start + line_bytes.len())
  }

  fn offset_to_position(&self, offset: usize) -> Position {
    let offset = offset.min(self.source.len());
    let line = self.line_starts.partition_point(|&s| s <= offset) - 1;
    let line_start = self.line_starts[line];
    let prefix = &self.source[line_start..offset];
    let character = prefix.chars().map(|c| c.len_utf16()).sum::<usize>() as u32;
    Position {
      line: line as u32,
      character,
    }
  }

  fn loc_to_range(&self, loc: &lale::SourceLocation) -> Range {
    let start = self.offset_to_position(loc.start_pos);
    let end = self.offset_to_position(loc.end_pos);
    Range { start, end }
  }
}

fn compute_line_starts(source: &str) -> Vec<usize> {
  let mut starts = vec![0usize];
  let mut offset = 0;
  for ch in source.chars() {
    offset += ch.len_utf8();
    if ch == '\n' {
      starts.push(offset);
    }
  }
  starts
}

// ---------------------------------------------------------------------------
// Backend
// ---------------------------------------------------------------------------

pub struct Backend {
  client: Client,
  docs: Arc<DashMap<Url, DocState>>,
}

impl Backend {
  pub fn new(client: Client) -> Self {
    Self {
      client,
      docs: Arc::new(DashMap::new()),
    }
  }

  fn compute_diagnostics(&self, uri: &Url) -> Vec<Diagnostic> {
    let doc = match self.docs.get(uri) {
      Some(d) => d,
      None => return vec![],
    };

    let source = &doc.source;

    // Step 1: Parse
    let pairs = match LaleParser::parse(Rule::program, source) {
      Ok(p) => p,
      Err(e) => {
        let msg = format!("Parse error: {}", e);
        return vec![Diagnostic {
          range: Range::default(),
          severity: Some(DiagnosticSeverity::ERROR),
          source: Some("lale".into()),
          message: msg,
          ..Default::default()
        }];
      }
    };

    // Step 2: Build AST
    let program = match build_program(pairs, "<lsp>") {
      Ok(p) => p,
      Err(e) => {
        return vec![Diagnostic {
          range: Range::default(),
          severity: Some(DiagnosticSeverity::ERROR),
          source: Some("lale".into()),
          message: format!("AST build error: {}", e),
          ..Default::default()
        }];
      }
    };

    // Step 2.5: Extract definition locations for goto-definition
    let mut definitions = std::collections::HashMap::new();
    extract_definitions(&program.statements, &mut definitions);

    // Step 3: Multi-file semantic analysis (matches compiler pipeline exactly)
    let (errors, warnings) = Self::run_full_analysis(uri, &program);

    // Step 4: Build span map from tokenizer (keywords, operators, literals) and AST (semantic enrichment)
    let mut spans = Vec::new();
    // Primary classification from the parser token stream
    if let Ok(tokens) = tokenize(source) {
      for tok in &tokens {
        let kind = match tok.kind {
          TokenKind::Keyword => SpanKind::Keyword,
          TokenKind::Identifier => SpanKind::Identifier,
          TokenKind::Number => SpanKind::Number,
          TokenKind::String => SpanKind::String,
          TokenKind::Comment => SpanKind::Comment,
          TokenKind::Operator => SpanKind::Operator,
          TokenKind::Unit => SpanKind::Unit,
          TokenKind::BoolLiteral => SpanKind::Keyword,
          TokenKind::CompilerConst => SpanKind::Keyword,
          TokenKind::Nothing => SpanKind::Keyword,
          TokenKind::Punctuation => continue, // Punctuation doesn't need hover labels
        };
        let label = match tok.kind {
          TokenKind::Keyword => Some(format!("**`{}`** — Keyword", tok.text)),
          TokenKind::Identifier => Some(format!("**`{}`** — Identifier", tok.text)),
          TokenKind::Number => Some(format!("**`{}`** — Number literal", tok.text)),
          TokenKind::String => Some("String literal".into()),
          TokenKind::Operator => Some(format!("**`{}`** — Operator", tok.text)),
          TokenKind::Unit => Some("Unit expression".into()),
          TokenKind::BoolLiteral => Some(format!("**`{}`** — Boolean literal", tok.text)),
          TokenKind::CompilerConst => Some(format!("**`{}`** — Compiler constant", tok.text)),
          TokenKind::Nothing => Some("**`nothing`** — Nothing literal".into()),
          _ => None,
        };
        spans.push(SpanEntry {
          kind,
          range: Range {
            start: doc.offset_to_position(tok.start_pos),
            end: doc.offset_to_position(tok.end_pos),
          },
          label,
          priority: 0, // base tokenizer classification
        });
      }
    }
    // Secondary: AST walk for semantic enrichment (Function/Parameter/Variable labels, TypeName spans)
    collect_ast_spans(&program, &doc, &mut spans);
    spans.sort_by_key(|s| s.range.start);
    drop(doc); // release immutable borrow before mutable access below

    // Step 5: Store spans and definitions in doc state
    if let Some(mut doc) = self.docs.get_mut(uri) {
      doc.spans = spans;
      doc.definitions = definitions;
    }

    let doc = match self.docs.get(uri) {
      Some(d) => d,
      None => return vec![],
    };

    let mut diags: Vec<Diagnostic> = errors
      .iter()
      .map(|e| semantic_error_to_diagnostic(e, &doc, DiagnosticSeverity::ERROR))
      .chain(
        warnings
          .iter()
          .map(|w| semantic_error_to_diagnostic(w, &doc, DiagnosticSeverity::WARNING)),
      )
      .collect();

    if errors.is_empty() {
      diags.push(Diagnostic {
        range: Range::default(),
        severity: Some(DiagnosticSeverity::INFORMATION),
        source: Some("lale".into()),
        message: "\u{2713} Compilation successful".into(),
        ..Default::default()
      });
    }

    diags
  }

  /// Run the exact same multi-file compilation pipeline as the compiler.
  /// This ensures `use` statement resolution and type checking match 1:1.
  fn run_full_analysis(
    uri: &Url,
    program: &ast::Program,
  ) -> (Vec<SemanticError>, Vec<SemanticError>) {
    let source_path = uri
      .to_file_path()
      .unwrap_or_else(|_| PathBuf::from("<lsp>"));
    let base_dir = source_path
      .parent()
      .unwrap_or_else(|| std::path::Path::new("."))
      .to_path_buf();

    // ---- Step 1: Module resolver (same as compiler) ----
    let resolver = Rc::new(RefCell::new(ModuleResolver::new(base_dir.clone())));

    // ---- Step 2: Build module graph from root ----
    {
      let mut resolver_mut = resolver.borrow_mut();
      if let Err(_e) = resolver_mut.build_graph_from_root(&source_path, program.clone()) {
        return (
          vec![SemanticError {
            message: format!("Module resolution failed: {}", _e),
            location: SourceLocation::dummy(),
            hint: None,
          }],
          vec![],
        );
      }
    }

    // ---- Step 3: Check for cycles ----
    {
      let resolver_ref = resolver.borrow();
      if let Some(cycle) = resolver_ref.graph.detect_cycle() {
        let cycle_str: Vec<_> = cycle.iter().map(|m| m.to_string()).collect();
        return (
          vec![SemanticError {
            message: format!("Circular import: {}", cycle_str.join(".")),
            location: SourceLocation::dummy(),
            hint: None,
          }],
          vec![],
        );
      }
    }

    // ---- Step 4: Compilation order (dependencies first) ----
    let compilation_order: Vec<ModuleId> = {
      let resolver_ref = resolver.borrow();
      resolver_ref.compilation_order().unwrap_or_default()
    };

    // ---- Step 5: Shared symbol manager (same as compiler) ----
    let mut shared_manager = SqliteSymbolManager::new();

    // Register text as a pre-defined type
    shared_manager.define_type_with_fields(
      "text",
      SourceLocation::dummy(),
      vec![
        ("ptr".to_string(), "pointer".to_string(), None, false),
        ("bytes".to_string(), "u64".to_string(), None, false),
        ("chars".to_string(), "u64".to_string(), None, false),
      ],
    );

    let options = CompilerOptions::default()
      .stdlib(StdlibLevel::Full)
      .debug(false);

    // ---- Step 6: Analyze builtins FIRST (same as compiler) ----
    // Skip builtins validation pass when analyzing builtins.lale itself — it will
    // be analyzed as the main module in step 7. Running the validation pass first
    // would deposit variables into the shared manager, causing spurious
    // "already defined" errors during the main analysis pass.
    let is_builtins_file = source_path.ends_with("builtins.lale");
    if !is_builtins_file {
      let builtins_source = lale::builtins::BUILTINS_SOURCE;
      if let Ok(builtins_pairs) = LaleParser::parse(Rule::program, builtins_source) {
        if let Ok(builtins_program) = build_program(builtins_pairs, "builtins.lale") {
          let mut builtins_analyzer =
            SemanticAnalyzer::with_mut_manager(&mut shared_manager, options.clone());
          builtins_analyzer.set_resolver(resolver.clone());
          builtins_analyzer.set_root_file_path(source_path.clone());
          builtins_analyzer.set_current_module_path(PathBuf::from("builtins.lale"));
          lale::ast::AstVisitor::visit_program(&mut builtins_analyzer, &builtins_program);
          // Builtins errors are not user-facing; silently ignore
        }
      }
    }

    // ---- Step 7: Analyze each module in order, populating exports ----
    let mut main_errors = Vec::new();
    let mut main_warnings = Vec::new();

    for module_id in &compilation_order {
      let module_program = {
        let resolver_ref = resolver.borrow();
        resolver_ref.graph.get(module_id).map(|m| m.program.clone())
      };

      let Some(module_program) = module_program else {
        continue;
      };

      let is_main = module_id.path() == source_path;

      let mut analyzer = SemanticAnalyzer::with_mut_manager(&mut shared_manager, options.clone());
      analyzer.set_resolver(resolver.clone());
      analyzer.set_root_file_path(source_path.clone());
      analyzer.set_current_module_path(module_id.path().to_path_buf());

      lale::ast::AstVisitor::visit_program(&mut analyzer, &module_program);

      if is_main {
        main_errors = analyzer.get_errors().to_vec();
        main_warnings = analyzer.get_warnings().to_vec();
      }
    }

    (main_errors, main_warnings)
  }
}

fn semantic_error_to_diagnostic(
  e: &SemanticError,
  doc: &DocState,
  severity: DiagnosticSeverity,
) -> Diagnostic {
  let range = doc.loc_to_range(&e.location);
  let message = if let Some(ref hint) = e.hint {
    format!("{} (hint: {})", e.message, hint)
  } else {
    e.message.clone()
  };
  Diagnostic {
    range,
    severity: Some(severity),
    source: Some("lale".into()),
    message,
    ..Default::default()
  }
}

// ---------------------------------------------------------------------------
// AST span collection — semantic enrichment only (keywords come from tokenizer)
// ---------------------------------------------------------------------------

/// Add semantic labels from AST nodes. The tokenizer already provides
/// base classification (Keyword/Identifier/Number/etc.), so this function
/// only enriches identifiers with semantic labels: Function, Variable,
/// Parameter, Type, and adds TypeName spans for type annotations.
fn collect_ast_spans(program: &ast::Program, doc: &DocState, out: &mut Vec<SpanEntry>) {
  for stmt in &program.statements {
    collect_stmt(stmt, doc, out);
  }
}

fn collect_stmt(stmt: &Stmt, doc: &DocState, out: &mut Vec<SpanEntry>) {
  match stmt {
    Stmt::VarDef(s) => {
      add_label(
        SpanKind::Identifier,
        &s.name.span,
        format!("**`{}`** — Variable", s.name.node),
        doc,
        out,
      );
      if let Some(ref t) = s.type_annotation {
        collect_type(t, doc, out);
      }
      if let Some(ref u) = s.unit {
        add_label(
          SpanKind::Unit,
          &u.location,
          "Unit expression".into(),
          doc,
          out,
        );
      }
      collect_expr(&s.value, doc, out);
    }
    Stmt::FnDef(s) => {
      add_label(
        SpanKind::Identifier,
        &s.name.span,
        format!("**`{}`** — Function", s.name.node),
        doc,
        out,
      );
      for param in &s.parameters {
        add_label(
          SpanKind::Identifier,
          &param.name.span,
          format!("**`{}`** — Parameter", param.name.node),
          doc,
          out,
        );
        collect_type(&param.type_annotation, doc, out);
        if let Some(ref u) = param.unit {
          add_label(
            SpanKind::Unit,
            &u.location,
            "Unit expression".into(),
            doc,
            out,
          );
        }
      }
      if let ast::ReturnTypeKind::Type(t) = &s.return_type.kind {
        collect_type(t, doc, out)
      }
      if let Some(ref u) = s.return_unit {
        add_label(
          SpanKind::Unit,
          &u.location,
          "Unit expression".into(),
          doc,
          out,
        );
      }
      for body_stmt in &s.body {
        collect_stmt(body_stmt, doc, out);
      }
    }
    Stmt::TypeDef(s) => {
      add_label(
        SpanKind::Identifier,
        &s.name.span,
        format!("**`{}`** — Type definition", s.name.node),
        doc,
        out,
      );
      for field in &s.fields {
        add_label(
          SpanKind::Identifier,
          &field.name.span,
          format!("**`{}`** — Field", field.name.node),
          doc,
          out,
        );
        collect_type(&field.field_type, doc, out);
      }
    }
    Stmt::FnSignature(s) => {
      add_label(
        SpanKind::Identifier,
        &s.name.span,
        format!("**`{}`** — Function signature", s.name.node),
        doc,
        out,
      );
    }
    Stmt::Assign(s) => {
      add_label(
        SpanKind::Identifier,
        &s.target.span,
        format!("**`{}`** — Variable", s.target.node.join(".")),
        doc,
        out,
      );
      collect_expr(&s.value, doc, out);
    }
    Stmt::CompoundAssign(s) => {
      add_label(
        SpanKind::Identifier,
        &s.target.span,
        format!("**`{}`** — Variable", s.target.node.join(".")),
        doc,
        out,
      );
      collect_expr(&s.value, doc, out);
    }
    Stmt::FnCall(s) => {
      add_label(
        SpanKind::Identifier,
        &s.target.span,
        format!("**`{}()`** — Function call", s.target.node.join(".")),
        doc,
        out,
      );
      for arg in &s.arguments {
        collect_expr(arg, doc, out);
      }
      if let Some(ref u) = s.unit {
        add_label(
          SpanKind::Unit,
          &u.location,
          "Unit expression".into(),
          doc,
          out,
        );
      }
    }
    Stmt::If(s) => {
      collect_expr(&s.condition.expr, doc, out);
      for stmt in &s.then_branch {
        collect_stmt(stmt, doc, out);
      }
      for (cond, body) in &s.else_if_branches {
        collect_expr(&cond.expr, doc, out);
        for stmt in body {
          collect_stmt(stmt, doc, out);
        }
      }
      if let Some(ref else_body) = s.else_branch {
        for stmt in else_body {
          collect_stmt(stmt, doc, out);
        }
      }
    }
    Stmt::Loop(s) => {
      if let Some(ref r) = s.range {
        add_label(
          SpanKind::Identifier,
          &r.variable.span,
          format!("**`{}`** — Loop variable", r.variable.node),
          doc,
          out,
        );
        collect_type(&r.var_type, doc, out);
        collect_expr(&r.from, doc, out);
        collect_expr(&r.to, doc, out);
      }
      for stmt in &s.body {
        collect_stmt(stmt, doc, out);
      }
    }
    Stmt::Return(s) => {
      if let Some(ref e) = s.value {
        collect_expr(e, doc, out);
      }
    }
    Stmt::Stdout(s) => {
      collect_expr(&s.value, doc, out);
    }
    Stmt::Stderr(s) => {
      collect_expr(&s.value, doc, out);
    }
    Stmt::Debug(s) => {
      collect_expr(&s.value, doc, out);
    }
    Stmt::Alert(s) => {
      collect_expr(&s.value, doc, out);
    }
    Stmt::UnsafeDecl(s) => {
      add_label(
        SpanKind::Identifier,
        &s.name.span,
        format!("**`{}`** — Unsafe declaration", s.name.node),
        doc,
        out,
      );
    }
    Stmt::Use(s) => {
      let origin_str = match s.origin.node {
        ast::ModuleOrigin::Std => "std",
        ast::ModuleOrigin::Local => "local",
      };
      add_label(
        SpanKind::Keyword,
        &s.origin.span,
        format!("**`{origin_str}`** — Dependency origin"),
        doc,
        out,
      );
      for seg in &s.path {
        add_label(
          SpanKind::Identifier,
          &seg.span,
          format!("**`{}`** — Module path", seg.node),
          doc,
          out,
        );
      }
    }
    Stmt::ValueAtAssign(s) => {
      collect_expr(&s.pointer, doc, out);
      collect_expr(&s.value, doc, out);
    }
    Stmt::EnumDef(s) => {
      add_label(
        SpanKind::Identifier,
        &s.name.span,
        format!("**`{}`** — Enum definition", s.name.node),
        doc,
        out,
      );
      for variant in &s.variants {
        add_label(
          SpanKind::Identifier,
          &variant.name.span,
          format!("**`{}`** — Enum variant", variant.name.node),
          doc,
          out,
        );
      }
    }
    Stmt::Assert(s) => {
      collect_expr(&s.condition, doc, out);
    }
    Stmt::Switch(s) => {
      collect_expr(&s.value, doc, out);
      for case in &s.cases {
        match &case.pattern {
          ast::SwitchPattern::Enum {
            variant_name,
            fields,
            ..
          } => {
            add_label(
              SpanKind::Identifier,
              &variant_name.span,
              format!("**`{}`** — Enum variant", variant_name.node),
              doc,
              out,
            );
            for field in fields {
              if let ast::SwitchPatternField::Bind(name) = field {
                // Bindings are local variables within the case body.
                // We add a label at the case location since we don't have
                // individual field spans.
                add_label(
                  SpanKind::Identifier,
                  &case.location,
                  format!("**`{}`** — Pattern binding", name),
                  doc,
                  out,
                );
              }
            }
          }
          ast::SwitchPattern::Literal { value, .. } => {
            collect_expr(value, doc, out);
          }
        }
        for body_stmt in &case.body {
          collect_stmt(body_stmt, doc, out);
        }
      }
      if let Some(ref default_body) = s.default_case {
        for stmt in default_body {
          collect_stmt(stmt, doc, out);
        }
      }
    }
    _ => {}
  }
}

fn collect_expr(expr: &Expr, doc: &DocState, out: &mut Vec<SpanEntry>) {
  match expr {
    Expr::FnCallExpr(call) => {
      add_label(
        SpanKind::Identifier,
        &call.target.span,
        format!("**`{}()`** — Function call", call.target.node.join(".")),
        doc,
        out,
      );
      for arg in &call.arguments {
        collect_expr(arg, doc, out);
      }
      if let Some(ref u) = call.unit {
        add_label(
          SpanKind::Unit,
          &u.location,
          "Unit expression".into(),
          doc,
          out,
        );
      }
    }
    Expr::StringLiteral(lit) => {
      for part in &lit.parts {
        if let StringPart::EmbeddedValue(inner_expr) = part {
          collect_expr(inner_expr, doc, out);
        }
      }
    }
    Expr::Binary(bin) => {
      collect_expr(&bin.left, doc, out);
      collect_expr(&bin.right, doc, out);
    }
    Expr::Unary(un) => {
      collect_expr(&un.operand, doc, out);
    }
    Expr::Conversion(conv) => {
      collect_expr(&conv.operand, doc, out);
      collect_type(&conv.target_type, doc, out);
    }
    Expr::MemberAccess(m) => {
      add_label(
        SpanKind::Identifier,
        &m.member.span,
        format!("**`{}`** — Field access", m.member.node),
        doc,
        out,
      );
    }
    Expr::ArrayLiteral(arr) => {
      for elem in &arr.elements {
        collect_expr(elem, doc, out);
      }
    }
    Expr::ArrayIndex(idx) => {
      for i in &idx.indices {
        collect_expr(i, doc, out);
      }
    }
    Expr::Grouped(inner) => collect_expr(inner, doc, out),
    Expr::HasValue(inner) | Expr::HasNoValue(inner) | Expr::TryPropagate(inner) => {
      collect_expr(inner, doc, out);
    }
    _ => {}
  }
}

fn collect_type(ty: &ast::TypeName, doc: &DocState, out: &mut Vec<SpanEntry>) {
  let type_str = format!("{:?}", ty.base_type).to_lowercase();
  add_label(
    SpanKind::TypeName,
    &ty.location,
    format!("**`{}`** — Type", type_str),
    doc,
    out,
  );
  for dim in &ty.array_dimensions {
    collect_expr(dim, doc, out);
  }
}

fn add_label(
  kind: SpanKind,
  loc: &lale::SourceLocation,
  label: String,
  doc: &DocState,
  out: &mut Vec<SpanEntry>,
) {
  let range = doc.loc_to_range(loc);
  out.push(SpanEntry {
    kind,
    range,
    label: Some(label),
    priority: 1, // AST semantic enrichment overrides tokenizer
  });
}

// ---------------------------------------------------------------------------
// LSP trait implementation
// ---------------------------------------------------------------------------

#[tower_lsp::async_trait]
impl LanguageServer for Backend {
  async fn initialize(&self, _params: InitializeParams) -> LspResult<InitializeResult> {
    Ok(InitializeResult {
      capabilities: ServerCapabilities {
        text_document_sync: Some(TextDocumentSyncCapability::Kind(TextDocumentSyncKind::FULL)),
        completion_provider: Some(CompletionOptions {
          trigger_characters: Some(vec![".".into(), " ".into()]),
          ..Default::default()
        }),
        hover_provider: Some(HoverProviderCapability::Simple(true)),
        definition_provider: Some(OneOf::Left(true)),
        document_symbol_provider: Some(OneOf::Left(true)),
        semantic_tokens_provider: Some(
          SemanticTokensServerCapabilities::SemanticTokensRegistrationOptions(
            SemanticTokensRegistrationOptions {
              text_document_registration_options: TextDocumentRegistrationOptions {
                document_selector: Some(vec![DocumentFilter {
                  language: Some("lale".into()),
                  scheme: None,
                  pattern: None,
                }]),
              },
              semantic_tokens_options: SemanticTokensOptions {
                legend: SemanticTokensLegend {
                  token_types: vec![
                    SemanticTokenType::KEYWORD,
                    SemanticTokenType::TYPE,
                    SemanticTokenType::FUNCTION,
                    SemanticTokenType::VARIABLE,
                    SemanticTokenType::STRING,
                    SemanticTokenType::NUMBER,
                    SemanticTokenType::COMMENT,
                    SemanticTokenType::OPERATOR,
                    SemanticTokenType::PARAMETER,
                  ],
                  token_modifiers: vec![
                    SemanticTokenModifier::DECLARATION,
                    SemanticTokenModifier::DEFINITION,
                    SemanticTokenModifier::READONLY,
                  ],
                },
                range: Some(false),
                full: Some(SemanticTokensFullOptions::Bool(true)),
                ..Default::default()
              },
              static_registration_options: StaticRegistrationOptions::default(),
            },
          ),
        ),
        ..Default::default()
      },
      server_info: Some(ServerInfo {
        name: "lale-lsp".into(),
        version: Some("0.1.0".into()),
      }),
    })
  }

  async fn initialized(&self, _: InitializedParams) {
    self
      .client
      .log_message(MessageType::INFO, "Lale Language Server initialized")
      .await;
  }

  async fn shutdown(&self) -> LspResult<()> {
    Ok(())
  }

  // ---- Document sync ---------------------------------------------------

  async fn did_open(&self, params: DidOpenTextDocumentParams) {
    let uri = params.text_document.uri;
    self
      .docs
      .insert(uri.clone(), DocState::new(params.text_document.text));
    self.publish_diagnostics(&uri).await;
  }

  async fn did_change(&self, params: DidChangeTextDocumentParams) {
    let uri = params.text_document.uri;
    if let Some(change) = params.content_changes.into_iter().last() {
      self.docs.insert(uri.clone(), DocState::new(change.text));
    }
    self.publish_diagnostics(&uri).await;
  }

  async fn did_close(&self, params: DidCloseTextDocumentParams) {
    self.docs.remove(&params.text_document.uri);
  }

  // ---- Completion ------------------------------------------------------

  async fn completion(&self, params: CompletionParams) -> LspResult<Option<CompletionResponse>> {
    let uri = &params.text_document_position.text_document.uri;
    let pos = params.text_document_position.position;
    let doc = match self.docs.get(uri) {
      Some(d) => d,
      None => return Ok(None),
    };
    let _offset = doc.position_to_offset(pos).unwrap_or(0);
    let completions = compute_completions();
    Ok(Some(CompletionResponse::Array(completions)))
  }

  // ---- Hover -----------------------------------------------------------

  async fn hover(&self, params: HoverParams) -> LspResult<Option<Hover>> {
    let uri = &params.text_document_position_params.text_document.uri;
    let pos = params.text_document_position_params.position;
    let doc = match self.docs.get(uri) {
      Some(d) => d,
      None => return Ok(None),
    };
    let offset = doc.position_to_offset(pos).unwrap_or(0);
    let spans = &doc.spans;
    if spans.is_empty() {
      return Ok(None);
    }

    // Find all spans containing this offset, pick the smallest (most specific)
    let best = spans
      .iter()
      .filter(|entry| {
        let s = doc
          .position_to_offset(entry.range.start)
          .unwrap_or(usize::MAX);
        let e = doc.position_to_offset(entry.range.end).unwrap_or(0);
        offset >= s && offset <= e
      })
      .min_by(|a, b| {
        let sa = doc.position_to_offset(a.range.start).unwrap_or(0);
        let ea = doc.position_to_offset(a.range.end).unwrap_or(usize::MAX);
        let sb = doc.position_to_offset(b.range.start).unwrap_or(0);
        let eb = doc.position_to_offset(b.range.end).unwrap_or(usize::MAX);
        let size_a = ea.saturating_sub(sa);
        let size_b = eb.saturating_sub(sb);
        // Smaller span wins; if equal, higher priority (AST > tokenizer) wins
        size_a
          .cmp(&size_b)
          .then_with(|| b.priority.cmp(&a.priority))
      });

    match best {
      Some(entry) if entry.kind == SpanKind::Comment => Ok(None),
      Some(entry) => {
        let text = entry.label.clone().unwrap_or_else(|| "?".into());
        Ok(Some(Hover {
          contents: HoverContents::Markup(MarkupContent {
            kind: MarkupKind::Markdown,
            value: text,
          }),
          range: Some(entry.range),
        }))
      }
      None => Ok(None),
    }
  }

  // ---- Semantic tokens --------------------------------------------------

  async fn semantic_tokens_full(
    &self,
    params: SemanticTokensParams,
  ) -> LspResult<Option<SemanticTokensResult>> {
    let uri = &params.text_document.uri;
    let doc = match self.docs.get(uri) {
      Some(d) => d,
      None => return Ok(None),
    };
    let spans = &doc.spans;
    if spans.is_empty() {
      return Ok(None);
    }

    // Map SpanKind → token type index (must match legend order)
    let token_type = |k: &SpanKind| -> u32 {
      match k {
        SpanKind::Keyword => 0,
        SpanKind::TypeName => 1,
        SpanKind::Identifier => 3, // VARIABLE by default (function detection via label)
        SpanKind::Number => 5,
        SpanKind::String => 4,
        SpanKind::Comment => 6,
        SpanKind::Operator => 7,
        SpanKind::Unit => 4, // STRING (unit is a specialized string)
      }
    };

    let mut data: Vec<SemanticToken> = Vec::new();
    let mut prev_line = 0u32;
    let mut prev_start = 0u32;

    for entry in spans.iter().filter(|e| e.kind != SpanKind::Comment) {
      let line = entry.range.start.line;
      let start = entry.range.start.character;
      let len = entry.range.end.character.saturating_sub(start);

      let modifiers = if entry
        .label
        .as_ref()
        .is_some_and(|l| l.contains("Function") || l.contains("Parameter"))
      {
        1 << 1 // DEFINITION modifier
      } else {
        0
      };

      let delta_line = line.saturating_sub(prev_line);
      let delta_start = if delta_line == 0 {
        start.saturating_sub(prev_start)
      } else {
        start
      };

      data.push(SemanticToken {
        delta_line,
        delta_start,
        length: len.max(1),
        token_type: token_type(&entry.kind),
        token_modifiers_bitset: modifiers,
      });

      prev_line = line;
      prev_start = start;
    }

    Ok(Some(SemanticTokensResult::Tokens(SemanticTokens {
      result_id: None,
      data,
    })))
  }

  // ---- Go to definition -----------------------------------------------

  async fn goto_definition(
    &self,
    params: GotoDefinitionParams,
  ) -> LspResult<Option<GotoDefinitionResponse>> {
    let uri = &params.text_document_position_params.text_document.uri;
    let pos = params.text_document_position_params.position;
    let doc = match self.docs.get(uri) {
      Some(d) => d,
      None => return Ok(None),
    };

    // Find the identifier token at the cursor position
    let offset = doc.position_to_offset(pos).unwrap_or(0);
    let spans = &doc.spans;
    let best = spans
      .iter()
      .filter(|entry| {
        entry.kind == SpanKind::Identifier && {
          let s = doc
            .position_to_offset(entry.range.start)
            .unwrap_or(usize::MAX);
          let e = doc.position_to_offset(entry.range.end).unwrap_or(0);
          offset >= s && offset <= e
        }
      })
      .min_by(|a, b| {
        let sa = doc.position_to_offset(a.range.start).unwrap_or(0);
        let ea = doc.position_to_offset(a.range.end).unwrap_or(usize::MAX);
        let sb = doc.position_to_offset(b.range.start).unwrap_or(0);
        let eb = doc.position_to_offset(b.range.end).unwrap_or(usize::MAX);
        (ea - sa).cmp(&(eb - sb))
      });

    // Get the identifier text to look up its definition
    let name = match best {
      Some(entry) => {
        let s = doc.position_to_offset(entry.range.start).unwrap_or(0);
        let e = doc.position_to_offset(entry.range.end).unwrap_or(0);
        if e > s && e <= doc.source.len() {
          doc.source[s..e].to_string()
        } else {
          return Ok(None);
        }
      }
      None => return Ok(None),
    };

    // Look up the definition location
    if let Some(&(start_pos, end_pos)) = doc.definitions.get(&name) {
      let target_range = Range {
        start: doc.offset_to_position(start_pos),
        end: doc.offset_to_position(end_pos),
      };
      Ok(Some(GotoDefinitionResponse::Scalar(Location::new(
        uri.clone(),
        target_range,
      ))))
    } else {
      Ok(None)
    }
  }

  // ---- Document symbols ------------------------------------------------

  async fn document_symbol(
    &self,
    params: DocumentSymbolParams,
  ) -> LspResult<Option<DocumentSymbolResponse>> {
    let uri = &params.text_document.uri;
    let doc = match self.docs.get(uri) {
      Some(d) => d,
      None => return Ok(None),
    };
    let symbols = extract_document_symbols(&doc.source);
    Ok(Some(DocumentSymbolResponse::Nested(symbols)))
  }
}

// ---------------------------------------------------------------------------
// Helpers — Completion
// ---------------------------------------------------------------------------

fn compute_completions() -> Vec<CompletionItem> {
  let mut items = Vec::new();
  let keywords: &[(&str, &str)] = &[
    // ---- Statements / definitions ----
    ("var", "Variable definition: `var name as type = value`"),
    (
      "fn",
      "Function definition: `fn name(params) returns type ... end fn`",
    ),
    (
      "fn signature",
      "Function signature: `fn signature name(params) returns type`",
    ),
    ("type", "Type (struct) definition: `type Name ... end type`"),
    ("use", "Module import: `use <symbols> from <origin>.<path>`"),
    (
      "all",
      "Import all exported symbols: `use all from <origin>.<path>`",
    ),
    (
      "std",
      "Standard-library origin: `use ... from std.<module>`",
    ),
    (
      "local",
      "Source-relative origin: `use ... from local.<module>`",
    ),
    ("unsafe", "Unsafe declaration or cast"),
    (
      "private",
      "Private field modifier: `private field_name as type`",
    ),
    ("switch", "Switch over value: `switch expr ... end switch`"),
    (
      "match",
      "Exhaustive pattern match over enum: `match expr ... end match`",
    ),
    (
      "when",
      "Conditional guard block: `when condition ... end when`",
    ),
    ("default", "Default case in `switch` or `match`"),
    ("case", "Pattern match arm: `case Variant(fields): body`"),
    ("if", "Conditional: `if condition ... end if`"),
    (
      "else",
      "Else branch in `if`, `switch`, or `match` statements",
    ),
    ("loop", "Loop: `loop ... end loop`"),
    ("return", "Return from function"),
    ("returns", "Return type specifier in function signature"),
    ("as", "Type cast operator: `expr as type`"),
    ("debug", "Debug print expression"),
    ("assert", "Runtime assertion: `assert condition`"),
    (
      "test suite",
      "Test suite definition: `test suite name ... end test suite`",
    ),
    (
      "test case",
      "Test case definition: `test case name ... end test case`",
    ),
    ("enum", "Enum definition: `enum Name ... end enum`"),
    ("rewind", "Rewind loop iteration"),
    ("export", "Export symbol from module"),
    ("import", "Import symbol into module"),
    ("decl", "Forward declaration"),
    ("move on", "No-op statement"),
    ("missing code", "Stub placeholder for unimplemented code"),
    ("exit program", "Exit with optional exit code"),
    ("copy", "Pass parameter by copy (not reference)"),
    // ---- Error handling ----
    ("add error", "Add an error message"),
    (
      "errors has messages",
      "Check if any errors have been raised",
    ),
    ("last error", "Retrieve the most recent error"),
    // ---- Unary / binary operators ----
    ("pointer to", "Create pointer to variable"),
    ("value of", "Extract value from optional type"),
    ("value at", "Dereference pointer"),
    ("has value", "Check if optional has a value"),
    ("has no value", "Check if optional has no value"),
    ("unsafe bitcast", "Unsafe type cast"),
    ("invert", "Invert boolean value"),
    ("not", "Logical NOT"),
    ("and", "Logical AND"),
    ("or", "Logical OR"),
    ("xor", "Logical XOR"),
    ("bitwise and", "Bitwise AND"),
    ("bitwise or", "Bitwise OR"),
    ("bitwise xor", "Bitwise XOR"),
    ("dot", "Dot product (multiplication)"),
    ("cross", "Cross product (multiplication)"),
    (
      "unsigned left shift",
      "Unsigned left shift: `expr unsigned left shift n`",
    ),
    (
      "unsigned right shift",
      "Unsigned right shift: `expr unsigned right shift n`",
    ),
    (
      "signed left shift",
      "Signed left shift: `expr signed left shift n`",
    ),
    (
      "signed right shift",
      "Signed right shift: `expr signed right shift n`",
    ),
    // ---- Equality / comparison operators ----
    ("==", "Equality operator: `a == b`"),
    ("≠", "Not-equal operator: `a ≠ b`"),
    ("≥", "Greater-than-or-equal operator: `a ≥ b`"),
    ("≤", "Less-than-or-equal operator: `a ≤ b`"),
    // ---- Loop range keywords ----
    ("from", "Loop range: `from 0 to 10`"),
    ("step", "Loop range: `step 2`"),
    // ---- I/O ----
    ("write", "Write to stdout"),
    ("warn", "Write to stderr"),
    ("alert", "Show alert dialog"),
    ("log", "Write a log entry to stderr"),
    ("write inline", "Write to stdout (inline expression)"),
    ("read", "Read from stdin into variable"),
    // ---- Literals ----
    ("nothing", "Nothing literal (void/no value)"),
    ("true", "Boolean true"),
    ("false", "Boolean false"),
    // ---- Compiler directives ----
    ("#main", "Main entry point directive"),
    ("#fail", "Fail compilation with message"),
    ("#warn", "Emit compile-time warning"),
    // ---- End keywords ----
    ("end if", "End of if block"),
    ("end loop", "End of loop block"),
    ("end fn", "End of function block"),
    ("end type", "End of type block"),
    ("end enum", "End of enum block"),
    ("end switch", "End of switch block"),
    ("end match", "End of match block"),
    ("end when", "End of when block"),
    ("end test suite", "End of test suite block"),
    ("end test case", "End of test case block"),
  ];
  for (kw, doc) in keywords {
    items.push(CompletionItem {
      label: kw.to_string(),
      kind: Some(CompletionItemKind::KEYWORD),
      detail: Some("Keyword".into()),
      documentation: Some(Documentation::MarkupContent(MarkupContent {
        kind: MarkupKind::Markdown,
        value: doc.to_string(),
      })),
      ..Default::default()
    });
  }
  let types = [
    "u8", "i8", "u16", "i16", "u32", "i32", "u64", "i64", "f16", "f32", "f64", "text", "bool",
    "byte", "char", "pointer",
  ];
  for t in &types {
    items.push(CompletionItem {
      label: t.to_string(),
      kind: Some(CompletionItemKind::TYPE_PARAMETER),
      detail: Some("Primitive type".into()),
      ..Default::default()
    });
  }
  for v in &["vec2", "vec3", "vec4"] {
    items.push(CompletionItem {
      label: v.to_string(),
      kind: Some(CompletionItemKind::TYPE_PARAMETER),
      detail: Some("Vector type".into()),
      ..Default::default()
    });
  }
  let comp_consts: &[(&str, &str)] = &[
    ("#compiler_version", "Compiler version string"),
    ("#source_file", "Current source file path"),
    ("#source_line", "Current source line number"),
    ("#compile_time", "Compilation timestamp"),
    ("#function_name", "Current function name"),
    ("#posix", "True when compiling for POSIX"),
    ("#windows", "True when compiling for Windows"),
    ("#debug", "True in debug builds"),
    ("#mode", "Execution mode: run or test"),
  ];
  for (c, doc) in comp_consts {
    items.push(CompletionItem {
      label: c.to_string(),
      kind: Some(CompletionItemKind::CONSTANT),
      detail: Some("Compiler constant".into()),
      documentation: Some(Documentation::MarkupContent(MarkupContent {
        kind: MarkupKind::Markdown,
        value: doc.to_string(),
      })),
      ..Default::default()
    });
  }
  let introspections: &[(&str, &str)] = &[
    ("#type of", "Get type of expression"),
    ("#unit of", "Get unit of expression"),
    ("#size of", "Get size of value in bytes"),
  ];
  for (op, doc) in introspections {
    items.push(CompletionItem {
      label: op.to_string(),
      kind: Some(CompletionItemKind::OPERATOR),
      detail: Some("Introspection operator".into()),
      documentation: Some(Documentation::MarkupContent(MarkupContent {
        kind: MarkupKind::Markdown,
        value: doc.to_string(),
      })),
      ..Default::default()
    });
  }
  let snippets: &[(&str, &str, &str)] = &[
    (
      "var ... as ... =",
      "var ${1:name} as ${2:type} = ${3:value}",
      "Variable definition",
    ),
    (
      "fn ... returns",
      "fn ${1:name}(${2:params}) returns ${3:type}\n    ${4:body}\nend fn",
      "Function definition",
    ),
    (
      "fn signature ... returns",
      "fn signature ${1:name}(${2:params}) returns ${3:type}",
      "Function signature",
    ),
    (
      "use ... from",
      "use ${1:all} from ${2:local}.${3:module}",
      "Module import",
    ),
    (
      "type ... end type",
      "type ${1:Name}\n    ${2:field} as ${3:type}\nend type",
      "Type (struct) definition",
    ),
    (
      "loop ... end loop",
      "loop\n    ${1:body}\nend loop",
      "Infinite loop",
    ),
    (
      "loop var ... end loop",
      "loop var ${1:i} as ${2:i32} from ${3:0} to ${4:10}\n    ${5:body}\nend loop",
      "Ranged loop",
    ),
    (
      "if ... end if",
      "if ${1:condition}\n    ${2:body}\nend if",
      "If statement",
    ),
    (
      "if ... else ... end if",
      "if ${1:condition}\n    ${2:body}\nelse\n    ${3:else_body}\nend if",
      "If/else statement",
    ),
    (
      "switch ... end switch",
      "switch ${1:expr}\n    case ${2:value}:\n        ${3:body}\n    default:\n        ${4:default_body}\nend switch",
      "Switch statement",
    ),
    (
      "match ... end match",
      "match ${1:expr}\n    case ${2:Variant}(${3:fields}):\n        ${4:body}\n    else:\n        ${5:else_body}\nend match",
      "Exhaustive pattern match",
    ),
    (
      "when ... end when",
      "when ${1:condition}\n    ${2:body}\nend when",
      "Conditional guard block",
    ),
    ("move on", "move on", "No-op statement"),
    ("missing code", "missing code", "Stub placeholder for unimplemented code"),
  ];
  for (label, insert, detail) in snippets {
    items.push(CompletionItem {
      label: label.to_string(),
      kind: Some(CompletionItemKind::SNIPPET),
      insert_text: Some(insert.to_string()),
      insert_text_format: Some(InsertTextFormat::SNIPPET),
      detail: Some(detail.to_string()),
      ..Default::default()
    });
  }
  items
}

// ---------------------------------------------------------------------------
// Helpers — Go to definition
// ---------------------------------------------------------------------------

/// Walk the AST and collect all definition sites (var, fn, type, enum, loop vars).
/// Returns a map from identifier name to (line, col, start_pos, end_pos).
fn extract_definitions(
  stmts: &[Stmt],
  defs: &mut std::collections::HashMap<String, (usize, usize)>,
) {
  for stmt in stmts {
    extract_defs_from_stmt(stmt, defs);
  }
}

fn extract_defs_from_stmt(
  stmt: &Stmt,
  defs: &mut std::collections::HashMap<String, (usize, usize)>,
) {
  match stmt {
    Stmt::VarDef(v) => {
      let loc = &v.name.span;
      defs.insert(v.name.node.clone(), (loc.start_pos, loc.end_pos));
      extract_defs_from_expr(&v.value, defs);
    }
    Stmt::FnDef(f) => {
      let loc = &f.name.span;
      defs.insert(f.name.node.clone(), (loc.start_pos, loc.end_pos));
      for param in &f.parameters {
        let ploc = &param.name.span;
        defs.insert(param.name.node.clone(), (ploc.start_pos, ploc.end_pos));
      }
      for s in &f.body {
        extract_defs_from_stmt(s, defs);
      }
    }
    Stmt::FnSignature(sig) => {
      let loc = &sig.name.span;
      defs.insert(sig.name.node.clone(), (loc.start_pos, loc.end_pos));
    }
    Stmt::TypeDef(td) => {
      let loc = &td.name.span;
      defs.insert(td.name.node.clone(), (loc.start_pos, loc.end_pos));
    }
    Stmt::EnumDef(ed) => {
      let loc = &ed.name.span;
      defs.insert(ed.name.node.clone(), (loc.start_pos, loc.end_pos));
    }
    Stmt::Loop(loop_stmt) => {
      if let Some(range) = &loop_stmt.range {
        let loc = &range.variable.span;
        defs.insert(range.variable.node.clone(), (loc.start_pos, loc.end_pos));
      }
      for s in &loop_stmt.body {
        extract_defs_from_stmt(s, defs);
      }
    }
    Stmt::Switch(switch_stmt) => {
      // Switch pattern bindings are name-only (not spanned), skip for now.
      for case in &switch_stmt.cases {
        for s in &case.body {
          extract_defs_from_stmt(s, defs);
        }
      }
    }
    Stmt::If(if_stmt) => {
      for s in &if_stmt.then_branch {
        extract_defs_from_stmt(s, defs);
      }
      for (_, else_if_body) in &if_stmt.else_if_branches {
        for s in else_if_body {
          extract_defs_from_stmt(s, defs);
        }
      }
      if let Some(else_branch) = &if_stmt.else_branch {
        for s in else_branch {
          extract_defs_from_stmt(s, defs);
        }
      }
    }
    // Compound statements that may contain VarDef in synthetic form:
    // read → handled by the AST builder's synthetic VarDefStmt
    // These are already converted to VarDef before we see them.
    _ => {}
  }
}

#[allow(clippy::only_used_in_recursion)]
fn extract_defs_from_expr(
  expr: &Expr,
  defs: &mut std::collections::HashMap<String, (usize, usize)>,
) {
  match expr {
    Expr::Binary(bin) => {
      extract_defs_from_expr(&bin.left, defs);
      extract_defs_from_expr(&bin.right, defs);
    }
    Expr::Unary(un) => {
      extract_defs_from_expr(&un.operand, defs);
    }
    Expr::Conversion(conv) => {
      extract_defs_from_expr(&conv.operand, defs);
    }
    Expr::Grouped(inner) => {
      extract_defs_from_expr(inner, defs);
    }
    Expr::FnCallExpr(call) => {
      for arg in &call.arguments {
        extract_defs_from_expr(arg, defs);
      }
    }
    Expr::ArrayLiteral(arr) => {
      for elem in &arr.elements {
        extract_defs_from_expr(elem, defs);
      }
    }
    _ => {}
  }
}

// ---------------------------------------------------------------------------
// Helpers — Document symbols
// ---------------------------------------------------------------------------

fn extract_document_symbols(source: &str) -> Vec<DocumentSymbol> {
  let mut symbols = Vec::new();
  for (line_idx, line) in source.lines().enumerate() {
    let trimmed = line.trim();
    if let Some(rest) = trimmed.strip_prefix("fn ") {
      if let Some(name) = rest.split(|c: char| c == '(' || c.is_whitespace()).next() {
        if !name.is_empty() && name != "signature" {
          let ns = name_start(name, line);
          symbols.push(make_sym(
            &format!("fn {}", name),
            SymbolKind::FUNCTION,
            line_idx as u32,
            line,
            trimmed,
            ns,
            name.len() as u32,
          ));
        }
      }
    } else if let Some(rest) = trimmed.strip_prefix("fn signature ") {
      if let Some(name) = rest.split(|c: char| c == '(' || c.is_whitespace()).next() {
        if !name.is_empty() {
          let ns = name_start(name, line);
          symbols.push(make_sym(
            &format!("fn signature {}", name),
            SymbolKind::INTERFACE,
            line_idx as u32,
            line,
            trimmed,
            ns,
            name.len() as u32,
          ));
        }
      }
    } else if let Some(rest) = trimmed.strip_prefix("type ") {
      if let Some(name) = rest.split_whitespace().next() {
        if name != "of" {
          let ns = name_start(name, line);
          symbols.push(make_sym(
            &format!("type {}", name),
            SymbolKind::STRUCT,
            line_idx as u32,
            line,
            trimmed,
            ns,
            name.len() as u32,
          ));
        }
      }
    } else if let Some(rest) = trimmed.strip_prefix("var ") {
      if let Some(name) = rest.split_whitespace().next() {
        let ns = name_start(name, line);
        symbols.push(make_sym(
          &format!("var {}", name),
          SymbolKind::VARIABLE,
          line_idx as u32,
          line,
          trimmed,
          ns,
          name.len() as u32,
        ));
      }
    }
  }
  symbols
}

fn name_start(name: &str, line: &str) -> u32 {
  line.find(name).unwrap_or(0) as u32
}

#[allow(deprecated)]
fn make_sym(
  name: &str,
  kind: SymbolKind,
  line: u32,
  full_line: &str,
  trimmed: &str,
  name_start: u32,
  name_len: u32,
) -> DocumentSymbol {
  let line_start = (full_line.len() - trimmed.len()) as u32;
  DocumentSymbol {
    name: name.to_string(),
    detail: None,
    kind,
    tags: None,
    deprecated: None,
    range: Range {
      start: Position {
        line,
        character: line_start,
      },
      end: Position {
        line,
        character: full_line.len() as u32,
      },
    },
    selection_range: Range {
      start: Position {
        line,
        character: name_start,
      },
      end: Position {
        line,
        character: name_start + name_len,
      },
    },
    children: None,
  }
}

// ---------------------------------------------------------------------------
// Backend — publish helper
// ---------------------------------------------------------------------------

impl Backend {
  async fn publish_diagnostics(&self, uri: &Url) {
    let diags = self.compute_diagnostics(uri);
    self
      .client
      .publish_diagnostics(uri.clone(), diags, None)
      .await;
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  #[allow(clippy::unwrap_used)]
  fn stdlib_function_imports_resolve_without_not_exported_errors() {
    // Regression test: the LSP's multi-file analysis must register stdlib
    // *function* exports under their own module path (e.g. "file_io_posix.lale"),
    // not under the importing module's path. A previous bug called
    // `load_stdlib_signatures()` inside the per-module loop, which re-registered
    // every stdlib function under the main module and made `get_exports` return
    // only the exported constants — producing spurious
    // "Symbol 'X' is not exported from module '...file_io_posix.lale'" errors.
    let source = "use openFile, writeFile, closeFile from std.file_io_posix\nwrite \"ok\"\n";

    // `resolve_stdlib_path()` falls back to a relative "stdlib/src" path, so run
    // from the workspace root (the parent of this crate's manifest directory).
    let workspace_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
      .parent()
      .unwrap()
      .to_path_buf();
    std::env::set_current_dir(&workspace_root).unwrap();

    let pairs = LaleParser::parse(Rule::program, source).unwrap();
    let program = build_program(pairs, "<test>").unwrap();
    let uri = Url::parse("file:///tmp/lale_lsp_test.lale").unwrap();

    let (errors, _warnings) = Backend::run_full_analysis(&uri, &program);

    let not_exported: Vec<_> = errors
      .iter()
      .filter(|e| e.message.contains("not exported"))
      .collect();
    assert!(
      not_exported.is_empty(),
      "stdlib function imports should resolve without 'not exported' errors, got: {:?}",
      errors
    );
  }
}
