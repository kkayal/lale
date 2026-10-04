//! Lale Compiler - CLI Entry Point
//!
//! This binary implements the **command-line interface for the Lale compiler**.
//! It orchestrates the compilation pipeline:
//!
//! # Pipeline Steps
//!
//! 1. **Input**: Read Lale source code from file or stdin
//! 2. **Parse**: Convert source to pest parse tree via `LaleParser::parse()`
//! 3. **Build AST**: Convert parse tree to semantic AST via `build_program()`
//! 4. **Semantic Analysis**: Type/unit/scope checking via the semantic analyzer
//! 5. **IR Generation**: Lower AST to structured intermediate representation
//! 6. **Interpret**: Execute IR via the built-in interpreter
//!
//! # Command-Line Options
//!
//! ```text
//! lale run <source_file> [OPTIONS]       # Compile and execute using the interpreter
//!
//! Compilation Options (run):
//!   --no-color              Disable colored terminal output
//!   --print-raw-parse-tree  Display raw pest parse tree (for grammar debugging)
//!   --print-ast             Display the constructed AST
//!   --print-ir              Display the intermediate representation (IR)
//!   --print-symbols         Display symbol tables (global and function scopes)
//! ```
//!
//! # Input Handling
//!
//! - **File Input**: Reads `.lale` files from the filesystem
//! - **Stdin Input**: Use `-` as the source file to read from stdin
//!   - Example: `echo 'write "hello"' | lale run -`
//!   - Errors display `<stdin>` as the file name
//!   - Module imports resolve relative to current working directory
//!
//! # Example Usage
//!
//! ```bash
//! # Compile and execute using the interpreter
//! lale run program.lale
//!
//! # Read source from stdin
//! echo 'write "hello"' | lale run -
//!
//! # Display intermediate representations
//! lale run program.lale --print-ast --print-symbols
//!
//! # Disable colors (for CI/CD pipelines)
//! lale run program.lale --no-color
//! ```

use std::error::Error;
use std::fs;
use std::io::{self, IsTerminal, Read};
use std::path::PathBuf;
use std::process;

use clap::Parser as ClapParser;
use colored::*;
use pest::Parser;

use std::cell::RefCell;
use std::path::Path;
use std::rc::Rc;

use lale::ast::SourceLocation;
use lale::ast::builder::build_program;
use lale::config::{ColorChoice, StdlibLevel};
use lale::ir::print_module;
use lale::ir_gen::IrGenerator;
use lale::printers::ast::print_ast as print_ast_tree;
use lale::printers::pairs::print_pairs;
use lale::printers::symbol_table::print_symbol_tables_owned;
use lale::semantic_analysis::{AnalyzerResults, ModuleId, ModuleResolver, SqliteSymbolManager};
use lale::{LaleParser, Rule};

mod builtins;
mod parity;

/// Extract just the filename from a path, or return the path as-is if it's "-"
fn display_filename(path: &str) -> String {
  if path == "-" {
    "<stdin>".to_string()
  } else {
    path.to_string()
  }
}

/// Resolve the effective color choice from the CLI flags. The legacy
/// `--no-color` flag takes precedence and forces `ColorChoice::Never`.
fn resolve_color_choice(color: &str, no_color: bool) -> ColorChoice {
  if no_color {
    return ColorChoice::Never;
  }
  match ColorChoice::parse(color) {
    Ok(choice) => choice,
    Err(e) => {
      eprintln!("{}", format!("Error: {}", e).red());
      process::exit(1);
    }
  }
}

/// Apply the resolved color choice to the process-wide `colored` crate so that
/// compile-time diagnostics (`eprintln!().red()` / `.yellow()`) honor the flag
/// alongside the interpreter's runtime-error output.
fn apply_color_override(choice: ColorChoice) {
  match choice {
    ColorChoice::Always => colored::control::set_override(true),
    ColorChoice::Never => colored::control::set_override(false),
    ColorChoice::Auto => colored::control::unset_override(),
  }
}

/// Command-line arguments for the Lale compiler.
#[derive(clap_derive::Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
  #[command(subcommand)]
  command: Command,
}

