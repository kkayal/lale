//! SQLite-Backed Symbol Table Management
//!
//! This module provides the canonical symbol table implementation using an
//! in-memory SQLite database as the storage backend.
//!
//! # Benefits
//!
//! - **Declarative queries**: SQL is self-documenting
//! - **Easier debugging**: Query the symbol table directly during development
//! - **Natural fit for modules**: Project-wide symbol graph maps to relational tables
//!
//! # Quick Navigation
//!
//!   line  66 — `TypeDefInfo` struct definition
//!   line  75 — `SqliteSymbolManager` struct definition
//!   line  94 — Constructor & initialization (`new`, `init_schema`, `migrate_schema`)
//!   line 512 — Symbol table queries (`get_symbol_table`, `get_all_symbol_tables`)
//!   line 989 — Scope management (`current_scope`)
//!   line1052 — Symbol operations (`define_symbol`, `define_function`, `define_type`, `define_type_with_fields`)
//!   line1277 — Variable operations (`define_variable`, `lookup_var_type`, `lookup_var_symbol`, `lookup_var_unit`, `update_symbol_unit`, `is_variable_defined`, `is_local_variable`)
//!   line1746 — Type operations (`lookup_type`, `lookup_type_fields`)
//!   line2108 — Function operations (`enter_function`, `exit_function`, `lookup_function`, `fn_info_cache`)
//!   line2218 — Module tracking (`register_module`, `set_module_path`, `get_module_path`, `module_exists`)
//!   line2611 — Test module (`#[cfg(test)] mod tests`)

use crate::ast::*;
use rusqlite::{Connection, params};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

use super::const_value::ConstValue;
use super::type_compatibility::TypeInference;
pub use crate::ast::{
  ExprUnit, Linkage, StorageClass, Symbol, SymbolKind, SymbolTable, Visibility,
};

/// Function scope key used to identify function scopes in symbol tables.
#[derive(Eq, PartialEq, Debug, Hash, Clone)]
pub struct FnScopeKey {
  pub name: String,
  pub param_types: Vec<String>,
  pub return_type: String,
}

/// Scope identifier for symbol table lookup.
#[derive(PartialEq, Eq, Debug, Hash, Clone)]
pub enum VarScope {
  /// The module-level global scope.
  Global,
  /// A function's local scope, identified by its scope key.
  Function { scope_key: FnScopeKey },
  /// A test suite's scope: suite-level variables shared by all of its cases.
  /// Module-level (global) variables are NOT visible from this scope.
  Suite { suite_name: String },
}

/// Information about a function for unit propagation.
/// This is stored separately from Symbol to hold AST references needed during analysis.
#[derive(Clone)]
pub struct FnInfo {
  pub parameters: Vec<Parameter>,
  pub return_unit: Option<Unit>,
  pub body: Vec<Stmt>,
}

/// A single field in a type definition: (name, type, optional-unit, is_private).
pub type TypeFieldInfo = (String, String, Option<String>, bool);

/// Information about a type definition.
#[derive(Clone)]
pub struct TypeDefInfo {
  pub name: String,
  pub location: SourceLocation,
  pub fields: Vec<TypeFieldInfo>,
  /// Module path where this type was defined (for private field visibility checks).
  pub module_path: String,
}

/// SQLite-backed symbol management
pub struct SqliteSymbolManager {
  conn: Connection,
  current_scope: VarScope,
  /// Name of the enclosing test suite while analyzing a suite or its cases.
  /// `None` outside test suites. Used so a case scope falls back to its suite
  /// scope (not to the global scope), making module-level variables invisible.
  current_suite: Option<String>,
  /// Current module path (empty string for current file)
  current_module_path: String,
  /// Root/main file path (for relative path calculation)
  root_file_path: Option<PathBuf>,
  /// In-memory cache of FnInfo for functions.
  /// Key: function name (simple, not qualified)
  ///
  /// The cache exists because `FnInfo.body` (`Vec<Stmt>`) is an arbitrary AST tree
  /// that can't be practically serialized to SQLite. The unit analyzer needs the body
  /// to trace return expression units. Parameters and return_unit are persisted in the
  /// database and reconstructed on demand when the cache misses.
  fn_info_cache: HashMap<String, FnInfo>,
}

impl SqliteSymbolManager {
  /// Create a new SQLite symbol manager with in-memory database.
  #[allow(clippy::expect_used)]
  pub fn new() -> Self {
    let conn = Connection::open_in_memory()
      .expect("Invariant: in-memory SQLite database should always be creatable");

    let manager = SqliteSymbolManager {
      conn,
      current_scope: VarScope::Global,
      current_suite: None,
      current_module_path: String::new(),
      root_file_path: None,
      fn_info_cache: HashMap::new(),
    };

    manager.init_schema();
    manager.migrate_schema();
    manager
  }

  /// Get a reference to the SQLite connection for advanced queries.
  /// This is used by SymbolTableAdapter and other internal components.
  pub fn get_connection(&self) -> &Connection {
    &self.conn
  }

  /// Export the symbol database to a file using SQLite's VACUUM INTO command.
  /// This creates a complete, vacuumed copy of the database that can be opened
  /// with any SQLite client.
  ///
  /// # Arguments
  /// * `path` - Destination file path for the database export
  ///
  /// # Returns
  /// * `Ok(())` - Successfully exported
  /// * `Err(message)` - Export failed (invalid path, permissions, disk space, etc.)
  ///
  /// # Notes
  /// - The database must not be modified during export (we only export after analysis completes)
  /// - The exported file is a valid SQLite database and can be opened with `sqlite3` CLI
  /// - Requires SQLite 3.27.0+ (available since 2019, ubiquitous)
  pub fn export_database(&self, path: &Path) -> Result<(), String> {
    // Format path for SQL injection safety and proper quoting
    let path_str = path
      .to_str()
      .ok_or_else(|| "Export path contains invalid UTF-8".to_string())?;

    // Remove existing file if it exists (like compilers overwriting object files)
    if path.exists() {
      std::fs::remove_file(path).map_err(|e| {
        format!(
          "Failed to remove existing export file '{}': {}",
          path.display(),
          e
        )
      })?;
    }

    // Use VACUUM INTO to create a vacuumed copy of the database
    let sql = format!("VACUUM INTO '{}'", path_str.replace('\'', "''"));
    self
      .conn
      .execute_batch(&sql)
      .map_err(|e| format!("Failed to export database: {}", e))?;

    Ok(())
  }

  /// Initialize the database schema for the.
  #[allow(clippy::expect_used)]
  fn init_schema(&self) {
    self
      .conn
      .execute_batch(
        r#"
-- Module definitions
CREATE TABLE IF NOT EXISTS modules (
  path TEXT PRIMARY KEY,
  base_dir TEXT NOT NULL,
  is_parsed BOOLEAN NOT NULL DEFAULT 0,
  is_analyzed BOOLEAN NOT NULL DEFAULT 0
);

-- Function definitions
-- qualified_name = simple_name + "_" + param_types.join("_") (C convention, NO return type)
CREATE TABLE IF NOT EXISTS functions (
  qualified_name TEXT PRIMARY KEY,
  simple_name TEXT NOT NULL,
  return_type TEXT NOT NULL,
  return_unit TEXT,
  module_path TEXT NOT NULL,
  line_number INTEGER NOT NULL DEFAULT 1,
  col_number INTEGER NOT NULL DEFAULT 1,
  start_pos INTEGER NOT NULL DEFAULT 0,
  end_pos INTEGER NOT NULL DEFAULT 0,
  is_export BOOLEAN NOT NULL DEFAULT 0,
  linkage TEXT NOT NULL DEFAULT 'Internal',
  visibility TEXT NOT NULL DEFAULT 'Private',
  storage_class TEXT NOT NULL DEFAULT 'Auto',
  is_definition BOOLEAN NOT NULL DEFAULT 1,
  FOREIGN KEY (module_path) REFERENCES modules(path),
  UNIQUE (module_path, simple_name, return_type)
);

-- Variables with composite key: (function_qualified_name, simple_name)
-- function_qualified_name IS NULL for global variables
CREATE TABLE IF NOT EXISTS variables (
  function_qualified_name TEXT,
  simple_name TEXT NOT NULL,
  var_type TEXT NOT NULL,
  physical_unit TEXT,
  module_path TEXT NOT NULL,
  line_number INTEGER NOT NULL DEFAULT 1,
  col_number INTEGER NOT NULL DEFAULT 1,
  start_pos INTEGER NOT NULL DEFAULT 0,
  end_pos INTEGER NOT NULL DEFAULT 0,
  is_global BOOLEAN NOT NULL DEFAULT 0,
  is_parameter BOOLEAN NOT NULL DEFAULT 0,
  param_index INTEGER,
  pass_mode TEXT NOT NULL DEFAULT 'copy',
  linkage TEXT NOT NULL DEFAULT 'Internal',
  visibility TEXT NOT NULL DEFAULT 'Private',
  storage_class TEXT NOT NULL DEFAULT 'Auto',
  is_definition BOOLEAN NOT NULL DEFAULT 1,
  is_initialized BOOLEAN NOT NULL DEFAULT 0,
  pointer_to_type TEXT,
  -- Constant-value tracking (single source of truth; see lookup_variable_const_value).
  --   is_shared     = FALSE → the compiler has exclusive knowledge of every write
  --                           (the only trustworthy state); TRUE → the storage is
  --                           shared with a pointer, a `ref` callee, or another
  --                           module (sticky: once TRUE, always TRUE).
  --   is_reassigned = TRUE once the variable has been assigned with `=` / `+=` etc.
  --                           (sticky: once TRUE, always TRUE). Used to distinguish
  --                           "never reassigned" (safe to fold at any use site)
  --                           from "reassigned" (only its final value is known).
  --   const_value   = the current known constant value, NULL when unknown.
  --                   Meaningful only while is_shared = FALSE. Do NOT read it raw
  --                   to test for a constant — use the variable_constants view or
  --                   lookup_variable_const_value().
  -- Aggregate values (structs, enums, arrays, optionals, vectors) are
  -- intentionally NOT tracked here: no consumer needs aggregate constants yet,
  -- and supporting them would require a recursive ConstValue, recursive literal
  -- folding, and a JSON-style encoding. See ARCHITECTURE.md §4.5.7a.
  is_shared BOOLEAN NOT NULL DEFAULT FALSE,
  is_reassigned BOOLEAN NOT NULL DEFAULT FALSE,
  const_value TEXT,
  PRIMARY KEY (function_qualified_name, simple_name),
  FOREIGN KEY (function_qualified_name) REFERENCES functions(qualified_name),
  FOREIGN KEY (module_path) REFERENCES modules(path)
);

-- Derived view: which variables are compile-time constants. A VIEW (not a column)
-- keeps the base table normalized while giving maintainers a single, documented
-- place to see both notions computed from `is_shared` + `is_reassigned` +
-- `const_value`:
--   is_constant             = the value is known *at the current point* of the
--                             semantic-analysis walk (flow-sensitive; used by
--                             div-zero / underflow detection).
--   is_effectively_constant = the variable is a true program-wide constant
--                             (never reassigned, never shared; used by IR
--                             constant propagation).
CREATE VIEW IF NOT EXISTS variable_constants AS
SELECT
  function_qualified_name,
  simple_name,
  is_global,
  is_shared,
  is_reassigned,
  const_value,
  (NOT is_shared AND const_value IS NOT NULL) AS is_constant,
  (NOT is_shared AND NOT is_reassigned AND const_value IS NOT NULL) AS is_effectively_constant
FROM variables;

-- Type definitions
CREATE TABLE IF NOT EXISTS type_defs (
  qualified_name TEXT PRIMARY KEY,
  module_path TEXT NOT NULL,
  line_number INTEGER NOT NULL DEFAULT 1,
  col_number INTEGER NOT NULL DEFAULT 1,
  start_pos INTEGER NOT NULL DEFAULT 0,
  end_pos INTEGER NOT NULL DEFAULT 0,
  is_export BOOLEAN NOT NULL DEFAULT 0,
  FOREIGN KEY (module_path) REFERENCES modules(path)
);

-- Type definition fields
CREATE TABLE IF NOT EXISTS type_def_fields (
  type_def_qualified_name TEXT NOT NULL,
  field_index INTEGER NOT NULL,
  field_name TEXT NOT NULL,
  field_type TEXT NOT NULL,
  physical_unit TEXT,
  is_private BOOLEAN NOT NULL DEFAULT 0,
  PRIMARY KEY (type_def_qualified_name, field_name),
  FOREIGN KEY (type_def_qualified_name) REFERENCES type_defs(qualified_name)
);

-- Indices for performance
CREATE INDEX IF NOT EXISTS idx_functions_simple_name ON functions(simple_name);
CREATE INDEX IF NOT EXISTS idx_functions_module ON functions(module_path);
CREATE INDEX IF NOT EXISTS idx_variables_function ON variables(function_qualified_name);
CREATE INDEX IF NOT EXISTS idx_variables_module ON variables(module_path);
CREATE INDEX IF NOT EXISTS idx_variables_simple_name ON variables(simple_name);
CREATE INDEX IF NOT EXISTS idx_type_defs_module ON type_defs(module_path);
CREATE INDEX IF NOT EXISTS idx_type_def_fields_type ON type_def_fields(type_def_qualified_name);

-- Key-value metadata store for analyzer state (avoids in-memory redundancy)
CREATE TABLE IF NOT EXISTS metadata (
  key TEXT PRIMARY KEY,
  value TEXT NOT NULL
);
    "#,
      )
      .unwrap_or_else(|e| {
        eprintln!(
          "ERROR: Failed to initialize SQLite schema: {}. \
           This may indicate database corruption or permission issues.",
          e
        );
      });
  }

