# Lale Programmer Silent Errors

**Date:** 2026-09-11
**Scope:** Errors a Lale programmer can write that compile and run without complaint but produce wrong results.

---

## Severity Legend

| Level                  | Meaning                                                                |
| ---------------------- | ---------------------------------------------------------------------- |
| 🔴 **Silent**          | Compiles, runs, no error, wrong result                                 |
| ✅ **Caught**          | Compile-time error prevents the program from running                   |
| ⚠️ **Compile Warning** | Compile-time warning; the program runs but aborts at runtime if hit    |
| 🟢 **By design**       | Explicitly allowed behind `unsafe` keyword or inherent to the platform |

---

## 🔴 Silent Errors

| #   | Scenario                                            | Example                                                                       | Why silent                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                      |
| --- | --------------------------------------------------- | ----------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| 1   | **Pointer arithmetic out of bounds**                | `var p = pointer to arr[1]; var q = p + 1000` → accesses arbitrary memory     | No runtime bounds checking — neither `pointer to` nor `+` require the `unsafe` keyword, so out-of-bounds pointer arithmetic is undetected undefined behavior                                                                                                                                                                                                                                                                                                                                                                                                                                                                                    |
| 2   | **Memory leak**                                     | `__lale_malloc(1024)` without corresponding `__lale_free`                     | `__lale_free` exists as a runtime hook. `text` memory is compiler-managed and freed automatically when a string reaches a named variable (scope-exit auto-free) or an output statement (inline free after output), plus enum/nested-field reclamation. One case is not covered: a temporary string passed straight to a function argument (`writeFile(fd, "{x}")`) is tracked by neither path and leaks — bind it to a variable first. Raw pointer leaks remain possible only for user-initiated `__lale_malloc` without `__lale_free`; the `on exit` statement now provides deferred cleanup. Leak detector runs on every test in debug builds. |
| 3   | **Type constructor with same-typed fields swapped** | `type Rect width as i32 height as i32` → `Rect(h, w)` instead of `Rect(w, h)` | Constructor arguments are matched by **position**, not by name. Since both fields are `i32`, the type checker accepts them in any order. The program compiles but uses wrong values. With differently-typed fields the swap would be caught (e.g., `Person(30, "John")` fails because `i32` ≠ `text` at position 1). The risk scales with same-typed field count — a `Point3D(x, y, z)` with three `f64` fields has six possible orderings, only one correct.                                                                                                                                                                                    |

---

## ✅ Caught by Compiler — Compile Error

| Scenario                                              | Example code                                                                           | Error message                                                                                                                                                                                                                                |
| ----------------------------------------------------- | -------------------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Variable `as` narrowing                               | `var big as u16 = 300; var small as u8 = big as u8`                                    | "Narrowing conversion not allowed: cannot convert 'u16' (16-bit) to 'u8' (8-bit). Consider widening the narrow data type (e.g., 'u8') instead of trying to narrow the wide data type (e.g., 'u16')"                                          |
| Float to integer (literal or variable)                | `var x as i32 = 42.5`                                                                  | "Type mismatch for 'x': declared type is 'i32' but initializer has type 'f64'. Use explicit conversion: 42.5 as i32"                                                                                                                         |
| Numeric arithmetic narrowing (assignment)             | `var x as u64 = 5; var y as u64 = 10; var result as i64 = x + y`                       | "Type mismatch for 'result': declared type is 'i64' but initializer has type 'u64'. Use explicit conversion: (x + y) as i64"                                                                                                                 |
| Dangling pointer via `return`                         | `fn leak() returns pointer\n    var x as i32 = 1\n    return pointer to x\nend fn`     | "Cannot return pointer to local variable 'x': reference would escape function scope"                                                                                                                                                         |
| Dangling pointer via global assignment                | `var g as pointer\nfn capture()\n    var x as i32 = 1\n    g = pointer to x\nend fn`   | "Cannot assign pointer to local variable 'x' to global 'g': reference would escape function scope"                                                                                                                                           |
| `unsafe bitcast` bit-width mismatch (tracked pointer) | `var p = pointer to i32_val\nvar x as f64 = unsafe value at p unsafe bitcast`          | "Unsafe bitcast bit-width mismatch: pointer points to 'i32' (32-bit) but target type 'f64' is 64-bit"                                                                                                                                        |
| `unsafe bitcast` with unit mismatch                   | `var p = pointer to length_in_meters\nvar x as i32 = unsafe value at p unsafe bitcast` | "Unsafe bitcast requires unitless source: variable 'length_in_meters' has unit 'm'. Bit reinterpretation is meaningless for dimensional values. Assign to a unitless variable first."                                                        |
| Unsigned subtraction underflow                        | `var x as u32 = 5; var y as u32 = 10; var z as u32 = x - y`                            | "Potential underflow in unsigned subtraction: 5 - 10 cannot fit in u32 (result would be negative but u32 cannot represent negative values). Solution: use a wider signed type for the operands: var result as i64 = (x as i64) - (y as i64)" |
| Signed integer overflow (constant)                    | `var a as i8 = 127; var b as i8 = a + 1`                                               | "integer overflow in constant expression 'a + 1' (type 'i8'); use a wider type or an explicit conversion"                                                                                                                                    |
| Unsigned `+`/`*` overflow (constant)                  | `var a as u8 = 255; var b as u8 = a + 1`                                               | "integer overflow in constant expression 'a + 1' (type 'u8'); use a wider type or an explicit conversion"                                                                                                                                    |
| Literal out of range                                  | `var x as u8 = 300`                                                                    | "Literal value 300 is out of range for type 'u8' (valid range: 0..255)"                                                                                                                                                                      |

---

## ⚠️ Caught by Compiler — Compile Warning

| Scenario                         | Example code                                                   | Warning                                                                                                                                                                                        |
| -------------------------------- | -------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `value of` on unchecked optional | `var x as i32 = value of opt` without `if opt has value` guard | "`value of opt` used without a visible `has value` / `has no value` check. If 'opt' is absent, this will abort the program at runtime." (then aborts: "unwrapped absent optional of type i32") |

---

## 🟢 Allowed by Design

| Scenario                      | Example                                           | Rationale                                                                                                                                                                                                                                                                                                                                    |
| ----------------------------- | ------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `value at p` (dereference)    | `var x as i32 = unsafe value at p`                | The `unsafe` keyword is required — the programmer explicitly accepts responsibility for pointer validity                                                                                                                                                                                                                                     |
| `unsafe bitcast`              | `var x as f64 = unsafe value at p unsafe bitcast` | The `unsafe` keyword is required. For tracked pointers, bit-width and unit compatibility are checked at compile time. For untracked pointers (from FFI or pointer arithmetic), the check is skipped — the memory safety checker documents: "The `unsafe` keyword already signals that the programmer accepts responsibility for correctness" |
| Uninitialized variable        | `unsafe decl x as u8`                             | The `unsafe` keyword is required. Normal `var` declarations must include an initializer (`var x as u8 = 5`)                                                                                                                                                                                                                                  |
| `value at assign`             | `unsafe value at p = 42`                          | The `unsafe` keyword is required for writing through a pointer                                                                                                                                                                                                                                                                               |
| Floating-point precision loss | `var x as f16 = 3.14159`                          | Inherent to IEEE 754 floating-point representation — applies equally to F16, F32, and F64. Not Lale-specific                                                                                                                                                                                                                                 |

---

## TODO

Moved to [doc/TODO.md](TODO.md) § Programmer Silent Errors.
