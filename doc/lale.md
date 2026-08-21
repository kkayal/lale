<img src="lale-logo.jpg" align="right" alt="Logo" width="200">

# Lale Programming Language

**Website**: [https://lale-lang.dev](https://lale-lang.dev)

## Overview

Lale is a programming language where your code reads like natural language, easy to learn and to maintain. Built for clarity and safety in scientific and engineering applications. It brings together features rarely found in one language:

- **Physical units**: The compiler checks your units so `inch + cm` is caught as an error—no more imperial/metric mix-ups
- **Math-friendly syntax**: Greek letters (`α`, `β`) and subscript digits (`x₁`, `x₂`) work as variable names—your code matches the formulas
- **No hidden conversions**: Every type change is visible in your code—no surprises
- **Pragmatic memory safety**: A per-function escape rule and compiler-managed strings catch common dangling-pointer errors; raw-pointer dereference stays explicitly unsafe
- **1-based arrays**: Arrays start at index 1, matching mathematical convention
- **Simple scoping**: Just two scopes (global and function-local)—easy to understand where variables live

### Built‑in, not bolted‑on

Lale takes a deliberate stance: features that make code safer or more readable belong in the language itself, not in a library you have to import.

```lale
// Optional types — no import needed, no method calls
fn parse_float(input as str) returns f64?
var result as f64? = parse_float(userInput)
if result has value          // reads like English
    m = value of result
else
    alert "invalid input"    // red Error: prefix, to stderr
end if

// Error stack — collect diagnostics without ceremony
add error "config file not found"
if errors has messages
    warn error messages      // print all at once
end if
```

Other languages implement `Option<T>`, `Result<T,E>`, logging, and error formatting as standard‑library types and functions. Lale bakes them into the grammar. Why?

- **Flatter learning curve.** `if val has value` is English. `if val.is_some()` requires knowing a method name.
- **Compile‑time safety.** The compiler knows about `T?` and can warn when you forget to check it. A library type can't.
- **Zero boilerplate.** No `use std::option` or `import logging`. The tools you use every day are always there.

The test for what becomes grammar vs. stays in the standard library is simple: _does the compiler need to know about it to keep you safe or make the code read naturally?_ If yes, it's language. If it's just data transformation (like parsing JSON), it's library.

## UTF-8 Everywhere: Full Unicode Support

Lale is designed with **UTF-8 as a first-class citizen throughout the compiler pipeline**—from parsing to code generation. This enables scientific computing code to naturally express mathematical concepts directly in source.

### Unicode Identifiers

Identifiers can include:

- **Latin letters** with diacritics: `café`, `naïve`, `résistance`
- **Latin-1 extended** characters: `À`, `é`, `ñ`, `ü`
- **Greek and Coptic letters**: `α`, `β`, `Δx`, `μ`, `σ`, `θ`
- **Cyrillic letters** (Russian, Ukrainian, Serbian, Bulgarian): `Д`, `о`, `л`, `ж`
- **CJK scripts** (Chinese, Japanese, Korean): `中`, `あ`, `한`, and 20,000+ ideographs
- **Hangul** (Korean): `ㄱ`, `ㄴ`, `ㄷ` (Jamo and syllables)
- **Indic scripts** (Devanagari, Bengali, Tamil, Telugu, Kannada, Malayalam, etc.)
- **Arabic and Hebrew**: `ع`, `ש`, `ר`
- **Thai, Lao, Tibetan, Myanmar** and other Southeast Asian scripts
- **Subscript digits**: `x₁`, `y₂`, `z₃` — **a rare feature** that allows variable names to directly mirror mathematical notation
- **Combining diacritical marks**: Combining marks that modify base characters (U+0300-U+036F, U+1AB0-U+1AFF, U+1DC0-U+1DFF, U+20D7)

Lale has **no reserved keywords** — words like `var`, `fn`, `type`, `if`, `loop`, and `return` are valid identifiers. The grammar uses PEG ordered choice to distinguish statement keywords from identifiers: `type()` is a function call, `type Point … end type` is a type definition.

### Subscript Digits: A Unique Feature

Most programming languages forbid subscript digits in identifiers. Lale allows them, enabling code that reads like mathematics:

```lale
// Mathematical notation directly in code
var x₁ as f64 = 2.5
var x₂ as f64 = 3.7
var Δx as f64 = x₂ - x₁
write "Δx = {Δx}"

// Compatible with Greek letters for physical quantities
var α₁ as f64 = 0.5
var α₂ as f64 = 1.5
write "α₁ = {α₁}, α₂ = {α₂}"
```

#### Why This Matters for Scientific Computing

Mathematical papers use subscripts extensively. Lale allows your code to match the notation directly, reducing the cognitive gap between the paper and the implementation. This is especially valuable when:

- Converting algorithms from research publications
- Working with indexed parameters (x₁, x₂, ... xₙ)
- Implementing physics formulas with multiple unknowns
- Building numerical methods with sequential approximations

#### Superscript Digits

Superscript digits are also supported — not as identifier characters, but as a concise power-operator syntax in expressions:

```lale
var m as f64 = 2.0
var v as f64 = 5.0
var E as f64 = 0.5⋅m⋅v²    // v² = v ^ 2, ⋅ = scalar multiply
write "Kinetic energy: {E} J"
```

Unlike subscript digits (which extend identifiers: `x₁` is a variable named `x₁`), superscript digits are operators: `v²` means `v ^ 2`, `x⁻¹` means `x ^ (-1)`, and `x²³` means `x ^ 23`. Only the ten superscript digits (`⁰¹²³⁴⁵⁶⁷⁸⁹`) and the superscript signs (`⁺⁻`) are recognized — other Unicode superscript characters (parentheses `⁽⁾`, letters `ᵃᵇ`, etc.) are not valid operators.

For scalar multiplication, the dot operator `⋅` is equivalent to `*` — use whichever makes your formulas more readable. Spaces around operators are optional. Combined with superscript powers, a formula like `0.5⋅m⋅v²` transcribes directly from a physics textbook.

## Installation

Lale requires a two-step build process: the compiler must be built first, then the standard library archive. Use the automated full build for simplicity.

### Quick Installation (All Platforms)

```bash
# Clone the repository
git clone <lale-repo-url>
cd lale

# Full release build (compiler + standard library)
cargo make build-full-release

# Install compiler and standard library to /usr/local/bin (Unix-like systems)
cp target/release/lale /usr/local/bin/
cp target/release/std.a /usr/local/bin/

# Verify installation
lale --version
```

### Windows Installation

```bash
# Clone the repository
git clone <lale-repo-url>
cd lale

# Full release build (compiler + standard library)
cargo make build-full-release

# Create user bin directory
mkdir %USERPROFILE%\bin

# Copy executable and standard library
copy target\release\lale.exe %USERPROFILE%\bin\
copy target\release\std.a %USERPROFILE%\bin\
```

## CLI Options

### Standard Library Level Selection

Lale supports standard library level selection to control which standard library modules are compiled in:

```bash
# No standard library (only built-in compiler functions)
lale run program.lale --stdlib-level=none

# Core standard library (core.lale, no file I/O)
lale run program.lale --stdlib-level=core

# Full standard library (default)
lale run program.lale
lale run program.lale --stdlib-level=full
```

All commands support standard library level selection:

```bash
lale run program.lale --stdlib-level=none
lale run program.lale --stdlib-level=core
lale watch program.lale --stdlib-level=full
```

### Release Mode

```bash
# Debug mode (default) — `#debug` is true, full error diagnostics
lale run program.lale

# Release mode — `#debug` is false, optimised output
lale run program.lale --release
```

The `--release` flag sets the `#debug` compile-time constant to `false`, which can be used with `#if` for conditional compilation of debug-only code. In release mode, `assert` and `debug` statements are not compiled, and error messages are more concise (no source location details).

### Exit Path Analysis

The compiler can statically trace every code location that can cause the program to
terminate or abort at runtime. This helps audit mission-critical and daemon applications:

```bash
# Show all exit paths (debug mode — includes asserts and debug output)
lale run program.lale --show-exit-paths

# Show exit paths in release mode (excludes asserts and debug, since they are not compiled)
lale run program.lale --show-exit-paths --release
```

#### Reported Exit Sources

| Source          | Example          | When it exits                   |
| --------------- | ---------------- | ------------------------------- |
| `exit program`  | `exit program 1` | Always (explicit)               |
| `assert`        | `assert x > 0`   | Condition is false (debug only) |
| `value of`      | `value of opt`   | Optional is Nothing             |
| Transitive call | `validate(x)`    | Callee has an exit path         |

#### What This Feature Is NOT

> ⚠️ The exit-path report identifies every **definite** exit site in the code —
> places where the compiler can syntactically determine that the program may
> terminate. It is **not a proof** that the program cannot crash. The following
> are NOT detected:
>
> - Array index out-of-bounds at runtime
> - Unsafe pointer dereferences (`value at`)
> - Null-pointer accesses through FFI
> - Division by zero
> - Stack overflow or out-of-memory conditions
>
> For those, treat this report as an audit aid, not a safety guarantee. A clean
> report means none of the **detected** exit categories were found — it does
> not mean the program is crash-free.

#### How It Works

#### No Standard Library Modules

`write`, `warn`, `read`, `debug` (language statements) remain available

- `--stdlib-level=core` — Core standard library only (includes core.lale but not std.lale, no file I/O)
- `--stdlib-level=full` — Full standard library (includes both core.lale and std.lale, default)

**Note**: Built-ins (compiler internals) are always included regardless of level. Builtins are not part of the standard library, they are integral to the compiler infrastructure.

### Manual Two-Step Build (Advanced)

If you prefer more control, you can build each step manually:

#### Step 1: Build the Compiler

```bash
cargo build --release              # Release mode
# or: cargo build                  # Debug mode
```

#### Step 2: Build the Standard Library

```bash
cargo make build-stdlib-release      # Release mode
# or: cargo make build-stdlib-debug  # Debug mode
```

Both steps must use the same build mode (both release or both debug).

## Getting Started

### Your First Program

```lale
write "Hello, World!"
```

Save this as `hello.lale` and run it:

```bash
lale run hello.lale
```

`write`, `warn`, `read`, and `debug` are language statements (keywords) — they are always available without any import. The standard library provides additional functions like file I/O, math functions, and type conversion helpers.

### Running the Compiler

```bash
# Compile and execute using the interpreter
lale run examples/complex.lale

# Display the parsed AST (for debugging)
lale run examples/complex.lale --print-ast

# Display the intermediate representation (IR)
lale run examples/complex.lale --print-ir
```

**Planned (future):** Native code compilation via `lale build` and `lale exec` will be implemented once the interpreter is complete (see `doc/ARCHITECTURE.md` §2).

## Read source from stdin

```bash
echo 'write "hello"' | lale run -
```

### Build Optimization

**Planned (future):** Native code compilation (`lale build` and `lale exec`) will use **O0 (no optimization)** by default initially, with a `--optimize [LEVEL]` flag to control optimization levels (0-3). O3 will become the default for production builds.

### Reading from stdin

Use `-` as the source file to pipe Lale source code through standard input
(not to be confused with the `read` statement which reads data during execution):

```bash
# Simple one-liner (code via stdin)
echo 'write "hello"' | lale run -

# Multi-line input
cat <<EOF | lale run -
var x as i32 = 42
write "Value: {x}"
EOF

# Pipe a file through stdin
cat program.lale | lale run -
```

When reading source code from stdin:

- Errors display `<stdin>` as the file name
- Module imports (`use` statements) resolve relative to the current working directory

To read data during program execution, use the `read` statement (see [I/O Statements](#io-statements))

### Compilation Modes

Lale currently offers one compilation mode, with more planned for the future:

| Mode              | Command                    | Engine  | Startup   | Runtime Speed      | Use Case                    |
| ----------------- | -------------------------- | ------- | --------- | ------------------ | --------------------------- |
| **Interpreter**   | `lale run file.lale`       | Rust IR | 50-100ms* | Slower (interpret) | Development & portability   |
| **Test**          | `lale test file.lale`      | Rust IR | 50-100ms* | Slower (interpret) | Run in-language test suites |
| **stdin**         | `echo '...' \| lale run -` | Rust IR | 50-100ms* | Slower (interpret) | Quick scripts               |
| **AOT build**\*   | `lale build file.lale`     | Native  | —         | ✅ Native speed    | Distribution (planned)      |
| **AOT execute**\* | `lale exec file.lale`      | Native  | —         | ✅ Native speed    | Performance (planned)       |

\*Planned for a future phase — not yet available.

*First run only; subsequent runs are ~20-40% faster with `--cache-dir`

> **Note:** AOT build and AOT execute modes are **planned for a future phase**. They will be implemented once the interpreter is complete (see `doc/ARCHITECTURE.md` §2).

#### Interpreter Mode (`lale run`)

The interpreter is a pure Rust implementation that:

- **Parses** source code to AST
- **Generates** IR (intermediate representation)
- **Executes** instructions directly in software
- **Compiles the standard library on-demand** from `stdlib/src/std.lale` and merges it into your module
- **Requires no external dependencies** — runs on any platform where Rust compiles
- **Supports all language features** including F16, physical units, and Unicode identifiers
- **Trade-off**: ~50-100ms startup overhead for standard library compilation (can be cached)

##### Portability Advantage

Since it's pure Rust, `lale run` works on **any platform where Rust compiles**, without requiring native compilation toolchains, C compiler, or external libraries. This makes it ideal for cross-platform scripts and embedded use cases.

#### Native Code Compilation (`lale exec` and `lale build`) — Planned

Ahead-of-time (AOT) compilation will produce native machine code:

- **Generates** IR from AST
- **Compiles to optimized IR** and produces native machine code
- **Links the standard library** from a pre-compiled archive file (`std.a`)
- **Produces platform-specific binaries** (x86-64, ARM, etc.)
- **Applies full compiler optimizations** (O3 in production builds)
- **No runtime interpretation** — direct execution at native speed
- **Trade-off**: Requires native compilation toolchain and produces binaries specific to the target platform

#### F16 (Half-Precision Float) Support

F16 is supported in the interpreter via software emulation. Native code support is planned for a future phase.

---

## Standard Library

### Importing the Standard Library

Use `use std` to import all standard library symbols, or specify particular symbols:

```lale
use std: abs    // Import specific functions from std
use std         // Import all standard library symbols
```

The standard library provides core functionality including absolute value operations and C library bindings. See **[stdlib.md](stdlib.md)** for complete documentation.

### How the Standard Library Works

The standard library works differently in each compilation mode:

#### For `lale run` (Interpreter)

- **Source code approach**: When you use `use std`, the compiler loads and compiles `stdlib/src/std.lale` to IR on-demand
- **Runtime merging**: The compiled stdlib IR is merged into your module's IR before execution
- **Pure interpreter**: No external binaries needed—the interpreter directly executes the merged IR
- **Startup cost**: ~50-100ms for standard library compilation on first run
- **Caching**: Can be optimized with `--cache-dir` flag to persist the compiled standard library between runs
- **Portability**: Works on any platform where Rust compiles (no native toolchain required)

#### Planned for Native Code Compilation (`lale exec` and `lale build`)

- **Binary linking**: The standard library will be pre-compiled to an archive file (`std.a`)
- **Automatic size optimization**: Only the functions your program actually uses are included in the final binary
- **No runtime compilation**: Pre-compiled binary linked into the final executable
- **Performance**: Full compiler optimizations applied during standard library build
- **Distribution**: A single, self-contained binary with no dependencies

#### Standard Library Build Process

##### Current (Interpreter)

The standard library source (`stdlib/src/`) is compiled on-demand at runtime.

**Planned (future):** When native code compilation is implemented:

1. The compiler itself is built first (just like now)
2. The compiler then compiles `stdlib/src/std.lale` to a pre-compiled library file
3. Both the compiler and the library are placed in the same directory
4. When running `lale exec` or `lale build`, the linker finds `std.a` automatically

### Standard Library Modules

The standard library is organized into focused modules:

- **Core**: `abs()`, `absf()` — Mathematical operations
- **File I/O** (POSIX): `openFile()`, `readFile()`, `writeFile()`, `closeFile()`, `seekFile()`
- **Runtime Support**: Customizable system functions (user-replaceable, see §4.8 of ARCHITECTURE.md)

For detailed documentation on all available functions, see **[stdlib.md](stdlib.md)**.

### Incremental Compilation Cache

For development workflows, Lale supports a persistent symbol cache to speed up recompilation. This is especially useful when iterating on code and running the compiler multiple times.

#### Enabling the Cache

Use the `--cache-dir` flag to specify a cache directory:

```bash
# First run: compiles and creates cache
lale run program.lale --cache-dir .lale-cache

# Second run: reuses cached symbols (20-40% faster)
lale run program.lale --cache-dir .lale-cache
```

The cache is created automatically and stored in a `symbols.db` SQLite database.

#### Watch Mode with Cache

The watch command automatically uses a cache directory for incremental recompilation:

```bash
# Watch with cache (default: .lale-cache)
lale watch program.lale

# Or specify a custom cache directory
lale watch program.lale --cache-dir /tmp/lale-cache
```

As you edit files, the compiler automatically invalidates stale symbols and recompiles incrementally, resulting in faster feedback loops during development.

#### Cache Performance

- **First run**: Full compilation (semantic analysis from scratch)
- **Unchanged files**: ~20-40% faster recompilation (symbols loaded from cache)
- **Modified files**: Automatic cache invalidation + incremental analysis

#### Clearing the Cache

To clear the cache and force a full recompilation:

```bash
# Option 1: Delete the cache directory
rm -rf .lale-cache

# Option 2: Use --clear-cache in watch mode
lale watch program.lale --clear-cache
```

#### When to Use the Cache

- **Development**: Use `--cache-dir` for faster iteration and feedback
- **CI/CD pipelines**: Omit `--cache-dir` for clean, deterministic builds
- **Large projects**: Cache benefits increase with project size
- **Watch mode**: Always beneficial for continuous development

### Custom Entry Points (No Standard Library)

Use the `--stdlib-level=none` flag to disable standard library:

```bash
lale run program.lale --stdlib-level=none
```

This allows you to provide your own implementations of runtime functions and entry point logic.

### Extending the Standard Library (Future)

Full module search paths and custom library locations will be supported via `$LALE_PATH` environment variable in a future phase.

---

## Language Reference

### Variable Definitions

Use `var` to define and initialize a variable:

```lale
var x as u8 = 5
var name as str = "Alice"
var active as bool = true
```

Lale has **no default numeric type** — unlike C (`int`), Rust (`i32`), or Python (`int`). When `var` has no `as type` annotation, the type is inferred from the right-hand side expression. A bare literal like `42` has no type of its own, so `var x = 42` is a compile error (no type to infer from). The programmer must provide context: `var x as u8 = 42`, `var x = 42 as i32`, or `var x = someExpression` where `someExpression` has a known type. Optional variables are declared with the `?` modifier (`var v as f64? = nothing`) and values are auto-wrapped: `var v as f64? = 42 as f64` produces a present optional.

#### Variables Inside Loops

You can define a `var` inside a loop body — its value can change each iteration. There is no performance penalty: the memory for the variable is reserved once, not re-allocated on every pass. This is the same as in C, C++, Rust, Go, and Zig.

Some statements **auto-define** variables without an explicit `var` declaration — the
type is implied by the statement. These work like `var` but with no declaration needed:

| Statement                | Auto-defined variable | Type  |
| ------------------------ | --------------------- | ----- |
| `loop over i as u32 ...` | `i`                   | `u32` |
| `read userInput`         | `userInput`           | `str` |
| `write ... to buf`       | `buf`                 | `str` |
| `warn ... to log`        | `log`                 | `str` |

Use `unsafe decl` to declare a variable without initialization (unsafe):

```lale
unsafe decl buffer as u8[1024]
```

### Data Types

| Type                      | Description                |
| ------------------------- | -------------------------- |
| `u8`, `u16`, `u32`, `u64` | Unsigned integers          |
| `i8`, `i16`, `i32`, `i64` | Signed integers            |
| `f16`, `f32`, `f64`       | Floating point             |
| `bool`                    | Boolean (`true` / `false`) |
| `char`                    | Single character           |
| `str`                     | String                     |
| `byte`                    | Raw byte                   |
| `pointer`                 | Raw pointer (untyped)      |

#### Arrays

Arrays in Lale use **1-based indexing**, following mathematical convention:

```lale
var numbers as i32[10] = [1, 2, 3, 4, 5, 6, 7, 8, 9, 10]

// First element is at index 1 (not 0)
write numbers[1]  // outputs 1
write numbers[5]  // outputs 5

// Uniform initialization
var filled as i32[5] = [fill with 42]
write filled[3]   // outputs 42
```

##### Why 1-Based?

Lale aligns with how scientists and mathematicians think. Sequences in math are indexed from 1, and this reduces off-by-one errors when implementing numerical algorithms from papers. Julia, MATLAB, and Fortran use 1-based indexing for the same reason.

##### Array Initialization

Arrays must be **completely initialized**. Lale provides two ways to initialize arrays:

##### 1. Complete Initialization with Explicit Elements

```lale
var arr as i32[5] = [10, 20, 30, 40, 50]      // All 5 elements specified
var matrix as f64[2][3] = [[1.0, 2.0, 3.0], [4.0, 5.0, 6.0]]
```

##### 2. Uniform Fill with the `fill` Keyword

For initializing all elements with the same value, use the `[fill with value]` syntax:

```lale
var zeros as i32[1000] = [fill with 0]        // 1000 zeros
var ones as f64[100] = [fill with 1.0]        // 100 ones
var buffer as u8[256] = [fill with 255]       // 256 max values
```

##### Arrays with Physical Units

Add `in <unit>` to the type annotation to give every element a physical dimension:

```lale
var temps as f64[3] in <K> = [273.15, 300.0, 310.5]
var positions as f64[10] in <m> = [fill with 0.0]
```

Individual elements carry the array's unit through reads and writes — `temps[1]` has type `f64` in `<K>`.

##### Partial Initialization Is Not Allowed

```lale
var arr as i32[10] = [1, 2, 3]    // ❌ ERROR: Only 3 elements, 10 expected
```

This restriction exists for **safety**: uninitialized array elements would contain garbage values. If you need an uninitialized array, use the `unsafe` keyword:

```lale
unsafe decl buffer as i32[1000]    // Uninitialized - programmer responsible for filling before use
```

##### Fill Syntax Validation

The element type must be compatible with the declared array element type:

```lale
var arr as i32[10] = [fill with 5]       // ✅ Element type matches
var bad as i32[10] = [fill with "x"]     // ❌ ERROR: incompatible type
```

##### Array Bounds Checking

All array accesses are **automatically checked at runtime** to prevent out-of-bounds access. If you attempt to access an invalid index, the program terminates with a clear error message:

```lale
var arr as i32[5] = [10, 20, 30, 40, 50]

write arr[3]       // ✅ OK (valid index in range [1, 5])
write arr[6]       // ❌ Runtime error: index out of bounds
```

##### Error Message Format

```text
ERROR at program.lale:3:10: array index out of bounds (index=6, length=5)
```

The error shows:

- **File and location** where the invalid access occurred
- **Index value** that was attempted
- **Array length** that was exceeded

##### Multiple Dimensions

Arrays with multiple dimensions are checked per-dimension:

```lale
var matrix as i32[3][4] = [[1, 2, 3, 4], [5, 6, 7, 8], [9, 10, 11, 12]]

write matrix[2][3]     // ✅ OK (2 ∈ [1,3], 3 ∈ [1,4])
write matrix[4][1]     // ❌ ERROR: first dimension out of bounds (4 > 3)
write matrix[1][5]     // ❌ ERROR: second dimension out of bounds (5 > 4)
```

**Note**: Compile-time bounds checking for literal indices is also performed by the compiler. The runtime bounds check is a safety net for dynamic indices.

#### Pointers

Lale has a single `pointer` type—all pointers are raw (untyped):

```lale
var bar as f32 = 42.0
var p as pointer = pointer to bar      // create pointer
var val as f32 = unsafe value at p     // dereference (type from context)
var bits as u32 = unsafe value at p    // same pointer, different interpretation
```

##### Why Is `unsafe value at` Unsafe?

Unlike array access (`arr[i]`), which is automatically bounds-checked at runtime, `unsafe value at p` reads or writes the raw memory address stored in `p`. There are no safety nets:

- **No bounds checking**: You can read or write past the end of an allocation
- **No type checking**: The same pointer can be reinterpreted as any type
- **No lifetime checking**: You can read memory after it has been freed (use-after-free)
- **No null checking**: Dereferencing a null pointer is undefined behavior

The `unsafe` keyword is required at every dereference site so that code reviewers can grep for these risks, and so that the programmer pauses before writing raw pointer operations.

The `pointer to` operator creates a pointer to any variable. The `unsafe value at` operator dereferences a pointer, with the target type determined by the assignment context.

##### Why Raw Pointers Instead of Typed Pointers?

1. **Same size on target platform**: All pointers occupy the same number of bytes (8 on 64-bit, 4 on 32-bit). The type of pointed-to data doesn't affect pointer storage.

2. **Type determined at dereference**: The type is explicit where it matters—at the point of use (`var val as f32 = unsafe value at p`). This makes code self-documenting without requiring the pointer declaration to track type information.

3. **Simpler type system**: Avoiding typed pointers (`pointer to T`) eliminates either:
   - Complex structural typing in the type system, or
   - Rust-style lifetime tracking

4. **Explicit unsafe behavior**: Raw pointers make dangerous operations (bit reinterpretation) explicit. The multi-step pointer path creates friction proportional to the danger, naturally guiding you toward safe type conversions (`x as u32`) instead.

5. **FFI compatibility**: Raw pointers map directly to C's `void*`, simplifying foreign function interface without type conversion boilerplate.

##### Type Tracking

While the grammar has only raw pointers, the compiler internally tracks the underlying type of each pointer for validation. The `unsafe cast` operator makes bit reinterpretation explicit while allowing compile-time verification of bit-width compatibility.

For detailed design rationale, see [ARCHITECTURE.md § 5.14 Raw Pointers Only](ARCHITECTURE.md#514-raw-pointers-only).

#### Pointer Arithmetic

Lale supports C-compatible pointer arithmetic for systems programming:

```lale
var ptr as pointer = pointer to arr[1]
var offset as u64 = 8

// Pointer addition/subtraction (returns pointer)
var ptr_next as pointer = ptr + offset      // Move pointer forward by 8 bytes
var ptr_prev as pointer = ptr - offset      // Move pointer backward by 8 bytes

// Pointer difference (returns signed i64)
var diff as i64 = ptr_next - ptr_prev       // Distance between pointers (can be negative)
```

##### Operations Allowed

- `ptr + u64` → `pointer` (move pointer forward)
- `ptr - u64` → `pointer` (move pointer backward)
- `ptr1 - ptr2` → `i64` (compute byte difference, may be negative)

##### Not Allowed

- `u64 + ptr` (addition not commutative for pointers)
- `ptr + ptr` (nonsensical)
- `ptr * u64`, `ptr / u64`, etc. (invalid operations)

##### Bounds Checking

Result pointers must remain in-bounds or one-past-end. Violating this creates undefined behavior. Lale places bounds checking responsibility on the programmer, following Rust's `unsafe` model for systems-level code.

#### Allocation

Lale provides `allocate` and `release` as built-in keywords for heap memory management:

```lale
var buf as pointer = allocate(1024 as u64)   // Allocate 1024 bytes
release buf                                   // Free the allocation
```

- `allocate(size)` — takes a `u64` size expression, returns a `pointer` to newly allocated heap memory.
  Can be used anywhere an expression is expected (variable initializers, function arguments, etc.).
- `release ptr` — takes a pointer expression, frees the corresponding heap allocation.
  Must be used as a statement by itself.

Both keywords compile directly to the `__lale_malloc_u64` / `__lale_free_pointer` FFI hooks.
The `on exit` keyword defers execution of a statement until the current scope exits, enabling automatic cleanup of allocations.

The compiler checks that every `allocate` in a function has a matching `release` or `on exit release`
on all paths. A compile-time warning is emitted if an allocation is never released:

```lale
fn leaky() returns nothing
    var buf as pointer = allocate(1024)
    // ⚠️ warning: 'buf' is allocated via allocate() but never released
end fn
```

Reassigning a pointer variable with `allocate()` without first releasing the previous
allocation also produces a warning:

```lale
buf = allocate(2048)
// ⚠️ warning: reassignment overwrites a previous allocate() without releasing it first
```

#### Memory Safety

> **`str` is a special type.** The Lale compiler takes full responsibility for
> memory management of the `str` type. String embedding, type-to-string
> conversions, and concatenation all allocate heap memory behind the scenes.
> The compiler automatically frees `str` data at scope exit — you never write
> `release` for a string. This is an intentional, documented compromise:
> simplicity of use wins over strict explicitness.
>
> For raw pointer allocations, use `allocate` and `release`:
>
> ```lale
> var buf as pointer = allocate(1024 as u64)
> on exit release buf
> ```
>
> The `on exit` keyword pairs `allocate`/`release` naturally for automatic cleanup at scope exit.
>
> The `str` type is defined in `builtins.lale` (visible, single source of
> truth). The compiler guarantees correct construction, tracking, and
> deallocation. See [TODO.md](TODO.md) § Programmer Silent Errors for
> remaining work.

Lale provides _pragmatic_ memory safety — a narrow, compile-time escape check — without garbage collection or complex lifetime annotations. The rule is simple:

##### References to Local Variables Must Not Escape the Function

| Allowed                                  | Forbidden (enforced)                                                  |
| ---------------------------------------- | --------------------------------------------------------------------- |
| `pointer to x` used within same function | `return pointer to localVar`                                          |
| `pointer to x` passed to function call   | Store `pointer to localVar` in a global                               |
| `return p` where `p` is a parameter      | `return p` or `global = p` where `p` was set to `pointer to localVar` |

> **Known limitation:** storing `pointer to localVar` into a struct/type that
> later escapes is **not** detected. The check is also syntactic in places — it
> does not follow a pointer through an alias such as `q = p`.

This works because:

- Pointers to global variables live forever → always safe.
- Pointers passed as parameters are _assumed_ to come from the caller's scope (longer-lived) → treated as safe. This assumption is **not verified** across a call boundary.
- A pointer variable assigned `pointer to localVar` is tracked and treated as pointing to short-lived data.
- Only `pointer to localVar` creates a pointer to short-lived data → forbidden to escape this function.

> ⚠️ **Inter-procedural escape gap.** The rule is per-function. A callee that
> stores a pointer it received as a parameter is not flagged, even when the
> caller passed `pointer to localVar`. Closing this would require whole-program
> escape analysis, which Lale intentionally does not do.

```lale
var leaked as pointer = null    // global

fn capture(p as pointer)
    leaked = p                  // ⚠️ not flagged: p is a parameter
end fn

fn boom()
    var x as i32 = 42
    capture(pointer to x)       // allowed: pointer-to-local passed to a call
end fn                           // after return, `leaked` dangles
```

> Raw `allocate`/`release` pointers get a compile-time _warning_ for obvious
> double-`release` and use-after-`release` dereference. This is a warning, not
> a guarantee — it is flow-insensitive and does not catch aliased or
> inter-procedural misuse.
>
> The `unsafe` keyword is just a label that marks dangerous operations so you
> can search for them in your code. It does not mean the rest of your code is
> automatically safe.

```lale
// ERROR: Cannot return pointer to local variable
fn bad() returns pointer
    var x as i32 = 42
    return pointer to x  // Compile error!
end fn

// OK: Returning pointer received as parameter
fn ok(p as pointer) returns pointer
    return p
end fn

// OK: Using pointer to local within same function
fn alsoOk() returns i32
    var x as i32 = 42
    var p as pointer = pointer to x
    return unsafe value at p
end fn
```

---

### Physical Units

Lale performs dimensional analysis at compile time. Attach units to numeric values using angle brackets:

```lale
var distance as f64 in <m> = 100
var time as f64 in <s> = 10
var speed as f64 = distance / time    // Inferred: <m/s>
```

#### Unit Syntax

| Syntax      | Meaning                   |
| ----------- | ------------------------- |
| `<m>`       | meters                    |
| `<m/s>`     | meters per second         |
| `<m/s²>`    | meters per second squared |
| `<kg⋅m/s²>` | Newtons (force)           |
| `<m²>`      | square meters             |
| `<m⁻²>`     | inverse square meters     |
| `<kg⁻³>`    | inverse cubic kilograms   |

You can use Unicode superscripts (`²`, `³`, `⁻¹`) or caret notation (`^2`, `^-1`).

The division operator inside angle brackets can be `/`, `÷` (division sign), `⁄` (fraction slash, U+2044), or `∕` (division slash, U+2215). All four produce identical normalizations:

```lale
var a as f64 in <m/s>  = 10
var b as f64 in <m⁄s>  = 10  // U+2044 fraction slash
var c as f64 in <m∕s>  = 10  // U+2215 division slash
var d as f64 in <m÷s> = 10  // U+00F7 division sign
```

Multiplication inside angle brackets can be `*` or `⋅` (dot operator, U+22C5).

#### Unit Rules

| Operation  | Rule                                      | Example                  |
| ---------- | ----------------------------------------- | ------------------------ |
| `+`, `-`   | Units must match                          | `5<m> + 3<m>` ✓          |
| `*`, `⋅`   | Units multiply (equivalent)               | `5<m> * 3<s>` → `<m*s>`  |
| `/`, `÷`   | Units divide (equivalent)                 | `10<m> / 2<s>` → `<m/s>` |
| `^`        | Exponent must be dimensionless            | `5<m> ^ 2` → `<m²>`      |
| Comparison | Units must match, result is dimensionless | `5<m> < 10<m>` ✓         |

```lale
// Compile-time error: cannot add meters and seconds
var bad = distance + time  // ERROR
```

#### Unit Normalization

Lale normalizes units for comparison. These are considered equivalent:

- `kg*m^2/s^2` and `<kg>*<m/s>^2` (both represent Joules)
- `m*s` and `s*m` (multiplication is commutative)

#### Unit Inference and No Unit Stripping (Design Decision)

When assigning a value with a unit to a variable without a declared unit, the unit is **inferred** from the right-hand side. This follows the same inference pattern as types: if the LHS doesn't declare one, the RHS provides it:

```lale
var distance as f64 in <m> = 100
var x as f64 = distance    // x has unit <m> (inferred from distance)
```

##### No Syntax to Strip Units

There is deliberately no way to remove units from a value. Once a value has a unit, it carries that unit through all assignments. This is a fundamental design choice to ensure dimensional safety.

##### Why Units Cannot Be Stripped

1. **Dimensional safety**: Units cannot be accidentally discarded in unsafe casts or conversions
2. **Explicit intent**: If you need a unitless value, you must start with one—no hidden unit loss
3. **Consistency**: Follows the "no implicit conversions" philosophy throughout the language

##### Practical Consequence

You cannot use `unsafe cast` on dimensional values. Bit reinterpretation is only allowed on unitless values, since reinterpreting a dimensional quantity would be physically meaningless:

```lale
var distance as f64 in <m> = 100.0
unsafe cast distance as u64  // ❌ ERROR: cannot cast dimensional value

var unitless as f64 = 100.0
unsafe cast unitless as u64  // ✅ OK: unitless value can be reinterpreted
```

If you need a unitless value, declare it without units from the start:

```lale
var raw as f64 = 100       // unitless (no unit declared or inferred)
var calc as f64 = raw * 2  // unitless (arithmetic on unitless value)
```

##### Attaching Units to Unitless Values

Values from I/O (e.g., `parse_float`) are unitless. To attach a unit, multiply by a dimensional literal of value `1.0`:

```lale
var input as f64 = value of parse_float(userInput)  // unitless
m = input * (1.0 <kg>)                              // now has unit <kg>
```

This leverages the arithmetic rules (unitless ⋅ `<kg>` → `<kg>`) without requiring dedicated syntax. It is explicit about intent: the programmer is choosing to interpret a raw number as having physical dimensions.

#### Runtime Unit Checking

When the exponent in a power operation is a variable (not a compile-time constant), the compiler cannot determine the resulting unit statically:

```lale
var base as f64 = 5 <m>
var n as i32 = 2
var result as f64 in <m^2> = base ^ n  // Unit of 'base ^ n' unknown at compile time
```

In such cases, **Lale generates runtime assertions** to verify unit compatibility. These assertions:

- Are present in **both debug and release builds** (unit safety is not optional)
- Terminate the program with a clear error if units don't match

---

### Vectors and Dot/Cross Product

Lale supports primitive vector types (`vec2 of T`, `vec3 of T`, `vec4 of T`) with built‑in dot and cross product operators. The `of` keyword separates storage composition from physical dimensions (`in <unit>`), avoiding bracket ambiguity with the unit system. This combines first‑class physical units with vector algebra in a way no other language offers.

#### Vector Types

```lale
var position as vec3 of f64 = vec3(1.0, 2.0, 3.0)
var velocity as vec3 of f64 in <m/s> = vec3(5.0, 0.0, 0.0)
```

#### Unicode Vector Notation

Lale supports the combining right arrow above (U+20D7) as a valid identifier character, enabling the standard mathematical notation for vectors:

```lale
var F⃗ as vec3 of f64 in <N>  = vec3(10.0, 0.0, 0.0)
var d⃗ as vec3 of f64 in <m>  = vec3(2.0, 3.0, 0.0)
var τ⃗ as vec3 of f64 in <N*m> = d⃗ cross F⃗
var W  as f64 in <J>          = F⃗ dot d⃗
```

Scalar-to-vector assignment is rejected — `var v as vec3 of f64 = 500` is a compile error. Use `vec3(500, 500, 500)` or `[500, 500, 500]` for explicit initialization.

#### Dot Product

(`u ⋅ v` or `u dot v`): Produces a scalar. Valid for vectors of the same dimension and inner type. The 1D dot product is ordinary scalar multiplication. Units multiply:

```lale
var force as vec3 of f64 in <N> = vec3(10.0, 0.0, 0.0)
var displacement as vec3 of f64 in <m> = vec3(2.0, 3.0, 0.0)
var work as f64 in <J> = force ⋅ displacement     // <N⋅m> = <J>
```

#### Cross Product

(`u ⨯ v` or `u cross v`): Produces a vector perpendicular to both inputs. Defined only for `vec3 of T`. The compiler rejects cross product on `vec2 of T` and `vec4 of T`.

**Important**: The dedicated cross‑product symbol `⨯` (U+2A2F VECTOR OR CROSS PRODUCT) is valid only in expressions, not in unit syntax (cross product of scalar units is meaningless). The generic multiplication sign `×` (U+00D7) and middle dot `·` (U+00B7) are **completely excluded from Lale**. Use `⋅` (U+22C5 DOT OPERATOR) for dot product and unit multiplication (`<kg⋅m/s²>`). `*` between two vectors is also rejected (no "plain multiplication" between vectors).

```lale
var torque as vec3 of f64 in <N⋅m> = displacement ⨯ force
```

#### Compile-Time Safety

The compiler validates operand types, dimensions, and units:

| Operation | Signature                           | Compile check                                       |
| --------- | ----------------------------------- | --------------------------------------------------- |
| `u ⋅ v`   | `vecN of T ⋅ vecN of T → T`         | Same N and inner type T. Valid for all N.           |
| `s ⋅ t`   | `T ⋅ T → T` (scalar dot)            | Both scalars of same type T. Equivalent to `s * t`. |
| `u ⨯ v`   | `vec3 of T ⨯ vec3 of T → vec3 of T` | Both must be `vec3 of T`. `vec2`/`vec4` rejected.   |

#### Scalar–Vector Multiplication

Use `*` (`s * v` or `v * s`), not `⋅`. The dot operator is reserved for the dot product.

**2D cross product — rejected**: The cross product is a 3D vector‑valued operation. `⨯` on `vec2 of T` is a compile error. Users needing a 2D signed area should compute `a.x * b.y − a.y * b.x` explicitly. Rationale: correctness over convenience.

---

### Operators

#### Arithmetic

| Operator             | Description                                       |
| -------------------- | ------------------------------------------------- |
| `+`                  | Addition                                          |
| `-`                  | Subtraction                                       |
| `*`                  | Multiplication                                    |
| `/`, `÷`             | Division                                          |
| `%`                  | Modulo                                            |
| `^`                  | Power (right-associative)                         |
| `²`, `³`, `⁻¹`, etc. | Power via superscript digits (`v²` means `v ^ 2`) |
| `dot`, `⋅`           | Dot product (scalar/vector)                       |
| `cross`, `⨯`         | Cross product (vec3 only)                         |

#### Comparison

| Operator             | Description             |
| -------------------- | ----------------------- |
| `==`                 | Equal                   |
| `!=`, `≠`            | Not equal               |
| `<`, `>`             | Less than, greater than |
| `<=`, `≤`, `>=`, `≥` | Less/greater or equal   |

`=` is assignment (a statement that produces no value), while `==` is equality
(an expression that returns `bool`). Because assignment produces no value,
`if x = 5` is a parse error rather than a silent assignment — the classic C
`=`/`==` footgun cannot occur in Lale.

#### Logical

| Operator | Description          |
| -------- | -------------------- |
| `and`    | Logical AND          |
| `or`     | Logical OR           |
| `xor`    | Logical XOR          |
| `not`    | Logical NOT (prefix) |

#### Bitwise

| Operator               | Description                  |
| ---------------------- | ---------------------------- |
| `invert`               | Bitwise NOT (prefix)         |
| `bitwise and`          | Bitwise AND (infix)          |
| `bitwise or`           | Bitwise OR (infix)           |
| `bitwise xor`          | Bitwise XOR (infix)          |
| `unsigned left shift`  | Unsigned left shift (infix)  |
| `unsigned right shift` | Unsigned right shift (infix) |
| `signed left shift`    | Signed left shift (infix)    |
| `signed right shift`   | Signed right shift (infix)   |

#### Other

| Operator | Description                                                                        |
| -------- | ---------------------------------------------------------------------------------- |
| `~`      | String/array append                                                                |
| `?`      | Propagate optional (postfix) — returns `nothing` from enclosing function if absent |

#### Operator Precedence (lowest to highest)

1. `or`
2. `xor`, `⊻`
3. `and`
4. `bitwise or`
5. `bitwise xor`
6. `bitwise and`
7. `==`, `!=`, `≠`
8. `<`, `>`, `<=`, `≥`, `≤`, `>=`
9. `+`, `-`
10. `*`, `/`, `%`, `÷`, `dot`, `⋅`, `cross`, `⨯`
11. `unsigned left shift`, `unsigned right shift`, `signed left shift`, `signed right shift`
12. `^` (right-associative), `²`, `³`, `⁻¹` (superscript powers)
13. `~` (append)
14. Unary operators (`-`, `not`, `invert`, `#type of`, `#size of`, `#unit of`, `pointer to`, `unsafe value at`, `value of`)
15. Postfix operators (`as T`, `has value`, `has no value`, `?`)

---

### Functions

```lale
fn add(x as i32, y as i32) returns i32
    return x + y
end fn

fn greet(name as str) returns nothing
    write "Hello, {name}!"
end fn
```

#### Copy Parameters

Use `copy` before a parameter to pass it by value rather than by reference:

```lale
fn process(copy x as u32) returns u32
    return x + 1
end fn

fn calc(copy a as u32, b as u32, copy c as f64) returns f64
    return a + b + c
end fn
```

Without `copy`, parameters are passed by reference — the function receives a pointer to the caller's value. With `copy`, the value is copied into the function's own storage, decoupling it from the caller. Use `copy` when you need a local snapshot or when the parameter is a small value where indirection adds overhead.

#### Return Units

Functions can declare a return unit:

```lale
fn getVelocity() returns f64 in <m/s>
    return 9.8<m/s>
end fn
```

#### Function Signatures

Declare a function without implementation (for external linkage):

```lale
fn signature printf(format as str) returns i32
```

#### Export and Import

```lale
export fn publicFunction() returns nothing
    write "visible to other modules"
end fn
```

```lale
import unsafe decl externalCounter as i32
```

---

### Optional Types

Lale provides optional types (`T?`) for fallible operations like parsing user input.

```lale
fn parse_float(input as str) returns f64?
    if input.len == 0
        return          // absent optional
    end if
    return strtod(input.ptr, 0 as pointer)  // present optional
end fn

var val as f64? = parse_float("42.5")
if val has value
    write value of val   // unwrap: prints 42.5
else
    write "invalid input"
end if

if val has no value
    exit program 1
end if
```

- `T?` is a type modifier: `f64?`, `i64?`, `u64?`, `str?`
- `return nothing` in a `T?` function produces an absent optional
- `return expr` in a `T?` function produces a present optional
- `has value` / `has no value` test presence (return `bool`)
- `value of expr` unwraps — runtime abort if absent. Like Rust's `unwrap()` on `None`, skipping the `has value` check is a programmer error that aborts the program.
- **Future**: the compiler will warn when `value of` is used on a variable that has no visible `has value` / `has no value` check in the same scope, catching unguarded unwraps at compile time.
- **The `?` operator**: Propagates `nothing` upward. If the expression is absent, the enclosing function returns `nothing` immediately. At the top level, it prints an error and exits with code 1. Only valid in functions returning `T?` or at the top level.

  ```lale
  fn parse_and_double(input as str) returns f64?
      var val as f64 = parse_float(input)?  // if absent, returns nothing to caller
      return val * 2.0
  end fn

  // Top-level scripts also work:
  var val as f64 = parse_float(input)?
  write "{val}"
  ```

  The `?` operator is sugar for:

  ```lale
  var tmp as f64? = parse_float(input)
  if tmp has no value
      return nothing  // (or exit 1 at top level)
  end if
  var val as f64 = value of tmp
  ```

  In debug mode (default), propagation prints full diagnostics via `__lale_error`. In release mode (`--release`), a brief message like `error: function 'parse_float' returned nothing` is written to stderr. This can be customised using `#if #debug`.

### Error Stack

Lale provides a built-in error stack for collecting diagnostic messages without complicating function signatures.

```lale
read input
var val as f64? = parse_float(input)
if val has no value
    add error "Invalid input"              // push a message
end if

// Pop and inspect the most recent message (also removes it)
if errors has messages
    var msg as str = last error            // pops the most recent
    warn "Problem: {msg}"
end if

// Drain all remaining messages (LIFO order) to stdout or stderr
warn error messages                         // idiomatic: use warn for diagnostics
write error messages                        // also works: prints to stdout
alert error messages                        // prints each with red "Error:" prefix
```

#### Why Decouple Diagnostics from Return Types?

Without the error stack, collecting multiple validation errors would require
either threading error state through every function signature or aborting on the
first failure. The error stack lets each function contribute issues to a shared
LIFO buffer while keeping its signature clean (`returns nothing`). The caller
decides how to surface the collected messages using the tiered drain operators:

```lale
// Validation: each function pushes issues onto the shared error stack

fn validate_port(port as i32) returns nothing
    if port < 0
        add error "validate_port: port {port} is negative"
        return
    end if
    if port > 65535
        add error "validate_port: port {port} exceeds maximum 65535"
    end if
end fn

fn validate_path(path as str) returns nothing
    if path.len == 0
        add error "validate_path: path is empty"
    end if
end fn

// Audit: collect issues to stdout for later review

fn audit_config(port as i32, path as str) returns nothing
    validate_port(port)
    validate_path(path)

    if errors has messages
        write "=== Audit Log ==="
        write error messages             // stdout, no prefix: informational record
    end if
end fn

// Diagnostics: non-fatal warnings during normal operation

fn check_environment() returns nothing
    add error "running without network access"
    if errors has messages
        warn error messages              // stderr, yellow Warning:: degraded but continuing
    end if
end fn

// Shutdown: leftover messages are fatal

fn abort_on_errors() returns nothing
    if errors has messages
        alert error messages
        exit program 1
    end if
end fn
```

All three drain operators consume the entire stack, so they are used in distinct
scenarios rather than chained together:

| Operator               | Channel | Prefix            | When                                       |
| ---------------------- | ------- | ----------------- | ------------------------------------------ |
| `write error messages` | stdout  | (none)            | Audit logs, informational diagnostics      |
| `warn error messages`  | stderr  | yellow `Warning:` | Non-fatal problems, service continues      |
| `alert error messages` | stderr  | red `Error:`      | Fatal conditions, pair with `exit program` |

#### Semantics

- `add error expr` — evaluates `expr` (must be convertible to `str`) and pushes onto a built-in LIFO stack
- `errors has messages` — returns `bool`, true if the stack is non-empty
- `last error` — pops and returns the most recent message as `str`. Returns the literal string `"Nothing"` if the stack is empty.
- `warn error messages` / `write error messages` / `alert error messages` — drains and prints all messages (LIFO: most recent first). Prints `"Nothing"` if the stack is empty. `write` goes to stdout, `warn` and `alert` go to stderr; `alert` prepends a red `Error:` prefix to each message.

Reading is consuming: `last error` removes the message from the stack; `warn error messages` drains everything. The word `"Nothing"` is the universal sentinel for absence — you see it whenever a slot that could hold a value is empty. Operations on the error stack never fail: an empty stack is not an error, just a state you can observe and react to.

#### Safety

- Future compiler versions will warn when a `T?` value is defined but never checked with `has value` / `has no value`, catching the most common mistake: silently ignoring a fallible result.

#### Best Practices for Descriptive Messages

Include context so the message is useful even when read far from its source:

```lale
// ❌ Poor: no context
add error "invalid input"

// ✅ Good: includes the offending value and what was expected
add error "parse_float: expected a number, got '{input}'"

// ✅ Good: includes function name for traceability
add error "load_config({path}): file not found"
```

The error stack is global — a message pushed in one function is visible everywhere. Use descriptive messages to make the source obvious.

---

### Module System

Lale supports multi-file projects using the `use` statement to import symbols from other modules and the standard library.

#### Importing from Standard Library

Lale provides a standard library with utility functions. Import it explicitly:

```lale
use std: abs

var x as i32 = -10
write "Absolute value: {abs(x)}"
```

##### Design Principle

Standard library functions are imported just like user modules. Nothing is magically available—all dependencies are explicit and visible in the code. This keeps Lale programs easy to understand and auditable.

#### Importing from User Modules

```lale
// Import specific symbols
use math -> lib: add, subtract

// Import all exported symbols (omit import list)
use physics -> gravity

// Subdirectory navigation
use physics -> mechanics -> force: calculate
```

#### Directory Structure and Module Paths

The directory structure maps directly to module names using `->` for subdirectory navigation:

```text
project/
├── main.lale                      # Module: main
├── math/
│   └── lib.lale                  # Module: math -> lib
├── physics/
│   ├── gravity.lale              # Module: physics -> gravity
│   └── mechanics/
│       └── force.lale            # Module: physics -> mechanics -> force
```

##### Import Syntax Rules

- Use `->` (arrow operator) to navigate into subdirectories
- Parent directory access (`..`) is forbidden for security and clarity
- All imports are relative to the importing file's directory
- Example: In `main.lale`, use `physics -> mechanics -> force: calculate` to import from `physics/mechanics/force.lale`

#### Export Symbols

Only symbols marked with `export` are visible to other modules:

```lale
// physics/gravity.lale
export var gravity_constant as f64 = 9.8

export fn calculate_weight(mass as f64) returns f64
    return mass * gravity_constant
end fn

fn internal_helper() returns nothing
    // Not exported—private to this module
end fn
```

#### Module Execution Model

##### Global Code Execution

All code at module scope (outside functions) executes when the module is imported or compiled:

```lale
// physics/gravity.lale
export var gravity_constant as f64 = 9.8

write "Physics module loaded"  // This executes when the module is imported
```

This enables:

- Module initialization code
- Runtime setup (opening files, initializing state)
- Side effects (logging, registration)

Any file can be a module or an entry point—global code always executes.

#### Import Rules and Constraints

##### Import Statement Location

- All `use` statements must be at the beginning of the file, before other code
- Compiler issues a warning if `use` statements appear after non-use code
- This improves readability and makes dependencies immediately visible

##### Circular Imports

- Circular imports are forbidden (compile-time error)
- The compiler enforces a directed acyclic graph (DAG) of module dependencies
- If two modules need shared functionality, extract it into a third module:

```lale
// common/shared.lale
export fn helper() returns nothing
    // shared implementation
end fn

// module_a.lale
use common -> shared: helper

// module_b.lale
use common -> shared: helper
// Both modules can now share functionality without circular dependency
```

#### C Foreign Function Interface (FFI)

Lale enables calling C library functions directly using the `import` keyword. This is essential for:

- Wrapping standard C libraries (math, memory, file I/O)
- Interfacing with system APIs
- Building embedded applications with C dependencies
- Creating no-std environments with custom implementations

##### Importing C Function Signatures

Declare external C functions using `import fn signature`:

```lale
// Math library
import fn signature sqrt(x as f64) returns f64
import fn signature sin(x as f64) returns f64
import fn signature pow(base as f64, exp as f64) returns f64

// Memory operations
import fn signature malloc(size as i64) returns pointer
import fn signature free(ptr as pointer) returns nothing

// File I/O
import fn signature open(path as pointer, flags as i32) returns i32
import fn signature read(fd as i32, buf as pointer, count as i64) returns i64
import fn signature close(fd as i32) returns i32
```

##### Syscalls Are FFI, Not a Keyword

Lale has no dedicated `syscall` keyword — and neither does C. In C, `syscall()`
is a library function from `<unistd.h>`, not a language construct. Lale reaches
the operating system the same way: through `import fn signature` declarations
and the standard-library wrappers that bind them (for example
`file_io_posix.lale` imports POSIX `open`/`read`/`write`/`lseek`/`close`).

If a raw syscall escape hatch is ever needed for a syscall without a wrapper,
it would be exposed as a standard-library `import fn` (mirroring C's
`syscall(number, ...)` function) — not as a keyword — and gated behind Lale's
platform layer, since it is inherently non-portable.

##### Type Mapping: Lale ↔ C

When calling C functions, Lale types map directly to C types:

| Lale Type | C Type     | Notes                   |
| --------- | ---------- | ----------------------- |
| `i32`     | `int32_t`  | Signed 32-bit integer   |
| `i64`     | `int64_t`  | Signed 64-bit integer   |
| `u32`     | `uint32_t` | Unsigned 32-bit integer |
| `u64`     | `uint64_t` | Unsigned 64-bit integer |
| `f64`     | `double`   | 64-bit floating point   |
| `pointer` | `void*`    | Raw untyped pointer     |

**Important**: Lale `str` values are null‑terminated: the byte at `ptr[len]` is always `\0`. This means `s.ptr` can be passed directly to C functions expecting `const char*`. The `len` field carries the byte count — Lale code never scans for the terminator. A bare `char*` from C is not a Lale `str`; use `__lale_construct_str_in_place(ptr, len)` to wrap it with a known length.

##### Standard Library Pattern: Wrapping C Functions

The recommended approach is to wrap C functions with type-safe Lale wrappers:

```lale
// stdlib/math.lale
import fn signature sqrt(x as f64) returns f64
import fn signature pow(base as f64, exp as f64) returns f64

export fn safe_sqrt(x as f64) returns f64
    if x < 0.0
        __lale_error("sqrt of negative number", "math.lale", 7, 5, -1, -1)
    end if
    return sqrt(x)
end fn

export fn power(base as f64, exp as f64) returns f64
    return pow(base, exp)
end fn
```

User code can then safely use the wrappers:

```lale
use std.math: safe_sqrt, power

fn calculate() returns f64
    var result as f64 = safe_sqrt(16.0)
    var squared as f64 = power(result, 2.0)
    return squared
end fn
```

##### Limitations

1. **Variadic functions** — Functions with variable argument lists are not supported
2. **Function pointers** — Callbacks and function pointers require wrapper functions
3. **Type safety** — C function correctness is the programmer's responsibility; Lale cannot verify C types at compile time

##### No Hidden Dependencies

Unlike many languages, all C dependencies in Lale are **explicit**:

- Every `import fn signature` is visible in your code
- You see exactly which C libraries you depend on
- In no-std environments, you can provide custom implementations of any hook
- The 8 runtime hooks (`__lale_write_stdout_pointer_i64`, `__lale_write_stderr_pointer_i64`, `__lale_error_pointer_i64`, `__lale_alert_pointer_i64`, `__lale_exit`, `__lale_pow_f64_f64`, `__lale_malloc_u64`, `__lale_free_pointer`) are user-replaceable

---

### Control Flow

Lale gives you four ways to make decisions. Each has a clear job, so your code is
easy to read and hard to get wrong.

| You want to…                           | Use                           | Arms                      |
| -------------------------------------- | ----------------------------- | ------------------------- |
| Choose between exactly two paths       | `if` / `else`                 | `else` required           |
| Do something only when true            | `when`                        | No `else`                 |
| Find the first condition that is true  | `match` / `when` / `else`     | Evaluated top to bottom   |
| Compare a value against specific cases | `switch` / `case` / `default` | Pattern or equality match |

Two helper words — **`move on`** and **`missing code`** — let you tell the compiler
"I know about this path" when there's nothing to do yet.

`else if` is removed and produces a compiler error. Use `match` or `switch` instead, which are never ambiguous.

#### Definite Assignment

A `var` defined inside a branch is usable after the construct only if it is
defined on **every path** with the **same type and unit**. Otherwise the compiler
rejects the use — it never silently reads an uninitialized value (zero).

```lale
if 2 as i8 == 3 as i8
    var yyy as i8 = 7
else
    move on
end if
debug yyy   // ERROR: 'yyy' is only defined in some branches
```

Define it in every branch and it joins into a single variable:

```lale
if flag
    var yyy as i8 = 7
else
    var yyy as i8 = 8
end if
debug yyy   // OK: 7 or 8
```

The same rule applies to `when`, `match`, `switch`, and `loop` bodies. A `when`
(or loop) body may not run, so a `var` there is never definitely assigned after
it; a `match`/`switch` must define the variable in every arm (and `else`/`default`).
The loop **header** variable (e.g. `i` in `loop over i as i32 …`) stays accessible
after the loop.

#### If / Else — Two-Sided Choice

When you use `if`, you are making a choice with exactly two sides. The compiler
**requires** you to handle both.

```lale
if week_day == "Sunday"
    write "It's my day off!"
else
    write "Time to get to work."
end if
```

##### What if the `else` Is Empty?

Use `move on` when you are skipping the `else` on purpose:

```lale
if user_name == "Admin"
    write "Special admin menu loaded."
else
    move on   // Normal users don't need a special message.
end if
```

Use `missing code` when the path is not yet implemented:

```lale
if button_clicked == "Settings"
    open_settings_menu()
else
    missing code  // Haven't built the other menus yet!
end if
```

#### When — One-Sided Action

Sometimes there is no second side. You just want to do something **if** a condition
is true. If it's not, the program moves on.

```lale
// A guard — bail out on bad input
when buffer is null
    exit program 2
end when

// A side task — the rest of the program doesn't depend on this
when battery_level < 10
    write "Warning: Please plug in your charger!"
end when
```

`when` has no `else`. There is no "No" side. Trying to add `else` or `else if` is
a parser error.

#### Match / When — First Condition That Matches

When you have several ordered conditions and want the first matching condition to
run, use `match`. Your program checks each `when` **in order** and runs the body
of the first one whose condition is true. `else` catches anything not matched.

Once a `when` arm runs, the program jumps to `end match` — the remaining arms are
not checked, even if they would also be true.

```lale
match
    when grade >= 90:
        write "You got an A!"
    when grade >= 80:
        write "You got a B!"
    when grade >= 70:
        write "You got a C!"
    else:
        write "Keep practicing!"
end match
```

##### Why Remove `else if`?

In most languages, nested `if` statements create a dangling-else problem. `else if`
syntax hides this nesting and makes the structure harder to see:

```lale
if grade >= 90
    if attendance >= 80
        write "Honors A"
else
    write "A"          // Does this belong to the first if, or the second?
end if
end if
```

A beginner sees `else` and might think it pairs with `if grade >= 90` (the outer
one). The compiler pairs it with `if attendance >= 80` (the nearest one). This is
a reading mistake that even experienced programmers make.

Lale replaces `else if` chains with `match`. Every arm is at the same level —
there is no nesting, so there is nothing to misread:

```lale
match
    when grade >= 90 and attendance >= 80:
        write "Honors A"
    when grade >= 90:
        write "A"
    else:
        write "Keep practicing!"
end match
```

The structure is flat, the order is explicit, and the reader never has to guess.

The `else` at the end catches everything that didn't match. You can use `move on`
or `missing code` there too:

```lale
match
    when code == 200:
        process_ok()
    when code == 301:
        follow_redirect()
    else:
        missing code   // other status codes not handled yet
end match
```

#### Switch / Case — Comparing a Value

When you have one value and want to compare it against several specific cases,
use `switch`. This is the right tool for enums and equality checks.

```lale
switch color
    case Red:
        write "Stop!"
    case Yellow:
        write "Slow down!"
    case Green:
        write "Go!"
    // No default — all variants of Color are handled.
    // Adding default here would be unreachable code.
end switch
```

When you don't handle every possible case, use `default` to catch the rest:

```lale
switch parse_result
    case Ok(document):
        render(document)
    case Warning(msg):
        move on            // warnings are non-fatal
    default:
        // Error variants and any future additions land here
        missing code       // error recovery not yet implemented
end switch
```

When the value is an **open type** — a type whose value space the compiler
cannot enumerate (strings, integers, floats, chars) — `default` is required
because the compiler cannot know every possible value:

```lale
switch day_of_week
    case "Saturday":
        write "Weekend!"
    case "Sunday":
        write "Weekend!"
    default:
        write "School day."
end switch
```

Each `case` matches the value against a specific pattern or literal. `default`
catches anything not matched. Once a case runs, the program jumps to `end switch` —
the remaining cases are not checked.

##### Patterns

Bind variant fields to variables:

```lale
switch p
    case Pair(left, right): write "{left}, {right}"
end switch
```

Use `_` to skip a field you don't need:

```lale
switch shape
    case Circle(_): write "got a circle"
    case Rectangle(w, _): write "width: {w}"
    case Point: write "point"
end switch
```

##### Exhaustiveness

`switch` must be exhaustive.

For enums, the compiler knows all possible variants. If you handle every one
of them, `default` is not needed and would be unreachable. If you skip some,
`default` catches the rest.

| All variants covered | `default` present | Result                       |
| -------------------- | ----------------- | ---------------------------- |
| Yes                  | No                | \u2705                       |
| Some                 | Yes               | \u2705                       |
| Yes                  | Yes               | \u274c unreachable `default` |
| No                   | No                | \u274c missing variants      |
| No variants          | Yes               | \u274c no-op                 |

Use `default` as a catch-all for remaining variants:

```lale
switch shape
    case Circle(r): write "radius: {r}"
    default: write "not a circle"
end switch
```

For **open types** — strings, integers, floats, chars — `default` is required
because the compiler cannot list every possible value.

`bool` is a **closed type**: it has exactly two values, `true` and `false`.
When both are covered, the switch is exhaustive and `default` is neither
required nor allowed:

```lale
switch flag
    case true: write "yes"
    case false: write "no"
end switch
```

Adding `default` when both `true` and `false` are already covered is a compile
error (unreachable code), just like adding `default` to a fully-covered enum.

##### Variables After `end switch`

A variable bound in every arm with the same type can be
used after the switch — like a loop variable:

```lale
switch shape
    case Square(a): write "a = {a}"
    case Rectangle(a, b):
        write "a = {a}, b = {b}"
end switch
write "a = {a}"      // ✅ a is bound in both arms with type f64
write "b = {b}"      // ❌ b is only in Rectangle
```

If a variable has different types across arms, it is usable inside the arms but not after
`end switch`:

```lale
switch shape
    case Square(a): write "{a}"          // a: f64
    case Rectangle(a, b): write "{a}"    // a: i32, b: f64
end switch
// a not accessible here — type differs (f64 vs i32)
```

#### Loops

##### Counted Loop

```lale
loop over i as u32 from 0 to 10
    write i
end loop
```

##### Loop with Entry Condition

The condition after `if` is checked **once** before entering the loop. It is not re-evaluated each iteration:

```lale
loop when should_run
    // body runs only after the one-time entry check passes
    process()
end loop
```

For a loop that re-checks the condition each iteration (like a C `while`), combine entry and exit conditions:

```lale
loop when running
    process()
end loop when not running
```

##### Loop with Exit Condition

```lale
loop
    process()
end loop when shouldContinue
```

##### Infinite Loop

```lale
loop
    // runs forever until exit
end loop
```

#### Loop Control

```lale
loop
    exit loop   // exit the loop
end loop
```

```lale
loop
    rewind      // continue to next iteration
end loop
```

```lale
exit program 0   // exit the program with code
```

#### Assertions

The `assert` statement checks a condition at runtime. If the condition is false, the program prints the assertion text, source location, and exits with code 1. In release mode (`--release`), asserts are disabled and generate no code:

```lale
var divisor as i32 = read_int()
assert divisor != 0      // aborts if zero, ignored in release builds
```

##### Error Output

When an assertion fails, the compiler emits:

```text
Assertion failed: divisor != 0
  at examples/program.lale:12:5
```

#### Testing (`test suite` / `test case`)

Lale has a built-in test framework: named test suites containing named test
cases, written inline at the end of a source file. Run them with
`lale test file.lale`; `lale run` skips them.

```lale
test suite MatrixTests
    var tolerance as f64 = 1.0e-6

    fn nearly_equal(a as f64, b as f64) returns bool
        return (a - b) < tolerance
    end fn

    test case multiplication
        var a as f64 = 2.0
        var b as f64 = 3.0
        assert a * b == 6.0
    end test case

    test case nearly_equal_check
        assert nearly_equal(1.0, 1.0 + tolerance)
    end test case
end test suite
```

Rules:

- **Names are identifiers** — `test suite MatrixTests`, `test case
multiplication` (no quotes). Underscores are allowed.
- **Placement** — a test suite must be the last top-level statement; nothing may
  follow it. A suite needs at least one test case; suites and cases cannot be
  nested.
- **Duplicate names** — duplicate suite names, and duplicate case names within a
  suite, are compile errors.
- **Scope** — a test case behaves like a function: variables declared in a case
  are case-local (invisible to other cases, other suites, and non-test code).
  A test suite may declare `var` and `fn` **before its first `test case`**:
  suite variables are shared by every case (and reachable from suite functions)
  but invisible to non-test code and other suites; suite functions are callable
  from cases and each other. `fn`, `type`, `enum`, and `use` definitions are not
  allowed inside a case body.
- **Invisible module globals** — module-level (global) variables are **not**
  visible from a suite or its cases: reading or writing one is an
  `Undefined variable` / `Assignment to undefined variable` error. Module-level
  **functions** remain callable. Shared mutable state belongs in suite-level
  variables, which a case may read and write (so execution order is an explicit
  suite-author choice, exactly as global state is for regular functions).
- **No auto-run fixtures** — `setup`/`teardown` are just a naming convention, not
  keywords: there are no automatically invoked hooks. Each case calls its
  fixtures explicitly, matching Lale's "everything is explicit" spirit.
- **Comments** — comment lines (and `///` docs) directly before a `test case`
  attach to that case; a standalone comment between cases (or before
  `end test suite`) is preserved as its own node. This is the same
  comment-attachment rule that applies to every statement.

| Mode        | Top-level code (`write`, `loop`, …) | `test suite` bodies | `assert` inside a case    |
| ----------- | ----------------------------------- | ------------------- | ------------------------- |
| `lale run`  | Executes                            | Skipped             | Aborts on failure         |
| `lale test` | Skipped                             | Executes            | Reports `Fail`, continues |

Output is per case: `Pass: <suite> / <case>` or `Fail: <suite> / <case>`
(written to stderr). The exit code is 0 if all cases pass, non-zero if any fail.

**Filtering (`--filter`)**: `lale test --filter <pattern>` runs only matching
suites/cases. The flag is repeatable — `--filter A --filter B` runs cases
matching any of the patterns. A pattern containing `/` (the same separator as
the `Pass: suite / case` output, e.g. `Math/Subtraction`) matches a substring
of the full `suite/case` path; any other pattern matches a substring of the
suite name or the case name. Non-matching cases are still fully validated by
semantic analysis — only execution is skipped — so a filtered run cannot hide
compile errors. If nothing matches, the run exits 0 without executing any case.

**Heap leaks fail the run**: in test mode, any heap allocation that is never
released (an `allocate` without a matching `release`/`on exit release`)
produces a `HEAP LEAK` report and makes the run exit non-zero, so the test
framework can be trusted as a leak detector. `lale run` reports leaks the same
way but does not change the exit code.

#### Move On and Missing Code

Both `move on` and `missing code` work anywhere a no-op or placeholder is needed.
In release builds they generate no instructions. The difference is for the
programmer and for tooling:

| Statement      | Meaning                         | How to find it later               |
| -------------- | ------------------------------- | ---------------------------------- |
| `move on`      | "I decided: nothing to do here" | Code review                        |
| `missing code` | "A gap I will implement later"  | `grep 'missing code'`, IDE markers |

##### Enforcement

- **Debug mode:** `missing code` prints a warning at runtime and continues.
- **Release mode** (`--release`): the compiler **refuses to finish** if any
  `missing code` remains.

---

### Strings and Embedded Values

Lale strings support embedded values using `{...}` syntax:

```lale
var name as str = "Alice"
var age as i32 = 30

write "Hello, {name}! You are {age} years old."
write "Next year: {age + 1}"

// Compiler constants also work in embedded values
write "Built with Lale {#compiler_version}"
write "Code from {#source_file} at line {#source_line}"
write "Function: {#function_name}"
```

Any expression can be embedded, including:

- Variables: `{name}`
- Arithmetic: `{x + y}` or `{price * 1.1}`
- Function calls: `{func(arg)}`
- Compiler constants: `{#compiler_version}`, `{#source_line}`, etc.

#### Float Formatting

When a floating-point value is written or embedded, Lale formats it using
the mainstream convention:

- Plain decimal for everyday magnitudes
- Scientific notation for very large or very small values
- Trailing zeros are stripped

```lale
var whole as f64 = 21.0
var half as f64 = 0.5
var pi as f64 = 3.14
var small as f64 = 0.0001

write whole   // 21
write half    // 0.5
write pi      // 3.14
write small   // 0.0001
```

The boundary between plain and scientific notation is a decimal exponent of
16: magnitudes from `1e-4` (inclusive) up to `1e16` (exclusive) print as
plain decimal, anything outside that range uses scientific notation (for
example `1e20`).

> **Precision note:** The current conversion extracts up to 15 significant
> digits using a digit-by-digit algorithm. Values whose binary representation
> is not exact (such as `3.14159`) may print with a small trailing deviation
> (`3.14158999999999`). Full shortest-round-trip formatting (the Ryu/Grisu
> algorithm) is planned future work.

---

### Unicode Identifier Examples

Lale supports Unicode in identifiers:

```lale
// Greek letters
var α as f64 = 3.14159
var β as f64 = 2.71828
var Δx as f64 = x₁ - x₀

// Subscript digits (within identifiers, not at start)
var x₀ as f64 = 0
var x₁ as f64 = 1
var v₂ as f64 = 9.8

// Hiragana
var こんにちは as str = "hello"
```

---

### I/O Statements

```lale
write "Output to stdout"
warn "Output to stderr"  // prints to stderr in yellow
alert "Fatal condition"   // prints to stderr in red with "Error:" prefix
var m as f64 = 10.0  <kg>
var v as f64 = 5.0  <m/s>
debug 0.5 * m * v^2      // prints debug info with value, type, unit, and source location to stderr
                       // Disabled in release mode (--release), like assert
```

#### Redirecting Output to a Variable

Both `write` and `warn` accept an optional `to var`
clause that captures the output into a string variable instead of writing it to
stdout/stderr. On first use, the target variable is **auto-defined as `str`** — no
explicit `var` declaration is needed. Subsequent uses append to the existing variable.
This works like the `read` statement and like loop variables:

```lale
var name as str = "Alice"
var age as i32 = 30

write "{name} is {age} years old" to message   // auto-defines message as str
write message                                      // prints: Alice is 30 years old

write "Age: " to info                              // auto-defines info as str
write "{age}" to info                              // appends to info
write info                                          // prints: Age: 30
```

The real power of the `to var` clause is composing text from multiple writes —
building log messages, reports, or multi-line output — without managing intermediate
variables. Like `write` to stdout, `write ... to var` appends a newline after each
statement; use `write inline ... to var` to suppress it:

```lale
write "Report for {date}" to report
write "   Items processed: {count}" to report
write "   Errors encountered: {errors}" to report
write report

// Use inline to build a single line piece by piece
write inline "Name: " to line
write inline "{name}" to line
write line    // prints: Name: Alice
```

If the target already exists but is not `str`, the compiler reports an error:

```lale
var n as i32 = 42
write "hello" to n       // ❌ ERROR: Cannot write to 'n': target must be str, but it is i32
```

#### Reading Input

The `read` statement also auto-defines its target as `str` — no `var`
is needed (like loop variables and `write ... to`):

```lale
read userInput
// Type conversion with error handling via optional types:
var val as f64? = parse_float(userInput)
if val has value
    write value of val
else
    write "invalid input"
end if
```

---

### Compile-Time Statements

Execute conditions at compile time. Compile-time conditions support boolean logic, arithmetic operations, and comparisons on signed integers (i64):

```lale
#if DEBUG
    write "Debug mode enabled"
#end if

#fail "This build configuration is not supported"
#warn "Deprecated feature used"
```

#### Compile-Time Arithmetic

`#if` conditions can evaluate arithmetic expressions at compile time, similar to C's preprocessor `#if` directives. This enables feature flags and configuration based on computed values:

##### Supported Operations on Signed Integers

- Arithmetic: `+`, `-`, `*`, `/`, `%`
- Comparison: `<`, `<=`, `>`, `>=`, `==`, `!=`
- Logical: `and`, `or`, `xor`, `not`
- Grouping: parentheses `(` `)`

##### Boolean Requirement

Compile-time conditions must be boolean expressions. Integer literals and arithmetic operations require explicit comparison (no implicit int-to-bool conversion). This follows Lale's principle of no implicit type conversions.

##### Valid

```lale
#if 5 > 3          // Comparison returns boolean
#if 2 + 3 > 4      // Arithmetic compared, returns boolean
```

##### Invalid (Will Cause Compile Error)

```lale
#if 5              // Integer literal without comparison
#if 2 + 3          // Arithmetic without comparison
#if -5             // Negation without comparison
```

##### Example: Feature Toggles Based on Comparison

```lale
#if 10 > 5 and #posix
    var HAS_ADVANCED_FEATURES as bool = true
    write "Advanced features enabled"
#else
    var HAS_ADVANCED_FEATURES as bool = false
    write "Using basic mode"
#end if
```

##### Example: Conditional Compilation with Arithmetic

```lale
#if 2 + 3 > 4
    var optimization_level as i32 = 3
#else if 2 * 2 == 4
    var optimization_level as i32 = 2
#else
    var optimization_level as i32 = 1
#end if

write "Optimization level: {optimization_level}"
```

##### Operator Precedence

Follows standard mathematical rules:

1. Parentheses `(` `)`
2. Unary operators: `-` (negation), `not`
3. Multiplication/Division/Modulo: `*`, `/`, `%`
4. Addition/Subtraction: `+`, `-`
5. Comparisons: `<`, `<=`, `>`, `>=`, `==`, `!=`
6. Logical AND: `and`
7. Logical OR: `or`

##### Example: Operator Precedence

```lale
// Evaluates as: 2 + (3 * 4) = 14 > 10
#if 2 + 3 * 4 > 10
    var result as str = "true"
#else
    var result as str = "false"
#end if

// Use parentheses for explicit grouping
#if (2 + 3) * 4 > 15
    var combined as str = "true"
#else
    var combined as str = "false"
#end if
```

##### Error Handling

Division by zero and modulo by zero in compile-time expressions are not evaluable and will cause the condition to be treated as unevaluated (neither true nor false branch is guaranteed).

---

### Compiler Constants

Compiler constants provide source location, context, and platform information at compile time:

| Constant            | Type | Value                                              |
| ------------------- | ---- | -------------------------------------------------- |
| `#source_file`      | str  | Current source file path                           |
| `#source_line`      | i32  | Current line number                                |
| `#function_name`    | str  | Current function name (or `"<global>"`)            |
| `#compiler_version` | str  | Compiler version from Cargo.toml                   |
| `#compile_time`     | str  | Compilation timestamp                              |
| `#main`             | bool | Is this the main module?                           |
| `#posix`            | bool | Is running on POSIX system (Unix/Linux/macOS)?     |
| `#windows`          | bool | Is running on Windows?                             |
| `#debug`            | bool | Is the build in debug mode? (false with --release) |

#### Basic Usage

```lale
write #source_file
write #source_line
write #function_name
write #compiler_version
```

#### Debug Logging Framework

```lale
fn debug_log(level as str, message as str) returns nothing
    write level
    write " ["
    write #function_name
    write "]: "
    write message
end fn

fn process_data() returns nothing
    debug_log("DEBUG", "Starting process")
    debug_log("INFO", "Processing complete")
end fn
```

#### Error Reporting with Context

```lale
fn assert_true(condition as bool, message as str) returns nothing
    if not condition
        write "ASSERTION FAILED in "
        write #function_name
        write " at "
        write #source_file
        write ":"
        write #source_line
        write ": "
        write message
    end if
end fn

fn test_calculations() returns nothing
    var x as i32 = 42
    assert_true(x > 0, "x should be positive")
    assert_true(x < 100, "x should be less than 100")
end fn
```

#### Version Information in Output

```lale
write "Built with Lale {#compiler_version}"
write "Code from {#source_file} at line {#source_line}"
write "Function: {#function_name}"
```

#### Platform-Specific Conditional Compilation

```lale
#if #posix
    var path_separator as str = "/"
    write "Running on POSIX system"
#else
    var path_separator as str = "\\"
    write "Running on Windows"
#end if

write "Path separator: {path_separator}"
```

#### Combining Multiple Constants

```lale
#if #posix and #source_file == "production.lale"
    write "Production build on Unix/Linux/macOS"
#else if #windows and #source_file == "production.lale"
    write "Production build on Windows"
#else
    write "Development build"
#end if
```

The `#posix` and `#windows` constants enable platform-specific code paths without requiring separate compilation. Only the selected branch is included in the final compiled program.

---

### Introspection Operators

| Operator        | Description            |
| --------------- | ---------------------- |
| `#type of expr` | Returns type as string |
| `#size of expr` | Returns size in bytes  |
| `#unit of expr` | Returns unit as string |

```lale
var x as i32 = 42
write #type of x      // "i32"
write #size of x      // 4
write #unit of speed  // "m/s"
```

---

### Type Conversion

#### Lale Requires Explicit Type Conversions

The language does not perform implicit type coercion, ensuring that the programmer's intent is always clear and preventing subtle bugs from hidden conversions.

Use the `as` keyword for **widening** value conversions between numeric types:

```lale
var x as i32 = 42
var y as f64 = x as f64    // Explicit widening: i32 → f64 (OK)
var z as i64 = x as i64    // Explicit widening: i32 → i64 (OK)
```

#### Conversion Rules

| Conversion Type            | Allowed | Example                    |
| -------------------------- | ------- | -------------------------- |
| Widening integer           | ✅      | `i32 as i64`, `u8 as u32`  |
| Widening float             | ✅      | `f32 as f64`               |
| Integer to float           | ✅      | `i32 as f64`, `u32 as f64` |
| Same-width signed↔unsigned | ✅      | `i32 as u32`, `u32 as i32` |
| Narrowing integer          | ❌      | `i64 as i32` (rejected)    |
| Narrowing float            | ❌      | `f64 as f32` (rejected)    |
| Float to integer           | ❌      | `f64 as i32` (rejected)    |
| String ↔ Numeric           | ❌      | `str as i32` (rejected)    |
| Bool ↔ Numeric             | ❌      | `bool as i32` (rejected)   |
| **Numeric → Char**         | **✅**  | **`48 as char` → '0'**     |

#### Semantic Conversions: Numeric to Character

Lale supports **semantic conversions** from numeric values to character type. Unlike narrowing conversions (which are forbidden), semantic conversions are **category changes without data loss**:

```lale
// Convert numeric values to their corresponding Unicode characters
var ascii_0 as char = 48 as char      // '0'
var ascii_A as char = 65 as char      // 'A'
var emoji as char = 0x1F600 as u32    // 😀

// Practical use: integer-to-string conversion
var num as u64 = 123
var str_num as str = __lale_u64_to_str(num)  // "123"
```

##### Why This Is Allowed

Semantic conversion changes the _category_ of the value (number → character), while narrowing loses information _within the same category_ (u64 → u8). The principle: category changes are safe, information loss is not.

##### Valid Range

Unicode codepoints must be in the valid range (0x00000000 to 0x10FFFF).

##### Design Principle

This distinction enables integer-to-string conversion in the standard library. For example, `__lale_u64_to_str()` converts digit values (0-9) to ASCII characters ('0'-'9') using semantic conversion:

```lale
var digit as u64 = n % 10               // 0-9, type u64
var char_code as u64 = 48 as u64 + digit  // ASCII code for '0'-'9'
var char_val as char = char_code as char  // ✓ Semantic conversion
write char_val to buffer
```

No narrowing of numeric information occurs—the numeric value is simply reinterpreted as a Unicode codepoint.

##### Units Are Preserved

Through type conversions:

```lale
var distance as i32 in <m> = 100
var d as f64 = distance as f64    // d has type f64 and unit <m>
```

#### Numeric Literal Conversions

Lale allows numeric literals to convert to any numeric type, as long as the literal value fits within the target type's range (compile-time validation):

```lale
// ✅ ALLOWED - Literals that fit in the target type
var a as u8 = 1 as u8           // 1 fits in u8 (0..255)
var b as u8 = 255 as u8         // 255 is max u8
var c as i8 = -128 as i8        // -128 is min i8

// ❌ NOT ALLOWED - Literals out of range
var d as u8 = 256 as u8         // Error: 256 > 255
var e as i8 = 128 as i8         // Error: 128 > 127
var f as i8 = -129 as i8        // Error: -129 < -128
```

Numeric literals are known at compile time, so the compiler can verify they fit in the target type. This allows natural patterns like `1 as u8` without requiring the user to pick the "right" literal type.

##### Variable Narrowing Is Still Rejected

The compiler cannot verify unknown values fit at compile time:

```lale
var x as i32 = 100
var y as u8 = x as u8           // Error: narrowing conversion rejected
```

#### Unsigned Arithmetic Overflow Detection

When performing subtraction with unsigned integer types (`u8`, `u16`, `u32`, `u64`), the compiler detects potential underflow at compile time:

```lale
var a as u32 = 5
var b as u32 = 10
var c as u32 = a - b    // ❌ ERROR: underflow risk (5 - 10 cannot fit in u32)
                        // Solution: Use signed integers
                        // var c as i32 = 5 as i32 - 10 as i32

// Literal subtraction: compiler can compare values at compile time
var safe as u32 = 100 - 50    // ✅ OK: 100 > 50, no underflow

var a as u32 = 5
var b as u32 = 10
var c as u32 = a - b    // ❌ ERROR: underflow risk (5 < 10)
```

##### Detection Rules

- If both operands are literals: Compare values at compile time. `100 - 50` is safe; `5 - 10` is not.
- If either operand is a variable: Reject conservatively and suggest signed integers. Even `var z as u32 = x - y` where `x` and `y` are both `u32` is rejected — the compiler cannot track variable values at compile time.
- Overflow in addition and multiplication: Similar detection for unsigned types approaching type boundaries

#### Conversion Chains

Conversions chain naturally as a left-to-right pipeline. Each `as` step is validated independently:

```lale
var x as i32 = 42
var result as f64 = x as u32 as f64        // i32 → u32 → f64
```

For readability, intermediate variables can document the conversion strategy:

```lale
var x as i32 = 42
var as_u32 as u32 = x as u32               // Step 1
var result as f64 = as_u32 as f64           // Step 2
```

Helper functions encapsulate frequent patterns:

```lale
fn i32_to_f64(value as i32) returns f64
    var as_u32 as u32 = value as u32
    return as_u32 as f64
end fn
```

#### Pointer Conversions

Lale allows explicit conversion from **unsigned integer types only** (`u8`, `u16`, `u32`, `u64`) to `pointer`:

```lale
// ✅ ALLOWED - Unsigned integers to pointer
var null_ptr as pointer = 0 as pointer
var addr as u64 = 0x7FFF0000
var ptr as pointer = addr as pointer
var small as u8 = 0
var ptr2 as pointer = small as pointer

// ❌ NOT ALLOWED - Signed integers cannot convert to pointers
var x as i32 = 42
var p as pointer = x as pointer  // Error: "Only unsigned integers"
```

##### Rationale

This enables low-level programming patterns like null pointer initialization and FFI interoperability while maintaining type safety by restricting conversions to unsigned integers only.

##### Conditions Must Be Boolean

Unlike C-like languages where any non-zero value is "truthy," Lale requires boolean expressions in conditions:

```lale
var x as i32 = 42
var active as bool = true

if active              // OK: boolean variable
    write "active"
end if

if x > 0               // OK: comparison produces boolean
    write "positive"
end if

// if x                // ERROR: i32 is not boolean
//     write "truthy"  // Lale rejects implicit truth testing
// end if
```

This explicit approach reduces errors by ensuring the source code accurately reflects the program's behavior.

For low-level bit reinterpretation (unsafe), use `unsafe cast` with pointers:

```lale
var bar as f32 = 42.0
var p as pointer = pointer to bar
var bits as u32 = unsafe value at p unsafe cast    // reinterprets f32 bits as u32
```

The `unsafe cast` operator is a postfix operator that follows `unsafe value at`, making the dangerous bit-reinterpretation operation explicit in the code.

##### Restriction

The source variable must be unitless. Bit reinterpretation is meaningless for dimensional values:

```lale
// ❌ ERROR: source has physical unit
var distance as f32 in <m> = 5.0
var p as pointer = pointer to distance
var bits as u32 = unsafe value at p unsafe cast    // Error: cannot unsafe cast value with unit <m>

// ✅ OK: use a unitless value
var raw as f32 = 42.0                       // no unit declared or inferred
var p as pointer = pointer to raw
var bits as u32 = unsafe value at p unsafe cast    // OK: source is unitless
```

Note: Lale infers units from the right-hand side. If you assign a value with a unit to a variable without a declared unit, the unit is inferred (not stripped). There is currently no syntax to strip units from a value.

---

### Unused Symbol Detection

Lale automatically detects and warns about unused variables, functions, parameters, and loop variables. This helps you identify and remove dead code:

```lale
var unused_var as i32 = 5           // warning: Unused variable 'unused_var'
var used_var as i32 = 10
write "Value: {used_var}"

fn unused_fn() returns nothing       // warning: Unused function 'unused_fn'
    write "Hello"
end fn

fn used_fn() returns nothing
    write "World"
end fn

fn has_unused_param(x as i32, y as i32) returns i32  // warning: Unused variable 'y'
    return x + 10
end fn

used_fn()
```

Warnings are **not** emitted for:

- **Exported symbols** (`export var`, `export fn`) — may be used by other modules
- **Imported symbols** (`import var`, `import fn`) — provided externally
- **Variables named `_`** — the underscore name explicitly signals an intentionally discarded value, commonly used when calling functions that return a value you don't need:

  ```lale
  var _ as i32 = closeFile(fd)  // no warning: intentionally discarded
  ```

---

### Comments

Lale supports single-line comments (`//`) and documentation comments (`///`).
Comment/doc attachment follows the same rules for **every** statement:

```lale
// Standalone comment (separated from the next statement by a blank line)

// Leading comment (directly above, no blank line)
var x as i32 = 5   // trailing inline comment

/// Documentation comment attached to the next item
fn documented() returns nothing
    write "I am documented"
end fn
```

- A comment or `///` doc **directly above** a statement (no blank line) becomes
  that statement's leading comment.
- An **inline** comment after a statement becomes its trailing comment.
- A comment **separated by a blank line** is a standalone comment node,
  preserved in source order.

Test suites and test cases follow these same rules: a comment directly above a
`test case` attaches to that case, while a standalone comment between cases (or
before `end test suite`) is preserved as its own node.

---

## Example Program

```lale
/// Calculate kinetic energy
fn kineticEnergy(mass as f64, velocity as f64) returns f64
    return 0.5 * mass * velocity ^ 2
end fn

var m as f64 = 10
var v as f64 = 5

var energy as f64 = kineticEnergy(m, v)
write "Kinetic energy: {energy} Joules"

loop over i as u32 from 1 to 5
    var scaled as f64 = kineticEnergy(m, v * i)
    write "At velocity {v * i}: {scaled} J"
end loop
```

### Unit Mismatch Example

The following example demonstrates a unit mismatch error. The function `kineticEnergy` does not specify units for its parameters and return value, so they are inferred from the units of the arguments at each call site.

Since `m` is in `<g>` and `v` is in `<m/s>`, the function returns `<g*m^2/s^2>`. However, the result is assigned to a variable `energy` with declared units of `<kg*m^2/s^2>`, which does not match. The compiler will raise an error:

```lale
fn kineticEnergy(mass as f64, velocity as f64) returns f64
    return 0.5 * mass * velocity ^ 2
end fn

var m as f64 = 10 <g>
var v as f64 = 5 <m/s>
var energy as f64 in <kg*m^2/s^2> = kineticEnergy(m, v)  // ERROR: unit mismatch
// Function returns <g*m^2/s^2>, not <kg*m^2/s^2>
```

---

## Types

Lale supports **user-defined types** — composite types (similar to C structs) that group multiple fields into a single entity. Types are defined with `type ... end type` and instantiated via auto-generated constructor functions:

```lale
type Person
    name as str
    age as i32
end type

var p as Person = Person("Alice", 30)
write "{p.name} is {p.age} years old"
```

### Fields with Physical Units

Fields can carry physical unit annotations, consistent with Lale’s first-class unit support. The compiler enforces unit matching on constructor arguments:

```lale
type Measurement
    value as f64 in <m>
    tolerance as f64 in <mm>
end type

var m as Measurement = Measurement(5.0 <m>, 0.1 <mm>)   // ✅ units match
// var bad as Measurement = Measurement(5.0 <s>, 1.0)   // ❌ ERROR: unit mismatch
```

Field reads infer the field’s declared unit — `m.value` has type `f64` in `<m>`. A variable without a declared unit picks it up automatically:

```lale
var v as f64 = m.value       // v infers unit <m> from m.value
var w as f64 in <s> = m.value // ❌ ERROR: inferred <m> doesn’t match declared <s>
```

### Field Visibility

Fields can be marked `private` to restrict access to within the defining module. From outside the module, reading or writing a private field produces a compile error:

```lale
type Config
    private api_key as str
    timeout as i32
end type

var cfg as Config = Config("sk-12345", 30)
var timeout as i32 = cfg.timeout       // ✅ public field: always accessible
// var key as str = cfg.api_key         // ❌ ERROR: private field (outside defining module)
```

Within the same module, private fields are freely accessible — the same access rules as all other symbols.

### Error Message

When a private field is accessed from outside its defining module, the compiler emits:

```text
Cannot access private field 'key' of type 'Secure' from outside its defining module 'types'
```

### Planned for Future Phases

- Methods and functions within types

### Composite Type Formatting

Struct types use JSON for both `debug` and embedded values. In `debug`, each field includes `"type"` and optional `"unit"` annotations. In `write`/embedded values, fields contain only `"value"` (and `"unit"` if present):

```lale
type Point
    x as f64
    y as f64
end type
var p = Point(1.5, 2.5)
debug p    // {"Point": {"x": {"value": 1.5, "type": "f64"}, "y": {"value": 2.5, "type": "f64"}}}
write "{p}"   // {"Point": {"x": {"value": 1.5}, "y": {"value": 2.5}}}
```

### Formatting Rules

| Feature                 | `debug`                                           | `write` / embedded values          |
| ----------------------- | ------------------------------------------------- | ---------------------------------- |
| Simple types            | `value as type in <unit>`                         | `value` (with `<unit>` if present) |
| Composite types         | JSON with `"type"` per field, no `as Type` suffix | JSON, values only, no `"type"`     |
| Zero-arg enum variants  | Treated as simple types                           | Treated as simple types            |
| Enum variants with data | `VariantName(jsonFields)`                         | `VariantName(jsonFields)`          |

Composite JSON fields in debug: `{"value": ..., "type": ..., "unit": ...}`. In `write`/embedded values: `{"value": ..., "unit": ...}`. The `"unit"` key only appears when the field has a physical unit.

### Enums

Lale supports **enums** — Rust-style algebraic data types where each variant can carry typed data. Enums are defined with `enum ... end enum` and variants are accessed via the `->` link separator:

```lale
// Multi-line definition
enum Shape
    Circle(f64)
    Rectangle(f64, f64)
    Point
end enum

// Single-line definition (variants separated by whitespace)
enum Color red green blue end enum

var s1 as Shape = Circle(3.14)              // bare name: no path needed
var s2 as Shape = Shape->Rectangle(3.0, 4.0)  // path name: module-qualified via ->
var s3 as Shape = Point
var c  as Color = Color->red
```

#### Variant Constructors

Each variant has an auto-generated constructor function that takes the variant's fields as arguments and returns a value of the enum type. Variants can be called with or without the enum qualifier.

#### Debug Output

The `debug` statement formats enum values as `VariantName(val1, val2, ...)`:

```lale
debug s1  // DEBUG: s1 = Circle(3.14) as Shape
debug s3  // DEBUG: s3 = Point as Shape
```

#### Embedded Values

Enum values format automatically in strings using the same `VariantName(val1, ...)` representation:

```lale
write "Current shape: {s1}"  // outputs: Current shape: Circle(3.14)
write "Color picked: {c}"    // outputs: Color picked: red
```

#### Type Safety

Enum values cannot be compared with non-enum types — the compiler rejects `enum_value > 5` at compile time. Comparisons between enum values of the same type are supported.

#### Pattern Matching

Use the `switch` statement to destructure enum values with exhaustive variant checking and pattern binding. See [§ Switch / Case — Comparing a Value](#switch--case--comparing-a-value).

## Visitor Pattern: Data and Operations

In object-oriented languages such as Java, C#, or C++, the **Visitor Pattern** is a common technique for keeping a data structure separate from the operations performed on it.

For example, a program that works with geometric shapes may need to:

- calculate an area,
- calculate a perimeter,
- print a description,
- export the shape to a file,
- draw it on the screen.

The classic Visitor Pattern stores the data in one place and implements each operation in a separate visitor class. Lale solves the same underlying problem with three simpler ideas:

- **enums** describe the data,
- **functions** describe the operations,
- **`switch`** dispatches on the actual data variant.

### Defining the Data

```lale
enum Shape
    Circle(f64)
    Rectangle(f64, f64)
    Triangle(f64, f64)
end enum
```

A `Shape` holds one of three variants. The enum contains only data — it knows nothing about areas, descriptions, or any other operation.

### Adding an Operation

```lale
fn area(shape as Shape) returns f64
    switch shape

    case Circle(radius):
        return 3.14159265359 ⋅ radius²

    case Rectangle(width, height):
        return width ⋅ height

    case Triangle(base, height):
        return 0.5 ⋅ base ⋅ height

    end switch
end fn
```

The function examines the shape and performs the calculation for the matching variant.

### Adding Another Operation

```lale
fn description(shape as Shape) returns str
    switch shape

    case Circle(radius):
        return "Circle with radius {radius}"

    case Rectangle(width, height):
        return "Rectangle {width} ⋅ {height}"

    case Triangle(base, height):
        return "Triangle {base} ⋅ {height}"

    end switch
end fn
```

Notice that the `Shape` enum did not change.

### Using the Functions

```lale
var shape as Shape = Circle(5.0)

write description(shape)
write "Area: {area(shape)}"
```

```text
Circle with radius 5
Area: 78.53981633975
```

### Why This Matters

Adding an operation is just adding a function — the data structure stays unchanged. Responsibilities stay clear:

- **Enums describe data.**
- **Functions perform work on that data.**

### Comparison with the Classical Visitor Pattern

The visitor pattern usually requires interfaces, virtual methods, visitor classes, and `accept()` methods. Lale reaches the same separation with ordinary functions and `switch`, which is simpler and easier to read.

### Adding a Variant Is Also Safe

The visitor pattern has a famous trade-off called the **expression problem**: adding an operation is easy, but adding a new data variant usually forces every operation to be updated.

Lale turns that trade-off into a safety net. The `switch` statement must be exhaustive over an enum, so adding a variant makes the compiler list every `switch` that needs a new case:

```text
Switch is not exhaustive: missing variant(s) 'Square' of enum 'Shape'. Add the missing case(s) or a default case.
```

That is exactly the change a developer must make, reported precisely instead of discovered at run time.

### Notes on the Example

- `⋅` (U+22C5) is Lale's dot multiplication operator; `*` works too.
- `radius²` uses a superscript digit as a power operator.

## Documentation

- [Compiler Architecture](doc/ARCHITECTURE.md) - For compiler developers
- [TODO List](doc/TODO.md) - Planned features

---

## Review by Google AI Studio

This is an exceptionally well-aligned pair of documents. The **User Guide** does a great job of translating the "strictness" of the **Architecture** into a narrative about "clarity and safety."

However, during a side-by-side audit, I found a few **internal inconsistencies** within the User Guide and some **minor discrepancies** between the User Guide and the Architecture document.

### 1. Inconsistencies with the Architectural Document

#### A. The Status of "Types"

- **Architecture (§ 4.3 & 4.8):** Implies Types/Structs are already part of the AST and IR (giving the example of `struct "Point"`). It lists them as a category of `Stmt`.
- **User Guide (Final Section):** Headed as **"Types"**, stating they are now available.
- **Impact:** The language now uses the `type` keyword instead of `record`.

#### B. Boolean Truthiness in `#if`

- **User Guide (§ Compile-Time Statements):** Explicitly states: "Integer literals and arithmetic operations require explicit comparison (no implicit int-to-bool conversion)." Example: `#if 5` is **Invalid**.
- **Architecture (§ 5.5):** Mentions "Condition Type Safety" for runtime `if` statements, but doesn't explicitly extend this rule to the preprocessor (`#if`).
- **Resolution:** The Architecture should be updated to clarify that the "Explicit Boolean" rule applies to _both_ runtime and compile-time conditions to maintain the "no hidden coercions" principle.

#### C. `f16` Support

- **User Guide (§ Compilation Modes):** States "F16 is only available in AOT backend mode... The interpreter backend uses software emulation which is slower."
- **Architecture (§ 4.7):** Lists `F16` as a standard `IrType` without backend restrictions.
- **Resolution:** If the Interpreter _can_ run it (via emulation), the User Guide shouldn't say it's "only available" in AOT backend, but rather "optimized for AOT backend." If the Interpreter truly doesn't support the instruction, the Architecture should note this backend limitation.

---

### 2. Internal Inconsistencies (User Guide Only)

#### A. Standard Library "Built-ins" vs. Level "None"

- **User Guide (§ CLI Options):** States `--stdlib-level=none` includes "no standard library modules (only built-in compiler functions)."
- **User Guide (§ Getting Started):** States "Lale includes a standard library accessible via `use std`. The core `write` function... [is] available after importing."
- **Correction:** This is contradictory. If `write` is a "built-in compiler function," it should be available even with `--stdlib-level=none` without a `use std` statement. If it requires `use std`, it is part of the library. (The Architecture implies `write` is a language keyword/statement, so it should likely be available without an import).

#### B. Array Initialization Syntax

- **User Guide (§ Arrays):** Show an example: `var numbers as i32[10] = [1, 2, 3, 4, 5, 6, 7, 8, 9, 10]`.
- **User Guide (§ Physical Units):** Later states: "You cannot use `unsafe cast` on dimensional values... Lale infers units from the right-hand side."
- **Question:** If I write `var x as i32[3] = [1<m>, 2<m>, 3<m>]`, is the unit inferred for the whole array? The User Guide doesn't explicitly show the syntax for arrays with units (e.g., `var x as i32[3] in <m>`).

---

### 3. Suggestions for Clarity

#### 1. Pointer Arithmetic & 1-Based Indexing

The Guide is very clear about 1-based indexing for `arr[1]`. However, for **Pointer Arithmetic**, it says `ptr + 8` moves the pointer forward 8 bytes.

- **Warning:** Users coming from 1-based languages (like Fortran) might find the transition to "byte-level" pointer offsets (which are essentially 0-based offsets) jarring. A small note explaining that "Array syntax is 1-based, but Pointer math is byte-addressed" would be helpful.

#### 2. The "Unsafe Cast" Postfix

- **User Guide (§ Type Conversion):** Shows `var bits as u32 = unsafe value at p unsafe cast`.
- **Architecture:** Mentions `unsafe cast` is a postfix operator.

#### 3. Unit "Normalization"

- The User Guide mentions `<kg*m/s²>` and `<kg⁻³>`.
- **Check:** Ensure the documentation clarifies if `<N>` (Newtons) or other derived units are supported as aliases, or if the user _must_ always use base units. (The Architecture implies base-unit normalization, but doesn't mention a "Unit Alias" table).

### Summary Checklist for Update

1. [x] **Fix:** Match the status of "Types" between both docs. (Done: `type` keyword documented, §4.3 reflects current AST)
2. [x] **Clarify:** `write`/`warn`/`read`/`debug` are language keywords — always available, no import needed.
3. [ ] **Update Architecture:** Formally forbid non-boolean `#if` conditions.
4. [ ] **Update User Guide:** Add an example of an **array with physical units** (e.g., `var temps as f64[3] in <K> = [273.15, 300.0, 310.5]`).