#[derive(clap_derive::Subcommand, Debug)]
enum Command {
  /// Compile and execute using the interpreter
  Run {
    /// Source file name
    source_file: String,

    /// Disable colors in terminal output
    #[arg(long, default_value_t = false)]
    no_color: bool,

    /// Control colored output: auto (default, TTY-detected), always, or never
    #[arg(long, default_value = "auto")]
    color: String,

    /// Print raw pest parse tree (for grammar debugging)
    #[arg(long, default_value_t = false)]
    print_raw_parse_tree: bool,

    /// Print the AST
    #[arg(long, default_value_t = false)]
    print_ast: bool,

    /// Print the intermediate representation (IR)
    #[arg(long, default_value_t = false)]
    print_ir: bool,

    /// Print the symbol tables
    #[arg(long, default_value_t = false)]
    print_symbols: bool,

    /// Export symbol database after compilation (SQLite format)
    #[arg(long)]
    export_symbols: Option<PathBuf>,

    /// Standard library inclusion level: none, core, or full
    #[arg(long = "stdlib-level", default_value = "full")]
    stdlib: String,

    /// Build in release mode: disables #debug, enables optimisations
    #[arg(long, default_value_t = false)]
    release: bool,

    /// Disable integer overflow trapping (wrap instead)
    #[arg(long, default_value_t = false)]
    unchecked_overflow: bool,

    /// Run in test mode: execute test suites instead of top-level code
    #[arg(long, default_value_t = false)]
    test: bool,

    /// Show all code paths that can exit the program
    #[arg(long, default_value_t = false)]
    show_exit_paths: bool,

    /// Serialize, deserialize, and execute the reconstructed IR
    #[arg(long, default_value_t = false)]
    roundtrip: bool,
  },

  /// Run test suites from the source file
  Test {
    /// Source file name
    source_file: String,

    /// Disable colors in terminal output
    #[arg(long, default_value_t = false)]
    no_color: bool,

    /// Control colored output: auto (default, TTY-detected), always, or never
    #[arg(long, default_value = "auto")]
    color: String,

    /// Print raw pest parse tree (for grammar debugging)
    #[arg(long, default_value_t = false)]
    print_raw_parse_tree: bool,

    /// Print the AST
    #[arg(long, default_value_t = false)]
    print_ast: bool,

    /// Print the intermediate representation (IR)
    #[arg(long, default_value_t = false)]
    print_ir: bool,

    /// Print the symbol tables
    #[arg(long, default_value_t = false)]
    print_symbols: bool,

    /// Export symbol database after compilation (SQLite format)
    #[arg(long)]
    export_symbols: Option<PathBuf>,

    /// Standard library inclusion level: none, core, or full
    #[arg(long = "stdlib-level", default_value = "full")]
    stdlib: String,

    /// Build in release mode: disables #debug, enables optimisations
    #[arg(long, default_value_t = false)]
    release: bool,

    /// Disable integer overflow trapping (wrap instead)
    #[arg(long, default_value_t = false)]
    unchecked_overflow: bool,

    /// Show all code paths that can exit the program
    #[arg(long, default_value_t = false)]
    show_exit_paths: bool,

    /// Serialize, deserialize, and execute the reconstructed IR
    #[arg(long, default_value_t = false)]
    roundtrip: bool,

    /// Run only test suites/cases matching any of the given patterns.
    /// Repeatable: `--filter A --filter B` runs cases matching A or B.
    /// A pattern containing `/` matches `suite/case` (same separator as the
    /// `Pass: suite / case` output); otherwise it matches the suite name or
    /// the case name.
    #[arg(long)]
    filter: Vec<String>,
  },

  /// Run the differential parity harness across registered backends
  Parity {
    /// Source file name
    source_file: String,

    /// Disable colors in terminal output
    #[arg(long, default_value_t = false)]
    no_color: bool,

    /// Control colored output: auto (default, TTY-detected), always, or never
    #[arg(long, default_value = "auto")]
    color: String,

    /// Standard library inclusion level: none, core, or full
    #[arg(long = "stdlib-level", default_value = "full")]
    stdlib: String,

    /// Build in release mode: disables #debug, enables optimisations
    #[arg(long, default_value_t = false)]
    release: bool,

    /// Disable integer overflow trapping (wrap instead)
    #[arg(long, default_value_t = false)]
    unchecked_overflow: bool,

    /// Show all code paths that can exit the program
    #[arg(long, default_value_t = false)]
    show_exit_paths: bool,
  },
}

