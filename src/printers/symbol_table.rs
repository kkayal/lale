//! Symbol Table Printer - Pretty-prints symbol tables.
//!
//! This module provides functionality to display the symbol tables
//! generated during semantic analysis.

use crate::semantic_analysis::{AnalyzerResults, OwnedAnalyzer, SemanticAnalyzer, VarScope};
use colored::*;

/// Print all symbol tables from the semantic analyzer.
pub fn print_symbol_tables(analyzer: &SemanticAnalyzer, file_name: &str, use_colors: bool) {
  let all_tables = analyzer.get_all_symbol_tables();
  print_symbol_tables_impl(&all_tables, analyzer, file_name, use_colors);
}

/// Print all symbol tables from an OwnedAnalyzer.
pub fn print_symbol_tables_owned(analyzer: &OwnedAnalyzer, file_name: &str, use_colors: bool) {
  let all_tables = analyzer.get_all_symbol_tables();
  print_symbol_tables_impl(&all_tables, analyzer, file_name, use_colors);
}

/// Internal implementation for printing symbol tables.
fn print_symbol_tables_impl(
  all_tables: &std::collections::HashMap<
    VarScope,
    std::collections::HashMap<String, crate::semantic_analysis::Symbol>,
  >,
  analyzer: &dyn AnalyzerResults,
  file_name: &str,
  use_colors: bool,
) {
  colored::control::set_override(use_colors);

  println!("\n=== SYMBOL TABLES ({}) ===\n", file_name);

  // Print global scope
  if let Some(global_table) = all_tables.get(&VarScope::Global) {
    print_scope_table("Global Scope\n", global_table, analyzer, use_colors);

    // Print type definitions in global scope
    print_type_defs_in_global_scope(global_table, use_colors);
  }

  // Print function scopes
  let mut function_scopes: Vec<_> = all_tables
    .iter()
    .filter_map(|(scope, table)| {
      if let VarScope::Function { scope_key } = scope {
        Some((scope_key, table))
      } else {
        None
      }
    })
    .collect();

  // Sort by function name for consistent output
  function_scopes.sort_by(|a, b| {
    let a_name = &a.0.name;
    let b_name = &b.0.name;
    a_name.cmp(b_name)
  });

  for (scope_key, table) in function_scopes {
    let scope_name = format!("Function: {}\n", scope_key.name);
    print_scope_table(&scope_name, table, analyzer, use_colors);
  }

  // Print unused symbol detection info
  print_unused_symbols_info(analyzer, use_colors);

  println!("\n=== END SYMBOL TABLES ===\n");
}

