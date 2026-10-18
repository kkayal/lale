# Dynamic Arrays and Views

**Status:** design proposal — **not yet approved**. Everything below is the intended design;
none of it is implemented until the design is approved (the array-notation branch carries a
feasibility spike only: the new grammar and the two tests). Cross-references such as
`ARCHITECTURE.md §5.14d` point at the current specification that the design extends or amends.

## How to read this document

The design has **two parts** with different schedules:

- **Part I — Dynamic arrays** (sections 2–7): the owned array type, plus a **narrow, whole-range
  reference** as the 1.0.0 non-owning form. **Planned for Lale 1.0.0.**
- **Part II — Ranged views** (section 8): non-owning **sub-ranges** (`view of T`, `xs[a to b]`).
  **Planned for a later version.**

Non-owning access (access that never frees the storage it touches) is not all-or-nothing. Lale
1.0.0 already uses non-owning access in three places: `ref` parameters (an expression-scoped
reference to the caller's storage); **references**, whose type is `ref of array of T` (§3.4),
produced by reading an `inline` field, by an FFI `ref` return, or explicitly by `ref to x`; and
`readonly` references (with the view layer, `readonly` views too), which forbid writes through the
holder. What Part II defers is the _ranged_ view — a sub-range window with its own freeze and
invalidation rules — not non-owning access as such.

Three concerns are often conflated but are kept separate here: **ownership** (who frees a
buffer), **references** (non-owning access), and **storage layout** (where bytes live). The
array type unifies their _surface syntax_ without pretending they have identical semantics.
Section 10 lists what lands in which version.

The **type notation** is part of this document: how the array type is spelled, and where the element
count is written (§2.9–§2.13). It was reviewed separately — `doc/array_type.md` keeps the full
rationale and the rejected alternatives, and is now a historic record — and it is merged here in
full.

### 1. The idea

Lale has a single array type: a **dynamic array** whose element count is a runtime value. A
dynamic array is carried as a **fat pointer** — a small **header** (a record of bookkeeping fields
that travels alongside the data) that bundles the address of the elements with their count — so one
type serves every count. This is the whole point:

- **The count is a runtime property.** A function such as `sum` accepts an array of any count
  without generics, because the count travels _with_ the value instead of being baked into its
  type.

```lale
fn sum(xs as array of i32) returns i32
    var total as i32 = 0
    loop var i as u32 from 1 to xs.count
        total = total + xs[i]
    end loop
    return total
end fn

var a as array of i32 = [10, 20, 30, 40, 50]
var b as array of i32 = [1, 2, 3]

write sum(a)   // 150
write sum(b)   // 6
```

`a` and `b` are the **same type**: `array of i32` (§2.9). The element count lives in the value, not
in the type, so `sum(a)` needs no conversion.

A later layer adds the **view**, a non-owning window onto a sub-range of an array (or of `text`
/ `binary`), so a function can read part of an array without copying it. Views are a separate
feature (section 8) and are not required by the array type itself.

## Part I — Dynamic arrays (planned for Lale 1.0.0)

### 2. The array type

The type is **spelled** `array of T` — one type, whatever the element count (§2.9). The word is
**"array"**, not "list": Python's `list` is heterogeneous and holds pointers to boxed objects,
while this type is homogeneous, contiguous, and growable, like Julia's `Array{T,1}` — and unlike
NumPy's arrays, which are fixed-shape. "Vector" is already taken by `vec2` / `vec3` / `vec4`.

#### 2.1 One type, runtime count

There is exactly one array type: **`array of T`** for an element type `T` (§2.9). There is no
separate fixed-size array type. The element count is **never part of the type** — it belongs to the
value — so a declaration supplies it through its initialiser (§2.10), or, where no value exists,
through the extent clause after the type (§2.11). Either way its count is whatever `.count` reports
at runtime.

The one type has one default representation: **heap-allocated** (§7.2) — the buffer lives in
long-lived memory that is freed only when the array is finished with it. A compiler may instead
place the buffer in a fixed-size **stack** slot — memory tied to the current function call and
reclaimed automatically when the function returns — when it can prove that the count never
changes (§7.3). This is an optimisation, never a semantic guarantee. The `inline` modifier (§4.1)
**enforces** the fixed-size form for a struct field, laying the elements out as the field's own
bytes so the field matches a C array member; but `inline` exists only for the C FFI context, so a
Lale programmer cannot force the optimisation on an ordinary variable.

#### 2.2 Declaration and initialisation

Arrays must be **completely initialised** (`ARCHITECTURE.md §5.10`) — no element may be left
with a garbage value. There are two initialisation forms, and both state the count **in the value**
(§2.10):

```lale
// Explicit elements: the count is the number of elements
var primes as array of i32 = [2, 3, 5, 7, 11]        // count 5

// Uniform fill: the fill says how many elements, then the value
var zeros as array of f64 = [fill 1000 elements with 0.0]
```

There is exactly **one** source of truth for the count: the initialiser states it itself — by how
many elements it lists, or by the `N elements` of a `fill` — so there is no second count to
disagree with it, and the old "declared 6, literal 5" mismatch cannot arise (§2.10). An
initialiser therefore always supplies the whole array.

**Fill evaluation.** `[fill N elements with expr]` evaluates `expr` **once** and replicates the
result into every element. For an element type that owns its buffer (a nested array, `text`,
`binary`, or a struct owning one), each element is a **deep copy** of that result, so the elements
are independent.

When an array must start uninitialised, the `unsafe` declaration states that explicitly — and since
there is no value to read a count from, the count is written after the type (§2.11):

```lale
unsafe decl buffer as array of i32 of 1000 elements   // the programmer must fill it before reading
```

That declaration creates an array whose `count` is **1000 immediately** — the elements are the
programmer's responsibility, exactly as with `unsafe decl x as i32`. The `unsafe` is on the
declaration; Lale does not track which elements have been written, so initialising every element
before use is the programmer's obligation, not a compiler guarantee. A systems language must allow
this, and it is explicitly marked `unsafe`.

In 1.0.0 `unsafe decl` is restricted to **cleanup-free element types**. If the element type itself
needs destruction — a nested array, `text`, `binary`, an optional, or a struct containing any of
these — an uninitialised element's garbage bytes could be followed by the destructor, so such a
declaration is rejected (§11.2).

#### 2.3 Count and capacity

`xs.count` reads the array's runtime element count. An array may be **empty** — an array with
zero elements is valid, and the empty literal is simply the count-zero case:

```lale
var empty as array of i32 = []
write empty.count   // 0 — any index into it is out of bounds
```

There are **no `count of` or `capacity of` operators**. `xs.count` and `xs.capacity` are
**read-only fields** of the array header. Both are runtime values; a compiler may fold either to a
constant where it is statically known. Capacity has these **language-level guarantees**:
`capacity >= count` always; `grow` never decreases `capacity`; `shrink` never increases it. The
exact capacity and growth factor remain unspecified, so a program must not assume a particular
value beyond those invariants, and the compiler remains free to choose the growth policy (§7.3).

A **zero-count** array needs no element storage, and the compiler never calls `malloc(0)`:
an array with `capacity == 0` has a **null** pointer, and any allocation has `capacity >= 1`.
A zero-count array may still have a non-null pointer when `capacity > 0` — for example after
`shrink` dropped the last element without releasing the buffer — but no valid element access
exists in either case, so nothing may dereference it. See §13.5 for the C boundary.

#### 2.4 1-based indexing and bounds checking

Indices start at 1 (mathematical convention, `ARCHITECTURE.md §5.9`):

```lale
var numbers as array of i32 = [10, 20, 30, 40, 50]
write numbers[1]   // 10 — the first element
write numbers[5]   // 50 — the last element
```

Every access is **bounds-checked** — the compiler inserts a check that the index lies within
`1 … count` before the element is read or written (`ARCHITECTURE.md §5.12`). An out-of-range index
aborts with a clear message rather than reading stray memory:

```text
ERROR at program.lale:3:10: array index out of bounds (index=6, count=5)
```

Multi-dimensional indexing (`m[2][3]`) is checked one dimension at a time.

#### 2.5 Physical units

An array may carry a physical unit for its elements, with the same rules as scalars
(`ARCHITECTURE.md §5.13`):

```lale
var temps as array of f64 in <K> = [273.15, 300.0, 310.5]
var positions as array of f64 in <m> = [fill 10 elements with 0.0]
```

Every element carries the array's unit through reads and writes — `temps[1]` has type `f64 in
<K>` — and a view (section 8 — Part II, deferred) inherits the unit of its origin.

#### 2.6 Global arrays are compile-time constants

A module-level `var` must initialise to a value the compiler can fold to a constant
(`ARCHITECTURE.md §5.29`), which exists to remove static-initialisation-order problems. For an
array this means **both the count and every element** must be constant:

- `var a as array of i32 = [10, 20, 30, 40, 50]` — count and elements are literals. ✓
- `var b as array of i32 = [x, y, z, w, v]` — the count is constant (five elements), but the
  elements are runtime values → rejected; move the setup into top-level code.
- `var c as array of i32 = make_ints(n)` — neither count nor elements are constant → rejected.

This is the same rule scalars already follow (`var x as i32 = read()` is rejected for the same
reason), so arrays add no new exception.

The constant folds into a **static** buffer: the elements live in the program's **static storage** —
memory baked into the program file itself and alive for the whole run — rather than the heap, so
**module load performs no runtime allocation**. A module-level array may still be mutated in place,
so that storage is writable; an array that `grow`s transitions to owned heap storage (§3.2).
Placement stays an implementation detail — a global that is never mutated could equally sit in
read-only storage.

An **imported** module-level array is shared by **reference**, like every imported variable
(`lale.md` § Module Execution Model): the importing module neither copies nor owns it, must not
free it, and sees the exporter's mutations. A `grow` / `shrink` / reassignment through the import
changes the exporter's array; the freeze rules of §3.4 apply to the importing module's code as
usual.

#### 2.7 Jagged arrays (nested dynamic arrays)

`2D array of i32` is a **dynamic array of dynamic arrays**, not a contiguous matrix (§2.12). The
element type of the outer array is a **row** (`array of i32`) — itself a dynamic array — so the
outer header's buffer holds two fat-pointer headers, each pointing at its own separately allocated
inner buffer:

```lale
var m as 2D array of i32 = [[1, 2, 3], [4, 5, 6]]
write m.count       // 2 — the outer count (rows)
write m[1].count    // 3 — the count of the first row
write m[2][3]       // 6
```

Consequences:

- **Allocation and freeing** are recursive: building `m` allocates the two inner buffers and
  then the outer buffer; freeing it releases the inner buffers and then the outer one.
- **Copying is recursive**: a deep copy of `m` copies every inner array (`ARCHITECTURE.md
§5.14d`).
- **Rows need not have the same count.** Because each inner array is an independent fat pointer,
  the shape is **jagged** — the literal above happens to give equal rows, but nothing requires
  it. There is no "all rows must be square" rule.
- **A chained index is an in-place path.** `m[2][3]` follows the stored row header in place; the
  intermediate `m[2]` is not deep-copied. A standalone read that binds the row — `var row =
m[2]` — is a value copy like any other array read.

A contiguous, fixed-size matrix is a **separate concern** and is not part of this design
(section 10). The one rectangular array form in Lale is the `inline` field, where the rectangle is
a C layout rather than a growable value (§4.1). This document only records that nested dynamic
arrays stay jagged.

#### 2.8 Growth

An array grows with `grow` and shrinks with `shrink`:

```lale
grow xs by 2 = [7, 8]   // append [7, 8]; count increases by 2
shrink xs by 3          // drop the last 3 elements; count decreases by 3
```

`grow` and `shrink` need an **owning** binding: a `ref` parameter may mutate elements but cannot
resize the caller's array (§3.1).

- The amount is `u64 in <elements>`. Lale has no implicit numeric conversion, so a `u32` variable
  must be widened explicitly — `grow xs by (n as u64)` — exactly as for every other numeric
  operation (§2.3). Because the type is unsigned, a **negative amount cannot be expressed**: a
  negative literal is a compile-time type error, and no well-typed runtime expression can be
  negative. `grow` / `shrink` never silently reverse direction, and there is no runtime
  negative-amount failure.

- `grow` is followed by an initialiser for exactly the new elements, because Lale requires
  complete initialisation: `grow xs by 2 = [7, 8]` supplies two explicit values, and
  `grow xs by 3 = [fill 3 elements with 0]` fills the three new elements uniformly — the fill states
  its count like every other initialiser (§2.11), so the amount is named twice here (a nit recorded
  in §11.4). An explicit literal whose count does not equal the amount grown (a mismatch that is
  statically known) is a **compile-time error**. A `grow` whose initialiser evaluates to `nothing`
  is **skipped** — no failure is involved (Annex B).
- `shrink` needs no initialiser (no new elements are created), but it **destructs the dropped
  elements**: a nested array, `text`, or `binary` element releases its own buffer. The array's own
  buffer is not freed.
- `grow` may reallocate the buffer; this is invisible under value semantics. A reallocation
  **moves** the existing elements: their bytes are copied to the new buffer and the old buffer is
  released without running the element destructors, so payloads owned by the elements (nested
  buffers, `text`, `binary`) are not copied.
- `grow xs by 0` and `shrink xs by 0` are **no-ops**.
- `shrink xs by n` with `n` greater than `xs.count` is a **failure** (it never silently
  underflows `count`).
- **Evaluation order (normative).** For `grow xs by n = init`: (1) evaluate `n` exactly once;
  (2) if `n == 0`, the grow is a **no-op** and `init` is **not** evaluated; (3) otherwise evaluate
  `init` exactly once; (4) if `init` evaluates to `nothing`, the grow is **skipped**; (5) otherwise
  the initialiser's element count must equal `n` — a statically known mismatch is a compile-time
  error, a runtime mismatch a failure; (6) only after steps 1–5 succeed may the buffer be
  (re)allocated, and the new `count` / buffer is committed last. A failure at any step aborts and
  leaves `xs` unchanged. `shrink xs by n` evaluates `n` once, then checks `n ≤ xs.count`, then
  destructs the dropped elements and lowers `count`.
- An **allocation failure** (out of memory) is a **failure** — the initial allocation of an array
  (a literal, a `fill`, a `copy` return, or a deep copy), or a reallocation during `grow`.
  See Annex B for the failure rules.
- **A failure is fatal.** A failure **aborts** the program with a clear message, and the target
  array is left unchanged up to the abort (never corrupted). 1.0.0 has no annotation that
  tolerates a failure; recoverable failure (`may fail`) is deferred (Annex B, §10).
- Growing a **nested** array (`grow m[1] by 2 = …`) follows the same rules for the inner array
  and leaves the outer array untouched, because each inner array owns its own buffer (§2.7).
- An indexed expression used as a **mutation target** — `grow m[1] by 2` or `m[1] = row` —
  designates the **stored** element, never a temporary copy produced by an ordinary read (§11.2).
- An **array-valued struct field** is a mutation target like any owning binding: `grow s.items by 2`
  and `s.items[1] = v` operate on the field's stored array, subject to the same freeze rule (§3.4,
  §8.5).
- `grow xs by n = f()` with a **runtime** `n` and a runtime-count initialiser is a **failure**
  whenever the initialiser's count differs from `n` (§11.2).
- If a deep copy must allocate several buffers (for a nested array) and an allocation fails
  partway, the buffers already allocated for the copy are released **before** the program aborts —
  cleanup itself must never double-free or leak.

#### 2.9 The type notation: `array of T`

The array type is written **`array of T`**. The bracketed forms no longer spell a _type_ — `i32[5]`
and `[i32]` are not ways to write it — while brackets keep both of their value roles: **indexing**
(`xs[i]`) and **value literals** (`[1, 2, 3]`). A _type_ therefore contains no brackets, and a
_value_ still does. (The full rationale, including every rejected alternative, is kept in
`doc/array_type.md`.)

`of` is part of a **two-word constructor**, like `pointer to`: `array of`, `ref of`, `view of`, and
the existing `vec2 of` each take a **type** as their argument, so chains nest right by construction
(`ref of array of i32` is `ref of (array of i32)`) and need no associativity rule. This is the same
`of` that `lale.md` documents for vectors: it separates _storage composition_ from _physical
dimensions_ (`in <unit>`), which is what keeps brackets out of the type grammar.

The spelling was chosen because it completes the `X of T` family — arrays were the last type still
spelled by punctuation — because `array of i32` reads as English where `[]` in a type position is
jargon, and because Ada and Pascal already spell arrays this way. It costs verbosity: a declaration
that states its extents, such as `2D array of i32 of 5 by 3 elements`, is ~9 tokens — acceptable
only because it is confined to the rare `unsafe` / `inline` cases that must state them (§2.11).

#### 2.10 The count is a value

The element count belongs to the **value**, not to the type. `array of i32` says nothing about how
many elements there are; the initialiser does (§2.2) — or, where no value exists, the declaration's
extent clause (§2.11) — and `.count` reads it back (§2.3).

That keeps one source of truth: there is no count in the type to keep in step with the count in the
value, so the two cannot contradict each other. The empty literal is the same rule at zero —
`var e as array of i32 = []` has count `0`, taken from the value, with no clause written.

Nothing is being converted: a count is a value, not a type, so taking it from the initialiser is
consistent with Lale's "no implicit conversions" rule.

#### 2.11 Where no value can supply the count

Four constructs cannot take the count from the value, so each obtains it another way:

| Construct                                   | Has a value?  | Why it needs the count                                                |
| ------------------------------------------- | ------------- | --------------------------------------------------------------------- |
| `[fill 10 elements with 0]`                 | yes (uniform) | a uniform value says nothing about _how many_                         |
| `unsafe decl b as array of T of n elements` | **no**        | it must allocate; there is no initialiser                             |
| `inline array of T of n elements`           | **no**        | the count _is_ the C array dimension (§4.1)                           |
| `returns ref of array of i32 of n elements` | **no**        | an FFI count binding: the count is a parameter's runtime value (§5.2) |

The clause uses bytes for `binary` and chars for `text`: `returns ref of binary of n bytes` and
`returns ref of text of n chars` (see §5.2).

The rule is that the count goes wherever the value can carry it: into the value for a `fill`, and
otherwise into the declaration, after the type, as an **extent clause**. The clause reads
`of N elements` — a **noun phrase**, so it is unambiguously a count — and more than one dimension
reads `of 5 by 3 elements` (§2.12). An extent may be a runtime expression (`of n elements`), and
`elements` is a suffix word, so it needs a word boundary (`elementsX` stays a different identifier).
Lale has no reserved words, so `array`, `of`, `elements`, and `by` remain ordinary identifiers.

Rejected spellings, and why: `count 5` (`array of i32 count 5`) reads unnaturally;
`[fill with 10 times 0]` reads as multiplication; `[fill 10 times with 0]` reads as _repeat the
fill_. A fill therefore always states its count — `[fill with 0]` is not kept, because it would read
its count from the declaration, a second source of truth.

#### 2.12 The rank prefix, and the forbidden nested spelling

Multi-dimensional arrays are written with a **rank prefix** on the type: `2D array of i32`,
`3D array of i32`, …. Rank 1 is the default and has no prefix.

**The nested spelling `array of array of …` is forbidden** — an `array of`'s element type is never
itself an array type. It is rejected because it is unwieldy to read and write, because it is
redundant with the rank prefix (an array of arrays _is_ a 2-D array — the semantics are nested
either way, §2.7), and because it is the one form where an extent clause binds to the wrong array.

The check is deliberately **not** made in the grammar, so that the diagnostic can name the fix: the
nested form parses, and semantic analysis rejects it.

**A reference or view is not an element type in 1.0.0.** `array of ref of text` would copy the
reference headers shallowly, so an array copy would alias the referenced data — contrary to the
deep-copy rule. Rejecting it is a pure restriction; allowing it later would be additive.

```text
ERROR: an array's element type may not itself be an array (`array of array of i32`):
       write `2D array of i32` for a 2-dimensional array
```

A grammar-level rejection could only report an unexpected token at the point of failure, which
leaves the writer without the reason.

The rank is also **checked against the value** where one is present — a rank-1 declaration whose
initialiser is two-dimensional is an error — and where the declaration states extents as well, the
number of extents must equal the rank:

```lale
array of i32 of 5 elements         // ✅ rank 1, one extent
2D array of i32 of 5 by 3 elements // ✅ rank 2, two extents
2D array of i32 of 5 elements      // ❌ rank 2, one extent
array of i32 of 5 by 3 elements    // ❌ rank 1, two extents
```

The prefix cannot be dropped: the rank must be expressible **without extents** — in a parameter, a
return type, a `ref of …`, or a non-`inline` struct field — and there the extents are not available. The cost is
a mild redundancy, since `2D` and the extent count say the same thing, accepted for teachability:
the writer never has to decide _when_ to write the prefix, and a mismatch is a compile error.

The extent separator is `by` (`of 5 by 3 elements`, "a 5 by 3 grid"). `5x3` cannot be used: ASCII
`x` is not an operator in Lale, and `×` (U+00D7) is excluded from the grammar
(`ARCHITECTURE.md §5.21`). The glued form `2Di32` / `3Df64` is rejected as well — nothing glues to
`2D vec3 of f64`, it abandons `of`, and it over-promises a rectangular shape.

#### 2.13 Number of elements: `count`

The vocabulary is standardised on **`count`**: an array's read-only `count` field, and diagnostics
say `count` too — for example `array index out of bounds (index=6, count=5)` (§2.4) — instead of the
older mix of `size` in compile-time messages and `length` in runtime ones.

`#type of` and `#unit of` remain compile-time operators. The other forms change:

- An array's `count` is a **read-only header field** (`xs.count`), not an operator. It reports the
  runtime element count; a compiler may fold it to a constant when the count is statically known.
  `text` and `binary` keep their own fields — `t.chars` / `t.bytes` and `b.bytes` — because their
  element unit is a code point or a byte, not an array element; there is no `.count` on them.
- `size of` is a **runtime** operator returning `u64 in <bytes>`. It reports the number of bytes
  occupied by the value's **contents** — for an array, `count × sizeof(T)` (the **live** elements,
  not `capacity`); for a view, `(last - first + 1) × sizeof(T)`; for `text` / `binary`, `.bytes` —
  not the fat-pointer header. For a scalar, struct, or enum it is the value's layout size; for a
  pointer it is the target **ABI**'s pointer size — an ABI (Application Binary Interface) is the
  exact byte layout and calling rules that two separately compiled pieces of code agree on — which
  is 8 bytes on the 64-bit platforms Lale 1.0.0 targets, generalised by `usize` / `isize` in 2.0.0
  (§10); for a reference it is the content size of the value it refers to (for an array reference,
  `count × sizeof(T)`). It is a **shallow** size: the payloads of nested heap buffers are not
  included. A compiler may fold it to a constant where the size is statically known, but there is no
  separate compile-time-only form. `#size of` becomes `size of`; this is a semantic change for
  `text` / `binary` (which previously reported the fat-pointer header rather than the content) and
  makes the operator runtime for a dynamic array (§13.14).