fn main() -> Result<(), Box<dyn Error>> {
  // Show hint if no arguments and stdin is piped
  let has_args = std::env::args().len() > 1;
  if !has_args && !io::stdin().is_terminal() {
    eprintln!("hint: To read source from stdin, use: lale run -");
    eprintln!();
    eprintln!("usage: lale run <source_file> [OPTIONS]");
    eprintln!();
    eprintln!("commands:");
    eprintln!("  run    - Compile and execute using the interpreter");
    eprintln!();
    eprintln!("example: lale run program.lale");
    eprintln!("         echo 'write \"hello\"' | lale run -");
    std::process::exit(1);
  }

  // Parse command line arguments
  let args = Args::parse();

  // Match on the subcommand
  match args.command {
    Command::Run {
      source_file,
      no_color,
      color,
      print_raw_parse_tree,
      print_ast,
      print_ir,
      print_symbols,
      export_symbols,
      stdlib,
      release,
      unchecked_overflow,
      test,
      show_exit_paths,
      roundtrip,
    } => {
      let stdlib_parsed = StdlibLevel::parse(&stdlib).unwrap_or_else(|e| {
        eprintln!("{}", format!("Error: {}", e).red());
        process::exit(1);
      });
      let choice = resolve_color_choice(&color, no_color);
      apply_color_override(choice);
      let use_color = choice.use_color();
      compile_and_execute(
        &source_file,
        use_color,
        print_raw_parse_tree,
        print_ast,
        print_ir,
        print_symbols,
        export_symbols.as_deref(),
        stdlib_parsed,
        !release,
        !unchecked_overflow,
        test,
        Vec::new(),
        show_exit_paths,
        roundtrip,
      )?;
    }
    Command::Test {
      source_file,
      no_color,
      color,
      print_raw_parse_tree,
      print_ast,
      print_ir,
      print_symbols,
      export_symbols,
      stdlib,
      release,
      unchecked_overflow,
      show_exit_paths,
      roundtrip,
      filter,
    } => {
      let stdlib_parsed = StdlibLevel::parse(&stdlib).unwrap_or_else(|e| {
        eprintln!("{}", format!("Error: {}", e).red());
        process::exit(1);
      });
      let choice = resolve_color_choice(&color, no_color);
      apply_color_override(choice);
      let use_color = choice.use_color();
      compile_and_execute(
        &source_file,
        use_color,
        print_raw_parse_tree,
        print_ast,
        print_ir,
        print_symbols,
        export_symbols.as_deref(),
        stdlib_parsed,
        !release,
        !unchecked_overflow,
        true,
        filter,
        show_exit_paths,
        roundtrip,
      )?;
    }
    Command::Parity {
      source_file,
      no_color,
      color,
      stdlib,
      release,
      unchecked_overflow,
      show_exit_paths,
    } => {
      let color_choice = resolve_color_choice(&color, no_color);
      apply_color_override(color_choice);
      parity::run_parity(&parity::ParityOptions {
        source_file,
        color: color_choice,
        stdlib,
        release,
        unchecked_overflow,
        show_exit_paths,
      })?;
    }
  }

  Ok(())
}

