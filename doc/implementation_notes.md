# Implementation notes

**Scope.** Notes from the work on the array-notation branch. The notation now lives in
`doc/dynamic_arrays_and_views.md` (§2.9–§2.13), which is authoritative; `doc/array_type.md` is the
historic rationale, kept as a record. These notes are kept apart from the proposals for two reasons. The first section
records rough edges that the array work **deliberately** left in place. The second records defects
that have **nothing** to do with the array design, so that section can be applied to `main`
unchanged, whether or not the array proposals are signed off.

Line numbers are as of the branch where these notes were written, and can drift; the quoted text is
the reliable anchor. Where a fix is given, it is given as find-and-replace.

## Known rough edges, left as they are

Every item follows from the array-notation design, which is merged but **not implemented**. None of
them should be carried to `main` on its own.

### 1. Only the `var` follow-on error is suppressed

When a type is rejected for its array notation (`array of array of T`, or a dynamic array that is not
implemented), the AST still carries the type's _element_ as the declared type, because
`build_type_name` flattens it. Without care this produced a second, misleading error:
`declared type is 'i32' but initializer has type 'i32[2][2]'`.

The suppression is in `visit_var_def` (`src/semantic_analysis/analyzer.rs`): the initialiser type
check is skipped when `def.type_annotation` has an `array_element`. Other downstream checks — return
types, call arguments — can still add a second message for the same rejected type.

### 2. The valid spellings are gated as "not implemented yet"

`array of T` and `2D array of T` are valid notation but not implemented:
`visit_type_name` reports _"dynamic array types (`array of T`) are not implemented yet"_.

This gate is deliberate. The alternative — letting the flattened element type stand in — would
silently treat `array of i32` as `i32`, which is exactly the failure mode the project forbids.
Removing the gate is the real implementation work: a runtime-count array needs its own representation
in `BaseType`/`TypeName`, in the symbol table, and in the IR.

### 3. `vec2 of array of T` hits the same gate

A vector whose element type is a dynamic array is not covered by the proposal. The AST builder carries
the array marker onto the vector's `TypeName` rather than dropping it, so the type is reported through
the same "not implemented" gate — the array-ness is never silently lost.

### 4. The rank prefix is parsed but not represented in the AST

Lowering keeps the element type (`array_element`) but not the rank (`2D`). Consequences:

- `2D array of i32` and `array of i32` are indistinguishable in the AST.
- The proposal's rank-versus-value check (§6) — for example that
  `var bad as array of i32 = [[1, 2], [3, 4]]` is an error — is not implemented, and neither is the
  rule that the number of extents must equal the rank.

The rejection of the _nested spelling_ does not depend on this: it counts nesting depth, which is what
the rank prefix stands for (§6.1).

### 5. The bracketed forms remain on the branch

`i32[5]` and `[i32]` still parse, because the existing test suite and `examples/complex.lale` use
them. The design removes them as type spellings **with the implementation**, not at sign-off
(§13.15 of `doc/dynamic_arrays_and_views.md`), so both spellings coexist until then.

## Unrelated bugs, which were fixed

Nothing in this section depends on the array proposals; each entry can be applied to `main` on its
own. All five are stale pointers in `doc/ARCHITECTURE.md`, and all five share one cause:
`doc/roadmap.md` was restructured into the §3.1–§3.8 tables, after which every pointer to a section
in the old numbering became stale.

### 1. §1 principle 8: pointer to a section that does not exist

`roadmap.md` has no §4.2. Its sections are 1 Decision Record, 2 Prerequisite gaps (2.1), 3 Versioned
Roadmap (3.1–3.8), 4 Scope, Risk, and Follow-ups, 5 References. Open point 15
("Language-surface clarity (grammar review)") is row 15 of the §3.2 table.

```text
before:  That gap is tracked as open point 15 of the 1.0.0 language-surface clarity review (`doc/roadmap.md` §4.2).
after:   That gap is tracked as open point 15 of the 1.0.0 language-surface clarity review (`doc/roadmap.md` §3.2).
```

### 2. Three function-overloading pointers to the same missing section

Function overloading is §3.2 **row 2** ("Function overloading … See [T-007]"). Three places point at
the non-existent §4.2. Note that the author's "item N" notation was already correct — it means table
row N — so only the section number is wrong.

```text
before:  | Internal (Lale→Lale) | mangled (`name + "_" + types`) | intended — see `roadmap.md` §4.2 | C++ default (Itanium) |
after:   | Internal (Lale→Lale) | mangled (`name + "_" + types`) | intended — see `roadmap.md` §3.2 | C++ default (Itanium) |

before:  unambiguous scheme. See `doc/roadmap.md` §4.2 "Function overloading" for the
after:   unambiguous scheme. See `doc/roadmap.md` §3.2 "Function overloading" for the

before:  `roadmap.md` §4.2 item 2.
after:   `roadmap.md` §3.2 item 2.
```

### 3. §4.6: the `hub` deferral pointer

