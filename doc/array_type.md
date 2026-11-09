# Array type notation

**Status:** historic record — every statement in this document has been merged into
`doc/dynamic_arrays_and_views.md`, which is authoritative. The body below is kept as the rationale
for the spelling choices; it is not a proposal to be approved on its own.

**Scope.** This document changed only how the array _type_ is **spelled**. The array's
_semantics_ — value semantics, ownership, references, and the C boundary — are specified in
`doc/dynamic_arrays_and_views.md`, which now also carries this notation (§2.9–§2.13) and is
authoritative. Both were reviewed together and merged; this file is kept as the rationale. Section 8
lists the work the decision scheduled.

**Reading note.** A _parsing expression grammar (PEG)_ is a grammar whose rules are tried in a
fixed order, each consuming input greedily.

## 1. Why name the array type

The array type is currently spelled with brackets — `i32[10]`, `Sample[41]`. The discussion began
with a readability question: does `[]` communicate "array" to a newcomer? It grew into four
questions, each settled in a section below:

1. Vocabulary — is "array" the right word, or should it be "list"? (§2)
2. Spelling — should the _type_ be named with a word (`array of T`) instead of brackets? (§3)
3. The count — should the element count live in the type at all? (§4)
4. Multi-dimensional arrays — how should they be spelled compactly? (§6)

## 2. Vocabulary: "array", not "list"

Replacing "array" with "list" throughout the documentation (and possibly the grammar) was
considered and rejected. The reasons:

- Python's `list` is **not** the same structure. It is **heterogeneous** (any objects) and stores
  **pointers to boxed objects**, not contiguous raw values. Lale's array type is **homogeneous**
  (one element type) and holds **contiguous raw values**.
- The closest analogue to Lale's type is Julia's **`Array{T,1}`** — 1-based, homogeneous, and, for
  bitstype elements, stored as **contiguous, unboxed raw values**; it is **growable** (`push!`,
  `resize!`). Julia also spells the 1-D case **`Vector{T}`**, but that is an **alias**
  (`const Vector{T} = Array{T,1}`), not a separate type: the **family** name is **`Array`**.
  MATLAB and NumPy likewise say **array**.
- **Why "array" and not "vector".** Julia itself uses both words for the same type, and "vector"
  is arguably the more precise word for a growable 1-D sequence. But Lale already uses `vec2` /
  `vec3` / `vec4` for linear-algebra vectors, so "vector" is taken. **Array** — Julia's family
  name — is the choice.
- `doc/lale.md` justifies 1-based indexing by citing **Julia, MATLAB, and Fortran** — all of which
  say "array". Renaming to "list" would move Lale away from its declared lineage.
- `inline array of i32 of 3 elements` is specified as **byte-for-byte a C array member**; the word
  "array" has a permanent home in the C-interop story.
- **NumPy is not evidence for "array implies growable".** NumPy arrays are **fixed-shape** after
  creation — `np.append` returns a _new_ array. The clean evidence for a growable array type is
  Julia.

**The word is "array".** No documentation-wide rename is made.

## 3. Spelling: `array of T`

**The array type is spelled `array of T`**, replacing the two bracketed forms — the suffix `T[n]`
and the prefix `[T]` (as in `i32[5]` and `[i32]`) — as _type_ spellings:

```lale
var numbers as array of i32 = [1, 2, 3, 4, 5, 6, 7, 8, 9, 10]
```

**Rationale.**

- **Completes the `X of T` family.** Lale already has `vec2 of T`, and the array design adds
  `ref of T` and `view of T`. Arrays are the only type still spelled by punctuation; `array of T`
  removes that exception.
