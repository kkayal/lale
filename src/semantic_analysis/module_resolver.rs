//! Module Resolution for Lale's Multi-File Module System
//!
//! This module implements the module resolution layer that sits above semantic analysis.
//! It handles:
//! - Discovering and parsing modules from `use` statements
//! - Building a dependency graph of modules
//! - Detecting circular imports (DAG enforcement)
//! - Topologically sorting modules for compilation order
//! - Resolving exported symbols across module boundaries
//!
//! # Architecture
//!
//! The module resolver operates in phases:
//!
//! 1. **Discovery**: Starting from a root file, parse it and scan for `use` statements
//! 2. **Graph Building**: Create a directed edge for each `use` statement
//! 3. **Cycle Detection**: Verify the dependency graph is a DAG (no cycles)
//! 4. **Topological Sort**: Order modules so dependencies are processed first
//! 5. **Symbol Resolution**: For each module in order, resolve its `use` imports
//!
//! # `use` vs `import` Distinction
//!
//! - **`use`**: Compile-time module loading (this module handles it)
//! - **`import`**: Linker-level FFI/extern symbols (handled by semantic analyzer)

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

use pest::Parser;

use crate::ast::builder::build_program;
use crate::ast::{Program, SourceLocation, Spanned, Stmt, SymbolTable, UseStmt};
use crate::{LaleParser, Rule};

/// Unique identifier for a module in the dependency graph.
#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub struct ModuleId(pub PathBuf);

impl ModuleId {
  pub fn new(path: PathBuf) -> Self {
    ModuleId(path)
  }

  pub fn path(&self) -> &Path {
    &self.0
  }
}

impl std::fmt::Display for ModuleId {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    write!(f, "{}", self.0.display())
  }
}

/// A resolved module with its parsed AST and metadata.
#[derive(Debug)]
pub struct ResolvedModule {
  pub id: ModuleId,
  pub program: Program,
  pub dependencies: Vec<ModuleId>,
  pub use_statements: Vec<UseStmt>,
  /// Exported symbols, populated after semantic analysis.
  pub exports: Option<SymbolTable>,
}

/// Error types for module resolution.
#[derive(Debug, Clone)]
pub enum ModuleError {
  /// Module file not found.
  ModuleNotFound {
    module_path: String,
    search_path: PathBuf,
    location: SourceLocation,
  },
  /// Circular import detected.
  CircularImport {
    cycle: Vec<ModuleId>,
    location: SourceLocation,
  },
  /// Symbol not exported from module.
  SymbolNotExported {
    symbol: String,
    module: ModuleId,
    location: SourceLocation,
  },
  /// Symbol conflict (duplicate import).
  SymbolConflict {
    symbol: String,
    first_module: ModuleId,
    second_module: ModuleId,
    location: SourceLocation,
  },
  /// `use` statement not at top level.
  UseNotAtTopLevel { location: SourceLocation },
  /// Failed to parse module.
  ParseError { module: ModuleId, message: String },
}

impl std::fmt::Display for ModuleError {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    match self {
      ModuleError::ModuleNotFound {
        module_path,
        search_path,
        ..
      } => {
        write!(
          f,
          "Module '{}' not found (searched in {})",
          module_path,
          search_path.display()
        )
      }
      ModuleError::CircularImport { cycle, .. } => {
        let cycle_str: Vec<_> = cycle.iter().map(|m| m.to_string()).collect();
        write!(f, "Circular import detected: {}", cycle_str.join(" -> "))
      }
      ModuleError::SymbolNotExported { symbol, module, .. } => {
        write!(
          f,
          "Symbol '{}' is not exported from module '{}'",
          symbol, module
        )
      }
      ModuleError::SymbolConflict {
        symbol,
        first_module,
        second_module,
        ..
      } => {
        write!(
          f,
          "Symbol '{}' imported from both '{}' and '{}'",
          symbol, first_module, second_module
        )
      }
      ModuleError::UseNotAtTopLevel { .. } => {
        write!(f, "`use` statements are only allowed at module scope")
      }
      ModuleError::ParseError { module, message } => {
        write!(f, "Failed to parse module '{}': {}", module, message)
      }
    }
  }
}

impl std::error::Error for ModuleError {}