This is part of the design in this document. The wording change and the operator changes are
compiler work, and land with the array implementation (§13.14).

### 3. Value semantics and ownership

#### 3.1 Copy by default

A dynamic array **owns** its buffer — it is responsible for freeing that memory — and has
**value semantics**: assigning or passing one by value produces an independent **deep copy** (the
elements themselves are duplicated, not just the small header), so one variable can never silently
change another through a shared underlying buffer (`ARCHITECTURE.md §5.14d`).

```lale
var a as array of i32 = [1, 2, 3]
var b as array of i32 = a   // deep copy — b owns its own [1, 2, 3]
b[1] = 99                   // a is unchanged
```

Because the buffer is heap-allocated by default and the copy is `O(n)`, the compiler warns once, at
the function definition, when an array parameter is copied at every call — and `ref` is the way to
avoid the copy:

| Modifier | Meaning                                                        |
| -------- | -------------------------------------------------------------- |
| _(none)_ | copy by value                                                  |
| `ref`    | mutable reference — the callee may change the caller's value   |
| `copy`   | explicit copy — acknowledges the copy and silences the warning |

```lale
fn total(xs as array of i32) returns i32        // copy: warns (O(n) per call)
fn total(ref xs as array of i32) returns i32    // reference: no copy, no warning
fn total(copy xs as array of i32) returns i32   // explicit copy: no warning
```

A `ref` parameter can **mutate elements** of the caller's array, but it can **not resize** it: a
`grow` or `shrink` needs an owning binding (§11.2). Resizing through a `ref` is excluded because it
could invalidate a reference or view the caller holds, across a call the compiler cannot analyse.
The exclusion covers **every** array reachable through the parameter, including an array-valued
element or struct field: inside the callee, `grow m[1] by 2` or `grow s.items by 2` is a compile
error when `m` / `s` is a `ref` parameter, because it could invalidate a caller's reference to that
inner array. Replacing an element or field that owns its own buffer, directly or through a struct
field — a nested array (`m[1] = row`), a `text` (`xs[i] = "new"`), or a `binary` — is likewise a
compile error: it is an element-level rebinding that would invalidate a caller's reference to that
element. A by-value (`copy`) parameter can
neither mutate the caller's value nor resize it. A **`readonly`** reference (§3.4) is the cheap,
write-forbidding counterpart.

A `ref` parameter never permits **whole-variable rebinding** (`xs = other`): it grants element
mutation only — no rebinding and no resizing — uniformly across origins, so the rule does not
depend on the call site (§11.2).

**Array equality is deep, like assignment.** `a == b` compares the counts, then the elements
pairwise; `a != b` is its negation. It costs `O(n)`, exactly as the `O(n)` deep copy of `a = b`
does, so the two operators are consistent: value semantics make both proportional to the count.
Elements are compared with their own `==` — a nested array compares deeply, a `text` element
compares as `text`, and a float element follows the ordinary float rule, `NaN` behaviour included.
That is the majority choice (Swift, Rust, and Julia propagate `NaN != NaN`); Java's
`Arrays.equals` and C#'s `Double.Equals` instead provide a **total** comparison in which `NaN`
equals `NaN`. A total, `isequal`-style operation is conceivable as a later addition, but this
document promises nothing and `==` would not change. If the element type has no `==`, the array
comparison is rejected.

**Arrays inside structs.** A non-`inline` `array of T` field is an ordinary owning value: copying
or assigning the struct deep-copies the array, and the struct's scope exit frees it. The copy cost
is `O(n)` per array field, exactly as for a bare array. An `inline` field is the opposite — its
bytes are part of the struct, so a struct copy copies them inline and there is no allocation
(§4.1).

#### 3.2 Normative ownership and lifetime rules

These rules are the source of truth for both the **interpreter** (the backend that runs the
program's internal form directly) and the (future) **AOT** compiler (Ahead-Of-Time: one that
translates the program to native machine code before it runs). An AOT backend cannot hide an
ownership mistake the way the interpreter's leak detector can.

| Operation                                          | Copy or move?                              | Who allocates the buffer                   | Who frees it, and when                                                                                              |
| -------------------------------------------------- | ------------------------------------------ | ------------------------------------------ | ------------------------------------------------------------------------------------------------------------------- |
| Initialise from a literal                          | —                                          | the compiler                               | the compiler, at scope exit                                                                                         |
| Assign one array to another (`a = b`)              | deep copy                                  | the compiler (for `a`)                     | the compiler, at `a`'s scope exit; `b` is untouched                                                                 |
| Pass an array by value (no modifier)               | deep copy                                  | the compiler (for the parameter)           | the compiler, when the callee's frame is reclaimed                                                                  |
| Pass an array by `ref`                             | no copy                                    | —                                          | never (the callee refers to the caller's storage)                                                                   |
| Return an array (`return a`)                       | transfer, or heap copy if stack/static     | the compiler (for the copy)                | heap buffer → moves to the caller; a stack-lowered or static buffer → a heap copy is made, owned by the caller      |
| Bind a call result (`var a as array of i32 = f()`) | move (rvalue)                              | the compiler (the result is already owned) | the compiler, at `a`'s scope exit; no temporary is left to release                                                  |
| Reassign an owned array (`a = b`, `a` non-empty)   | deep copy                                  | the compiler (for the new `a`)             | `a`'s **old** buffer is released as part of the assignment, then `a` holds the copy                                 |
| Grow an owned array (`grow a by n`)                | old elements copied only if it reallocates | the compiler, if a new buffer is needed    | when the grow commits, the old buffer is released; a failed grow frees the new buffer and keeps the old one         |
| Shrink an owned array (`shrink a by n`)            | no element copy                            | —                                          | the dropped elements are destructed (freeing any buffers they own); the array's buffer and `capacity` are unchanged |
| Leave a scope                                      | —                                          | —                                          | every live local array is freed, in reverse order of creation, exactly once                                         |
| Receive a C buffer                                 | per FFI modifier                           | foreign side (original); Lale (copy)       | the program, via the library's deallocator; **never Lale** for the original (see note)                              |

Notes:

- **Return: transfer, or a heap copy.** A returned array cannot be freed by the callee (it would
  be lost). If its buffer is on the **heap**, ownership moves to the caller with **no copy**. If
  the buffer was placed on the **stack** by the count-never-changes lowering (§7.3), it cannot
  outlive the frame, so the return **materialises a heap copy**. If the buffer is a module-level
  array's **static** storage (§2.6), returning it also **materialises a heap copy** — static
  storage must never be handed to the heap deallocator — and the global binding is **never
  moved-from**. Either way the caller receives an owned, heap-backed array — the only form that can
  outlive the callee.
- **Growth and static storage.** A module-level array that `grow`s copies its elements from
  static storage into a new heap buffer; the static storage is never freed (see the static-storage
  bullet below).
- **Moved-from.** After an ownership transfer (such as `return a`), the source `a` is
  **moved-from**: it must not be read, and scope cleanup must not free it. Reading it is a
  **compile error** (a definite-assignment-style linear pass, §11.2). Reinitialising it makes it
  usable again. (Move semantics for transfer coexist with deep-copy assignment: ordinary `a = b`
  still copies.)
- **Freeing the received value.** The caller owns the received array and frees it at its own scope
  exit — or immediately if it reassigns the variable that holds it. A returned value used directly
  as an expression (an rvalue) is released at the end of the statement it appears in, unless it is
  bound to a variable — binding moves it (see below).
- **Binding a call result — rvalue moves, lvalue copies.** `var a as array of i32 = f()` binds an
  **rvalue** (a temporary value with no name — a call result, a conditional expression, or another
  temporary), so the temporary is **moved** into `a`: one allocation, no copy, and nothing left to
  release. Binding an **lvalue** (a named place — a variable, an element, a field) still copies:
  `var a as array of i32 = b` deep-copies `b`, and `var a as array of i32 = m[1]` copies the stored
  row. A literal is constructed directly in `a`. This is the ordinary
  value-semantics rule and matches the move already performed on `return`; allocation counts are
  therefore deterministic.
- **Reassignment order.** For `a = b`, the compiler evaluates the copy of `b` first, releases
  `a`'s old buffer, then stores the copy. This makes `a = a` safe.
- **`ref` duration.** A `ref` array parameter refers to the caller's storage only for the
  duration of the call; the callee must not retain it afterwards (the per-function escape rule,
  `ARCHITECTURE.md §5.14`). It may mutate non-replacing elements but may not `grow` / `shrink`,
  rebind, or replace an element or field that owns its own buffer (§3.1).
- **A foreign buffer is never freed by Lale.** For `ref` / `readonly ref`, the program releases the
  original with the library's own function; for `copy`, Lale owns and auto-frees its **copy**, and
  the original remains the program's to release (§5.2).
- **Static storage is not heap memory.** When the compiler places a buffer in **static storage** —
  a module-level array (§2.6) — that storage must **never** be handed to the heap deallocator; it
  is released, if at all, only with the program's data segment. An array that later needs a
  different buffer (a `grow`, or any operation that no longer fits the fixed static slot) first
  **transitions to owned heap storage** and is an ordinary owned array from then on. Program-exit
  cleanup frees heap buffers but never static storage. Any **heap payloads** owned by elements of
  a static-backed array — a nested array's buffer, a `text`, a `binary` — are ordinary heap
  allocations: they are released at program exit (or when the element is dropped), exactly as for a
  local array. Only the static element storage itself is never freed. Where the bytes live stays an
  implementation detail; these ownership consequences are the observable part.

#### 3.3 Local vs global lifetime

A local array lives until the end of the function that created it; a module-level array lives
until the program ends. Deep copies make this simple: because `a = b` copies, `b` must stay valid
only until the end of its own scope — the copy in `a` is independent and survives `b`.

#### 3.4 The reference type: `ref of T`, `ref to x`, `readonly`

A **reference** is Lale's non-owning value for a count-carrying type: it _points at_ an array (or
a `text` / `binary`) that someone else owns. It carries the **same header as its source type**, but
it owns nothing: making one copies no elements, and the compiler never frees it. For an array that
header is `{ ptr, count, capacity }`; for `text` it is `{ ptr, bytes, chars }`; for `binary` it
is `{ ptr, bytes }`. A reference to a `text` / `binary` has no `capacity`. References are the 1.0.0
half of non-owning access (§4.2, §8.3); the _ranged_ **view** is deferred (Part II).