The deferral is stated in §3.2 **row 13** ("The `hub` origin and `version` are explicitly deferred to
2.0.0"). Row 9 of §3.3 is the 2.0.0 deliverable itself.

```text
before:  (roadmap §4.3). A path must end in a file: a bare directory is rejected with a dedicated
after:   (roadmap §3.2 item 13). A path must end in a file: a bare directory is rejected with a dedicated
```

### 4. §5.14d: the read-only-reference pointer

Read-only references are §3.3 **row 8** ("Read-only reference (compiler-inferred)"), so the author's
"item 8" was right and only the section number was wrong.

```text
before:  `doc/roadmap.md` §4.3 item 8 for the open questions.
after:   `doc/roadmap.md` §3.3 item 8 for the open questions.
```

### 5. §4.6 open questions: pointer to a non-existent §11

`roadmap.md` has no §11. The three module questions (whether `import fn` is unified with the origin
syntax, how `std.<path>` maps on disk, and `use all` re-export semantics) are `[T-031]` items 5–7 in
`doc/TODO.md`.

```text
before:  These are tracked in `roadmap.md` §11.
after:   These are tracked as `[T-031]` items 5–7 in `doc/TODO.md`.
```

### Not a bug: two nearby `§4.3` mentions are correct

Lines 178 and 221 refer to `ARCHITECTURE.md`'s **own** §4.3 (AST Structure and Source Fidelity), not
to the roadmap. Do not change them.

**Discovered, not fixed**

Recorded so they are not lost with the branch. None of them is part of the array design.

### A. The bracketed prefix array form is silently lowered as a scalar

`[i32]` is the prefix spelling of an array type, and the frozen semantics treat it as one. But
`build_type_name` (`src/ast/expr_parser.rs`) has no arm for the bracket wrapper: the match sees only
the inner type rule, so `[i32]` sets `base_type = I32` and leaves `array_dimensions` empty. The
brackets are simply dropped and the type becomes a plain `i32`. Where a value then forces a check the
result is a misleading message (`declared type is 'i32'` for an array initialiser); where none does —
a parameter, a field, an `unsafe decl` — nothing reports it at all. Compare `i32[5]`, which does
populate `array_dimensions`.

Not fixed here because it is an array-representation question, and the array proposal removes the
bracket spellings anyway. If the proposal is not signed off, it stands on `main` as a silent
mis-lowering: either represent the prefix form as an array type or reject it explicitly.

### B. Three-way drift in the element-count vocabulary

Verified against the compiler, not inferred:

- the language operator is `count of`;
- the runtime bounds message says **length** —
  `array index out of bounds (index=5, length=3)`, rendered by
  `src/ir/builder.rs` (`RenderPart::Text(", length=".to_string())`) and asserted in
  `tests/runtime_error_format_tests.rs`;
- the compile-time message says **size** —
  `Array index out of bounds: dimension 1 has size 3 but got literal 5. Valid range is 1..3.`,
  in `src/semantic_analysis/analyzer.rs` (two sites, `visit_array_index`).

One word (`count`) would cover all three. The design now settles this: §2.13 of
`doc/dynamic_arrays_and_views.md` standardises on `count`, and §13.14 is the work item. So the
compiler wording is a **scheduled change**, not an open mismatch.

### C. The count operator is spelled `#count of` in the compiler

Verified against the compiler, not inferred:

- `write #count of xs` prints the element count (`3` for a three-element array);
- `count of xs` — without the `#` — is a _grammar error_: the parser reads `count` as an ordinary
  identifier and then rejects the `of`;
- the grammar defines both intrinsics in the `unary` rule as `count_op = { "#count of" ~ ms_ln }`
  and `size_op = { "#size of" ~ ms_ln }`, and `doc/ARCHITECTURE.md` calls `#size of` / `#count of`
  introspection operators.

Against that, `doc/array_type.md` §8.1 and the frozen `doc/dynamic_arrays_and_views.md` §2.3 name
the operator `count of` — the frozen document even states there is "no compile-time `#count of`" —
and the frozen document's examples write `count of xs` throughout, which does not compile.
`doc/lale.md` documents neither spelling, so the user guide is silent on the operator.

Either the wording should be `#count of`, or the plan to drop the `#` (and the compile-time
`#size of`) should be stated where it belongs.

**Now stated, and decided.** §2.13 of `doc/dynamic_arrays_and_views.md` settles the wording on
`count` and drops the `#` from `count of` and `size of`, keeping it on `#type of` and `#unit of`;
§13.14 is the compiler work item. So the `#count of` / `#size of` spellings in the grammar and in
`ARCHITECTURE.md` are **corrected with the array implementation** — no longer an open mismatch, and
not fixed here, because the implementation is outside the notation work.

### D. `write` on a vector value is an internal compiler error

```lale
var v as vec2 of f64 = vec2(1.0, 2.0)
write v
```

```text
internal compiler error: generate_output: unsupported value type for output
  (expected text, numeric, bool, char, or byte)
  (Rust: src/ir_gen.rs:4186)
```

An internal error is not an acceptable diagnostic for a user-level statement: either `write` of a
vector should be supported, or it should be rejected with a clear message. Not caused by the
array-notation work — nothing in it touches output formatting or vector handling.

### E. Grammar and testing hazards worth knowing (not bugs)

Two facts about the codebase that cost time to rediscover, recorded for the next person who edits the
grammar or writes parse tests.

- **`LaleParser::parse` accepts a rule that matches only a _prefix_ of its input.** Parsing
  `"array of array of i32"` as `Rule::type_name` returns `Ok`, having matched just the identifier
  `array` (5 of 21 characters). Any test shaped like `assert!(parse(...).is_err())` can therefore pass
  or fail for the wrong reason; a parse test should compare the matched span with the input.
- **The grammar manages whitespace explicitly** — there is no implicit `WHITESPACE` rule, and `os` /
  `ms` / `os_ln` / `ms_ln` are written out at each site. A new rule that needs a space between two
  tokens must carry it (`"2D" ~ ms_ln ~ "array"`), or the parse fails on input that reads correctly.