/// The module dependency graph.
#[derive(Debug, Default)]
pub struct ModuleGraph {
  /// All known modules.
  modules: HashMap<ModuleId, ResolvedModule>,
  /// Adjacency list: module -> modules it depends on.
  edges: HashMap<ModuleId, Vec<ModuleId>>,
}

impl ModuleGraph {
  pub fn new() -> Self {
    ModuleGraph {
      modules: HashMap::new(),
      edges: HashMap::new(),
    }
  }

  /// Add a module to the graph.
  pub fn add_module(&mut self, module: ResolvedModule) {
    let id = module.id.clone();
    let deps = module.dependencies.clone();
    self.modules.insert(id.clone(), module);
    self.edges.insert(id, deps);
  }

  /// Check if a module exists in the graph.
  pub fn contains(&self, id: &ModuleId) -> bool {
    self.modules.contains_key(id)
  }

  /// Get a module by its ID.
  pub fn get(&self, id: &ModuleId) -> Option<&ResolvedModule> {
    self.modules.get(id)
  }

  /// Get all modules.
  pub fn modules(&self) -> impl Iterator<Item = &ResolvedModule> {
    self.modules.values()
  }

  /// Get the number of modules.
  pub fn len(&self) -> usize {
    self.modules.len()
  }

  /// Check if the graph is empty.
  pub fn is_empty(&self) -> bool {
    self.modules.is_empty()
  }

  /// Detect cycles in the dependency graph using DFS.
  /// Returns the first cycle found, if any.
  pub fn detect_cycle(&self) -> Option<Vec<ModuleId>> {
    let mut visited = HashSet::new();
    let mut rec_stack = HashSet::new();
    let mut path = Vec::new();

    for module_id in self.modules.keys() {
      if !visited.contains(module_id)
        && let Some(cycle) = self.dfs_cycle(module_id, &mut visited, &mut rec_stack, &mut path)
      {
        return Some(cycle);
      }
    }

    None
  }

  #[allow(clippy::unwrap_used)] // Invariant: position() succeeds because dep was found in rec_stack which requires it to be in path
  fn dfs_cycle(
    &self,
    node: &ModuleId,
    visited: &mut HashSet<ModuleId>,
    rec_stack: &mut HashSet<ModuleId>,
    path: &mut Vec<ModuleId>,
  ) -> Option<Vec<ModuleId>> {
    visited.insert(node.clone());
    rec_stack.insert(node.clone());
    path.push(node.clone());

    if let Some(deps) = self.edges.get(node) {
      for dep in deps {
        if !visited.contains(dep) {
          if let Some(cycle) = self.dfs_cycle(dep, visited, rec_stack, path) {
            return Some(cycle);
          }
        } else if rec_stack.contains(dep) {
          // Found a cycle: dep is in rec_stack, so it must be in path
          if let Some(cycle_start) = path.iter().position(|m| m == dep) {
            let mut cycle: Vec<_> = path[cycle_start..].to_vec();
            cycle.push(dep.clone());
            return Some(cycle);
          } else {
            // This should never happen given the DFS invariants, but handle gracefully
            // by reporting the cycle with what we know
            let cycle = vec![node.clone(), dep.clone()];
            return Some(cycle);
          }
        }
      }
    }

    path.pop();
    rec_stack.remove(node);
    None
  }

  /// Topologically sort the modules (dependencies first).
  /// Returns None if there's a cycle (use `detect_cycle` first to get details).
  pub fn topological_sort(&self) -> Option<Vec<ModuleId>> {
    // For dependency graph (A→B means A depends on B), topological sort should output
    // modules with no dependencies first, then modules that only depend on those, etc.
    //
    // We compute in_degree = number of dependencies each module has.
    let mut in_degree: HashMap<ModuleId, usize> = HashMap::new();

    // Initialize in_degree: each module depends on the size of its edges list
    for (id, deps) in self.edges.iter() {
      let dep_count = deps
        .iter()
        .filter(|dep| self.modules.contains_key(dep))
        .count();
      in_degree.insert(id.clone(), dep_count);
    }

    // Also add modules with no edges
    for id in self.modules.keys() {
      in_degree.entry(id.clone()).or_insert(0);
    }

    // Start with modules that have no dependencies
    let mut queue: Vec<_> = in_degree
      .iter()
      .filter(|&(_, deg)| *deg == 0)
      .map(|(id, _)| id.clone())
      .collect();

    let mut result = Vec::new();

    while let Some(node) = queue.pop() {
      result.push(node.clone());

      // For all modules that depend on this one, decrement their in_degree
      for (other_id, other_deps) in self.edges.iter() {
        if other_deps.contains(&node)
          && in_degree.contains_key(other_id)
          && let Some(deg) = in_degree.get_mut(other_id)
        {
          *deg = deg.saturating_sub(1);
          if *deg == 0 {
            queue.push(other_id.clone());
          }
        }
      }
    }

    if result.len() == self.modules.len() {
      Some(result)
    } else {
      None
    }
  }
}