**The type.** A reference is written `ref of T`, with the **source type** after `of` — the same
parameterised-type form as `vec2 of T` and `view of T` — and, like `array of`, it contains no
brackets (§2.9):

```text
ref of array of i32   // a reference to an i32 array
ref of array of f64
ref of text           // a reference to a text
```

**The value.** A reference to an array, `text`, or `binary` that Lale already owns is created with
`ref to`, the analogue of `pointer to`:

```lale
var xs as array of i32 = [1, 2, 3]
var r as ref of array of i32 = ref to xs   // no copy; r points at xs itself
write r[2]                                 // reads xs[2] through the reference

var s as text = "abc"
var rs as ref of text = ref to s           // the same form works for text and binary
```

Reading an `inline` field (§4.2) or binding an FFI `ref` return (§5.2) **produces** a reference
without `ref to`; `ref to` is the explicit form for an owning array, `text`, or `binary`.

**`ref` parameter versus `ref of T` value.** These are two different things. `fn f(ref xs as array
of i32)` passes the caller's array **by reference**: `xs` is an alias, element writes go to the
caller's storage, and the callee may not rebind or resize it (§3.1). `fn g(xs as ref of array of
i32)` passes a reference **value**: the header is copied, element writes still go through the
copied `ptr`, and rebinding `xs` inside `g` changes only the local copy. A caller supplies a
reference value explicitly (`g(ref to xs)`); an owning array is not implicitly converted to a
reference.

**Read-only references.** `readonly` is a **capability modifier** on a reference (or a view): it
says the holder may not _write_ through it — no element writes, no `grow`, no `shrink`. It replaces
the earlier FFI-only `const ref`, and it introduces no `const` keyword anywhere.

```lale
var ro as readonly ref of array of i32 = ref to xs   // ro is read-only; xs may still change
```

`readonly` names a **permission the holder is denied**, not a property of the data: `xs` may still
be changed through `xs` itself, through another (writable) reference, or — at the FFI boundary — by
the foreign side (§5.3). It is therefore more honest than `const`, which would suggest the data
cannot change. Accepting a _declared_ capability re-opens `ARCHITECTURE.md §5.2`'s "infer, don't
declare" principle; the justification is recorded in §9 and the decision in §11.2.

**Header fields are read-only.** The address and length fields of the fat pointers — `text.ptr`,
`binary.ptr`, and an array's `ptr`, `count`, and `capacity` — may be read but not assigned from
Lale code: `s.ptr = p` and `xs.count = n` are compile errors. The compiler owns the header. This is
a targeted rule for the compiler-managed fat pointers, not a general `readonly` field modifier;
enforcement is a work item (§12).

**Variance.** A writable reference may be passed or assigned to a `readonly ref` parameter or
variable (the restriction is added); a `readonly ref` may not be passed to a writable `ref`
parameter or assigned to a writable reference. The same rule applies to views. A view taken through
a `readonly ref` is itself `readonly`; a writable view cannot be obtained from a read-only
reference.

**Why a reference and not a raw `pointer`?** A raw pointer carries no length; a reference carries
its source type's header — `count` / `capacity` for an array, `bytes` / `chars` for `text`,
`bytes` for `binary` — so the length field, indexing, and bounds checking work through it. That is
what earns the reference its place for the **count-carrying** types. A reference is the **direct**
non-owning form; a ranged **view** (§8.3) is the **header-following** form, holding a pointer to
the origin's header (a pointer to a pointer), so it survives growth at the cost of one extra
indirection. The two are deliberately different concepts.

**Escape.** A reference **escapes** when it is allowed to outlive the storage it points at — a
dangling hazard. A reference that refers to a **local** array, local `text`, or local `binary` —
directly, or through a field of a local struct (an `inline` field or an ordinary array / `text` /
`binary` field) — must not be **returned** from the function or **stored in a global**, because the
local's storage is reclaimed when the function returns. The check is per-function and syntactic,
extending the raw-pointer escape rule (`ARCHITECTURE.md §5.14`), and it is **transitive through
`type` values**: a struct that contains a reference to a local is itself marked, so returning that
struct or storing it in a global is a compile error. §8.5 applies the same boundary to ranged
views.

**Freeze.** A reference holds a **copy** of the origin's header — `ptr`, `count`, `capacity` for an
array; `ptr`, `bytes`, `chars` for `text`; `ptr`, `bytes` for `binary` — not a pointer to the
header. So once a reference to an array is **taken**, that array must not be `grow`n, `shrink`n, or
reassigned for the rest of the function — the freeze is conservative and keyed on the `taken` mark
(§8.5), not on whether the reference is still used: any of the three would leave the reference with
a stale count or a released buffer. If the reference refers to an **element or field** that owns
its own buffer (a `text`, a `binary`, or a nested array), that element or field must not be
reassigned either — the element-level freeze of §11.2. For `text` and `binary`, reassignment is the
invalidating event. All of these are **compile errors** (the `taken` mark of §8.5), and the
text/binary rule is part of **Lale 1.0.0**. A reference to a **global** origin may escape, and the
per-function freeze does not see invalidating operations in other functions; the rule for that
case is an open decision in §11.1.

The freeze is **structural**: writing an element — `xs[i] = v`, or `r[i] = v` through the
reference — stays allowed, because it neither moves the buffer nor changes the count. The exception
is a reference to an element that itself owns a buffer (`xs[i]` a `text` / `binary` / nested
array): reassigning that element is a **compile error** under the element-level freeze (§11.2). A
ranged view is less restrictive, because it follows the header and therefore **survives growth**; it
freezes only `shrink` and reassignment (§8.5).

**Scope.** In 1.0.0 a reference exists only for those count-carrying types. Extending `ref of T`
to _all_ types (`ref of i32`, `ref of Sample`) is left open (§11.1); it would be additive, not
breaking.

### 4. Struct arrays: the `inline` field

C stores an array inside a struct as **inline bytes** (`struct { int coords[3]; }` is 12 bytes,
not a pointer). A dynamic array is a 24-byte header, so a naive `char[14]` field would put a
header where C expects 14 inline bytes — and there would be no pointer to hand across the C
boundary. The `inline` modifier reconciles the two.

#### 4.1 `inline array of T of n elements` — a fixed inline field

`inline` is a **field-only** type modifier, allowed **only inside a `type` definition** (never in
a `var`, a parameter, or a return type — a hard grammar rule, not a convention):

```lale
type Sample
    t as f64
    values as inline array of i32 of 3 elements
end type
```

`inline array of i32 of 3 elements` lays out three **contiguous** inline `i32`s (12 bytes) with the
alignment of `i32` — byte-for-byte identical to the C array **member** `int values[3]`. The extent
must be a **compile-time constant**; a runtime count is the deferred flexible form below. The
**enclosing** struct is C-compatible only when its **complete** layout — field offsets, padding,
alignment, and every field's type — is verified against the target ABI (§13.3, §13.5), not merely
because one field is an inline array.

`inline` is the **enforced** form of the layout optimisation of §7.3: the count is fixed, and the
buffer is the field's own bytes rather than a heap allocation. It exists only for the C FFI context
— it is field-only, and its purpose is to reproduce a C array member — so a Lale programmer cannot
use it to force stack allocation of an ordinary variable. The field may also carry a **rank**, and
then it is the one rectangular array form in Lale: `inline 2D array of i32 of 5 by 3 elements` is a
fixed C `int[5][3]` member. Its rectangle is part of the ABI, so there is nothing to grow (§2.7).

The element type must have a defined C layout: `i8` / `i16` / `i32` / `i64`, `u8` / `u16` / `u32` /
`u64`, `f32` / `f64`, `byte`, `bool`, `pointer`, or a `type` whose own layout is verified
C-compatible. `char` (4 bytes — a Unicode code point, not C's 1-byte `char`), `f16`, `text` /
`binary`, `T?`, vectors, and dynamic arrays are excluded (§11.2).

The **flexible** form, `inline array of T of len elements`, is a C99 flexible array member: a
trailing field whose element count is read from another (named) field `len` at construction. It is
**deferred** (section 10) — it needs variable-sized-struct semantics.

#### 4.2 Reading an `inline` field yields a reference

Reading an `inline` field produces a **reference** — a value of type `ref of array of T` (§3.4) — a
non-owning, whole-range view of the inline bytes. A reference is part of **Lale 1.0.0** (it is
what `inline` fields and FFI `ref` returns produce); only the _ranged_ view of Part II is deferred.
The same type is what `ref to x` yields for an owning array (§3.4). Its `count` is `n` and its
`ptr` addresses the inline bytes. No copy occurs, and the ordinary array operations — `.count`,
indexing, iteration — apply unchanged:

```lale
fn sum(xs as array of i32) returns i32
    var total as i32 = 0
    loop var i as u32 from 1 to xs.count
        total = total + xs[i]
    end loop
    return total
end fn

// s is built elsewhere; s.values holds three inline i32s
write sum(s.values)   // s.values is a reference over the three inline elements
```

The same `sum` works whether the array lives on the heap, on the stack, or inline in a struct.

#### 4.3 `ref` over an `inline` field

Because the field is inline bytes rather than a header, `ref` binds a temporary reference header:

```lale
fn sum(ref xs as array of i32) returns i32
    // no copy: xs refers to s's inline bytes
    ...
end fn
```

The compiler (1) takes the field's address, (2) materialises a reference header
`{ ptr = address of s.values, count = 3, capacity = 3 }` on the stack, and (3) binds `xs` to
it. The result is **mutable elements, not a reassignable variable**: `xs[i] = v` writes through
the reference header into the inline bytes, but reassigning the whole array (`xs = other`) would
write to the temporary and be lost, so it is rejected. Resizing is likewise unavailable for a
reference: `grow` and `shrink` require an **owning** binding (§3.1), and a reference is never one,
so any attempt to resize through a reference is a **compile error**, regardless of where the
reference came from.

**Freeze.** A reference into an `inline` field points at the struct's own bytes, so reassigning the
containing struct while the reference is live is a **compile error**, and so is reassigning the
field itself (`s.values = other`) — either would overwrite the bytes. If the struct is an element of
an array, taking the reference also marks the root array as `taken` (§8.5). Element writes through
the reference remain allowed.

#### 4.4 Bounds checking, without `unsafe`

Both `inline` forms get the ordinary runtime bounds check (`ARCHITECTURE.md §5.12`): the fixed
form checks against the extent `n`; the flexible form (deferred, section 10) checks against a
runtime field. The only thing the check cannot catch is a count field that overstates the real
buffer — the same "corrupted count" hazard a `text.bytes` field can present. The header fields are
**read-only** (§3.4), so safe Lale code cannot change the count; an `unsafe` write or a foreign
buffer that lies about its extent remains the programmer's responsibility, not a reason to mark
the field `unsafe`.

### 5. The C FFI boundary

An **FFI** (Foreign Function Interface) is the point where Lale code calls, or is called by, code
written in another language — here, C. At this boundary Lale's automatic rules (ownership, bounds
checking, the fat pointer) no longer apply, and the programmer must state how a foreign pointer is
owned and freed.

#### 5.1 Ownership and the `C` boundary

A C function can hand Lale a pointer that Lale must **not** free, or one that Lale **must** free
— and there is no way to tell from the pointer alone. Since a Lale dynamic array is **owning and
auto-freed**, installing a foreign pointer in the header without taking ownership would make Lale
free a buffer it does not own (and the header fields are read-only, §3.4). Ownership at the
boundary is therefore **explicit, with no default**, using the existing `ref` / `copy` keywords and
the `readonly` capability:

| Modifier       | Meaning at the FFI boundary                                              |
| -------------- | ------------------------------------------------------------------------ |
| `ref`          | **reference** — non-owning; Lale never frees the buffer                  |
| `copy`         | **own a copy** — deep-copy into a Lale-owned buffer that Lale auto-frees |
| `readonly ref` | **reference, read-only** — like `ref`, plus "may not write through it"   |

`readonly` is the general **capability modifier** of §3.4 (it forbids writes through the holder); at
the FFI boundary it carries the same meaning, mirroring C's `const`.

An array **return** must carry one of the three — omitting the modifier is a compile error, because
the compiler cannot infer who owns the buffer:

```lale
// Non-owning: Lale never frees the buffer.
import fn signature ints_view(n as u64) returns ref of array of i32 of n elements

// Owning: Lale deep-copies, and frees the copy at scope exit.
import fn signature ints_copy(n as u64) returns copy array of i32 of n elements

