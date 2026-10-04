<img src="Lale-logo.jpg" style="float:right" alt="Logo" width="200">

# Lale Standard Library Reference

**Website**: [https://lale-lang.dev](https://lale-lang.dev)

This document is a comprehensive guide to the Lale standard library for users. It starts with tutorial examples and progresses to detailed API reference.

---

## Table of Contents

1. [Overview](#1-overview)
2. [Getting Started](#2-getting-started)
3. [Importing the Standard Library](#3-importing-the-standard-library)
4. [Tutorial: Common Patterns](#4-tutorial-common-patterns)
5. [Module Reference](#5-module-reference)
   - 5.1 [Full Library Module](#51-full-library-module)
   - 5.2 [File I/O Module](#52-file-io-module-phase-1-posix)
6. [Error Handling](#6-error-handling)
7. [Platform-Specific Behavior](#7-platform-specific-behavior)
8. [Building the Standard Library](#8-building-the-standard-library)
9. [Adding New Stdlib Modules](#9-adding-new-stdlib-modules)
10. [API Reference Summary](#10-api-reference-summary)

---

## 1. Overview

The Lale standard library is organized into modules providing specialized functionality.

### User-Facing Public API

These functions are documented for users:

- **Math operations**: Absolute value (`abs`, `absf`) from `std` module
- **String parsing**: `parse_float`, `parse_int`, `parse_uint` from the `core` module (import with `use … from std.core`)
- **File I/O**: Reading, writing, and seeking within files from `file_io` module (POSIX systems only)

### Internal Implementation Details

The stdlib also contains internal functions (prefixed with `__`) that are used by the compiler and runtime:

- `__lale_malloc()` — Memory allocation hook (compiler use)
- `__lale_pow()` — Power function hook (compiler use)
- `__lale_write_stdout()`, `__lale_write_stderr()`, `__lale_error()` — Output hooks (compiler use)

These internal functions are **not intended for user code** and are not documented here. Users should use the `write` language statement for output instead.

For advanced use cases (such as custom entry points without the standard library), see [ARCHITECTURE.md § 4.6 Module System](ARCHITECTURE.md#46-module-system) for details on the `--no_std_lib` flag and providing custom runtime function implementations.

The stdlib is designed following Lale's principles:

- **Explicit**: All operations are clear and visible in code
- **Safe**: No hidden allocations or type coercions
- **Modular**: Organize functionality into logical modules (util, file_io, etc.)
- **Portable**: Works on the interpreter backend

---

## 2. Getting Started

### Quickest Start

The `write` statement is a language built-in, always available without import:

```lale
write "Hello, Lale!"
```

This is **not** a stdlib function—it's a core language feature. The stdlib provides additional functions you import as needed.

### Using Full Library Functions

Import the `std` module to access math functions:

```lale
use abs from std.full

var x as i32 = -5
var abs_x as i32 = abs(x)
write "Absolute value of {x} is {abs_x}"
```

### Using File I/O (Phase 1)

File operations are in the file_io module (POSIX systems only):

```lale
use openFile, readFile, writeFile, closeFile from std.file_io_posix

// Write to a file
var fd as i32 = openFile("output.txt", "w")
if fd < 0 as i32
  write "Error: could not create file"
  return
end if

var written as i64 = writeFile(fd, "Hello, file!")
var close_result as i32 = closeFile(fd)

write "File written successfully"
```

---

## 3. Importing the Standard Library

### Import All Symbols

```lale
use all from std.full
```

This imports all exported symbols from `stdlib/src/full.lale`, including:

- `abs()`, `absf()` — Math operations

### Import Specific Symbols

```lale
use abs from std.full
```

This imports only `abs`, keeping your namespace clean. If you need `absf()`, explicitly import it.

### Import from Core Module

The string-parsing functions live in the `core` module and must be imported before use:

```lale
use all from std.core
// or, selectively:
use parse_float, parse_int, parse_uint from std.core
```

Unlike the compiler built-ins, these are standard-library functions — they are not
implicitly available.

### Import from File I/O Module (Phase 1)

```lale
use openFile, readFile, writeFile, closeFile from std.file_io_posix
```

The `stdlib.file_io` syntax imports from the file_io module within the standard library.

### Conditional Imports (POSIX only)

File I/O is only available on POSIX systems. The module is guarded with `#if #posix`:

```lale
#if #posix
    use openFile, readFile, writeFile, closeFile from std.file_io_posix
#end if
```

---

## 4. Tutorial: Common Patterns

### Pattern 1: Absolute Value

```lale
use abs, absf from std.full

var temperature as i32 = -5
var deviation as f64 = -3.14

var mag_temp as i32 = abs(temperature)
var mag_dev as f64 = absf(deviation)

write "Temperature magnitude: {mag_temp}"
write "Deviation magnitude: {mag_dev}"
```

#### Output

```text
Temperature magnitude: 5
Deviation magnitude: 3.14
```

### Pattern 2: File I/O - Reading a File

```lale
#if #posix
  use openFile, readFile, closeFile from std.file_io_posix

  // Open file for reading
  var fd as i32 = openFile("data.txt", "r")
  if fd < 0 as i32
    write "Error: file not found"
    return
  end if

  // Read up to 1024 bytes
  var content as text = readFile(fd, 1024 as i64)
  var close_result as i32 = closeFile(fd)

  write "File contents:"
  write content
#end if
```

### Pattern 3: File I/O - Creating and Writing

```lale
#if #posix
  use openFile, writeFile, closeFile from std.file_io_posix

  // Create/truncate file for writing
  var fd as i32 = openFile("output.txt", "w")
  if fd < 0 as i32
    write "Error: could not create file"
    return
  end if

  // Write data
  var written as i64 = writeFile(fd, "Line 1\n")
  var written2 as i64 = writeFile(fd, "Line 2\n")
  var close_result as i32 = closeFile(fd)

  write "File written successfully"
#end if
```

### Pattern 4: File I/O - Appending to a File

```lale
#if #posix
  use openFile, writeFile, closeFile from std.file_io_posix

  // Open file in append mode
  var fd as i32 = openFile("log.txt", "a")
  if fd < 0 as i32
    write "Error: could not open file"
    return
  end if

  // Append a line
  var written as i64 = writeFile(fd, "[INFO] Application started\n")
  var close_result as i32 = closeFile(fd)

  write "Log entry written"
#end if
```

---

## 5. Module Reference

### 5.1 Full Library Module

#### Location

`stdlib/src/full.lale`

This module provides essential C bindings and mathematical functions.

#### Math: Absolute Value

##### Functions

- `abs(x: i32) -> i32` — Absolute value of 32-bit integer
- `absf(x: f64) -> f64` — Absolute value of 64-bit float

##### Signature

```lale
export fn abs(x as i32) returns i32
export fn absf(x as f64) returns f64
```

##### Behavior

- For integers: Returns the positive equivalent
- For floats: Returns the positive magnitude
- `abs(-5)` returns `5`
- `absf(-3.14)` returns `3.14`

##### Example

```lale
use abs, absf from std.full

var a as i32 = -10
var b as f64 = -2.5

write abs(a)    // 10
write absf(b)   // 2.5
```

#### String Parsing

These functions are in the **core** module (`stdlib/src/core.lale`). Import them before use, e.g. `use parse_float, parse_int, parse_uint from std.core`.

##### Functions

- `parse_float(input: text) -> f64?` — Parse a string to a 64-bit float. Returns nothing on empty input.
- `parse_int(input: text) -> i64?` — Parse a string to a signed 64-bit integer (base 10). Returns nothing on empty input.
- `parse_uint(input: text) -> u64?` — Parse a string to an unsigned 64-bit integer (base 10). Returns nothing on empty input.

##### Signature

```lale
export fn parse_float(input as text) returns f64?
export fn parse_int(input as text) returns i64?
export fn parse_uint(input as text) returns u64?
```

##### Behavior

- Empty input (`""`) or input with zero length returns an absent optional (`nothing`)
- Invalid input (e.g., `"abc"` for any type, `"1.5"` for integers) returns an absent optional
- Valid numeric input returns a present optional with the parsed value
- Parse functions use `strtod` internally; invalid input is detected by the interpreter returning a sentinel value (`f64::INFINITY`), which the stdlib checks
- `parse_int` and `parse_uint` additionally verify the parsed value is a whole number (no fractional part)
- `parse_uint` rejects negative values

##### Example

```lale
use parse_float from std.core

read userInput as text                    // read line from stdin
var val as f64? = parse_float(userInput)
if val has value
    var num as f64 = value of val         // safe unwrap
    write "Parsed: {num}"
else
    write "invalid input"
end if
```

For attaching physical units to parsed values, multiply by a unit literal:

```lale
var raw as f64 = value of parse_float(userInput)
var m as f64 in <kg> = raw * (1.0 <kg>)       // unitless → <kg>
```

---

### 5.2 File I/O Module (Phase 1 - POSIX)

#### Location

`stdlib/src/file_io_posix.lale`

##### Platform

POSIX systems only (Linux, macOS, BSD)

##### Status

Phase 1 implementation (basic operations)

This module provides file operations built on POSIX syscalls.

#### Overview

File operations follow the pattern: open → read/write → seek → close

##### Available Functions

- `openFile(path: text, mode: text) -> i32` — Open a file
- `readFile(fd: i32, count: i64) -> text` — Read bytes from file
- `writeFile(fd: i32, data: text) -> i64` — Write string to file
- `closeFile(fd: i32) -> i32` — Close file descriptor
- `seekFile(fd: i32, offset: i64, whence: i32) -> i64` — Seek to position

#### Opening Files

##### Function `openFile(path: text, mode: text) -> i32`

##### Parameters

- `path` — File path (string)
- `mode` — One of: `"r"` (read), `"w"` (write, truncate), `"a"` (append), `"rw"` (read+write)

##### Returns

- `>= 0` — File descriptor (success)
- `-1` — Error (file not found, permission denied, etc.)

##### Signature

```lale
export fn openFile(path as text, mode as text) returns i32
```

##### Behavior

- `"r"` — Open existing file for reading. Returns -1 if file doesn't exist.
- `"w"` — Create new file or truncate existing. Always succeeds (creates if needed).
- `"a"` — Open for appending. Creates file if doesn't exist.
- `"rw"` — Open for both reading and writing. Creates if doesn't exist.
- File permissions default to `0644` (readable by all, writable by owner)

##### Example

```lale
use openFile, closeFile from std.file_io_posix

var fd as i32 = openFile("config.txt", "r")
if fd < 0 as i32
  write "Error: cannot open config.txt"
  return
end if

// File is now open, use fd for read/write operations

var close_result as i32 = closeFile(fd)
if close_result != 0 as i32
  write "Error: cannot close file"
end if
```

---

#### Reading Files

##### Function `readFile(fd: i32, count: i64) -> text`

##### Parameters

- `fd` — File descriptor from `openFile()`
- `count` — Maximum bytes to read

##### Returns

- String containing data read (may be shorter than `count`)
- Empty string on error or EOF

##### Signature

```lale
export fn readFile(fd as i32, count as i64) returns text
```

##### Behavior

- Reads up to `count` bytes from current file position
- Returns empty string (`""`) if error occurs
- Returns empty string at end of file
- Current implementation limited to 1024-byte buffer (Phase 1 limitation)
- Advances file position by bytes read

#### Phase 1 Limitations

- Fixed buffer size of 1024 bytes
- Cannot read files larger than 1024 bytes in single call
- Future phases will support dynamic buffer allocation

##### Example

```lale
use openFile, readFile, closeFile from std.file_io_posix

  var fd as i32 = openFile("data.txt", "r")
  if fd < 0 as i32
    write "Cannot open file"
    return
  end if

  // Read up to 512 bytes
  var content as text = readFile(fd, 512 as i64)

  if content == ""
    write "File is empty or error occurred"
  else
    write "Read: {content}"
  end if

  var close_result as i32 = closeFile(fd)
```

---

#### Writing Files

##### Function `writeFile(fd: i32, data: text) -> i64`

##### Parameters

- `fd` — File descriptor from `openFile()`
- `data` — String data to write

##### Returns

- `>= 0` — Number of bytes written
- `-1` — Error

##### Signature

```lale
export fn writeFile(fd as i32, data as text) returns i64
```

##### Behavior

- Writes entire string to file at current position
- Return value is byte count written
- Advances file position
- Returns -1 if error (bad fd, disk full, etc.)

##### Example

```lale
use openFile, writeFile, closeFile from std.file_io_posix

var fd as i32 = openFile("output.txt", "w")
if fd < 0 as i32
  write "Cannot create file"
  return
end if

var line1 as text = "First line\n"
var line2 as text = "Second line\n"

var written1 as i64 = writeFile(fd, line1)
var written2 as i64 = writeFile(fd, line2)

if written1 > 0 as i64 and written2 > 0 as i64
  write "Success: wrote {written1} and {written2} bytes"
else
  write "Error writing to file"
end if

var close_result as i32 = closeFile(fd)
```

---

#### Closing Files

##### Function `closeFile(fd: i32) -> i32`

##### Parameters

- `fd` — File descriptor from `openFile()`

##### Returns

- `0` — Success
- `-1` — Error (bad fd, I/O error during flush)

##### Signature

```lale
export fn closeFile(fd as i32) returns i32
```

##### Behavior

- Closes file descriptor
- Flushes any pending writes
- Returns -1 if descriptor is invalid or already closed
- File descriptor should not be used after closing

##### Important

Always close files to ensure data is written and resources are freed.

##### Example

```lale
use openFile, writeFile, closeFile from std.file_io_posix

var fd as i32 = openFile("data.txt", "w")
if fd < 0 as i32
  return
end if

var written as i64 = writeFile(fd, "Important data\n")

// Always close the file
var close_result as i32 = closeFile(fd)
if close_result != 0 as i32
  write "Warning: close operation failed"
end if
```

---

#### Seeking Within Files

##### Function `seekFile(fd: i32, offset: i64, whence: i32) -> i64`

##### Parameters

- `fd` — File descriptor
- `offset` — Byte offset
- `whence` — Reference point (SEEK_SET, SEEK_CUR, or SEEK_END)

##### Returns

- `>= 0` — New absolute position in file
- `-1` — Error

##### Constants

```lale
export var SEEK_SET as i32 = 0   // Start of file
export var SEEK_CUR as i32 = 1   // Current position
export var SEEK_END as i32 = 2   // End of file
```

##### Signature

```lale
export fn seekFile(fd as i32, offset as i64, whence as i32) returns i64
```

##### Behavior

- Moves file position to specified location
- `SEEK_SET`: Absolute position from start
- `SEEK_CUR`: Relative to current position
- `SEEK_END`: Relative to end of file (use negative offset)
- Returns new position, or -1 on error

##### Example

```lale
use openFile, readFile, seekFile, closeFile, SEEK_SET, SEEK_END from std.file_io_posix

  var fd as i32 = openFile("file.txt", "r")
  if fd < 0 as i32
   return
  end if

  // Read first 10 bytes
  var header as text = readFile(fd, 10 as i64)

  // Seek to beginning
  var pos1 as i64 = seekFile(fd, 0 as i64, SEEK_SET)

  // Seek to 100 bytes from start
  var pos2 as i64 = seekFile(fd, 100 as i64, SEEK_SET)

  // Seek to end of file
  var eof_pos as i64 = seekFile(fd, 0 as i64, SEEK_END)

  write "File size: {eof_pos}"

  var close_result as i32 = closeFile(fd)
```

---

#### File Constants

#### Open Flags

(used internally by openFile)

```lale
export var O_RDONLY as i32 = 0      // Read-only
export var O_WRONLY as i32 = 1      // Write-only
export var O_RDWR as i32 = 2        // Read and write
export var O_APPEND as i32 = 1024   // Append to end
export var O_CREAT as i32 = 64      // Create if not exists
export var O_TRUNC as i32 = 512     // Truncate file
```

These are automatically used by `openFile()` based on mode string.

#### Standard File Descriptors

```lale
export var STDIN_FILENO as i32 = 0   // Standard input
export var STDOUT_FILENO as i32 = 1  // Standard output
export var STDERR_FILENO as i32 = 2  // Standard error
```

#### Seek Constants

```lale
export var SEEK_SET as i32 = 0       // Start of file
export var SEEK_CUR as i32 = 1       // Current position
export var SEEK_END as i32 = 2       // End of file
```

---

## 6. Error Handling

The Lale standard library uses **return-based error handling** (no exceptions).

### Return-Based Errors

All stdlib functions that can fail return explicit error indicators:

| Function      | Success                            | Failure               |
| ------------- | ---------------------------------- | --------------------- |
| `openFile()`  | fd >= 0                            | fd == -1              |
| `readFile()`  | non-empty string (or empty at EOF) | empty string on error |
| `writeFile()` | bytes written > 0                  | returns -1            |
| `closeFile()` | returns 0                          | returns -1            |
| `seekFile()`  | position >= 0                      | returns -1            |

### Error Checking Pattern

Always check return values before proceeding:

```lale
use openFile, closeFile from std.file_io_posix

var fd as i32 = openFile("data.txt", "r")
if fd < 0 as i32
  write "Error: could not open file"
  return
end if

// File is open, safe to use

var close_result as i32 = closeFile(fd)
if close_result != 0 as i32
  write "Warning: close returned error"
end if
```

### Common Error Cases

#### File I/O Errors

- `openFile()` returns -1 if:
  - File doesn't exist (mode "r")
  - Permission denied
  - Path is invalid
  - Too many open files (system limit)

- `readFile()` returns empty string if:
  - File descriptor invalid
  - I/O error occurred
  - End of file reached (also valid)

- `writeFile()` returns -1 if:
  - File descriptor invalid
  - Disk full
  - I/O error

- `closeFile()` returns -1 if:
  - File descriptor invalid or already closed
  - I/O error during flush

### Graceful Degradation

When operations fail, decide whether to:

1. **Abort**: Exit function early with `return`
2. **Retry**: Attempt operation again
3. **Use default**: Continue with fallback value
4. **Log and continue**: Write error, proceed with caution

```lale
use openFile, writeFile, closeFile from std.file_io_posix

fn saveData(filename as text, data as text) returns bool
    var fd as i32 = openFile(filename, "w")
    if fd < 0 as i32
        // Abort: return failure indicator
        return false
    end if

    var written as i64 = writeFile(fd, data)
    var close_result as i32 = closeFile(fd)

    if written < 0 as i64 or close_result != 0 as i32
        // Write failed
        return false
    end if

    return true
end fn
```

---

## 7. Platform-Specific Behavior

### POSIX Systems (Linux, macOS, BSD)

File I/O is fully supported:

- All file operations available
- Uses native POSIX syscalls
- Standard Unix file semantics
- File permissions respect umask

### Windows (Not Yet Supported)

File I/O is not available in Phase 1. Phase 3 will add Windows support via WinAPI.

To conditionally compile for POSIX only:

```lale
#if #posix
    use openFile, readFile, writeFile, closeFile from std.file_io_posix
#end if
```

### Portable Code Pattern

For code that works on both platforms:

```lale
use all from std.full

#if #posix
  use openFile, readFile, writeFile, closeFile from std.file_io_posix

  fn read_config() returns text
    var fd as i32 = openFile("config.txt", "r")
    if fd < 0 as i32
        return ""
    end if
    var content as text = readFile(fd, 1024 as i64)
    var _ as i32 = closeFile(fd)
    return content
  end fn
#end if

#if not #posix
  fn read_config() returns text
    // Fallback for non-POSIX systems
    return ""
  end fn
#end if

var config as text = read_config()
if config == ""
  write "No configuration available"
else
  write "Loaded: {config}"
end if
```

---

## 8. Building the Standard Library

### For Users

The standard library is pre-built and installed with the Lale compiler:

```bash
# Installation includes both executable and stdlib
cargo install lale
```

Both `lale` executable and `std.a` archive are installed together.

### For Developers

To rebuild the stdlib after modifying `stdlib/src/full.lale` or `stdlib/src/file_io_posix.lale`:

#### Debug Mode

```bash
cargo make build-stdlib-debug
```

#### Release Mode

```bash
cargo make build-stdlib-release
```

The build system automatically:

1. Compiles the Lale compiler
2. Uses the compiler to compile stdlib source
3. Outputs `std.a` archive

### Build Modes

- **Debug stdlib**: Faster compilation, slower execution (development)
- **Release stdlib**: Slower compilation, faster execution (production)

Both modes must match the compiler mode:

```bash
# Debug compiler + debug stdlib
cargo build
cargo make build-stdlib-debug

# Release compiler + release stdlib
cargo build --release
cargo make build-stdlib-release
```

---

## 9. Adding New Stdlib Modules

### Module Structure

Stdlib modules are Lale source files in `stdlib/src/`:

```text
stdlib/src/
├── full.lale              # Full-level utility functions
├── file_io_posix.lale    # POSIX file I/O (Phase 1)
├── file_io_windows.lale  # Windows file I/O (future, Phase 3)
└── other_module.lale     # Future modules
```

### Creating a New Module

1. **Create source file**: `stdlib/src/new_module.lale`

2. **Export functions**:

   ```lale
   export fn myFunction(param as i32) returns i32
       return param + 1 as i32
   end fn
   ```

3. **Users import with**:

   ```lale
   use myFunction from std.new_module
   ```

4. **Module compiles** when stdlib is built:

   ```bash
   cargo make build-stdlib-release
   ```

### Module Design Principles

- **Single responsibility**: One module = one functional area
- **Explicit API**: All exports documented
- **Error handling**: Use return codes
- **Conditional compilation**: Use `#if #posix`, `#if #windows` for platform-specific code

### Example: Creating a Math Module

```lale
// stdlib/src/math_extra.lale
// Advanced mathematical functions

// Factorial (recursive)
export fn factorial(n as u32) returns u64
    if n == 0 as u32
        return 1 as u64
    else
        return (n as u64) * factorial(n - 1 as u32)
    end if
end fn

// Power function (for integers)
export fn pow_int(base as i32, exp as u32) returns i64
    var result as i64 = 1 as i64
    var i as u32 = 0 as u32
    loop i from 0 as u32 to exp
        result = result * (base as i64)
    end loop
    return result
end fn
```

Then use it:

```lale
use factorial, pow_int from std.math_extra

var f as u64 = factorial(5 as u32)
write "5! = {f}"

var p as i64 = pow_int(2 as i32, 8 as u32)
write "2^8 = {p}"
```

---

## 10. API Reference Summary

### Quick Lookup

#### Full Library Functions (full.lale)

| Function | Signature      | Returns |
| -------- | -------------- | ------- |
| `abs()`  | `abs(x: i32)`  | `i32`   |
| `absf()` | `absf(x: f64)` | `f64`   |

#### Core Library Functions (core.lale — always loaded)

| Function        | Signature                  | Returns | Notes                             |
| --------------- | -------------------------- | ------- | --------------------------------- |
| `parse_float()` | `parse_float(input: text)` | `f64?`  | Nothing on empty/invalid input    |
| `parse_int()`   | `parse_int(input: text)`   | `i64?`  | Nothing on empty/invalid/non-int  |
| `parse_uint()`  | `parse_uint(input: text)`  | `u64?`  | Nothing on empty/invalid/negative |

#### File I/O Functions (POSIX only)

| Function      | Signature                                     | Returns | Error |
| ------------- | --------------------------------------------- | ------- | ----- |
| `openFile()`  | `openFile(path: text, mode: text)`            | `i32`   | -1    |
| `readFile()`  | `readFile(fd: i32, count: i64)`               | `text`  | ""    |
| `writeFile()` | `writeFile(fd: i32, data: text)`              | `i64`   | -1    |
| `closeFile()` | `closeFile(fd: i32)`                          | `i32`   | -1    |
| `seekFile()`  | `seekFile(fd: i32, offset: i64, whence: i32)` | `i64`   | -1    |

#### File I/O Constants

| Constant        | Value | Purpose                    |
| --------------- | ----- | -------------------------- |
| `SEEK_SET`      | 0     | Seek from start of file    |
| `SEEK_CUR`      | 1     | Seek from current position |
| `SEEK_END`      | 2     | Seek from end of file      |
| `STDIN_FILENO`  | 0     | Standard input             |
| `STDOUT_FILENO` | 1     | Standard output            |
| `STDERR_FILENO` | 2     | Standard error             |

---

## Related Documentation

- **[ARCHITECTURE.md](ARCHITECTURE.md)** — Compiler internals and module system
- **[lale.md](lale.md)** — Language reference and syntax
- **[TODO.md](TODO.md)** — Roadmap and future stdlib modules

---

## Quick Start Examples

### Example 1: Sum of Absolute Values

```lale
use abs from std.full

var values as i32[5] = [-3, 7, -2, 5, -1]
var sum as i32 = 0 as i32

loop i from 1 to 5
  sum = sum + abs(values[i])
end loop

write "Sum of absolute values: {sum}"
```

### Example 2: File Copy (POSIX)

```lale
#if #posix
  use openFile, readFile, writeFile, closeFile from std.file_io_posix

  fn copyFile(source as text, dest as text) returns bool
    var in_fd as i32 = openFile(source, "r")
    if in_fd < 0 as i32
        return false
    end if

    var out_fd as i32 = openFile(dest, "w")
    if out_fd < 0 as i32
        return false
    end if

    var content as text = readFile(in_fd, 1024 as i64)
    var written as i64 = writeFile(out_fd, content)

    var close1 as i32 = closeFile(in_fd)
    var close2 as i32 = closeFile(out_fd)

    return written > 0 as i64
  end fn
#end if
```

---

## Conclusion

The Lale standard library provides essential functionality for everyday programming tasks. As the language evolves, the stdlib will grow with new modules for networking, JSON parsing, regular expressions, and more.

All stdlib modules follow Lale's design principles: **explicit**, **safe**, and **clear**.