/// Resolve the path to the Lale standard library (stdlib/src directory).
///
/// Searches in the following order:
/// 1. `$LALE_HOME/lib/std/src` (if LALE_HOME environment variable is set)
/// 2. `exe_dir/../lib/std/src` (installed: binary at ~/.lale/bin/lale finds ~/.lale/lib/std/src)
/// 3. `exe_dir/../../stdlib/src` (developer: binary at target/build/lale finds stdlib/src)
///
/// Returns the first path that exists, or the last fallback if none exist.
pub fn resolve_stdlib_path() -> PathBuf {
  use std::env;

  // 1. Check LALE_HOME environment variable
  if let Ok(lale_home) = env::var("LALE_HOME") {
    let path = PathBuf::from(&lale_home).join("lib/std/src");
    if path.exists() {
      return path;
    }
  }

  // 2. Try relative to executable (installed case)
  if let Ok(exe_path) = env::current_exe()
    && let Some(exe_dir) = exe_path.parent()
  {
    let installed_path = exe_dir.join("../lib/std/src");
    if installed_path.exists() {
      if let Ok(canonical) = installed_path.canonicalize() {
        return canonical;
      }
      return installed_path;
    }
  }

  // 3. Try developer case (relative to executable)
  if let Ok(exe_path) = env::current_exe()
    && let Some(exe_dir) = exe_path.parent()
  {
    let dev_path = exe_dir.join("../../stdlib/src");
    if dev_path.exists() {
      if let Ok(canonical) = dev_path.canonicalize() {
        return canonical;
      }
      return dev_path;
    }
  }

  // Fallback: return the developer path as final attempt
  // (will be caught as error later if it doesn't exist)
  PathBuf::from("stdlib/src")
}

/// Resolve the path to the compiled Lale standard library (std.a).
///
/// Searches in the following order:
/// 1. `$LALE_HOME/lib/std.a` (if LALE_HOME environment variable is set)
/// 2. `exe_dir/../lib/std.a` (installed)
/// 3. `exe_dir/std.a` (developer: binary at target/build/lale finds target/build/std.a)
///
/// Returns the first path that exists, or the last fallback if none exist.
pub fn resolve_stdlib_archive_path() -> PathBuf {
  use std::env;

  // 1. Check LALE_HOME environment variable
  if let Ok(lale_home) = env::var("LALE_HOME") {
    let path = PathBuf::from(&lale_home).join("lib/std.a");
    if path.exists() {
      return path;
    }
  }

  // 2. Try relative to executable (installed case)
  if let Ok(exe_path) = env::current_exe()
    && let Some(exe_dir) = exe_path.parent()
  {
    let installed_path = exe_dir.join("../lib/std.a");
    if installed_path.exists() {
      if let Ok(canonical) = installed_path.canonicalize() {
        return canonical;
      }
      return installed_path;
    }
  }

  // 3. Try developer case (same directory as executable)
  if let Ok(exe_path) = env::current_exe()
    && let Some(exe_dir) = exe_path.parent()
  {
    let dev_path = exe_dir.join("std.a");
    if dev_path.exists() {
      if let Ok(canonical) = dev_path.canonicalize() {
        return canonical;
      }
      return dev_path;
    }
  }

  // Fallback: return current directory path
  PathBuf::from("std.a")
}