// Non-owning and read-only.
import fn signature ints_ro(n as u64) returns readonly ref of array of i32 of n elements
```

The declaration is a **claim about the foreign code**, not something the compiler can verify: the
programmer must read the C documentation and state who frees the buffer. Every language with an FFI
puts that burden on the binding author — C, Rust, Go, and Swift all require the binding to encode
the foreign ownership contract.

The FFI boundary is spelled with the **`C` modifier**. Lale has **two export forms**, because the
two audiences have different ABIs (the byte-level layouts and calling rules each audience expects):

| Form          | Audience  | ABI                       | `array of T` parameter? | Symbol name  |
| ------------- | --------- | ------------------------- | ----------------------- | ------------ |
| `export fn`   | Lale code | Lale ABI (24-byte header) | yes                     | mangled      |
| `export C fn` | C code    | C ABI (pointer + count)   | no                      | plain C name |

`C` is a **contextual modifier**, meaningful only immediately after `export` — consistent with
Lale's "no reserved words" rule. It marks the **C ABI** and ties the symbol name to it: Lale-internal
names are **mangled** (the compiler rewrites them into a unique encoded form), while C names are
left plain (`ARCHITECTURE.md §4.6`). The import side carries no marker, because
`import fn signature` is already unambiguously the C → Lale form
(`lale.md` § _Importing C Function Signatures_): there is no second thing an `import fn signature`
could mean. The asymmetry is deliberate — `C` disambiguates an **export**, which would otherwise
read as Lale-to-Lale.

```lale
// Lale-to-Lale export — the Lale ABI; a dynamic array parameter is fine
export fn total(xs as array of i32) returns i32
    var sum as i32 = 0
    loop var i as u32 from 1 to xs.count
        sum = sum + xs[i]
    end loop
    return sum
end fn

// C export — the C ABI; only C-compatible types are allowed
export C fn total_c(data as pointer, count as u64) returns i32
    // `data` points at `count` i32s; index them with `unsafe value at`
    ...
end fn
```

```lale
// ❌ compile error: an `array of T` has no C layout
export C fn bad(xs as array of i32) returns i32
    ...
end fn
```

```lale
// ✅ Lale → C: the caller passes the data pointer and the count explicitly (§13.5).
import fn signature c_sum(data as pointer, count as u64) returns i32

var xs as array of i32 = [10, 20, 30]
write c_sum(xs.ptr, xs.count)
```

The call has the same arity as the declaration — one argument per declared parameter — so the
ordinary argument-count rule applies; nothing is silently expanded. `xs.ptr` is the address of
the element buffer and `xs.count` is its element count.

**A unit note.** `xs.count` carries the unit `<elements>`, while the imported parameter
`count as u64` declares no unit. Unit checking follows `lale.md`: a mismatch against a **declared**
unit is an **error**, in a Lale-defined function and in an import alike. A parameter that declares
no unit has no declared unit to mismatch; for an **imported** parameter the compiler emits a
**warning** instead, because a foreign declaration carries no unit information and the binding
author is responsible for the match. To make the contract checked, declare the unit in the import
(`count as u64 in <elements>`) and a mismatch becomes an error. The same warning applies to
`text.bytes` (`<bytes>`) and `binary.bytes` passed to unitless C parameters.

**A rank-2 warning.** For a rank-2 array, `m.ptr` addresses the outer buffer of **row headers**,
not a flat `T` buffer. A C function expecting `T *` must not be given `m.ptr`; a jagged array
needs one call per row (a contiguous matrix is a separate type, out of scope here).

The ownership marking (`ref` / `copy` / `readonly ref`, above) applies to a value that **crosses
from C into Lale**: an `import fn signature` **return value**. An `export C fn` parameter is also
C → Lale data, but it is a bare `pointer` + `count` — an array cannot cross the C ABI — so it has
no spelling for these modifiers: the C caller owns the buffer, and Lale must not free it.

Ownership stays with the allocating side in the other direction too. Lale owns `xs` and only lends
its data pointer for the duration of a call, so C must not retain it; the C caller owns `data`, and
Lale must not free it.

**Ownership is visible in the value.** A reference result (`ref`) stays **non-owning**: it cannot
be bound to an owning `array of i32`, because the compiler would then free a buffer it does not own.
Taking ownership requires the explicit `copy`.

#### 5.2 Return values

**Case A — the import is a bare pointer (no array-ness, no count):**

```lale
// C: int* make_ints(size_t n);   // malloc'd; the caller owns and frees it
import fn signature make_ints(n as u64) returns pointer

var p as pointer = make_ints(5 as u64)
free(p)   // you own it, free it
```

Here `free` is correct only because this buffer came from the **platform allocator** — the system's
memory manager, the one `malloc` and `free` talk to — as the `malloc'd` comment states. For a buffer
from a library with its **own allocator** (its own matching pair of allocate/release functions),
`free` is wrong — the library's release function is the only valid one (see the prominent example
below).

**Case B — the import returns an array.** Two ownership modes. Unlike Case A, the two array
declarations use **different foreign contracts**, because `copy` is only valid when the foreign
side retains the original (§11.2).

_Own a copy (`copy`) — the 1.0.0 path:_ the foreign side is a **getter that retains ownership** of
the buffer it hands back.

```lale
// C: const int* lib_ints(size_t n);   // pointer into library-owned storage; the library retains it (caller must not free)
import fn signature lib_ints(n as u64) returns copy array of i32 of n elements

var xs as array of i32 = lib_ints(5 as u64)   // a Lale-owned deep copy; the library still owns the original
write xs[1]
```

`copy` deep-copies the foreign buffer into Lale-owned memory; `xs` is an ordinary owning array.
Because the library retains the original, Lale **must not** free it, and neither must the program —
the library's own release, if any, is the only valid one. A **transferred** allocation cannot be
handled by `copy`: the program would hold only the copy and no pointer with which to release the
original. `copy` is therefore restricted to the case where the foreign side **retains** the
original (§11.2).

_Reference (`ref`) — non-owning:_

```lale
// C: int* lib_alloc_ints(size_t n);   // library-allocated; release with lib_free, not free()
import fn signature lib_alloc_ints(n as u64) returns ref of array of i32 of n elements

var xs as ref of array of i32 = lib_alloc_ints(5 as u64)   // non-owning reference; count = 5
write xs[1]
lib_free(xs.ptr)   // the program frees the ORIGINAL with the library's own deallocator
```

`ref` is non-owning: Lale never frees the buffer. The non-owning binding type is `ref of array of T`
(§3.4). `lib_free` here stands for the library's **own** release function — never `free` (see
the prominent example below).

Here the extent clause `of n elements` is not a type-level count — the array type carries no count —
it is an FFI-import **count binding**: the returned array's `count` is the runtime value of the `n`
parameter.

**Prominent example — a library with its own allocator.** Many C libraries do not use the
platform allocator, so `free` is simply **wrong** for their buffers: the library provides the only
function that may release them. Lale therefore never releases a foreign buffer — the program must,
with the library's own function:

```lale
// FFTW allocates with fftw_malloc and releases with fftw_free — NOT free():
//   double *fftw_alloc_real(size_t n);
//   void    fftw_free(void *p);
import fn signature fftw_alloc_real(n as u64) returns ref of array of f64 of n elements
import fn signature fftw_free(p as pointer) returns nothing

var x as ref of array of f64 = fftw_alloc_real(1024 as u64)   // x refers to the library's buffer
// ... use x ...
fftw_free(x.ptr)   // ✅ the library's own deallocator — never free(x.ptr)
```

This is why foreign-buffer deallocation is a **requirement**, not an open point: a language that
insisted on freeing the original itself could not use any library with a custom allocator.

**Text and binary returns.** The extent clause uses the type's element unit: bytes for `binary`,
code points for `text`. Both ownership modes are available, exactly as for an array: `ref` is
non-owning, `copy` deep-copies into Lale-owned storage.

```lale
import fn signature read_buf(n as u64) returns ref of binary of n bytes
import fn signature read_buf_copy(n as u64) returns copy binary of n bytes

import fn signature read_text(n as u64) returns ref of text of n chars
import fn signature read_text_copy(n as u64) returns copy text of n chars
```

`ref of binary of n bytes` is a non-owning reference whose `ptr` is the foreign buffer and whose
`bytes` is the runtime value of `n`; Lale never frees it. `copy binary of n bytes` copies exactly
`n` bytes into a Lale-owned buffer that is auto-freed at scope exit. The same restriction as for
an array applies: `copy` is for a getter that keeps the original, because a transferred allocation
would leave the program without a pointer with which to release it (§11.2).

`text` counts code points, so its **descriptor** — the small structure that describes a buffer,
its pointer plus its lengths — is spelled in chars. The runtime scans from `ptr` until it has
counted `n` code points, sets `bytes` to the number of bytes consumed, and checks the `text`
invariant — a **null terminator** (a zero byte that marks the end of a C-style string) at
`ptr[bytes]`. The existing `__lale_construct_text_in_place` helper performs this scan and returns an
owning `text`, so it is
the natural implementation of `copy text of n chars`; the reference path needs the same scan to
fill a reference header (or a sibling helper returning `ref of text`). The scan is `O(bytes)`; it
counts code points but performs **no implicit UTF-8 validation** — the binding author must ensure
the bytes are valid. A missing terminator or fewer than `n` code points is a runtime failure. For
`ref`, the buffer remains foreign and is never freed by Lale; for `copy`, the copy is Lale-owned
and the original is never freed by Lale.

#### 5.3 `readonly ref` — what it does and does not promise

`readonly ref` is a **Lale-side "will not write through it" marker** — the §3.4 capability
modifier applied at the boundary. It does not stop the foreign side
from mutating the buffer — C can still write through its own pointer, another alias, or another
thread. This is the same weakness as C's own `const` (and Rust's `*const T`), and it is inherent
to FFI: no language can prevent foreign code from mutating data it still holds. Therefore:

- `readonly ref` is **a declaration of the interface contract**, not an immutability guarantee. The
  compiler must **not** use it to licence optimisations that assume the buffer is stable across
  reads (for example, caching a value it read once).
- The only mode that gives a real "it will not change under me" guarantee is `copy` — after the
  deep copy Lale owns a private buffer that C cannot reach.
- Concurrent mutation is a separate data-race and memory-model concern, outside this design.
- `readonly` governs **safe code**. Reading the raw address field (`s.ptr`, `xs.ptr`) and
  writing through it with `unsafe value at` is outside the guarantee — the same way C's `const`
  can be cast away. `unsafe` is the explicit escape hatch, not a hole in `readonly`.

`readonly` is **not FFI-scoped**: it is a general capability modifier on a reference or a view
(§3.4), and it is not a `const` keyword — the same word forbids writes through a Lale reference.
What is specific to the FFI boundary is only the _caveat_ above: there, `readonly` cannot promise
that the data is stable, because the foreign side is outside the compiler's control
(`ARCHITECTURE.md §5.2`).

#### 5.4 Deferred: zero-copy + own