  /// Convert VarScope to SQLite key components.
  #[allow(dead_code)]
  fn scope_to_key(scope: &VarScope) -> (bool, String) {
    match scope {
      VarScope::Global => (true, String::new()),
      VarScope::Function { scope_key } => {
        let fn_id = format!("{}:{}", scope_key.name, scope_key.param_types.join(","));
        (false, fn_id)
      }
      VarScope::Suite { suite_name } => (false, suite_name.clone()),
    }
  }

  /// Convert SymbolKind to string for storage.
  #[allow(dead_code)]
  fn kind_to_str(kind: &SymbolKind) -> &'static str {
    match kind {
      SymbolKind::Variable => "variable",
      SymbolKind::Parameter => "parameter",
      SymbolKind::Function => "function",
      SymbolKind::TypeDef => "typedef",
    }
  }

  /// Convert string to SymbolKind.
  #[allow(dead_code)]
  fn str_to_kind(s: &str) -> SymbolKind {
    match s {
      "function" => SymbolKind::Function,
      "parameter" => SymbolKind::Parameter,
      _ => SymbolKind::Variable,
    }
  }

  /// Add columns that were added after the initial schema.
  /// ALTER TABLE ADD COLUMN is silently ignored if the column already exists.
  fn migrate_schema(&self) {
    let migrations = [
      "ALTER TABLE functions ADD COLUMN start_pos INTEGER DEFAULT 0",
      "ALTER TABLE functions ADD COLUMN end_pos INTEGER DEFAULT 0",
      "ALTER TABLE variables ADD COLUMN start_pos INTEGER DEFAULT 0",
      "ALTER TABLE variables ADD COLUMN end_pos INTEGER DEFAULT 0",
      "ALTER TABLE variables ADD COLUMN pass_mode TEXT NOT NULL DEFAULT 'copy'",
      "ALTER TABLE variables ADD COLUMN is_shared BOOLEAN NOT NULL DEFAULT FALSE",
      "ALTER TABLE variables ADD COLUMN is_reassigned BOOLEAN NOT NULL DEFAULT FALSE",
      "ALTER TABLE variables ADD COLUMN const_value TEXT",
      "ALTER TABLE type_defs ADD COLUMN start_pos INTEGER DEFAULT 0",
      "ALTER TABLE type_defs ADD COLUMN end_pos INTEGER DEFAULT 0",
    ];
    for m in migrations {
      // Migrations are idempotent and best-effort: on a freshly created in-memory
      // database the columns already exist, so "duplicate column" errors are
      // expected and harmless. Genuine failures are intentionally non-fatal here.
      let _ = self.conn.execute(m, []);
    }
  }

  /// Convert Linkage to string.
  fn linkage_to_str(linkage: &Linkage) -> &'static str {
    match linkage {
      Linkage::Internal => "internal",
      Linkage::Import => "import",
      Linkage::Export => "export",
    }
  }

  /// Convert string to Linkage.
  fn str_to_linkage(s: &str) -> Linkage {
    match s {
      "import" => Linkage::Import,
      "export" => Linkage::Export,
      _ => Linkage::Internal,
    }
  }

  /// Convert Visibility to string.
  fn visibility_to_str(visibility: &Visibility) -> &'static str {
    match visibility {
      Visibility::Private => "private",
      Visibility::Public => "public",
    }
  }

  /// Convert string to Visibility.
  fn str_to_visibility(s: &str) -> Visibility {
    match s {
      "public" => Visibility::Public,
      _ => Visibility::Private,
    }
  }

  /// Convert StorageClass to string.
  fn storage_class_to_str(sc: &StorageClass) -> &'static str {
    match sc {
      StorageClass::Default => "default",
      StorageClass::ThreadLocal => "thread_local",
    }
  }

  /// Convert string to StorageClass.
  fn str_to_storage_class(s: &str) -> StorageClass {
    match s {
      "thread_local" => StorageClass::ThreadLocal,
      _ => StorageClass::Default,
    }
  }

  /// Construct a qualified function name from simple name and parameter types.
  /// Pattern: simple_name + "_" + param_types.join("_")
  /// If no parameters, just returns simple_name.
  fn make_qualified_function_name(simple_name: &str, param_types: &[String]) -> String {
    if param_types.is_empty() {
      simple_name.to_string()
    } else {
      format!("{}_{}", simple_name, param_types.join("_"))
    }
  }

  /// Returns all symbol tables (global + function scopes).
  /// Reconstructs HashMap structure for compatibility with existing API.
  pub fn get_all_symbol_tables(&self) -> Result<HashMap<VarScope, SymbolTable>, String> {
    let mut result: HashMap<VarScope, SymbolTable> = HashMap::new();

    // Get all global variables
    let mut stmt = self
      .conn
      .prepare(
        "SELECT simple_name, var_type, physical_unit, linkage, visibility, storage_class,
               is_definition, is_initialized, pointer_to_type, line_number, col_number,
               start_pos, end_pos, module_path
         FROM variables WHERE is_global = 1",
      )
      .map_err(|e| format!("Failed to prepare global variables query: {}", e))?;

    let mut global_symbols: SymbolTable = HashMap::new();
    let rows = stmt
      .query_map([], |row| {
        Ok((
          row.get::<_, String>(0)?,
          row.get::<_, String>(1)?,
          row.get::<_, Option<String>>(2)?,
          row.get::<_, String>(3)?,
          row.get::<_, String>(4)?,
          row.get::<_, String>(5)?,
          row.get::<_, bool>(6)?,
          row.get::<_, bool>(7)?,
          row.get::<_, Option<String>>(8)?,
          row.get::<_, i64>(9)?,
          row.get::<_, i64>(10)?,
          row.get::<_, i64>(11)?,
          row.get::<_, i64>(12)?,
          row.get::<_, String>(13)?,
        ))
      })
      .map_err(|e| format!("Failed to query global variables: {}", e))?;

    for result in rows {
      let (
        name,
        var_type,
        phys_unit,
        linkage,
        visibility,
        storage_class,
        is_def,
        is_init,
        ptr_type,
        line,
        col,
        start_pos,
        end_pos,
        module_path,
      ) = result.map_err(|e| format!("Failed to read global variable row: {}", e))?;
      let symbol = Symbol {
        source_location: SourceLocation {
          line: line as usize,
          col: col as usize,
          start_pos: start_pos as usize,
          end_pos: end_pos as usize,
          source_file: "<unknown>".to_string(),
        },
        data_type: var_type,
        physical_unit: phys_unit,
        kind: SymbolKind::Variable,
        linkage: Self::str_to_linkage(&linkage),
        visibility: Self::str_to_visibility(&visibility),
        storage_class: Self::str_to_storage_class(&storage_class),
        is_definition: is_def,
        is_initialized: is_init,
        pointer_to_type: ptr_type,
        module_path,
      };
      global_symbols.insert(name, symbol);
    }

    // Get all global functions and add them to global scope
    let mut stmt = self
      .conn
      .prepare(
        "SELECT simple_name, return_type, linkage, visibility, storage_class,
               is_definition, line_number, col_number, start_pos, end_pos, module_path
         FROM functions",
      )
      .map_err(|e| format!("Failed to prepare functions query: {}", e))?;

    let rows = stmt
      .query_map([], |row| {
        Ok((
          row.get::<_, String>(0)?,
          row.get::<_, String>(1)?,
          row.get::<_, String>(2)?,
          row.get::<_, String>(3)?,
          row.get::<_, String>(4)?,
          row.get::<_, bool>(5)?,
          row.get::<_, i64>(6)?,
          row.get::<_, i64>(7)?,
          row.get::<_, i64>(8)?,
          row.get::<_, i64>(9)?,
          row.get::<_, String>(10)?,
        ))
      })
      .map_err(|e| format!("Failed to query functions: {}", e))?;

    for result in rows {
      let (
        name,
        return_type,
        linkage,
        visibility,
        storage_class,
        is_def,
        line,
        col,
        start_pos,
        end_pos,
        module_path,
      ) = result.map_err(|e| format!("Failed to read function row: {}", e))?;
      let symbol = Symbol {
        source_location: SourceLocation {
          line: line as usize,
          col: col as usize,
          start_pos: start_pos as usize,
          end_pos: end_pos as usize,
          source_file: "<unknown>".to_string(),
        },
        data_type: return_type,
        physical_unit: None,
        kind: SymbolKind::Function,
        linkage: Self::str_to_linkage(&linkage),
        visibility: Self::str_to_visibility(&visibility),
        storage_class: Self::str_to_storage_class(&storage_class),
        is_definition: is_def,
        is_initialized: true, // Functions are always "initialized"
        pointer_to_type: None,
        module_path,
      };
      global_symbols.insert(name, symbol);
    }

    // Get all type definitions and add them to global scope
    let mut stmt = self
      .conn
      .prepare(
        "SELECT qualified_name, is_export, line_number, col_number, start_pos, end_pos, module_path
         FROM type_defs",
      )
      .map_err(|e| format!("Failed to prepare type definitions query: {}", e))?;

    let rows = stmt
      .query_map([], |row| {
        Ok((
          row.get::<_, String>(0)?,
          row.get::<_, bool>(1)?,
          row.get::<_, i64>(2)?,
          row.get::<_, i64>(3)?,
          row.get::<_, i64>(4)?,
          row.get::<_, i64>(5)?,
          row.get::<_, String>(6)?,
        ))
      })
      .map_err(|e| format!("Failed to query type definitions: {}", e))?;

    for result in rows {
      let (qualified_name, is_export, line, col, start_pos, end_pos, module_path) =
        result.map_err(|e| format!("Failed to read type definition row: {}", e))?;
      let symbol = Symbol {
        source_location: SourceLocation {
          line: line as usize,
          col: col as usize,
          start_pos: start_pos as usize,
          end_pos: end_pos as usize,
          source_file: "<unknown>".to_string(),
        },
        data_type: "composite".to_string(),
        physical_unit: None,
        kind: SymbolKind::TypeDef,
        linkage: if is_export {
          Linkage::Export
        } else {
          Linkage::Internal
        },
        visibility: Visibility::Public,
        storage_class: StorageClass::Default,
        is_definition: true,
        is_initialized: true, // Types are always "initialized"
        pointer_to_type: None,
        module_path,
      };
      global_symbols.insert(qualified_name, symbol);
    }

    result.insert(VarScope::Global, global_symbols);

    // Get all functions and their local variables
    let mut stmt = self
      .conn
      .prepare("SELECT qualified_name, simple_name, return_type FROM functions")
      .map_err(|e| format!("Failed to prepare functions scope query: {}", e))?;

    let functions: Vec<(String, String, String)> = stmt
      .query_map([], |row| {
        Ok((
          row.get::<_, String>(0)?, // qualified_name
          row.get::<_, String>(1)?, // simple_name
          row.get::<_, String>(2)?, // return_type
        ))
      })
      .map_err(|e| format!("Failed to query functions for scopes: {}", e))?
      .collect::<Result<Vec<_>, _>>()
      .map_err(|e| {
        format!(
          "Failed to decode a function row while rebuilding scopes: {}",
          e
        )
      })?;

    for (qualified_name, simple_name, return_type) in functions {
      let symbols = self.get_symbols_for_scope(&qualified_name)?;

      if !symbols.is_empty() {
        // Reconstruct parameter types from the qualified name suffix.
        // Qualified names use _ as separator between name and param types,
        // which is ambiguous for names containing underscores. We find the
        // boundary by looking for the simple_name prefix.
        let param_types: Vec<String> = if qualified_name.len() > simple_name.len() + 1 {
          qualified_name[simple_name.len() + 1..]
            .split('_')
            .map(|s| s.to_string())
            .collect()
        } else {
          Vec::new()
        };

        let scope = VarScope::Function {
          scope_key: FnScopeKey {
            name: simple_name,
            param_types,
            return_type,
          },
        };

        result.insert(scope, symbols);
      }
    }

    Ok(result)
  }

  /// Get symbols for a specific scope from the database.
  /// For global scope: pass empty string for function_qualified_name
  /// For function scope: pass the qualified_name
  fn get_symbols_for_scope(&self, function_qualified_name: &str) -> Result<SymbolTable, String> {
    let mut table = HashMap::new();

    // Determine if querying global or function scope
    let is_global = function_qualified_name.is_empty();

    if is_global {
      // Query global variables
      let mut stmt = self
        .conn
        .prepare(
          "SELECT simple_name, var_type, physical_unit, linkage, visibility,
                storage_class, is_definition, is_initialized, pointer_to_type,
                line_number, col_number, start_pos, end_pos, module_path, is_parameter
         FROM variables WHERE is_global = 1",
        )
        .map_err(|e| format!("Failed to prepare global variables query: {}", e))?;

      let rows = stmt
        .query_map([], |row| {
          Ok((
            row.get::<_, String>(0)?,
            row.get::<_, String>(1)?,
            row.get::<_, Option<String>>(2)?,
            row.get::<_, String>(3)?,
            row.get::<_, String>(4)?,
            row.get::<_, String>(5)?,
            row.get::<_, bool>(6)?,
            row.get::<_, bool>(7)?,
            row.get::<_, Option<String>>(8)?,
            row.get::<_, i64>(9)?,
            row.get::<_, i64>(10)?,
            row.get::<_, i64>(11)?,
            row.get::<_, i64>(12)?,
            row.get::<_, String>(13)?,
            row.get::<_, bool>(14)?,
          ))
        })
        .map_err(|e| format!("Failed to query global symbols: {}", e))?;

      for result in rows {
        let (
          name,
          var_type,
          physical_unit,
          linkage,
          visibility,
          storage_class,
          is_definition,
          is_initialized,
          pointer_to_type,
          source_line,
          source_col,
          start_pos,
          end_pos,
          module_path,
          is_parameter,
        ) = result.map_err(|e| format!("Failed to read global symbol row: {}", e))?;
        let symbol = Symbol {
          source_location: SourceLocation {
            line: source_line as usize,
            col: source_col as usize,
            start_pos: start_pos as usize,
            end_pos: end_pos as usize,
            source_file: "<unknown>".to_string(),
          },
          data_type: var_type,
          physical_unit,
          kind: if is_parameter {
            SymbolKind::Parameter
          } else {
            SymbolKind::Variable
          },
          linkage: Self::str_to_linkage(&linkage),
          visibility: Self::str_to_visibility(&visibility),
          storage_class: Self::str_to_storage_class(&storage_class),
          is_definition,
          is_initialized,
          pointer_to_type,
          module_path,
        };
        table.insert(name, symbol);
      }

      // Also query global functions and include them in the symbol table
      let mut fn_stmt = self
        .conn
        .prepare("SELECT simple_name, return_type, line_number, col_number, start_pos, end_pos, linkage, is_definition FROM functions")
        .map_err(|e| format!("Failed to prepare global functions query: {}", e))?;

      let fn_rows = fn_stmt
        .query_map([], |row| {
          Ok((
            row.get::<_, String>(0)?,
            row.get::<_, String>(1)?,
            row.get::<_, i64>(2)?,
            row.get::<_, i64>(3)?,
            row.get::<_, i64>(4)?,
            row.get::<_, i64>(5)?,
            row.get::<_, String>(6)?,
            row.get::<_, bool>(7)?,
          ))
        })
        .map_err(|e| format!("Failed to query global functions: {}", e))?;

      for fn_result in fn_rows {
        let (
          name,
          return_type,
          source_line,
          source_col,
          start_pos,
          end_pos,
          linkage,
          is_definition,
        ) = fn_result.map_err(|e| format!("Failed to read global function row: {}", e))?;
        let symbol = Symbol {
          source_location: SourceLocation {
            line: source_line as usize,
            col: source_col as usize,
            start_pos: start_pos as usize,
            end_pos: end_pos as usize,
            source_file: "<unknown>".to_string(),
          },
          data_type: return_type,
          physical_unit: None,
          kind: SymbolKind::Function,
          linkage: Self::str_to_linkage(&linkage),
          visibility: Visibility::default(),
          storage_class: StorageClass::Default,
          is_definition,
          is_initialized: is_definition,
          pointer_to_type: None,
          module_path: String::new(),
        };
        table.insert(name, symbol);
      }
    } else {
      // Query function-local variables
      let mut stmt = self
        .conn
        .prepare(
          "SELECT simple_name, var_type, physical_unit, linkage, visibility,
                storage_class, is_definition, is_initialized, pointer_to_type,
                line_number, col_number, start_pos, end_pos, module_path, is_parameter
         FROM variables WHERE function_qualified_name = ?1",
        )
        .map_err(|e| format!("Failed to prepare function variables query: {}", e))?;

      let rows = stmt
        .query_map(params![function_qualified_name], |row| {
          Ok((
            row.get::<_, String>(0)?,
            row.get::<_, String>(1)?,
            row.get::<_, Option<String>>(2)?,
            row.get::<_, String>(3)?,
            row.get::<_, String>(4)?,
            row.get::<_, String>(5)?,
            row.get::<_, bool>(6)?,
            row.get::<_, bool>(7)?,
            row.get::<_, Option<String>>(8)?,
            row.get::<_, i64>(9)?,
            row.get::<_, i64>(10)?,
            row.get::<_, i64>(11)?,
            row.get::<_, i64>(12)?,
            row.get::<_, String>(13)?,
            row.get::<_, bool>(14)?,
          ))
        })
        .map_err(|e| format!("Failed to query function-local symbols: {}", e))?;

      for result in rows {
        let (
          name,
          var_type,
          physical_unit,
          linkage,
          visibility,
          storage_class,
          is_definition,
          is_initialized,
          pointer_to_type,
          source_line,
          source_col,
          start_pos,
          end_pos,
          module_path,
          is_parameter,
        ) = result.map_err(|e| format!("Failed to read function-local symbol row: {}", e))?;
        let symbol = Symbol {
          source_location: SourceLocation {
            line: source_line as usize,
            col: source_col as usize,
            start_pos: start_pos as usize,
            end_pos: end_pos as usize,
            source_file: "<unknown>".to_string(),
          },
          data_type: var_type,
          physical_unit,
          kind: if is_parameter {
            SymbolKind::Parameter
          } else {
            SymbolKind::Variable
          },
          linkage: Self::str_to_linkage(&linkage),
          visibility: Self::str_to_visibility(&visibility),
          storage_class: Self::str_to_storage_class(&storage_class),
          is_definition,
          is_initialized,
          pointer_to_type,
          module_path,
        };
        table.insert(name, symbol);
      }
    }

    Ok(table)
  }

  /// Returns the symbol table for a specific scope.
  pub fn get_symbol_table(&self, scope: VarScope) -> SymbolTable {
    match scope {
      VarScope::Global => self.get_symbols_for_scope("").unwrap_or_default(),
      VarScope::Function { scope_key } => {
        let qualified_name =
          Self::make_qualified_function_name(&scope_key.name, &scope_key.param_types);
        self
          .get_symbols_for_scope(&qualified_name)
          .unwrap_or_default()
      }
      VarScope::Suite { suite_name } => self.get_symbols_for_scope(&suite_name).unwrap_or_default(),
    }
  }

  /// Return a module's exported symbols, queried directly from the database.
  ///
  /// `module_path` is the value stored in the `variables`/`functions`/
  /// `type_defs` tables for that module (the module file name, as set by
  /// `set_current_module_path`). This is the single source of truth for
  /// cross-module `use` resolution: the `ModuleResolver` no longer keeps a
  /// redundant in-memory copy of exports.
  pub fn get_exports(&self, module_path: &str) -> Result<SymbolTable, String> {
    let mut exports = SymbolTable::new();

    // Exported global variables. `simple_name` is appended after the standard
    // `var_row_to_symbol` projection so the name can be used as the map key.
    {
      let sql = "SELECT var_type, physical_unit, linkage, visibility, storage_class, \
                 is_definition, is_initialized, pointer_to_type, line_number, col_number, \
                 module_path, simple_name \
                 FROM variables WHERE is_global = 1 AND linkage = 'export' AND module_path = ?1";
      let mut stmt = self
        .conn
        .prepare_cached(sql)
        .map_err(|e| format!("Failed to prepare get_exports variables query: {}", e))?;
      let rows = stmt
        .query_map(params![module_path], |row| {
          let symbol = Self::var_row_to_symbol(row)?;
          let name: String = row.get(11)?;
          Ok((name, symbol))
        })
        .map_err(|e| format!("Failed to query exported variables: {}", e))?;
      for r in rows {
        let (name, symbol) = r.map_err(|e| format!("Failed to read exported variable: {}", e))?;
        exports.insert(name, symbol);
      }
    }

    // Exported functions.
    {
      let sql = "SELECT simple_name, return_type, linkage, visibility, storage_class, \
                 is_definition, line_number, col_number, module_path \
                 FROM functions WHERE linkage = 'export' AND module_path = ?1";
      let mut stmt = self
        .conn
        .prepare_cached(sql)
        .map_err(|e| format!("Failed to prepare get_exports functions query: {}", e))?;
      let rows = stmt
        .query_map(params![module_path], |row| {
          let name: String = row.get(0)?;
          let return_type: String = row.get(1)?;
          let linkage: String = row.get(2)?;
          let visibility: String = row.get(3)?;
          let storage_class: String = row.get(4)?;
          let is_definition: bool = row.get(5)?;
          let line: i64 = row.get(6)?;
          let col: i64 = row.get(7)?;
          let module_path: String = row.get(8)?;
          Ok((
            name,
            Symbol {
              source_location: SourceLocation {
                line: line as usize,
                col: col as usize,
                start_pos: 0,
                end_pos: 0,
                source_file: "<unknown>".to_string(),
              },
              data_type: return_type,
              physical_unit: None,
              kind: SymbolKind::Function,
              linkage: Self::str_to_linkage(&linkage),
              visibility: Self::str_to_visibility(&visibility),
              storage_class: Self::str_to_storage_class(&storage_class),
              is_definition,
              is_initialized: true,
              pointer_to_type: None,
              module_path,
            },
          ))
        })
        .map_err(|e| format!("Failed to query exported functions: {}", e))?;
      for r in rows {
        let (name, symbol) = r.map_err(|e| format!("Failed to read exported function: {}", e))?;
        exports.insert(name, symbol);
      }
    }

    // Exported type definitions.
    {
      let sql = "SELECT qualified_name, is_export, line_number, col_number, start_pos, end_pos, \
                 module_path FROM type_defs WHERE is_export = 1 AND module_path = ?1";
      let mut stmt = self
        .conn
        .prepare_cached(sql)
        .map_err(|e| format!("Failed to prepare get_exports type_defs query: {}", e))?;
      let rows = stmt
        .query_map(params![module_path], |row| {
          let qualified_name: String = row.get(0)?;
          let is_export: bool = row.get(1)?;
          let line: i64 = row.get(2)?;
          let col: i64 = row.get(3)?;
          let module_path: String = row.get(6)?;
          Ok((
            qualified_name,
            Symbol {
              source_location: SourceLocation {
                line: line as usize,
                col: col as usize,
                start_pos: 0,
                end_pos: 0,
                source_file: "<unknown>".to_string(),
              },
              data_type: "composite".to_string(),
              physical_unit: None,
              kind: SymbolKind::TypeDef,
              linkage: if is_export {
                Linkage::Export
              } else {
                Linkage::Internal
              },
              visibility: Visibility::Public,
              storage_class: StorageClass::Default,
              is_definition: true,
              is_initialized: true,
              pointer_to_type: None,
              module_path,
            },
          ))
        })
        .map_err(|e| format!("Failed to query exported type definitions: {}", e))?;
      for r in rows {
        let (name, symbol) = r.map_err(|e| format!("Failed to read exported type: {}", e))?;
        exports.insert(name, symbol);
      }
    }

    Ok(exports)
  }

  /// Return the source module path of an existing *import* of a global variable,
  /// or `None` if the name has not been imported. This is the record of "which
  /// module a name was imported from", used to detect ambiguity when a second
  /// `use` imports the same name from a different module.
  pub fn imported_var_source(&self, name: &str) -> Result<Option<String>, String> {
    let mut stmt = self
      .conn
      .prepare_cached(
        "SELECT module_path FROM variables \
         WHERE simple_name = ?1 AND is_global = 1 AND linkage = 'import' LIMIT 1",
      )
      .map_err(|e| format!("Failed to prepare imported_var_source query: {}", e))?;
    match stmt.query_row(params![name], |row| row.get::<_, String>(0)) {
      Ok(src) => Ok(Some(src)),
      Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
      Err(e) => Err(format!(
        "Failed to query imported source for '{}': {}",
        name, e
      )),
    }
  }

  /// Get the current scope.
  pub fn current_scope(&self) -> &VarScope {
    &self.current_scope
  }

  /// Convert an absolute path to a relative path from the root file's directory.
  fn to_relative_path(&self, path: &str) -> String {
    if path.is_empty() {
      return String::new();
    }

    let path_buf = PathBuf::from(path);

    // If we have a root file path, use its directory as the base
    if let Some(root) = &self.root_file_path {
      // Get the root file's directory
      if let Some(root_dir) = root.parent() {
        // If the path is the root file itself, just return the filename
        if path_buf == *root {
          return root
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or(path)
            .to_string();
        }

        // Try to get relative path from root directory
        if let Ok(relative) = path_buf.strip_prefix(root_dir) {
          return relative.to_string_lossy().to_string();
        }
      }
    }

    // Try to get the current working directory as fallback
    if let Ok(cwd) = std::env::current_dir() {
      // Try to get the relative path
      if let Ok(relative) = path_buf.strip_prefix(&cwd) {
        return relative.to_string_lossy().to_string();
      }
    }

    // If we can't make it relative, return the original path
    path.to_string()
  }

  /// Set the root file path for relative path calculation.
  pub fn set_root_file_path(&mut self, path: PathBuf) {
    self.root_file_path = Some(path);
  }

  /// Set the current module path for symbol definitions.
  pub fn set_module_path(&mut self, path: String) {
    self.current_module_path = self.to_relative_path(&path);
  }

  /// Get the current module path.
  pub fn get_module_path(&self) -> &str {
    &self.current_module_path
  }

  /// Get a reference to the underlying SQLite connection.
  pub fn conn(&self) -> &Connection {
    &self.conn
  }

  /// Register a symbol in the specified scope (New schema).
  /// Handles functions, variables, and parameters using the new normalized schema.
  #[allow(clippy::too_many_arguments)]
  #[allow(clippy::expect_used)] // Internal insert on valid schema - panic indicates corruption
  pub fn define_symbol(
    &mut self,
    scope: VarScope,
    name: &str,
    location: &SourceLocation,
    data_type: &str,
    physical_unit: Option<String>,
    kind: SymbolKind,
    linkage: Linkage,
    visibility: Visibility,
    is_definition: bool,
    is_initialized: bool,
    pointer_to_type: Option<String>,
  ) -> Result<(), String> {
    // Ensure the module path exists in the modules table
    let module_path = self.current_module_path.clone();
    if !self.module_exists(&module_path) {
      self.register_module(&module_path, "");
    }

    match kind {
      SymbolKind::TypeDef => Err(format!(
        "Type definition '{}' should be registered via define_type_with_fields, not define_symbol",
        name
      )),
      SymbolKind::Function => {
        // Check whether a function with the same name and return type already
        // exists *in this module*. A function imported in builtins may later be
        // exported by the stdlib in the same module; a same-named function in a
        // *different* module is a distinct symbol (module-qualified access).
        let existing = {
          let mut stmt = match self.conn.prepare_cached(
            "SELECT linkage, is_definition, module_path FROM functions
             WHERE simple_name = ?1 AND return_type = ?2 AND module_path = ?3",
          ) {
            Ok(s) => s,
            Err(e) => {
              return Err(format!(
                "Failed to check for existing function '{}': {}",
                name, e
              ));
            }
          };

          match stmt.query_row(params![name, data_type, &module_path], |row| {
            let existing_linkage_str: String = row.get(0)?;
            let existing_is_def: bool = row.get(1)?;
            let existing_module: String = row.get(2)?;
            Ok((
              Self::str_to_linkage(&existing_linkage_str),
              existing_is_def,
              existing_module,
            ))
          }) {
            Ok(existing) => Some(existing),
            Err(rusqlite::Error::QueryReturnedNoRows) => None,
            Err(e) => {
              return Err(format!(
                "Failed to check for existing function '{}': {}",
                name, e
              ));
            }
          }
        };

        // Allow replacing an import with an export
        if let Some((existing_linkage, _, existing_module)) = existing {
          if existing_linkage == Linkage::Import && linkage == Linkage::Export {
            // Replace the import with the export implementation
            let mut stmt = match self.conn.prepare_cached(
              "UPDATE functions SET
               linkage = ?1, visibility = ?2, is_definition = ?3,
               line_number = ?4, col_number = ?5
               WHERE module_path = ?6 AND simple_name = ?7 AND return_type = ?8",
            ) {
              Ok(s) => s,
              Err(e) => {
                return Err(format!("Failed to update function '{}': {}", name, e));
              }
            };

            if stmt
              .execute(params![
                Self::linkage_to_str(&linkage),
                Self::visibility_to_str(&visibility),
                is_definition,
                location.line as i64,
                location.col as i64,
                &existing_module,
                name,
                data_type,
              ])
              .is_err()
            {
              return Err(format!("Failed to replace import '{}' with export", name));
            }
            return Ok(());
          } else if linkage == Linkage::Import {
            // The symbol already exists (whether as Import or Export) and
            // we're trying to import it — idempotent, silently skip.
            // This handles multi-path imports (e.g., stdlib already loaded
            // the module, and user code also explicitly imports from it).
            return Ok(());
          } else {
            // True duplicate: both are exports, or mismatched
            return Err(format!(
              "Function '{}' is already defined in this scope (line {}, column {})",
              name, location.line, location.col
            ));
          }
        }

        // Insert new function
        let mut stmt = match self.conn.prepare_cached(
          "INSERT INTO functions
           (qualified_name, simple_name, return_type, module_path, line_number, col_number,
            start_pos, end_pos, linkage, visibility, storage_class, is_definition)
           VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)",
        ) {
          Ok(s) => s,
          Err(e) => {
            return Err(format!(
              "Failed to prepare SQLite statement for defining function '{}': {}",
              name, e
            ));
          }
        };

        if let Err(_e) = stmt.execute(params![
          name,                                               // qualified_name
          name,                      // simple_name (for now, full qualified_name as simple_name)
          data_type,                 // return_type
          &self.current_module_path, // module_path
          location.line as i64,      // line_number
          location.col as i64,       // col_number
          location.start_pos as i64, // start_pos
          location.end_pos as i64,   // end_pos
          Self::linkage_to_str(&linkage), // linkage
          Self::visibility_to_str(&visibility), // visibility
          Self::storage_class_to_str(&StorageClass::Default), // storage_class
          is_definition,             // is_definition
        ]) {
          return Err(format!(
            "Function '{}' is already defined in this scope (line {}, column {})",
            name, location.line, location.col
          ));
        }
        Ok(())
      }
      SymbolKind::Variable | SymbolKind::Parameter => {
        // Insert into variables table
        let is_global = matches!(scope, VarScope::Global);
        let is_parameter = matches!(kind, SymbolKind::Parameter);
        let function_qualified_name = match &scope {
          VarScope::Global => None,
          VarScope::Function { scope_key } => {
            let qualified_name =
              Self::make_qualified_function_name(&scope_key.name, &scope_key.param_types);
            Some(qualified_name)
          }
          VarScope::Suite { suite_name } => Some(suite_name.clone()),
        };

        // For global variables, check for collisions before inserting. SQLite
        // allows multiple NULLs in a PRIMARY KEY, so an explicit check is needed.
        //
        // Imports and definitions are checked separately: an import records the
        // source module (`linkage = Import`), while a definition is the module's
        // own storage (`linkage = Export`/`Internal`). Two modules may each
        // define a same-named global; ambiguity is resolved at import time.
        if is_global && !is_parameter {
          let check_sql = match linkage {
            Linkage::Import => {
              "SELECT line_number, col_number FROM variables \
               WHERE simple_name = ?1 AND is_global = 1 AND linkage = 'import' AND module_path = ?2"
            }
            Linkage::Export | Linkage::Internal => {
              "SELECT line_number, col_number FROM variables \
               WHERE simple_name = ?1 AND is_global = 1 \
                 AND linkage IN ('export', 'internal') AND module_path = ?2"
            }
          };

          let mut check_stmt = match self.conn.prepare_cached(check_sql) {
            Ok(s) => s,
            Err(e) => {
              return Err(format!(
                "Failed to prepare duplicate check for variable '{}': {}",
                name, e
              ));
            }
          };

          let existing: Option<(i64, i64)> = match check_stmt
            .query_row(params![name, &module_path], |row| {
              Ok((row.get(0)?, row.get(1)?))
            }) {
            Ok(v) => Some(v),
            Err(rusqlite::Error::QueryReturnedNoRows) => None,
            Err(e) => {
              return Err(format!(
                "Failed to check for duplicate variable '{}': {}",
                name, e
              ));
            }
          };

          if let Some((existing_line, existing_col)) = existing {
            match linkage {
              // Re-importing the same name from the same source is idempotent.
              Linkage::Import => return Ok(()),
              Linkage::Export | Linkage::Internal => {
                return Err(format!(
                  "Variable '{}' is already defined at line {}, column {}",
                  name, existing_line, existing_col
                ));
              }
            }
          }
        }

        let mut stmt = match self.conn.prepare_cached(
          "INSERT INTO variables
          (function_qualified_name, simple_name, var_type, physical_unit, module_path,
           line_number, col_number, start_pos, end_pos, is_global, is_parameter, param_index, pass_mode,
           linkage, visibility, storage_class, is_definition, is_initialized, pointer_to_type)
          VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18, ?19)",
        ) {
          Ok(s) => s,
          Err(e) => {
            return Err(format!(
              "Failed to prepare SQLite statement for defining variable '{}': {}",
              name, e
            ));
          }
        };

        if let Err(_e) = stmt.execute(params![
          function_qualified_name,                            // function_qualified_name
          name,                                               // simple_name
          data_type,                                          // var_type
          physical_unit,                                      // physical_unit
          &self.current_module_path,                          // module_path
          location.line as i64,                               // line_number
          location.col as i64,                                // col_number
          location.start_pos as i64,                          // start_pos
          location.end_pos as i64,                            // end_pos
          is_global,                                          // is_global
          is_parameter,                                       // is_parameter
          0i64,   // param_index (not used for variables, only for parameters)
          "copy", // pass_mode (not used for variables, only for parameters)
          Self::linkage_to_str(&linkage), // linkage
          Self::visibility_to_str(&visibility), // visibility
          Self::storage_class_to_str(&StorageClass::Default), // storage_class
          is_definition, // is_definition
          is_initialized, // is_initialized
          pointer_to_type, // pointer_to_type
        ]) {
          return Err(format!(
            "Variable '{}' is already defined in this scope (line {}, column {})",
            name, location.line, location.col
          ));
        }
        Ok(())
      }
    }
  }

  /// Register a variable in the specified scope.
  #[allow(clippy::too_many_arguments)]
  pub fn define_variable(
    &mut self,
    scope: VarScope,
    name: &str,
    location: &SourceLocation,
    data_type: &str,
    physical_unit: Option<String>,
    linkage: Linkage,
    is_initialized: bool,
  ) -> Result<(), String> {
    self.define_variable_with_pointer_type(
      scope,
      name,
      location,
      data_type,
      physical_unit,
      linkage,
      is_initialized,
      None,
    )
  }

  /// Register a variable with explicit pointer source type tracking.
  #[allow(clippy::too_many_arguments)]
  pub fn define_variable_with_pointer_type(
    &mut self,
    scope: VarScope,
    name: &str,
    location: &SourceLocation,
    data_type: &str,
    physical_unit: Option<String>,
    linkage: Linkage,
    is_initialized: bool,
    pointer_to_type: Option<String>,
  ) -> Result<(), String> {
    self.define_symbol(
      scope,
      name,
      location,
      data_type,
      physical_unit,
      SymbolKind::Variable,
      linkage,
      Visibility::default(),
      true,
      is_initialized,
      pointer_to_type,
    )
  }

  /// Register a function parameter in the current scope.
  pub fn define_parameter(
    &mut self,
    scope: VarScope,
    name: &str,
    location: &SourceLocation,
    data_type: &str,
    physical_unit: Option<String>,
    linkage: Linkage,
  ) -> Result<(), String> {
    self.define_symbol(
      scope,
      name,
      location,
      data_type,
      physical_unit,
      SymbolKind::Parameter,
      linkage,
      Visibility::default(),
      true,
      false,
      None,
    )
  }

  /// Register a function in the global scope.
  pub fn define_function(
    &mut self,
    name: &str,
    location: &SourceLocation,
    return_type: &str,
    _return_unit: Option<String>,
    fn_info: FnInfo,
    linkage: Linkage,
  ) -> Result<(), String> {
    // Cache FnInfo (needed for body — Vec<Stmt> can't be persisted to SQLite.
    // Parameters and return_unit are also persisted to the database and can be
    // reconstructed on demand for cache misses.)
    self.fn_info_cache.insert(name.to_string(), fn_info.clone());

    // Build qualified name from function name and parameter types
    let param_types: Vec<String> = fn_info
      .parameters
      .iter()
      .map(|p| TypeInference::type_name_to_string(&p.type_annotation))
      .collect();
    let qualified_name = if param_types.is_empty() {
      name.to_string()
    } else {
      format!("{}_{}", name, param_types.join("_"))
    };

    // Insert into functions table directly with the qualified name
    let module_path = self.current_module_path.clone();
    if !self.module_exists(&module_path) {
      self.register_module(&module_path, "");
    }

    // Extract return_unit as string for storage
    let return_unit_str = fn_info.return_unit.as_ref().map(|u| u.raw.clone());

    let mut stmt = match self.conn.prepare_cached(
      "INSERT OR REPLACE INTO functions
       (qualified_name, simple_name, return_type, return_unit, module_path, line_number, col_number,
        start_pos, end_pos, linkage, visibility, storage_class, is_definition)
       VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)",
    ) {
      Ok(s) => s,
      Err(e) => {
        return Err(format!(
          "Failed to prepare SQLite statement for defining function '{}': {}",
          name, e
        ));
      }
    };

    if let Err(e) = stmt.execute(params![
      &qualified_name,                                    // qualified_name
      name,                                               // simple_name
      return_type,                                        // return_type
      &return_unit_str,                                   // return_unit
      &module_path,                                       // module_path
      location.line as i64,                               // line_number
      location.col as i64,                                // col_number
      location.start_pos as i64,                          // start_pos
      location.end_pos as i64,                            // end_pos
      Self::linkage_to_str(&linkage),                     // linkage
      Self::visibility_to_str(&Visibility::default()),    // visibility
      Self::storage_class_to_str(&StorageClass::Default), // storage_class
      true,                                               // is_definition
    ]) {
      return Err(format!(
        "Failed to insert function '{}' into SQLite database: {}",
        name, e
      ));
    }

    // Persist function parameters to database
    for (param_idx, param) in fn_info.parameters.iter().enumerate() {
      let param_type = TypeInference::type_name_to_string(&param.type_annotation);
      let param_unit = param.unit.as_ref().map(|u| u.raw.clone());

      // Don't use prepare_cached in a loop - create a fresh statement each time
      let mut param_stmt = match self.conn.prepare(
        "INSERT OR REPLACE INTO variables
         (function_qualified_name, simple_name, var_type, physical_unit, module_path,
          line_number, col_number, start_pos, end_pos, is_global, is_parameter, param_index, pass_mode,
          linkage, visibility, storage_class, is_definition, is_initialized)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18)",
      ) {
        Ok(s) => s,
        Err(e) => {
          return Err(format!(
            "Failed to prepare SQLite statement for defining parameter '{}' in function '{}': {}",
            param.name.node, name, e
          ));
        }
      };

      if let Err(e) = param_stmt.execute(params![
        &qualified_name,                                    // function_qualified_name
        &param.name.node,                                   // simple_name
        &param_type,                                        // var_type
        param_unit,                                         // physical_unit
        &module_path,                                       // module_path
        param.location.line as i64,                         // line_number
        param.location.col as i64,                          // col_number
        param.location.start_pos as i64,                    // start_pos
        param.location.end_pos as i64,                      // end_pos
        false,                                              // is_global
        true,                                               // is_parameter
        param_idx as i64,                                   // param_index
        param.pass_mode.to_str(),                           // pass_mode
        Self::linkage_to_str(&Linkage::Internal),           // linkage
        Self::visibility_to_str(&Visibility::default()),    // visibility
        Self::storage_class_to_str(&StorageClass::Default), // storage_class
        true,                                               // is_definition
        true, // is_initialized (parameters are always initialized)
      ]) {
        return Err(format!(
          "Failed to insert parameter '{}' into SQLite database: {}",
          param.name.node, e
        ));
      }
    }

    Ok(())
  }

  /// Look up a function by name and reconstruct FnInfo from database.
  /// Queries the database for the function's parameters and body information.
  /// Look up the qualified name of a function (for IR generation).
  /// This queries the symbol database directly to get the proper qualified name
  /// without rebuilding it from parameters.
  /// Returns (qualified_name, linkage) tuple.
  pub fn lookup_function_qualified_name_and_linkage(
    &self,
    simple_name: &str,
  ) -> Result<Option<(String, String)>, String> {
    let mut stmt = self
      .conn
      .prepare_cached(
        "SELECT qualified_name, linkage FROM functions WHERE simple_name = ?1 LIMIT 1",
      )
      .map_err(|e| format!("Failed to prepare qualified name lookup: {}", e))?;

    match stmt.query_row([simple_name], |row| {
      Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
    }) {
      Ok(t) => Ok(Some(t)),
      Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
      Err(e) => Err(format!(
        "Failed to query qualified name for '{}': {}",
        simple_name, e
      )),
    }
  }

  pub fn lookup_function(&self, name: &str) -> Option<FnInfo> {
    // Check cache first (contains full FnInfo with return_unit and parameters)
    if let Some(cached) = self.fn_info_cache.get(name) {
      return Some(cached.clone());
    }

    // Query the functions table to verify the function exists
    let mut stmt = match self
      .conn
      .prepare_cached("SELECT qualified_name FROM functions WHERE simple_name = ?1 LIMIT 1")
    {
      Ok(s) => s,
      Err(_) => return None,
    };

    let qualified_name: String = match stmt.query_row([name], |row| row.get(0)) {
      Ok(qn) => qn,
      Err(_) => return None,
    };

    // Query return_unit from functions table
    let return_unit = match self.conn.query_row(
      "SELECT return_unit FROM functions WHERE qualified_name = ?1",
      params![&qualified_name],
      |row| row.get::<_, Option<String>>(0),
    ) {
      Ok(unit_str) => unit_str.map(|raw| Unit {
        raw,
        location: SourceLocation {
          line: 0,
          col: 0,
          start_pos: 0,
          end_pos: 0,
          source_file: String::new(),
        },
      }),
      Err(_) => None,
    };

    // Query parameters for this function from the variables table
    let mut param_stmt = match self.conn.prepare_cached(
      "SELECT simple_name, var_type, physical_unit, pass_mode, param_index,
              line_number, col_number
       FROM variables
       WHERE function_qualified_name = ?1 AND is_parameter = 1
       ORDER BY param_index ASC",
    ) {
      Ok(s) => s,
      Err(_) => return None,
    };

    let params_result = param_stmt.query_map([&qualified_name], |row| {
      Ok((
        row.get::<_, String>(0)?,         // simple_name
        row.get::<_, String>(1)?,         // var_type
        row.get::<_, Option<String>>(2)?, // physical_unit
        row.get::<_, String>(3)?,         // pass_mode
        row.get::<_, i64>(4)?,            // param_index
        row.get::<_, i64>(5)?,            // line_number
        row.get::<_, i64>(6)?,            // col_number
      ))
    });

    let mut parameters = Vec::new();
    match params_result {
      Ok(rows) => {
        for result in rows {
          match result {
            Ok((param_name, param_type, physical_unit, pass_mode, _param_idx, line, col)) => {
              // Reconstruct Parameter from database
              let type_annotation = Self::reconstruct_type_name(&param_type);
              let unit = physical_unit.map(|u| Unit {
                raw: u,
                location: SourceLocation {
                  line: 0,
                  col: 0,
                  start_pos: 0,
                  end_pos: 0,
                  source_file: String::new(),
                },
              });

              let parameter = Parameter {
                pass_mode: ParameterPassMode::from_db_str(&pass_mode),
                name: Spanned {
                  node: param_name,
                  span: SourceLocation {
                    line: line as usize,
                    col: col as usize,
                    start_pos: 0,
                    end_pos: 0,
                    source_file: String::new(),
                  },
                },
                type_annotation,
                unit,
                location: SourceLocation {
                  line: line as usize,
                  col: col as usize,
                  start_pos: 0,
                  end_pos: 0,
                  source_file: String::new(),
                },
              };
              parameters.push(parameter);
            }
            Err(_) => {
              // Ignore errors reading parameter row
            }
          }
        }
      }
      Err(_) => {
        // Ignore errors querying parameters
      }
    }

    // Return FnInfo with reconstructed parameters and return_unit
    // Note: body is empty since we don't persist AST to database
    Some(FnInfo {
      parameters,
      return_unit,
      body: Vec::new(),
    })
  }

  /// Look up a function's return type from the database.
  /// Returns the return_type string (e.g., "Color", "i32").
  pub fn lookup_function_return_type(&self, simple_name: &str) -> Result<Option<String>, String> {
    let mut stmt = self
      .conn
      .prepare_cached("SELECT return_type FROM functions WHERE simple_name = ?1 LIMIT 1")
      .map_err(|e| format!("Failed to prepare lookup_function_return_type: {}", e))?;
    match stmt.query_row([simple_name], |row| row.get(0)) {
      Ok(t) => Ok(Some(t)),
      Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
      Err(e) => Err(format!(
        "Failed to query return type for '{}': {}",
        simple_name, e
      )),
    }
  }

  /// Reconstruct a TypeName from a type string.
  /// This is used when loading parameters from the database.
  fn reconstruct_type_name(type_str: &str) -> TypeName {
    let mut array_dimensions = Vec::new();

    // Parse base type
    let type_part = if let Some(bracket_pos) = type_str.find('[') {
      let base = &type_str[..bracket_pos];
      let array_part = &type_str[bracket_pos..];

      // Parse array dimensions: e.g., "[3][4]" -> convert to Expr literals
      for dim_str in array_part.split('[').skip(1) {
        if let Some(end) = dim_str.find(']')
          && let Ok(dim) = dim_str[..end].parse::<i64>()
        {
          // Create an Expr::IntLiteral for the dimension
          let expr = Expr::IntLiteral(IntLiteral {
            value: dim,
            unit: None,
            location: SourceLocation {
              line: 0,
              col: 0,
              start_pos: 0,
              end_pos: 0,
              source_file: String::new(),
            },
          });
          array_dimensions.push(expr);
        }
      }

      base
    } else {
      type_str
    };

    let base_type = match type_part {
      "pointer" => BaseType::Pointer,
      "i8" => BaseType::I8,
      "i16" => BaseType::I16,
      "i32" => BaseType::I32,
      "i64" => BaseType::I64,
      "u8" => BaseType::U8,
      "u16" => BaseType::U16,
      "u32" => BaseType::U32,
      "u64" => BaseType::U64,
      "f16" => BaseType::F16,
      "f32" => BaseType::F32,
      "f64" => BaseType::F64,
      "text" => BaseType::Text,
      "char" => BaseType::Char,
      "bool" => BaseType::Bool,
      _ => BaseType::Pointer,
    };

    TypeName {
      base_type,
      inner_type: None,
      array_dimensions,
      is_optional: false,
      location: SourceLocation {
        line: 0,
        col: 0,
        start_pos: 0,
        end_pos: 0,
        source_file: String::new(),
      },
    }
  }

  /// Register a type definition.
  pub fn define_type(&mut self, name: &str, location: SourceLocation) {
    self.define_type_with_fields(name, location, vec![]);
  }

  /// Register a type definition with field information.
  pub fn define_type_with_fields(
    &mut self,
    name: &str,
    location: SourceLocation,
    fields: Vec<TypeFieldInfo>,
  ) {
    // Persist to database (best-effort). During stdlib/builtin bootstrap a type
    // can be registered before its owning module row exists, which fails with a
    // FOREIGN KEY constraint. That is benign and the discard is intentional.
    let _ = self.conn.execute(
      "INSERT OR REPLACE INTO type_defs (qualified_name, module_path, line_number, col_number, start_pos, end_pos, is_export)
       VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
      params![
        name,
        &self.current_module_path,
        location.line as i64,
        location.col as i64,
        location.start_pos as i64,
        location.end_pos as i64,
        0, // is_export - types are not exported by default
      ],
    );

    // Insert type fields
    for (field_idx, (field_name, field_type, field_unit, field_is_private)) in
      fields.iter().enumerate()
    {
      // Best-effort: field persistence can fail for the same bootstrap reasons
      // as the type-definition insert above. Intentional discard.
      let _ = self.conn.execute(
        "INSERT OR REPLACE INTO type_def_fields (type_def_qualified_name, field_index, field_name, field_type, physical_unit, is_private)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        params![
          name,
          field_idx as i64,
          field_name,
          field_type,
          field_unit,
          field_is_private,
        ],
      );
    }
  }

  /// Look up a type definition by name (queries database).
  pub fn lookup_type(&self, name: &str) -> Option<TypeDefInfo> {
    // Query database for type definition
    let mut stmt = match self.conn.prepare_cached(
      "SELECT qualified_name, line_number, col_number, module_path, start_pos, end_pos FROM type_defs WHERE qualified_name = ?1 LIMIT 1",
    ) {
      Ok(s) => s,
      Err(_) => return None,
    };

    let found = stmt.query_row(params![name], |row| {
      let qual_name: String = row.get(0)?;
      let line: i64 = row.get(1)?;
      let col: i64 = row.get(2)?;
      let module_path: String = row.get(3)?;
      let start_pos: i64 = row.get(4)?;
      let end_pos: i64 = row.get(5)?;
      Ok((qual_name, line, col, module_path, start_pos, end_pos))
    });

    if let Ok((qual_name, line, col, module_path, start_pos, end_pos)) = found {
      // Query the type's fields (needed for MemberAccess type resolution)
      // Empty field list is correct for types with no fields (opaque types, extern types).
      let fields = self.lookup_type_fields(&qual_name).unwrap_or_default();

      Some(TypeDefInfo {
        name: qual_name,
        location: SourceLocation {
          line: line as usize,
          col: col as usize,
          start_pos: start_pos as usize,
          end_pos: end_pos as usize,
          source_file: "<unknown>".to_string(),
        },
        fields,
        module_path,
      })
    } else {
      None
    }
  }

  /// Get all variant type names for an enum.
  /// Returns a list of (qualified_variant_type_name, simple_variant_name) tuples.
  pub fn get_enum_variants(&self, enum_name: &str) -> Vec<(String, String)> {
    let pattern = format!("{}__%", enum_name);
    let mut stmt = match self
      .conn
      .prepare("SELECT qualified_name FROM type_defs WHERE qualified_name LIKE ?1")
    {
      Ok(s) => s,
      Err(_) => return Vec::new(),
    };

    let rows = stmt.query_map(params![pattern], |row| {
      let qname: String = row.get(0)?;
      Ok(qname)
    });

    let mut variants = Vec::new();
    if let Ok(mapped) = rows {
      for row in mapped.flatten() {
        if let Some(suffix) = row.strip_prefix(&format!("{}__", enum_name)) {
          variants.push((row.clone(), suffix.to_string()));
        }
      }
    }
    variants
  }

  /// Query the fields of a type from the database.
  fn lookup_type_fields(&self, type_name: &str) -> Result<Vec<TypeFieldInfo>, String> {
    let mut stmt = self.conn.prepare_cached(
      "SELECT field_name, field_type, physical_unit, is_private FROM type_def_fields WHERE type_def_qualified_name = ?1 ORDER BY field_index",
    ).map_err(|e| format!("Failed to prepare lookup_type_fields for '{}': {}", type_name, e))?;

    let rows = stmt
      .query_map(params![type_name], |row| {
        let field_name: String = row.get(0)?;
        let field_type: String = row.get(1)?;
        let physical_unit: Option<String> = row.get(2)?;
        let is_private: bool = row.get::<_, bool>(3)?;
        Ok((field_name, field_type, physical_unit, is_private))
      })
      .map_err(|e| format!("Failed to query type fields for '{}': {}", type_name, e))?;

    let mut fields = Vec::new();
    for result in rows {
      fields
        .push(result.map_err(|e| format!("Failed to read type field for '{}': {}", type_name, e))?);
    }
    Ok(fields)
  }

  /// Check if variable exists in the current scope (with the appropriate
  /// fallback: global → suite → none for suites/cases).
  /// Uses prepare_cached() for optimal performance on repeated lookups.
  #[allow(clippy::expect_used)] // Internal query on valid schema - panic indicates corruption
  pub fn is_variable_defined(&self, name: &str) -> bool {
    match &self.current_scope {
      VarScope::Global => {
        // Check global scope
        let mut stmt = self
          .conn
          .prepare_cached("SELECT COUNT(*) FROM variables WHERE simple_name = ?1 AND is_global = 1")
          .expect("Failed to prepare is_variable_defined query");

        let count: i32 = stmt.query_row(params![name], |row| row.get(0)).unwrap_or(0);

        count > 0
      }
      VarScope::Suite { suite_name } => {
        // Suite scope: suite-level variables only (module globals invisible).
        let mut stmt = self
          .conn
          .prepare_cached(
            "SELECT COUNT(*) FROM variables WHERE simple_name = ?1 AND function_qualified_name = ?2",
          )
          .expect("Failed to prepare is_variable_defined query");

        let count: i32 = stmt
          .query_row(params![name, suite_name], |row| row.get(0))
          .unwrap_or(0);

        count > 0
      }
      VarScope::Function { scope_key } => {
        let qualified_name =
          Self::make_qualified_function_name(&scope_key.name, &scope_key.param_types);

        match &self.current_suite {
          Some(suite_name) => {
            // Test case: fall back to the suite scope, NOT the global scope.
            let mut stmt = self
              .conn
              .prepare_cached(
                "SELECT COUNT(*) FROM variables WHERE simple_name = ?1 AND function_qualified_name IN (?2, ?3)",
              )
              .expect("Failed to prepare is_variable_defined query");

            let count: i32 = stmt
              .query_row(params![name, &qualified_name, suite_name], |row| row.get(0))
              .unwrap_or(0);

            count > 0
          }
          None => {
            // Regular function: fall back to global scope.
            let mut stmt = self
              .conn
              .prepare_cached(
                "SELECT COUNT(*) FROM variables WHERE simple_name = ?1 AND (function_qualified_name = ?2 OR is_global = 1)",
              )
              .expect("Failed to prepare is_variable_defined query");

            let count: i32 = stmt
              .query_row(params![name, &qualified_name], |row| row.get(0))
              .unwrap_or(0);

            count > 0
          }
        }
      }
    }
  }

  /// Map a `variables` table row to a `Symbol` (columns are the standard
  /// `SELECT var_type, physical_unit, linkage, visibility, storage_class,
  /// is_definition, is_initialized, pointer_to_type, line_number, col_number,
  /// module_path` projection).
  fn var_row_to_symbol(row: &rusqlite::Row<'_>) -> rusqlite::Result<Symbol> {
    let var_type: String = row.get(0)?;
    let physical_unit: Option<String> = row.get(1)?;
    let linkage: String = row.get(2)?;
    let visibility: String = row.get(3)?;
    let storage_class: String = row.get(4)?;
    let is_definition: bool = row.get(5)?;
    let is_initialized: bool = row.get(6)?;
    let pointer_to_type: Option<String> = row.get(7)?;
    let source_line: i64 = row.get(8)?;
    let source_col: i64 = row.get(9)?;
    let module_path: String = row.get(10)?;

    Ok(Symbol {
      source_location: SourceLocation {
        line: source_line as usize,
        col: source_col as usize,
        start_pos: 0,
        end_pos: 0,
        source_file: "<unknown>".to_string(),
      },
      data_type: var_type,
      physical_unit,
      kind: SymbolKind::Variable,
      linkage: Self::str_to_linkage(&linkage),
      visibility: Self::str_to_visibility(&visibility),
      storage_class: Self::str_to_storage_class(&storage_class),
      is_definition,
      is_initialized,
      pointer_to_type,
      module_path,
    })
  }

  /// Look up a symbol in the current scope (with the appropriate fallback).
  /// Uses prepare_cached() for optimal performance on repeated lookups.
  pub fn lookup_var_symbol(&self, name: &str) -> Result<Option<Symbol>, String> {
    const COLS: &str = "var_type, physical_unit, linkage, visibility, storage_class,
             is_definition, is_initialized, pointer_to_type,
             line_number, col_number, module_path";

    // Each arm prepares a query selecting the standard column list with a
    // scope-appropriate WHERE clause, then maps the row with var_row_to_symbol.
    macro_rules! query_one {
      ($where:expr, $($param:expr),+ $(,)?) => {{
        let sql = format!("SELECT {} FROM variables WHERE {} LIMIT 1", COLS, $where);
        let mut stmt = self
          .conn
          .prepare_cached(&sql)
          .map_err(|e| format!("Failed to prepare lookup_var_symbol query: {}", e))?;
        match stmt.query_row(params![$($param),+], Self::var_row_to_symbol) {
          Ok(sym) => Ok(Some(sym)),
          Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
          Err(e) => Err(format!("Failed to query variable '{}': {}", name, e)),
        }
      }};
    }

    match &self.current_scope {
      // Prefer an Import entry (what the current module imported) over Export/Internal,
      // so an unqualified imported name resolves to the symbol the module actually
      // imported even when another module exports the same name.
      VarScope::Global => query_one!(
        "simple_name = ?1 AND is_global = 1 ORDER BY CASE WHEN linkage = 'import' THEN 0 ELSE 1 END",
        name
      ),
      VarScope::Suite { suite_name } => query_one!(
        "simple_name = ?1 AND function_qualified_name = ?2",
        name,
        suite_name
      ),
      VarScope::Function { scope_key } => {
        let qualified_name =
          Self::make_qualified_function_name(&scope_key.name, &scope_key.param_types);
        match &self.current_suite {
          Some(suite_name) => {
            // Test case: fall back to the suite scope, not the global scope.
            let sql = format!(
              "SELECT {} FROM variables WHERE simple_name = ?1 AND function_qualified_name IN (?2, ?3) ORDER BY CASE WHEN function_qualified_name = ?2 THEN 0 ELSE 1 END LIMIT 1",
              COLS
            );
            let mut stmt = self
              .conn
              .prepare_cached(&sql)
              .map_err(|e| format!("Failed to prepare lookup_var_symbol query: {}", e))?;
            match stmt.query_row(
              params![name, &qualified_name, suite_name],
              Self::var_row_to_symbol,
            ) {
              Ok(sym) => Ok(Some(sym)),
              Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
              Err(e) => Err(format!("Failed to query variable '{}': {}", name, e)),
            }
          }
          None => {
            // Regular function: fall back to the global scope.
            query_one!(
              "simple_name = ?1 AND (function_qualified_name = ?2 OR is_global = 1) ORDER BY CASE WHEN function_qualified_name = ?2 THEN 0 ELSE 1 END",
              name,
              &qualified_name
            )
          }
        }
      }
    }
  }

  /// Look up all symbols with a given name in the current scope (with the
  /// appropriate fallback). Used for ambiguity detection.
  pub fn lookup_all_var_symbols(&self, name: &str) -> Result<Vec<Symbol>, String> {
    const COLS: &str = "var_type, physical_unit, linkage, visibility, storage_class,
             is_definition, is_initialized, pointer_to_type,
             line_number, col_number, module_path";

    let mut symbols = Vec::new();
    let mut collect = |sql: &str, params: Vec<&dyn rusqlite::ToSql>| -> Result<(), String> {
      let mut stmt = self
        .conn
        .prepare_cached(sql)
        .map_err(|e| format!("Failed to prepare lookup_all_var_symbols: {}", e))?;
      let rows = stmt
        .query_map(rusqlite::params_from_iter(params), Self::var_row_to_symbol)
        .map_err(|e| format!("Failed to query all symbols: {}", e))?;
      for r in rows {
        symbols.push(r.map_err(|e| format!("Failed to read symbol row: {}", e))?);
      }
      Ok(())
    };

    match &self.current_scope {
      VarScope::Global => {
        let sql = format!(
          "SELECT {} FROM variables WHERE simple_name = ?1 AND is_global = 1",
          COLS
        );
        collect(&sql, vec![&name])?;
      }
      VarScope::Suite { suite_name } => {
        let sql = format!(
          "SELECT {} FROM variables WHERE simple_name = ?1 AND function_qualified_name = ?2",
          COLS
        );
        collect(&sql, vec![&name, suite_name])?;
      }
      VarScope::Function { scope_key } => {
        let qualified_name =
          Self::make_qualified_function_name(&scope_key.name, &scope_key.param_types);
        match &self.current_suite {
          Some(suite_name) => {
            let sql = format!(
              "SELECT {} FROM variables WHERE simple_name = ?1 AND function_qualified_name IN (?2, ?3) ORDER BY CASE WHEN function_qualified_name = ?2 THEN 0 ELSE 1 END",
              COLS
            );
            collect(&sql, vec![&name, &qualified_name, suite_name])?;
          }
          None => {
            let sql = format!(
              "SELECT {} FROM variables WHERE simple_name = ?1 AND (function_qualified_name = ?2 OR is_global = 1) ORDER BY CASE WHEN function_qualified_name = ?2 THEN 0 ELSE 1 END",
              COLS
            );
            collect(&sql, vec![&name, &qualified_name])?;
          }
        }
      }
    }
    Ok(symbols)
  }

  /// Look up the type of a variable from the symbol table.
  pub fn lookup_var_type(&self, name: &str) -> Option<String> {
    self
      .lookup_var_symbol(name)
      .unwrap_or_else(|e| {
        ice!("lookup_var_type: DB query failed for '{}': {}", name, e);
      })
      .map(|sym| sym.data_type)
  }

  /// Look up the unit of a variable from the symbol table.
  /// Searches the current scope first, then falls back to all function scopes.
  pub fn lookup_var_unit(&self, name: &str) -> ExprUnit {
    // Try current scope first (correct during semantic analysis)
    if let Ok(Some(symbol)) = self.lookup_var_symbol(name) {
      return symbol
        .physical_unit
        .as_ref()
        .filter(|u| !u.is_empty())
        .map(|u| ExprUnit::from_string(u))
        .unwrap_or(ExprUnit::Unitless);
    }
    // Fall back: search all function scopes (needed during IR generation
    // where current_scope is Global but the expression contains parameters)
    let all_tables = self.get_all_symbol_tables().unwrap_or_default();
    for (scope, table) in &all_tables {
      if matches!(scope, VarScope::Function { .. })
        && let Some(symbol) = table.get(name)
      {
        return symbol
          .physical_unit
          .as_ref()
          .filter(|u| !u.is_empty())
          .map(|u| ExprUnit::from_string(u))
          .unwrap_or(ExprUnit::Unitless);
      }
    }
    ExprUnit::Unknown
  }

  // ==================== Constant-value tracking ====================
  //
  // A variable's compile-time constant value is persisted in the `variables`
  // table (`const_value` column), guarded by `is_shared`. A variable is a
  // constant only while `is_shared = FALSE` (never address-taken, ref-passed,
  // or export/import) *and* `const_value IS NOT NULL` (its last write was a
  // constant expression). The `variable_constants` view exposes this predicate
  // as a derived `is_constant` column. This is the single source of truth for
  // analyses such as division-by-zero detection, unsigned-subtraction underflow
  // detection, and (future) constant propagation.

  /// Resolve a variable name in the current scope to its storage key
  /// `(function_qualified_name, is_global)`, using the same fallback rules as
  /// `lookup_var_symbol` (local first, then global / suite). Returns `None` if
  /// the name is not defined.
  fn resolve_variable_key(&self, name: &str) -> Result<Option<(Option<String>, bool)>, String> {
    const COLS: &str = "function_qualified_name, is_global";

    macro_rules! query_key {
      ($where:expr, $($param:expr),+ $(,)?) => {{
        let sql = format!("SELECT {} FROM variables WHERE {} LIMIT 1", COLS, $where);
        let mut stmt = self
          .conn
          .prepare_cached(&sql)
          .map_err(|e| format!("Failed to prepare resolve_variable_key query: {}", e))?;
        match stmt.query_row(params![$($param),+], |row| {
          Ok((row.get::<_, Option<String>>(0)?, row.get::<_, bool>(1)?))
        }) {
          Ok(v) => Ok(Some(v)),
          Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
          Err(e) => Err(format!("Failed to resolve variable '{}': {}", name, e)),
        }
      }};
    }

    match &self.current_scope {
      VarScope::Global => query_key!("simple_name = ?1 AND is_global = 1", name),
      VarScope::Suite { suite_name } => query_key!(
        "simple_name = ?1 AND function_qualified_name = ?2",
        name,
        suite_name
      ),
      VarScope::Function { scope_key } => {
        let qualified_name =
          Self::make_qualified_function_name(&scope_key.name, &scope_key.param_types);
        match &self.current_suite {
          Some(suite_name) => query_key!(
            "simple_name = ?1 AND function_qualified_name IN (?2, ?3) \
             ORDER BY CASE WHEN function_qualified_name = ?2 THEN 0 ELSE 1 END",
            name,
            &qualified_name,
            suite_name
          ),
          None => query_key!(
            "simple_name = ?1 AND (function_qualified_name = ?2 OR is_global = 1) \
             ORDER BY CASE WHEN function_qualified_name = ?2 THEN 0 ELSE 1 END",
            name,
            &qualified_name
          ),
        }
      }
    }
  }

  /// Look up the compile-time constant value of a variable in the current scope
  /// (with the same fallback rules as `lookup_var_symbol`). Returns `Some` only
  /// when the variable is not shared and its value is known — the
  /// `variable_constants` view computes the `is_constant` predicate.
  pub fn lookup_variable_const_value(&self, name: &str) -> Option<ConstValue> {
    let (fqn, is_global) = match self.resolve_variable_key(name) {
      Ok(Some(key)) => key,
      Ok(None) => return None,
      Err(e) => ice!("lookup_variable_const_value: {}", e),
    };

    let mut stmt = match self.conn.prepare_cached(
      "SELECT const_value FROM variable_constants \
       WHERE simple_name = ?1 AND function_qualified_name IS ?2 AND is_global = ?3 \
         AND is_constant = TRUE LIMIT 1",
    ) {
      Ok(s) => s,
      Err(e) => ice!("lookup_variable_const_value prepare: {}", e),
    };

    let const_value: Option<String> =
      match stmt.query_row(params![name, fqn, is_global], |row| row.get(0)) {
        Ok(v) => v,
        Err(rusqlite::Error::QueryReturnedNoRows) => return None,
        Err(e) => ice!("lookup_variable_const_value query: {}", e),
      };

    const_value.as_deref().and_then(ConstValue::from_db)
  }

  /// Set a variable's current constant value in the current scope.
  ///
  /// If the variable is shared (it may be mutated out of the compiler's sight),
  /// the write is ignored so a stale constant is never kept. `None` clears the
  /// constant.
  pub fn update_variable_const_value(&mut self, name: &str, value: Option<ConstValue>) {
    let serialized = value.and_then(|v| v.to_db());
    let (fqn, is_global) = match self.resolve_variable_key(name) {
      Ok(Some(key)) => key,
      Ok(None) => return,
      Err(e) => ice!("update_variable_const_value: {}", e),
    };

    let mut stmt = match self.conn.prepare_cached(
      "UPDATE variables SET const_value = ?1 \
       WHERE simple_name = ?2 AND function_qualified_name IS ?3 \
         AND is_global = ?4 AND is_shared = FALSE",
    ) {
      Ok(s) => s,
      Err(e) => ice!("update_variable_const_value prepare: {}", e),
    };
    if let Err(e) = stmt.execute(params![serialized, name, fqn, is_global]) {
      ice!("update_variable_const_value execute: {}", e);
    }
  }

  /// Mark a variable in the current scope as shared, so it can no longer be a
  /// constant: its storage has been exposed via `pointer to`, a `ref` argument,
  /// or `export`/`import`.
  pub fn mark_variable_shared(&mut self, name: &str) {
    let (fqn, is_global) = match self.resolve_variable_key(name) {
      Ok(Some(key)) => key,
      Ok(None) => return,
      Err(e) => ice!("mark_variable_shared: {}", e),
    };

    let mut stmt = match self.conn.prepare_cached(
      "UPDATE variables SET is_shared = TRUE, const_value = NULL \
       WHERE simple_name = ?1 AND function_qualified_name IS ?2 \
         AND is_global = ?3",
    ) {
      Ok(s) => s,
      Err(e) => ice!("mark_variable_shared prepare: {}", e),
    };
    if let Err(e) = stmt.execute(params![name, fqn, is_global]) {
      ice!("mark_variable_shared execute: {}", e);
    }
  }

  /// Mark a variable in the current scope as reassigned (`=` / `+=` etc.).
  ///
  /// This is sticky: once a variable has been reassigned, its final `const_value`
  /// only holds for code after the last write, so it can no longer be treated as
  /// a program-wide constant (see `is_effectively_constant`).
  pub fn mark_variable_reassigned(&mut self, name: &str) {
    let (fqn, is_global) = match self.resolve_variable_key(name) {
      Ok(Some(key)) => key,
      Ok(None) => return,
      Err(e) => ice!("mark_variable_reassigned: {}", e),
    };

    let mut stmt = match self.conn.prepare_cached(
      "UPDATE variables SET is_reassigned = TRUE \
       WHERE simple_name = ?1 AND function_qualified_name IS ?2 \
         AND is_global = ?3",
    ) {
      Ok(s) => s,
      Err(e) => ice!("mark_variable_reassigned prepare: {}", e),
    };
    if let Err(e) = stmt.execute(params![name, fqn, is_global]) {
      ice!("mark_variable_reassigned execute: {}", e);
    }
  }

  /// Look up the compile-time constant value of a variable that is *effectively
  /// constant* (never reassigned and never shared), so it is safe to fold the
  /// same value at any use site — used by IR constant propagation.
  pub fn lookup_effectively_constant_value(&self, name: &str) -> Option<ConstValue> {
    let (fqn, is_global) = match self.resolve_variable_key(name) {
      Ok(Some(key)) => key,
      Ok(None) => return None,
      Err(e) => ice!("lookup_effectively_constant_value: {}", e),
    };

    let mut stmt = match self.conn.prepare_cached(
      "SELECT const_value FROM variable_constants \
       WHERE simple_name = ?1 AND function_qualified_name IS ?2 AND is_global = ?3 \
         AND is_effectively_constant = TRUE LIMIT 1",
    ) {
      Ok(s) => s,
      Err(e) => ice!("lookup_effectively_constant_value prepare: {}", e),
    };

    let const_value: Option<String> =
      match stmt.query_row(params![name, fqn, is_global], |row| row.get(0)) {
        Ok(v) => v,
        Err(rusqlite::Error::QueryReturnedNoRows) => return None,
        Err(e) => ice!("lookup_effectively_constant_value query: {}", e),
      };

    const_value.as_deref().and_then(ConstValue::from_db)
  }

  /// Create and switch to a function's local scope, register parameters.
  pub fn enter_function(&mut self, fn_def: &FnDefStmt) {
    let scope_key = FnScopeKey {
      name: fn_def.name.node.clone(),
      param_types: fn_def
        .parameters
        .iter()
        .map(|p| TypeInference::type_name_to_string(&p.type_annotation))
        .collect(),
      return_type: match &fn_def.return_type.kind {
        ReturnTypeKind::Nothing => "nothing".to_string(),
        ReturnTypeKind::Type(t) => TypeInference::type_name_to_string(t),
      },
    };
    let scope = VarScope::Function { scope_key };
    self.current_scope = scope.clone();

    for param in &fn_def.parameters {
      // Registration is intentionally best-effort: re-processing a signature that
      // was already seen (e.g. a declaration followed by its definition) hits a
      // UNIQUE constraint on (function_qualified_name, simple_name). The parameter
      // is already present, so the error is benign and safely discarded.
      let _ = self.define_parameter(
        scope.clone(),
        &param.name.node,
        &param.name.span,
        &TypeInference::type_name_to_string(&param.type_annotation),
        param.unit.as_ref().map(|u| u.raw.clone()),
        Linkage::Internal,
      );
    }
  }

  /// Exit function scope, returning to the enclosing scope. For a test case or
  /// suite function this is its suite scope (and `current_suite` stays set so
  /// subsequent cases/declarations keep falling back to it); for a regular
  /// function this is global.
  pub fn exit_function(&mut self) {
    self.current_scope = match &self.current_suite {
      Some(suite_name) => VarScope::Suite {
        suite_name: suite_name.clone(),
      },
      None => VarScope::Global,
    };
  }

  /// Enter a test suite scope: suite-level variables live here, and module-level
  /// (global) variables are invisible from it or its cases.
  pub fn enter_suite_scope(&mut self, suite_name: &str) {
    self.current_suite = Some(suite_name.to_string());
    self.current_scope = VarScope::Suite {
      suite_name: suite_name.to_string(),
    };
  }

  /// Exit a test suite scope, returning to global.
  pub fn exit_suite_scope(&mut self) {
    self.current_suite = None;
    self.current_scope = VarScope::Global;
  }

  /// Switch to a function-like scope for a test case. Test cases behave like
  /// functions: their local variables are case-scoped, and lookups fall through
  /// to the enclosing suite scope (NOT the global scope).
  pub fn enter_test_case_scope(&mut self, scope_key: FnScopeKey) {
    self.current_scope = VarScope::Function { scope_key };
  }

  /// Check if a variable is defined in the current function scope (local).
  /// Uses prepare_cached() for optimal performance on repeated lookups.
  #[allow(clippy::expect_used)] // Internal query on valid schema - panic indicates corruption
  pub fn is_local_variable(&self, name: &str) -> bool {
    match &self.current_scope {
      VarScope::Global | VarScope::Suite { .. } => false,
      VarScope::Function { scope_key } => {
        let qualified_name =
          Self::make_qualified_function_name(&scope_key.name, &scope_key.param_types);

        let mut stmt = self.conn.prepare_cached(
          "SELECT COUNT(*) FROM variables WHERE simple_name = ?1 AND function_qualified_name = ?2 AND is_parameter = 0"
        ).expect("Failed to prepare is_local_variable query");

        let count: i32 = stmt
          .query_row(params![name, &qualified_name], |row| row.get(0))
          .unwrap_or(0);

        count > 0
      }
    }
  }

  /// Update a symbol's unit in the current scope or global scope.
  /// Uses prepare_cached() for optimal performance on repeated updates.
  #[allow(clippy::expect_used)] // Internal update on valid schema - panic indicates corruption
  pub fn update_symbol_unit(&mut self, name: &str, unit: Option<String>) {
    match &self.current_scope {
      VarScope::Global => {
        // Update global variable
        let mut stmt = self
          .conn
          .prepare_cached(
            "UPDATE variables SET physical_unit = ?1
           WHERE simple_name = ?2 AND is_global = 1",
          )
          .expect("Failed to prepare update_symbol_unit query");
        stmt
          .execute(params![unit, name])
          .unwrap_or_else(|e| ice!("update_symbol_unit (global) failed: {}", e));
      }
      VarScope::Suite { suite_name } => {
        // Update suite-level variable.
        let mut stmt = self
          .conn
          .prepare_cached(
            "UPDATE variables SET physical_unit = ?1
           WHERE simple_name = ?2 AND function_qualified_name = ?3",
          )
          .expect("Failed to prepare update_symbol_unit query");
        stmt
          .execute(params![unit, name, suite_name])
          .unwrap_or_else(|e| ice!("update_symbol_unit (suite) failed: {}", e));
      }
      VarScope::Function { scope_key } => {
        // Update function-local variable
        let qualified_name =
          Self::make_qualified_function_name(&scope_key.name, &scope_key.param_types);

        let rows_updated = {
          let mut stmt = self
            .conn
            .prepare_cached(
              "UPDATE variables SET physical_unit = ?1
             WHERE simple_name = ?2 AND function_qualified_name = ?3",
            )
            .expect("Failed to prepare update_symbol_unit query");
          stmt
            .execute(params![unit, name, &qualified_name])
            .unwrap_or(0)
        };

        // If not found in function scope, try global scope
        if rows_updated == 0 {
          let mut stmt = self
            .conn
            .prepare_cached(
              "UPDATE variables SET physical_unit = ?1
             WHERE simple_name = ?2 AND is_global = 1",
            )
            .expect("Failed to prepare update_symbol_unit fallback query");
          stmt
            .execute(params![unit, name])
            .unwrap_or_else(|e| ice!("update_symbol_unit (fallback) failed: {}", e));
        }
      }
    }
  }

  // ==================== Module Management ====================

  /// Register a module in the database.
  /// Uses prepare_cached() for optimal performance.
  #[allow(clippy::expect_used)] // Internal insert on valid schema - panic indicates corruption
  pub fn register_module(&mut self, path: &str, base_dir: &str) {
    let mut stmt = self
      .conn
      .prepare_cached(
        "INSERT OR IGNORE INTO modules (path, base_dir, is_parsed, is_analyzed)
       VALUES (?1, ?2, 0, 0)",
      )
      .expect("Failed to prepare register_module query");
    stmt
      .execute(params![path, base_dir])
      .expect("Failed to register module");
  }

  /// Mark a module as parsed.
  #[allow(clippy::expect_used)] // Internal update on valid schema - panic indicates corruption
  pub fn mark_module_parsed(&mut self, path: &str) {
    let mut stmt = self
      .conn
      .prepare_cached("UPDATE modules SET is_parsed = 1 WHERE path = ?1")
      .expect("Failed to prepare mark_module_parsed query");
    stmt
      .execute(params![path])
      .expect("Failed to mark module as parsed");
  }

  /// Mark a module as analyzed.
  #[allow(clippy::expect_used)] // Internal update on valid schema - panic indicates corruption
  pub fn mark_module_analyzed(&mut self, path: &str) {
    let mut stmt = self
      .conn
      .prepare_cached("UPDATE modules SET is_analyzed = 1 WHERE path = ?1")
      .expect("Failed to prepare mark_module_analyzed query");
    stmt
      .execute(params![path])
      .expect("Failed to mark module as analyzed");
  }

  /// Check if a module exists in the database.
  #[allow(clippy::expect_used)] // Internal query on valid schema - panic indicates corruption
  pub fn module_exists(&self, path: &str) -> bool {
    let mut stmt = self
      .conn
      .prepare_cached("SELECT COUNT(*) FROM modules WHERE path = ?1")
      .expect("Failed to prepare module_exists query");
    let count: i32 = stmt.query_row(params![path], |row| row.get(0)).unwrap_or(0);
    count > 0
  }

  /// Check if a module has been parsed.
  #[allow(clippy::expect_used)] // Internal query on valid schema - panic indicates corruption
  pub fn is_module_parsed(&self, path: &str) -> bool {
    let mut stmt = self
      .conn
      .prepare_cached("SELECT is_parsed FROM modules WHERE path = ?1")
      .expect("Failed to prepare is_module_parsed query");
    stmt
      .query_row(params![path], |row| row.get(0))
      .unwrap_or(false)
  }

  /// Check if a module has been analyzed.
  #[allow(clippy::expect_used)] // Internal query on valid schema - panic indicates corruption
  pub fn is_module_analyzed(&self, path: &str) -> bool {
    let mut stmt = self
      .conn
      .prepare_cached("SELECT is_analyzed FROM modules WHERE path = ?1")
      .expect("Failed to prepare is_module_analyzed query");
    stmt
      .query_row(params![path], |row| row.get(0))
      .unwrap_or(false)
  }

  /// Store a metadata key-value pair. Used for analyzer state that must not
  /// be duplicated across in-memory structures.
  #[allow(clippy::expect_used)]
  pub fn set_metadata(&mut self, key: &str, value: &str) {
    let mut stmt = self
      .conn
      .prepare_cached("INSERT OR REPLACE INTO metadata (key, value) VALUES (?1, ?2)")
      .expect("Failed to prepare set_metadata query");
    stmt
      .execute(params![key, value])
      .unwrap_or_else(|e| ice!("set_metadata failed: {}", e));
  }

  /// Retrieve a metadata value by key. Returns None if the key doesn't exist.
  pub fn get_metadata(&self, key: &str) -> Result<Option<String>, String> {
    let mut stmt = self
      .conn
      .prepare_cached("SELECT value FROM metadata WHERE key = ?1")
      .map_err(|e| format!("Failed to prepare get_metadata: {}", e))?;
    match stmt.query_row(params![key], |row| row.get(0)) {
      Ok(v) => Ok(Some(v)),
      Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
      Err(e) => Err(format!("Failed to query metadata '{}': {}", key, e)),
    }
  }
}