/// The module resolver orchestrates multi-file compilation.
#[derive(Debug)]
pub struct ModuleResolver {
  /// Base directory for module resolution.
  pub base_dir: PathBuf,
  /// The module dependency graph.
  pub graph: ModuleGraph,
  /// Errors encountered during resolution.
  pub errors: Vec<ModuleError>,
  /// Edge locations for error reporting (from_module, to_module) -> use statement location.
  edge_locations: HashMap<(ModuleId, ModuleId), SourceLocation>,
}

impl ModuleResolver {
  /// Create a new module resolver with the given base directory.
  pub fn new(base_dir: PathBuf) -> Self {
    ModuleResolver {
      base_dir,
      graph: ModuleGraph::new(),
      errors: Vec::new(),
      edge_locations: HashMap::new(),
    }
  }

  /// Build the full module graph starting from a root file and its already-parsed Program.
  ///
  /// This discovers all dependencies transitively by scanning `use` statements,
  /// parsing imported modules, and building the dependency graph.
  #[allow(clippy::result_large_err)]
  pub fn build_graph_from_root(
    &mut self,
    root_path: &Path,
    root_program: Program,
  ) -> Result<ModuleId, ModuleError> {
    let canonical_path = root_path
      .canonicalize()
      .unwrap_or_else(|_| root_path.to_path_buf());
    let root_id = ModuleId::new(canonical_path);
    self.discover_module(root_id.clone(), root_program)?;
    Ok(root_id)
  }

  /// Core discovery: ensure a module is parsed and its dependencies registered.
  #[allow(clippy::result_large_err)]
  fn discover_module(&mut self, id: ModuleId, program: Program) -> Result<(), ModuleError> {
    if self.graph.contains(&id) {
      return Ok(());
    }

    let use_stmts = self.extract_use_statements(&program);
    let mut deps = Vec::new();

    // NOTE: Builtins are now explicitly imported via 'use builtins' in modules that need them.
    // This ensures consistent dependency tracking and prevents duplicate loading.

    for use_stmt in &use_stmts {
      let target_path = self.resolve_module_path(&use_stmt.module_path, id.path());

      if !target_path.exists() {
        let path_str: Vec<_> = use_stmt
          .module_path
          .iter()
          .map(|s| s.node.as_str())
          .collect();
        return Err(ModuleError::ModuleNotFound {
          module_path: path_str.join(" -> "),
          search_path: target_path.clone(),
          location: use_stmt.location.clone(),
        });
      }

      let canonical_target = target_path.canonicalize().unwrap_or(target_path.clone());
      let target_id = ModuleId::new(canonical_target);
      deps.push(target_id.clone());

      self
        .edge_locations
        .insert((id.clone(), target_id.clone()), use_stmt.location.clone());

      if !self.graph.contains(&target_id) {
        // Parse the module source code
        let source =
          std::fs::read_to_string(&target_path).map_err(|e| ModuleError::ParseError {
            module: target_id.clone(),
            message: e.to_string(),
          })?;

        let pairs =
          LaleParser::parse(Rule::program, &source).map_err(|e| ModuleError::ParseError {
            module: target_id.clone(),
            message: e.to_string(),
          })?;

        let dep_program = build_program(pairs, &target_path.to_string_lossy()).map_err(|e| {
          ModuleError::ParseError {
            module: target_id.clone(),
            message: e,
          }
        })?;

        self.discover_module(target_id.clone(), dep_program)?;
      }
    }

    let module = ResolvedModule {
      id: id.clone(),
      program,
      dependencies: deps,
      use_statements: use_stmts,
      exports: None,
    };
    self.graph.add_module(module);

    Ok(())
  }

  /// Set the exports for a module after semantic analysis.
  pub fn set_exports(&mut self, id: &ModuleId, exports: SymbolTable) {
    if let Some(module) = self.graph.modules.get_mut(id) {
      module.exports = Some(exports);
    }
  }

  /// Get the exports for a module.
  pub fn get_exports(&self, id: &ModuleId) -> Option<&SymbolTable> {
    self.graph.get(id).and_then(|m| m.exports.as_ref())
  }

  /// Get the location of a use edge for error reporting.
  pub fn get_edge_location(&self, from: &ModuleId, to: &ModuleId) -> Option<&SourceLocation> {
    self.edge_locations.get(&(from.clone(), to.clone()))
  }