A third mode — wrap a `malloc`-compatible foreign buffer and **take ownership without copying**
(Rust's `Vec::from_raw_parts`) — is deferred to **2.0.0** as an `unsafe`, purely additive
operation (no ABI change, no breakage). It must be explicit, because the compiler cannot verify
that a foreign pointer really came from a compatible allocator.

Many real libraries need exactly this: FFTW (`fftw_free`), GSL (`gsl_vector_free`), libcurl
(`curl_free`), GLib/GTK (`g_free`), SDL (`SDL_FreeSurface`), and OpenCV (`cvReleaseMat`) all hand
back owned buffers paired with **their own** free function.

### 6. Rendering arrays and views

- **`write` and embedded values** use **plain** array formatting: `[1, 2, 3]`, with no type or
  unit annotations. An array and a view of it render identically.
- **`debug` and structured diagnostics** use JSON that states the element **type and unit once
  for the whole array**, not per element (`unit` is `""` when the array is unitless):

  ```text
  debug arr   // {"type": "i32", "unit": "", "values": [1, 2, 3]}
  ```

The once-only rule follows from the shape of the data: an array is **homogeneous**, so repeating
`type` and `unit` on every element would repeat one fact. A struct is **heterogeneous**, so its
existing per-field annotations (each field has its own type and unit) stay as they are. This is
the one intentional difference between the array and struct debug formats.

### 7. Performance

#### 7.1 The fat pointer

A dynamic array is represented as a 24-byte **fat pointer** header:

```text
array of T = { ptr: pointer, count: u64 in <elements>, capacity: u64 in <elements> }
```

`ptr` addresses the element buffer (`count × sizeof(T)` bytes), `count` is the live element
count, and `capacity` is the allocated element count. This mirrors the `text` type's header —
`{ ptr, bytes, chars }` — and is the same 24-byte size; `binary` is the 16-byte `{ ptr, bytes }`.
An array with `capacity == 0` has a **null** `ptr` and owns no buffer; any allocation has
`capacity >= 1` (§2.3).

The header fields are **read-only** from Lale code (§3.4). The address field is `ptr` for all three
fat pointers (`text.ptr`, `binary.ptr`, an array's `ptr`); the layout is unchanged.

An `array of T` **owns** its buffer, and the compiler manages that memory automatically: it is freed
at scope exit, exactly as for `text` and `binary` (`ARCHITECTURE.md §5.14b`). An array with
`capacity == 0` owns no buffer, so there is nothing to free. The user never writes `release` for an
array. This widens a deliberately narrow exception — the compiler-managed heap types — from `text`
and `binary` to arrays as well (section 9).

#### 7.2 By default, arrays live on the heap

The straightforward implementation puts every array's buffer on the heap: one allocation and one
free per array, and the elements sit in a separate block from the surrounding locals. A
register-held `pointer` still makes ordinary indexing a single load through a register base (not
a pointer chase), but the costs differ by access path — plain indexing, `ref` parameters, and
inline-field access each have their own profile, and should be measured separately rather than
assumed.

#### 7.3 Optimisation: the "count never changes" lowering

A **lowering** is a compiler transformation that rewrites a program into a cheaper or more
concrete form. The one described here keeps an array's elements in a fixed stack slot instead of
heap memory.

Lale already tracks, per variable, whether it is reassigned or shared (`is_reassigned` /
`is_shared`, `ARCHITECTURE.md §4.5.7a`; the columns are declared in §4.5.8). Adding a "count is
provably constant" fact lets the
compiler **keep the fat pointer but place the buffer on the stack instead of the heap**:
`var a as array of i32 = [1, 2, 3, 4, 5]` becomes a fixed-size slot in the **frame** (the per-call
area of stack memory) plus `{ ptr = address of that slot, count = 5, capacity = 5 }`.

A local is **never** moved into a global by this lowering: that would share one buffer across
calls and break recursion and reentrancy. A module-level array's buffer already lives in static
storage (§2.6); static storage is for globals, stack storage for locals.

This is **best-effort, an optimisation and never a semantic promise**: a constant count is no
guarantee of stack storage. It reuses the existing `is_reassigned` / `is_shared` tracking and
silently de-optimises when a `grow`, a reassignment, or a runtime-count initialiser appears.

**The lowering is restricted.** It is applied only when the compiler can establish that the
array's data pointer does **not** escape the frame. The escape routes are: passing `.ptr` to an
FFI call, exposing the array through an `inline` field, deriving a raw pointer from the array,
returning the array (which materialises a heap copy anyway), and storing a reference or view
beyond the frame (already a compile error, §3.4 / §8.5). If any of these is present — or the
compiler cannot decide — the buffer stays on the heap. Because the lowering is best-effort,
restricting it cannot change a program's meaning; it only means that fewer arrays receive the
optimisation. This settles the former "no escape analysis" assumption — the earlier idea that the
compiler would not need to check whether a data pointer can outlive its storage: the optimisation
now relies on a **narrow** escape check, not on the absence of one.

A buffer lowered into **static storage** is not heap memory and must never be freed by the heap
deallocator; if such an array later needs storage the fixed slot cannot provide, it transitions to
owned heap storage (§3.2).

#### 7.4 Genuinely dynamic arrays, and the cost of copying

Arrays that really do grow and shrink heap-allocate as a matter of course — normal for a resizable
array. Deep-copy value semantics can be expensive for large arrays: passing a million-element
array by value costs `O(n)`. The definition-site warning helps, but it is not a substitute for an
efficient **read-only** pass. That pass is the `readonly ref` parameter of §3.4, which is **Lale
1.0.0**; what remains deferred to 2.0.0 is only the _compiler-inferred_ read-only-ness that needs
no declaration (§10, `ARCHITECTURE.md §5.14d`).
Automatic allocation and freeing remove the bookkeeping, not the cost, so allocation-heavy
workloads should be benchmarked rather than assumed cheap.

## Part II — Ranged views (planned for a later version)

A **ranged view** is a separate feature, recommended to follow 1.0.0 (section 10). It is the
deferred part of non-owning access — the **sub-range** window (`view of T`, `xs[a to b]`) with
its own freeze and invalidation rules. Whole-range references (`ref of array of T`) are already
part of 1.0.0 (Part I).

### 8. Views

#### 8.1 What a view is

A **view** is a non-owning window onto a contiguous range of elements of an array (or of `text` /
`binary`). It is non-owning; it never copies and never owns, so it is never freed and never triggers a
deep copy.

The name **`view`** was chosen because it states the semantics directly; `slice`, the term used
by several other languages, means a _copy_ in Python and Ruby — the communities closest to
Lale's audience — so it would mislead exactly the readers Lale targets.

#### 8.2 Creating a view

A view of a Lale array is created with a ranged subscript `origin[first to last]`, with
inclusive, 1-based bounds and **no** `from`:

```lale
var temps as array of f64 = [273.15, 300.0, 310.5]
var mid as view of array of f64 = temps[2 to 3]   // elements 2..3, inclusive
write mid                                        // [300, 310.5]
mid[1] = 301.0                                   // writes through the view into temps[2]
```

- **Type:** written `view of T`, where `T` is the **source type** (for example `view of array of i32`,
  `view of text`) — the same convention as `ref of T` (§3.4) — mirroring the existing
  parameterised-type form `vec2 of T`. The source type is always explicit — no inference —
  consistent with Lale's no-implicit-conversion rule.
- **Unit:** a view inherits the type and unit of its origin.
- **Range validity:** the bounds must satisfy `1 ≤ first ≤ last ≤ the origin's count` —
  `origin.count` for an array, `origin.chars` for `text`, `origin.bytes` for `binary`. A
  **constant** violation — an inverted range like `xs[3 to 2]`, or a bound out of range like
  `xs[0 to 3]` or `xs[2 to 99]` — is a **compile-time error**; a **runtime** violation **aborts**
  with a clear message. (A tolerating variant that treats it as a no-op is a possible future
  extension.) An _array_ may be empty (§2.3), but a view cannot have an inverted range; a zero-count
  view, if wanted, needs a separate spelling.
- **`[a to b]` vs `[a]`:** `origin[a]` is the single element at index `a`; `origin[a to b]` is a
  view of the range `a…b`. Only the ranged form creates a view.
- **`text` / `binary`:** the same syntax applies. `myText[i]` returns a `char` — the i-th **code
  point** (no unit) — and `myBinary[i]` returns a `byte` — the i-th **byte** (no unit). A ranged
  subscript gives a non-owning view: `myText[3 to 5]` is a view of code points 3…5 and
  `myBinary[3 to 5]` a view of bytes 3…5 (a pointer to the origin plus `first` and `last`). Code
  points — not grapheme clusters — are Lale's unit, matching the `chars` field of `text`
  (`ARCHITECTURE.md §5.14a`, §5.14c); grapheme clustering is an explicit non-goal. (Code-point
  indexing scans the UTF-8 bytes from the start of the string, so `myText[i]` costs `O(i)`.)
  A `text` view stores **code-point indices** (`first`, `last`), not byte offsets: access scans
  UTF-8 from the origin's start, so `mid[i]` costs `O(first + i)`. This is the UTF-8-everywhere
  choice; a byte-offset text view is rejected. A `binary` view stores byte indices, but it is a
  view of a different type.

`view`, `of`, and `to` are **contextual**, not reserved words, consistent with Lale's "no
reserved words" rule.

#### 8.3 Representation, by form and origin

What a non-owning access stores depends on its **form** — a ranged `view` or a whole-range
`ref` — and, for a view, on whether the origin has a **Lale header** (a fat pointer that can be
followed after reallocation):

| Form                               | Origin                                    | Lale header? | Result                                                 |
| ---------------------------------- | ----------------------------------------- | ------------ | ------------------------------------------------------ |
| `xs[a to b]` (ranged view)         | a Lale dynamic array, `text`, or `binary` | yes          | a **ranged view** `{ header: pointer, first, last }`   |
| `ref to x` (whole-range reference) | a Lale dynamic array, `text`, or `binary` | yes          | a **reference**: a copy of the source header           |
| field read / `ref` FFI return      | inline bytes, or a foreign C buffer       | no           | a **reference**: a header copy with `capacity = count` |

**Observable semantics are identical** in all cases — the length field (`count`, or `chars` /
`bytes` for `text` / `binary`), indexing, iteration, non-owning, the escape rule. Only the
_representation_ differs, and it is dictated by the form and the origin:

- **A ranged view of an origin with a header** (a Lale array, `text`, or `binary`). The view stores
  a pointer to the origin's header plus the range bounds. The extra hop is deliberate: the header
  lives in the array variable's slot and does not move, so when the array grows and its buffer is
  reallocated, the view re-reads the header and keeps seeing the correct buffer. This is what makes
  a view **survive growth**.
- **A reference** (`ref to x`, an `inline` field read, or an FFI `ref` result). There is no header
  to follow in the inline / foreign case, and `ref to` deliberately chooses the direct form even
  when one exists, so the reference is a copy of the source type's header — `{ ptr, count,
capacity = count }` for an array, `{ ptr, bytes, chars }` for `text`, `{ ptr, bytes }` for
  `binary` — whose `ptr` addresses the buffer directly (§13.4). A reference to an owning array
  copies the origin's capacity (which may exceed `count`). It cannot survive reallocation, so its
  origin is frozen against growth, shrinkage, and reassignment (§3.4).

This distinction is also why reading an `inline` field yields a **reference** (`ref of array of T`,
§4.2) rather than a ranged view: there is no Lale header, so the natural form is "the whole
buffer, reference."

The header's **address must be stable** for growth to be survivable. The first bullet's guarantee
holds only while the header does not move: for a **top-level array variable** the header lives in
that variable's slot and never moves, so a view of it survives growth. For an **array-valued
element of another array** (`m[i]`, §2.7) the header lives _inside the outer array's buffer_. The
**root-freeze rule** settles this: taking a reference or view of `m[i]` marks the root `m` as
`taken`, so `m` cannot grow, shrink, or be reassigned while the access lives (§8.5), and
(conservatively) no row of `m` may be resized. The header therefore cannot move, and the view
stays valid. The cost is that a view of `m[i]` freezes the whole outer array, not just row `i`.

**References and views are distinct.** A `ref of T` holds a **direct pointer to the data** (a copy
of the source header): it is the cheapest non-owning form, and it freezes the origin's growth,
shrinkage, and reassignment so the pointer cannot dangle. A `view of T` holds a **pointer to the
origin's header** — a pointer to a pointer, since the header's first field is `ptr` — so it follows
the buffer across reallocation and **survives growth**; it freezes only `shrink` and reassignment.
The extra indirection is the price of growth-safety. They are two types, not one type with two
names.

#### 8.4 Passing a view

Passing a view **by value** copies the **descriptor** — the pointer to the origin's header plus
the bounds — not the elements. The cost is `O(1)`, the callee still views the caller's storage,
and writes through the view are visible to the caller. A `readonly view of T` is the same `O(1)`
copy but forbids writes through the holder. `ref xs as view of T` is a **compile error**: a view is
already a non-owning handle, so `ref` would add nothing and would hide the aliasing.

```lale
var data as array of i32 = [10, 20, 30, 40, 50]
var mid as view of array of i32 = data[2 to 4]      // view of elements 2..4

// by value: `xs` is a copy of the descriptor; it still views data[2..4]
fn sum_view(xs as view of array of i32) returns i32
    var total as i32 = 0
    loop var i as u32 from 1 to xs.count
        total = total + xs[i]
    end loop
    return total
end fn

// by value: writes through the view are visible to the caller
fn bump(xs as view of array of i32) returns nothing
    loop var i as u32 from 1 to xs.count
        xs[i] = xs[i] + 1                  // writes into `data`
    end loop
end fn

// read-only: the same O(1) descriptor copy, no writes through the holder
fn sum_readonly(xs as readonly view of array of i32) returns i32
    ...
end fn
```

| Parameter / type           | What the callee gets                                   | Cost | Mutates the caller's data? |
| -------------------------- | ------------------------------------------------------ | ---- | -------------------------- |
| `xs as view of T`          | a copy of the view descriptor                          | O(1) | yes, through the view      |
| `xs as readonly view of T` | a copy of the descriptor; no writes through the holder | O(1) | no (safe code)             |
| `ref xs as view of T`      | **compile error**                                      | —    | —                          |

There is no `copy` form of a view parameter: copying the descriptor already happens for a plain
by-value view parameter, so `copy` would add nothing. A view is also **not** implicitly converted
to an owning `array of T` — passing one to an `array of T` parameter is a compile error. Obtaining
an owned copy therefore needs an explicit element-wise materialisation, and the `copy` keyword is a
**parameter modifier** (§3.1), not a call-site operator, so this proposal leaves the spelling of
that materialisation to the view layer. A `readonly view` is the cheap read-only form; it ships
with views (section 10).

#### 8.5 The escape rule, and lifetime across resize

A view refers to the origin's storage. If the origin is a **local** array, that storage is a
stack slot, and the view dangles once the function returns. The compiler forbids this, extending
the existing raw-pointer escape rule (`ARCHITECTURE.md §5.14`):

> A view that references a _local_ array — directly, or transitively through another view — must
> not be **returned** from the function or **stored in a global**.

Detection is **per-function and syntactic**: a view of a local array is marked "references a
local"; a view of an already-marked view inherits the mark; and returning or globally storing a
marked view is a compile error.

```lale
// ❌ compile error: the view outlives its local origin
fn bad() returns view of array of i32
    var local as array of i32 = [1, 2, 3]
    return local[1 to 3]           // local's storage is a stack slot — dangles on return
end fn

// ✅ the origin outlives the view
var g as array of i32 = [1, 2, 3]   // module-level array

fn ok() returns view of array of i32
    return g[1 to 2]                // fine: g outlives the function
end fn
```

This rule is a **boundary of the guarantee**, not a general proof of view safety. In particular:

- A view of a **global** array is fine — the global outlives the function.
- A view of a **by-value array parameter** refers to a local copy, so it must not escape.
- A view of a **`ref` array parameter** refers to the caller's storage; the compiler does **not**
  verify the reference across the call boundary — the same caveat the raw-pointer rule already
  accepts.
- A by-value **view parameter** also refers to the caller's storage. Like a view of a `ref`
  parameter, its origin is not verified across the call boundary: the callee must not return or
  store it unless the caller guarantees that the origin outlives the returned view.
- Passing a view _into_ a callee is allowed (the rule is per-function).

**A non-owning access freezes its origin.** For a ranged view, growth is safe (§8.3). Two
operations are not: `shrink` (if it drops below the view's `last`, the view points past the new
end) and reassignment (the header slot is overwritten in place, so the view silently re-points).
The rule is tracked per array variable — and, for the element-level freeze, per element or field
that owns a buffer — whether a non-owning access has been taken of it: a `taken` mark alongside
`is_reassigned` / `is_shared`. If set, `shrink` and reassignment of that array are **compile
errors** for a view; for a **reference**, `grow`, `shrink`, and reassignment are all compile
errors, because a reference holds a header copy (§3.4). The freeze is **structural** — element
writes through the view, the reference, or the array itself remain allowed, except that
reassigning an element or field that owns a buffer (a nested array, `text`, `binary`) is frozen
while a reference to it lives (§11.2). The reference half (arrays, `text`, `binary`) is **Lale
1.0.0**; the ranged-view half ships with this layer. The
rule is per-function and deliberately conservative (it also rejects the case where the non-owning
access is already dead); relaxing it later with liveness analysis is additive. The **global case is
open**: a view of a global can escape, so view-taking on a global is cross-function — either forbid
shrinking a global that has had a view taken anywhere, or restrict views to non-global origins.

The freeze applies to the **root** array, not only to the sub-array named in the access. Taking a
reference or view of any part of a multidimensional array — `m[i]`, or an `inline` field — marks
the root (`m`) as `taken`: growing, shrinking, or reassigning `m`, and (conservatively) resizing
any row of `m`, while the access lives is a compile error, because growth can move the element
header the access depends on. Element writes that do not replace the referenced array-valued
element remain allowed; replacing it (`m[1] = row`) is a **compile error** under the element-level
freeze (§11.2).

**Across a call (`ref`).** A `ref` array parameter cannot resize its array (§3.1), cannot resize an
array-valued element reached through it, cannot replace an element or field that owns its own
buffer (`grow m[1]`, `m[1] = row`, `xs[i] = "new"` through a `ref` parameter are compile errors),
and cannot rebind it (§11.2). It can only mutate non-replacing elements, which cannot invalidate a
view. So the per-function rule above extends across a call without extra analysis: a callee cannot
dangle the caller's view or reference.

#### 8.6 A view of a view; read-only views

`view of view of T` is **rejected in Lale 1.0.0** — a pure restriction, so allowing it later is
additive and non-breaking. As with the nested array spelling (§2.12), the rejection is made in
semantic analysis rather than in the grammar, so the diagnostic can name the fix: the element of
`view of` may not itself be a `view`.

There is likewise no way to get that promise **without a declaration**: a plain `view` (or a
writable reference) is writable. A **`readonly`** reference (1.0.0) or `readonly` view (with the
view layer) supplies it — it forbids writes through the holder while staying non-owning. What
remains deferred to 2.0.0 is merely _inferring_ read-only-ness without the declaration
(`ARCHITECTURE.md §5.14d`).

## Part III — Principles, versions, and follow-ups

### 9. Fit with Lale's design principles

- **Language over Library.** Bounds-checked, deep-copying arrays with a read-only `.count` require compiler
  awareness — a textbook fit. The resizing syntax (`grow` / `shrink` / `by`, §2.8) matches the
  "reads like the problem domain" criterion.
- **1-based indexing** is respected throughout.
- **Raw pointers only** (`ARCHITECTURE.md §5.14`): the header's `ptr` field is a raw pointer;
  no typed pointers are introduced.
- **No implicit conversions** (`ARCHITECTURE.md §5.5`): `array of i32` being one type whatever the
  count makes "no coercion" literally true, and `inline` field access is materialisation, not a
  conversion.
- **A type contains no brackets** (§2.9): `[]` keeps only its value roles — indexing (`xs[i]`) and
  array literals (`[1, 2, 3]`) — so a reader tells a type from a value at a glance.
- **No type aliases** (`ARCHITECTURE.md §5.3`): `inline` is a field-only modifier, not an alias.
- **Complete initialisation** (`ARCHITECTURE.md §5.10`): the `grow`-initialiser count rule (§2.8)
  is consistent with it, and a declaration cannot mismatch its count at all, because the initialiser
  supplies it (§2.2).

Four deliberate, documented expansions of existing rules:

- **A declared `readonly` capability.** `readonly ref` / `readonly view` _declare_ that the holder
  may not write through the reference, whereas mutability is normally inferred from SSA — **Single
  Static Assignment**, a program form in which each variable is assigned once, so the compiler can
  see exactly what changes — not declared (`ARCHITECTURE.md §5.2`). Declaring it is justified
  because a **reference type's capability** is a property of the _handle_ — there is no single
  function body to infer it from — not a variable whose SSA the compiler can analyse. This is
  recorded as decided in §11.2.
- **Compiler-managed arrays.** Arrays extend the "compiler manages the heap" exception beyond
  `text` / `binary` (`ARCHITECTURE.md §5.14b`). Requiring `release` for every array would leak
  implementation details into user code, so the exception is widened deliberately.
- **Observable capacity, with invariants.** `xs.capacity` is a read-only field with the guarantees
  `capacity >= count`, `grow` never decreases it, and `shrink` never increases it. The exact
  capacity and growth factor remain unspecified. This deliberately makes an allocation detail
  observable, so programs can reason about spare room without constraining the implementation.
- **New contextual keywords, and one new token shape.** The design adds `array`, `inline`
  (field-only), `ref` / `ref of` / `ref to`, `readonly`, `view` / `of` / `to`, `elements` / `by`,
  `grow` / `shrink`, and `C` (after `export`) — all **contextual**, per Lale's no-reserved-words
  rule, so none of them becomes unusable as an identifier. (`may fail` would be a further
  contextual form, but it is deferred with the failure-recovery design, Annex B.) The rank prefix
  (`2D`, `3D`, …) is a new token shape: the first digit-led token that is not a number (§2.12).

### 10. Version plan

| Feature                                                                                   | Planned for              | Notes                                                                           |
| ----------------------------------------------------------------------------------------- | ------------------------ | ------------------------------------------------------------------------------- |
| Dynamic array type `array of T` (runtime count)                                           | Lale 1.0.0               | one array type for every count                                                  |
| Array type notation `array of T`; rank prefix `2D` / `3D`                                 | Lale 1.0.0               | `[]` no longer spells a type (§2.9, §2.12)                                      |
| Complete initialisation, bounds checking                                                  | Lale 1.0.0               | core correctness guarantees                                                     |
| Deep-copy value semantics; `ref` / `copy`                                                 | Lale 1.0.0               | ownership rules per §3.2                                                        |
| Compiler-managed array memory (auto-free)                                                 | Lale 1.0.0               | as `text` / `binary`                                                            |
| `grow` / `shrink` / `by` resizing                                                         | Lale 1.0.0               | arrays are dynamic; resizing is intrinsic                                       |
| `inline array of T of n elements` fixed struct fields                                     | Lale 1.0.0               | C-compatible layout                                                             |
| References (`ref of T`: arrays, `text`, `binary`; `inline` fields, FFI `ref`, `ref to x`) | Lale 1.0.0               | whole-range, non-owning; the 1.0.0 half of non-owning access                    |
| `readonly` references                                                                     | Lale 1.0.0               | declared capability modifier (§3.4); replaces `const ref`                       |
| `readonly` views                                                                          | with ranged views        | the same modifier on `view of T`                                                |
| FFI ownership `ref` / `copy` / `readonly ref`                                             | Lale 1.0.0               | C → Lale direction                                                              |
| Array rendering (§6)                                                                      | Lale 1.0.0               | plain `write`, JSON `debug`                                                     |
| View rendering (§6)                                                                       | with ranged views        | the same rules applied to a view                                                |
| "Count never changes" stack lowering                                                      | Lale 1.0.0 (best-effort) | optimisation only; never a semantic promise                                     |
| Ranged **views** (`view of T`, `xs[a to b]`)                                              | **later version**        | sub-range windows; the deferred part of non-owning access                       |
| Freeze rule for references                                                                | Lale 1.0.0               | `grow` / `shrink` / reassignment of an array; reassignment of `text` / `binary` |
| Freeze rule for ranged views                                                              | with ranged views        | `shrink` / reassignment only; a view survives growth                            |
| Recoverable allocation failure (`may fail`)                                               | later version            | 1.0.0 is abort-only on failure; adding it later is additive                     |
| `inline array of T of len elements` (flexible field)                                      | later version            | needs variable-sized-struct semantics                                           |
| `view of view of T`                                                                       | later version            | additive, non-breaking                                                          |
| Compiler-inferred read-only references (`ARCHITECTURE.md §5.14d`)                         | Lale 2.0.0               | inference, not declaration                                                      |
| Zero-copy + own at the FFI boundary                                                       | Lale 2.0.0               | `unsafe`, additive                                                              |
| `usize` / `isize` (pointer-sized integers)                                                | Lale 2.0.0               | 1.0.0 FFI uses `u64`/`i64` (roadmap §1)                                         |
| Built-in matrices                                                                         | independent roadmap item | out of scope here; nested arrays stay jagged                                    |
| Generics                                                                                  | separate concern         | needed for "any element type", independent of this design                       |

The layering is deliberate: 1.0.0 ships **owned arrays plus a narrow whole-range reference**, and
defers **ranged views**, **recoverable allocation failure**, and **advanced systems interop**
(flexible fields, foreign-buffer adoption, extra ABI mappings) — so 1.0.0 does not depend on range
invalidation, global view tracking, liveness, failure recovery, and variable-sized-struct layout
all at once.

### 11. Open questions and work items

This section is split by **kind**, so that what blocks approval is visible at a glance: decisions
still to make (§11.1), decisions already taken (§11.2), work items (§11.3), and nits (§11.4).

#### 11.1 Decisions still to make

**Blocking Lale 1.0.0:** three decisions remain. (`may fail` is deferred, so 1.0.0 is abort-only
and the attribution question does not arise; Annex B.)

- **References to globals that escape.** A reference to a global array (or a global `text` /
  `binary`) may currently escape (§3.4), and a later `grow` / `shrink` / reassignment in another
  function can invalidate it. Options: (a) **forbid** references to globals from escaping and from
  being stored globally; (b) **track** outstanding references to global origins across functions
  and reject invalidating operations while they exist. (a) is the simpler, spec-complete rule.
- **Static-storage returns and global element payloads.** Returning a static-backed global array is
  now specified as a heap copy that never moves the binding (§3.2), but §2.6's "no runtime
  allocation at module load" still conflicts with a global array whose elements own heap payloads
  (nested arrays, `text`, `binary`, or structs owning them). Options: (a) **restrict** module-level
  array element types to cleanup-free, statically representable types, so the whole value is static;
  (b) **admit** a specified startup allocation for element payloads and drop the "no runtime
  allocation" guarantee.
- **Reference provenance across calls.** A function that receives a `ref of T` parameter and
  returns it cannot tell from the type whether the origin outlives the call. Options: (a) **reject**
  returning a `ref` parameter (and any reference whose origin is not locally proven to be a global);
  (b) make the provenance **explicit** in the signature; (c) place responsibility on the **caller**
  and document the hole.

**Recommendation:** 1(a), 2(a), 3(a) — the three smallest, most enforceable rules.

**For the ranged-view layer and later versions:**

- **Views of `text` / `binary`, and global origins.** Reassignment and storage invalidation of
  `text` / `binary` views lack rules comparable to the array rules (§8.3); and a view of a global
  can escape, so view-taking on a global is cross-function.
- **A ranged view of a headerless origin.** A view of an `inline` field (or another origin with no
  Lale header) needs a descriptor that combines a materialised header with range bounds. The form
  is not specified; the view layer must either define it or reject it. `ref to` a temporary
  expression is likewise unspecified.
- **A ranged view of a `ref` value.** A `ref of T` holds a **copy** of the origin's header, so a view
  of the `ref` would have to point at that copy. The copy's address is stable only while the `ref`
  variable lives, and reassigning the `ref` would silently re-point the view. Either forbid views of
  `ref` values in 1.0.0 or define the rule that the `ref` is frozen while the view lives.
- **Generalising `ref of T` beyond the count-carrying types.** Allowing `ref of T` for a scalar
  (`ref of i32`) or a struct (`ref of Sample`) later would be **additive, not breaking**, but it
  would turn `ref` into a general **typed reference** — effectively the typed pointers that §5.14
  ("Raw Pointers Only") excludes — and would generalise the reference-variable lifetime bookkeeping
  to every type. **Treat it as its own design.**
- **Resizing through a `ref` parameter — deferred.** A `ref grow` modifier (or a resizing effect in
  the signature) could add it later with the capability explicit at the call site. 1.0.0 does not
  have it (§3.1).

#### 11.2 Resolved — for the record

- **Reference variables — option (c): allowed, with per-function restrictions.** A reference may be
  held in a variable, assigned between references, passed, and returned subject to the escape rule
  (§3.4). Because Lale has only two scopes, a reference to a local cannot dangle _within_ the
  function through scoping. The invalidating events are early frees — the origin is returned (the
  local becomes moved-from and its slot is reclaimed), reassigned, or resized — and the freeze rule
  makes them compile errors while the reference is live. Both sub-points are settled: a reference
  **may** be passed to an **owning** parameter, which materialises a copy (§4.2's `sum(s.values)`
  is the precedent); and the freeze keys on **"non-owning access was taken"**, not on liveness.
- **A non-owning access freezes its origin.** While a reference to an array is live, `grow`,
  `shrink`, and whole-array reassignment of that array are **compile errors**; if the reference
  names an element or field that owns a buffer, reassigning that element or field is also a
  compile error (element-level freeze, below). A ranged view freezes `shrink` and reassignment but
  **survives growth** (§3.4, §8.5). The reference half is 1.0.0.
- **A `ref` parameter — element mutation only.** It grants neither whole-variable rebinding nor
  `grow` / `shrink`, uniformly across origins (§3.1). It also does not permit resizing an
  array-valued element reached through it, or replacing an element or field that owns its own
  buffer (`grow m[1]`, `m[1] = row`, `xs[i] = "new"` through a `ref` parameter are compile errors).
  These exclusions are what make the rule independent of the call site: a callee cannot replace the
  caller's array or invalidate a reference the caller holds.
- **`ref` as a modifier vs. `ref of T` as a type — distinguishable.** The type is `ref of T` (§3.4);
  the parameter modifier stays `ref xs as T` (§3.1). They differ at the second token (`of` vs an
  identifier), so a PEG (a Parsing Expression Grammar — the notation Lale's grammar is written in)
  orders them unambiguously. The only pathological collision is a parameter literally named `of`
  (`ref of as T`), which the grammar rejects at that point.
- **Runtime initialiser-count mismatch — a failure.** `grow xs by n = f()` with a runtime `n` and a
  runtime-count initialiser fails (Annex B: fatal) whenever the initialiser's count differs from
  `n` (§2.8).
- **Binding an rvalue array — a move (option 3).** `var a as array of i32 = f()` moves the call's
  result into `a`; binding an lvalue (`var a = b`, `var a = m[1]`) deep-copies. There is no copy
  elision to permit: the language specifies one allocation for an rvalue binding, so abort
  behaviour is deterministic. This matches the move already performed on `return` (§3.2).
- **Moved-from detection — a linear check.** Reading a moved-from variable is a **compile error**;
  cleanup must not free it (§3.2). The check is small because `return a` is the only move whose
  source is a named variable (an rvalue binding moves a temporary); conditional moves are covered
  by the same definite-assignment-style pass.
- **`unsafe decl` — cleanup-free element types only in 1.0.0.** Recursive destruction of an
  uninitialised element could follow a garbage pointer, so `unsafe decl` is restricted to element
  types that need no cleanup — including a struct or optional that contains a cleanup-needing type
  (§2.2).
- **`inline` element types — the C-compatible set.** The fixed-width integers and floats, `byte`,
  `bool`, `pointer`, or a `type` whose layout is verified C-compatible. Excluded: `char` (4 bytes,
  a Unicode code point, not C's 1-byte `char`), `f16` (no portable C type in 1.0.0), `text` /
  `binary`, `T?`, vectors, and dynamic arrays (§4.1). Adding an excluded type later is additive.
- **Empty arrays across the FFI — `(NULL, 0)`, and the capacity rule.** An array with
  `capacity == 0` has a **null** pointer; the compiler never calls `malloc(0)`, and any allocation
  has `capacity >= 1`. Such an array passes `(NULL, 0)` to C. Lale does **not** promise a non-null
  dummy: a C wrapper that requires a non-null pointer must guard on `count == 0` (or the program may
  pass a static dummy). A C `(NULL, 0)` result becomes a valid empty Lale array (§13.5).
- **`copy` on an FFI return — only when the foreign side retains the original.** A transferred
  allocation must be declared `ref` and released with the library's own function; `copy` is for a
  getter that keeps ownership, because otherwise the program would hold only the copy and no pointer
  with which to release the original (§5.2). The same rule applies to the text / binary returns
  `copy text of n chars` and `copy binary of n bytes`.
- **Resizing a reference — always a compile error.** `grow` and `shrink` require an owning binding
  (§3.1); a reference is never owning, so the attempt is rejected statically regardless of the
  reference's origin (§4.3).
- **Nested-array mutation target — normative.** An indexed expression used as an assignment or
  mutation target designates the **stored** element, never a temporary copy from an ordinary read;
  a plain read still yields a value copy (§2.8).
- **Array equality — deep element-wise `==` / `!=`.** `a == b` compares the counts, then the
  elements pairwise, at `O(n)` — consistent with the `O(n)` deep copy of assignment. Elements use
  their own `==` (§3.1).
- **Cross-call invalidation — closed by construction.** Because a `ref` parameter cannot resize
  or rebind its array, and cannot resize an array-valued element reached through it (§3.1, §11.2),
  a callee cannot invalidate the caller's reference or view. Element mutation is visible but safe.
  No cross-call analysis is needed (§8.5).
- **The family spelling of the address field — `ptr`.** `text.ptr`, `binary.ptr`, and an array's
  `ptr` use the same field name; the layout is unchanged. `pointer` remains the type name and the
  `.ptr` field is read-only.
- **`count of` removed in favor of read-only fields.** An array exposes `xs.count` and `xs.capacity`
  as read-only fields carrying `<elements>`; `text` / `binary` use `.chars` / `.bytes`. `count of`
  and `#count of` are removed. Capacity has the invariants `capacity >= count`, `grow` never
  decreases it, and `shrink` never increases it; the exact value and growth factor remain
  unspecified. A view follows the header for `.ptr`; its `.count` (or `.chars` for a `text` view)
  is `last - first + 1`, and a `text` view's `.bytes` is the byte length of that code-point range.
  It has no `.capacity`.
- **Text / binary references — freeze and escape in 1.0.0.** A reference to a local `text` or
  `binary` follows the same escape rule as an array, and reassigning the origin while the reference
  lives is a compile error (§3.4, §8.5).
- **Views of array-valued elements — root freeze.** Taking a reference or view of any part of a
  multidimensional array (`m[i]`, an `inline` field) marks the root array as `taken`; the root
  cannot grow, shrink, or be reassigned while the access lives (§8.3, §8.5). This replaces the
  earlier "reject the view when the outer can reallocate" proposal.
- **A reference to an element that owns a buffer — element-level freeze (option 1).** A reference
  such as `ref to xs[i]` — where the element is `text` / `binary` / a nested array — or
  `ref to s.name` for such a field freezes **reassignment of that element or field** while the
  reference lives: reassigning it (`xs[i] = "new"`, `m[1] = row`, `s.name = "new"`) is a compile
  error. The `taken` mark is therefore tracked per element / field as well as per root array; the
  root-freeze rule above still covers the header-moving case. Restricting `ref to` to whole
  variables was rejected because it would lose useful element references; accepting the hole was
  rejected because it is a dangling hazard.
- **Stack lowering — restricted, not assumed safe.** The "count never changes" lowering (§7.3) is
  applied only when the compiler can establish that the array's data pointer does not escape the
  frame; otherwise the buffer stays on the heap. It remains best-effort and is never a semantic
  promise, so restricting it cannot change a program's meaning.
- **References and views are distinct — decided.** `ref of T` is a direct pointer to the data and
  freezes growth, shrinkage, and reassignment; `view of T` is a pointer to the origin's header,
  follows reallocation, and freezes only shrinkage and reassignment. They are not unified into one
  type (§3.4, §8.3).
- **View parameter passing — descriptor copy.** A by-value view copies the descriptor, not the
  elements (`O(1)`); writes through it are visible to the caller. `readonly view` forbids them, and
  `ref xs as view` is a compile error. There is no `copy` form of a view parameter; a view is not
  implicitly converted to an owning array, and the spelling for materialising the viewed elements
  is deferred with the view layer (§8.4, Annex A).
- **A declared `readonly` capability — accepted for 1.0.0.** `readonly ref` (and, with the view
  layer, `readonly view`) is Lale's single declared capability modifier. `ARCHITECTURE.md §5.2`'s
  "infer, don't declare" rule concerns a **variable's** mutability, which SSA can see; a
  reference's capability is a property of the _handle_, with no single function body to infer it
  from. The §5.2 carve-out is recorded in §13.1. Scoping the modifier to the FFI boundary (the
  earlier `const ref` position) is rejected as an asymmetric spelling.
- **`may fail` — deferred.** 1.0.0 is **abort-only** on array failure: a negative runtime amount, a
  `shrink` larger than the count, a runtime initialiser-count mismatch, or an allocation failure
  aborts with a clear message. The opt-in recovery annotation (`may fail`) and its error-stack
  integration are deferred to a later version (Annex B, §10); adding it later is additive.
- **Number of elements — `count`.** The array header's read-only `count` field is the spelling;
  diagnostics say `count`; `size of` loses its `#` (§2.13, §13.14).

#### 11.3 Work items

- **A normative owned / reference operation matrix.** §3.2's table is owning-only. Extend it with a
  reference column covering element mutation, growth and shrinkage, assignment between references,
  whole-variable rebinding, passing, returning, and destruction. A deliverable, not a decision.
- **The IR spelling of the dynamic array** (`i32[]` vs `dyn<i32>` vs a named fat-pointer struct) —
  an implementation choice, tracked with the other gaps in §12. (The **IR**, or Intermediate
  Representation, is the compiler's own internal form of a program, between source text and the
  code the machine runs.)
- **The compiler and documentation work** listed in §13.

#### 11.4 Nits

- **`grow` plus `fill` states the amount twice.** `grow xs by 3 = [fill 3 elements with 0]` names
  the amount in two places, and the two must agree (§2.8). The redundancy is forced by the rule
  that a `fill` always states its own count (§2.11). Recorded; no change proposed.

### 12. Implementation gaps

- **Struct layout.** `inline array of T of n elements` claims byte-for-byte C compatibility, but the
  interpreter does not yet model struct field offsets and sizes: `MemoryManager::compute_type_size` hardcodes
  structs to 64 bytes and `IrType::size_bytes()` returns `None` for a struct. This is the largest
  implementation gap for the inline feature.
- **IR / ABI rewrite.** The dynamic array replaces the `[N x T]` inline representation in the IR
  type enum, the **serialisation** format (the stored form in which the IR is written out and read
  back), the interpreter's memory manager, and the ABI spec. This is substantial work, not a tweak.
- **FFI boundary checks.** The boundary is explicit (§13.5): the compiler type-checks the passed
  pointer and count, and follows `lale.md`'s unit rule — a mismatch against a **declared** unit is
  an error, while a unit-carrying argument passed to a **unitless** imported parameter produces a
  warning (the foreign side cannot declare or check a unit). No automatic array-to-parameter
  expansion is performed. Needs platform tests.
- **Array-type predicates.** The compiler recognizes arrays with `contains('[')` /
  `starts_with('[')` in at least ten places (the `count` field's validity and unit, `compute_type_size`,
  type compatibility, `get_type_category`, binary operand checks). The new spelling has no
  brackets, so these must be replaced by a structural `is_array_type` check; otherwise `array of T`
  is silently treated as a non-array.
- **Read-only fat-pointer fields.** `text`'s `ptr` / `bytes` / `chars` are assignable today
  (`s.ptr = p` is accepted; the fields are not `private`). The read-only rule (§3.4, §7.1) needs a
  semantic-analysis check for all three fat pointers.
- **Element-level freeze.** Semantic analysis must track a `taken` mark per element or field that
  owns a buffer (a nested array, `text`, `binary`) and reject reassignment while a reference to it
  lives (§11.2). The root-freeze mark covers the header-moving case; the element mark covers the
  buffer-replacement case.
- **FFI unit warning.** Unit checking currently runs only for parameters that declare a unit. An
  imported parameter that declares **no** unit therefore accepts a unit-carrying argument silently;
  add the warning described in §13.5. A mismatch against an imported parameter that **does**
  declare a unit is an error, per `lale.md`.
- **Views and their rules.** Deferred with the ranged-view layer (§8) — the view type, the escape
  rule, and the freeze tracking.
- **Recoverable allocation failure (`may fail`).** Deferred (Annex B, §10); 1.0.0 aborts on
  failure, so there is nothing to implement here for this proposal.
- **The `C` modifier** — the export grammar, and making an `export C fn` symbol a plain C name.

### 13. Planned specification changes (apply on approval)

None of the following is applied until the design is approved.

#### 13.1 `ARCHITECTURE.md §5.2` — add a `#### Declared capabilities` subsection

After the existing `#### Rationale` paragraph, insert:

```markdown
#### Declared capabilities

The "infer, don't declare" rule concerns the mutability of **variables**, which
SSA makes inferable. A **reference or view whose capability is read-only** is a
different axis: its read-only-ness is a property of the _handle_, recorded in
the type, and there is no function body from which to infer it — an FFI
signature, a struct field, a stored reference, or a separately compiled
function's parameter. Lale therefore admits a single declared capability
modifier, `readonly`, on a `ref of T` / `view of T` type.

`readonly` denies _writes through the holder_; it is not an immutability
guarantee. The referenced data may still change through the owner, another
(writable) reference, or — at a C FFI boundary — the foreign side, which the
compiler cannot constrain. It must not be treated as licence to optimise
assuming the buffer is stable (only a deep copy gives that guarantee).
```

#### 13.2 `ARCHITECTURE.md §5.14d` — amend the read-only-reference note

Rename the subsection (currently `#### Deferred to 2.0.0 — read-only reference`) to
`#### Read-only references`, then replace its body. Replace:

```markdown
Rather than add a `const ref` keyword (rejected in §5.2: constness is inferred
from SSA, not declared), a future read-only reference is **compiler-inferred**:
a `ref` parameter that is never written is treated as read-only, for
optimization and possibly a lint.
```

with:

```markdown
A read-only reference is **declared** with the `readonly` capability modifier
(§5.2's "Declared capabilities"), because a reference's capability is a property
of the handle and cannot be inferred. Independently, the compiler may still
**infer** that a `ref` parameter is never written and treat it as read-only for
optimization and possibly a lint — an optimization, not a language guarantee,
and one that does not need the `readonly` spelling.
```

#### 13.3 `ABI_SPECIFICATION.md §2` — type layout

Replace the array row with two rows:

| Type                                           | Size / alignment                                                           |
| ---------------------------------------------- | -------------------------------------------------------------------------- |
| `array of T` (dynamic array)                   | 24 bytes — fat pointer `{ ptr: 8, count: 8, capacity: 8 }`, 8-byte aligned |
| `inline array of T of n elements` (field-only) | `n * sizeof(T)` contiguous bytes, alignment of `T`                         |

#### 13.4 `ABI_SPECIFICATION.md` — new `§3.2`, the dynamic array

The canonical language-level layout is §7.1; this section is the ABI target and must not drift
from it.

```text
array of T = { ptr: pointer, count: u64, capacity: u64 }
```

- Layout: `ptr` at offset 0, `count` at offset 8, `capacity` at offset 16 (24 bytes, the same
  size as `text`). In the Lale type definition, `count` and `capacity` carry `<elements>`; the unit
  does not affect the layout.
- `ptr` addresses a `count`-element buffer (`count × sizeof(T)`); `capacity` is the allocated
  element count.
- An `array of T` **owns** its buffer (compiler-managed, freed at scope exit).
- A `ref of array of T` to a headerless origin (an `inline` field or an FFI `ref` result) is
  non-owning, with `capacity = count`. That `capacity = count` is a conservative **descriptor
  value**, not a claim about the foreign allocator's true allocation; a reference cannot resize,
  and its `capacity` field does not grant that capability.
- A `ref to` an owning array copies the origin's header, so its `capacity` is the origin's
  capacity (which may exceed `count`); the reference still cannot grow.
- A `ref of text` / `ref of binary` reuses the `text` / `binary` header of §13.3 unchanged —
  `{ ptr, bytes, chars }` / `{ ptr, bytes }`; there is no array-style `capacity` field.
- An `inline array of T of n elements` field has **no** header: its elements are inline bytes, and
  field access yields a reference.
- The header carries `capacity` as a **read-only** field with the invariants `capacity >= count`,
  `grow` never decreases it, and `shrink` never increases it; exact values and the growth factor
  remain unspecified (§2.3, §11.2). Independently compiled Lale modules must agree on a fixed
  layout, so the field stays in the ABI.

#### 13.5 `ABI_SPECIFICATION.md §4` — FFI boundary (two layers)

The boundary is specified in two layers, not as an automatic conversion:

1. **Lale's internal representation** — the fat pointer (§13.4).
2. **The declared foreign signature** — the actual C parameters and return type, as written in
   the `import fn signature` / `export C fn` declaration.

There is no third "marshalling" layer — no automatic machinery that converts a Lale value into a
different foreign form — that expands one Lale argument into several C parameters. The programmer
passes each foreign parameter explicitly:

```lale
import fn signature c_sum(data as pointer, count as u64) returns i32

var xs as array of i32 = [10, 20, 30]
write c_sum(xs.ptr, xs.count)
```

The ordinary argument-count and type rules then apply; a call that tries to pass the array in
place of both declared parameters is an arity error, not a hidden expansion. `xs.ptr` is the
address of the element buffer and `xs.count` is its element count. This is the same idiom as
`s.ptr, s.bytes` for `text` and `b.ptr, b.bytes` for `binary`.

**Units at the boundary.** `xs.count` carries `<elements>`. Unit checking follows `lale.md`: a
mismatch against a **declared** unit is an **error**, whether the function is defined in Lale or
imported. A C count parameter normally declares no unit, so there is no declared unit to mismatch;
in that case the compiler emits a **warning** that a unit-carrying value is passed to a unitless
foreign parameter, because the foreign side cannot check it. The same warning applies to
`text.bytes` (`<bytes>`) and `binary.bytes` when passed to unitless C parameters. If the binding
author wants a checked contract, they write the unit in the import signature (`count as u64 in
<elements>`), and a mismatch then becomes an error like any other declared-unit mismatch.

The reverse direction is not a C return type at all: `returns ref of array of i32 of n elements` is
an **FFI descriptor** (reference + count binding). The same shape serves the other count-carrying
types: `returns ref of binary of n bytes`, `returns copy binary of n bytes`,
`returns ref of text of n chars`, and `returns copy text of n chars` (§5.2).

A `type` with `inline array of T of n elements` fields has a C-compatible layout (inline bytes) and
needs no conversion on either side — but "C-compatible" means its layout, field types, and calling
convention must all match the foreign declaration; it is not automatically safe across every
boundary. These conversions should not be described as universal ABI compatibility until the
mappings are defined and **tested for the target ABI**.

**The supported 1.0.0 baseline (concrete).** To keep the boundary predictable, 1.0.0 supports:

- a declared **pointer parameter** ← the array's `ptr`;
- a separate declared **count parameter** ← the array's count (`xs.count`), using the exact
  integer type written in the foreign signature (in 1.0.0, `u64` — C's `size_t` on 64-bit hosts);
- a **zero-count array** ← its pointer (null when `capacity == 0`) and count `0`; the C side must
  guard on `count == 0`, because Lale does not promise a non-null pointer (§11.2);
- a **struct with `inline array of T of n elements` fields** ← its inline bytes, only when the
  layout, field types, and calling convention match the declared C type;
- an **array return** ← only through an explicit FFI descriptor
  (`returns ref of array of i32 of n elements` / `returns copy array of i32 of n elements`), never a
  bare C return type;
- a **text / binary return** ← `returns ref of text of n chars` / `returns ref of binary of n bytes`
  (non-owning), or `returns copy text of n chars` / `returns copy binary of n bytes` (a Lale-owned
  deep copy). The `copy` restriction of §5.2 applies;
- a **read-only reference return** ← `returns readonly ref of array of i32 of n elements`,
  `returns readonly ref of text of n chars`, or `returns readonly ref of binary of n bytes` — the
  same header as the mutable reference, with writes through it forbidden (§3.4, §5.3).

For a **rank-2** array, `m.ptr` addresses the outer buffer of row headers, not a flat `T`
buffer; it is not a C array of `T`.

A plain `export fn` — a **Lale-ABI** export — may declare `array of T` parameters and returns; those
are called by other Lale modules, not by C. A **C-ABI** export (`export C fn`) must use C-compatible
types only. Everything else — a bare-pointer parameter, a `{ptr, len}` struct, a library
descriptor — needs its own explicit rule before it is supported.

#### 13.6 `ARCHITECTURE.md §4.7` — IR serialisation

- Add a dynamic-array `ir_type` form (the Lale source spelling is settled as `array of T`, §2.9).
- `[N x T]` is retained, now denoting the `inline` field type (field-only).

**Open:** the exact IR spelling (`i32[]`, `dyn<i32>`, or a named fat-pointer struct) is
unsettled.

#### 13.7 `lale.md`

- Document the array type: 1-based indexing, complete initialisation, `.count`, units,
  `grow` / `shrink`, and the value-semantics rules.
- Document the read-only header fields — `ptr`, `count`, `capacity` for an array; `ptr`, `bytes`,
  `chars` for `text`; `ptr`, `bytes` for `binary` — and the capacity invariants (`capacity >=
count`; `grow` never decreases it; `shrink` never increases it).
- Document that a reference to a local `text` / `binary` follows the same escape and freeze rules
  as an array (§3.4).
- Document the **escape rule** for references and views: a reference or view of a local array /
  `text` / `binary` (directly or through a field) must not be returned or stored in a global
  (§3.4, §8.5).
- Update **Parameter Passing** for arrays: copy by default, `ref` for element mutation only (no
  rebinding, no resizing, no replacing an owned-buffer element), `copy` for an explicit copy, and
  the definition-site warning for an array parameter copied at every call (§3.1).
- Document **binding**: `var a as array of i32 = f()` moves the call's rvalue; `var a as array of
i32 = b` and `= m[1]` copy (§3.2).
- Document the array **type notation** (§2.9–§2.13): `array of T`, the rank prefix, the extent
  clause, and the count idiom. Today `lale.md` documents only the bracketed spellings.
- Rewrite the existing array examples: `i32[10]` → `array of i32`; `[fill with 42]` →
  `[fill 10 elements with 42]`; `unsafe decl buffer as i32[1000]` →
  `unsafe decl buffer as array of i32 of 1000 elements`; the diagnostic `length=5` → `count=5`
  (§13.13, §13.14).
- Fix the operator table: `count of` is **removed** in favour of the array's read-only `count`
  field (`xs.count`; `t.chars` / `t.bytes` for `text`, `b.bytes` for `binary`). `#size of` becomes
  the runtime `size of` returning `u64 in <bytes>`; document the semantic change (§2.13, §13.14).
- Document `inline` fields and the rendering rules.
- Update the **C FFI** section: an array crosses C as `ptr` + `count`, never as one argument; an
  array / `text` / `binary` return uses a `ref` / `copy` / `readonly ref` descriptor with an
  extent clause (`of n elements` / `of n bytes` / `of n chars`) (§5.2, §13.5).
- Add a **Views** subsection when the ranged-view layer lands (§8) — including the code-point
  indexing rule for `text`.
- Add a **note for experienced programmers** on why Lale writes **`readonly`** where they expect
  `const`. The point is not a design principle but the **absent guarantee**: `readonly` promises only
  that _the holder may not write through this reference_ — it does **not** promise that the data is
  constant, because the owner, another (writable) reference, or a foreign library can still change
  it. `const` would suggest an immutability Lale cannot deliver; `readonly` names the permission that
  is withheld.

#### 13.8 `roadmap.md` and `TODO.md`

`roadmap.md §2 row 14` and `TODO.md [T-030]` state a different array model and must be brought
into line with this design.

#### 13.9 `ARCHITECTURE.md` — add a terminology note: "reference", not "borrow"

Add a short note in the language-design-decisions part (for example beside §5.14's pointer rules)
explaining why Lale avoids the established word **borrow**:

```markdown
#### Why "reference", not "borrow"

Other languages call non-owning access _borrowing_ — temporarily taking something that
belongs to someone else. The metaphor is vivid, but it is a **possession** word used for a
**permission** idea, and it misdescribes Lale's semantics.

Picture the everyday case: a physical book. When a book is lent, it leaves the owner's hands — the
owner cannot read it while it is out. But a _shared_ reference does no such thing: several readers —
**including the owner** — keep reading, and only writing is excluded. And even an _exclusive_
reference does not transfer the book: the owner still owns it and gets it back.

So the accurate picture is not "the book is gone" but "**my rights to it are temporarily reduced**":
many readers and no writer, or one writer and no other reader. Lale therefore names the concept
**reference** (`ref of T`, `ref to x`), and keeps _borrow_ — if it is used at all — for explaining
the idea, never as the name of the thing. Where a holder is denied writes, Lale says **`readonly`**,
naming the permission that is withheld rather than a property of the data.
```

#### 13.10 `ARCHITECTURE.md §5.20` — state the `of` rule once, for the family

Insert, beside the existing vector grammar:

```markdown
#### The `X of T` constructor family

`of` is part of a **two-word constructor** — `vec2 of T`, `array of T`, `ref of T`, `view of T` —
and each constructor takes a **type** as its argument. Chained forms therefore nest to the right by
construction (`ref of array of i32` is `ref of (array of i32)`) and need no associativity rule.
```

Today the rule is written down only for vectors (`ARCHITECTURE.md §5.20`, `lale.md` § Vectors);
the family is now large enough to state it once.

#### 13.11 Grammar (`src/grammar/lale.pest`) — the new syntax

The 1.0.0 array surface is not only `array of T`; the grammar needs every form this proposal
introduces. The branch already carries the array-type and extent rules; the rest is still to add.

- Add `array_type` — `array of T`, plus the rank prefix `2D` / `3D` / …, as a `type_name`
  alternative. (Already on the branch.)
- Remove the bracketed array spellings `i32[5]` and `[i32]` from `type_name` **with the
  implementation** (§13.15). `arr_index` keeps its **value** roles only — `postfix` (`xs[i]`),
  `assign` (`xs[i] = v`), and `compound_assign`.
- Add the **extent clause** `of N elements` (`of 5 by 3 elements` for more than one dimension),
  with a word-boundary guard on `elements`. The same clause admits `of n bytes` for
  `returns ref of binary of n bytes` and `of n chars` for `returns ref of text of n chars` (§5.2);
  both suffix words need the same guard. (Already on the branch.)
- Add `inline` as a **field-only** type modifier in `type_field`. The form is
  `values as inline array of i32 of 3 elements`. It must be rejected in a `var`, a parameter, and
  a return type (§4.1). The grammar may parse it in those positions and let semantic analysis reject
  it with a clear message, or restrict the production to `type_field`.
- Add `ref of T` and `view of T` as `type_name` alternatives, and `readonly` as an optional
  prefix on either. `view of T` parses even though the view layer is deferred, so semantic analysis
  can report "not implemented yet" rather than a syntax error.
- Add `ref to x` as an expression form (the value side of a reference). It must be distinguishable
  from the `ref` parameter modifier (`ref xs as T`), which it is at the second token (`to` vs an
  identifier).
- Extend `return_type` for an `import fn signature`: an optional `ref` / `copy` / `readonly ref`
  before the type, followed by the extent clause. A `copy` return is only valid on an imported
  declaration.
- Add the `grow` / `shrink` statements: `grow xs by n = initialiser` and `shrink xs by n`, with
  `by` contextual. Do **not** add `may fail` in 1.0.0 (deferred, Annex B); if the grammar accepts
  it early, semantic analysis must reject it with a clear message.
- Replace `#size of` with a runtime `size of` operator, and remove `#count of` (§13.14).
- The rank token is recognised **only inside** `array_type`: there is no separate lexer pass, so
  outside a type position `2D` still reads as `2` followed by `D`.
- The nested spelling `array of array of …` **parses**; it is rejected in semantic analysis
  (§13.12).

#### 13.12 Semantic analysis — reject the nested array spelling

Reject an array whose element type is itself an array, with a message that names the fix:

```text
ERROR: an array's element type may not itself be an array (`array of array of i32`):
       write `2D array of i32` for a 2-dimensional array
```

The check is deliberately **not** in the grammar, so the diagnostic can name the fix rather than
merely report an unexpected token. The same policy governs `view of view of T` (§8.6), an array
whose element type is a reference or view (§2.12), and the parameter modifiers on a non-owning
type. The form `fn f(ref xs as view of T)` is a semantic error, because a view is already a
non-owning handle; `fn f(copy xs as view of T)` is a semantic error too, because a plain by-value
view already copies the descriptor; and `ref` / `copy` add nothing to a `ref of T` value either.
The diagnostic points to `readonly view` for the no-write form and to the deferred materialisation
spelling for an owned copy.

#### 13.13 `ARCHITECTURE.md §5.10` — the array initialiser's value grammar

`[fill with 0]` becomes `[fill 10 elements with 0]`. A `fill` states its own count (§2.11), because
otherwise it would read the count from the declaration — a second source of truth. This is
**expression** syntax, not a type change.

#### 13.14 Diagnostics and operators — one word, `count`

- Diagnostics say `count`: `array index out of bounds (index=6, count=5)` (§2.4), replacing `size`
  in compile-time messages and `length` in runtime ones.
- `#count of` is **removed**: use the read-only field — `xs.count` for an array, `t.chars` /
  `t.bytes` for `text`, `b.bytes` for `binary`.
- `#size of` becomes the runtime `size of` returning `u64 in <bytes>` (§2.13). It is not a
  compile-time-only intrinsic; it may be folded to a constant when the value is known.
- `#type of` and `#unit of` are unchanged.
- **This is a semantic change, not only a rename.** `size of` now reports the shallow **content**
  size — `count × sizeof(T)` for an array (live elements, not `capacity`),
  `(last - first + 1) × sizeof(T)` for a view, `.bytes` for `text` / `binary` — not the fat-pointer
  header. Existing `#size of` tests that expect the header size, or that assume a compile-time
  constant for a dynamic array, must be updated; `#count of` tests must be migrated to the
  read-only fields (`xs.count`, `t.chars` / `t.bytes`, `b.bytes`). The result carries the unit
  `<bytes>`; a count carries `<elements>`.

#### 13.15 Migration sequencing — the removals land with the implementation

The bracketed spellings (`i32[5]`, `[i32]`) are **not** removed at sign-off. They are removed
**with the array implementation**, in the same change that lands the new spelling. The reason: the
bracketed forms are the only array spellings that work today, and §2.1 abolishes the fixed-size
array — so removing them first would leave the test suite and `examples/complex.lale` without array
coverage rather than migrating them. Sign-off freezes the **design**; the code migration is a
separate, subsequent change. Its scope is the grammar (array type and extent clause), the AST — the
**Abstract Syntax Tree**, the compiler's tree-shaped internal copy of the program —
(`array_element`, rank), the **symbol table** (the compiler's record of names — variables,
functions, types — and their properties) and type representation, the IR type and serialisation,
the interpreter's memory manager, the ABI specification, the type predicates that test for `[`,
the diagnostic vocabulary, and the tests / examples that use the bracketed forms.
`doc/array_type.md` §8.2 is the historic record.

#### 13.16 `builtins.lale` / IR — read-only fat-pointer fields

The header fields of `text`, `binary`, and the dynamic array are **read-only** from Lale code
(§3.4, §7.1). The `text` / `binary` definitions in `builtins.lale` and the array header description
stand; semantic analysis must reject assignment to these fields (`s.ptr = p`, `xs.count = n`).
`private` is not sufficient, because FFI code must read the address.

`IR_SPECIFICATION.md §1` spells the `text` struct as:

```text
%text = type { ptr: ptr, bytes: u64, chars: u64 }
```

The family field name is now `ptr` (§11.2), so this example already matches. The dynamic array's IR
spelling is still open (§13.6); whichever form is chosen, its header fields are `ptr`, `count`, and
`capacity` at the ABI offsets, and the layout (offsets and sizes) does not change.

## Annex A — View parameter passing (resolved)

A view is a non-owning **handle**: `{ header_ptr, first, last }`. Passing it by value copies the
handle (the descriptor), not the elements. The cost is `O(1)`; the callee still views the caller's
storage, and writes through it are visible to the caller.

```lale
var data as array of i32 = [10, 20, 30, 40, 50]
var mid as view of array of i32 = data[2 to 4]

fn sum_view(xs as view of array of i32) returns i32
    // xs is a copy of the descriptor; it still views data[2..4]
    ...
end fn

fn bump(xs as view of array of i32) returns nothing
    loop var i as u32 from 1 to xs.count
        xs[i] = xs[i] + 1      // writes into data
    end loop
end fn
```

- **By value:** `O(1)` descriptor copy; no element copy; writes are visible to the caller.
- **`readonly view of T`:** the same `O(1)` copy, but no writes through the holder.
- **`ref xs as view of T`:** a **compile error**. A view is already a non-owning handle; `ref` adds
  nothing and would hide the aliasing. Use `readonly view` to forbid writes.
- **No `copy` form.** A plain by-value view parameter already copies the descriptor, so `copy` would
  add nothing. A view is not implicitly converted to an owning `array of T`, and `copy` is a
  **parameter modifier** (§3.1), not a call-site operator, so there is no spelling here for
  materialising the viewed elements. That spelling is deferred with the view layer.

This closes the former Option P / Option R fork: the parameter keeps the view type, and there is no
hidden materialisation. The explicit owned-copy path is deferred with the view layer; `copy` is not
an acknowledgement on a view parameter.

## Annex B — Fallible allocation and resizing

Array construction, `grow`, and `shrink` can fail. This annex specifies the failure causes and the
**1.0.0 behaviour: fail fast**. Recoverable failure (`may fail`) is deferred to a later version;
the sketch below is kept so the deferred design is not lost.

### The failure causes

1. **A negative amount cannot arise.** The amount is `u64 in <elements>` and Lale has no implicit
   numeric conversion, so a negative literal is a compile-time type error and a runtime negative
   value is not expressible (§2.8). `grow` / `shrink` never silently reverse direction.
2. **A `shrink` amount larger than the count.** `shrink xs by n` with `n > xs.count` is a
   failure — it never silently underflows the count. A **constant** violation is a compile-time
   error; a **runtime** one is a runtime failure.
3. **An allocation failure (out of memory).** `grow` may need to enlarge the buffer; an array
   literal, a `fill`, a `copy` return, or a deep copy may need to allocate. If the allocation
   cannot be served, that is a failure. (`shrink` never allocates, so memory is not a cause for
   it.)
4. **A grow-initialiser count mismatch.** `grow xs by n = f()` with a runtime `n` and a
   runtime-count initialiser is a failure when the initialiser's count differs from `n` (§2.8,
   §11.2). A **statically known** mismatch is a compile-time error.

An amount of **zero** is neither: `grow xs by 0` and `shrink xs by 0` are **no-ops**.

### Default: fail fast

A failure **aborts** the program with a clear message, and the target array is left unchanged up to
the abort (it is never corrupted). An array-initialising `var` aborts before the variable is bound.
There is no recovery annotation in 1.0.0.

```lale
grow xs by 2 = [7, 8]   // if this cannot be done, the program aborts
shrink xs by 3          // if xs has fewer than 3 elements, the program aborts
```

### Deferred: opt-in recovery (`may fail`)

The **`may fail`** annotation (a contextual two-word form, meaningful on `grow` / `shrink` and on
an array-initialising `var`) is **not part of 1.0.0**. The deferred design is:

- `grow` / `shrink`: the target is left **unchanged** — a **no-op**;
- a `var` construction: the variable is bound to an **empty** array (see below);
- an **error is recorded on the error stack** (`ARCHITECTURE.md §5.18`);
- the **program continues**.

```lale
// Deferred — not valid in 1.0.0.
grow xs by 2 = [7, 8] may fail   // on failure: xs unchanged, an error recorded, execution continues
shrink xs by n may fail          // n negative or larger than xs.count → no-op + error
```

**Construction is fallible too.** The same deferred rules would cover an allocation performed by an
array-initialising declaration:

```lale
var xs as array of i32 = [fill 1000000 elements with 0] may fail
```

On failure, the error would be recorded and `xs` bound to an **empty** array (count 0, capacity 0),
so the variable is definitely assigned and execution continues. This is the declaration equivalent
of the no-op rule for `grow` / `shrink`. In 1.0.0, a construction that cannot allocate aborts before
the variable is bound.

The annotation is **declarative, not a command**: it states that the operation may fail, rather
than naming an action to take. It is also **distinct from `?`**: `?` propagates an absent optional
_out of the enclosing function_ (an expression concern), whereas `may fail` would tolerate a failure
_of this statement_ and let execution continue in place. The error stack would make a tolerated
failure inspectable (`has errors` / `last error`), so a program could react to a failure it chose to
survive.

### Conditional growth: an optional initialiser

A `grow` whose initialiser evaluates to `nothing` is **skipped** — no failure is involved:

```lale
grow xs by n = try_read(n)   // try_read returns array of i32?; `nothing` ⇒ the grow is skipped
```

This is the single-statement way to grow _conditionally_. With `if` / `when` (statements), a guard
block is usually clearer:

```lale
when extra > 0
    grow xs by extra = read_extra(extra)
end when
```

Because a negative amount is itself a failure (and a negative constant a compile error), such a
guard is **not required for safety** — the language never silently grows by a negative amount — it
is a matter of intent.

### Interaction with nested arrays and partial copies

`grow` on a nested array follows the same rules for the inner array and leaves the outer array
untouched (§2.7). If a deep copy must allocate several buffers and an allocation fails partway,
the buffers already allocated for the copy are released before the program aborts — cleanup itself
must never double-free or leak.

### Deferred design questions

- **Warn when a `may fail` error is never consulted.** As with the aggregate-copy warning, a
  `may fail` operation whose error is never read before the function returns could be flagged.