/// Compile and execute Lale source code using the interpreter.
///
/// This function orchestrates the full compilation pipeline:
/// 1. Parse and build AST
/// 2. Semantic analysis (multi-module if use statements present)
/// 3. IR generation
/// 4. Interpreter execution
///
/// # Arguments
///
/// - `source_file`: Path to the source file, or "-" to read from stdin
/// - `use_color`: Whether to emit colored output (already resolved from `--color`)
/// - `print_raw_parse_tree`: Print raw parse tree
/// - `print_ast`: Print AST
/// - `print_ir`: Print intermediate representation
/// - `print_symbols`: Print symbol tables
/// - `export_symbols`: Export symbol table to file
/// - `stdlib`: Standard library level (none, core, or full)
/// - `is_debug`: Debug mode (true by default, false with --release)
/// - `checked_overflow`: Whether integer overflow traps (true by default)
#[allow(clippy::too_many_arguments)]
fn compile_and_execute(
  source_file: &str,
  use_color: bool,
  print_raw_parse_tree: bool,
  print_ast: bool,
  print_ir: bool,
  print_symbols: bool,
  export_symbols: Option<&std::path::Path>,
  stdlib: StdlibLevel,
  is_debug: bool,
  checked_overflow: bool,
  test_mode: bool,
  test_filter: Vec<String>,
  show_exit_paths: bool,
  roundtrip: bool,
) -> Result<(), Box<dyn Error>> {
  // Read source: from stdin if "-", otherwise from file
  let (input, display_name) = if source_file == "-" {
    let mut buffer = String::new();
    io::stdin()
      .read_to_string(&mut buffer)
      .map_err(|e| format!("Error reading from stdin: {}", e))?;
    (buffer, "<stdin>".to_string())
  } else {
    let content = fs::read_to_string(source_file)
      .map_err(|e| format!("Error reading file {}: {}", source_file, e))?;
    (content, display_filename(source_file))
  };

  // Parse source code into pest parse tree
  let pairs = LaleParser::parse(Rule::program, &input).map_err(|e| {
    let (line, col) = match e.line_col {
      pest::error::LineColLocation::Pos((l, c)) => (l, c),
      pest::error::LineColLocation::Span((l, c), _) => (l, c),
    };
    let error_msg = format!(
      "{}:{}:{}: grammar error: {}",
      display_name,
      line,
      col,
      e.variant.message()
    );
    eprintln!("{}", error_msg.red());
    for line in e.to_string().lines() {
      if !line.starts_with(" -->") && !line.contains("= expected") {
        eprintln!("{}", line.red());
      }
    }
    "Parse error"
  })?;

  // Optionally print raw parse tree (for grammar debugging)
  if print_raw_parse_tree {
    print_pairs(pairs.clone(), &display_name, 0, use_color);
    println!("\n======\n");
  }

  // Build the AST from parse tree
  let program = build_program(pairs, source_file)?;

  // Optionally print the AST
  if print_ast {
    print_ast_tree(&program, &display_name, use_color);
  }

  // Build compiler options (interpreter backend)
  let options = lale::CompilerOptions::default()
    .stdlib(stdlib)
    .debug(is_debug)
    .checked_overflow(checked_overflow)
    .test_mode(test_mode);

  // Set up module resolver for multi-file compilation
  let (source_path, base_dir) = if source_file == "-" {
    let cwd = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    (cwd.join("<stdin>"), cwd)
  } else {
    let path = PathBuf::from(source_file);
    let dir = path.parent().unwrap_or(Path::new(".")).to_path_buf();
    (path, dir)
  };
  let resolver = Rc::new(RefCell::new(ModuleResolver::new(base_dir)));

  // Build module graph from root (clone program for the resolver)
  {
    let mut resolver_mut = resolver.borrow_mut();
    match resolver_mut.build_graph_from_root(&source_path, program.clone()) {
      Ok(_) => {}
      Err(e) => {
        eprintln!("{}", format!("{}: module error: {}", display_name, e).red());
        process::exit(1);
      }
    }

    // Check for cycles
    if let Some(cycle) = resolver_mut.graph.detect_cycle() {
      let cycle_str: Vec<_> = cycle.iter().map(|m| m.to_string()).collect();
      eprintln!(
        "{}",
        format!(
          "{}: module error: Circular import detected: {}",
          display_name,
          cycle_str.join(".")
        )
        .red()
      );
      process::exit(1);
    }
  }

  // Get compilation order (dependencies first)
  let compilation_order: Vec<ModuleId> = {
    let resolver_ref = resolver.borrow();
    resolver_ref
      .compilation_order()
      .expect("No cycles (already checked)")
  };

  // Create a SINGLE shared symbol manager for all modules
  let mut shared_manager = SqliteSymbolManager::new();

  // Register str as a pre-defined type
  shared_manager.define_type_with_fields(
    "text",
    SourceLocation::dummy(),
    vec![
      ("ptr".to_string(), "pointer".to_string(), None, false),
      (
        "bytes".to_string(),
        "u64".to_string(),
        Some("<bytes>".to_string()),
        false,
      ),
      (
        "chars".to_string(),
        "u64".to_string(),
        Some("<chars>".to_string()),
        false,
      ),
    ],
  );

  // Validate builtins.lale first (fail‑fast, temp manager to avoid conflicts
  // with the stdlib loading that happens later in load_stdlib_and_dependencies).
  let builtins_source = builtins::BUILTINS_SOURCE;

  let builtins_pairs = LaleParser::parse(Rule::program, builtins_source)
    .map_err(|e| format!("Parse error in embedded builtins.lale: {}", e))?;

  let builtins_program = build_program(builtins_pairs, "builtins.lale")?;

  // Validate builtins with a TEMPORARY manager to avoid polluting shared_manager.
  // The actual builtins symbol registration happens in load_stdlib_and_dependencies.
  {
    let mut temp_manager = lale::semantic_analysis::SqliteSymbolManager::new();
    let mut builtins_analyzer = lale::semantic_analysis::SemanticAnalyzer::with_mut_manager(
      &mut temp_manager,
      options.clone(),
    );

    builtins_analyzer.set_resolver(resolver.clone());
    builtins_analyzer.set_root_file_path(source_path.clone());
    builtins_analyzer.set_current_module_path(PathBuf::from("builtins.lale"));

    lale::ast::AstVisitor::visit_program(&mut builtins_analyzer, &builtins_program);

    if !builtins_analyzer.get_errors().is_empty() {
      eprintln!("Errors in embedded builtins.lale:");
      for error in builtins_analyzer.get_errors() {
        eprintln!("  {}", error);
      }
      return Err("Failed to compile builtins".into());
    }
  }

  // Analyze each module in order (dependencies first: builtins → core → stdlib → user modules)
  let mut root_errors = Vec::new();
  let mut root_warnings = Vec::new();
  let mut root_defined_symbols = std::collections::HashMap::new();
  let mut root_used_symbols = std::collections::HashSet::new();

  let canonical_source = match source_path.canonicalize() {
    Ok(p) => p,
    Err(_) => source_path.clone(),
  };

  for module_id in compilation_order.iter() {
    let program_to_analyze;
    {
      let resolver_ref = resolver.borrow();
      let module = resolver_ref.graph.get(module_id).expect("Module exists");
      program_to_analyze = module.program.clone();
    }

    let mut analyzer = lale::semantic_analysis::SemanticAnalyzer::with_mut_manager(
      &mut shared_manager,
      options.clone(),
    );

    analyzer.set_resolver(resolver.clone());
    analyzer.set_root_file_path(source_path.clone());
    analyzer.set_current_module_path(module_id.path().to_path_buf());

    lale::ast::AstVisitor::visit_program(&mut analyzer, &program_to_analyze);

    // Keep root module's analyzer data for error reporting and code generation
    if module_id.path() == canonical_source.as_path() {
      root_errors = analyzer.get_errors().to_vec();
      root_warnings = analyzer.get_warnings().to_vec();
      root_defined_symbols = analyzer.get_defined_symbols().clone();
      root_used_symbols = analyzer.get_used_symbols().clone();
    }
  }

  // Create an OwnedAnalyzer with the results from the root module
  let owned_analyzer = lale::semantic_analysis::OwnedAnalyzer::new(
    shared_manager,
    root_errors,
    root_warnings,
    root_defined_symbols,
    root_used_symbols,
    std::collections::HashSet::new(),
    std::collections::HashSet::new(),
    options.is_debug,
    options.checked_overflow,
    options.test_mode,
  );

  // Optionally print the symbol tables
  if print_symbols {
    print_symbol_tables_owned(&owned_analyzer, &display_name, use_color);
  }

  // Print any errors that occurred during analysis
  if !owned_analyzer.is_valid() {
    for error in owned_analyzer.get_errors() {
      eprintln!(
        "{}",
        format!(
          "{}:{}:{}: semantic error: {}",
          display_name,
          error.line(),
          error.col(),
          error.message
        )
        .red()
      );
    }
    process::exit(1);
  }

  // Print any warnings that occurred during analysis
  for warning in owned_analyzer.get_warnings() {
    eprintln!(
      "{}",
      format!(
        "{}:{}:{}: Compile-time warning: {}",
        display_name,
        warning.line(),
        warning.col(),
        warning.message
      )
      .yellow()
    );
  }

  // Export symbol database if requested
  if let Some(export_path) = export_symbols {
    match owned_analyzer.symbols().export_database(export_path) {
      Ok(_) => {
        if use_color {
          eprintln!(
            "{}",
            format!("✓ Symbol database exported: {}", export_path.display()).green()
          );
        } else {
          eprintln!("Symbol database exported: {}", export_path.display());
        }
      }
      Err(e) => {
        eprintln!("{}", format!("Failed to export symbols: {}", e).red());
        process::exit(1);
      }
    }
  }

  // Process compile-time directives (#if/#end if)
  let processed_program = lale::semantic_analysis::process_ct_directives(&program, &owned_analyzer);

  // Show exit paths if requested
  if show_exit_paths {
    let exit_paths = lale::exit_paths::find_exit_paths(&processed_program, is_debug);
    let report = lale::exit_paths::format_exit_paths(&exit_paths, &display_name);
    eprintln!("{}", report);
  }

  // IR Generation: Convert AST to Intermediate Representation.
  //
  // Step 1: Create module and load builtins.lale (type str, FFI declarations,
  // type conversions). Builtins are always loaded — they are embedded in the
  // compiler and have no external dependency.
  let mut module = lale::ir::Module::new("main");
  IrGenerator::load_builtins_into_module(&mut module, checked_overflow)?;

  // Step 2: Optionally load the standard library (depends on builtins).
  // Skip when the source file IS the stdlib (stdlib development mode).
  let is_stdlib_source = std::path::Path::new(source_file)
    .canonicalize()
    .map(|p| {
      p.to_string_lossy().contains("/stdlib/src/")
        || p.to_string_lossy().contains("\\stdlib\\src\\")
    })
    .unwrap_or(false);

  if stdlib != StdlibLevel::None && !is_stdlib_source {
    IrGenerator::load_stdlib_into_module(&mut module, stdlib, checked_overflow)?;
  }

  // Step 2b: Generate and merge IR for non-root user modules, in dependency
  // order. Their functions and export globals are copied into the final module
  // (the root module itself is generated in Step 3). Stdlib modules are skipped
  // — they were already loaded via `load_stdlib_into_module`.
  for module_id in compilation_order.iter() {
    if module_id.path() == canonical_source.as_path() {
      continue; // root module — generated below
    }
    let path_str = module_id.path().to_string_lossy();
    if path_str.contains("/stdlib/src/") || path_str.contains("\\stdlib\\src\\") {
      continue; // stdlib module — already merged
    }
    let module_program = {
      let resolver_ref = resolver.borrow();
      resolver_ref.graph.get(module_id).map(|m| m.program.clone())
    };
    let Some(module_program) = module_program else {
      continue;
    };
    // Resolve compile-time directives against the shared symbol manager.
    let processed =
      lale::semantic_analysis::process_ct_directives(&module_program, &owned_analyzer);
    let module_name = match module_id.path().file_name() {
      Some(f) => f.to_string_lossy().to_string(),
      None => module_id.path().to_string_lossy().to_string(),
    };
    IrGenerator::generate_and_merge_module(
      &mut module,
      &module_name,
      &processed,
      owned_analyzer.symbols(),
      is_debug,
      checked_overflow,
      test_mode,
    )?;
  }

  // Step 3: Generate user code into the pre-populated module.
  let mut ir_generator = IrGenerator::with_module_and_stdlib(module, "main", stdlib);
  ir_generator.set_no_color(!use_color);
  ir_generator.set_test_filter(test_filter);
  let mut ir_module = ir_generator.try_generate_owned(
    &processed_program,
    &owned_analyzer,
    stdlib != StdlibLevel::None,
  )?;

  // Print IR module if requested
  if print_ir {
    eprintln!("\n{}", "=== IR Module ===".cyan());
    eprintln!("{}", print_module(&ir_module));
  }

  // Exercise the serialization contract: serialize the freshly generated IR,
  // deserialize it, and execute the reconstructed module. This catches printer
  // drift on every compile, not only in dedicated round-trip tests.
  if roundtrip {
    let text = print_module(&ir_module);
    ir_module =
      lale::ir::parse_module(&text).map_err(|e| format!("IR round-trip failed: {}", e))?;
  }

  // Execute the IR module using the interpreter
  lale::interpreter::set_color_enabled(use_color);
  match lale::interpreter::execute_module(&ir_module) {
    Ok(exit_code) => {
      if exit_code != 0 {
        eprintln!(
          "{}",
          format!("Program exited with code: {}", exit_code).yellow()
        );
        // Honour interpreter-reported exit codes (e.g. a failing test case
        // forces exit code 1). `__lale_exit` already terminates the process
        // directly, so this path is only reached for non-exit-statement
        // outcomes such as test failures.
        process::exit(exit_code);
      }
    }
    Err(e) => {
      eprintln!(
        "{}",
        format!("compiler error: Execution failed: {}", e).red()
      );
      process::exit(1);
    }
  }

  Ok(())
}