/// Print a single symbol table for a given scope.
fn print_scope_table(
  scope_name: &str,
  table: &std::collections::HashMap<String, crate::semantic_analysis::Symbol>,
  analyzer: &dyn AnalyzerResults,
  use_colors: bool,
) {
  let header = if use_colors {
    scope_name.cyan().to_string()
  } else {
    scope_name.to_string()
  };

  println!("{}", header);

  if table.is_empty() {
    println!("  (empty)");
    println!();
    return;
  }

  // Collect and sort symbols by name
  let mut symbols: Vec<_> = table.iter().collect();
  symbols.sort_by_key(|(name, _)| *name);

  // Prepare rows with all column data
  let mut rows = Vec::new();
  for (name, symbol) in &symbols {
    let type_str = symbol.data_type.clone();
    let ptr_to_type_str = symbol
      .pointer_to_type
      .as_ref()
      .map(|t| format!("→ {}", t))
      .unwrap_or_default();
    let mut unit_str = symbol
      .physical_unit
      .as_ref()
      .map(|u| format!("[{}]", u))
      .unwrap_or_default();

    // For functions, also include the return unit from FnInfo if present
    let is_function = matches!(symbol.kind, crate::semantic_analysis::SymbolKind::Function);
    if is_function
      && let Some(fn_info) = analyzer.lookup_function(name)
      && let Some(return_unit) = &fn_info.return_unit
    {
      unit_str = format!("[{}]", return_unit.raw);
    }

    let kind_str = format!("{:?}", symbol.kind);
    let linkage_str = format!("{:?}", symbol.linkage);
    let is_parameter = matches!(symbol.kind, crate::semantic_analysis::SymbolKind::Parameter);
    let init_display = if is_parameter || is_function {
      "-"
    } else if symbol.is_initialized {
      "✓"
    } else {
      "✗"
    };
    let init_str = init_display.to_string();
    let location_str = format!(
      "{}:{}:{}",
      symbol.source_location.line, symbol.source_location.col, symbol.source_location.start_pos
    );
    let module_str = if symbol.module_path.is_empty() {
      "(current)".to_string()
    } else {
      symbol.module_path.clone()
    };

    rows.push((
      name.to_string(),
      type_str.clone(),
      ptr_to_type_str,
      unit_str,
      kind_str,
      linkage_str,
      init_str,
      location_str,
      module_str,
      symbol.is_initialized,
      type_str,
      is_parameter || is_function,
    ));
  }

  // Define headers
  let headers = (
    "Name",
    "Type",
    "Points To",
    "Unit",
    "Kind",
    "Linkage",
    "Init",
    "Location",
    "Module",
  );

  // Calculate column widths (max of header width and data width)
  // For the Init column, we need to count visible characters, not bytes
  // because the check mark (✓) is 3 bytes but 1 visible character
  let init_width = rows
    .iter()
    .map(|r| r.6.chars().count())
    .max()
    .unwrap_or(0)
    .max(headers.6.len())
    + 2;

  let col_widths = (
    rows
      .iter()
      .map(|r| r.0.len())
      .max()
      .unwrap_or(0)
      .max(headers.0.len()),
    rows
      .iter()
      .map(|r| r.1.len())
      .max()
      .unwrap_or(0)
      .max(headers.1.len()),
    rows
      .iter()
      .map(|r| r.2.len())
      .max()
      .unwrap_or(0)
      .max(headers.2.len()),
    rows
      .iter()
      .map(|r| r.3.len())
      .max()
      .unwrap_or(0)
      .max(headers.3.len()),
    rows
      .iter()
      .map(|r| r.4.len())
      .max()
      .unwrap_or(0)
      .max(headers.4.len()),
    rows
      .iter()
      .map(|r| r.5.len())
      .max()
      .unwrap_or(0)
      .max(headers.5.len()),
    init_width,
    rows
      .iter()
      .map(|r| r.7.len())
      .max()
      .unwrap_or(0)
      .max(headers.7.len()),
    rows
      .iter()
      .map(|r| r.8.len())
      .max()
      .unwrap_or(0)
      .max(headers.8.len()),
  );

  // Print markdown table header
  let init_header = format!("{:^width$}", headers.6, width = col_widths.6);
  println!(
    "| {:<width0$} | {:<width1$} | {:<width2$} | {:<width3$} | {:<width4$} | {:<width5$} | {} | {:<width7$} | {:<width8$} |",
    headers.0,
    headers.1,
    headers.2,
    headers.3,
    headers.4,
    headers.5,
    init_header,
    headers.7,
    headers.8,
    width0 = col_widths.0,
    width1 = col_widths.1,
    width2 = col_widths.2,
    width3 = col_widths.3,
    width4 = col_widths.4,
    width5 = col_widths.5,
    width7 = col_widths.7,
    width8 = col_widths.8,
  );

  // Print separator
  // Init column (index 6) is centered
  println!(
    "|{}|{}|{}|{}|{}|{}|:{}:|{}|{}|",
    "-".repeat(col_widths.0 + 2),
    "-".repeat(col_widths.1 + 2),
    "-".repeat(col_widths.2 + 2),
    "-".repeat(col_widths.3 + 2),
    "-".repeat(col_widths.4 + 2),
    "-".repeat(col_widths.5 + 2),
    "-".repeat(col_widths.6),
    "-".repeat(col_widths.7 + 2),
    "-".repeat(col_widths.8 + 2),
  );

  // Print rows
  for (
    name,
    type_str_display,
    ptr_to_type_str,
    unit_str,
    kind_str,
    linkage_str,
    init_str,
    location_str,
    module_str,
    is_initialized,
    type_str,
    is_parameter,
  ) in rows
  {
    // Format type with padding first, then apply color if pointer
    let type_padded = format!("{:<width$}", type_str_display, width = col_widths.1);
    let type_for_print = if use_colors && type_str == "pointer" {
      type_padded.yellow().to_string()
    } else {
      type_padded
    };

    // Format pointer_to_type with color if present
    let ptr_to_padded = format!("{:<width$}", ptr_to_type_str, width = col_widths.2);
    let ptr_to_for_print = if use_colors && !ptr_to_type_str.is_empty() {
      ptr_to_padded.cyan().to_string()
    } else {
      ptr_to_padded
    };

    // Center-align the check mark first, then apply color
    // Parameters show "-" without color since init doesn't apply to them
    let init_centered = format!("{:^width$}", init_str, width = col_widths.6);

    let init_colored = if is_parameter {
      init_centered
    } else if use_colors {
      if is_initialized {
        init_centered.green().to_string()
      } else {
        init_centered.red().to_string()
      }
    } else {
      init_centered
    };

    println!(
      "| {:<width0$} | {} | {} | {:<width3$} | {:<width4$} | {:<width5$} | {} | {:<width7$} | {:<width8$} |",
      name,
      type_for_print,
      ptr_to_for_print,
      unit_str,
      kind_str,
      linkage_str,
      init_colored,
      location_str,
      module_str,
      width0 = col_widths.0,
      width3 = col_widths.3,
      width4 = col_widths.4,
      width5 = col_widths.5,
      width7 = col_widths.7,
      width8 = col_widths.8,
    );
  }

  println!();
}

