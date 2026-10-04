//! Qualified Names and Natural Keys
//!
//! This module provides type-safe representations of qualified names that form the
//! natural composite keys for the redesigned symbol table schema.
//!
//! **Design Philosophy**: No implicit conversions. All qualified names are explicitly constructed
//! with their components, eliminating ID divergence bugs between IR and symbol table.

use std::fmt;

/// Qualified function name: `simple_name + "_" + param_types.join("_")`
///
/// **C Convention**: Parameter-based naming (NO return type), matching C object file conventions.
///
/// **Examples**:
/// - `"sum_i32_i32"` for `sum(a as i32, b as i32) returns i32`
/// - `"abs_f64"` for `abs(x as f64) returns f64`
/// - `"main"` for `main() returns void`
///
/// **Why no return type?**
/// 1. **C Compatibility**: C object files use parameter-based naming
/// 2. **Semantic Principle**: Return type determined by context, not function identity
/// 3. **Duplicate Detection**: Two functions cannot have same name+params with different return types
#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub struct QualifiedFunctionName(pub String);

impl QualifiedFunctionName {
  /// Construct a qualified function name from simple name and parameter types.
  ///
  /// # Arguments
  /// * `simple_name` - Function name (e.g., "sum", "abs")
  /// * `param_types` - Parameter type strings (e.g., vec!["i32", "i32"])
  ///
  /// # Returns
  /// Qualified name with format: `simple_name` or `simple_name_param1_param2_...`
  ///
  /// # Examples
  /// ```
  /// # use lale::semantic_analysis::QualifiedFunctionName;
  /// let qn = QualifiedFunctionName::new("sum", vec!["i32", "i32"]);
  /// assert_eq!(qn.as_str(), "sum_i32_i32");
  ///
  /// let qn = QualifiedFunctionName::new("main", vec![]);
  /// assert_eq!(qn.as_str(), "main");
  /// ```
  pub fn new(simple_name: &str, param_types: Vec<&str>) -> Self {
    if param_types.is_empty() {
      QualifiedFunctionName(simple_name.to_string())
    } else {
      QualifiedFunctionName(format!("{}_{}", simple_name, param_types.join("_")))
    }
  }

  /// Extract simple name from qualified name
  ///
  /// Given qualified name, returns the first component before the first underscore,
  /// if parameters follow. If no parameters, returns the entire string.
  ///
  /// # Examples
  /// ```
  /// # use lale::semantic_analysis::QualifiedFunctionName;
  /// let qn = QualifiedFunctionName::from("sum_i32_i32");
  /// assert_eq!(qn.simple_name(), "sum");
  ///
  /// let qn = QualifiedFunctionName::from("main");
  /// assert_eq!(qn.simple_name(), "main");
  /// ```
  pub fn simple_name(&self) -> &str {
    // Find the first '_' and return everything before it
    self.0.split('_').next().unwrap_or(&self.0)
  }

  /// Extract parameter types from qualified name
  ///
  /// Given simple_name and qualified_name, extracts parameter types by removing
  /// the "simple_name_" prefix and splitting on '_'.
  ///
  /// # Arguments
  /// * `simple_name` - The simple name component (needed for anchor point)
  ///
  /// # Returns
  /// Vector of parameter type strings
  ///
  /// # Examples
  /// ```
  /// # use lale::semantic_analysis::QualifiedFunctionName;
  /// let qn = QualifiedFunctionName::from("sum_i32_i32");
  /// assert_eq!(qn.param_types("sum"), vec!["i32", "i32"]);
  ///
  /// let qn = QualifiedFunctionName::from("main");
  /// assert_eq!(qn.param_types("main"), Vec::<String>::new());
  /// ```
  pub fn param_types(&self, simple_name: &str) -> Vec<String> {
    let prefix = format!("{}_", simple_name);
    if let Some(param_str) = self.0.strip_prefix(&prefix) {
      param_str.split('_').map(|s| s.to_string()).collect()
    } else {
      Vec::new()
    }
  }

  /// Get the qualified name as a string slice
  pub fn as_str(&self) -> &str {
    &self.0
  }
}

impl From<String> for QualifiedFunctionName {
  fn from(s: String) -> Self {
    QualifiedFunctionName(s)
  }
}

impl From<&str> for QualifiedFunctionName {
  fn from(s: &str) -> Self {
    QualifiedFunctionName(s.to_string())
  }
}

impl fmt::Display for QualifiedFunctionName {
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    write!(f, "{}", self.0)
  }
}

