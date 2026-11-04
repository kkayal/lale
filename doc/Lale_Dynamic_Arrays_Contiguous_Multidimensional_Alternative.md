# Dynamic Arrays and Views --- Contiguous Multidimensional Alternative

**Status:** design proposal --- **not yet approved**. Everything below
is the intended design; none of it is implemented until the design is
approved (the array-notation branch carries a feasibility spike only:
the new grammar and the two tests). Cross-references such as
`ARCHITECTURE.md §5.14d` point at the current specification that the
design extends or amends.

## How to read this document

This alternative keeps the proposal's two-part schedule, but changes the
storage model for multidimensional arrays:

- **Part I --- Dynamic arrays** (sections 2--7): owned one-dimensional
  arrays, contiguous rectangular multidimensional arrays, jagged
  nested arrays, plus a **narrow, whole-range reference** as the 1.0.0
  non-owning form. **Planned for Lale 1.0.0.**
- **Part II --- Ranged views** (section 8): non-owning **sub-ranges**
  (`view of T`, `xs[a to b]`). **Planned for 2.0.0.**

Non-owning access (access that never frees the storage it touches) is
not all-or-nothing. Lale 1.0.0 already uses non-owning access in three
places: `ref` parameters (an expression-scoped reference to the caller's
storage); **references**, whose type is `ref of array of T` (§3.4),
produced by reading an `inline` field, by an FFI `ref` return, or
explicitly by `ref to x`; and `readonly` references (with the view
layer, `readonly` views too), which forbid writes through the holder.
What Part II defers is the _ranged_ view --- a sub-range window with its
own freeze and invalidation rules --- not non-owning access as such.

Three concerns are often conflated but are kept separate here:
**ownership** (who frees a buffer), **references** (non-owning access),
and **storage layout** (where bytes live). The array type unifies their
_surface syntax_ without pretending they have identical semantics.
Section 10 lists what lands in which version.

The **type notation** is part of this document: how the array type is
spelled, and where the element count is written (§2.9--§2.13). It was
reviewed separately --- `doc/array_type.md` keeps the full rationale and
the rejected alternatives, and is now a historic record --- and it is
merged here in full.

### 1. The idea

Lale has one array family: `array of T` for a one-dimensional dynamic
array, `2D array of T`, `3D array of T`, and so on for **contiguous
rectangular blocks**, plus nested `array of array of T` when a jagged
shape is wanted. An array value is carried as a **descriptor** --- a
small structure that travels alongside the data and records its address and
runtime shape --- so one type serves different counts and extents. This
is the whole point:

- **The count is a runtime property.** A function such as `sum`
  accepts an array of any count without generics, because the count
  travels _with_ the value instead of being baked into its type.

```lale
fn sum(xs as array of i32) returns i32
    var total as i32 = 0
    loop var i as u64 from 1 to xs.elements
        total = total + xs[i]
    end loop
    return total
end fn

var a as array of i32 = [10, 20, 30, 40, 50]
var b as array of i32 = [1, 2, 3]

write sum(a)   // 150
write sum(b)   // 6
```

`a` and `b` are the **same type**: `array of i32` (§2.9). The element
count lives in the value, not in the type, so `sum(a)` needs no
conversion. The same principle applies to rectangular arrays: their rank
is part of the type, but their extents are runtime properties of the
value.

A later layer adds the **view**, a non-owning window onto a sub-range of
an array (or of `text` / `binary`), so a function can read part of an
array without copying it. Views are a separate feature (section 8) and
are not required by the array type itself.

**A note on cost.** Each of `sum(a)` and `sum(b)` deep-copies its
argument --- Lale passes arrays by value (§3.1), so the elements are
duplicated, not shared. Because the `sum` parameter is copied at every
call, the compiler warns once at the function definition; writing `copy`
before the parameter (`copy xs as array of i32`) silences the warning by
acknowledging the copy. To skip the copy entirely, pass the array by
`ref` (§3.1) for a writable reference, or use a `readonly ref` (§3.4)
for a cheap read-only reference.

## Part I --- Dynamic arrays (planned for Lale 1.0.0)

### 2. The array type

The type is **spelled** `array of T` --- one type, whatever the element
count (§2.9). The word is **"array"**, not "list": Python's `list` is
heterogeneous and holds pointers to boxed objects, while this type is
homogeneous, contiguous, and growable, like Julia's `Array{T,1}`. The
same array family also represents vectors and rectangular matrices; Lale
does not need separate built-in vector or matrix storage types (§9.1).

#### 2.1 One type, runtime count

There is exactly one array type: **`array of T`** for an element type
`T` (§2.9). There is no separate fixed-size array type. The element
count is **never part of the type** --- it belongs to the value --- so a
declaration supplies it through its initialiser (§2.10), or, where no
value exists, through the extent clause after the type (§2.11). Either
way its count is whatever `.elements` reports at runtime.

The one type has one default representation: **heap-allocated** (§7.2)
--- the buffer lives in long-lived memory that is freed only when the
array is finished with it. A compiler may instead place the buffer in a
fixed-size **stack** slot --- memory tied to the current function call
and reclaimed automatically when the function returns --- when it can
prove that the count never changes (§7.3). This is an optimisation,
never a semantic guarantee. The `inline` modifier (§4.1) **enforces**
the fixed-size form for a struct field, laying the elements out as the
field's own bytes so the field matches a C array member; but `inline`
exists only for the C FFI context, so a Lale programmer cannot force the
optimisation on an ordinary variable.

#### 2.2 Declaration and initialisation

Arrays must be **completely initialised** (`ARCHITECTURE.md §5.10`) ---
no element may be left with a garbage value. Ordinary arrays are
initialised from an explicit element list or a uniform fill; the empty
array is the explicit empty literal `[]`. A rectangular shape whose total
count is zero may instead be stated directly with an extent clause
(§2.11).

```lale
// Explicit elements: the number of listed values gives the element count
var primes as array of i32 = [2, 3, 5, 7, 11]        // 5 elements

// Explicit empty array: [] means zero elements
var empty as array of i32 = []

// Uniform fill: the fill says how many elements, then the value
var zeros as array of f64 = [fill 1000 elements with 0.0]
```

There is exactly **one** source of truth for the element count: an
ordinary initializer states it itself --- by how many elements it lists,
or by the `N elements` of a `fill` --- so there is no second count to
disagree with it, and the old "declared 6, literal 5" mismatch cannot
arise (§2.10). A zero-product rectangular declaration states the shape
explicitly and has no element values to initialise (§2.11).

**Fill evaluation.** `[fill N elements with expr]` evaluates `N` exactly
once. If `N == 0`, the fill produces an empty array and **does not
evaluate `expr` at all**. If `N > 0`, it evaluates `expr` exactly once
and replicates the result into every element.
For `N > 0`, an element type that owns its buffer (a nested array,
`text`, `binary`, or a struct owning one) gets a **deep copy** of the
evaluated result in each element, so the elements are independent.

**`[]` is the empty-array literal.** `[0]` is a one-element array
containing zero, and `[0, 0]` a two-element array --- the element list
means exactly what it lists. There is no special reservation and no need
to spell a one-element zero with a fill. Nesting is unambiguous: `[]` is
empty and `[[0]]` is a 1 × 1 matrix containing zero.

When an array must start uninitialised, the `unsafe` declaration states
that explicitly --- and since there is no value to read a count from,
the count is written after the type (§2.11):

```lale
unsafe decl buffer as array of i32 of 1000 elements   // the programmer must fill it before reading
```

That declaration creates an array whose `.elements` is **1000 immediately**
--- the elements are the programmer's responsibility, exactly as with
`unsafe decl x as i32`. The `unsafe` is on the declaration; Lale does
not track which elements have been written, so initialising every
element before use is the programmer's obligation, not a compiler
guarantee. A systems language must allow this, and it is explicitly
marked `unsafe`.

In 1.0.0 `unsafe decl` is restricted to **cleanup-free element types**.
If the element type itself needs destruction --- a nested array, `text`,
`binary`, an optional, or a struct containing any of these --- an
uninitialised element's garbage bytes could be followed by the
destructor, so such a declaration is rejected (§11.2).

#### 2.3 Element count, dimensions, and capacity

`xs.elements` is the read-only **total number of elements** covered by an
array value, carrying the `<elements>` unit. `xs.dimensions` returns an
ordinary, independent rank-1 array of `u64 in <elements>` containing the
size of each dimension in order. The returned array has `xs.rank` elements; its own `.elements`
therefore reports the rank. For a rank-1 array, `.dimensions` is a
one-element array containing its extent `n`. When `n` is zero, that
value is `[0]` --- a one-element array containing zero, distinct from the
empty array `[]`.
For a rectangular 3D array with
shape `1 × 3 × 2`, `.dimensions` is `[1, 3, 2]`; after selecting its
first index, `m[1].dimensions` is `[3, 2]`. The dimensions array is a
value, not an alias to the descriptor, so modifying that returned array
does not modify the source array. Even an empty rank-1 array has one
dimension: its `.dimensions` value has one element, `0`. For a jagged
nested array, `.dimensions` reports only the outer array's extent; it
does not imply a rectangular shape for the nested rows.

There are **no `count of` or `capacity of` operators**. The source-level
properties `.elements`, `.dimensions`, `.rank`, `.ptr`, and (rank 1 only)
`.capacity` are read-only; `.elements` and `.dimensions` are properties,
not necessarily physically stored fields. The ABI descriptor retains a field named
`count` for the rank-1 element count, and `count_i` fields for rectangular
dimension counts; these are representation names, not source-level
property names (§7.1, §13.4). `.elements` maps to the rank-1 `count`
field and is computed as the product of `count_i` for a rectangular
array. `.dimensions` maps to those per-dimension counts.

A compiler may fold `.elements`, `.dimensions` components, `.rank`, or
`.capacity` to constants where statically known. Capacity is a **rank-1**
concept --- `xs.capacity` exists only on `array of T`. Rectangular arrays
(`2D array of T` and up) have no spare capacity: changing any dimension
rebuilds the compact payload when the new shape has elements; a
zero-product shape has no payload buffer (§2.7, §7.1). For rank 1, capacity has these
**language-level guarantees**: `capacity >= elements` always; `grow`
never decreases `capacity`; `shrink` never increases it. The exact
capacity and growth factor remain unspecified, so a program must not
assume a particular value beyond those invariants, and the compiler
remains free to choose the growth policy (§7.3). `capacity` after a deep
copy, a reassignment, or an rvalue binding is likewise **unspecified**
--- it may equal `elements` or any larger value.

A zero-element **owning rank-1 array** needs no element storage, and the
compiler never calls `malloc(0)`: in an owning descriptor, `capacity == 0`
means `ptr` is **null**, and any owned allocation has `capacity >= 1`.
The `[]` empty literal starts with `capacity == 0` and a null pointer.
An owning zero-element array may still have a non-null pointer when
`capacity > 0` --- for example after `shrink` drops the last element
without releasing the buffer --- but no valid element access exists.
These allocation invariants apply to owning arrays, not to non-owning
references that wrap inline or foreign storage: such a rank-1 reference
preserves the origin pointer verbatim at count zero (the runtime does
not null it), and its synthetic `capacity == count` does not describe an
owned allocation.

A rectangular array has no `capacity` field. If any extent is zero, its
`.elements` is zero and its `ptr` is **null**; the runtime checks for a
zero extent **before multiplying the extents**, so a later zero cannot
be preceded by a spurious overflow failure. No `malloc(0)` is performed.
See §13.5 for the C boundary.

#### 2.4 1-based indexing and bounds checking

Indices start at 1 (mathematical convention, `ARCHITECTURE.md §5.9`):

```lale
var numbers as array of i32 = [10, 20, 30, 40, 50]
write numbers[1]   // 10 — the first element
write numbers[5]   // 50 — the last element
```

An index and a loop counter are **`u64`**, the same width as the
descriptor's element-count fields. `.elements` and `.capacity` carry the
`<elements>` unit --- a count of elements --- and a loop counter over an array **infers that same unit
from its `to` bound** (`loop var i as u64 from 1 to xs.elements` makes `i`
a count of elements), following the ordinary rule that a binding without
a declared unit takes the unit of the value it is bound to (`lale.md` §
Unit Inference). A literal index such as `xs[3]` infers `<elements>`
from the indexing context; the index's unit does not affect the element
access.

Every access is **bounds-checked** (`ARCHITECTURE.md §5.12`). For a rank-1
array, the index must lie within `1 … xs.elements`. For a rectangular
array, each index is checked against the extent of its corresponding
dimension before the row-major address is calculated. An out-of-range
index aborts with a clear message rather than reading stray memory:

```text
ERROR at program.lale:3:10: array index out of bounds (index=6, elements=5)
```

For a rectangular array the diagnostic lists every index against every
extent, for example a `3 × 4` matrix indexed out of bounds at `m[2][5]`:

```text
ERROR at program.lale:4:12: array index out of bounds (index=[2,5], dimensions=[3,4])
```

Multidimensional indexing (`m[2][3]`) checks each index against its
corresponding extent; for a rectangular block, the compiler then
computes one row-major offset. For nested jagged arrays, each step loads
the next row descriptor and checks that row's own element count. The
standard loop idiom `loop var i as u64 from 1 to xs.elements` executes
zero iterations when `xs.elements == 0`, so it is safe for an empty array.

#### 2.5 Physical units

An array may carry a physical unit for its elements, with the same rules
as scalars (`ARCHITECTURE.md §5.13`):

```lale
var temps as array of f64 in <K> = [273.15, 300.0, 310.5]
var positions as array of f64 in <m> = [fill 10 elements with 0.0]
```

Every element carries the array's unit through reads and writes ---
`temps[1]` has type `f64 in <K>` --- and a view (section 8 --- Part II,
deferred) inherits the unit of its origin.

#### 2.6 Global arrays are compile-time constants

A module-level `var` must initialise to a value the compiler can fold to
a constant (`ARCHITECTURE.md §5.29`), which exists to remove
static-initialisation-order problems. For an array this means **both the
count and every element** must be constant:

- `var a as array of i32 = [10, 20, 30, 40, 50]` --- count and
  elements are literals. ✓
- `var b as array of i32 = [x, y, z, w, v]` --- the count is constant
  (five elements), but the elements are runtime values → rejected;
  move the setup into top-level code.
- `var c as array of i32 = make_ints(n)` --- neither count nor
  elements are constant → rejected.

This is the same rule scalars already follow (`var x as i32 = read()` is
rejected for the same reason), so arrays add no new exception.

A module-level rectangular array declared without an element initializer
under the zero-product rule in §2.11 is also valid only when every extent
is a compile-time constant. Its descriptor and shape are emitted as static
data with a null `ptr`; no payload allocation or runtime initialisation
is needed. A runtime-dependent extent at module scope remains rejected.

The constant folds into **static** data: **the whole value --- its
descriptors and every payload --- is baked into the program file**, in the
program's **static storage** --- memory that lives for the whole run ---
rather than the heap. A global `array of i32 = [10, 20, 30]` stores its
three elements statically; a global `array of text = ["alice", "bob"]`
stores the two `text` descriptors **and** the bytes of `"alice"` and `"bob"`
statically; a global `2D array of i32 = [[1, 2], [3, 4]]` stores the
descriptor and one contiguous element block statically; a global
`array of array of i32 = [[1, 2], [3, 4]]` stores the outer descriptor,
the row descriptors, and their separate buffers statically. Because a
constant's representation is fully known at compile time, **module load
performs no runtime allocation**, whatever the element type. This is a
compiler task --- folding arbitrary constant values into static data ---
not a restriction on which element types a global may use.

A module-level array may still be mutated in place, so that storage is
writable; an array that `grow`s transitions to owned heap storage
(§3.2). For a rectangular block, the complete block is copied; for a
jagged array, only the resized array's own descriptor and element buffer
move. Placement stays an implementation detail --- a global that is
never mutated could equally sit in read-only storage. The ownership rule
that keeps the static payloads safe when this happens --- each value
remembers whether its own buffer is static or heap-owned --- is stated
in §3.2.

An **imported** module-level array is shared by **reference**, like
every imported variable (`lale.md` § Module Execution Model): the
importing module neither copies nor owns it, must not free it, and sees
the exporter's mutations. A `grow` / `shrink` / reassignment through the
import changes the exporter's array; the freeze rules of §3.4 apply to
the importing module's code as usual. The heap is process-global, so a
`grow` that moves the payload to heap storage allocates one shared buffer
regardless of which module issues the `grow`; the exporter's descriptor
(the shared global) is updated, and the runtime owns the buffer, freeing
it at program end.

#### 2.7 Multidimensional arrays: contiguous blocks and jagged nesting

The rank-prefixed form --- `2D array of i32`, `3D array of f64`, and so
on --- is a **rectangular multidimensional array stored in one
contiguous block** (§2.12). It is not an array of row descriptors. Rank is
fixed by the type; each dimension has a runtime `count`. The buffer
holds exactly the product of the per-dimension counts in compact
row-major order, and the last index varies fastest. There is **no spare
capacity** (§2.7), so changing any dimension's count changes the payload: a nonzero-product
shape receives a fresh buffer with surviving elements moved to their
corresponding indices, removed elements destructed, and new slots
initialized as specified in §2.8; a zero-product shape has a null `ptr`
and no payload buffer. This keeps `ptr` suitable
for C APIs that expect a compact rectangular block:

```lale
var m as 2D array of i32 = [[1, 2, 3], [4, 5, 6]]
write m.dimensions[1]  // 2 — rows
write m.dimensions[2]  // 3 — columns
write m.elements       // 6 — total elements
write m[2][3]          // 6

// Zero extents preserve shape even though the payload is empty.
var no_columns as 2D array of i32 of 3 by 0 elements
var no_rows as 2D array of i32 of 0 by 5 elements
write no_columns.elements             // 0
write no_columns.dimensions[1]         // 3
write no_columns.dimensions[2]         // 0
write no_columns[1].elements           // 0 — a valid empty row
// no_columns[1][1] is out of bounds: that row has no columns.
// no_rows[1] is out of bounds: that matrix has no first row.
```

A 3D array follows the same rules; the last index varies fastest:

```lale
var v as 3D array of f64 = [[[1.0, 2.0], [3.0, 4.0]], [[5.0, 6.0], [7.0, 8.0]]]
write v.dimensions[1]   // 2 — depth
write v.dimensions[2]   // 2 — rows
write v.dimensions[3]   // 2 — columns
write v.elements        // 8
write v[2][1][2]        // 6.0
```

The nested literal syntax is convenient, but every row of a
rank-prefixed rectangular array must have the same shape. A shape
mismatch is a compile-time error when statically known, otherwise a
runtime failure during construction. The flattened offset for 1-based
indices `(i1, …, ir)` is computed from the per-dimension counts; no
pointer is loaded from an intermediate row. The descriptor stores one
`count_i` per dimension. The total live element count is the product of
the counts, and the buffer holds exactly that many element slots --- no
spare capacity. Before computing a product, the runtime first checks whether any extent
is zero. If so, the total element count is zero without multiplying the
other extents, `ptr` is null, and there is no payload allocation. Only
when every extent is nonzero are products and byte sizes computed, with
overflow checks before allocation or resizing.

Consequences:

- **One payload allocation.** Constructing a rectangular array
  allocates one element buffer (unless it is a compile-time constant
  in static storage). Destruction releases that buffer once; it does
  not recursively destroy row buffers because rows are not separately
  allocated values.
- **Copying is linear and contiguous.** A deep copy copies the
  complete element block. Element types that themselves own resources,
  such as `text` or `binary`, are still copied and destroyed element
  by element under the ordinary ownership rules (§3.2).
- **The shape is rectangular.** Each extent is recorded in the value.
  Rank-2 rows all have the same column count; rank-3 planes share the
  same row and column extents.
- **Each dimension can grow or shrink --- by reallocating.** A
  rectangular array may change any dimension's live count while
  preserving the rank. Because there is no spare capacity, a shape change
  creates a fresh compact row-major buffer (unless the new total element
  count is zero, in which case the new `ptr` is null). Elements whose
  multidimensional indices exist in both shapes are **moved** to the same
  indices in the new buffer; they are not deep-copied, and their payload
  ownership transfers with them. Elements whose indices no longer exist
  are destructed exactly once. Newly created slots are initialised under
  §2.8's rules. The old buffer is released after surviving elements have
  been moved and dropped elements destructed. A shape change that adds a
  slice containing zero element slots performs no element initialisation
  and does not evaluate or shape-check an initializer expression.
- **Resizing is `O(total elements)`, not `O(slice)`.** Because a
  rectangular array has no spare capacity, any shape change --- including
  growing or shrinking one dimension by one --- reallocates the entire
  compact block and moves every surviving element. For a large matrix,
  `grow dimension 1 of m by 1 = row` is `O(m.elements)`, not `O(columns)`;
  repeated one-dimension growth is therefore quadratic (§7.4). A hot path
  that grows a dimension repeatedly is better served by a jagged
  `array of array of T` (whose outer array grows in amortised `O(1)`) or
  by allocating the rectangular shape up front --- at the cost of losing
  the contiguous-layout guarantee.

- **Compact layout is automatic.** A reshape with a nonzero total
  element count produces a fresh buffer, so the live elements are dense
  and row-major by construction; a zero-product shape has no payload
  buffer. No in-place element rearrangement is ever required.
- **A chained index is a single checked address calculation.**
  `m[2][3]` is bounds-checked against each corresponding extent and
  then accesses the calculated element directly.
- **A partial index is an lvalue, not a view.** `m[1]` on a
  `2D array of T` designates the stored row --- an lvalue of type
  `array of T`. Writing through it (`m[1][2] = v`) mutates in place,
  while binding it (`var a as array of i32 = m[1]`) deep-copies the row
  (§3.2). The non-owning form is `ref to m[1]`, which points into the
  frozen payload (§3.4).

When genuinely unequal row sizes are useful, use the explicitly nested
form `array of array of T`:

```lale
var rows as array of array of i32 = [[1, 2, 3], [4], [5, 6]]
write rows.elements       // 3 — number of rows
write rows[1].elements    // 3 — first row's count
write rows[2][1]       // 4
```

The two forms may nest in any combination; a jagged outer array may hold
contiguous rectangular blocks of differing shapes:

```lale
var blocks as array of 2D array of i32 = [[[1, 2], [3, 4]], [[5]]]
write blocks.elements        // 2 — two blocks
write blocks[1][2][2]        // 4
write blocks[2].dimensions   // [1, 1] — the second block is 1 × 1
```

Nested arrays remain independently growable and may be jagged. They use
separate row buffers, recursive copy/destruction, and pointer
indirection. This is the opt-in representation for irregular data; it is
not the default meaning of `2D array of T`. The `inline` field remains a
separate fixed-size C-layout form (§4.1).

#### 2.8 Growth

An array grows with `grow` and shrinks with `shrink`:

```lale
grow xs by 2 = [7, 8]   // append [7, 8]; count increases by 2
shrink xs by 3          // drop the last 3 elements; count decreases by 3
```

`grow` and `shrink` need an **owning** binding: a `ref` parameter may
mutate elements but cannot resize the caller's array (§3.1).

`grow` and `shrink` apply to rank-1 arrays, individual jagged rows, and
selected dimensions of rectangular arrays. For a rectangular array,
the operation changes the live count of the selected dimension while
preserving the rank. It does not change the type. The spelling is
`grow dimension d of m by k = init` and `shrink dimension d of m by k`,
where `d` is a compile-time constant in `1 .. rank`; a rank-1 array keeps
`grow m by k` / `shrink m by k`. A rectangular array
has no spare capacity (§2.7), so resizing any dimension reallocates the
entire element buffer. Surviving elements move to their corresponding
multidimensional indices in the fresh compact row-major block; elements
that fall outside the new shape are destructed exactly once, and new
slots are initialized according to the rules below.

- The amount is `u64 in <elements>`. A literal amount infers that type
  and unit from the `by` context --- `grow xs by 2` means
  `2 as u64 in <elements>` --- the same literal inference as
  `var a as u32 = 5`. Lale has no implicit numeric conversion, so a
  `u32` variable must be widened explicitly ---
  `grow xs by (n as u64)` --- exactly as for every other numeric
  operation (§2.3). Because the type is unsigned, a **negative amount
  cannot be expressed**: a negative literal is a compile-time type
  error, and no well-typed runtime expression can be negative. `grow`
  / `shrink` never silently reverse direction, and there is no runtime
  negative-amount failure.