/// Print type definitions from the global scope.
fn print_type_defs_in_global_scope(
  global_table: &std::collections::HashMap<String, crate::semantic_analysis::Symbol>,
  use_colors: bool,
) {
  // Filter type defs from the global symbol table
  let type_defs: Vec<_> = global_table
    .iter()
    .filter(|(_, symbol)| matches!(symbol.kind, crate::semantic_analysis::SymbolKind::TypeDef))
    .collect();

  if type_defs.is_empty() {
    return;
  }

  let header = if use_colors {
    "\n=== TYPE DEFINITIONS ===\n".cyan().to_string()
  } else {
    "\n=== TYPE DEFINITIONS ===\n".to_string()
  };
  println!("{}", header);

  // Sort type defs by name
  let mut sorted_type_defs = type_defs;
  sorted_type_defs.sort_by_key(|(name, _)| *name);

  for (type_name, symbol) in sorted_type_defs {
    let export_str = match symbol.linkage {
      crate::semantic_analysis::Linkage::Export => "export",
      _ => "internal",
    };

    let location_str = format!(
      "{}:{}:{}",
      symbol.source_location.line, symbol.source_location.col, symbol.source_location.start_pos
    );

    if use_colors {
      println!(
        "  {} {} (at {})",
        type_name.cyan(),
        export_str.dimmed(),
        location_str.dimmed()
      );
    } else {
      println!("  {} {} (at {})", type_name, export_str, location_str);
    }
  }

  println!();
}

