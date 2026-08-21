# Lale IR Format Specification

**Status:** Authoritative reference for the line-oriented IR serialization format
**IR format version:** 1.0
**Snapshot:** Lale v0.9.1

This document is the single source of truth for the Lale Intermediate
Representation (IR) text format. It is produced by `src/ir/printer.rs` and
consumed by `src/ir/parser.rs`. The interpreter and future AOT backends consume
this format.

The IR itself is a **block-based control-flow graph (CFG)** in SSA form. It is
not a structured tree and contains no `phi` or `Select` nodes.

---

## 1. Overview

A serialized module is a plain-text, line-oriented file. Each construct occupies
one line, except function bodies, which span multiple lines delimited by braces.

```text
; ir-version: 1.0
; Module: example

%str = type { ptr: ptr, len: u64 }

declare @write(i32, ptr<i8>, i64) -> i64

@answer: i64 = 42

export func @main(%0 n: i64) -> i64 {
entry:
    %1 = alloca i64
    store %0, %1, i64
    %2 = load i64 %1
    %3 = const i64 0
    %4 = gt %2, %3
    condbr %4, %then, %else
then:
    %5 = const i64 1
    br %merge
else:
    %6 = const i64 2
    br %merge
merge:
    %7 = add %5, %6
    ret %7
}
```

---

## 2. Versioning

Every serialized module begins with a version line:

```text
; ir-version: <major>.<minor>
```

- **major** — increments on breaking changes. A parser MUST reject an unsupported
  major version.
- **minor** — increments on additive, backward-compatible changes. A parser may
  accept any minor version within a supported major version.

The current major/minor are defined in `src/ir/mod.rs` as `IR_FORMAT_MAJOR` and
`IR_FORMAT_MINOR`.

---

## 3. Lexical grammar

- The file is a sequence of lines separated by `\n` (a trailing `\r` is ignored).
- A line comment begins with `;` and runs to the end of the line.
- Whitespace separates tokens; indentation is not significant.
- Blank lines are ignored.

### Tokens

- **ident** — a run of non-whitespace characters other than the punctuation
  below, `;`, and `"`.
- **string** — `"..."` with Rust `escape_default` escapes: `\n`, `\t`, `\r`,
  `\\`, `\"`, and `\u{...}`.
- **punct** — one of `= : , ( ) { } [ ] < >`.
- **arrow** — `->` is lexed as a single token.

SSA values are written `%N` (an `ident`). Block references are written
`%block-name`. Function/global/extern names are written `@name`. Struct names are
written `%name`.

---

## 4. Module grammar

```text
module         := version_line module_header struct_section? extern_section?
                  global_section? function*
version_line   := "; ir-version: " major "." minor
module_header  := "; Module: " name

struct_section := ("; Struct definitions" line)? struct_def*
struct_def     := "%" name " = type { " field (", " field)* " }"
field          := name ": " type

extern_section := ("; External functions" line)? extern_def*
extern_def     := "declare @" name "(" params ")" "->" type
params         := (type (", " type)* (", ...")?)?

global_section := ("; Global variables" line)? global_def*
global_def     := linkage? "@" name ":" type (" = " constant)?
                 (" ; unit: <" unit ">")?

function       := linkage? "func @" name "(" params_defs ")" "->" type unit_opt?
                 "{" block* "}"
params_defs    := (param (", " param)*)?
param          := value_id name ":" type unit_opt?
unit_opt       := "<" unit ">"

block          := name ":" instruction*
```

`linkage` is one of `export`, `import`, or absent (internal).

---

## 5. Types

`type` matches the `IrType` `Display` form:

| Form                          | Meaning                      |
| ----------------------------- | ---------------------------- |
| `void`                        | `Void`                       |
| `bool`                        | `Bool`                       |
| `i8` `i16` `i32` `i64`        | signed integers              |
| `u8` `u16` `u32` `u64`        | unsigned integers            |
| `f16` `f32` `f64`             | IEEE-754 floats              |
| `char`                        | Unicode scalar               |
| `ptr`                         | opaque pointer (`Ptr(Void)`) |
| `ptr<T>`                      | typed pointer                |
| `[N x T]`                     | array                        |
| `%Name`                       | struct reference             |
| `vec2<T>` `vec3<T>` `vec4<T>` | vectors                      |
| `T?`                          | optional                     |

Nested pointer and vector types use nested angle brackets, e.g.
`ptr<ptr<i32>>`, `vec3<f64>`.

---

## 6. Constants

`constant` (used only in global initializers):

| Form               | Meaning                             |
| ------------------ | ----------------------------------- |
| `N`                | signed integer (`Constant::Int`)    |
| `Nu`               | unsigned integer (`Constant::Uint`) |
| `N.N` / `Ne±N`     | float (`Constant::Float`)           |
| `true` / `false`   | boolean (`Constant::Bool`)          |
| `"..."`            | string (`Constant::String`)         |
| `null`             | null pointer (`Constant::Null`)     |
| `zeroinit`         | zero-initialized (`Constant::Zero`) |
| `[ c, c, ... ]`    | array (`Constant::Array`)           |
| `{ name: c, ... }` | struct literal (`Constant::Struct`) |

---

## 7. Instructions

Every value-producing instruction has the form `%dst = op operands`. Effect
instructions have no destination. Terminators end a block.

### Arithmetic, vectors, bitwise

