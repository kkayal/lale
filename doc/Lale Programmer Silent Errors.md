# Lale Programmer Silent Errors

**Date:** 2026-08-05
**Scope:** Errors a Lale programmer can write that compile and run without complaint but produce wrong results.

---

## Severity Legend

| Level            | Meaning                                                                |
| ---------------- | ---------------------------------------------------------------------- |
| 🔴 **Silent**    | Compiles, runs, no error, wrong result                                 |
| ✅ **Caught**    | Compile-time error prevents the program from running                   |
| 🟢 **By design** | Explicitly allowed behind `unsafe` keyword or inherent to the platform |

---

## 🔴 Silent Errors

| #   | Scenario                                            | Example                                                                       | Why silent                                                                                                                                                                                                                                                                                                                                                                                                                                                   |
| --- | --------------------------------------------------- | ----------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| 1   | **Signed integer overflow**                         | `127 as i8 + 1` → wraps to -128                                               | No overflow detection anywhere in the compiler                                                                                                                                                                                                                                                                                                                                                                                                               |
| 2   | **Unsigned overflow in addition/multiplication**    | `255 as u8 + 1` wraps to 0; `200 as u8 * 2` → 144                             | Subtraction underflow IS caught by the compiler; `+` and `*` overflow is not                                                                                                                                                                                                                                                                                                                                                                                 |
| 3   | **Pointer arithmetic out of bounds**                | `var p = pointer to arr[1]; var q = p + 1000` → accesses arbitrary memory     | By design — neither `pointer to` nor `+` require the `unsafe` keyword; Lale has no runtime bounds checking for pointers                                                                                                                                                                                                                                                                                                                                      |
| 4   | **Memory leak**                                     | `__lale_malloc(1024)` without corresponding `__lale_free`                     | `__lale_free` exists as a runtime hook. `str` memory is fully compiler-managed and leak-free (auto-free at scope exit, inline free after output, plus enum/nested-field reclamation — verified 0 leaks in the test suite). Raw pointer leaks remain possible only for user-initiated `__lale_malloc` without `__lale_free`; the `on exit` statement now provides deferred cleanup. Leak detector runs on every test in debug builds.                         |
| 5   | **Type constructor with same-typed fields swapped** | `type Rect width as i32 height as i32` → `Rect(h, w)` instead of `Rect(w, h)` | Constructor arguments are matched by **position**, not by name. Since both fields are `i32`, the type checker accepts them in any order. The program compiles but uses wrong values. With differently-typed fields the swap would be caught (e.g., `Person(30, "John")` fails because `i32` ≠ `str` at position 1). The risk scales with same-typed field count — a `Point3D(x, y, z)` with three `f64` fields has six possible orderings, only one correct. |

---

## ✅ Caught by Compiler — Compile Error

| Scenario                                           | Example code                                                                         | Error message                                                                                      |
| -------------------------------------------------- | ------------------------------------------------------------------------------------ | -------------------------------------------------------------------------------------------------- |
| Variable `as` narrowing                            | `var big as u16 = 300; var small as u8 = big as u8`                                  | "Narrowing conversion not allowed: cannot convert 'u16' (16-bit) to 'u8' (8-bit)"                  |
| Dangling pointer via `return`                      | `fn leak() returns pointer\n    var x as i32 = 1\n    return pointer to x\nend fn`   | "Cannot return pointer to local variable 'x': reference would escape function scope"               |
| Dangling pointer via global assignment             | `var g as pointer\nfn capture()\n    var x as i32 = 1\n    g = pointer to x\nend fn` | "Cannot assign pointer to local variable 'x' to global 'g': reference would escape function scope" |
| `unsafe cast` bit-width mismatch (tracked pointer) | `var p = pointer to i32_val\nvar x as f64 = unsafe value at p unsafe cast`           | "Unsafe cast bit-width mismatch: pointer points to 'i32' (32-bit) but target type 'f64' is 64-bit" |
| `unsafe cast` with unit mismatch                   | `var p = pointer to length_in_meters\nvar x as i32 = unsafe value at p unsafe cast`  | "Unsafe cast requires unitless source: variable 'length_in_meters' has unit 'm'"                   |
| Unsigned subtraction underflow                     | `var x as u32 = 5; var y as u32 = 10; var z as u32 = x - y`                          | "Potential underflow in unsigned subtraction: use signed integers (i32) instead"                   |
| Literal out of range                               | `var x as u8 = 300`                                                                  | "Literal value 300 is out of range for type 'u8' (valid range: 0..255)"                            |
| `value of` on unchecked optional                   | `var x as i32 = value of opt` without `if opt has value` guard                       | Warning emitted; runtime panic if `opt` is `nothing`                                               |

---

## 🟢 Allowed by Design

| Scenario                      | Example                                        | Rationale                                                                                                                                                                                                                                                                                                                                    |
| ----------------------------- | ---------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `value at p` (dereference)    | `var x as i32 = unsafe value at p`             | The `unsafe` keyword is required — the programmer explicitly accepts responsibility for pointer validity                                                                                                                                                                                                                                     |
| `unsafe cast`                 | `var x as f64 = unsafe value at p unsafe cast` | The `unsafe` keyword is required. For tracked pointers, bit-width and unit compatibility are checked at compile time. For untracked pointers (from FFI or pointer arithmetic), the check is skipped — the memory safety checker documents: "The `unsafe` keyword already signals that the programmer accepts responsibility for correctness" |
| Uninitialized variable        | `unsafe decl x as u8`                          | The `unsafe` keyword is required. Normal `var` declarations must include an initializer (`var x as u8 = 5`)                                                                                                                                                                                                                                  |
| `value at assign`             | `unsafe value at p = 42`                       | The `unsafe` keyword is required for writing through a pointer                                                                                                                                                                                                                                                                               |
| Floating-point precision loss | `var x as f16 = 3.14159`                       | Inherent to IEEE 754 floating-point representation — applies equally to F16, F32, and F64. Not Lale-specific                                                                                                                                                                                                                                 |

---

## TODO

Moved to [doc/TODO.md](TODO.md) § Programmer Silent Errors.