- **Ordinary `grow` initialisation.** A rank-1 `grow` is followed by an
  initialiser for exactly the new elements, because Lale requires
  complete initialisation: `grow xs by 2 = [7, 8]` supplies two values,
  and `grow xs by 3 = [fill 3 elements with 0]` fills three new elements
  uniformly. The fill states its count like every other initialiser
  (§2.11), so the amount is named twice here (a nit in §11.4). A
  statically known count mismatch is a compile-time error. For
  rectangular growth, the initialiser supplies the new block directly: it
  is a rank-`r` array whose extent along the grown dimension `d` is `k`,
  and whose extent along every other dimension equals the array's current
  extent in that dimension. For `k == 1` the singleton extent along `d`
  may be omitted, so the initialiser is written as a single `(r-1)`-dimensional
  slice (adding a row requires a slice of `.dimensions[2]` elements,
  adding a column a slice of `.dimensions[1]` elements, and so on). A
  statically known mismatch is a compile-time error. The zero-extent exceptions below take
  precedence over initializer evaluation and shape checking. A `grow`
  whose initializer evaluates to `nothing` is **skipped** in the
  ordinary case --- no failure is involved (Annex B).

- **Shrinking drops and destructs elements.** No initializer is needed.
  Every removed element is destructed exactly once; a `text`, `binary`,
  or nested-array element thereby releases any buffer it owns. For a
  rank-1 array or an individual jagged row, shrinking leaves that array's
  own buffer and `capacity` unchanged. For a rectangular array, shrinking
  a dimension is a shape change: surviving elements are moved to their
  corresponding indices in the new row-major block, removed elements
  are destructed, and the old block is released. If the new shape has a
  zero extent, its `ptr` is null and no payload block is retained.

- `grow` may reallocate a rank-1 buffer; rectangular resizing always
  replaces the buffer when the shape changes. Surviving elements are
  **moved**, not deep-copied: their element representations are relocated
  without running their destructors, so payloads they own (nested buffers,
  `text`, `binary`) transfer to the destination slots rather than being
  copied or freed. Elements dropped by a shrink are destructed exactly
  once, as specified above. The old block is released only after all
  surviving elements have been safely transferred.

- `grow xs by 0` and `shrink xs by 0` are **no-ops**.

- For a rank-1 array or jagged row, `shrink xs by n` with `n` greater
  than `xs.elements` is a **failure** (it never silently underflows the
  stored `count`). For a rectangular array, the amount is checked against
  the selected dimension's extent, not the total `.elements` value; a
  shrink larger than that extent is a failure.

- **Zero extents in rectangular arrays.** If a dimension resize adds a
  slice whose element count is zero because another extent is zero, the
  operation changes the shape only: it does not evaluate or shape-check
  `init`, does no element copies or destruction, and allocates no payload
  buffer. If a resize changes a zero-element rectangular array into a
  non-empty array, the newly added slice must be supplied by an explicit
  initialiser: Lale 1.0.0 has no built-in default element value, and the
  supplied `init` expression is evaluated and shape-checked against the
  newly added slice. Because extents are runtime values, a resize that
  adds a non-empty slice to a zero-element rectangular array is a runtime
  failure when the initialiser's shape does not match; if the compiler can
  prove the mismatch from static shape information, it may diagnose it at
  compile time. The failure occurs before the target is modified.

  The zero-extent cases override the ordinary initializer evaluation order
  below. The normal initializer-count check is for rank-1 growth;
  rectangular growth checks the initializer against the new-block shape of
  §11.2. This exception does not change ordinary initialization for
  resizes that add a non-empty slice to a non-empty rectangular array.

- **Evaluation order (normative).** For an ordinary rank-1 `grow xs by n = init`
  --- that is, not one of the zero-extent rectangular cases above ---
  (1) evaluate `n` exactly once;

  (2) if `n == 0`, the grow is a **no-op** and `init` is **not**
  evaluated; (3) otherwise evaluate `init` exactly once; (4) if
  `init` evaluates to `nothing`, the grow is **skipped**; (5)
  otherwise the initialiser's element count must equal `n` --- a
  statically known mismatch is a compile-time error, a runtime
  mismatch a failure; (6) only after steps 1--5 succeed may the
  buffer be (re)allocated, and the new `count` / buffer is
  committed last. A failure at any step aborts and leaves the
  **target array** `xs` unchanged (never corrupted); side effects
  performed while evaluating `n` or the initialiser are not
  undone.

  A rectangular `grow dimension d of m by k = init` follows the same
  shape: (1) evaluate `k` exactly once; (2) if `k == 0`, the grow is a
  **no-op** and `init` is **not** evaluated; (3) if the added block has
  zero element slots because another extent is zero, the operation
  changes the shape only --- `init` is **not** evaluated or shape-checked
  (§2.8); (4) otherwise evaluate `init` exactly once; (5) if `init`
  evaluates to `nothing`, the grow is **skipped**; (6) otherwise `init`'s
  rank and shape must match the new block (extent `k` along `d`, the
  current extents along every other dimension) --- a statically known
  mismatch is a compile-time error, a runtime mismatch a failure; (7)
  only after steps 1--6 succeed may the new buffer be allocated and the
  shape committed last. A failure leaves `m` unchanged; side effects of
  evaluating `k` or `init` are not undone.

  A rank-1 `shrink xs by n` evaluates `n` once, then checks
  `n ≤ xs.elements`, then destructs the dropped elements and lowers
  `count`. A rectangular shrink evaluates `n` once and checks it against
  the selected dimension's extent; its shape-change and commit rules are
  those above, including zero-extent cases.

- An **allocation failure** (out of memory) is a **failure**. This
  includes initial allocation (a literal or positive-count `fill`), a
  `copy` return or deep copy, a rank-1 `grow` that needs a new buffer,
  and any rectangular shape-changing `grow` or `shrink` whose new shape
  has a nonzero element count. Rank-1 `shrink` does not allocate; a
  rectangular resize whose new shape has a zero extent does not allocate
  a payload buffer. See Annex B for the failure rules.

- **A failure is fatal.** A failure **aborts** the program with a
  clear message, and the target array is left unchanged up to the
  abort (never corrupted). 1.0.0 has no annotation that tolerates a
  failure; recoverable failure (`may fail`) is deferred (Annex B,
  §10).

- Growing a **nested** array (`grow rows[1] by 2 = …`) follows the
  same rules for the inner array and leaves the outer array untouched,
  because each inner array owns its own buffer (§2.7).

- An indexed expression used as a **mutation target** ---
  `grow rows[1] by 2` or `rows[1] = row` --- designates the **stored**
  element, never a temporary copy produced by an ordinary read
  (§11.2).

- An **array-valued struct field** is a mutation target like any
  owning binding: `grow s.items by 2` and `s.items[1] = v` operate on
  the field's stored array, subject to the same freeze rule (§3.4,
  §8.5).

- For **rank-1** `grow xs by n = f()`, a runtime `n` and a runtime-count
  initializer produce a **failure** when the initializer's element count
  differs from `n` (§11.2). Rectangular growth instead validates the
  initializer against the selected slice shape when that initializer is
  applicable; the zero-extent exceptions above take precedence.

- If a deep copy must allocate several buffers (for a nested array)
  and an allocation fails partway, the buffers already allocated for
  the copy are released **before** the program aborts --- cleanup
  itself must never double-free or leak.

#### 2.9 The type notation: `array of T`

The array type is written **`array of T`**. The bracketed forms no
longer spell a _type_ --- `i32[5]` and `[i32]` are not ways to write it
--- while brackets keep their value roles: **indexing** (`xs[i]`) and
**value literals** (`[1, 2, 3]`, with `[]` the empty literal). A _type_ therefore contains no brackets, and a
_value_ still does. (The full rationale,
including every rejected alternative, is kept in `doc/array_type.md`.)

`of` is part of a **two-word constructor**, like `pointer to`:
`array of`, `ref of`, and `view of` each take a **type** as their
argument, so chains nest right by construction (`ref of array of i32`
is `ref of (array of i32)`) and need no associativity rule. This keeps
_storage composition_ separate from _physical dimensions_ (`in <unit>`)
and keeps brackets out of the type grammar. The specialised `vec2 of T`,
`vec3 of T`, `vec4 of T`, and built-in matrix types are replaced by the
array representation and array-aware operations (§9.1).

The spelling was chosen because `array of T` reads as English where
`[]` in a type position is jargon, and because Ada and Pascal already
spell arrays this way. Array, reference, and view types use the same
`X of T` constructor pattern; vector- and matrix-specific type
constructors are removed by §9.1. It costs verbosity: a declaration
that states its extents, such as `2D array of i32 of 5 by 3 elements`,
is about nine tokens. The same extent clause also makes zero-product
rectangular shapes explicit and safe to declare without an element
initializer (§2.11).

#### 2.10 The count is a value

The element count belongs to the **value**, not to the type.
`array of i32` says nothing about how many elements there are; the
initialiser does (§2.2) --- or, where no value exists, the declaration's
extent clause (§2.11) --- and `.elements` reads it back (§2.3).

That keeps one source of truth: there is no count in the type to keep in
step with the count in the value, so the two cannot contradict each
other. The empty literal is the same rule at zero ---
`var e as array of i32 = []` has `.elements == 0`, taken from the
explicit empty literal, with no extent clause written.

Nothing is being converted: a count is a value, not a type, so taking it
from the initialiser is consistent with Lale's "no implicit conversions"
rule.

#### 2.11 Where no value can supply the count or shape

Some constructs cannot derive their count or shape from a list of
values, so they state it explicitly:

| Construct                                   | Element values? | Why the count or shape is explicit                                   |
| ------------------------------------------- | --------------- | -------------------------------------------------------------------- |
| `[fill 10 elements with 0]`                 | yes (uniform)   | a uniform value says nothing about _how many_                        |
| `unsafe decl b as array of T of n elements` | no              | uninitialised storage is requested explicitly                        |
| `var m as 2D array of T of 0 by 5 elements` | no; zero slots  | the zero-product shape is complete and needs no element initialisers |
| `inline array of T of n elements`           | no              | the count is the C array dimension (§4.1)                            |
| `returns ref of array of i32 of n elements` | no              | the count is bound to a runtime parameter (§5.2)                     |

The clause uses bytes for `binary` and chars for `text`:
`returns ref of binary of n bytes` and `returns ref of text of n chars`
(see §5.2).

The rule is that the count goes wherever the value can carry it: into
the value for a `fill`, and otherwise into the declaration, after the
type, as an **extent clause**. A safe declaration may omit an element
initializer only when the product of the stated extents is zero: there
are no element slots left uninitialised. If the product is nonzero, the
ordinary complete-initialisation rule still applies; absent an
initializer, such storage requires `unsafe decl`. The clause reads
`of N elements` --- a **noun phrase**, so it is unambiguously a count
--- and more than one dimension reads `of 5 by 3 elements` (§2.12). An
extent may be a runtime expression (`of n elements`), and `elements` is
a suffix word, so it needs a word boundary (`elementsX` stays a different
identifier). Lale has no reserved words, so `array`, `of`, `elements`,
and `by` remain ordinary identifiers.

For a rectangular declaration with zero total elements, the explicit
extent clause itself supplies the complete value shape, for example
`var m as 2D array of i32 of 0 by 5 elements`. No `unsafe` is needed,
because there are no element slots to initialise. The same declaration
form with a nonzero product still requires an initializer or `unsafe
decl`.

Rejected spellings, and why: `count 5` (`array of i32 count 5`) reads
unnaturally; `[fill with 10 times 0]` reads as multiplication;
`[fill 10 times with 0]` reads as _repeat the fill_. A fill therefore
always states its count --- `[fill with 0]` is not kept, because it
would read its count from the declaration, a second source of truth.

#### 2.12 The rank prefix and nested array types

Rectangular multidimensional arrays are written with a **rank prefix**
on the type: `2D array of i32`, `3D array of i32`, and so on. Rank 1 is
the default and has no prefix. Rank is fixed by the type; the number of
live elements in each dimension is a runtime property. A rank-prefixed
array stores one contiguous block of exactly the product of its
dimension counts in compact row-major order. This dense live layout is
the representation exposed through `ptr` for C interoperability; there
is no spare capacity (§2.7).

The nested spelling `array of array of T` is **allowed and has different
semantics**: it is a jagged array whose rows are independent dynamic
arrays. Use it when rows genuinely need different counts. Do not treat
it as shorthand for `2D array of T`; the two forms have distinct storage
and shape guarantees (§2.7). For a rectangular array, rank remains fixed
while dimension counts may change --- each resize reallocates the buffer
(§2.7).

A reference or view is not an array element type in 1.0.0, and this is
**transitive**: an array element type may not contain a `ref` or `view`,
even through a struct field. `array of ref of text` would copy reference
descriptors shallowly, so an array copy would alias referenced data ---
contrary to the deep-copy rule --- and `array of Holder` with a
`ref`-valued field has the same problem. A single struct with a `ref`
field remains allowed (§3.4); only _collections_ of non-owning handles
are excluded. The restriction is independent of the permission to nest
owning array types. The workarounds and the 3.0.0 deferral are recorded
in §11.2.

The rank is checked against the value where one is present --- a rank-1
declaration whose initialiser is two-dimensional is an error --- and
where the declaration states extents as well, the number of extents must
equal the rank. For a rectangular array, every nested row/plane in the
initialiser must match the declared shape:

```lale
array of i32 of 5 elements         // ✅ rank 1, one extent
2D array of i32 of 5 by 3 elements // ✅ rank 2, two extents
2D array of i32 of 5 elements      // ❌ rank 2, one extent
array of i32 of 5 by 3 elements    // ❌ rank 1, two extents
```

The prefix cannot be dropped: the rank must be expressible **without
extents** --- in a parameter, a return type, a `ref of …`, or a
non-`inline` struct field --- and there the extents are not available.
The cost is a mild redundancy, since `2D` and the extent count say the
same thing, accepted for teachability: the writer never has to decide
_when_ to write the prefix, and a mismatch is a compile error. A
zero-product rectangular shape can be declared safely without an
initializer, for example `var m as 2D array of i32 of 0 by 5 elements`;
there are no element slots to initialise.

The extent separator is `by` (`of 5 by 3 elements`, "a 5 by 3 grid").
`5x3` cannot be used: ASCII `x` is not an operator in Lale, and `×`
(U+00D7) is excluded from the grammar (`ARCHITECTURE.md §5.21`). The
glued form `2Di32` / `3Df64` is rejected as well --- the rank prefix, `array`, `of`, and element type remain separate parts
of the type spelling.

#### 2.13 Element count and shape properties

The source-level property for the total number of array elements is
**`.elements`**. The ABI descriptor still uses the field name `count` for
a rank-1 array and `count_i` for each rectangular extent; those are
internal layout names, not source-level property names. Diagnostics use
`elements` when reporting the total count, for example
`array index out of bounds (index=6, elements=5)` (§2.4).

- **`.elements`** is a read-only property carrying the `<elements>` unit.
  For rank 1 it reads the descriptor's `count` field; for rectangular
  arrays it is the product of the per-dimension counts. For a jagged
  nested array it is the number of elements in that particular array
  value (for the outer value, the number of rows, not the recursively
  flattened leaf count).
- **`.dimensions`** returns an independent rank-1 array of the current
  dimension sizes (`u64 in <elements>`). Rectangular arrays return one
  entry per rank; a partial index returns the remaining dimensions. A
  rank-1 array returns `[n]`. For jagged nesting it reports only that
  array value's own extent and does not flatten row shapes.
- `text` and `binary` keep their own fields --- `t.chars` / `t.bytes`
  and `b.bytes` --- because their element unit is a code point or a
  byte, not an array element; there is no `.elements` property on them.
- `size of` is a **runtime** operator returning `u64 in <bytes>`. It
  reports the number of bytes occupied by the value's **contents** ---
  for an array or an array view, `.elements × sizeof(T)`; for `text` and
  a text view, `.bytes` (the encoded UTF-8 byte length, not a code-point
  count); for `binary` and a binary view, `.bytes` ---
  not the descriptor. For a scalar, struct, or enum it is the
  value's layout size; for a pointer it is the target **ABI**'s pointer
  size --- an ABI (Application Binary Interface) is the exact byte
  layout and calling rules that two separately compiled pieces of code
  agree on --- which is 8 bytes on the 64-bit platforms Lale 1.0.0
  targets, generalised by `usize` / `isize` in 2.0.0 (§10); for a
  reference it is the content size of the value it refers to (for an
  array reference, `.elements × sizeof(T)`; for a `text` / `binary`
  reference, `.bytes`). It is a **shallow** size:
  the payloads of nested heap buffers are not included. A compiler may
  fold it to a constant where the size is statically known, but there is
  no separate compile-time-only form. `#size of` becomes `size of`; this
  is a semantic change for `text` / `binary` (which previously reported
  the descriptor rather than the content) and makes the operator
  runtime for a dynamic array (§13.14).

`#type of` and `#unit of` remain compile-time operators. These property
and operator changes are part of this design; the wording and operator
changes land with the array implementation (§13.14).

### 3. Value semantics and ownership

#### 3.1 Copy by default

A dynamic array **owns** its buffer --- it is responsible for freeing
that memory --- and has **value semantics**: assigning or passing one by
value produces an independent **deep copy** (the elements themselves are
duplicated, not just the small descriptor), so one variable can never
silently change another through a shared underlying buffer
(`ARCHITECTURE.md §5.14d`).

```lale
var a as array of i32 = [1, 2, 3]
var b as array of i32 = a   // deep copy — b owns its own [1, 2, 3]
b[1] = 99                   // a is unchanged
```

Because the buffer is heap-allocated by default and the copy is `O(n)`,
the compiler warns once, at the function definition, when an array
parameter may be copied at every call --- and `ref` is the way to avoid
the copy:

| Modifier | Meaning                                                          |
| -------- | ---------------------------------------------------------------- |
| _(none)_ | by value --- rvalue moves, lvalue copies                         |
| `ref`    | mutable reference --- the callee may change the caller's value   |
| `copy`   | explicit copy --- acknowledges the copy and silences the warning |

```lale
fn total(xs as array of i32) returns i32        // copy: warns (O(n) per call)
fn total(ref xs as array of i32) returns i32    // reference: no copy, no warning
fn total(copy xs as array of i32) returns i32   // explicit copy: no warning
```

A `ref` parameter can **mutate elements** of the caller's array, but it
can **not resize** it: a `grow` or `shrink` needs an owning binding
(§11.2). Resizing through a `ref` is excluded because it could
invalidate a reference or view the caller holds, across a call the
compiler cannot analyse. The exclusion covers **every** array reachable
through the parameter, including an array-valued element or struct
field: inside the callee, `grow rows[1] by 2` or `grow s.items by 2` is
a compile error when `m` / `s` is a `ref` parameter, because it could
invalidate a caller's reference to that inner array. Replacing an
element or field that owns its own buffer, directly or through a struct
field --- a jagged nested array (`rows[1] = row`), a `text`
(`xs[i] = "new"`), or a `binary` --- is likewise a compile error: it is
an element-level rebinding that would invalidate a caller's reference to
that element. A by-value (`copy`) parameter can neither mutate the
caller's value nor resize it. A **`readonly`** reference (§3.4) is the
cheap, write-forbidding counterpart.

A `ref` parameter never permits **whole-variable rebinding**
(`xs = other`): it grants element mutation only --- no rebinding and no
resizing --- uniformly across origins, so the rule does not depend on
the call site (§11.2).

Because a `ref` parameter cannot grow the caller's array, the sanctioned
accumulation idiom is **return-and-reassign**: `xs = append_one(xs)`,
where `append_one(xs as array of i32)` copies its parameter and returns
a new, larger array. Each call is an `O(n)` deep copy, so `n` appends
are `O(n²)` until a resizing `ref` (deferred, §11.1) or a later
copy-elision optimisation lands (§7.4).

**Array equality is deep, like assignment.** `a == b` compares the
counts, then the elements pairwise; `a != b` is its negation. It costs
`O(n)`, exactly as the `O(n)` deep copy of `a = b` does, so the two
operators are consistent: value semantics make both proportional to the
count. Elements are compared with their own `==` --- a nested array
compares deeply, a `text` element compares as `text`, and a float
element follows the ordinary float rule, `NaN` behaviour included. That
is the majority choice (Swift, Rust, and Julia propagate `NaN != NaN`);
Java's `Arrays.equals` and C#'s `Double.Equals` instead provide a
**total** comparison in which `NaN` equals `NaN`. A total,
`isequal`-style operation is conceivable as a later addition, but this
document promises nothing and `==` would not change. If the element type
has no `==`, the array comparison is rejected.

**Arrays inside structs.** A non-`inline` `array of T` field is an
ordinary owning value: copying or assigning the struct deep-copies the
array, and the struct's scope exit frees it. The copy cost is `O(n)` per
array field, exactly as for a bare array. An `inline` field is the
opposite --- its bytes are part of the struct, so a struct copy copies
them inline and there is no allocation (§4.1).

#### 3.2 Normative ownership and lifetime rules