- **Reuses a documented rationale.** `lale.md` (§ Vectors) says the `of` keyword exists to
  **avoid bracket ambiguity with the unit system** ("`of` separates storage composition from
  physical dimensions (`in <unit>`)"). Extending `of` to arrays finishes a pattern Lale already
  chose and justified.
- **Teachable.** `array of i32` reads as English, and a count reads as `N elements`
  (`[fill 10 elements with 0]`); `[]` in a type position is jargon to a newcomer.
- **Un-overloads `[]`.** After the change, brackets in a _type_ position are gone; `[...]` remains
  only as a value literal and `xs[i]` as indexing.
- **Precedent.** Ada uses `array (1 .. 10) of Integer`; Pascal uses `array[1..10] of Integer`.

**`of` joins the type into a two-word constructor.** The array design spells references
source-typed (`ref of array of i32`, `view of array of f64`) — a chained `of`. The chained form is
accepted: explicitness is a Lale principle. It needs no associativity rule, because `of` is part of
the **constructor keyword**, not a free operator — `array of`, `ref of`, `view of`, and `vec2 of`
are two-word constructors (exactly like `pointer to`), each taking a **type** as its argument. The
chaining then nests right _by construction_ (`ref of array of i32` = `ref of (array of i32)`).

**Brackets remain for indexing and for values.** `i32[5]` and the prefix `[i32]` are removed as
type spellings; `array of i32` is the only one. Brackets stay in their two value roles — **indexing**
(`xs[i]`) and **value literals** (`[1, 2, 3]`) — so a _type_ contains no brackets at all, while a
_value_ still does.

## 4. The count is a value, not part of the type

**The element count belongs to the value, not to the type.** §2.1 of the array design already says
so: there is one array type, whatever the count, and the old `i32[5]` / `i32[]` split was only
notation. Putting a count in the type spelling was therefore always a small inaccuracy.

**The count is never written for a value-initialised array.** It comes from the initialiser:

```lale
var numbers as array of i32 = [1, 2, 3, 4, 5, 6, 7, 8, 9, 10]   // count 10, from the value
var a       as array of i32 = [1, 2, 3]                          // count 3, from the value
var b       as array of i32 = [1, 2]                             // count 2, from the value
```

**Wins.**

- **Dissolves the ordering dilemma.** `array of 10 i32` vs. `array 10 of i32` — both disappear,
  because the count is not in the type at all.
- **Makes §2.1 literally true:** one spelling, `array of i32`, and the count lives only in the
  value.
- **One source of truth.** The value states the count once, so there is no second copy to keep in
  sync and nothing that can disagree with it — a declared count and a literal count can no longer
  contradict each other.
- **Type and value agree by construction.** `var x as array of i32 = <array value>` is an ordinary
  assignment; the value already carries its count.

A count _is_ still written where no value can supply one — `fill`, `unsafe decl`, and `inline`
(§5).

**The empty literal is the same rule at zero.** `var e as array of i32 = []` has count `0`, taken
from the value like any other; no count clause is written or needed.

**Compatible with "no implicit conversions".** Taking the count from the value is consistent with
Lale's no-implicit-conversions rule: a _count_ is a value, not a type, so nothing is being
coerced. The reasoning is recorded here because the rule is worth stating.

**For multi-dimensional arrays.** The same reasoning gives the extents of a multi-dimensional
array from its value; where no value exists, they come from the extent clause of §5.2 / §6.

## 5. The count where no value can supply it

Three constructs have **no value** to read a count from, so each must obtain it some other way:

| Construct                     | Has a value?  | Needs the count because…                      |
| ----------------------------- | ------------- | --------------------------------------------- |
| `[fill 10 elements with 0]`   | yes (uniform) | a uniform value says nothing about _how many_ |
| `unsafe decl b as array of T` | **no**        | it must allocate; there is no initialiser     |
| `inline array of T`           | **no**        | the count _is_ the C array dimension          |

**The count goes wherever the value can carry it:** into the value for `fill` (§5.1); into the
declaration, after the type (`of N elements`), only for `unsafe decl` and `inline`, which have no
value at all (§5.2).

### 5.1 `fill` carries its count in the value

The count is stated in the value:

```lale
var zeros   as array of i32    = [fill 10 elements with 0]
var samples as array of Sample = [fill 3 elements with Sample(0, 0, 0)]
var grid    as 2D array of i32 = [fill 10 elements with [0, 0]]   // ten 2-element rows
```

A count-less fill is **not** kept: `[fill with 0]` would have to take its count from the
declaration — a second source of truth (§4).

`10 elements` is a **noun phrase**, so it is unambiguously a count, and it handles nested values
cleanly (`fill 10 elements with [0, 0]` reads "ten elements, each `[0, 0]`").

Considered and rejected:

- `[fill with 10 times 0]` — reads as "fill with (10 × 0)", i.e. **no count** (multiplication).
- `[fill 10 times with 0]` — reads as "**fill 10 times** with 0", i.e. _repeat the fill_, not ten
  elements (repetition). `times` is the English multiplier/occurrence word; it resists meaning
  "element count".

### 5.2 `unsafe decl` and `inline` carry the count in the declaration

These have no value, so they need a declaration-level count — written after the type:

```lale
unsafe decl buffer as array of i32 of 5 elements
inline      array of i32 of 5 elements
```

**Rejected: `count 5`** (`array of i32 count 5`) — it reads unnaturally. The natural form is
`of 5 elements`, matching the `10 elements` of a `fill` value (§5.1), so "**N elements**" becomes
the one way to state an element count anywhere in the language.

**`of 5 elements` does not collide with the type-`of` chain.** Two `of`s appear in one type here
(`array of i32 of 5 elements`), which invited the worry that the second could be read as
composition. But the grammar (`src/grammar/lale.pest:209`) shows that, in a _type_, `of` is
**always** preceded by a **kind keyword** (`vec2` / `vec3` / `vec4`, and prospectively `ref` /
`view` / `array`) — never by a plain type name. So after `array of i32`, a following `of` can only
be the extent clause: `of 5 elements` and `of n elements` (a variable extent) are both
unambiguous.

The one place it _does_ mis-bind is the **nested** spelling `array of array of i32 of 5 by 3
elements`, where the clause sits inside the recursive rule and the **innermost** `array of` takes
it — rejection reason (c) in §6. The nested spelling never reaches code generation: the grammar
accepts it and semantic analysis rejects it (§6).

## 6. Multi-dimensional arrays: a rank prefix

Multi-dimensional arrays are spelled with a **rank prefix** on the array type: `2D array of T`,
`3D array of T`, ….

**The nested spelling `array of array of …` is rejected — forbidden, not merely discouraged.** An
`array of`'s element type is **never itself an array type**; every multi-dimensional array is
written `ND array of T`. The nested form is the "true" representation, but it is rejected because
it is (a) unwieldy to read and write, (b) **redundant** with the rank prefix — an array of arrays
_is_ a 2-D array, since the semantics are already nested (§6.1) — and (c) the one form where the
extent clause mis-binds.

Reason (c) in detail: if the extent clause sat inside a recursive `array of` rule,
`array of array of i32 of 5 by 3 elements` would parse as
`array of (array of i32 of 5 by 3 elements)` — the **inner** array takes the extents. Because
every array type is flat, that ambiguity never arises (the semantics stay nested and jagged, §6.1).

**How it is enforced: the grammar accepts the nested form, and semantic analysis rejects it.** The
check is deliberately _not_ made in the grammar, so that the diagnostic can name the fix:
`array of array of i32` parses, and semantic analysis then reports that an array's element type may
not itself be an array, pointing at the spelling to use instead:

```text
ERROR: an array's element type may not itself be an array (`array of array of i32`):
       write `2D array of i32` for a 2-dimensional array
```

A grammar-level rejection cannot say that; it can only report an unexpected token at the point of
failure, which leaves the writer without the reason.

```lale
2D array of Sample
2D array of vec3 of f64
```

**Why a prefix, and why it cannot be dropped.** The rank must be expressible **without extents** —
in a parameter, a return type, a `ref of …`, or a non-`inline` struct field — and there the extents
are not available. Nesting would carry the rank; since nesting is rejected, the rank needs its own marker.
So dropping `2D` entirely cannot work: `array of Sample` alone is rank-1.

**Rank and extents together.** The extents are the extent clause of §5.2 (an `of … elements`
clause). That clause exists **only** where no value can supply them — `unsafe decl` and `inline`
(§5) — so a value-initialised array states no extents at all (§4):

```lale
var m   as 2D array of i32 = [[1, 2], [3, 4]]            // extents from the value
unsafe decl buffer as 2D array of i32 of 5 by 3 elements // extents from the clause
inline 2D array of i32 of 5 by 3 elements                // a C `int[5][3]` member
```

(For a rank-1 array the one extent _is_ the element count; for higher ranks there is one extent per
dimension.)

**The rank prefix is always written.** `2D` / `3D` is part of the type; rank 1 is the default and
has no prefix. The rank is **checked against the value** wherever one is present — the value's
nesting must match the declared rank:

```lale
var ok  as 2D array of i32 = [[1, 2], [3, 4]]   // ✅ rank 2, a 2-D value
var bad as array of i32    = [[1, 2], [3, 4]]   // ❌ rank 1, a 2-D value
```

and, wherever the declaration also states extents, the number of extents must equal the rank:

```lale
array of i32 of 5 elements         // ✅ rank 1, one extent
2D array of i32 of 5 by 3 elements // ✅ rank 2, two extents
2D array of i32 of 5 elements      // ❌ rank 2, one extent
array of i32 of 5 by 3 elements    // ❌ rank 1, two extents
```

This costs a mild redundancy — `2D` and the extent count say the same thing — which is accepted
for teachability: the writer never has to decide _when_ to write the prefix, and a mismatch is a
compile error.

**Rejected: the glued form** `2Di32`, `3Df64`. It only works when the element type is a single
short name — nothing glues to `2D vec3 of f64` — it abandons `of`, and it over-promises a
rectangular shape.

**The extent separator is `by`.** `of 5 by 3 elements` reads like "a 5 by 3 grid". `5x3` cannot be
used — ASCII `x` is not an operator (and `x` begins an identifier) and `×` (U+00D7) is excluded
from the grammar (`ARCHITECTURE.md §5.21`); `,` is quieter and `*` reads as multiplication. `by` is
also the word in `grow xs by 3`, which is fine: Lale has no reserved words.

### 6.1 Rectangular vs. jagged — what the words mean

A **rectangular** (contiguous) array is one block with equal-length rows — the shape belongs to
the whole, so a single row cannot be grown without rebuilding the block:

```text
[1, 2, 3]
[4, 5, 6]
[7, 8, 9]     // every row has length 3
```

A **jagged** (ragged) array is an _array of arrays_: each row is an independent array with its own
length, so rows may differ and may be grown one at a time:

```text
[1, 2, 3]
[4, 5]
[6, 7, 8, 9]  // row lengths 3, 2, 4
```

**Why this matters for Lale.** The array design (§2.7) is **jagged by construction**: a 2-D
array is a dynamic array _of_ dynamic arrays, so the outer buffer holds one **header** per row,
each header pointing at that row's own separately allocated buffer. That is why rows can differ,
why copying and freeing recurse, and why `grow m[1] by 2` touches only row 1. The design adds
that "a contiguous, fixed-size matrix is a **separate concern** and is not part of this design".

**Lale's arrays are jagged.** Lale already has **vectors** (`vec2` / `vec3` / `vec4`), and
**matrices** are planned as a separate **rectangular** type. If arrays were rectangular too, the
language would have **no jagged type at all** — an unacceptable restriction. Arrays therefore stay
jagged, which is also the design model (§2.7).

The one rectangular array in Lale is the **`inline` field**: `inline 2D array of i32 of 5 by 3
elements` is a fixed C `int[5][3]` member, so its rectangle _is_ an invariant and there is nothing
to grow (§4.1 of the array design). Jaggedness is a property of the dynamic array type.

Consequences:

- The flat spelling `2D array of T` names the **leaf** element type; the array itself is the
  nested `array of array of T`. The _outer_ element type is a **row** (`array of i32`); `i32` is
  the leaf.
- `of 5 by 3 elements` sets only the **initial** extents; later growth or ragged data may make the
  rows differ. The rectangle is not an invariant.
- A nested literal is the natural initialiser: `2D array of i32 = [[1, 2], [3, 4]]` — its element
  type is `array of i32`, matching the jagged type.
- `grow` / `shrink` act at the **outermost** level (`grow m by 2 = [row1, row2]` appends rows); a
  single row is grown through indexing (`grow m[1] by 2`), which leaves the outer array untouched
  (§2.8 of the array design; that document still lists the general rule for an indexed mutation
  target as open, §11).

## 7. Language facts the notation relies on

- **No reserved keywords.** `ARCHITECTURE.md` §1, principle 8: statement and expression keywords
  are **valid identifiers**; disambiguation is by PEG ordered choice. **So `elements`, `of`, etc.
  reserve nothing.**
  - **Known gap in the same principle.** The principle records it directly: bare alphabetic _infix_
    operators (`or`, `and`, `xor`, `dot`, `cross`) are matched without a word boundary, so
    `a oranges` parses as `a or anges` (open point 15 of the 1.0.0 roadmap, `doc/roadmap.md` §3.2).
    A new suffix word such as `elements` therefore needs a boundary guard, exactly as `write inline`
    already uses one (`write_inline = { "write inline" ~ !identifier_continue }`, `lale.pest:575`) —
    so `"elements" ~ !identifier_continue` avoids a partial match on `elementsX`.
- **`×` (U+00D7) is excluded from the grammar** (`ARCHITECTURE.md §5.21`). Multiplication uses
  `⨯` (U+2A2F), `*`, `⋅` / `dot`, and `cross` (`multiplication`, `lale.pest:267`). This constrains
  any multi-dimensional separator.
- **Grounds for a two-word constructor (`of`), from the existing grammar.** The two-word shape
  already exists: `ptr_op = { "pointer to" ~ ms_ln }` (`lale.pest:243`), with a guard on the bare
  `pointer` (`pointer = { "pointer" ~ !(ms_ln ~ "to") }`, `lale.pest:201`). Constructor `of` is
  already delimited, because `ms_ln` is _mandatory_ whitespace (`lale.pest:19`). And the `of` form
  nests right _by construction_: `vector_type = { vector_kind ~ ms_ln ~ "of" ~ ms_ln ~ type_name }`
  (`lale.pest:209`), where `type_name` tries `vector_type` first (`lale.pest:210-211`) — so
  `vec2 of vec3 of i32` already parses as `vec2 of (vec3 of i32)`.

## 8. Consequences, and the work it scheduled

- **The `X of T` family grows, and gains a rank modifier.** `array of T` joins `vec2 of T`,
  `ref of T`, and `view of T`; and the rank prefix (`2D`, `3D`, …) is a new kind of type modifier.
- **An array of vectors becomes expressible.** The bracket form admits only builtin scalar types
  and one identifier — the compiler rejects `[vec2 of f64]` with a grammar error listing the
  allowed atoms — so no array of vectors can be written today. `array of vec2 of f64` composes by
  construction, as §6's `2D array of vec3 of f64` shows. This is an added capability, not only a
  respelling.
- **Count-from-value is a mild inference.** It is consistent with "no implicit conversions" (a
  count is a value), but that reasoning should be written down.
- **`[]` becomes indexing-only in types** (plus value literals) — a real, if partial, notational
  simplification.
- **A count idiom** ("N elements") now works in both positions: the `fill` value (§5.1) and the
  extent clause (§5.2).

### 8.1 Editorial cleanup: one word for "number of elements"

"Number of elements" has three spellings in the compiler: `count of` (the operator), `size` (the
compile-time analyzer), and `length` (the runtime diagnostic — §2.4 of the array design, recorded
as a follow-up in its §11). **Standardize on `count`** in the grammar, the diagnostics, and the
docs.

This began as an **editorial cleanup** outside the notation work. It is now **part of the merged
design** (`doc/dynamic_arrays_and_views.md` §2.13), because the same decision drops the `#` from the
runtime operators `count of` and `size of`. It is not implemented yet; the work item is that
document's §13.14.

```lale
write count of xs   // the operator already says "count"
```

```text
ERROR: array index out of bounds (index=5, length=3)   // the runtime says "length" → to become "count"
```

### 8.2 Migration — performed with the implementation

The broad edit is **not** done at sign-off. It lands **with the array implementation**, in the same
change that introduces the new spelling (`doc/dynamic_arrays_and_views.md` §13.15 states the
sequencing). The one part already done is the merge of the notation into the design document
(§2.9–§2.13 there). The full scope:

- **Grammar** (`src/grammar/lale.pest`): the two array spellings `[i32]` and `i32[5]`; a new
  digit-led **rank token** (`2D`), which must take precedence over an integer literal (`2` already
  starts a number); the `of … elements` clause; `elements` with a boundary guard. The grammar
  **accepts** an array whose element type is itself an array — the nested spelling is rejected in
  semantic analysis instead, so that the diagnostic can name the fix (§6).
- **Semantic analysis** (`src/semantic_analysis/`): reject an array whose element type is itself an
  array, with the message shown in §6 (`array of array of i32` → "write `2D array of i32`").
- **All docs**, `examples/complex.lale`, and the test suite.
- **ABI / IR** type spelling: the array layout `T[n]` in `ABI_SPECIFICATION.md` §2, plus the
  dynamic-array `ir_type` form that the array design's §13.6 adds to `ARCHITECTURE.md §4.7`
  (whose exact spelling is unsettled there — `i32[]`, `dyn<i32>`, or a named struct).
- **The array design document** (`doc/dynamic_arrays_and_views.md`): every bracketed array type
  spelling became `array of T` across declarations, parameters, fields and return types, plus the
  reference and view spellings (`ref of i32[]` → `ref of array of i32`, `view of f64[]` →
  `view of array of f64`). **Done** — that merge is complete, and the document is authoritative.

Example of an affected program:

```lale
// before
var samples as Sample[41] = [fill with Sample(0, 0, 0)]

// after
var samples as array of Sample = [fill 41 elements with Sample(0, 0, 0)]
```

### 8.3 One general statement of `of`

The `X of T` family — `vec2 of T`, `array of T`, `ref of T`, `view of T` — is now large enough that
its rule should be stated once, for the family rather than per type: `of` is part of the
**constructor keyword**, each constructor takes a **type** as its argument (§3, §7), so chained
forms such as `ref of array of i32` nest by construction and need no associativity rule. Today the
rule is written down only for vectors (`ARCHITECTURE.md` §5.20, `lale.md` § Vectors).

## 9. Costs and risks

### 9.1 Verbosity

The nested `array of array of i32` spelling is gone (§6), but a declaration that states its extents —
`2D array of i32 of 5 by 3 elements` — is still ~9 tokens. Acceptable only because it is confined to
the rare `unsafe` / `inline` cases that must state them.

### 9.2 Scope creep: the change is not confined to the array type

The change reaches the general type grammar, so it must be scoped and scheduled deliberately rather
than folded into the array semantics. Six things outside the array type itself are touched:

**a. `type_name` is the type slot for the whole language.** Nine grammar rules reference it, so
altering the array spelling alters the type spelling of every declaration form there is:

| Grammar rule                | What it types                       |
| --------------------------- | ----------------------------------- |
| `var_symbol`, `decl_symbol` | variable definitions, `unsafe decl` |
| `type_field`                | struct fields                       |
| `enum_variant_field`        | enum variant payloads               |
| `fn_parameter`              | parameters                          |
| `return_type`               | returns                             |
| `range`                     | loop variables                      |
| `conversion` (`x as T`)     | cast targets                        |
| `vector_type`               | vector element types                |

**b. `arr_index` is shared between types and indexing.** One rule serves `i32[5]` (a _type_) and
three value uses — `postfix` (`xs[i]`), `assign` (`xs[i] = v`), and `compound_assign`. Retiring the
bracket's _type_ role therefore means dropping the _reference_ from `type_name` and leaving
`arr_index` itself alone, so that indexing and assignment are not touched.

**c. The value grammar is reached too.** `[fill with 0]` → `[fill 10 elements with 0]` is
_expression_ syntax (`a_literal` / `primary`), documented as a language decision in
`ARCHITECTURE.md` §5.10 — not a type change at all.

**d. A new token shape, scoped to the type grammar.** `2D` / `3D` is the first digit-led token that
is not a number. There is no separate lexer pass — pest matches rules where they are referenced — so
the rank is recognised only inside `array_type`. Outside a type position, `2D` still reads as `2`
followed by `D`.

**e. The `of` family compounds.** The existing `vec2 of T` shares the constructor with the new array,
so `vec2 of array of i32` becomes expressible and §8.3's general `of` rule is pulled in — vectors are
affected by an array proposal.

**f. Outside the grammar.** The spelling also appears in the ABI and IR specifications and
throughout the array design document; §8.2 lists the full migration.