| Opcode                                    | Form                                       |
| ----------------------------------------- | ------------------------------------------ |
| `add` `sub` `mul` `div` `rem`             | `%d = op %l, %r`                           |
| `neg`                                     | `%d = neg %s`                              |
| `ptrdiff`                                 | `%d = ptrdiff %l, %r`                      |
| `pow`                                     | `%d = pow %base, %exp`                     |
| `cross` `dot`                             | `%d = op %l, %r`                           |
| `checked_add` `checked_sub` `checked_mul` | `%d = op T %l, %r at "file" line col`      |
| `checked_neg`                             | `%d = checked_neg T %s at "file" line col` |
| `bitand` `bitor` `bitxor`                 | `%d = op %l, %r`                           |
| `bitnot`                                  | `%d = bitnot %s`                           |
| `shl` `shr` `ushr`                        | `%d = op %l, %r`                           |
| `buildvec2`                               | `%d = buildvec2 %x, %y`                    |
| `buildvec3`                               | `%d = buildvec3 %x, %y, %z`                |
| `buildvec4`                               | `%d = buildvec4 %x, %y, %z, %w`            |
| `extractvecelem`                          | `%d = extractvecelem %s, N, T`             |

### Comparison & logical

| Opcode                        | Form             |
| ----------------------------- | ---------------- |
| `eq` `ne` `lt` `le` `gt` `ge` | `%d = op %l, %r` |
| `and` `or` `xor`              | `%d = op %l, %r` |
| `not`                         | `%d = not %s`    |

### Memory

| Opcode        | Form                                               |
| ------------- | -------------------------------------------------- |
| `alloca`      | `%d = alloca T`                                    |
| `load`        | `%d = load T %ptr`                                 |
| `store`       | `store %val, %ptr, T`                              |
| `gep`         | `%d = gep %base, %i1, ...`                         |
| `getfieldptr` | `%d = getfieldptr %Struct %base, N`                |
| `fieldptr`    | `%d = fieldptr %Struct %base, field`               |
| `ptrtoint`    | `%d = ptrtoint %s`                                 |
| `inttoptr`    | `%d = inttoptr %s`                                 |
| `boundscheck` | `boundscheck %idx, %len, "msg", "file", line, col` |
| `zerocheck`   | `zerocheck %v, "msg", "file", line, col`           |

### Constants

| Opcode             | Form                         |
| ------------------ | ---------------------------- |
| `const` (int)      | `%d = const T N`             |
| `const` (uint)     | `%d = const T Nu`            |
| `const` (float)    | `%d = const T N.N`           |
| `const` (bool)     | `%d = const bool true/false` |
| `const` (string)   | `%d = const str "..."`       |
| `const` (null)     | `%d = const ptr null`        |
| `const` (null int) | `%d = const ptr 0`           |

### Control flow

| Opcode       | Form                         |
| ------------ | ---------------------------- |
| `br`         | `br %block`                  |
| `condbr`     | `condbr %cond, %then, %else` |
| `ret`        | `ret %val`                   |
| `ret` (void) | `ret void`                   |

### Calls

| Opcode        | Form                                            |
| ------------- | ----------------------------------------------- |
| `call`        | `%d = call @func(%a, ...) [at "file" line col]` |
| `call` (void) | `call @func(%a, ...) [at "file" line col]`      |

`@func` resolves to an internal function if the module defines it, otherwise to
an external function.

### Type conversions

| Opcode                                                          | Form              |
| --------------------------------------------------------------- | ----------------- |
| `trunc` `sext` `zext`                                           | `%d = op %s to T` |
| `fptosi` `fptoui` `sitofp` `uitofp` `fptrunc` `fpext` `bitcast` | `%d = op %s to T` |

### Structs, optionals, strings, error stack, tests

| Opcode           | Form                                                       |
| ---------------- | ---------------------------------------------------------- |
| `buildstruct`    | `%d = buildstruct %Struct { %f1, ... }`                    |
| `extractfield`   | `%d = extractfield %Struct %s, N`                          |
| `insertfield`    | `%d = insertfield %Struct %s, N, %v`                       |
| `concat`         | `%d = concat %l, %r`                                       |
| `strcopy`        | `%d = strcopy %s`                                          |
| `some`           | `%d = some %v`                                             |
| `none`           | `%d = none`                                                |
| `unwrapoptional` | `%d = unwrapoptional %Struct %s, "msg" at "file" line col` |
| `pusherror`      | `pusherror %v`                                             |
| `poperror`       | `%d = poperror`                                            |
| `errorcount`     | `%d = errorcount`                                          |
| `drainerrors`    | `drainerrors stdout\|stderr [prefix "p"]`                  |
| `assertunit`     | `assertunit %v, <unit>`                                    |
| `testbegin`      | `testbegin "suite" "case"`                                 |
| `testfail`       | `testfail "file" line col`                                 |
| `testend`        | `testend`                                                  |
| `globaladdr`     | `%d = globaladdr @name`                                    |

---

## 8. Semantics and determinism

- **SSA** — each `%N` is defined exactly once within its function.
- **Block CFG** — every block ends in exactly one terminator (`br`, `condbr`,
  `ret`, or `ret void`). There are no `phi` or `Select` nodes; values that must
  live across a merge are written to and loaded from allocas.
- **No undefined behaviour** — every value is a concrete result. Integer overflow
  traps by default via `checked_*` instructions; `--unchecked-overflow` emits the
  wrapping `add`/`sub`/`mul`/`neg` forms. Division/remainder by zero trap via
  `zerocheck`. Floating-point follows IEEE-754 (Rust `f32`/`f64` semantics).
- **Backend independence** — the interpreter is the golden reference. A backend is
  correct iff its observable output `(stdout, stderr, exit_code)` matches the
  interpreter for every valid module.

---

## 9. Known limitations

- Global physical units are currently emitted as a trailing `; unit: <...>`
  comment and are not reconstructed by the parser. Parameter and return units are
  machine-readable inline tokens.
- A binary serialization counterpart may be added later for performance; it is
  not part of this version.