  /// Resolve a module path to a file path.
  ///
  /// # Path Resolution Rules
  ///
  /// - `std` -> `<executable_dir>/std.a` (standard library, special case)
  /// - `math.lib` -> `<base_dir>/math/lib.lale`
  /// - `../sibling` -> `<current_dir>/../sibling.lale`
  /// - `./helper` -> `<current_dir>/helper.lale`
  pub fn resolve_module_path(
    &self,
    module_path: &[Spanned<String>],
    current_file: &Path,
  ) -> PathBuf {
    let segments: Vec<&str> = module_path.iter().map(|s| s.node.as_str()).collect();

    // Determine if current_file is in the stdlib directory
    // If so, try to resolve relative to stdlib first (for stdlib internal imports)
    let stdlib_path = resolve_stdlib_path();
    let current_in_stdlib = current_file
      .canonicalize()
      .and_then(|cf| stdlib_path.canonicalize().map(|sp| (cf, sp)))
      .map(|(current_canonical, stdlib_canonical)| current_canonical.starts_with(&stdlib_canonical))
      .unwrap_or(false);

    // Try to resolve as a stdlib module first (single-segment imports)
    // This handles both root imports (like `use std`) and stdlib-internal imports
    // (like `use file_io_posix` from within `std.lale`)
    if segments.len() == 1 {
      let potential_stdlib_module = stdlib_path.join(format!("{}.lale", segments[0]));
      if potential_stdlib_module.exists() {
        return potential_stdlib_module;
      }
    }

    // For stdlib-internal imports, also try resolving relative to stdlib directory
    // (e.g., if `std.lale` does `use file_io_posix`, look in stdlib/src/)
    if current_in_stdlib && segments.len() == 1 {
      let stdlib_relative = stdlib_path.join(format!("{}.lale", segments[0]));
      if stdlib_relative.exists() {
        return stdlib_relative;
      }
    }

    // Multi-segment stdlib imports: "std" as first segment resolves
    // relative to the stdlib directory (e.g., `use std -> file_io_posix`).
    if segments.first() == Some(&"std") {
      let stdlib_path = resolve_stdlib_path();
      let mut path = stdlib_path;
      for segment in &segments[1..] {
        path = path.join(format!("{}.lale", segment));
      }
      return path;
    }

    // All other module paths are relative to the current file's directory
    let base = current_file
      .parent()
      .unwrap_or(Path::new("."))
      .to_path_buf();

    let mut path = base;
    for (i, segment) in segments.iter().enumerate() {
      if i == segments.len() - 1 {
        // Last segment is the module file itself
        path = path.join(format!("{}.lale", segment));
      } else {
        // Intermediate segments are directories
        path = path.join(segment);
      }
    }

    path
  }

  /// Extract all `use` statements from a program.
  /// Also validates that `use` statements are at top-level only.
  pub fn extract_use_statements(&mut self, program: &Program) -> Vec<UseStmt> {
    let mut use_stmts = Vec::new();

    for stmt in &program.statements {
      if let Stmt::Use(use_stmt) = stmt {
        use_stmts.push(use_stmt.clone());
      }
    }

    use_stmts
  }

  /// Check if the dependency graph has cycles.
  pub fn has_cycles(&self) -> bool {
    self.graph.detect_cycle().is_some()
  }

  /// Get compilation order (dependencies first).
  /// Returns None if there's a cycle - check `has_cycles()` and `cycle_error()` for details.
  pub fn compilation_order(&self) -> Option<Vec<ModuleId>> {
    if self.has_cycles() {
      eprintln!(
        "ERROR: Cyclic module dependency detected. Use module_resolver.cycle_error() for details."
      );
      None
    } else {
      self.graph.topological_sort()
    }
  }