impl Default for SqliteSymbolManager {
  fn default() -> Self {
    Self::new()
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_sqlite_symbol_manager_basic() {
    let mut manager = SqliteSymbolManager::new();

    let location = SourceLocation::dummy();

    let _ = manager.define_variable(
      VarScope::Global,
      "x",
      &location,
      "i32",
      Some("<m>".to_string()),
      Linkage::Internal,
      true,
    );

    assert!(manager.is_variable_defined("x"));
    assert!(!manager.is_variable_defined("y"));

    let sym = manager
      .lookup_var_symbol("x")
      .unwrap()
      .expect("x should exist");
    assert_eq!(sym.data_type, "i32");
    assert_eq!(sym.physical_unit, Some("<m>".to_string()));
  }

  #[test]
  fn test_sqlite_scope_management() {
    let mut manager = SqliteSymbolManager::new();

    let global_loc = SourceLocation::dummy();

    // Define a global variable
    let _ = manager.define_variable(
      VarScope::Global,
      "global_var",
      &global_loc,
      "i32",
      None,
      Linkage::Internal,
      true,
    );

    // Define a local variable scope
    let fn_scope_key = FnScopeKey {
      name: "test_fn".to_string(),
      param_types: vec!["i32".to_string()],
      return_type: "nothing".to_string(),
    };

    let fn_scope = VarScope::Function {
      scope_key: fn_scope_key,
    };

    manager.current_scope = fn_scope.clone();

    // Define a local variable (Note: this requires the function to be pre-registered
    // in production code to satisfy FOREIGN KEY constraints)
    // In this test, we manually insert the function first
    {
      let qualified_name = "test_fn_i32".to_string();
      let module_path = manager.current_module_path.clone();
      if !manager.module_exists(&module_path) {
        manager.register_module(&module_path, "");
      }

      if let Ok(mut stmt) = manager.conn.prepare_cached(
        "INSERT OR REPLACE INTO functions
         (qualified_name, simple_name, return_type, module_path, is_definition)
         VALUES (?1, ?2, ?3, ?4, ?5)",
      ) {
        let _ = stmt.execute(params![
          &qualified_name,
          "test_fn",
          "nothing",
          &module_path,
          1,
        ]);
      }
    }

    let _ = manager.define_variable(
      fn_scope.clone(),
      "local_var",
      &global_loc,
      "f64",
      None,
      Linkage::Internal,
      true,
    );

    assert!(manager.is_variable_defined("global_var"));
    assert!(manager.is_variable_defined("local_var"));
    assert!(manager.is_local_variable("local_var"));
    assert!(!manager.is_local_variable("global_var"));
  }

  #[test]
  fn test_sqlite_update_unit() {
    let mut manager = SqliteSymbolManager::new();

    let location = SourceLocation::dummy();
    let _ = manager.define_variable(
      VarScope::Global,
      "x",
      &location,
      "f64",
      None,
      Linkage::Internal,
      true,
    );

    assert_eq!(manager.lookup_var_unit("x"), ExprUnit::Unitless);

    manager.update_symbol_unit("x", Some("<m/s>".to_string()));

    assert_eq!(manager.lookup_var_unit("x"), ExprUnit::from_string("<m/s>"));
  }

  #[test]
  fn test_module_registration() {
    let mut manager = SqliteSymbolManager::new();

    assert!(!manager.module_exists("/path/to/main.lale"));

    manager.register_module("/path/to/main.lale", "/path/to");

    assert!(manager.module_exists("/path/to/main.lale"));
    assert!(!manager.is_module_parsed("/path/to/main.lale"));
    assert!(!manager.is_module_analyzed("/path/to/main.lale"));

    manager.mark_module_parsed("/path/to/main.lale");
    assert!(manager.is_module_parsed("/path/to/main.lale"));

    manager.mark_module_analyzed("/path/to/main.lale");
    assert!(manager.is_module_analyzed("/path/to/main.lale"));
  }

  #[test]
  fn test_fn_info_cache_stores_return_unit() {
    let mut manager = SqliteSymbolManager::new();
    let location = SourceLocation::dummy();

    // Create a function with return unit
    let return_unit = Some(Unit {
      raw: "<meters>".to_string(),
      location: location.clone(),
    });

    let fn_info = FnInfo {
      parameters: vec![],
      return_unit: return_unit.clone(),
      body: vec![],
    };

    // Define the function
    manager
      .define_function(
        "measure_distance",
        &location,
        "i32",
        None,
        fn_info,
        Linkage::Internal,
      )
      .unwrap();

    // Look up the function and verify return_unit is cached
    let retrieved = manager.lookup_function("measure_distance").unwrap();
    assert_eq!(retrieved.return_unit.unwrap().raw, "<meters>");
  }

  #[test]
  fn test_fn_info_cache_with_parameters() {
    let mut manager = SqliteSymbolManager::new();
    let location = SourceLocation::dummy();

    // Create a function with parameters and return unit
    let param_unit = Some(Unit {
      raw: "<kg>".to_string(),
      location: location.clone(),
    });

    let return_unit = Some(Unit {
      raw: "<m/s>".to_string(),
      location: location.clone(),
    });

    let parameter = Parameter {
      pass_mode: ParameterPassMode::ByValueExplicit,
      name: Spanned::new("mass".to_string(), location.clone()),
      type_annotation: TypeName {
        base_type: BaseType::F64,
        inner_type: None,
        array_dimensions: vec![],
        is_optional: false,
        location: location.clone(),
      },
      unit: param_unit,
      location: location.clone(),
    };

    let fn_info = FnInfo {
      parameters: vec![parameter],
      return_unit: return_unit.clone(),
      body: vec![],
    };

    // Define the function
    manager
      .define_function(
        "velocity",
        &location,
        "f64",
        None,
        fn_info,
        Linkage::Internal,
      )
      .unwrap();

    // Look up the function and verify both parameters and return_unit are cached
    let retrieved = manager.lookup_function("velocity").unwrap();
    assert_eq!(retrieved.parameters.len(), 1);
    assert_eq!(retrieved.parameters[0].name.node, "mass");
    assert_eq!(retrieved.return_unit.unwrap().raw, "<m/s>");
  }

  #[test]
  fn test_variable_const_value_round_trip_global() {
    let mut manager = SqliteSymbolManager::new();
    let location = SourceLocation::dummy();
    let _ = manager.define_variable(
      VarScope::Global,
      "a",
      &location,
      "i32",
      None,
      Linkage::Internal,
      true,
    );

    // A freshly-defined variable has no constant value yet.
    assert!(manager.lookup_variable_const_value("a").is_none());

    manager.update_variable_const_value("a", Some(ConstValue::Int(42)));
    assert_eq!(
      manager.lookup_variable_const_value("a"),
      Some(ConstValue::Int(42))
    );

    // Clearing removes the constant.
    manager.update_variable_const_value("a", None);
    assert!(manager.lookup_variable_const_value("a").is_none());
  }

  #[test]
  fn test_variable_const_value_globals_do_not_collide() {
    let mut manager = SqliteSymbolManager::new();
    let location = SourceLocation::dummy();
    let _ = manager.define_variable(
      VarScope::Global,
      "x",
      &location,
      "i32",
      None,
      Linkage::Internal,
      true,
    );
    let _ = manager.define_variable(
      VarScope::Global,
      "y",
      &location,
      "i32",
      None,
      Linkage::Internal,
      true,
    );

    manager.update_variable_const_value("x", Some(ConstValue::Int(42)));
    manager.update_variable_const_value("y", Some(ConstValue::Int(0)));

    // Both globals share a NULL function_qualified_name; the lookup must still
    // distinguish them by simple_name.
    assert_eq!(
      manager.lookup_variable_const_value("x"),
      Some(ConstValue::Int(42))
    );
    assert_eq!(
      manager.lookup_variable_const_value("y"),
      Some(ConstValue::Int(0))
    );
  }

  #[test]
  fn test_mark_variable_shared_clears_and_is_sticky() {
    let mut manager = SqliteSymbolManager::new();
    let location = SourceLocation::dummy();
    let _ = manager.define_variable(
      VarScope::Global,
      "a",
      &location,
      "i32",
      None,
      Linkage::Internal,
      true,
    );

    manager.update_variable_const_value("a", Some(ConstValue::Int(5)));
    assert_eq!(
      manager.lookup_variable_const_value("a"),
      Some(ConstValue::Int(5))
    );

    manager.mark_variable_shared("a");
    assert!(manager.lookup_variable_const_value("a").is_none());

    // Once shared, a later direct assignment must not re-establish a constant
    // (the pointer/ref/export/import could still mutate it).
    manager.update_variable_const_value("a", Some(ConstValue::Int(10)));
    assert!(manager.lookup_variable_const_value("a").is_none());
  }

  fn view_is_constant(manager: &SqliteSymbolManager, name: &str) -> bool {
    manager
      .conn
      .query_row(
        "SELECT is_constant FROM variable_constants \
         WHERE simple_name = ?1 AND is_global = 1",
        params![name],
        |row| row.get(0),
      )
      .unwrap()
  }

  #[test]
  fn test_variable_constants_view_derives_is_constant() {
    let mut manager = SqliteSymbolManager::new();
    let location = SourceLocation::dummy();
    let _ = manager.define_variable(
      VarScope::Global,
      "a",
      &location,
      "i32",
      None,
      Linkage::Internal,
      true,
    );
    let _ = manager.define_variable(
      VarScope::Global,
      "b",
      &location,
      "i32",
      None,
      Linkage::Internal,
      true,
    );

    // Fresh variables have no value → not constant.
    assert!(!view_is_constant(&manager, "a"));
    assert!(!view_is_constant(&manager, "b"));

    // A known value → constant.
    manager.update_variable_const_value("a", Some(ConstValue::Int(7)));
    assert!(view_is_constant(&manager, "a"));

    // Shared → no longer constant, even though const_value would remain non-NULL.
    manager.update_variable_const_value("b", Some(ConstValue::Int(3)));
    assert!(view_is_constant(&manager, "b"));
    manager.mark_variable_shared("b");
    assert!(!view_is_constant(&manager, "b"));
  }

  fn view_is_effectively_constant(manager: &SqliteSymbolManager, name: &str) -> bool {
    manager
      .conn
      .query_row(
        "SELECT is_effectively_constant FROM variable_constants \
         WHERE simple_name = ?1 AND is_global = 1",
        params![name],
        |row| row.get(0),
      )
      .unwrap()
  }

  #[test]
  fn test_effectively_constant_requires_never_reassigned() {
    let mut manager = SqliteSymbolManager::new();
    let location = SourceLocation::dummy();
    let _ = manager.define_variable(
      VarScope::Global,
      "a",
      &location,
      "i32",
      None,
      Linkage::Internal,
      true,
    );

    // No value yet → not effectively constant.
    assert!(manager.lookup_effectively_constant_value("a").is_none());

    // A known value, never reassigned → effectively constant.
    manager.update_variable_const_value("a", Some(ConstValue::Int(5)));
    assert_eq!(
      manager.lookup_effectively_constant_value("a"),
      Some(ConstValue::Int(5))
    );
    assert!(view_is_effectively_constant(&manager, "a"));

    // Once reassigned → no longer effectively constant.
    manager.mark_variable_reassigned("a");
    assert!(manager.lookup_effectively_constant_value("a").is_none());
    assert!(!view_is_effectively_constant(&manager, "a"));
  }

  #[test]
  fn test_variable_const_value_is_scope_aware() {
    let mut manager = SqliteSymbolManager::new();
    let location = SourceLocation::dummy();

    // Global `x` = 42.
    let _ = manager.define_variable(
      VarScope::Global,
      "x",
      &location,
      "i32",
      None,
      Linkage::Internal,
      true,
    );
    manager.update_variable_const_value("x", Some(ConstValue::Int(42)));

    // Register a function scope and enter it (satisfies the FK constraint).
    let fn_scope = VarScope::Function {
      scope_key: FnScopeKey {
        name: "f".to_string(),
        param_types: vec!["i32".to_string()],
        return_type: "nothing".to_string(),
      },
    };
    {
      let module_path = manager.current_module_path.clone();
      if !manager.module_exists(&module_path) {
        manager.register_module(&module_path, "");
      }
      if let Ok(mut stmt) = manager.conn.prepare_cached(
        "INSERT OR REPLACE INTO functions \
         (qualified_name, simple_name, return_type, module_path, is_definition) \
         VALUES (?1, ?2, ?3, ?4, ?5)",
      ) {
        let _ = stmt.execute(params!["f_i32", "f", "nothing", &module_path, 1]);
      }
    }
    manager.current_scope = fn_scope.clone();

    // A local `x` = 0 shadows the global.
    let _ = manager.define_variable(
      fn_scope.clone(),
      "x",
      &location,
      "i32",
      None,
      Linkage::Internal,
      true,
    );
    manager.update_variable_const_value("x", Some(ConstValue::Int(0)));
    assert_eq!(
      manager.lookup_variable_const_value("x"),
      Some(ConstValue::Int(0))
    );

    // A global-only name still falls back to the global value from a function.
    assert_eq!(
      manager.lookup_variable_const_value("x"),
      Some(ConstValue::Int(0))
    );

    // Back in global scope, the global `x` is unchanged.
    manager.current_scope = VarScope::Global;
    assert_eq!(
      manager.lookup_variable_const_value("x"),
      Some(ConstValue::Int(42))
    );
  }
}