These rules are the source of truth for both the **interpreter** (the
backend that runs the program's internal form directly) and the (future)
**AOT** compiler (Ahead-Of-Time: one that translates the program to
native machine code before it runs). An AOT backend cannot hide an
ownership mistake the way the interpreter's leak detector can.

| Operation                                                   | Copy or move?                                     | Who allocates the buffer                   | Who frees it, and when                                                                                              |
| ----------------------------------------------------------- | ------------------------------------------------- | ------------------------------------------ | ------------------------------------------------------------------------------------------------------------------- |
| Initialise from a literal                                   | ---                                               | the compiler                               | the compiler, at scope exit                                                                                         |
| Assign one array to another (`a = b`)                       | deep copy                                         | the compiler (for `a`)                     | the compiler, at `a`'s scope exit; `b` is untouched                                                                 |
| Pass an array by value (no modifier)                        | deep copy                                         | the compiler (for the parameter)           | the compiler, when the callee's frame is reclaimed                                                                  |
| Pass an array by `ref`                                      | no copy                                           | ---                                        | never (the callee refers to the caller's storage)                                                                   |
| Return an array (`return a`)                                | transfer, or heap copy if stack/static            | the compiler (for the copy)                | heap buffer → moves to the caller; a stack-lowered or static buffer → a heap copy is made, owned by the caller      |
| Bind a call result (`var a as array of i32 = f()`)          | move (rvalue)                                     | the compiler (the result is already owned) | the compiler, at `a`'s scope exit; no temporary is left to release                                                  |
| Reassign an owned array (`a = b`, `a` non-empty)            | deep copy                                         | the compiler (for the new `a`)             | `a`'s **old** buffer is released as part of the assignment, then `a` holds the copy                                 |
| Grow a rank-1 array (`grow a by n`)                         | existing elements moved only if it reallocates    | the compiler, if a new buffer is needed    | when the grow commits, the old buffer is released; a failed grow frees the new buffer and keeps the old one         |
| Shrink a rank-1 array (`shrink a by n`)                     | no element move                                   | ---                                        | the dropped elements are destructed (freeing any buffers they own); the array's buffer and `capacity` are unchanged |
| Grow a rectangular array (`grow dimension d of m by k`)     | surviving elements moved to the new compact block | the compiler (a fresh buffer)              | the old block is released after surviving elements are moved and dropped elements are destructed                    |
| Shrink a rectangular array (`shrink dimension d of m by k`) | surviving elements moved to the new compact block | the compiler (a fresh buffer)              | the old block is released after surviving elements are moved and dropped elements are destructed                    |
| Leave a scope                                               | ---                                               | ---                                        | every live local array is freed, in reverse order of creation, exactly once                                         |
| Receive a C buffer                                          | per FFI modifier                                  | foreign side (original); Lale (copy)       | the program, via the library's deallocator; **never Lale** for the original (see note)                              |

Notes:

- **Return: transfer, or a heap copy.** A returned array cannot be
  freed by the callee (it would be lost). If its buffer is on the
  **heap**, ownership moves to the caller with **no copy**. If the
  buffer was placed on the **stack** by the count-never-changes
  lowering (§7.3), it cannot outlive the frame, so the return
  **materialises a heap copy**. If the buffer is a module-level
  array's **static** storage (§2.6), returning it also **materialises
  a heap copy** --- static storage must never be handed to the heap
  deallocator --- and the global binding is **never moved-from**.
  Either way the caller receives an owned, heap-backed array --- the
  only form that can outlive the callee.
- **Growth and static storage.** A module-level array that `grow`s
  copies its elements from static storage into a new heap buffer; the
  static storage is never freed (see the static-storage bullet below).
- **Moved-from.** After an ownership transfer (such as `return a`),
  the source `a` is **moved-from**: it must not be read, and scope
  cleanup must not free it. Reading it is a **compile error** (a
  definite-assignment-style linear pass, §11.2). Reinitialising it
  makes it usable again. (Move semantics for transfer coexist with
  deep-copy assignment: ordinary `a = b` still copies.)
- **Freeing the received value.** The caller owns the received array
  and frees it at its own scope exit --- or immediately if it
  reassigns the variable that holds it. A returned value used directly
  as an expression (an rvalue) is released at the end of the statement
  it appears in, unless it is bound to a variable --- binding moves it
  (see below).
- **Binding a call result --- rvalue moves, lvalue copies.**
  `var a as array of i32 = f()` binds an **rvalue** (a temporary value
  with no name --- a call result, a conditional expression, or another
  temporary), so the temporary is **moved** into `a`: one allocation,
  no copy, and nothing left to release. Binding an **lvalue** (a named
  place --- a variable, an element, a field) still copies:
  `var a as array of i32 = b` deep-copies `b`, and
  `var a as array of i32 = m[1]` copies the stored row. A literal is
  constructed directly in `a`. This is the ordinary value-semantics
  rule and matches the move already performed on `return`; allocation
  counts are therefore deterministic.
- **Passing by value --- rvalue moves, lvalue copies.** Passing an
  array argument to a by-value parameter (no modifier) follows the
  same rule as binding: an **rvalue** argument --- a call result, a
  `copy of …` expression, or another temporary --- is **moved** (one
  allocation, no copy), while an **lvalue** argument (a variable,
  element, or field) is deep-copied. This is what makes
  `sum(copy of s.values)` a single copy: `copy of s.values` produces a
  temporary, and the temporary moves into the parameter. The compiler
  warns when an explicit `copy of` is **redundant** --- for example
  `copy of` an owning value that is then bound to a variable and
  passed by value --- so an avoidable second copy is visible rather
  than silent.
- **Reassignment order.** For `a = b`, the compiler evaluates the copy
  of `b` first, releases `a`'s old buffer, then stores the copy. This
  makes `a = a` safe.
- **Reassignment --- rvalue moves, lvalue copies.** `a = b` with an
  **lvalue** `b` deep-copies (the "Reassignment order" rule above);
  `a = f()` with an **rvalue** `f()` **moves** the call's result into
  `a` --- no copy, and `a`'s old buffer is released --- symmetric with
  binding (§above). This keeps the accumulation idiom
  `xs = append_one(xs)` at one copy per call, not two (§3.1).
- **`ref` duration.** A `ref` array parameter refers to the caller's
  storage only for the duration of the call; the callee must not
  retain it afterwards (the per-function escape rule,
  `ARCHITECTURE.md §5.14`). It may mutate non-replacing elements but
  may not `grow` / `shrink`, rebind, or replace an element or field
  that owns its own buffer (§3.1).
- **A foreign buffer is never freed by Lale.** For `ref` /
  `readonly ref`, the program releases the original with the library's
  own function; for `copy`, Lale owns and auto-frees its **copy**,
  while the original stays with the foreign side that retained it ---
  the program must not free it (§5.2).
- **Static storage is not heap memory, and each payload remembers
  which it is.** When the compiler places a buffer in **static
  storage** --- a module-level array (§2.6) --- that storage must
  **never** be handed to the heap deallocator; it is released, if at
  all, only with the program's data segment. Because a global's
  initialiser is a compile-time constant, its **payloads are static
  too**: a jagged nested array's row buffers, and the bytes of a
  `text` / `binary`, are baked into the program file --- they are not
  heap allocations. The rule that keeps this safe under later mutation
  is that **each owning value records its buffer's ownership** ---
  heap-owned (freed by its destructor) or **non-heap** (never freed by
  the destructor: static storage or a stack-lowered buffer) --- as
  **out-of-band runtime metadata**, _not_ as a field of the array
  descriptor (§7.1): the descriptor stays `{ ptr, rank, count, capacity }`,
  and the ownership marker travels alongside the value in the compiler's
  and runtime's internal representation. The interpreter already does
  this --- a literal string is a `Value::String`, a heap buffer a
  `Value::Pointer` --- and the AOT backend keeps the same distinction
  in its own metadata. A stack-lowered buffer (§7.3) is one _non-heap_
  case: it is an **optimisation** of a local's constant-count buffer,
  not a third semantic ownership state. Its scoped lifetime is
  guaranteed separately --- the lowering applies only when the data
  pointer does not escape the frame (§7.3), and returning such an
  array materialises a heap copy --- so the metadata needs only the
  two states, _free it_ (heap) or _do not free it_ (everything else).
  A `grow` that moves the outer buffer to the heap copies the element
  descriptors verbatim, so each element keeps its own provenance;
  recursive destruction frees only heap-owned payloads. This is the
  same distinction `text` / `binary` already need between literal
  bytes (static) and bytes produced at runtime (heap). An array (or
  one of its inner arrays) that later needs a different buffer first
  **transitions to owned heap storage** and is an ordinary owned array
  from then on. Where the bytes live stays an implementation detail;
  these ownership consequences are the observable part.

#### 3.3 Local vs global lifetime

A local array lives until the end of the function that created it; a
module-level array lives until the program ends. Deep copies make this
simple: because `a = b` copies, `b` must stay valid only until the end
of its own scope --- the copy in `a` is independent and survives `b`.
Destruction is "reverse order of creation, exactly once" (§3.2). An
owning cycle is impossible by construction: a type that contains itself
by value is an infinitely-sized type and is rejected, and the non-owning
indirections (`ref`, `pointer`) own nothing, so there is no owning cycle
to destruct.

#### 3.4 The reference type: `ref of T`, `ref to x`, `readonly`

A **reference** is Lale's non-owning value for a count-carrying type: it
_points at_ an array (or a `text` / `binary`) that someone else owns. It
carries the **same descriptor as its source type**, but it owns nothing:
making one copies no elements, and the compiler never frees it. For a
rank-1 array that descriptor is `{ ptr, rank, count, capacity }`; for a
rectangular array it is `{ ptr, rank, count_1, …, count_r }` (no
capacity, §7.1); for `text` it is `{ ptr, bytes, chars }`; for `binary`
it is `{ ptr, bytes }`. References to `text`, `binary`, and rectangular
arrays have no `capacity`. References are the 1.0.0 half of non-owning
access (§4.2, §8.3); the _ranged_ **view** is deferred (Part II).

**The type.** A reference is written `ref of T`, with the **source
type** after `of` --- the same constructor-family form as `array of T`
and `view of T` --- and, like `array of`, it contains no brackets
(§2.9):

```text
ref of array of i32   // a reference to an i32 array
ref of array of f64
ref of text           // a reference to a text
```

The source type after `of` must be a **count-carrying** type, so
`ref of ref of T` is **rejected in 1.0.0** --- a pure restriction,
mirroring `view of view of T` (§8.6): the element of `ref of` may not
itself be a `ref`. The diagnostic names the fix, and allowing it later
is additive.

**The value.** A reference to an array, `text`, or `binary` that Lale
already owns is created with `ref to`, the analogue of `pointer to`:

```lale
var xs as array of i32 = [1, 2, 3]
var r as ref of array of i32 = ref to xs   // no copy; r points at xs itself
write r[2]                                 // reads xs[2] through the reference

var s as text = "abc"
var rs as ref of text = ref to s           // the same form works for text and binary
```

Reading an `inline` field (§4.2) or binding an FFI `ref` return (§5.2)
**produces** a reference without `ref to`; `ref to` is the explicit form
for an owning array, `text`, or `binary`. `ref to m[i]` on a rectangular
array creates a **direct** reference into the frozen payload --- there is
no row descriptor to follow --- and a rank-1 result reports the synthetic
`capacity == count` (§8.3).

`ref to` requires an **addressable, stable storage location**: a
variable, a stored field, or a stored array subobject (`m[i]`) are valid
origins; an **rvalue** --- a function result, an array literal, or any
temporary --- is rejected, because the temporary is released at the end
of the expression and the reference would dangle. This is a semantic
rule, not a syntactic one: the grammar parses `ref to <expression>`, and
the analyser rejects an expression that does not designate stored
storage (§13.12).

**`ref` parameter versus `ref of T` value.** These are two different
things. `fn f(ref xs as array of i32)` passes the caller's array **by
reference**: `xs` is an alias, element writes go to the caller's
storage, and the callee may not rebind or resize it (§3.1).
`fn g(xs as ref of array of i32)` passes a reference **value**: the
descriptor is copied, element writes still go through the copied `ptr`, and
rebinding `xs` inside `g` changes only the local copy. A caller supplies
a reference value explicitly (`g(ref to xs)`); an owning array is not
implicitly converted to a reference.

**`copy of x` --- the reverse of `ref to x`.** Neither direction is
implicit. `ref to x` makes a reference from an owning value; `copy of x`
makes an **owned copy** from a reference (or from an owning value --- an
explicit deep copy). The two are mirror images: `ref to` points _at_ a
value without owning it, `copy of` copies _from_ a value into owned
storage. `copy of` is the **call-site** counterpart to `ref to`,
distinct from the `copy` parameter modifier (§3.1): where
`sum(ref xs as array of i32)` lets a caller hand over an inline field
without copying, `sum(copy of s.values)` lets a caller materialise an
owned copy **without touching `sum`'s definition** --- exactly the
spelling a `ref of T` argument needs when the parameter is an owning
`array of T`, because that conversion is otherwise a compile error
(§4.2, §13.12). Because a reference is always whole-range, `copy of` is
a copy, not a shape change; materialising a ranged _view_ (a shape
change) stays deferred (§8.4).

**Read-only references.** `readonly` is a **capability modifier** on a
reference (or a view): it says the holder may not _write_ through it ---
no element writes, no `grow`, no `shrink`. It replaces the earlier
FFI-only `const ref`, and it introduces no `const` keyword anywhere.

```lale
var ro as readonly ref of array of i32 = ref to xs   // ro is read-only; xs may still change
```

`readonly` names a **permission the holder is denied**, not a property
of the data: `xs` may still be changed through `xs` itself, through
another (writable) reference, or --- at the FFI boundary --- by the
foreign side (§5.3). It is therefore more honest than `const`, which
would suggest the data cannot change. Accepting a _declared_ capability
re-opens `ARCHITECTURE.md §5.2`'s "infer, don't declare" principle; the
justification is recorded in §9 and the decision in §11.2.

**Descriptor properties are read-only.** The address and shape properties
exposed to Lale code --- `text.ptr`, `text.bytes`, `text.chars`,
`binary.ptr`, `binary.bytes`, and an array's `ptr`, `rank`, `.elements`,
`.dimensions`, and (rank 1) `.capacity` --- may be read but not assigned:
`s.ptr = p` and `xs.elements = n` are compile errors. The ABI fields
`count` and `count_i` are not separate source properties; the compiler
owns the descriptor. This is a
targeted rule for the compiler-managed descriptors, not a general
`readonly` field modifier; enforcement is a work item (§12).

**Variance.** A writable reference may be passed or assigned to a
`readonly ref` parameter or variable (the restriction is added); a
`readonly ref` may not be passed to a writable `ref` parameter or
assigned to a writable reference. The same rule applies to views. A view
taken through a `readonly ref` is itself `readonly`; a writable view
cannot be obtained from a read-only reference.

**Why a reference and not a raw `pointer`?** A raw pointer carries no
length; a reference carries its source type's descriptor --- `rank` and
`count` / `capacity` for a rank-1 array, `rank` and per-dimension counts
for a rectangular array, `bytes` / `chars` for `text`, `bytes` for
`binary` --- so the length field, indexing, and bounds checking work through it. That is what earns the reference its place for the
**count-carrying** types. A reference is the **direct** non-owning form;
a ranged **view** (§8.3) is the **descriptor-following** form, holding a
pointer to the origin's descriptor (a pointer to a pointer), so it survives
growth at the cost of one extra indirection. The two are deliberately
different concepts.

**Escape.** A reference **escapes** when it is allowed to outlive the
storage it points at --- a dangling hazard. In 1.0.0 a reference **never
escapes the function that creates or receives it**. The compiler rejects
four things, all per-function and syntactic, extending the raw-pointer
escape rule (`ARCHITECTURE.md §5.14`):

- **Returning or globally storing a reference to a local.** A
  reference to a local array, local `text`, or local `binary` ---
  directly, or through a field of a local struct (an `inline` field or
  an ordinary array / `text` / `binary` field) --- must not be
  **returned** or **stored in a global**, because the local's storage
  is reclaimed when the function returns.
- **Returning or globally storing a reference to a global.** A global
  lives for the whole run, so this is not an immediate dangling
  hazard. The problem is subtler: a later `grow` / `shrink` /
  reassignment of that global, in some _other_ function or module, would
  invalidate the escaped reference, and the per-function freeze cannot
  see an operation in another function. 1.0.0 therefore forbids this
  too (§11.2). More broadly, taking a reference to a global whose
  referenced storage may be **resized or replaced**, or whose binding is
  **shared across modules**, is a compile error: only a module-private
  global whose referenced storage (including nested sub-objects) is
  provably never resized or replaced may be borrowed (§11.2), because
  even a non-escaping local reference can be invalidated by a call or
  another module that resizes the global.
- **Returning a reference received as a parameter.** A `ref of T`
  parameter points at the caller's storage, which the callee cannot
  see, so its origin may or may not outlive the callee. Returning it
  is rejected (§11.2).
- **Returning or globally storing an FFI-origin reference.** A
  reference produced by an FFI `ref` / `readonly ref` return (§5.2) has
  an origin --- a foreign buffer --- whose lifetime and reallocation are
  outside the compiler's control, so it is treated like the other
  origins: returning or globally storing it is a compile error, even
  though the foreign buffer might still be alive. This is the same
  "not locally proven" rule as §11.2.

The check is **transitive through `type` values**: a struct that
contains such a reference is itself marked, so returning that struct or
storing it in a global is a compile error. §8.5 applies the same
boundary to ranged views.

These four prohibitions are deliberate restrictions, not the final
word. A later version may relax them with **provenance / lifetime
analysis** --- tracking a global's outstanding references across
functions, or proving that a parameter's origin outlives the callee.
Relaxations of this kind are **additive**: every program valid under the
1.0.0 rules stays valid, because relaxing only admits programs that were
previously rejected. So 1.0.0 errs on the side of safety (§11.2).

**Freeze.** A reference holds a **copy** of the origin's descriptor ---
`ptr`, `rank`, `count`, `capacity` for a rank-1 array (`ptr`, `rank`,
and per-dimension counts for a rectangular array); `ptr`, `bytes`,
`chars` for `text`; `ptr`, `bytes` for `binary` --- not a pointer to the
descriptor. So
once a reference to an array is **taken**, that array must not be
`grow`n, `shrink`n, or reassigned for the rest of the function --- the
freeze is conservative and keyed on the `taken` mark (§8.5), not on
whether the reference is still used: any of the three would leave the
reference with a stale count or a released buffer. If the reference
refers to an **element or field** that owns its own buffer (a `text`, a
`binary`, or a nested array), that element or field must not be
reassigned either --- the element-level freeze of §11.2. For `text` and
`binary`, reassignment is the invalidating event. All of these are
**compile errors** (the `taken` mark of §8.5), and the text/binary rule
is part of **Lale 1.0.0**. Because a reference never escapes (§above)
and a global whose storage may be resized or replaced, or whose binding
is shared, cannot be borrowed (§11.2), the freeze only has to hold for
the rest of the function that took the reference; there is no
cross-function invalidation to track in 1.0.0.

The `taken` mark is set only when a reference or view is **bound** ---
stored in a variable, a parameter, or a field --- so it can be used
after the expression that created it. A **transient** reference created
and consumed within one expression (`sum(copy of s.values)`,
`g(ref to x)`) does **not** set the mark: it cannot outlive that
expression, and the origin cannot be resized or reassigned inside an
expression (those are statements, not expressions), so there is nothing
to freeze.

The freeze is **structural**: writing an element --- `xs[i] = v`, or
`r[i] = v` through the reference --- stays allowed, because it neither
moves the buffer nor changes the count. The exception is a reference to
an element that itself owns a buffer (`xs[i]` a `text` / `binary` /
nested array): reassigning that element is a **compile error** under the
element-level freeze (§11.2). A ranged view is less restrictive, because
it follows the descriptor and therefore **survives growth**; it freezes only
`shrink` and reassignment (§8.5).

**Scope.** In 1.0.0 a reference exists only for those count-carrying
types. Extending `ref of T` to _all_ types (`ref of i32`,
`ref of Sample`) is left open (§11.1); it would be additive, not
breaking.

#### 3.5 The operation matrix

The table below is the single reference for what each kind of value may
do. Its three columns are: an **owning array** (`array of T`, which owns
and frees its buffer), a **writable reference** (`ref of T`), and a
**read-only reference** (`readonly ref of T`). "Rebinding the variable"
means pointing the handle somewhere else; "reassigning the origin" means
changing the array the handle refers to. Those two are different, and
the difference is the place most likely to confuse a reader.

| Operation                                                                 | Owning array (`array of T`)                                                    | Writable reference (`ref of T`)                                                                             | Read-only reference (`readonly ref of T`)                    |
| ------------------------------------------------------------------------- | ------------------------------------------------------------------------------ | ----------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------ |
| Read elements (`x[i]`)                                                    | yes                                                                            | yes (through the copied pointer)                                                                            | yes                                                          |
| Write elements (`x[i] = v`)                                               | yes                                                                            | yes (writes through the pointer)                                                                            | **forbidden** through the holder                             |
| `grow` / `shrink`                                                         | yes, subject to §2.8 / Annex B                                                 | **forbidden** --- needs an owning binding                                                                   | **forbidden**                                                |
| Reassign the whole binding                                                | yes --- deep copy (`a = a` is safe)                                            | rebind the _variable_: yes (local); reassign the _origin_: **forbidden** while it lives (freeze, §3.4)      | same distinction                                             |
| Replace an owned-buffer element / field (`xs[i] = row`, `s.name = "new"`) | yes (mutation target)                                                          | **forbidden** (element-level freeze, §11.2; a `ref` parameter never replaces an owned-buffer element, §3.1) | **forbidden** through the holder                             |
| Pass by value (no modifier)                                               | deep copy, `O(n)`                                                              | descriptor copy, `O(1)`; writes visible to the caller                                                       | descriptor copy, `O(1)`; writes forbidden through the holder |
| Pass by `ref` (parameter modifier)                                        | yes --- element mutation only (no rebind, resize, or owned-buffer replacement) | **error** --- `ref` on a `ref of T` parameter is not meaningful (§13.12)                                    | **error** --- same                                           |
| Return, or store in a global                                              | yes (heap value moves / copies; stack or static value copies to heap)          | **forbidden** --- a reference never escapes (§3.4)                                                          | **forbidden** --- same                                       |
| Destroy at scope exit                                                     | frees its buffer (skipping non-heap payloads, §3.2)                            | frees nothing (the origin still owns it)                                                                    | frees nothing                                                |

The **owning-array column assumes no live reference or view**: taking
`ref to` an array, or a ranged view of it, freezes `grow`, `shrink`, and
reassignment of that array for the rest of the function (§3.4, §8.5),
even after the reference or view is no longer used. The cells marked
"yes" above are the un-frozen baseline.

Passing a `ref of T` value to an owning `array of T` parameter is a
**compile error** --- neither direction converts implicitly (§3.4). The
explicit spellings are `copy of r` at the call site (an owned copy,
`O(n)`) or a `ref` parameter at the definition (no copy); §4.2 shows
both.

A reference may be **assigned between reference variables** (`r2 = r1`):
that rebinds `r2`'s handle to `r1`'s origin and is the "rebind the
variable" cell above. The freeze stays conservative: the origin `r2`
used to point at remains `taken` until the end of the function, even if
no reference to it is used again (§8.5).

### 4. Struct arrays: the `inline` field

C stores an array inside a struct as **inline bytes**
(`struct { int coords[3]; }` is 12 bytes, not a pointer). A dynamic
array is a 32-byte descriptor (at rank 1), so a naive `char[14]` field
would put a descriptor where C expects 14 inline bytes --- and there would be no pointer
to hand across the C boundary. The `inline` modifier reconciles the two.

#### 4.1 `inline array of T of n elements` --- a fixed inline field

`inline` is a **field-only** type modifier, allowed **only inside a
`type` definition** (never in a `var`, a parameter, or a return type ---
a hard grammar rule, not a convention):

```lale
type Sample
    t as f64
    values as inline array of i32 of 3 elements
end type
```

`inline array of i32 of 3 elements` lays out three **contiguous** inline
`i32`s (12 bytes) with the alignment of `i32` --- byte-for-byte
identical to the C array **member** `int values[3]`. The extent must be
a **compile-time constant**; a runtime count is the deferred flexible
form below. The **enclosing** struct is C-compatible only when its
**complete** layout --- field offsets, padding, alignment, and every
field's type --- is verified against the target ABI (§13.3, §13.5), not
merely because one field is an inline array.

`inline` is the **enforced** form of the layout optimisation of §7.3:
the count is fixed, and the buffer is the field's own bytes rather than
a heap allocation. It exists only for the C FFI context --- it is
field-only, and its purpose is to reproduce a C array member --- so a
Lale programmer cannot use it to force stack allocation of an ordinary
variable. The field may also carry a **rank**, and then it is the one
rectangular array form in Lale:
`inline 2D array of i32 of 5 by 3 elements` is a fixed C `int[5][3]`
member. Its rectangle is part of the ABI, so there is nothing to grow
(§2.7).

The element type must have a defined C layout: `i8` / `i16` / `i32` /
`i64`, `u8` / `u16` / `u32` / `u64`, `f32` / `f64`, `byte`, `bool`,
`pointer`, or a `type` whose own layout is verified C-compatible. `char`
(4 bytes --- a Unicode code point, not C's 1-byte `char`), `f16`, `text`
/ `binary`, `T?`, and dynamic arrays are excluded (§11.2).

The **flexible** form, `inline array of T of len elements`, is a C99
flexible array member: a trailing field whose element count is read from
another (named) field `len` at construction. It is **deferred** (section 10) --- it needs variable-sized-struct semantics.

#### 4.2 Reading an `inline` field yields a reference

Reading an `inline` field produces a **reference** --- a value of type
`ref of array of T` (§3.4) --- a non-owning, whole-range view of the
inline bytes. A reference is part of **Lale 1.0.0** (it is what `inline`
fields and FFI `ref` returns produce); only the _ranged_ view of Part II
is deferred. The same type is what `ref to x` yields for an owning array
(§3.4). Its `.elements` is `n` (the rank-1 descriptor's internal `count` is `n`)
and its `ptr` addresses the inline bytes. No copy occurs, and the ordinary
array operations --- `.elements`, indexing,
iteration --- apply unchanged:

```lale
fn sum(xs as array of i32) returns i32
    var total as i32 = 0
    loop var i as u64 from 1 to xs.elements
        total = total + xs[i]
    end loop
    return total
end fn

// s is built elsewhere; s.values holds three inline i32s (a ref of array of i32)
write sum(s.values)            // ❌ compile error: ref of array of i32 → array of i32 is not implicit
write sum(copy of s.values)    // ✅ owned copy of the three inline elements (O(n))
```

The same `sum` works on any origin, but the conversion must be explicit
--- `s.values` is a `ref of array of i32`, and there is no implicit
conversion to the owning `array of i32` parameter (§3.4) --- so
`copy of s.values` materialises an owned copy, while §4.3's `ref`
parameter passes the inline bytes with **no copy**.

#### 4.3 `ref` over an `inline` field

Because the field is inline bytes rather than a descriptor, `ref` binds a
temporary reference descriptor:

```lale
fn sum(ref xs as array of i32) returns i32
    // no copy: xs refers to s's inline bytes
    ...
end fn
```

The compiler (1) takes the field's address, (2) materialises a reference
descriptor `{ ptr = address of s.values, count = 3, capacity = 3 }` on the
stack, and (3) binds `xs` to it. The result is **mutable elements, not a
reassignable variable**: `xs[i] = v` writes through the reference descriptor
into the inline bytes, but reassigning the whole array (`xs = other`)
would write to the temporary and be lost, so it is rejected. Resizing is
likewise unavailable for a reference: `grow` and `shrink` require an
**owning** binding (§3.1), and a reference is never one, so any attempt
to resize through a reference is a **compile error**, regardless of
where the reference came from.

**Freeze.** A reference into an `inline` field points at the struct's
own bytes, so reassigning the containing struct while the reference is
live is a **compile error**, and so is reassigning the field itself
(`s.values = other`) --- either would overwrite the bytes. If the struct
is an element of an array, taking the reference also marks the root
array as `taken` (§8.5). Element writes through the reference remain
allowed.

#### 4.4 Bounds checking, without `unsafe`

Both `inline` forms get the ordinary runtime bounds check
(`ARCHITECTURE.md §5.12`): the fixed form checks against the extent `n`;
the flexible form (deferred, section 10) checks against a runtime field.
The only thing the check cannot catch is a count field that overstates
the real buffer --- the same "corrupted count" hazard a `text.bytes`
field can present. The descriptor fields are **read-only** (§3.4), so safe
Lale code cannot change the count; an `unsafe` write or a foreign buffer
that lies about its extent remains the programmer's responsibility, not
a reason to mark the field `unsafe`.

### 5. The C FFI boundary

An **FFI** (Foreign Function Interface) is the point where Lale code
calls, or is called by, code written in another language --- here, C. At
this boundary Lale's automatic rules (ownership, bounds checking, the
descriptor) no longer apply, and the programmer must state how a
foreign pointer is owned and freed.

#### 5.1 Ownership and the `C` boundary

A C function can hand Lale a pointer that Lale must **not** free, or one
that Lale **must** free --- and there is no way to tell from the pointer
alone. Since a Lale dynamic array is **owning and auto-freed**,
installing a foreign pointer in the descriptor without taking ownership
would make Lale free a buffer it does not own (and the descriptor fields are
read-only, §3.4). Ownership at the boundary is therefore **explicit,
with no default**, using the existing `ref` / `copy` keywords and the
`readonly` capability:

| Modifier       | Meaning at the FFI boundary                                                |
| -------------- | -------------------------------------------------------------------------- |
| `ref`          | **reference** --- non-owning; Lale never frees the buffer                  |
| `copy`         | **own a copy** --- deep-copy into a Lale-owned buffer that Lale auto-frees |
| `readonly ref` | **reference, read-only** --- like `ref`, plus "may not write through it"   |

`readonly` is the general **capability modifier** of §3.4 (it forbids
writes through the holder); at the FFI boundary it carries the same
meaning, mirroring C's `const`.

An array **return** must carry one of the three --- omitting the
modifier is a compile error, because the compiler cannot infer who owns
the buffer:

```lale
// Non-owning: Lale never frees the buffer.
import fn signature ints_view(n as u64) returns ref of array of i32 of n elements

// Owning: Lale deep-copies, and frees the copy at scope exit.
import fn signature ints_copy(n as u64) returns copy array of i32 of n elements

// Non-owning and read-only.
import fn signature ints_ro(n as u64) returns readonly ref of array of i32 of n elements
```

The declaration is a **claim about the foreign code**, not something the
compiler can verify: the programmer must read the C documentation and
state who frees the buffer. Every language with an FFI puts that burden
on the binding author --- C, Rust, Go, and Swift all require the binding
to encode the foreign ownership contract.

Beyond **who frees**, a foreign declaration also states **how long the
buffer stays usable**. A `ref` or `readonly ref` to a foreign buffer is
valid only while the foreign allocation remains **alive, at the same
address, and with the declared extent**. `readonly` does not change this
--- it forbids writes through the Lale handle, not foreign reallocation
or release --- so the library must keep the buffer stable for as long as
Lale uses the reference, and the binding author is responsible for that
guarantee (§5.2).

The FFI boundary is spelled with the **`C` modifier**. Lale has **two
export forms**, because the two audiences have different ABIs (the
byte-level layouts and calling rules each audience expects):

| Form          | Audience  | ABI                         | `array of T` parameter? | Symbol name  |
| ------------- | --------- | --------------------------- | ----------------------- | ------------ |
| `export fn`   | Lale code | Lale ABI (array descriptor) | yes                     | mangled      |
| `export C fn` | C code    | C ABI (pointer + count)     | no                      | plain C name |

`C` is a **contextual modifier**, meaningful only immediately after
`export` --- consistent with Lale's "no reserved words" rule. It marks
the **C ABI** and ties the symbol name to it: Lale-internal names are
**mangled** (the compiler rewrites them into a unique encoded form),
while C names are left plain (`ARCHITECTURE.md §4.6`). The import side
carries no marker, because `import fn signature` is already
unambiguously the C → Lale form (`lale.md` § _Importing C Function
Signatures_): there is no second thing an `import fn signature` could
mean. The asymmetry is deliberate --- `C` disambiguates an **export**,
which would otherwise read as Lale-to-Lale.

```lale
// Lale-to-Lale export — the Lale ABI; a dynamic array parameter is fine
export fn total(xs as array of i32) returns i32
    var sum as i32 = 0
    loop var i as u64 from 1 to xs.elements
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
write c_sum(xs.ptr, xs.elements)
```

The call has the same arity as the declaration --- one argument per
declared parameter --- so the ordinary argument-count rule applies;
nothing is silently expanded. `xs.ptr` is the address of the element
buffer and `xs.elements` is its element count.

**A unit note.** `xs.elements` carries the unit `<elements>`, while the
imported parameter `count as u64` declares no unit. Unit checking
follows `lale.md`: a mismatch against a **declared** unit is an
**error**, in a Lale-defined function and in an import alike. A
parameter that declares no unit has no declared unit to mismatch; for an
**imported** parameter the compiler emits a **warning** instead, because
a foreign declaration carries no unit information and the binding author
is responsible for the match. To make the contract checked, declare the
unit in the import (`count as u64 in <elements>`) and a mismatch becomes
an error. The same warning applies to `text.bytes` (`<bytes>`) and
`binary.bytes` passed to unitless C parameters.

**Multidimensional FFI.** For a rank-prefixed rectangular array, `m.ptr`
addresses one contiguous row-major `T` buffer. A C function expecting
`T *` may receive it only when the declared C signature and explicitly
supplied extents agree. The array descriptor is not automatically
expanded into pointer and dimensions. A jagged `array of array of T` is
not a flat `T` buffer and must not be passed as one --- a binding passes
each row's `ptr` and count separately, for example a loop that fills a C
`const T**` and a parallel `size_t` length array.

The ownership marking (`ref` / `copy` / `readonly ref`, above) applies
to a value that **crosses from C into Lale**: an `import fn signature`
**return value**. An `export C fn` parameter is also C → Lale data, but
it is a bare `pointer` + `count` --- an array cannot cross the C ABI ---
so it has no spelling for these modifiers: the C caller owns the buffer,
and Lale must not free it.

Ownership stays with the allocating side in the other direction too.
Lale owns `xs` and only lends its data pointer for the duration of a
call, so C must not retain it; the C caller owns `data`, and Lale must
not free it.

**Ownership is visible in the value.** A reference result (`ref`) stays
**non-owning**: it cannot be bound to an owning `array of i32`, because
the compiler would then free a buffer it does not own. Taking ownership
requires the explicit `copy`.

#### 5.2 Return values

**Case A --- the import is a bare pointer (no array-ness, no count):**

```lale
// C: int* make_ints(size_t n);   // malloc'd; the caller owns and frees it
import fn signature make_ints(n as u64) returns pointer

var p as pointer = make_ints(5 as u64)
free(p)   // you own it, free it
```

Here `free` is correct only because this buffer came from the **platform
allocator** --- the system's memory manager, the one `malloc` and `free`
talk to --- as the `malloc'd` comment states. For a buffer from a
library with its **own allocator** (its own matching pair of
allocate/release functions), `free` is wrong --- the library's release
function is the only valid one (see the prominent example below).

**Case B --- the import returns an array.** Two ownership modes. Unlike
Case A, the two array declarations use **different foreign contracts**,
because `copy` is only valid when the foreign side retains the original
(§11.2).

_Own a copy (`copy`) --- the 1.0.0 path:_ the foreign side is a **getter
that retains ownership** of the buffer it hands back.

```lale
// C: const int* lib_ints(size_t n);   // pointer into library-owned storage; the library retains it (caller must not free)
import fn signature lib_ints(n as u64) returns copy array of i32 of n elements

var xs as array of i32 = lib_ints(5 as u64)   // a Lale-owned deep copy; the library still owns the original
write xs[1]
```

`copy` deep-copies the foreign buffer into Lale-owned memory; `xs` is an
ordinary owning array. Because the library retains the original, Lale
**must not** free it, and neither must the program --- the library's own
release, if any, is the only valid one. A **transferred** allocation
cannot be handled by `copy`: the program would hold only the copy and no
pointer with which to release the original. `copy` is therefore
restricted to the case where the foreign side **retains** the original
(§11.2).

**Not to be confused with `copy of` at a call site.** The `copy` here is
a **return-mode modifier** on an `import fn signature` --- it declares,
once, that the foreign function returns a buffer whose original the
library retains, so Lale makes and owns a copy. `copy of x` (§3.4) is a
**call-site expression** that materialises an owned copy of a reference
(or of an owning value) at a particular call. The two are distinct: the
first is a property of a foreign declaration, the second an operator on
a value.

_Reference (`ref`) --- non-owning:_

```lale
// C: int* lib_alloc_ints(size_t n);   // library-allocated; release with lib_free, not free()
import fn signature lib_alloc_ints(n as u64) returns ref of array of i32 of n elements

var xs as ref of array of i32 = lib_alloc_ints(5 as u64)   // non-owning reference; count = 5
write xs[1]
lib_free(xs.ptr)   // the program frees the ORIGINAL with the library's own deallocator
```

`ref` is non-owning: Lale never frees the buffer. The non-owning binding
type is `ref of array of T` (§3.4). `lib_free` here stands for the
library's **own** release function --- never `free` (see the prominent
example below).

Here the extent clause `of n elements` is not a type-level count --- the
array type carries no count --- it is an FFI-import **count binding**:
the returned array's `count` is the runtime value of the `n` parameter.

A foreign declaration is a claim about the C code, which the compiler
cannot verify. One claim it _can_ check cheaply: for a non-empty buffer
(`n > 0`), a **null** pointer handed back by `ref` / `copy` /
`readonly ref` is a contract violation, not a valid array. The runtime
rejects it with a clear error instead of reading through null. The same
check applies to `text` and `binary` returns.

**Prominent example --- a library with its own allocator.** Many C
libraries do not use the platform allocator, so `free` is simply
**wrong** for their buffers: the library provides the only function that
may release them. Lale therefore never releases a foreign buffer --- the
program must, with the library's own function:

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

This is why foreign-buffer deallocation is a **requirement**, not an
open point: a language that insisted on freeing the original itself
could not use any library with a custom allocator.

**Lifetime after release.** A foreign deallocator call (`fftw_free`,
`lib_free`) invalidates **every** reference and alias to that buffer.
After it, the reference value still exists syntactically --- and copies
of it may exist --- but using any of them is **undefined behaviour**:
Lale does not auto-invalidate references to foreign storage, and the
binding author must ensure no alias is used after release. Lale follows
its existing raw `allocate` / `release` rule here (`lale.md` § _Memory
Safety_): obvious double-release and use-after-release of a foreign
buffer are a **flow-insensitive warning**, not a guarantee --- it does
not catch aliased or inter-procedural misuse. A future **consuming
release** that consumes the reference so the compiler can reject
use-after-release by definite assignment is deferred (§11.3), not an
`unsafe`-keyword change.

**Text and binary returns.** The extent clause uses the type's element
unit: bytes for `binary`, code points for `text`. Both ownership modes
are available, exactly as for an array: `ref` is non-owning, `copy`
deep-copies into Lale-owned storage.

```lale
import fn signature read_buf(n as u64) returns ref of binary of n bytes
import fn signature read_buf_copy(n as u64) returns copy binary of n bytes

import fn signature read_text(n as u64) returns ref of text of n chars
import fn signature read_text_copy(n as u64) returns copy text of n chars
```

`ref of binary of n bytes` is a non-owning reference whose `ptr` is the
foreign buffer and whose `bytes` is the runtime value of `n`; Lale never
frees it. `copy binary of n bytes` copies exactly `n` bytes into a
Lale-owned buffer that is auto-freed at scope exit. The same restriction
as for an array applies: `copy` is for a getter that keeps the original,
because a transferred allocation would leave the program without a
pointer with which to release it (§11.2).

`text` counts code points, so its **descriptor** --- the small structure
that describes a buffer, its pointer plus its lengths --- is spelled in
chars. The runtime scans from `ptr` until it has counted `n` code
points, sets `bytes` to the number of bytes consumed, and checks the
`text` invariant --- a **null terminator** (a zero byte that marks the
end of a C-style string) at `ptr[bytes]`. The existing
`__lale_construct_text_in_place` helper performs this scan and returns
an owning `text`, so it is the natural implementation of
`copy text of n chars`; the reference path needs the same scan to fill a
reference descriptor (or a sibling helper returning `ref of text`). The scan
is `O(bytes)`; it counts code points but performs **no implicit UTF-8
validation** --- the binding author must ensure the bytes are valid. A
missing terminator or fewer than `n` code points is a runtime failure.
Because the scan reads until it has counted `n` code points **before**
checking the terminator, a buffer that is too short --- a missing
terminator, or a hostile or buggy source that understates its length ---
is read **out of bounds during the scan**, before the failure fires. The
`text` scan is therefore only for **trusted** sources whose terminator
the binding author has verified. For **untrusted or unverified** sources,
bind as `binary of n bytes` instead: `binary` copies exactly `n` bytes
with no terminator scan, so it is safe by construction.
For `ref`, the buffer remains foreign and is never freed by Lale; for
`copy`, the copy is Lale-owned and the original is never freed by Lale.

**`n == 0` is a special case.** For an empty result the returned pointer
may be **null** (a C function commonly returns `(NULL, 0)` for "no
data"), and no terminator is read: the runtime constructs an empty
`text` (`chars == 0`, `bytes == 0`) without dereferencing `ptr`. This
applies to both `ref text` and `copy text` returns. `binary` needs no
such rule --- it has no terminator invariant.

#### 5.3 `readonly ref` --- what it does and does not promise

`readonly ref` is a **Lale-side "will not write through it" marker** ---
the §3.4 capability modifier applied at the boundary. It does not stop
the foreign side from mutating the buffer --- C can still write through
its own pointer, another alias, or another thread. This is the same
weakness as C's own `const` (and Rust's `*const T`), and it is inherent
to FFI: no language can prevent foreign code from mutating data it still
holds. Therefore:

- `readonly ref` is **a declaration of the interface contract**, not
  an immutability guarantee. The compiler must **not** use it to
  licence optimisations that assume the buffer is stable across reads
  (for example, caching a value it read once).
- The only mode that gives a real "it will not change under me"
  guarantee is `copy` --- after the deep copy Lale owns a private
  buffer that C cannot reach.
- Concurrent mutation is a separate data-race and memory-model
  concern, outside this design.
- `readonly` governs **safe code**. Reading the raw address field
  (`s.ptr`, `xs.ptr`) and writing through it with `unsafe value at` is
  outside the guarantee --- the same way C's `const` can be cast away.
  `unsafe` is the explicit escape hatch, not a hole in `readonly`.

`readonly` is **not FFI-scoped**: it is a general capability modifier on
a reference or a view (§3.4), and it is not a `const` keyword --- the
same word forbids writes through a Lale reference. What is specific to
the FFI boundary is only the _caveat_ above: there, `readonly` cannot
promise that the data is stable, because the foreign side is outside the
compiler's control (`ARCHITECTURE.md §5.2`).

#### 5.4 Deferred: zero-copy + own

A third mode --- wrap a `malloc`-compatible foreign buffer and **take
ownership without copying** (Rust's `Vec::from_raw_parts`) --- is
deferred to **2.0.0** as an `unsafe`, purely additive operation (no ABI
change, no breakage). It must be explicit, because the compiler cannot
verify that a foreign pointer really came from a compatible allocator.

Many real libraries need exactly this: FFTW (`fftw_free`), GSL
(`gsl_vector_free`), libcurl (`curl_free`), GLib/GTK (`g_free`), SDL
(`SDL_FreeSurface`), and OpenCV (`cvReleaseMat`) all hand back owned
buffers paired with **their own** free function.

### 6. Rendering arrays and views

- **`write` and embedded values** use plain array formatting, with no
  type or unit annotations. An empty rank-1 array is displayed as `[]`,
  and a one-element array containing zero as `[0]`, matching the source
  literals, so output can be copied back as source unambiguously. An
  array and a view of it render identically.

- **`debug` and structured diagnostics** use JSON that states the
  element **type and unit once for the whole array**, not per element
  (`unit` is `""` when the array is unitless), and includes a
  `dimensions` list. Including dimensions is necessary to distinguish
  rectangular shapes with the same values, especially shapes with a zero
  extent. JSON also distinguishes an empty array (`"values": []`) from a
  one-element zero array (`"values": [0]`) without relying on the Lale
  literal spelling:

  ```text
  debug arr   // {"type": "i32", "unit": "", "dimensions": [3], "values": [1, 2, 3]}
  debug m     // {"type": "i32", "unit": "", "dimensions": [3, 0], "values": []}
  ```

The once-only rule follows from the shape of the data: an array is
**homogeneous**, so repeating `type` and `unit` on every element would
repeat one fact. A struct is **heterogeneous**, so its existing
per-field annotations (each field has its own type and unit) stay as
they are. This is the one intentional difference between the array and
struct debug formats.

### 7. Performance

#### 7.1 The array descriptor

Every array is represented as an **array descriptor** --- a small descriptor
that travels with the value and records its address, rank, and shape. A
`rank` field is stored on **every** rank, so `x.rank` is a read-only,
runtime-readable field for all arrays:

```text
array of T    = { ptr: pointer, rank: u64, count: u64 in <elements>, capacity: u64 in <elements> }
2D array of T = { ptr: pointer, rank: u64, count_1: u64, count_2: u64 }
rD array of T = { ptr: pointer, rank: u64, count_1: u64, ..., count_r: u64 }
```

`ptr` addresses the element buffer; `rank` is the constant rank --- 1 for
`array of T`, 2 for `2D array of T`, and so on. `count` is the rank-1
descriptor's stored element count; `count_i` are the stored counts for
the rectangular dimensions. At source level, `.elements` is the total
element-count property and `.dimensions` returns the per-dimension counts
as an independent rank-1 array value. When only `.dimensions[i]` is used,
the compiler may read the corresponding descriptor count directly without
materialising that temporary array. When `.dimensions` is used as a whole
value (for example `var d = m.dimensions`), the returned temporary has a
compile-time-constant count (`xs.rank` elements) and may be stack-lowered
by §7.3 when its data pointer does not escape the frame. `capacity`
appears only on the rank-1
descriptor: it is the allocated element count, and `x.capacity` is a
compile error for a rank greater than one. A rank-1 descriptor is
32 bytes (four 8-byte fields); a rectangular descriptor is `2 + r`
8-byte fields. An owning rank-1 array with `capacity == 0` has a **null** `ptr`
and owns no buffer; any owned rank-1 allocation has `capacity >= 1` (§2.3).

The rank field is constant for a given array type and must agree with
that type; it is stored in the descriptor for runtime inspection and ABI
validation, not to make rank dynamically changeable. The descriptor size
therefore depends on rank, while rank itself never changes. The logical
`.elements` property is the product of all dimension counts; indexing the
`.dimensions` property returns the corresponding per-dimension count. There is **no capacity** for a rectangular
array: the allocation holds exactly the product of the counts in compact
row-major order, so `ptr` is always a dense C-style block. Resizing any
dimension replaces the payload buffer when the resulting shape is
non-empty, moves surviving elements to their corresponding indices,
destructs removed elements, and initializes newly created slots (§2.8);
no in-place rearrangement is ever required. A zero-product shape has a
null `ptr` and no payload buffer. Ownership remains out-of-band
metadata carried alongside the value, as in the existing proposal (§3.2).

The source-level descriptor properties are **read-only** (§3.4): `ptr`,
`rank`, `.elements`, `.dimensions`, and (rank 1) `.capacity`. `.elements`
and `.dimensions` are semantic properties; neither requires a dedicated
field beyond the stored counts shown above. The address
field is `ptr` for `text`, `binary`, and every array descriptor (`array.ptr`).

An `array of T` **owns** its buffer, and the compiler manages that
memory automatically: it is freed at scope exit, exactly as for `text`
and `binary` (`ARCHITECTURE.md §5.14b`). An array with `capacity == 0`
owns no buffer, so there is nothing to free. The user never writes
`release` for an array. This widens a deliberately narrow exception ---
the compiler-managed heap types --- from `text` and `binary` to arrays
as well (section 9).

#### 7.2 By default, arrays live on the heap

The straightforward implementation puts every array's buffer on the
heap: one allocation and one free per array, and the elements sit in a
separate block from the surrounding locals. A register-held `pointer`
still makes ordinary indexing a single load through a register base (not
a pointer chase), but the costs differ by access path --- plain
indexing, `ref` parameters, and inline-field access each have their own
profile, and should be measured separately rather than assumed.

#### 7.3 Optimisation: the "count never changes" lowering

A **lowering** is a compiler transformation that rewrites a program into
a cheaper or more concrete form. The one described here keeps an array's
elements in a fixed stack slot instead of heap memory.

Lale already tracks, per variable, whether it is reassigned or shared
(`is_reassigned` / `is_shared`, `ARCHITECTURE.md §4.5.7a`; the columns
are declared in §4.5.8). Adding a "count is provably constant" fact lets
the compiler **keep the descriptor but place the buffer on the stack
instead of the heap**: `var a as array of i32 = [1, 2, 3, 4, 5]` becomes
a fixed-size slot in the **frame** (the per-call area of stack memory)
plus `{ ptr = address of that slot, rank = 1, count = 5, capacity = 5 }`.

A local is **never** moved into a global by this lowering: that would
share one buffer across calls and break recursion and reentrancy. A
module-level array's buffer already lives in static storage (§2.6);
static storage is for globals, stack storage for locals.

This is **best-effort, an optimisation and never a semantic promise**: a
constant count is no guarantee of stack storage. It reuses the existing
`is_reassigned` / `is_shared` tracking and silently de-optimises when a
`grow`, a reassignment, or a runtime-count initialiser appears.

**The lowering is restricted.** It is applied only when the compiler can
establish that the array's data pointer does **not** escape the frame.
The escape routes are: passing `.ptr` to an FFI call, exposing the array
through an `inline` field, deriving a raw pointer from the array,
returning the array (which materialises a heap copy anyway), and storing
a reference or view beyond the frame (already a compile error, §3.4 /
§8.5). If any of these is present --- or the compiler cannot decide ---
the buffer stays on the heap. Because the lowering is best-effort,
restricting it cannot change a program's meaning; it only means that
fewer arrays receive the optimisation. This settles the former "no
escape analysis" assumption --- the earlier idea that the compiler would
not need to check whether a data pointer can outlive its storage: the
optimisation now relies on a **narrow** escape check, not on the absence
of one.

A buffer lowered into **stack storage** is not heap memory and must
never be freed by the heap deallocator; the stack slot is reclaimed when
the frame returns. An array that may grow --- one whose count is not
provably constant --- is kept on the heap from the outset (§7.3), so a
stack-lowered array never needs a larger buffer and there is no runtime
transition to the heap. The only way its value outlives the frame is the
return rule, which materialises a heap copy (§3.2).

#### 7.4 Genuinely dynamic arrays, and the cost of copying

Arrays that really do grow and shrink heap-allocate as a matter of
course --- normal for a resizable array. Deep-copy value semantics can
be expensive for large arrays: passing a million-element array by value
costs `O(n)`. The definition-site warning helps, but it is not a
substitute for an efficient **read-only** pass. That pass is the
`readonly ref` parameter of §3.4, which is **Lale 1.0.0**; what remains
deferred to 2.0.0 is only the _compiler-inferred_ read-only-ness that
needs no declaration (§10, `ARCHITECTURE.md §5.14d`). Automatic
allocation and freeing remove the bookkeeping, not the cost, so
allocation-heavy workloads should be benchmarked rather than assumed
cheap. Rectangular resizing is the same shape: it reallocates and moves
the whole compact block (§2.7), so a dimension grow or shrink is
`O(total elements)` and repeated one-dimension growth is quadratic.

## Part II --- Ranged views (planned for 2.0.0)

A **ranged view** is a separate feature, recommended to follow 1.0.0
(section 10). It is the deferred part of non-owning access --- the
**sub-range** window (`view of T`, `xs[a to b]`) with its own freeze and
invalidation rules. Whole-range references (`ref of array of T`) are
already part of 1.0.0 (Part I).

### 8. Views

#### 8.1 What a view is

A **view** is a non-owning window onto a contiguous range of elements of
an array (or of `text` / `binary`). It is non-owning; it never copies
and never owns, so it is never freed and never triggers a deep copy.

The name **`view`** was chosen because it states the semantics directly;
`slice`, the term used by several other languages, means a _copy_ in
Python and Ruby --- the communities closest to Lale's audience --- so it
would mislead exactly the readers Lale targets.

#### 8.2 Creating a view

A view of a Lale array is created with a ranged subscript
`origin[first to last]`, with inclusive, 1-based bounds and **no**
`from`:

```lale
var temps as array of f64 = [273.15, 300.0, 310.5]
var mid as view of array of f64 = temps[2 to 3]   // elements 2..3, inclusive
write mid                                        // [300, 310.5]
mid[1] = 301.0                                   // writes through the view into temps[2]
```

- **Type:** written `view of T`, where `T` is the **source type** (for
  example `view of array of i32`, `view of text`) --- the same
  convention as `ref of T` (§3.4) --- mirroring the existing
  constructor-family form `array of T`. The source type is always
  explicit --- no inference --- consistent with Lale's
  no-implicit-conversion rule.
- **Unit:** a view inherits the type and unit of its origin.
- **Range validity:** the bounds must satisfy
  `1 ≤ first ≤ last ≤ the origin's count` --- `origin.elements` for a
  **rank-1** array, `origin.chars` for `text`, `origin.bytes` for
  `binary`. (For a rectangular array the meaning of `m[a to b]` is an
  open Part II decision, §11.1; the `origin.elements` bound here is
  rank-1 semantics for now.) A
  **constant** violation --- an inverted range like `xs[3 to 2]`, or a
  bound out of range like `xs[0 to 3]` or `xs[2 to 99]` --- is a
  **compile-time error**; a **runtime** violation **aborts** with a
  clear message. (A tolerating variant that treats it as a no-op is a
  possible future extension.) An _array_ may be empty (§2.3), but a
  view cannot have an inverted range; a zero-count view, if wanted,
  needs a separate spelling.
- **`[a to b]` vs `[a]`:** `origin[a]` is the single element at index
  `a`; `origin[a to b]` is a view of the range `a…b`. Only the ranged
  form creates a view.
- **`text` / `binary`:** the same syntax applies. `myText[i]` returns
  a `char` --- the i-th **code point** (no unit) --- and `myBinary[i]`
  returns a `byte` --- the i-th **byte** (no unit). A ranged subscript
  gives a non-owning view: `myText[3 to 5]` is a view of code points
  3...5 and `myBinary[3 to 5]` a view of bytes 3...5 (a pointer to the
  origin plus `first` and `last`). Code points --- not grapheme
  clusters --- are Lale's unit, matching the `chars` field of `text`
  (`ARCHITECTURE.md §5.14a`, §5.14c); grapheme clustering is an
  explicit non-goal. (Code-point indexing scans the UTF-8 bytes from
  the start of the string, so `myText[i]` costs `O(i)`.) A `text` view
  stores **code-point indices** (`first`, `last`), not byte offsets:
  access scans UTF-8 from the origin's start, so `mid[i]` costs
  `O(first + i)`. This is the UTF-8-everywhere choice; a byte-offset
  text view is rejected. A `binary` view stores byte indices, but it
  is a view of a different type.

`view`, `of`, and `to` are **contextual**, not reserved words,
consistent with Lale's "no reserved words" rule.

#### 8.3 Representation, by form and origin

What a non-owning access stores depends on its **form** --- a ranged
`view` or a whole-range `ref` --- and, for a view, on whether the origin
has a **Lale descriptor** (a descriptor that can be followed after
reallocation):

| Form                               | Origin                                    | Lale descriptor? | Result                                                     |
| ---------------------------------- | ----------------------------------------- | ---------------- | ---------------------------------------------------------- |
| `xs[a to b]` (ranged view)         | a Lale dynamic array, `text`, or `binary` | yes              | a **ranged view** `{ descriptor: pointer, first, last }`   |
| `ref to x` (whole-range reference) | a Lale dynamic array, `text`, or `binary` | yes              | a **reference**: a copy of the source descriptor           |
| field read / `ref` FFI return      | inline bytes, or a foreign C buffer       | no               | a **reference**: a descriptor copy with `capacity = count` |

**Observable semantics are identical** in all cases --- the source-level
length property (`elements` for arrays, `chars` for `text`, and `bytes`
for `binary`), indexing, iteration, non-owning access, and the escape
rule. The underlying ABI fields can differ (`count` for a rank-1 array,
`count_i` for rectangular extents, and `chars` / `bytes` for `text` /
`binary`). Only the _representation_ differs, and it is dictated by the
form and the origin:

- **A ranged view of an origin with a descriptor** (a Lale array, `text`,
  or `binary`). The view stores a pointer to the origin's descriptor plus
  the range bounds. The extra hop is deliberate: the descriptor lives in
  the array variable's slot and does not move, so when the array grows
  and its buffer is reallocated, the view re-reads the descriptor and
  keeps seeing the correct buffer. This is what makes a view **survive
  growth**.
- **A reference** (`ref to x`, an `inline` field read, or an FFI `ref`
  result). There is no descriptor to follow in the inline / foreign case,
  and `ref to` deliberately chooses the direct form even when one
  exists, so the reference is a copy of the source type's descriptor ---
  `{ ptr, rank, count, capacity = count }` for a rank-1 array (a
  rectangular array's descriptor is copied directly, with no capacity),
  `{ ptr, bytes, chars }` for `text`, `{ ptr, bytes }` for `binary`
  --- whose `ptr` addresses the buffer directly (§13.4). A reference
  to an owning rank-1 array copies the origin's capacity (which may
  exceed `count`). A descriptorless inline/foreign rank-1 reference instead
  uses synthetic `capacity = count`; at count zero it preserves the
  origin pointer verbatim (the runtime does not null it). The owning-array
  invariant
  `capacity == 0 ⇒ ptr == null` does not apply to this non-owning
  reference, whose pointer cannot be dereferenced at zero count. A direct
  reference cannot survive reallocation, so its origin is frozen against
  growth, shrinkage, and reassignment (§3.4).

This distinction is also why reading an `inline` field yields a
**reference** (`ref of array of T`, §4.2) rather than a ranged view:
there is no Lale descriptor, so the natural form is "the whole buffer,
reference."

The descriptor's **address must be stable** for growth to be survivable. The
first bullet's guarantee holds only while the descriptor does not move: for
a **top-level array variable** the descriptor lives in that variable's slot
and never moves, so a view of it survives growth. For an **array-valued
element of another array** in a jagged `array of array of T`
(`rows[i]`, §2.7) the descriptor lives _inside the outer array's buffer_.
The **root-freeze rule** settles this: taking a
reference or view of `rows[i]` marks the root `rows` as `taken`, so `rows`
cannot grow, shrink, or be reassigned while the access lives (§8.5), and
(conservatively) no row of `rows` may be resized. The descriptor therefore
cannot move, and the view stays valid. The cost is that a view of
`rows[i]` freezes the whole outer array, not just row `i`.

The jagged discussion above does not apply to a **rectangular** array: a
rectangular payload has **no per-row descriptors**. `m[i]` on a
`2D array of T` selects a contiguous lower-rank region of the single
payload, not an independently stored array value. A reference to that
region --- `ref to m[i]` --- is therefore a **direct** reference into the
frozen payload, exactly like a descriptorless `inline` or foreign origin:
there is no origin descriptor to follow, and none is needed, because the
root-freeze rule prevents `m` from reallocating while the reference
lives. For a rank-2 `m`, `ref to m[i]` is a rank-1 reference whose
descriptor is `{ ptr = address of row i, rank = 1, count = the column
extent, capacity = count }` --- the synthetic `capacity == count` of the
descriptorless-origin row above, because the selected region has no
independent spare capacity. It therefore exposes `.capacity`, because the
result is rank 1, reporting that synthetic value. The same applies at
any rank: selecting one index of an `r`-dimensional rectangular array
yields a reference of rank `r - 1` whose counts are the remaining
extents, with no capacity field when `r - 1 > 1`.

**References and views are distinct.** A `ref of T` holds a **direct
pointer to the data** (a copy of the source descriptor): it is the cheapest
non-owning form, and it freezes the origin's growth, shrinkage, and
reassignment so the pointer cannot dangle. A `view of T` holds a
**pointer to the origin's descriptor** --- a pointer to a pointer, since the
descriptor's first field is `ptr` --- so it follows the buffer across
reallocation and **survives growth**; it freezes only `shrink` and
reassignment. The extra indirection is the price of growth-safety. They
are two types, not one type with two names.

#### 8.4 Passing a view

Passing a view **by value** copies the **descriptor** --- the pointer to
the origin's descriptor plus the bounds --- not the elements. The cost is
`O(1)`, the callee still views the caller's storage, and writes through
the view are visible to the caller. A `readonly view of T` is the same
`O(1)` copy but forbids writes through the holder. `ref xs as view of T`
is a **compile error**: a view is already a non-owning handle, so `ref`
would add nothing and would hide the aliasing. The `O(1)` pass is the
benefit; the aliasing --- a by-value parameter writing through into the
caller's data --- is the cost, and it is stated here rather than hidden
behind a materialising copy.

```lale
var data as array of i32 = [10, 20, 30, 40, 50]
var mid as view of array of i32 = data[2 to 4]      // view of elements 2..4

// by value: `xs` is a copy of the descriptor; it still views data[2..4]
fn sum_view(xs as view of array of i32) returns i32
    var total as i32 = 0
    loop var i as u64 from 1 to xs.elements
        total = total + xs[i]
    end loop
    return total
end fn

// by value: writes through the view are visible to the caller
fn bump(xs as view of array of i32) returns nothing
    loop var i as u64 from 1 to xs.elements
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
| `ref xs as view of T`      | **compile error**                                      | ---  | ---                        |

There is no `copy` form of a view parameter: copying the descriptor
already happens for a plain by-value view parameter, so `copy` would add
nothing. A view is also **not** implicitly converted to an owning
`array of T` --- passing one to an `array of T` parameter is a compile
error. `copy of` is the call-site operator that materialises an owned
copy of a **reference** (§3.4); because a reference is always
whole-range, that is a copy, not a shape change. Whether `copy of`
should also materialise a **view** --- a sub-range, which is a shape
change, not a mere copy --- is deferred with the view layer. A
`readonly view` is the cheap read-only form; it ships with views
(section 10).

#### 8.5 The escape rule, and lifetime across resize

The rules in this section govern **ranged views**, which are deferred
(§10). They are stated now so the 1.0.0 reference rules (§3.4) read as
the reference-only half of one non-owning design; the view-specific
guarantees below take effect only when the view layer ships. Until then,
only `ref` (§3.4) is available.

A view refers to the origin's storage. If the origin is a **local**
array, that storage is a stack slot, and the view dangles once the
function returns. The compiler forbids this, extending the existing
raw-pointer escape rule (`ARCHITECTURE.md §5.14`). Views start from the
**same conservative stance as references** (§3.4):

> A view **never escapes the function that creates or receives it**. A
> view of a local array, a global array, a by-value array parameter, a
> `ref` parameter, or a view parameter must not be **returned** or
> **stored in a global**.

Detection is **per-function and syntactic**: a view of a local array is
marked "references a local"; a view of an already-marked view inherits
the mark; and returning or globally storing a marked view is a compile
error.

```lale
// ❌ compile error: the view outlives its local origin
fn bad() returns view of array of i32
    var local as array of i32 = [1, 2, 3]
    return local[1 to 3]           // local's storage is a stack slot — dangles on return
end fn

// ❌ compile error (the initial, conservative rule): a view of a global does not escape
var g as array of i32 = [1, 2, 3]   // module-level array

fn not_yet() returns view of array of i32
    return g[1 to 2]                // rejected: a later grow/shrink/reassignment of g could invalidate it
end fn
```

This rule is a **boundary of the guarantee**, not a general proof of
view safety. In particular:

- A view of a **by-value array parameter** refers to a local copy, so
  it must not escape.
- A view of a **`ref` array parameter** refers to the caller's
  storage, which the callee cannot see, so it must not escape --- the
  same compile error as returning a `ref` parameter (§3.4).
- A by-value **view parameter** likewise refers to the caller's
  storage and must not be returned or stored (§3.4).
- Passing a view _into_ a callee is allowed (the rule is
  per-function).

**A conservative start, with a recorded relaxation (option B2).** Views
adopt the reference's escape stance first, so both non-owning types have
**one** rule to learn: nothing escapes. The relaxation is additive, so
adopting it later cannot break an existing program. The principled
alternative --- kept here so the deferred view decision can reconsider
it --- is that a view **follows the origin's descriptor** (§8.3), whereas a
reference holds a _copy_ of it: a view of a **global** could therefore
safely escape, because a later `grow` reallocates the buffer and the
view re-reads the descriptor. `shrink` and reassignment still invalidate it,
so even under this alternative a view of a `ref` / view parameter may
not escape --- its origin is unverifiable across the call boundary.

**A non-owning access freezes its origin.** For a ranged view, growth is
safe (§8.3). Two operations are not: `shrink` (if it drops below the
view's `last`, the view points past the new end) and reassignment (the
descriptor slot is overwritten in place, so the view silently re-points).
The rule is tracked per array variable --- and, for the element-level
freeze, per element or field that owns a buffer --- whether a non-owning
access has been taken of it: a `taken` mark alongside `is_reassigned` /
`is_shared`. If set, `shrink` and reassignment of that array are
**compile errors** for a view; for a **reference**, `grow`, `shrink`,
and reassignment are all compile errors, because a reference holds a
descriptor copy (§3.4). The freeze is **structural** --- element writes
through the view, the reference, or the array itself remain allowed,
except that reassigning an element or field that owns a buffer (a nested
array, `text`, `binary`) is frozen while a reference to it lives
(§11.2). The reference half (arrays, `text`, `binary`) is **Lale
1.0.0**; the ranged-view half ships with this layer. The rule is
per-function and deliberately conservative (it also rejects the case
where the non-owning access is already dead); relaxing it later with
liveness analysis is additive. The **reference** half has no global case
in 1.0.0: references never escape (§3.4), so the freeze only has to hold
within the taking function. The **view** half is per-function only for
a local or parameter origin; a locally held view of a **mutable** global
can still be invalidated by a call that shrinks or reassigns that global
(§11.1), so views of mutable globals are not yet permitted unless global
stability is established. If the view layer later
adopts the global-escape relaxation (option B2), a view of a global can
escape and view-taking on a global becomes cross-function --- then the
rule must either forbid shrinking a global that has had a view taken
anywhere, or restrict views to non-global origins.

The freeze applies to the **root** array, not only to the sub-array
named in the access. Taking a reference or view of any part of a
multidimensional array --- `rows[i]`, or an `inline` field --- marks the
root (`m`) as `taken`: growing, shrinking, or reassigning `m`, and
(conservatively) resizing any row of `m`, while the access lives is a
compile error, because growth can move the element descriptor the access
depends on. Element writes that do not replace the referenced
array-valued element remain allowed; replacing it (`rows[1] = row`) is a
**compile error** under the element-level freeze (§11.2).

**Across a call (`ref`).** A `ref` array parameter cannot resize its
array (§3.1), cannot resize an array-valued element reached through it,
cannot replace an element or field that owns its own buffer
(`grow m[1]`, `rows[1] = row`, `xs[i] = "new"` through a `ref` parameter
are compile errors), and cannot rebind it (§11.2). It can only mutate
non-replacing elements, which cannot invalidate a view. So the
per-function rule above extends across a call without extra analysis: a
callee cannot dangle the caller's view or reference.

#### 8.6 A view of a view; read-only views

`view of view of T` is **rejected when the view layer ships** (2.0.0) ---
a pure restriction, so allowing it later is additive and non-breaking. As
with the nested array spelling (§2.12), the rejection is made in semantic
analysis rather than in the grammar, so the diagnostic can name the fix:
the element of `view of` may not itself be a `view`.

There is likewise no way to get that promise **without a declaration**:
a plain `view` (or a writable reference) is writable. A **`readonly`**
reference (1.0.0) or `readonly` view (with the view layer) supplies it
--- it forbids writes through the holder while staying non-owning. What
remains deferred to 2.0.0 is merely _inferring_ read-only-ness without
the declaration (`ARCHITECTURE.md §5.14d`).

## Part III --- Principles, versions, and follow-ups

### 9. Fit with Lale's design principles

- **Language over Library.** Bounds-checked, deep-copying arrays with
  read-only `.elements` and `.dimensions` properties require compiler awareness --- a textbook fit.
  The resizing syntax (`grow` / `shrink` / `by`, §2.8) matches the
  "reads like the problem domain" criterion.
- **1-based indexing** is respected throughout.
- **Raw pointers only** (`ARCHITECTURE.md §5.14`): the descriptor's `ptr`
  field is a raw pointer; no typed pointers are introduced.
- **No implicit conversions** (`ARCHITECTURE.md §5.5`): `array of i32`
  being one type whatever the count makes "no coercion" literally
  true, and `inline` field access is materialisation, not a
  conversion.
- **A type contains no brackets** (§2.9): brackets keep their value
  roles --- indexing (`xs[i]`) and array literals (`[1, 2, 3]`, with `[]`
  the empty literal) --- so a reader tells a type from a value at a
  glance.
- **No type aliases** (`ARCHITECTURE.md §5.3`): `inline` is a
  field-only modifier, not an alias.
- **Complete initialisation** (`ARCHITECTURE.md §5.10`): the
  `grow`-initialiser count rule (§2.8) is consistent with it, and a
  declaration cannot mismatch its count at all, because the
  initialiser supplies it (§2.2).

Five deliberate, documented expansions of existing rules:

- **A declared `readonly` capability.** `readonly ref` /
  `readonly view` _declare_ that the holder may not write through the
  reference, whereas mutability is normally inferred from SSA ---
  **Single Static Assignment**, a program form in which each variable
  is assigned once, so the compiler can see exactly what changes ---
  not declared (`ARCHITECTURE.md §5.2`). Declaring it is justified
  because a **reference type's capability** is a property of the
  _handle_ --- there is no single function body to infer it from ---
  not a variable whose SSA the compiler can analyse. This is recorded
  as decided in §11.2.
- **Compiler-managed arrays.** Arrays extend the "compiler manages the
  heap" exception beyond `text` / `binary` (`ARCHITECTURE.md §5.14b`).
  Requiring `release` for every array would leak implementation
  details into user code, so the exception is widened deliberately.
- **Observable capacity, with invariants (rank 1).** On a rank-1
  array, `xs.capacity` is a read-only field with the guarantees
  `capacity >= elements`, `grow` never decreases it, and `shrink` never
  increases it. The exact capacity and growth factor remain
  unspecified. Rectangular arrays (rank greater than one) have no
  capacity (§2.7). This deliberately makes an allocation detail
  observable, so programs can reason about spare room without
  constraining the implementation.
- **Handle types alias by value --- the one carve-out to value
  semantics.** A **handle** --- `ref of T` or `view of T` --- is a
  non-owning value over someone else's buffer. Passing one **by
  value** copies only its small descriptor / descriptor, not the elements,
  so writes through it reach the origin's storage. This is the single
  exception to §3.1's "one variable can never silently change
  another": a handle is _meant_ to be shared, and the aliasing is the
  point, not a surprise. Every owning value --- an array, a `text`, a
  `binary`, a struct --- still deep-copies.
- **New contextual keywords, and one new token shape.** The design
  adds `array`, `inline` (field-only), `ref` / `ref of` / `ref to`,
  `readonly`, `view` / `of` / `to`, `elements` / `by`, `dimension`,
  `grow` / `shrink`, and `C` (after `export`) --- all **contextual**, per
  Lale's no-reserved-words rule, so none of them becomes unusable as
  an identifier. (`may fail` would be a further contextual form, but
  it is deferred with the failure-recovery design, Annex B.) The rank
  prefix (`2D`, `3D`, ...) is a new token shape: the first digit-led
  token that is not a number (§2.12).

### 9.1 One array representation for vectors and matrices

Lale's specialised built-in vector and matrix data types are to be
**removed in favour of the array family**. A rank-1 array is the
representation for a vector; a rank-2 rectangular array is the
representation for a matrix. Higher-rank arrays use the same storage
model. There is no parallel vector or matrix storage type and no implicit
conversion to one. The mathematical operations previously available for
vector/matrix operands are provided directly for array operands, with
applicability determined by rank, shape, element type, and physical unit.

The semantic analyser checks every operation. It rejects an operation at
compile time when rank, type, shape or unit incompatibility is statically
known; otherwise the generated program checks runtime extents and fails
clearly if the operands do not fit. The operation uses the corresponding
scalar operation's type and unit rules for each element, with no implicit
numeric conversion. No implicit broadcasting is performed unless an
individual operation explicitly defines it.

The following are the core contracts for the existing vector operations
and the rectangular matrix operations in this proposal:

| Operation                                                                                                    | Array operands and required shape                                                                                              | Result and checks                                                                                                                                                                                         |
| ------------------------------------------------------------------------------------------------------------ | ------------------------------------------------------------------------------------------------------------------------------ | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Component-wise unary operation                                                                               | Any rank supported by the corresponding scalar operator                                                                        | Same rank and dimensions; element operation determines result type and unit                                                                                                                               |
| Element-wise binary operation explicitly defined for arrays (for example, existing component-wise `+` / `-`) | Same rank and identical dimensions; element types must satisfy the corresponding scalar operator                               | Same shape; scalar type and unit rules apply; mismatched shapes are rejected                                                                                                                              |
| Scalar-array multiplication, `s * a` or `a * s`                                                              | Scalar plus an array of any supported rank; element types must satisfy scalar multiplication rules                             | Same array shape; scalar multiplication rules determine element type and unit                                                                                                                             |
| Dot product, `u ⋅ v` or `dot(u, v)`                                                                          | Two rank-1 arrays with equal `.elements` and the same supported numeric element type                                           | Scalar carrying the product of the operands' units; two equal empty arrays give `0` in the product unit (for two `<m>` arrays, `<m²>`), since the unit comes from the operand types, not any element      |
| Cross product, `u ⨯ v` or `cross(u, v)`                                                                      | Two rank-1 numeric arrays, each with exactly 3 elements and the same supported numeric element type                            | Rank-1 result with 3 elements; units multiply. Length other than 3 is rejected statically when known, otherwise at runtime                                                                                |
| Matrix multiplication                                                                                        | Two rank-2 numeric rectangular arrays with left column extent equal to right row extent                                        | Rank-2 result with shape `(left rows, right columns)`; scalar multiplication and addition rules determine element type and units                                                                          |
| Matrix × vector                                                                                              | One rank-2 numeric rectangular array and one rank-1 numeric array with the matrix's column extent equal to the vector's length | Rank-1 result whose length is the matrix's row extent; scalar multiplication and addition rules determine element type and units                                                                          |
| `transpose`                                                                                                  | One rank-2 rectangular array                                                                                                   | Rank-2 result with the two extents swapped; element type and unit unchanged                                                                                                                               |
| `determinant`                                                                                                | One square rank-2 floating-point rectangular array (equal extents)                                                             | Scalar; the result unit is the element unit raised to the square extent `n` (dimensionless elements give a unitless result); see prose for the runtime-extent rule                                        |
| `inverse`                                                                                                    | One square rank-2 floating-point rectangular array (equal extents)                                                             | Rank-2 result with the same shape; a singular matrix returns `nothing`, so the result type is the optional form of the rank-2 floating-point array; the result unit is the reciprocal of the element unit |

The scalar operation `s ⋅ t` remains the existing scalar multiplication
operation and follows ordinary scalar type and unit rules; the array dot
product is the rank-1 operation in the table above.

These are array operations, not operations on a hidden vector/matrix
wrapper: the semantic analyser selects the rule from the operands' ranks,
element types, extents, and units. It checks every statically knowable
precondition at compile time. When an extent is only known at runtime,
the generated operation checks it before accessing payload elements or
committing a result; a failed check is a fatal runtime failure in 1.0.0
(Annex B), not a partially completed result.

For matrix multiplication, an inner dimension of zero is valid: multiplying
a matrix of shape `m × 0` by one of shape `0 × n` produces an `m × n`
matrix filled with the additive identity, zero, provided the result element
type supports the required numeric operations. The result's element unit
is the product of the operand element units, and each zero has that result
unit. The zero-element cases are not shape errors; the semantic analyser
still checks the dimensions that are required to match: the left second
extent must equal the right first extent, and zero is valid rather than
a wildcard. If either output extent is zero, the result is an empty
rectangular array retaining that output shape.

`transpose` and `determinant` follow the table above. `determinant` and
`inverse` are both defined only for square floating-point element types:
an integer matrix has no closed inverse in general, and the floating-point
elimination below is not applied to integers because Lale performs no
implicit numeric conversion. `inverse` returns the optional form of the
rank-2 floating-point
array: an invertible matrix yields a rank-2 result with the same shape,
and a singular matrix yields `nothing`. There is no trap on a singular
matrix; the caller inspects the optional result. A matrix is **singular**
exactly when elimination produces an exactly-zero pivot --- the same
operational test LAPACK's `DGETRF` / `DGETRI` use. The computed
determinant is **not** the test: a zero determinant can arise from
floating-point underflow for an invertible matrix (for example a diagonal
matrix of `1e-308` entries in `f64` has a representable inverse even
though its determinant underflows to zero). No tolerance is applied, and
a near-singular (ill-conditioned) matrix with non-zero pivots is **not**
detected in 1.0.0 (a documented limitation, matching LAPACK's default).
The interpreter and AOT backends must implement the same elimination
(partial pivoting): the pivot is the largest-magnitude entry in the
active column, and on an exact tie the **lowest row index** wins. NaN and
infinity propagate through the factorization per IEEE 754 --- they are
not "exactly zero" pivots, so such a matrix is not classified as singular,
and its `determinant` / `inverse` result is NaN or infinity. The two
backends must share this pivot-selection semantics, but need not guarantee
bit-identical low-order result bits. The result elements carry the reciprocal of
the element unit, matching the determinant's unit handling. `inverse` and
`determinant` share the same factorization, so a singular matrix --- one
with an exactly-zero pivot --- has `determinant` zero, not an error;
otherwise `determinant` follows the documented floating-point arithmetic,
including possible underflow or overflow.

**Zero-by-zero square matrices.** Because zero extents are permitted, a
`0 × 0` rectangular array is a valid square matrix. Its `determinant` is
`1` --- the empty product, the multiplicative identity --- and it is
invertible: `inverse` of a `0 × 0` matrix returns the `0 × 0` matrix
itself (its own inverse), not `nothing`. This adopts the standard
empty-product convention, consistent with the empty-sum result for
matrix multiplication and the dot product.

**Determinant units and runtime extents.** The determinant of an
`n × n` matrix whose elements carry unit `U` has unit `Uⁿ`. Because
extents are runtime values (§2.12), `Uⁿ` is a static Lale unit only when
`n` is a compile-time constant. `determinant` therefore requires either
dimensionless elements (the result is unitless regardless of `n`) or a
statically-known square extent `n` (the result unit is the element unit
raised to that compile-time constant `n`). A unit-bearing determinant
whose square extent is a runtime value is a compile-time error; the
diagnostic names both fixes --- make the elements dimensionless, or make
the extent a compile-time constant. This keeps Lale's unit model static;
no runtime-dependent units are introduced for 1.0.0. Concretely, because
a function parameter's extent is always a runtime value (§2.1), a
unit-bearing `determinant` call cannot cross a function boundary in
1.0.0: a reusable `determinant` wrapper over unit-bearing matrices is
rejected, because its square extent is not statically visible at the
call site. This is the intended 1.0.0 trade-off --- runtime-dependent
units would be a larger design change.

A unit-bearing determinant of a runtime `0 × 0` matrix is, mathematically,
well-defined --- `U⁰` is dimensionless for any `U` --- but the uniform rule
rejects it anyway: an "unless the extent happens to be zero" exception
would make the result's unit depend on a runtime value. The over-rejection
is deliberate.

```text
ERROR: determinant of a unit-bearing matrix with a runtime extent
  `m` has element unit `<m>` and a square extent that is a runtime value
  The result unit would be `<m>ⁿ`, which is not a static Lale unit.
  Fixes: make the elements dimensionless, or make the extent a compile-time constant.
```

`transpose` builds a **new** compact result: a compact row-major block has
no strides, so the two extents cannot be swapped while keeping the same
payload. The source is preserved, and element deep-copy rules govern any
elements that own their own resources.

These contracts generalise the existing vector dot/cross product rules
without requiring fixed vector-length types. For example, the dot product
works for rank-1 arrays of any equal length; the cross product remains
restricted to three components. The element type and unit checks remain
those of the underlying scalar operations, and the array dimensions are
checked separately. This does **not** give `*` an automatic element-wise
meaning for two array operands: the existing rejection of plain vector
`*` (a rank-1 operand on either side) remains, and dot and cross retain
their dedicated operators. Matrix multiplication is the one deliberate
exception, and it is **not symmetric**:

- `matrix * matrix` (two rank-2 numeric rectangular arrays) is matrix ×
  matrix, valid when the left column extent equals the right row extent.
- `matrix * vector` (a rank-2 numeric rectangular array on the left and a
  rank-1 numeric array on the right) is matrix × vector, valid when the
  matrix's column extent equals the vector's length.
- `vector * matrix` (a rank-1 array on the left) is **rejected**; no
  implicit row-vector operation is defined.
- `vector * vector` (two rank-1 arrays) is **rejected**; use `dot` or
  `cross`.

A zero inner dimension uses the same empty-sum convention as the dot
product: a matrix of shape `m × 0` multiplied by an empty vector produces
a length-`m` vector of zeros (the additive identity) in the product of
the operand units, and a `0 × 0` matrix multiplied by an empty vector
produces a length-0 vector. `*` therefore never means an element-wise
product between two array operands; it denotes matrix multiplication for
these rank combinations only.

These matrix and component-wise contracts apply to **rectangular** arrays
only. A jagged `array of array of T` is a rank-1 array whose elements are
themselves arrays, so its outer shape is not a rectangular shape and it
does not satisfy a `2D array of T` operand contract. Because `array of
array of T` and `2D array of T` are distinct static types, a statically
typed jagged operand is always rejected at compile time; runtime shape
validation applies only to malformed descriptors at an ABI boundary, not
to a statically-typed jagged array. The diagnostic names the required
form with an example:

```text
ERROR: rectangular array expected in matrix multiplication
  `rows` has type `array of array of i32` (jagged), not `2D array of i32`
  Matrix `*` requires contiguous rectangular operands whose shape is known
  (e.g. `var m as 2D array of i32 = [[1, 2], [3, 4]]`), because its shape
  contract is `m × n` times `n × p`, checked against each extent.
  Use `array of array of T` only when rows genuinely differ in length;
  for matrix arithmetic, declare the operand as `2D array of T`.
```

The rank and shape contract above is normative and must not depend on a
dedicated matrix type. An operation that has not yet been given a precise
rank, shape, element-type, and unit contract must not be silently
removed: it belongs in the vector/matrix operation migration audit
(§13.8) before the specialised types it replaces are deleted.

This means a `2D array of f64` is the matrix representation, not a
wrapper around a separate matrix type. The operator audit, literal
migration, ABI migration, and existing-program migration remain
necessary, but they are implementation and compatibility work for the
array replacement, not a reason to retain parallel built-in storage
models indefinitely. The specialised vector/matrix type forms are removed
with this design's array implementation; their supported operations are
migrated to array-aware operations with the checks above.

### 10. Version plan

| Feature                                                                                   | Planned for      | Notes                                                                                                                            |
| ----------------------------------------------------------------------------------------- | ---------------- | -------------------------------------------------------------------------------------------------------------------------------- |
| Dynamic array type `array of T` (runtime count)                                           | Lale 1.0.0       | one array type for every count                                                                                                   |
| Array type notation `array of T`; rank prefix `2D` / `3D`                                 | Lale 1.0.0       | brackets never spell a type; `[]` is the empty literal (§2.9, §2.12)                                                             |
| Complete initialisation, bounds checking                                                  | Lale 1.0.0       | core correctness guarantees                                                                                                      |
| Deep-copy value semantics; `ref` / `copy`                                                 | Lale 1.0.0       | ownership rules per §3.2                                                                                                         |
| Compiler-managed array memory (auto-free)                                                 | Lale 1.0.0       | as `text` / `binary`                                                                                                             |
| `grow` / `shrink` / `by` resizing                                                         | Lale 1.0.0       | arrays are dynamic; resizing is intrinsic                                                                                        |
| `inline array of T of n elements` fixed struct fields                                     | Lale 1.0.0       | C-compatible layout                                                                                                              |
| References (`ref of T`: arrays, `text`, `binary`; `inline` fields, FFI `ref`, `ref to x`) | Lale 1.0.0       | whole-range, non-owning; the 1.0.0 half of non-owning access                                                                     |
| `readonly` references                                                                     | Lale 1.0.0       | declared capability modifier (§3.4); replaces `const ref`                                                                        |
| `readonly` views                                                                          | Lale 2.0.0       | the same modifier on `view of T`                                                                                                 |
| FFI ownership `ref` / `copy` / `readonly ref`                                             | Lale 1.0.0       | C → Lale direction                                                                                                               |
| Array rendering (§6)                                                                      | Lale 1.0.0       | plain `write`, JSON `debug`                                                                                                      |
| View rendering (§6)                                                                       | Lale 2.0.0       | the same rules applied to a view                                                                                                 |
| "Count never changes" stack lowering (best-effort)                                        | Lale 1.0.0       | optimisation only; never a semantic promise                                                                                      |
| Ranged **views** (`view of T`, `xs[a to b]`)                                              | Lale 2.0.0       | sub-range windows; the deferred part of non-owning access                                                                        |
| Freeze rule for references                                                                | Lale 1.0.0       | `grow` / `shrink` / reassignment of an array; reassignment of `text` / `binary`                                                  |
| Freeze rule for ranged views                                                              | Lale 2.0.0       | `shrink` / reassignment only; a view survives growth                                                                             |
| Recoverable allocation failure (`may fail`)                                               | later version    | 1.0.0 is abort-only on failure; adding it later is additive                                                                      |
| `inline array of T of len elements` (flexible field)                                      | later version    | needs variable-sized-struct semantics                                                                                            |
| `view of view of T`                                                                       | later version    | additive, non-breaking                                                                                                           |
| Compiler-inferred read-only references (`ARCHITECTURE.md §5.14d`)                         | Lale 2.0.0       | inference, not declaration                                                                                                       |
| Escaping references via provenance / lifetime analysis                                    | later version    | additive relaxation of the 1.0.0 "references never escape" rule (§3.4, §11.2)                                                    |
| Zero-copy + own at the FFI boundary                                                       | Lale 2.0.0       | `unsafe`, additive                                                                                                               |
| `usize` / `isize` (pointer-sized integers)                                                | Lale 2.0.0       | 1.0.0 FFI uses `u64`/`i64` (roadmap §1)                                                                                          |
| Rectangular multidimensional arrays                                                       | Lale 1.0.0       | contiguous row-major live block; per-dimension counts, no spare capacity; reshape moves survivors and destructs dropped elements |
| Jagged nested arrays (`array of array of T`)                                              | Lale 1.0.0       | explicit nested form; rows may differ in size                                                                                    |
| Replacing specialised vector and matrix data types with arrays                            | Lale 1.0.0       | migrate supported operations to shape-checked array operators; audit literals, ABI, and existing programs                        |
| Generics                                                                                  | separate concern | needed for "any element type", independent of this design                                                                        |

The layering is deliberate: 1.0.0 ships **owned arrays plus a narrow
whole-range reference**, and defers **ranged views**, **recoverable
allocation failure**, and **advanced systems interop** (flexible fields,
foreign-buffer adoption, extra ABI mappings) --- so 1.0.0 does not
depend on range invalidation, global view tracking, liveness, failure
recovery, and variable-sized-struct layout all at once.

### 11. Open questions and work items

This section is split by **kind**, so that what blocks approval is
visible at a glance: decisions still to make (§11.1), decisions already
taken (§11.2), work items (§11.3), and nits (§11.4). Three statuses are
kept distinct: **design decisions** (settled or open, §11.1 / §11.2),
**specification completeness** (whether every rule is written down,
§11.3), and **implementation readiness** (compiler, interpreter, ABI,
tests, §12). Settling the design does not mean the specification is
complete or the feature is implemented.

#### 11.1 Decisions still to make

**Blocking Lale 1.0.0:** none --- all 1.0.0 design decisions are settled
and recorded in §11.2. The specification is not yet complete (§11.3) and
nothing is implemented (§12).

**For the ranged-view layer and later versions:**

- **Views of `text` / `binary`, and global origins.** Reassignment and
  storage invalidation of `text` / `binary` views lack rules
  comparable to the array rules (§8.3). The escape stance for views is
  decided (option B1): a view never escapes the function that creates
  or receives it, the same as a reference (§8.5). Option B2 ---
  letting a view of a global escape, because a view follows the descriptor
  and survives growth --- is recorded in §8.5 as the principled
  alternative for the view layer to reconsider. Separately, a locally
  held view of a **mutable** global can be invalidated by a call that
  shrinks or reassigns that global, exactly as a reference can (§11.2);
  the view layer must either forbid views of mutable globals or add
  cross-function tracking, mirroring the reference rule.
- **A ranged view of a descriptorless origin.** A view of an `inline`
  field (or another origin with no Lale descriptor) needs a descriptor
  that combines a materialised descriptor with range bounds. The form is
  not specified; the view layer must either define it or reject it.
- **A ranged view of a `ref` value.** A `ref of T` holds a **copy** of
  the origin's descriptor, so a view of the `ref` would have to point at
  that copy. The copy's address is stable only while the `ref`
  variable lives, and reassigning the `ref` would silently re-point
  the view. Either forbid views of `ref` values in 1.0.0 or define the
  rule that the `ref` is frozen while the view lives.
- **Generalising `ref of T` beyond the count-carrying types.**
  Allowing `ref of T` for a scalar (`ref of i32`) or a struct
  (`ref of Sample`) later would be **additive, not breaking**, but it
  would turn `ref` into a general **typed reference** --- effectively
  the typed pointers that §5.14 ("Raw Pointers Only") excludes --- and
  would generalise the reference-variable lifetime bookkeeping to
  every type. **Treat it as its own design.**
- **Resizing through a `ref` parameter --- deferred.** A `ref grow`
  modifier (or a resizing effect in the signature) could add it later
  with the capability explicit at the call site. 1.0.0 does not have
  it (§3.1).
- **Multidimensional ranged views.** For a rectangular array, the
  meaning of `m[a to b]` is open: a view of rows `a..b` (shape
  `(b-a+1) × …`), a flattened element range, or rank-1-only. The view
  descriptor is only `{ descriptor, first, last }` (§8.3), which does not by
  itself express a rectangular sub-shape. Whether `m[i][a to b]` is
  supported, and how its view is represented, must be decided with the
  view layer (§8).

#### 11.2 Resolved --- for the record

- **Reference variables --- option (c): allowed, with per-function
  restrictions.** A reference may be held in a variable, assigned
  between references, and passed --- but never returned or stored
  globally (§3.4). Because Lale has only two scopes, a reference to a
  local cannot dangle _within_ the function through scoping. The
  invalidating events are early frees --- the origin is returned (the
  local becomes moved-from and its slot is reclaimed), reassigned, or
  resized --- and the freeze rule makes them compile errors while the
  reference is live. Both sub-points are settled: a reference is
  **not** implicitly converted to an owning `array of T` --- passing
  one to an owning parameter is a compile error, and the explicit
  spellings are `copy of r` at the call site or a `ref` parameter at
  the definition (§3.4, §4.2); and the freeze keys on **"non-owning
  access was taken"**, not on liveness.
- **A non-owning access freezes its origin.** While a reference to an
  array is live, `grow`, `shrink`, and whole-array reassignment of
  that array are **compile errors**; if the reference names an element
  or field that owns a buffer, reassigning that element or field is
  also a compile error (element-level freeze, below). A ranged view
  freezes `shrink` and reassignment but **survives growth** (§3.4,
  §8.5). The reference half is 1.0.0.
- **References to globals --- module-private, constant-shape only
  (1.0.0).** A reference (writable or `readonly`) to a global array ---
  or a global `text` / `binary` --- is a compile error **unless the
  global is module-private (internal linkage, not exported or imported)
  and its referenced storage is provably never replaced or resized**,
  including any nested sub-object reached through the reference (a
  jagged row, an array-valued element, or a struct field). The danger is
  not that the global dies; it lives for the whole run. The danger is
  that a `grow` / `shrink` / reassignment --- possibly in _another_
  module, or in a function reached through a call --- would invalidate
  the reference, and the per-function freeze cannot see a mutation
  hidden behind a call or a module boundary. Reading a global directly
  needs no reference; only a provably constant-shape module-private
  global may be borrowed. The proof is a **whole-module analysis** over
  every mutation site (each `grow` / `shrink` / reassignment of the global
  or of a nested sub-object reachable through it); it is sound because an
  exported or imported global is categorically un-borrowable, so no other
  module can invalidate the reference. Exported and imported globals
  cannot be borrowed in 1.0.0, because no single module can prove their
  stability.
  A later version may instead track a global's outstanding references
  across calls and modules (option (b)) and lift the restriction; that
  relaxation is additive.
- **Returning a `ref` parameter is rejected (1.0.0).** A `ref of T`
  parameter points at the caller's storage, which the callee cannot
  see, so its origin may or may not outlive the callee. 1.0.0 rejects
  returning a `ref` parameter --- and, more generally, any reference
  whose origin is not locally proven (an FFI `ref` return, §5.2, is
  such an origin). Chosen option (a). Together with
  the previous rule this yields the 1.0.0 invariant: a reference never
  escapes the function that creates or receives it. A later provenance
  / lifetime analysis may relax this --- proving a parameter's origin
  outlives the callee, or expressing it in the signature --- again
  additively.
- **Global initialisers fold fully to static storage.** A module-level
  array's initialiser is a compile-time constant, and the compiler
  emits the **whole value --- its descriptors and every payload --- into
  static storage**, so module load performs no runtime allocation,
  whatever the element type. A global
  `array of text = ["alice", "bob"]` stores the two `text` descriptors and
  their bytes statically; a global `2D array = [[1, 2], [3, 4]]`
  stores one contiguous element block and its shape descriptor
  statically; a global `array of array of i32` stores row descriptors
  and separate row buffers (§2.6, §3.2). This is a compiler task
  (constant folding), not an element-type restriction; a later `grow`
  or reassignment moves the affected buffer to the heap.
- **A `ref` parameter --- element mutation only.** It grants neither
  whole-variable rebinding nor `grow` / `shrink`, uniformly across
  origins (§3.1). It also does not permit resizing an array-valued
  element reached through it, or replacing an element or field that
  owns its own buffer (`grow m[1]`, `rows[1] = row`, `xs[i] = "new"`
  through a `ref` parameter are compile errors). These exclusions are
  what make the rule independent of the call site: a callee cannot
  replace the caller's array or invalidate a reference the caller
  holds.
- **`ref` as a modifier vs. `ref of T` as a type ---
  distinguishable.** The type is `ref of T` (§3.4); the parameter
  modifier stays `ref xs as T` (§3.1). They differ at the second token
  (`of` vs an identifier), so a PEG (a Parsing Expression Grammar ---
  the notation Lale's grammar is written in) orders them
  unambiguously. The only pathological collision is a parameter
  literally named `of` (`ref of as T`), which the grammar rejects at
  that point.
- **Runtime rank-1 initialiser-count mismatch --- a failure.**
  `grow xs by n = f()` with a runtime `n` and a runtime-count rank-1
  initializer fails (Annex B: fatal) whenever the initializer's element
  count differs from `n` (§2.8). Rectangular growth validates the slice
  shape separately.
- **Binding an rvalue array --- a move (option 3).**
  `var a as array of i32 = f()` moves the call's result into `a`;
  binding an lvalue (`var a = b`, `var a = m[1]`) deep-copies. There
  is no copy elision to permit: the language specifies one allocation
  for an rvalue binding, so abort behaviour is deterministic. This
  matches the move already performed on `return` (§3.2).
- **Moved-from detection --- a linear check.** Reading a moved-from
  variable is a **compile error**; cleanup must not free it (§3.2).
  The check is small because `return a` is the only move whose source
  is a named variable (an rvalue binding moves a temporary);
  conditional moves are covered by the same definite-assignment-style
  pass.
- **`unsafe decl` --- cleanup-free element types only in 1.0.0.**
  Recursive destruction of an uninitialised element could follow a
  garbage pointer, so `unsafe decl` is restricted to element types
  that need no cleanup --- including a struct or optional that
  contains a cleanup-needing type (§2.2).
- **`inline` element types --- the C-compatible set.** The fixed-width
  integers and floats, `byte`, `bool`, `pointer`, or a `type` whose
  layout is verified C-compatible. Excluded: `char` (4 bytes, a
  Unicode code point, not C's 1-byte `char`), `f16` (no portable C
  type in 1.0.0), `text` / `binary`, `T?`, and dynamic arrays
  (§4.1). Adding an excluded type later is additive.
- **Empty arrays across the FFI.** A rank-1 array with
  `capacity == 0` has a **null** pointer; after shrinking to zero it may
  retain a non-null buffer when `capacity > 0`. A rectangular array has
  a null pointer whenever any extent is zero. The compiler never calls
  `malloc(0)`. C bindings must guard on `.elements == 0` before
  dereferencing the pointer; a zero-element array has no guaranteed
  non-null dummy pointer (§2.3, §13.5). A C `(NULL, 0)` result can
  represent a valid empty rank-1 Lale array.
- **`copy` on an FFI return --- only when the foreign side retains the
  original.** A transferred allocation must be declared `ref` and
  released with the library's own function; `copy` is for a getter
  that keeps ownership, because otherwise the program would hold only
  the copy and no pointer with which to release the original (§5.2).
  The same rule applies to the text / binary returns
  `copy text of n chars` and `copy binary of n bytes`.
- **Resizing a reference --- always a compile error.** `grow` and
  `shrink` require an owning binding (§3.1); a reference is never
  owning, so the attempt is rejected statically regardless of the
  reference's origin (§4.3).
- **Nested-array mutation target --- normative.** An indexed
  expression used as an assignment or mutation target designates the
  **stored** element, never a temporary copy from an ordinary read; a
  plain read still yields a value copy (§2.8).
- **Array equality --- deep element-wise `==` / `!=`.** `a == b`
  compares the counts, then the elements pairwise, at `O(n)` ---
  consistent with the `O(n)` deep copy of assignment. Elements use
  their own `==` (§3.1).
- **Cross-call invalidation --- closed by construction.** Because a
  `ref` parameter cannot resize or rebind its array, and cannot resize
  an array-valued element reached through it (§3.1, §11.2), a callee
  cannot invalidate the caller's reference or view through a parameter.
  Element mutation is visible but safe. The global case is closed
  separately: a global whose storage may be resized or replaced, or whose
  binding is shared, cannot be borrowed (§11.2), so a called function ---
  in this or another module --- cannot resize it behind a live reference.
  No cross-call analysis is needed (§8.5).
- **The family spelling of the address field --- `ptr`.** `text.ptr`,
  `binary.ptr`, and an array's `ptr` use the same field name; the
  layout is unchanged. `pointer` remains the type name and the `.ptr`
  field is read-only.
- **Source properties are `.elements` and `.dimensions`; ABI fields retain `count`.**
  Array source code uses `.elements` for the total element count and
  `.dimensions` for the dimension-count array; `.rank`, `.ptr`, and
  rank-1-only `.capacity` are also read-only properties. The rank-1 ABI field remains
  `count`, with rectangular `count_i` fields, but Lale code cannot access
  those descriptor names directly. `text` / `binary` keep `.chars` /
  `.bytes`. Capacity has the invariants `capacity >= elements`, `grow`
  never decreases it, and `shrink` never increases it; the exact value
  and growth factor remain unspecified. A view uses `.elements` (or
  `.chars` for a `text` view) for its visible range length, and a `text`
  view's `.bytes` is the byte length of that code-point range. A view
  has no `.capacity`.
- **Text / binary references --- freeze and escape in 1.0.0.** A
  reference to a local `text` or `binary` follows the same escape rule
  as an array, and reassigning the origin while the reference lives is
  a compile error (§3.4, §8.5).
- **Views of array-valued elements --- root freeze.** Taking a
  reference or view of any part of a jagged array (`rows[i]`, an
  `inline` field) marks the root array as `taken`; the root cannot
  grow, shrink, or be reassigned while the access lives (§8.3, §8.5).
  This replaces the earlier "reject the view when the outer can
  reallocate" proposal.
- **A reference to an element that owns a buffer --- element-level
  freeze (option 1).** A reference such as `ref to xs[i]` --- where
  the element is `text` / `binary` / a nested array --- or
  `ref to s.name` for such a field freezes **reassignment of that
  element or field** while the reference lives: reassigning it
  (`xs[i] = "new"`, `rows[1] = row`, `s.name = "new"`) is a compile
  error. The `taken` mark is therefore tracked per element / field as
  well as per root array; the root-freeze rule above still covers the
  descriptor-moving case. Restricting `ref to` to whole variables was
  rejected because it would lose useful element references; accepting
  the hole was rejected because it is a dangling hazard.
- **Stack lowering --- restricted, not assumed safe.** The "count
  never changes" lowering (§7.3) is applied only when the compiler can
  establish that the array's data pointer does not escape the frame;
  otherwise the buffer stays on the heap. It remains best-effort and
  is never a semantic promise, so restricting it cannot change a
  program's meaning.
- **References and views are distinct --- decided.** `ref of T` is a
  direct pointer to the data and freezes growth, shrinkage, and
  reassignment; `view of T` is a pointer to the origin's descriptor,
  follows reallocation, and freezes only shrinkage and reassignment.
  They are not unified into one type (§3.4, §8.3).
- **View parameter passing --- descriptor copy.** A by-value view
  copies the descriptor, not the elements (`O(1)`); writes through it
  are visible to the caller. `readonly view` forbids them, and
  `ref xs as view` is a compile error. There is no `copy` form of a
  view parameter; a view is not implicitly converted to an owning
  array, and the spelling for materialising the viewed elements is
  deferred with the view layer (§8.4, Annex A).
- **A declared `readonly` capability --- accepted for 1.0.0.**
  `readonly ref` (and, with the view layer, `readonly view`) is Lale's
  single declared capability modifier. `ARCHITECTURE.md §5.2`'s
  "infer, don't declare" rule concerns a **variable's** mutability,
  which SSA can see; a reference's capability is a property of the
  _handle_, with no single function body to infer it from. The §5.2
  carve-out is recorded in §13.1. Scoping the modifier to the FFI
  boundary (the earlier `const ref` position) is rejected as an
  asymmetric spelling.
- **No `ref` / `view` inside an array element type --- transitive.** An
  array element type may not contain a `ref` or `view`, even through a
  struct field (§2.12); otherwise an array copy would shallow-copy the
  handles and alias referenced data, bypassing the deep-copy rule. A
  single struct with a `ref` field is still allowed. The excluded pattern
  --- a collection of non-owning handles (a list of views, a cache, a
  symbol table, a graph) --- has 1.0.0 workarounds: own the data (deep
  copy), use a raw `pointer` plus a count (unsafe), or index into a shared
  arena (manual). The safe, ergonomic form (generic containers of
  references) is deferred to 3.0.0 with the stdlib container layer (§10).
- **`may fail` --- deferred.** 1.0.0 is **abort-only** on array
  failure: a `shrink` larger than the count, a runtime
  initialiser-count mismatch, or an allocation failure aborts with a
  clear message. The opt-in recovery annotation (`may fail`) and its
  error-stack integration are deferred to a later version (Annex B,
  §10); adding it later is additive.
- **Descriptor `count`, source property `.elements`.** The ABI keeps
  `count` as the rank-1 descriptor field and `count_i` as rectangular
  per-dimension fields; source code uses `.elements` for the total
  element count and `.dimensions` for the per-dimension counts (§2.3,
  §2.13, §13.4). Diagnostics report `elements`; `size of` loses its `#`.
- **`.elements` and `.dimensions` semantics.** For an array of any rank,
  `x.elements` returns the product of the extents (for a jagged outer
  array, the number of outer elements, not recursive leaf elements).
  `x.dimensions` returns an independent rank-1 array of extent counts. A
  partial index yields a lower-rank rectangular array whose properties
  describe the remaining dimensions; a fully indexed value is a scalar
  and has neither property.
- **Array-based vector and matrix operations.** The specialised vector
  and matrix data types are replaced by the general array family. Their
  supported mathematical operations apply directly to arrays under the
  rank, shape, element-type, and unit contracts in §9.1. Statically
  knowable incompatibilities are compile-time errors; dynamic extent
  incompatibilities are checked at runtime and fail before a result is
  committed. The migration audit remains an implementation requirement
  (§13.8), not a reason to retain parallel storage types.
- **The `rank` field is universal; `capacity` is rank-1 only.** Every
  array descriptor stores a `rank` field --- including rank 1 --- so
  `x.rank` is a read-only, runtime-readable value on every array while
  `x.capacity` is a compile error for a higher rank. A rectangular
  descriptor has **no** `capacity` field at all (not a mandatory field
  fixed at 0). Rank remains fixed by the static type; the stored field
  makes it readable, not changeable.
- **`ref to m[i]` on a rectangular array --- allowed, with root freeze.**
  A reference to a row or plane of a rectangular array is a reference
  into `m`'s buffer (not a reference to a temporary copy), and it marks
  the root `m` as `taken` (§8.5): `m` cannot grow, shrink, or be
  reassigned while the reference lives. Element writes through the
  reference remain allowed.
- **`grow` / `shrink` dimension selection --- `dimension d of m`.** A
  rectangular array is resized with `grow dimension d of m by k = init`
  and `shrink dimension d of m by k`; a rank-1 array keeps the plain
  `grow m by k` / `shrink m by k`. `d` is a compile-time constant in
  `1 .. rank`, and `dimension` is a contextual keyword.
- **The rectangular `grow` initialiser shape.** The initialiser supplies
  the new block directly: it is a rank-`r` array whose extent along the
  grown dimension `d` is `k`, and whose extent along every other dimension
  equals the array's current extent in that dimension. For `k == 1` the
  singleton extent along `d` may be omitted, so the initialiser is written
  as a single `(r-1)`-dimensional slice --- `grow dimension 1 of m by 1 =
[7, 8, 9]` adds a row, and `grow dimension 2 of m by 1 = [7, 8]` adds a
  column. For `k > 1` it is a rank-`r` array of the new block: growing
  dimension 2 (columns) of a `3 × 4` matrix by 2 takes a `3 × 2`
  initialiser, and growing dimension 1 (rows) of a `3 × 4` matrix by 2
  takes a `2 × 4` initialiser. The evaluation order is in §2.8.
- **`rank == 0` is invalid.** A scalar is mathematically a rank-0 tensor
  (a 0-dimensional array), but Lale models scalars as primitive types,
  not as arrays; the `array of T` type has rank >= 1 by construction.
  Rank 0 is rejected as a deliberate modelling choice, not a
  mathematical necessity.
- **Zero-length semantics --- decided.** `[]` is the explicit empty
  rank-1 literal; a rectangular zero-product shape preserves every
  extent, has `.elements == 0` and a null `ptr`; zero extents are checked
  before product overflow; a fill with zero elements does not evaluate
  its expression; and a rectangular resize that creates no element slots
  does not evaluate or shape-check its initializer. A zero-element
  rectangular array that becomes non-empty has **no built-in default**
  element value in 1.0.0: the newly added slice must be supplied by an
  explicit initialiser, which is evaluated and shape-checked against the
  new slice (§2.8).
- **`determinant` unit with a runtime extent.** `determinant` of an
  `n × n` matrix with element unit `U` has unit `Uⁿ`. Because `n` is a
  runtime value, `Uⁿ` is a static Lale unit only when `n` is a compile-time
  constant; `determinant` therefore requires either dimensionless elements
  (unitless result) or a statically-known `n` (§9.1). A unit-bearing
  determinant with a runtime extent is a compile-time error. Runtime-
  dependent units are not introduced for 1.0.0.
- **`inverse` singularity --- zero pivot, no tolerance.** A matrix is
  singular exactly when elimination produces an exactly-zero pivot;
  `inverse` then returns `nothing`. The computed determinant is **not**
  the test --- a zero determinant can result from underflow for an
  invertible matrix --- so "singular" is defined operationally by the
  pivot, not by the determinant (§9.1). No tolerance is applied, and a
  near-singular (ill-conditioned) matrix with non-zero pivots is not
  detected in 1.0.0 (§9.1). This is the same rule LAPACK's `DGETRF` /
  `DGETRI` use; the interpreter and AOT backends implement one
  elimination (partial pivoting, deterministic tie-break) so the
  zero-pivot test is unambiguous.

#### 11.3 Work items

- **The compiler and documentation work** listed in §13.
- **`roadmap.md` / `TODO.md` coherence.** Adopting this design also
  requires the §13.8 re-adaptation, because `roadmap.md` §3.2 row 16 and
  `TODO.md [T-038]` currently describe the rank-1-only model.
- **Possible freeze-rule relaxation (`drop r`).** An explicit release
  annotation that ends a reference's freeze --- so the origin may grow or
  shrink once the reference is no longer needed --- is a candidate future
  enhancement, not a 1.0.0 feature; 1.0.0 keeps the conservative `taken`
  mark (§3.4, §11.2).
- **Consuming release for foreign buffers (deferred).** A release
  operation that consumes a `ref of T` --- so the compiler can reject
  use-after-release by definite assignment --- is a candidate future
  enhancement for FFI buffers; 1.0.0 uses the warning-based contract of
  §5.2.

(The owned / reference operation matrix is now written down as §3.5, and
the IR spelling of the dynamic array is settled as `array<i32>` in
§13.6.)

#### 11.4 Nits

- **`grow` plus `fill` states the amount twice.**
  `grow xs by 3 = [fill 3 elements with 0]` names the amount in two
  places, and the two must agree (§2.8). The redundancy is forced by
  the rule that a `fill` always states its own count (§2.11).
  Recorded; no change proposed.

### Alternatives considered

This section records design ideas that were considered and rejected, with
the reason, so a decision is not revisited without new information.

#### Per-dimension capacity in the rectangular descriptor (rejected)

**Considered.** A `{ count_i, capacity_i }` pair for each dimension of a
rectangular descriptor, keeping the live elements in a compact row-major
layout for C interoperability, so that growing a dimension within its
capacity would avoid a reallocation.

**Rejected.** The compact C-compatible layout is precisely what keeps the
capacity from paying for itself:

- Growing the **first** dimension (rows) appends and would benefit, but
  growing any **other** dimension changes the row-major stride, so the
  live elements must be reshuffled in place --- an `O(n)` element move on
  every resize, spare capacity or not.
- That reshuffle is a **strided** move and cache-hostile, often slower
  than a single contiguous `memcpy` into a fresh buffer. For element types
  that own a buffer (`text`, `binary`, nested arrays) it is worse: a
  per-element, ownership-aware relocation, not a byte copy.
- The capacity saves only the allocator call (malloc/free and page
  faults), not the `O(n)` element movement, which happens either way for a
  non-last dimension.
- Allocating a compact destination and moving surviving elements is simpler ---
  no capacity field, no in-place reshape algorithm to write and test ---
  and is frequently no slower, often faster, than the strided reshuffle.

**Decision.** Rectangular arrays are dense and fixed-shape per value: the
descriptor stores only the per-dimension counts, with **no capacity**, and
every shape-changing `grow` / `shrink` creates a compact destination
buffer when the new element count is nonzero, moves surviving elements,
and destructs elements dropped by the new shape. A zero-product shape
has no payload buffer. Rank-1 `array of T` keeps its
`{ ptr, rank, count, capacity }` descriptor and the standard amortised-append
behaviour, which is a separate, well-understood case.

### 12. Implementation gaps

The items below are **not independent**; they should land roughly in
dependency order: the `is_array_type` predicate rewrite first (it gates
nearly every type check), then the type system and descriptor/ABI
changes, then the semantic checks (root freeze before element-level
freeze; `readonly` variance after `ref of T` is in the predicates), then
FFI boundary work, then the vector/matrix operator migration, and last
the `C` modifier. In particular, `grow` / `shrink` must not land before
the freeze-tracking pass that makes them reference-safe.

- **Struct layout.** `inline array of T of n elements` claims
  byte-for-byte C compatibility, but the interpreter does not yet
  model struct field offsets and sizes:
  `MemoryManager::compute_type_size` hardcodes structs to 64 bytes and
  `IrType::size_bytes()` returns `None` for a struct. This is the
  largest implementation gap for the inline feature.
- **IR / ABI rewrite.** The dynamic array replaces the `[N x T]`
  inline representation in the IR type enum, the **serialisation**
  format (the stored form in which the IR is written out and read
  back), the interpreter's memory manager, and the ABI spec. This is
  substantial work, not a tweak.
- **Constant folding to static.** A global array's initialiser ---
  including nested buffers and `text` / `binary` bytes --- must be
  emitted as static data so module load performs no allocation (§2.6,
  §11.2). Text literals already work this way (`ConstString`); arrays
  and nested values need the same treatment.
- **FFI boundary checks.** The boundary is explicit (§13.5): the
  compiler type-checks the passed pointer and count, and follows
  `lale.md`'s unit rule --- a mismatch against a **declared** unit is
  an error, while a unit-carrying argument passed to a **unitless**
  imported parameter produces a warning (the foreign side cannot
  declare or check a unit). No automatic array-to-parameter expansion
  is performed. Needs platform tests.
- **Array-type predicates.** The compiler recognizes arrays with
  `contains('[')` / `starts_with('[')` in at least ten places (the
  `count` field's validity and unit, `compute_type_size`, type
  compatibility, `get_type_category`, binary operand checks). The new
  spelling has no brackets, so these must be replaced by a structural
  `is_array_type` check; otherwise `array of T` is silently treated as
  a non-array. After the rewrite, a grep of the compiler for
  `contains('[')` / `starts_with('[')` is a **merged-pull-request
  checklist item**: this is the one gap where a silent miss produces
  wrong compilation rather than a loud failure.
- **Read-only descriptor fields.** `text`'s `ptr` / `bytes` / `chars`
  are assignable today (`s.ptr = p` is accepted; the fields are not
  `private`). The read-only rule (§3.4, §7.1) needs a
  semantic-analysis check for the `text` / `binary` and array descriptor
  fields.
- **Element-level freeze.** Semantic analysis must track a `taken`
  mark per element or field that owns a buffer (a nested array,
  `text`, `binary`) and reject reassignment while a reference to it
  lives (§11.2). The root-freeze mark covers the descriptor-moving case;
  the element mark covers the buffer-replacement case.
- **FFI unit warning.** Unit checking currently runs only for
  parameters that declare a unit. An imported parameter that declares
  **no** unit therefore accepts a unit-carrying argument silently; add
  the warning described in §13.5. A mismatch against an imported
  parameter that **does** declare a unit is an error, per `lale.md`.
- **Views and their rules.** Deferred with the ranged-view layer (§8)
  --- the view type, the escape rule, and the freeze tracking.
- **Recoverable allocation failure (`may fail`).** Deferred (Annex B,
  §10); 1.0.0 aborts on failure, so there is nothing to implement here
  for this proposal.
- **Vector/matrix operator migration.** Remove the specialised `vec2` /
  `vec3` / `vec4` and matrix types from the grammar and type system, and
  migrate their literals and constructors to the array forms. Implement
  the array-aware operations of §9.1 --- component-wise unary and binary
  operations, dot and cross products, matrix × matrix, matrix × vector,
  `transpose`, `determinant`, and `inverse` --- in the interpreter and
  the future AOT backends, with identical contracts. Add static and
  runtime shape-check tests covering zero extents, empty operands, and
  physical units.
- **The `C` modifier** --- the export grammar, and making an
  `export C fn` symbol a plain C name.

### 13. Planned specification changes (apply on approval)

None of the following is applied until the design is approved.

#### 13.1 `ARCHITECTURE.md §5.2` --- add a `#### Declared capabilities` subsection

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

#### 13.2 `ARCHITECTURE.md §5.14d` --- amend the read-only-reference note

Rename the subsection (currently
`#### Deferred to 2.0.0 — read-only reference`) to
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

#### 13.3 `ABI_SPECIFICATION.md §2` --- type layout

Replace the array row with two rows:

| Type                                           | Size / alignment                                                                                 |
| ---------------------------------------------- | ------------------------------------------------------------------------------------------------ |
| `array of T` (rank 1)                          | 32 bytes --- `{ ptr, rank, count, capacity }`, 8-byte aligned; `count` is exposed as `.elements` |
| `2D array of T` / higher ranks                 | rank-dependent descriptor: `{ ptr, rank, count_i × rank }`, 8-byte aligned                       |
| `inline array of T of n elements` (field-only) | `n * sizeof(T)` contiguous bytes, alignment of `T`                                               |

#### 13.4 `ABI_SPECIFICATION.md` --- new `§3.2`, the dynamic array

The canonical language-level layout is §7.1; this section is the ABI
target and must not drift from it.

```text
array of T       = { ptr: pointer, rank: u64, count: u64, capacity: u64 }
2D array of T    = { ptr: pointer, rank: u64, count_1: u64, count_2: u64 }
rD array of T    = { ptr: pointer, rank: u64, count_1: u64, ..., count_r: u64 }
```

- For rank 1, layout is `ptr` at offset 0, `rank` at offset 8,
  `count` at offset 16, `capacity` at offset 24 (32 bytes). For rank
  `r > 1`, the descriptor contains the rank and `r` per-dimension counts
  (no capacities). Exact offsets and padding are fixed by rank and
  specified by the ABI. The stored rank must agree with the static type;
  a mismatched rank is rejected at any boundary that accepts a Lale array
  descriptor, including a Lale-ABI call across compiled-module boundaries.
  The documented C ABI in §13.5 passes a data pointer and scalar counts,
  not this descriptor, so that interface does not receive or validate the
  stored rank field. Rank is never dynamically changed.
- `count_i` is the live size of dimension `i`. `.elements` is zero when
  any extent is zero; otherwise it is the product of all `count_i` values.
  `.dimensions` returns those values in a new, independent rank-1 array,
  so `.dimensions[i]` is `count_i`. The runtime checks for zero extents
  before multiplying, then checks nonzero products and byte sizes for
  overflow. The buffer
  holds exactly `product(count_i)` element slots in compact row-major
  order, so `ptr` exposes a dense C-compatible block. All count fields
  use element-count units where applicable; units do not affect layout.
- A rectangular array has **no spare capacity**: resizing any
  dimension to a nonzero-product shape allocates a fresh compact
  row-major buffer, moves surviving elements into their corresponding
  indices, destructs dropped elements, and releases the old buffer only
  after the transfer is safe. A zero-product result has a null `ptr` and
  no payload allocation. Allocation failure follows Annex B; the old
  value must not be corrupted before the fatal abort.
- An `array of T` **owns** its buffer (compiler-managed, freed at
  scope exit).
- A `ref of array of T` to a descriptorless origin (an `inline` field or
  an FFI `ref` result) is non-owning. For rank 1, `capacity = count` is
  synthetic metadata, not a claim about the foreign allocator's true
  allocation. A zero-count reference therefore preserves the origin
  pointer verbatim (the runtime does not null it) even though its capacity
  is zero; the owning-array null-pointer invariant does not apply. A
  reference cannot resize.
- A `ref to` an owning rank-1 array copies the origin's descriptor, so
  its `capacity` is the origin's capacity (which may exceed `count`);
  the reference still cannot grow. A `ref to` a rectangular array
  copies its counts (there is no capacity to copy).
- A `ref of text` / `ref of binary` reuses the `text` / `binary`
  descriptor of §13.3 unchanged --- `{ ptr, bytes, chars }` /
  `{ ptr, bytes }`; there is no array-style `capacity` field.
- An `inline array of T of n elements` field has **no** descriptor: its
  elements are inline bytes, and field access yields a reference.
- Descriptor fields are **read-only** from source code. For rank 1,
  `capacity >= elements`, `grow` never decreases it, and `shrink` never
  increases it. Rectangular arrays (rank greater than one) have no
  capacity and therefore no capacity invariant. Descriptor layout is
  fixed for each rank so independently compiled Lale modules agree.

#### 13.5 `ABI_SPECIFICATION.md §4` --- FFI boundary (two layers)

The boundary is specified in two layers, not as an automatic conversion:

1.  **Lale's internal representation** --- the array descriptor (§13.4).
2.  **The declared foreign signature** --- the actual C parameters and
    return type, as written in the `import fn signature` / `export C fn`
    declaration.

There is no third "marshalling" layer --- no automatic machinery that
converts a Lale value into a different foreign form --- that expands one
Lale argument into several C parameters. The programmer passes each
foreign parameter explicitly:

```lale
import fn signature c_sum(data as pointer, count as u64) returns i32

var xs as array of i32 = [10, 20, 30]
write c_sum(xs.ptr, xs.elements)
```

The ordinary argument-count and type rules then apply; a call that tries
to pass the array in place of both declared parameters is an arity
error, not a hidden expansion. `xs.ptr` is the address of the element
buffer and `xs.elements` is its element count. This is the same idiom as
`s.ptr, s.bytes` for `text` and `b.ptr, b.bytes` for `binary`.

**Units at the boundary.** `xs.elements` carries `<elements>`. Unit
checking follows `lale.md`: a mismatch against a **declared** unit is an
**error**, whether the function is defined in Lale or imported. A C
count parameter normally declares no unit, so there is no declared unit
to mismatch; in that case the compiler emits a **warning** that a
unit-carrying value is passed to a unitless foreign parameter, because
the foreign side cannot check it. The same warning applies to
`text.bytes` (`<bytes>`) and `binary.bytes` when passed to unitless C
parameters. If the binding author wants a checked contract, they write
the unit in the import signature (`count as u64 in <elements>`), and a
mismatch then becomes an error like any other declared-unit mismatch.

```text
WARNING: unit-carrying value passed to a unitless foreign parameter
  `xs.elements` has unit `<elements>`; parameter `count` of `c_sum` declares no unit
  The foreign side cannot check a unit, so this mismatch is not enforced.
  To enforce it, declare the unit in the import signature:
    import fn signature c_sum(data as pointer, count as u64 in <elements>) returns i32
```

The reverse direction is not a C return type at all:
`returns ref of array of i32 of n elements` is an **FFI descriptor**
(reference + count binding). The same shape serves the other
count-carrying types: `returns ref of binary of n bytes`,
`returns copy binary of n bytes`, `returns ref of text of n chars`, and
`returns copy text of n chars` (§5.2).

A **rectangular** array is returned the same way, with the descriptor
naming each extent in row-major order. This is supported in 1.0.0:

```lale
// C: const double* lib_matrix(size_t rows, size_t cols);   // library-owned; caller must not free
import fn signature lib_matrix(rows as u64, cols as u64) returns readonly ref of 2D array of f64 of rows by cols elements

var m as readonly ref of 2D array of f64 = lib_matrix(2 as u64, 3 as u64)   // non-owning, read-only; extents 2 × 3
write m[1][2]

// C: double* lib_alloc_matrix(size_t rows, size_t cols);   // library-allocated; caller releases with lib_free
import fn signature lib_alloc_matrix(rows as u64, cols as u64) returns ref of 2D array of f64 of rows by cols elements

var n as ref of 2D array of f64 = lib_alloc_matrix(2 as u64, 3 as u64)   // non-owning reference; extents 2 × 3
lib_free(n.ptr)                                                           // the program frees the ORIGINAL with the library's deallocator
```

The Lale side receives a rectangular array whose `.dimensions` is
`[rows, cols]`. A zero extent is permitted and produces a zero-product
shape with a null payload pointer (§2.7, §2.3); the C side must still
guard on the extents before dereferencing. For a zero-product rectangular
result the foreign function **must return null** and must **not** leave
an allocation for the caller to release: Lale sets the descriptor's `ptr`
to null whenever an extent is zero (§2.7), so a non-null returned pointer
would be lost and could never be released through the library's
deallocator.

A `type` with `inline array of T of n elements` fields has a
C-compatible layout (inline bytes) and needs no conversion on either
side --- but "C-compatible" means its layout, field types, and calling
convention must all match the foreign declaration; it is not
automatically safe across every boundary. These conversions should not
be described as universal ABI compatibility until the mappings are
defined and **tested for the target ABI**.

**The supported 1.0.0 baseline (concrete).** To keep the boundary
predictable, 1.0.0 supports:

- a declared **pointer parameter** ← the array's `ptr`;
- a separate declared **count parameter** ← the array's element count
  (`xs.elements`), using the exact integer type written in the foreign
  signature (in 1.0.0, `u64` --- C's `size_t` on 64-bit hosts);
- a **zero-element rank-1 array** ← its pointer (null when
  `capacity == 0`) and count `0`; after a `shrink` it may retain a
  non-null buffer when `capacity > 0`. A **zero-element rectangular
  array** has a null pointer whenever any extent is zero. In either case,
  the C side must guard on the element count before dereferencing the
  pointer; Lale does not promise a non-null pointer for zero elements (§2.3);
- a **struct with `inline array of T of n elements` fields** ← its
  inline bytes, only when the layout, field types, and calling
  convention match the declared C type;
- an **array return** ← only through an explicit FFI descriptor
  (`returns ref of array of i32 of n elements` /
  `returns copy array of i32 of n elements`), never a bare C return
  type;
- a **text / binary return** ← `returns ref of text of n chars` /
  `returns ref of binary of n bytes` (non-owning), or
  `returns copy text of n chars` / `returns copy binary of n bytes` (a
  Lale-owned deep copy). The `copy` restriction of §5.2 applies. A
  `text` return scans for the null terminator and is for trusted sources
  only; for untrusted data, prefer `binary of n bytes` (§5.2);
- a **read-only reference return** ←
  `returns readonly ref of array of i32 of n elements`,
  `returns readonly ref of text of n chars`, or
  `returns readonly ref of binary of n bytes` --- the same descriptor as
  the mutable reference, with writes through it forbidden (§3.4,
  §5.3).

For a rank-prefixed rectangular array, `m.ptr` addresses the contiguous
row-major `T` block of `m.elements * sizeof(T)` bytes, with no spare
capacity (§2.7). The total element count alone does not preserve a
zero-sized shape: if the C API needs to distinguish shapes such as
`3 × 0` and `0 × 5`, the binding must pass the individual values in
`m.dimensions` separately. The `.dimensions` property itself is a Lale
array value; the binding passes the required scalar extents explicitly.
After any successful resize, the buffer is reallocated and the live
elements are copied into a new compact C-style rectangular block, so
`m.ptr` again exposes the current shape. For a jagged
`array of array of T`, the outer buffer contains row descriptors and is
not a flat `T` buffer.

A plain `export fn` --- a **Lale-ABI** export --- may declare
`array of T` parameters and returns; those are called by other Lale
modules, not by C. A **C-ABI** export (`export C fn`) must use
C-compatible types only. Everything else --- a bare-pointer parameter, a
`{ptr, len}` struct, a library descriptor --- needs its own explicit
rule before it is supported.

#### 13.6 `ARCHITECTURE.md §4.7` --- IR serialisation

- Add a dynamic-array `ir_type` form (the Lale source spelling is
  settled as `array of T`, §2.9).
- `[N x T]` is retained, now denoting the `inline` field type
  (field-only).
- The IR spelling is **`array<i32>`** --- the source's `array of i32`
  written in the IR's existing parametric convention, matching
  `ptr<T>`. The rank-1 layout is the 32-byte
  `{ ptr, rank, count, capacity }` descriptor of §13.4; the spelling is
  only how the type is written in the serialised IR. A rank-prefixed form
  (`2D array<i32>` and up) carries the rank and per-dimension counts,
  with no capacities (§13.4).

#### 13.7 `lale.md`

- Document the array type: 1-based indexing, complete initialisation,
  `.elements`, `.dimensions`, units, `grow` / `shrink`, and the
  value-semantics rules.
- Document the read-only source properties --- `ptr`, `rank`, `.elements`,
  `.dimensions`, and `.capacity` for a rank-1 array (`ptr`, `rank`,
  `.elements`, and `.dimensions` for a rectangular array, with no
  capacity); `ptr`, `bytes`, `chars` for `text`; `ptr`, `bytes` for
  `binary` --- and the rank-1 capacity
  invariants (`capacity >= elements`; `grow` never decreases it; `shrink`
  never increases it).
- Document that a reference to a local `text` / `binary` follows the
  same escape and freeze rules as an array (§3.4).
- Document the **escape rule** for references: a reference never
  escapes the function that creates or receives it --- a reference to
  a local or a global array / `text` / `binary`, or a `ref` parameter,
  must not be returned or stored in a global (§3.4, §8.5). (Views
  follow §8.5.)
- Document that a global array's **constant initialiser folds to
  static storage**, including nested buffers and `text` / `binary`
  bytes, so module load performs no runtime allocation (§2.6).
- Update **Parameter Passing** for arrays: copy by default, `ref` for
  element mutation only (no rebinding, no resizing, no replacing an
  owned-buffer element), `copy` for an explicit copy, and the
  definition-site warning for an array parameter copied at every call
  (§3.1).
- Add a **note for experienced programmers** that a by-value
  **handle** parameter --- `ref of T` or, later, `view of T` ---
  copies only the descriptor / descriptor, not the elements, so writes
  through it are visible to the caller (§3.4, §8.4, §9). This is the
  single most surprising rule in the design for anyone arriving from
  value-semantics intuition or from Python/Ruby, where "slice" means a
  copy.
- Add a **note for experienced programmers** on the matrix-multiplication
  spelling: `*` between two rank-2 rectangular arrays is matrix
  multiplication, and `matrix * vector` is valid, but `vector * matrix`
  (a rank-1 array on the left) is **rejected** --- no implicit
  row-vector operation exists, unlike NumPy where a 1-D array broadcasts
  as a row or column. Use `dot` / `cross` for vectors, or reshape
  explicitly (§9.1).
- Document **binding**: `var a as array of i32 = f()` moves the call's
  rvalue; `var a as array of i32 = b` and `= m[1]` copy (§3.2).
- Document the array **type notation** (§2.9--§2.13): `array of T`,
  the rank prefix, the extent clause, and the count idiom. Today
  `lale.md` documents only the bracketed spellings.
- Rewrite the existing array examples: `i32[10]` → `array of i32`;
  `[fill with 42]` → `[fill 10 elements with 42]`;
  `unsafe decl buffer as i32[1000]` →
  `unsafe decl buffer as array of i32 of 1000 elements`; the
  diagnostic `length=5` / `count=5` → `elements=5` (§13.13, §13.14).
- Fix the operator table: `count of` is **removed** in favour of the
  array's read-only `.elements` and `.dimensions` properties (`count`
  remains an ABI field name; `t.chars` / `t.bytes` for `text`, `b.bytes`
  for `binary`). `#size of` becomes the runtime
  `size of` returning `u64 in <bytes>`; document the semantic change
  (§2.13, §13.14).
- Document `inline` fields and the rendering rules.
- Update the **C FFI** section: an array crosses C as `ptr` + `count`,
  never as one argument; an array / `text` / `binary` return uses a
  `ref` / `copy` / `readonly ref` descriptor with an extent clause
  (`of n elements` / `of n bytes` / `of n chars`) (§5.2, §13.5).
- Add a **Views** subsection when the ranged-view layer lands (§8) ---
  including the code-point indexing rule for `text`.
- Add a **note for experienced programmers** on why Lale writes
  **`readonly`** where they expect `const`. The point is not a design
  principle but the **absent guarantee**: `readonly` promises only
  that _the holder may not write through this reference_ --- it does
  **not** promise that the data is constant, because the owner,
  another (writable) reference, or a foreign library can still change
  it. `const` would suggest an immutability Lale cannot deliver;
  `readonly` names the permission that is withheld.

#### 13.8 Migration audit --- vector/matrix operations, `roadmap.md`, and `TODO.md`

Before the specialised vector and matrix types are removed, inventory every
operation currently attached to them in `ARCHITECTURE.md §5.20`, the
compiler, examples, and tests. For each operation, record its operand
rank, extent constraints, element-type requirements, physical-unit rules,
operator spelling, and whether a failed shape check is static or runtime.
Migrate every supported operation to array operands; do not silently drop
an existing operation because its old signature mentioned `vec2`, `vec3`,
`vec4`, or a matrix type. The contracts in §9.1 are the minimum semantic
requirements. In particular, preserve dot and cross products, scalar-array
operations, matrix × matrix, matrix × vector, `transpose`, `determinant`,
and `inverse` with the shape checks in §9.1; retain the existing rule
that plain `*` between two vector operands is not an implicit dot, cross,
or component-wise product, with matrix multiplication as the one deliberate
exception, spelled `*` between two rank-2 numeric rectangular arrays
(matrix × matrix) and between a rank-2 numeric rectangular array and a
rank-1 numeric array (matrix × vector). `inverse` returns the optional
form of the rank-2 floating-point array: `nothing` on a singular matrix. Update `ARCHITECTURE.md §5.20` and `lale.md` to
remove the obsolete specialised type forms and document the replacement
array-aware operators.

`[T-009]` (built-in `matN` / `matNxM` types) is superseded by this design:
there is no separate matrix type, so the item is retired rather than
implemented, and its operations migrate to the array-aware forms above.
`roadmap.md §3.2 item 11` (Matrix types) is removed or reworded to point
at §9.1. `roadmap.md §3.2 row 16` and `TODO.md [T-038]` also describe
the rank-1-only array model and must be brought into line with this
design.

This reconciliation is **not** done at sign-off: it lands **with the
implementation change**, for the same reason as §13.15 --- retiring
`TODO.md` items and rewording `roadmap.md` while the old grammar still
works would leave the trackers describing neither state.

#### 13.9 `ARCHITECTURE.md` --- add a terminology note: "reference", not "borrow"

Add a short note in the language-design-decisions part (for example
beside §5.14's pointer rules) explaining why Lale avoids the established
word **borrow**:

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

#### 13.10 `ARCHITECTURE.md §5.20` --- state the `of` rule once, for the family

Update the existing vector-specific grammar section, removing the obsolete
vector constructor grammar and replacing it with the general type-constructor family:

```markdown
#### The `X of T` constructor family

`of` is part of a **two-word constructor** — `array of T`, `ref of T`, and `view of T` —
and each constructor takes a **type** as its argument. Chained forms therefore nest to the right by
construction (`ref of array of i32` is `ref of (array of i32)`) and need no associativity rule. The
specialised `vec2 of T`, `vec3 of T`, `vec4 of T`, and matrix type constructors are removed in favour
of rank-1 and rank-prefixed arrays (§9.1).
```

Update `ARCHITECTURE.md §5.20` and `lale.md` to remove the vector-specific
`of` constructor grammar and document the remaining type-constructor family.

#### 13.11 Grammar (`src/grammar/lale.pest`) --- the new syntax

The 1.0.0 array surface is not only `array of T`; the grammar needs
every form this proposal introduces. The branch already carries the
array-type and extent rules; the rest is still to add.

- Add `array_type` --- `array of T`, plus the rank prefix `2D` / `3D`
  / ..., as a `type_name` alternative. (Already on the branch.)
- Remove the bracketed array type spellings `i32[5]` and `[i32]` from
  `type_name` **with the implementation** (§13.15). `arr_index` keeps
  indexing and array-literal roles, with `[]` the empty literal and
  `[0]` a one-element zero array.
- Add the **extent clause** `of N elements` (`of 5 by 3 elements` for
  more than one dimension), with a word-boundary guard on `elements`.
  The same clause admits `of n bytes` for
  `returns ref of binary of n bytes` and `of n chars` for
  `returns ref of text of n chars` (§5.2); both suffix words need the
  same guard. (Already on the branch.)
- Add `inline` as a **field-only** type modifier in `type_field`. The
  form is `values as inline array of i32 of 3 elements`. It must be
  rejected in a `var`, a parameter, and a return type (§4.1). The
  grammar may parse it in those positions and let semantic analysis
  reject it with a clear message, or restrict the production to
  `type_field`.
- Add `ref of T` and `view of T` as `type_name` alternatives, and
  `readonly` as an optional prefix on either. `view of T` parses even
  though the view layer is deferred, so semantic analysis can report
  "not implemented yet" rather than a syntax error.
- Add `ref to x` as an expression form (the value side of a
  reference). It must be distinguishable from the `ref` parameter
  modifier (`ref xs as T`), which it is at the second token (`to` vs
  an identifier). The one pathological case is a parameter literally
  named `to` (`ref to as T`), which the grammar rejects, mirroring the
  `ref of as T` collision in §11.2.
- Add `copy of x` as an expression form --- the value side of the
  reverse direction (`ref to` makes a reference; `copy of` makes an
  owned copy). It is distinguishable from the `copy` parameter
  modifier (`copy xs as T`, a bare `copy` in a parameter list) and
  from the `X of T` type family, because `copy of` is followed by an
  **expression**, not a type (like `size of`, §13.14).
- Extend `return_type` for an `import fn signature`: an optional `ref`
  / `copy` / `readonly ref` before the type, followed by the extent
  clause. A `copy` return is only valid on an imported declaration.
- Add the `grow` / `shrink` statements: `grow xs by n = initialiser`
  and `shrink xs by n`, plus the rectangular forms
  `grow dimension d of m by k = initialiser` and
  `shrink dimension d of m by k`, with `by` and `dimension` contextual.
  Do **not** add `may fail` in 1.0.0 (deferred, Annex B); if the grammar
  accepts it early, semantic analysis must reject it with a clear message.
- Replace `#size of` with a runtime `size of` operator, and remove
  `#count of` (§13.14).
- The rank token is recognised **only inside** `array_type`: there is
  no separate lexer pass, so outside a type position `2D` still reads
  as `2` followed by `D`.
- The nested spelling `array of array of …` parses and is accepted for
  jagged arrays; rectangular arrays use the rank-prefixed form (§2.7,
  §2.12).

#### 13.12 Semantic analysis --- distinguish rectangular and jagged arrays

Do not reject an array whose element type is itself an owning array.
`array of array of T` is the explicit jagged representation;
`2D array of T` is the contiguous rectangular representation. Semantic
analysis must preserve the distinction and reject mismatched initialiser
shapes for rank-prefixed rectangular arrays. The separate restrictions
on arrays whose element type is a reference or view (§2.12), and on
parameter modifiers for a non-owning type, remain unchanged. Semantic
analysis must permit dimension-specific resizing of rectangular arrays
and ensure that the rank in a descriptor agrees with the static type.
It must also implement the array-aware vector/matrix operator contracts
in §9.1: validate operand rank, static shape constraints, element types,
and units; emit runtime extent checks where dimensions are dynamic; and
reject incompatible shapes before payload access or result commitment.
A zero extent makes the total element count zero without product
overflow, but it does not waive an operation's shape constraints. A
zero-count fill does not evaluate its element expression; rectangular
resizes that create no slots skip initializer evaluation and shape
checking, while a zero-to-non-empty resizing has no built-in default and
requires the newly added slice to be supplied by an explicit initialiser
(§2.8). The form
`fn f(ref xs as view of T)` is a semantic error, because a view is
already a non-owning handle; `fn f(copy xs as view of T)` is a semantic
error too, because a plain by-value view already copies the descriptor;
and `ref` / `copy` add nothing to a `ref of T` value either. The
diagnostic points to `readonly view` for the no-write form and to the
deferred materialisation spelling for an owned copy **of a view**.

Passing a `ref of T` value to an owning `array of T` parameter is
likewise a semantic error --- a reference is not implicitly converted to
an owning array (§3.4) --- and the diagnostic names both fixes:

```text
ERROR: type mismatch in argument 1 of `sum`
  `s.values` has type `ref of array of i32` (a non-owning reference, §3.4)
  parameter `xs` has type `array of i32` (an owning array)
  A reference is not implicitly converted to an owning array (no implicit conversions, §9).
  To pass an owned copy at the call site:  sum(copy of s.values)           — O(n) copy
  To pass without copying (needs the definition):  fn sum(ref xs as array of i32)
```

#### 13.13 `ARCHITECTURE.md §5.10` --- the array initialiser's value grammar

`[fill with 0]` becomes `[fill 10 elements with 0]`. A `fill` states its
own count (§2.11), because otherwise it would read the count from the
declaration --- a second source of truth. A zero-count fill does not
evaluate its expression (§2.2). `[]` is the explicit empty-array
literal, and `[0]` is a one-element zero array.
This is **expression** syntax, not a type change.

#### 13.14 Diagnostics and operators --- element count and shape

- Diagnostics report the total as `elements`:
  `array index out of bounds (index=6, elements=5)` (§2.4), replacing
  `size` in compile-time messages and `length` in runtime ones.
- `#count of` is **removed**. Array source code uses `.elements` for the
  total element count and `.dimensions` for a rank-1 array containing
  the per-dimension counts. `count` remains the rank-1 descriptor's ABI
  field name; rectangular descriptor fields remain `count_i`.
- `#size of` becomes the runtime `size of` returning `u64 in <bytes>`
  (§2.13). It is not a compile-time-only intrinsic; it may be folded
  to a constant when the value is known.
- `#type of` and `#unit of` are unchanged.
- **This is a semantic change, not only a rename.** `size of` now
  reports the shallow **content** size --- `.elements × sizeof(T)` for
  an array or an array view, `.bytes` for `text` (or a text view, the
  encoded byte length) and for `binary` (or a binary view) --- not the
  descriptor. Existing `#size of`
  tests that expect the descriptor size, or that assume a compile-time
  constant for a dynamic array, must be updated; `#count of` tests must
  migrate to `.elements` / `.dimensions` (`t.chars` / `t.bytes` for
  `text`, `b.bytes` for `binary`). `size of` carries `<bytes>`;
  `.elements` and the extent values in `.dimensions` carry `<elements>`.

#### 13.15 Migration sequencing --- the removals land with the implementation

The bracketed spellings (`i32[5]`, `[i32]`) are **not** removed at
sign-off. They are removed **with the array implementation**, in the
same change that lands the new spelling. The reason: the bracketed forms
are the only array spellings that work today, and §2.1 abolishes the
fixed-size array --- so removing them first would leave the test suite
and `examples/complex.lale` without array coverage rather than migrating
them. Sign-off freezes the **design**; the code migration is a separate,
subsequent change. Its scope is the grammar (array type and extent
clause), the AST --- the **Abstract Syntax Tree**, the compiler's
tree-shaped internal copy of the program --- (`array_element`, rank),
the **symbol table** (the compiler's record of names --- variables,
functions, types --- and their properties) and type representation, the
IR type and serialisation, the interpreter's memory manager, the ABI
specification, the type predicates that test for `[`, the diagnostic
vocabulary, and the tests / examples that use the bracketed forms.
`doc/array_type.md` §8.2 is the historic record.

#### 13.16 `builtins.lale` / IR --- read-only descriptor fields

The descriptor fields of `text`, `binary`, and the dynamic array are
**read-only** from Lale code (§3.4, §7.1). The `text` / `binary`
definitions in `builtins.lale` and the array descriptor description stand;
semantic analysis must reject assignment to these fields (`s.ptr = p`,
`xs.elements = n`). `private` is not sufficient, because FFI code must read
the address.

`IR_SPECIFICATION.md §1` spells the `text` struct as:

```text
%text = type { ptr: ptr, bytes: u64, chars: u64 }
```

The family field name is now `ptr` (§11.2), so this example already
matches. The dynamic array's IR spelling is settled as `array<i32>`
(§13.6, §11.3). For **rank 1**, its descriptor fields are `ptr`, `rank`,
`count`, and `capacity` at the ABI offsets; a rectangular descriptor
instead carries `ptr`, `rank`, and the `count_i` extents with no
capacity. The source-level `.elements` property maps to `count` (rank 1)
or the product of the `count_i` values (rectangular). The layout (offsets
and sizes) does not change.

## Annex A --- View parameter passing (resolved)

A view is a non-owning **handle**: `{ descriptor_ptr, first, last }`.
Passing it by value copies the handle (the descriptor), not the
elements. The cost is `O(1)`; the callee still views the caller's
storage, and writes through it are visible to the caller.

```lale
var data as array of i32 = [10, 20, 30, 40, 50]
var mid as view of array of i32 = data[2 to 4]

fn sum_view(xs as view of array of i32) returns i32
    // xs is a copy of the descriptor; it still views data[2..4]
    ...
end fn

fn bump(xs as view of array of i32) returns nothing
    loop var i as u64 from 1 to xs.elements
        xs[i] = xs[i] + 1      // writes into data
    end loop
end fn
```

- **By value:** `O(1)` descriptor copy; no element copy; writes are
  visible to the caller.
- **`readonly view of T`:** the same `O(1)` copy, but no writes
  through the holder.
- **`ref xs as view of T`:** a **compile error**. A view is already a
  non-owning handle; `ref` adds nothing and would hide the aliasing.
  Use `readonly view` to forbid writes.
- **No `copy` form.** A plain by-value view parameter already copies
  the descriptor, so `copy` would add nothing. A view is not
  implicitly converted to an owning `array of T`. `copy of` is the
  call-site operator that materialises an owned copy of a
  **reference** (§3.4); whether it should also materialise a **view**
  (a sub-range --- a shape change) is deferred with the view layer.

This closes the former Option P / Option R fork: the parameter keeps the
view type, and there is no hidden materialisation. The explicit
owned-copy path is deferred with the view layer; `copy` is not an
acknowledgement on a view parameter.

## Annex B --- Fallible allocation and resizing

Array construction, `grow`, and `shrink` can fail. This annex specifies
the failure causes and the **1.0.0 behaviour: fail fast**. Recoverable
failure (`may fail`) is deferred to a later version; the sketch below is
kept so the deferred design is not lost.

### The failure causes

1.  **A negative amount cannot arise.** The amount is
    `u64 in <elements>` and Lale has no implicit numeric conversion, so
    a negative literal is a compile-time type error and a runtime
    negative value is not expressible (§2.8). `grow` / `shrink` never
    silently reverse direction.
2.  **A `shrink` amount larger than the resized extent.** For a rank-1
    array or jagged row, `shrink xs by n` with `n > xs.elements` is a
    failure --- it never silently underflows the stored `count`. For a
    rectangular array, the amount is compared with the selected
    dimension's extent, not the total `.elements` value. A **constant**
    violation is a compile-time error when the relevant extent is known
    at compile time; a runtime violation is a runtime failure.
3.  **An allocation failure (out of memory).** An array literal, a
    positive-count `fill`, a `copy` return, or a deep copy may need to
    allocate; rank-1 `grow` may need a larger buffer; and a rectangular
    shape-changing `grow` or `shrink` whose result has a nonzero element
    count needs a new buffer. If allocation cannot be served, that is a
    failure. Rank-1 `shrink` does not allocate, and a rectangular resize
    whose result has a zero extent does not allocate a payload buffer.
4.  **A grow-initializer mismatch.** For rank-1 `grow xs by n = f()`,
    a runtime `n` and a runtime-count initializer cause a failure when
    the initializer's element count differs from `n` (§2.8, §11.2); a
    statically known mismatch is a compile-time error. Rectangular growth
    instead checks the initializer against the new-block shape of §2.8
    and §11.2. The zero-extent rules in §2.8 take precedence and skip
    initializer evaluation and shape checking in the specified cases.
5.  **A zero-to-non-empty resize without a matching initialiser.** A
    rectangular resize that changes a zero-element array into a non-empty
    array must be supplied an explicit initialiser for the newly added
    slice; Lale 1.0.0 has no built-in default element value. If the
    initialiser's shape does not match the new slice, the resize fails
    before the target is modified. If static shape analysis proves the
    mismatch, it may be rejected at compile time; otherwise the failure
    is detected on the runtime path (§2.8).

An amount of **zero** is neither: `grow xs by 0` and `shrink xs by 0`
are **no-ops**.

A **huge amount** --- for instance a negative `i64` reinterpreted as
`u64` through `unsafe bitcast` --- is not a signedness hole: it becomes
an enormous allocation that fails as an out-of-memory abort (cause 3),
never a silent negative growth. The wrap-around is the intended,
`unsafe`-gated behaviour.

### Default: fail fast

A failure **aborts** the program with a clear message, and the target
array is left unchanged up to the abort (it is never corrupted). An
array-initialising `var` aborts before the variable is bound. There is
no recovery annotation in 1.0.0.

```lale
grow xs by 2 = [7, 8]   // if this cannot be done, the program aborts
shrink xs by 3          // if xs has fewer than 3 elements, the program aborts
```

### Deferred: opt-in recovery (`may fail`)

The **`may fail`** annotation (a contextual two-word form, meaningful on
`grow` / `shrink` and on an array-initialising `var`) is **not part of
1.0.0**. The deferred design is:

- `grow` / `shrink`: the target is left **unchanged** --- a **no-op**;
- a `var` construction: the variable is bound to an **empty** array
  (see below);
- an **error is recorded on the error stack**
  (`ARCHITECTURE.md §5.18`);
- the **program continues**.

```lale
// Deferred — not valid in 1.0.0.
grow xs by 2 = [7, 8] may fail   // on failure: xs unchanged, an error recorded, execution continues
shrink xs by n may fail          // n larger than xs.elements → no-op + error
```

**Construction is fallible too.** The same deferred rules would cover an
allocation performed by an array-initialising declaration:

```lale
var xs as array of i32 = [fill 1000000 elements with 0] may fail
```

On failure, the error would be recorded and `xs` bound to an **empty**
array (count 0, capacity 0), so the variable is definitely assigned and
execution continues. This is the declaration equivalent of the no-op
rule for `grow` / `shrink`. In 1.0.0, a construction that cannot
allocate aborts before the variable is bound.

The annotation is **declarative, not a command**: it states that the
operation may fail, rather than naming an action to take. It is also
**distinct from `?`**: `?` propagates an absent optional _out of the
enclosing function_ (an expression concern), whereas `may fail` would
tolerate a failure _of this statement_ and let execution continue in
place. The error stack would make a tolerated failure inspectable
(`has errors` / `last error`), so a program could react to a failure it
chose to survive.

### Conditional growth: an optional initialiser

A `grow` whose initialiser evaluates to `nothing` is **skipped** --- no
failure is involved:

```lale
grow xs by n = try_read(n)   // try_read returns array of i32?; `nothing` ⇒ the grow is skipped
```

This is the single-statement way to grow _conditionally_. With `if` /
`when` (statements), a guard block is usually clearer:

```lale
when extra > 0
    grow xs by extra = read_extra(extra)
end when
```

Because a negative amount cannot arise (the amount is
`u64 in <elements>`, §2.8), such a guard is **not required for safety**
--- the language never silently grows by a negative amount --- it is a
matter of intent.

### Interaction with nested arrays and partial copies

`grow` on a nested array follows the same rules for the inner array and
leaves the outer array untouched (§2.7). If a deep copy must allocate
several buffers and an allocation fails partway, the buffers already
allocated for the copy are released before the program aborts ---
cleanup itself must never double-free or leak.

### Deferred design questions

- **Warn when a `may fail` error is never consulted.** As with the
  aggregate-copy warning, a `may fail` operation whose error is never
  read before the function returns could be flagged.