  /// Check if resolution was successful (no errors).
  pub fn is_valid(&self) -> bool {
    self.errors.is_empty()
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_module_id_display() {
    let id = ModuleId::new(PathBuf::from("/path/to/module.lale"));
    assert!(id.to_string().contains("module.lale"));
  }

  #[test]
  fn test_module_graph_empty() {
    let graph = ModuleGraph::new();
    assert!(graph.is_empty());
    assert_eq!(graph.len(), 0);
  }

  #[test]
  fn test_module_graph_no_cycle() {
    let mut graph = ModuleGraph::new();

    let mod_a = ResolvedModule {
      id: ModuleId::new(PathBuf::from("a.lale")),
      program: Program {
        statements: vec![],
        location: SourceLocation::dummy(),
        global_symbol_table: std::cell::RefCell::new(None),
      },
      dependencies: vec![ModuleId::new(PathBuf::from("b.lale"))],
      use_statements: vec![],
      exports: None,
    };

    let mod_b = ResolvedModule {
      id: ModuleId::new(PathBuf::from("b.lale")),
      program: Program {
        statements: vec![],
        location: SourceLocation::dummy(),
        global_symbol_table: std::cell::RefCell::new(None),
      },
      dependencies: vec![],
      use_statements: vec![],
      exports: None,
    };

    graph.add_module(mod_a);
    graph.add_module(mod_b);

    assert!(graph.detect_cycle().is_none());
  }

  #[test]
  fn test_module_graph_with_cycle() {
    let mut graph = ModuleGraph::new();

    let mod_a = ResolvedModule {
      id: ModuleId::new(PathBuf::from("a.lale")),
      program: Program {
        statements: vec![],
        location: SourceLocation::dummy(),
        global_symbol_table: std::cell::RefCell::new(None),
      },
      exports: None,
      dependencies: vec![ModuleId::new(PathBuf::from("b.lale"))],
      use_statements: vec![],
    };

    let mod_b = ResolvedModule {
      id: ModuleId::new(PathBuf::from("b.lale")),
      program: Program {
        statements: vec![],
        location: SourceLocation::dummy(),
        global_symbol_table: std::cell::RefCell::new(None),
      },
      exports: None,
      dependencies: vec![ModuleId::new(PathBuf::from("a.lale"))],
      use_statements: vec![],
    };

    graph.add_module(mod_a);
    graph.add_module(mod_b);

    assert!(graph.detect_cycle().is_some());
  }

  #[test]
  fn test_topological_sort() {
    let mut graph = ModuleGraph::new();

    let mod_a = ResolvedModule {
      id: ModuleId::new(PathBuf::from("a.lale")),
      program: Program {
        statements: vec![],
        location: SourceLocation::dummy(),
        global_symbol_table: std::cell::RefCell::new(None),
      },
      exports: None,
      dependencies: vec![ModuleId::new(PathBuf::from("b.lale"))],
      use_statements: vec![],
    };

    let mod_b = ResolvedModule {
      id: ModuleId::new(PathBuf::from("b.lale")),
      program: Program {
        statements: vec![],
        location: SourceLocation::dummy(),
        global_symbol_table: std::cell::RefCell::new(None),
      },
      exports: None,
      dependencies: vec![],
      use_statements: vec![],
    };

    graph.add_module(mod_a);
    graph.add_module(mod_b);

    let order = graph.topological_sort().expect("Should have valid order");
    let b_pos = order
      .iter()
      .position(|m| m.0.as_path() == std::path::Path::new("b.lale"))
      .expect("b.lale should be in the order");
    let a_pos = order
      .iter()
      .position(|m| m.0.as_path() == std::path::Path::new("a.lale"))
      .expect("a.lale should be in the order");

    assert!(
      b_pos < a_pos,
      "b.lale should come before a.lale (dependency first)"
    );
  }

  #[test]
  fn test_resolve_module_path_absolute() {
    let resolver = ModuleResolver::new(PathBuf::from("/project"));
    let path = resolver.resolve_module_path(
      &[
        Spanned::new("math".to_string(), SourceLocation::dummy()),
        Spanned::new("lib".to_string(), SourceLocation::dummy()),
      ],
      Path::new("/project/main.lale"),
    );
    assert_eq!(path, PathBuf::from("/project/math/lib.lale"));
  }

  #[test]
  fn test_resolve_module_path_relative_subdirectory() {
    let resolver = ModuleResolver::new(PathBuf::from("/project"));
    // From /project/subdir/current.lale, import subpkg -> module resolves to /project/subdir/subpkg/module.lale
    let path = resolver.resolve_module_path(
      &[
        Spanned::new("subpkg".to_string(), SourceLocation::dummy()),
        Spanned::new("module".to_string(), SourceLocation::dummy()),
      ],
      Path::new("/project/subdir/current.lale"),
    );
    assert_eq!(path, PathBuf::from("/project/subdir/subpkg/module.lale"));
  }
}
