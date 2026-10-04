# Lale Runtime ABI Specification

**Status:** Authoritative reference for the Lale runtime ABI and FFI boundary
**ABI version:** 1.0 (first authoritative freeze)
**Snapshot:** Lale v0.1.0

This document freezes the runtime contract shared by the interpreter and future
AOT backends.

**Design decision:** Lale uses a canonical, private **AAPCS64-based internal ABI**
for Lale-to-Lale code, and the **platform C ABI** at the FFI boundary. Internal
determinism is preserved; native interop is direct.

---

## 1. Versioning

Any change to a layout rule or extern signature is a **breaking change** and
requires a major-version bump with a changelog entry.

| Version | Date       | Summary                                                                                                                |
| ------- | ---------- | ---------------------------------------------------------------------------------------------------------------------- |
| 1.0     | 2026-08-16 | First authoritative freeze: AAPCS64 internal ABI + platform C ABI FFI. Supersedes the provisional conservative layout. |

---

## 2. Internal ABI (AAPCS64-based)

The internal ABI is the layout and calling convention used **only** for
Lale-to-Lale code. It is fixed and deterministic.

### Canonical properties

- **Endianness:** little-endian.
- **Pointers:** 8 bytes.
- **Stack alignment:** 16 bytes at call boundaries.
- **Extended float:** 128-bit quad (`f128`, IEEE binary128); no 80-bit x87.
- **Homogeneous aggregates:** structs whose fields are all the same floating-point
  or short-vector type follow AAPCS64 **HFA/HVA** rules and may be passed in
  FP/SIMD registers.

### Type layout

| Type                  | Size / alignment                                       |
| --------------------- | ------------------------------------------------------ |
| `bool`                | 1 byte (0 = false, 1 = true)                           |
| `i8` / `u8`           | 1 byte                                                 |
| `byte`                | 1 byte                                                 |
| `i16` / `u16` / `f16` | 2 bytes, 2-byte aligned                                |
| `i32` / `u32` / `f32` | 4 bytes, 4-byte aligned                                |
| `i64` / `u64` / `f64` | 8 bytes, 8-byte aligned                                |
| `char`                | 4 bytes, 4-byte aligned                                |
| `pointer`             | 8 bytes, 8-byte aligned                                |
| `f128` (future)       | 16 bytes, 16-byte aligned                              |
| array `T[n]`          | `n * sizeof(T)` contiguous bytes                       |
| `vec2<T>`             | `2 * sizeof(T)`, alignment of `T`                      |
| `vec3<T>`             | `3 * sizeof(T)`, alignment of `T`                      |
| `vec4<T>`             | `4 * sizeof(T)`, alignment of `T`                      |
| struct                | fields in declaration order, AAPCS64 padding/alignment |

### Optional type `T?`

`T?` is represented as a two-field struct:

```text
T? = { is_present: bool, value: T }
```

with AAPCS64 field layout (declaration order, natural alignment and padding).

---

## 3. The `text` type

`text` is a **fat pointer** struct:

```text
text = { ptr: pointer, bytes: u64, chars: u64 }
```

- Logical layout: `ptr` at offset 0, `bytes` at offset 8, `chars` at offset 16 (24 bytes total).
- `ptr` points to UTF-8 bytes, null-terminated at `ptr[bytes]`.
- `bytes` is the byte length (O(1) length, binary safety).
- `chars` is the Unicode scalar-value (code point) count.
- At the FFI boundary, `text` is **not** a C type; it is marshaled to `ptr + bytes`
  or a NUL-terminated C string as needed. `chars` is not marshaled.

### 3.1 The `binary` type

```text
binary = { ptr: pointer, bytes: u64 }
```

- Logical layout: `ptr` at offset 0, `bytes` at offset 8 (16 bytes total).
- `ptr` points to raw bytes; `bytes` is the byte length.
- **No** null terminator and **no** code-point counter — these are the only
  differences from `text`.
- `binary` is compiler-managed and owned like `text` (auto-freed at scope exit,
  deep-copied on by-value parameters).
- At the FFI boundary, `binary` is marshaled to `ptr` (a `void*`) plus `bytes`
  (a `size_t`); it has no C representation of its own.