/// Print unused symbol detection information.
fn print_unused_symbols_info(analyzer: &dyn AnalyzerResults, use_colors: bool) {
  let header = if use_colors {
    "\n=== UNUSED SYMBOL DETECTION ===\n".cyan().to_string()
  } else {
    "\n=== UNUSED SYMBOL DETECTION ===\n".to_string()
  };
  println!("{}", header);
  println!("Note: 'Used' column shows usage status for each symbol at its specific location.\n");

  // Get defined and used symbols from the analyzer
  let used_symbols = analyzer.get_used_symbols();
  let used_locations = analyzer.get_used_symbol_locations();
  let unused_locations = analyzer.get_unused_symbol_locations();
  let all_tables = analyzer.get_all_symbol_tables();

  // Collect all symbols from the symbol tables (both defined and imported).
  // Value is (name, scope, kind, module, start_pos).
  type SymbolRow = (String, String, String, String, usize);
  let mut all_symbols: std::collections::HashMap<(usize, usize), SymbolRow> =
    std::collections::HashMap::new();

  // Add global scope symbols
  if let Some(global_table) = all_tables.get(&VarScope::Global) {
    for (name, symbol) in global_table.iter() {
      let kind_str = match &symbol.kind {
        crate::semantic_analysis::SymbolKind::Variable => "Variable".to_string(),
        crate::semantic_analysis::SymbolKind::Parameter => "Parameter".to_string(),
        crate::semantic_analysis::SymbolKind::Function => "Function".to_string(),
        crate::semantic_analysis::SymbolKind::TypeDef => "TypeDef".to_string(),
      };
      let module_str = if symbol.module_path.is_empty() {
        "main.lale".to_string()
      } else {
        symbol.module_path.clone()
      };
      all_symbols.insert(
        (symbol.source_location.line, symbol.source_location.col),
        (
          name.clone(),
          "Global".to_string(),
          kind_str,
          module_str,
          symbol.source_location.start_pos,
        ),
      );
    }
  }

  // Add function scope symbols
  for (scope, table) in &all_tables {
    if let VarScope::Function { scope_key } = scope {
      for (name, symbol) in table.iter() {
        let kind_str = match &symbol.kind {
          crate::semantic_analysis::SymbolKind::Variable => "Variable".to_string(),
          crate::semantic_analysis::SymbolKind::Parameter => "Parameter".to_string(),
          crate::semantic_analysis::SymbolKind::Function => "Function".to_string(),
          crate::semantic_analysis::SymbolKind::TypeDef => "TypeDef".to_string(),
        };
        let module_str = if symbol.module_path.is_empty() {
          "main.lale".to_string()
        } else {
          symbol.module_path.clone()
        };
        all_symbols.insert(
          (symbol.source_location.line, symbol.source_location.col),
          (
            name.clone(),
            scope_key.name.clone(),
            kind_str,
            module_str,
            symbol.source_location.start_pos,
          ),
        );
      }
    }
  }

  if all_symbols.is_empty() {
    println!("  No symbols defined.");
    println!();
    return;
  }

  // Convert to sorted vector
  let mut symbol_vec: Vec<_> = all_symbols.iter().collect();
  symbol_vec.sort_by_key(|(loc, _)| *loc);

  // Prepare rows
  let mut rows = Vec::new();
  for ((line, col), (name, scope_name, kind_str, module_str, start_pos)) in symbol_vec {
    // Check if this specific location was marked as unused
    let location_key = format!("{}:{}", line, col);
    let is_unused = unused_locations.contains(&location_key);

    // If not explicitly marked as unused, check if used
    let is_used = if is_unused {
      false
    } else {
      used_locations.contains(&location_key) || used_symbols.contains(name)
    };

    let used_str = if is_used { "✓" } else { "✗" };
    let location_str = format!("{}:{}:{}", line, col, start_pos);

    rows.push((
      name.clone(),
      kind_str.clone(),
      scope_name.clone(),
      used_str.to_string(),
      location_str,
      module_str.clone(),
      is_used,
    ));
  }

  // Define headers
  let headers = ("Name", "Kind", "Scope", "Used", "Location", "Module");

  // Calculate column widths
  let used_width = rows
    .iter()
    .map(|r| r.3.chars().count())
    .max()
    .unwrap_or(0)
    .max(headers.3.len())
    + 2;

  let col_widths = (
    rows
      .iter()
      .map(|r| r.0.len())
      .max()
      .unwrap_or(0)
      .max(headers.0.len()),
    rows
      .iter()
      .map(|r| r.1.len())
      .max()
      .unwrap_or(0)
      .max(headers.1.len()),
    rows
      .iter()
      .map(|r| r.2.len())
      .max()
      .unwrap_or(0)
      .max(headers.2.len()),
    used_width,
    rows
      .iter()
      .map(|r| r.4.len())
      .max()
      .unwrap_or(0)
      .max(headers.4.len()),
    rows
      .iter()
      .map(|r| r.5.len())
      .max()
      .unwrap_or(0)
      .max(headers.5.len()),
  );

  // Print markdown table header
  let used_header = format!("{:^width$}", headers.3, width = col_widths.3);
  println!(
    "| {:<width0$} | {:<width1$} | {:<width2$} | {} | {:<width4$} | {:<width5$} |",
    headers.0,
    headers.1,
    headers.2,
    used_header,
    headers.4,
    headers.5,
    width0 = col_widths.0,
    width1 = col_widths.1,
    width2 = col_widths.2,
    width4 = col_widths.4,
    width5 = col_widths.5,
  );

  // Print separator
  println!(
    "|{}|{}|{}|:{}:|{}|{}|",
    "-".repeat(col_widths.0 + 2),
    "-".repeat(col_widths.1 + 2),
    "-".repeat(col_widths.2 + 2),
    "-".repeat(col_widths.3),
    "-".repeat(col_widths.4 + 2),
    "-".repeat(col_widths.5 + 2),
  );

  // Print rows
  for (name, kind_str, scope_str, used_str, location_str, module_str, is_used) in rows {
    // Center-align the check mark first, then apply color
    let used_centered = format!("{:^width$}", used_str, width = col_widths.3);

    let used_colored = if use_colors {
      if is_used {
        used_centered.green().to_string()
      } else {
        used_centered.red().to_string()
      }
    } else {
      used_centered
    };

    println!(
      "| {:<width0$} | {:<width1$} | {:<width2$} | {} | {:<width4$} | {:<width5$} |",
      name,
      kind_str,
      scope_str,
      used_colored,
      location_str,
      module_str,
      width0 = col_widths.0,
      width1 = col_widths.1,
      width2 = col_widths.2,
      width4 = col_widths.4,
      width5 = col_widths.5,
    );
  }

  println!();
}