/// Variable key: composite key `(function_qualified_name, simple_name)`
///
/// **Semantics**:
/// - **Global variable**: `function_qualified_name = None`
/// - **Local variable**: `function_qualified_name = Some("parent_fn_qualified_name")`
///
/// **Design Rationale**:
/// - Ensures uniqueness within scope (global or function-local)
/// - Natural composite key (no synthetic IDs)
/// - Easier to query: "Find all variables in function X"
/// - More normalized design
///
/// **Examples**:
/// ```
/// # use lale::semantic_analysis::VariableKey;
/// // Global variable
/// let key = VariableKey::global("PI");
/// assert_eq!(key.function_qualified_name, None);
/// assert_eq!(key.simple_name, "PI");
///
/// // Local variable
/// let key = VariableKey::local("sum_i32_i32", "buffer");
/// assert_eq!(key.function_qualified_name, Some("sum_i32_i32".to_string()));
/// assert_eq!(key.simple_name, "buffer");
/// ```
#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub struct VariableKey {
  /// NULL for globals, qualified function name for locals
  pub function_qualified_name: Option<String>,
  /// Variable name (unique within function or globally)
  pub simple_name: String,
}

impl VariableKey {
  /// Construct a global variable key
  pub fn global(simple_name: &str) -> Self {
    VariableKey {
      function_qualified_name: None,
      simple_name: simple_name.to_string(),
    }
  }

  /// Construct a local variable key
  pub fn local(function_qualified_name: &str, simple_name: &str) -> Self {
    VariableKey {
      function_qualified_name: Some(function_qualified_name.to_string()),
      simple_name: simple_name.to_string(),
    }
  }

  /// Check if this is a global variable
  pub fn is_global(&self) -> bool {
    self.function_qualified_name.is_none()
  }

  /// Check if this is a local variable
  pub fn is_local(&self) -> bool {
    self.function_qualified_name.is_some()
  }
}

impl fmt::Display for VariableKey {
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    match &self.function_qualified_name {
      None => write!(f, "{}", self.simple_name),
      Some(fn_name) => write!(f, "{}::{}", fn_name, self.simple_name),
    }
  }
}

/// Qualified type name (simple name only, globally scoped)
///
/// Types are globally scoped with no nesting. The qualified name is simply
/// the simple name since there's no scope to distinguish them.
///
/// **Examples**: `"Point"`, `"text"`, `"RawMemoryRegion"`
#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub struct QualifiedTypeName(pub String);

impl QualifiedTypeName {
  /// Construct a type definition name
  pub fn new(simple_name: &str) -> Self {
    QualifiedTypeName(simple_name.to_string())
  }

  /// Get the type definition name as a string slice
  pub fn as_str(&self) -> &str {
    &self.0
  }
}

impl From<String> for QualifiedTypeName {
  fn from(s: String) -> Self {
    QualifiedTypeName(s)
  }
}

impl From<&str> for QualifiedTypeName {
  fn from(s: &str) -> Self {
    QualifiedTypeName(s.to_string())
  }
}

impl fmt::Display for QualifiedTypeName {
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    write!(f, "{}", self.0)
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_qualified_function_name_construction() {
    let qn = QualifiedFunctionName::new("sum", vec!["i32", "i32"]);
    assert_eq!(qn.as_str(), "sum_i32_i32");

    let qn = QualifiedFunctionName::new("abs", vec!["f64"]);
    assert_eq!(qn.as_str(), "abs_f64");

    let qn = QualifiedFunctionName::new("main", vec![]);
    assert_eq!(qn.as_str(), "main");
  }

  #[test]
  fn test_qualified_function_name_simple_name() {
    let qn = QualifiedFunctionName::from("sum_i32_i32");
    assert_eq!(qn.simple_name(), "sum");

    let qn = QualifiedFunctionName::from("main");
    assert_eq!(qn.simple_name(), "main");
  }

  #[test]
  fn test_qualified_function_name_param_extraction() {
    let qn = QualifiedFunctionName::from("sum_i32_i32");
    let params: Vec<String> = qn.param_types("sum");
    assert_eq!(params, vec!["i32".to_string(), "i32".to_string()]);

    let qn = QualifiedFunctionName::from("openFile_text_text");
    let params: Vec<String> = qn.param_types("openFile");
    assert_eq!(params, vec!["text".to_string(), "text".to_string()]);

    let qn = QualifiedFunctionName::from("main");
    let params: Vec<String> = qn.param_types("main");
    assert_eq!(params, Vec::<String>::new());
  }

  #[test]
  fn test_variable_key_global() {
    let key = VariableKey::global("PI");
    assert!(key.is_global());
    assert!(!key.is_local());
    assert_eq!(key.simple_name, "PI");
  }

  #[test]
  fn test_variable_key_local() {
    let key = VariableKey::local("sum_i32_i32", "buffer");
    assert!(!key.is_global());
    assert!(key.is_local());
    assert_eq!(key.simple_name, "buffer");
    assert_eq!(key.function_qualified_name, Some("sum_i32_i32".to_string()));
  }

  #[test]
  fn test_variable_key_display() {
    let global = VariableKey::global("PI");
    assert_eq!(global.to_string(), "PI");

    let local = VariableKey::local("sum_i32_i32", "buffer");
    assert_eq!(local.to_string(), "sum_i32_i32::buffer");
  }

  #[test]
  fn test_type_def_name() {
    let rn = QualifiedTypeName::new("Point");
    assert_eq!(rn.as_str(), "Point");
  }
}