---

## 4. FFI boundary (platform C ABI)

Extern functions declared via `import fn` use the **host platform's C ABI** —
not the internal AAPCS64 ABI.

Extern symbols are **plain C names** — no mangling, no overloading; see
`doc/ARCHITECTURE.md` §4.6 "Symbol Naming: Mangled Internals vs. Plain-C Externs".

- Integer/float args and returns map directly to their C equivalents.
- `pointer` maps to a C pointer.
- `bool` maps to a C `_Bool`-compatible 1-byte value.
- `text` is marshaled (it has no C representation).
- `T?` and other Lale-only types are marshaled to C-compatible forms.

### Allowed extern set

The interpreter dispatches through a single `call_extern`. The allowed externs are:

| Name                  | Signature                                         | Notes                              |
| --------------------- | ------------------------------------------------- | ---------------------------------- |
| `write`               | `(i32 fd, ptr<i8> buf, i64 count) -> i64`         | POSIX write                        |
| `read`                | `(i32 fd, ptr<i8> buf, i64 count) -> i64`         | POSIX read                         |
| `open`                | `(ptr<i8> path, i32 flags, i32 mode) -> i32`      | file descriptor (opaque)           |
| `close`               | `(i32 fd) -> i32`                                 | 0 / -1                             |
| `lseek`               | `(i32 fd, i64 offset, i32 whence) -> i64`         | POSIX lseek                        |
| `malloc`              | `(u64 size) -> ptr<i8>`                           | alias for `__lale_malloc_u64`      |
| `free`                | `(ptr<i8>) -> void`                               | alias for `__lale_free_pointer`    |
| `pow`                 | `(f64 base, f64 exp) -> f64`                      | floating exponentiation            |
| `puts`                | `(ptr<i8>) -> void`                               | C puts                             |
| `strtod`              | `(ptr<i8> nptr, ptr<i8> endptr) -> f64`           | C strtod                           |
| `strtol`              | `(ptr<i8> nptr, ptr<i8> endptr, i32 base) -> i64` | C strtol                           |
| `strtoul`             | `(ptr<i8> nptr, ptr<i8> endptr, i32 base) -> u64` | C strtoul                          |
| `__lale_exit`         | `(i32 code) -> void`                              | program termination                |
| `__lale_malloc_u64`   | `(u64 size) -> ptr<i8>`                           | dynamic allocation                 |
| `__lale_free_pointer` | `(ptr<i8>) -> void`                               | dynamic deallocation               |
| `__lale_read_line`    | `() -> %text`                                     | read one line from stdin as `text` |
| `__lale_timestamp`    | `() -> %text`                                     | runtime UTC timestamp (ISO-8601)   |
| `__lale_log_level`    | `() -> i32`                                       | log-level threshold (0..3)         |

`__lale_error` is **not** an extern. It is the internal runtime-error path
(`lale_error_and_abort`) for bounds violations, division by zero, absent-optional
unwrap, and checked-overflow traps.

**Deliberately excluded:** the buffered C stdio family (`fopen`, `fread`,
`fwrite`, `fclose`, `fseek`) is **not** part of the FFI boundary. Lale stdlib
builds buffered/stream abstractions **in Lale** on top of the single unbuffered
`open`/`read`/`write`/`close` boundary, avoiding a POSIX-and-C-stdio duplication.

---

## 5. Known gaps

- The stdio family (`fopen`/`fread`/`fwrite`/`fclose`/`fseek`) was **removed from
  the boundary** by the single low-level FFI decision (§4). The historical
  `fread` `unimplemented!()` stub is superseded — buffered I/O is now Lale stdlib
  code, so `fread` is no longer an extern at all.
- The interpreter's raw-byte path (`v3_store`/`v3_load`) currently uses
  host-native endianness, not explicit `to_le_bytes`/`from_le_bytes`. This must be
  corrected for the canonical little-endian internal ABI.
- Struct/optional field packing described here is the **target**; the interpreter
  still allocates structs conservatively (64 bytes) and optionals as fixed 16
  bytes. Precise AAPCS64 layout is pending implementation and conformance tests
  (plan 3.2).
