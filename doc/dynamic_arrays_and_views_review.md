# Dynamic Arrays and Views — Consolidated Review (V2)

**Subject:** `doc/dynamic_arrays_and_views.md` (V2 — the revision carrying the documentation-only
corrections).

**Role:** This document is a **review**, not part of the proposal. It combines three independent
external reviews into one deduplicated assessment, cross-checked against the proposal text and the
compiler.

## 1. Provenance

Three external reviews are folded together below. They agree on the core direction; they differ in
emphasis, and two overlap on several items.

| Review         | Distinctive contributions                                                                                                                                                                                                                                                                                     |
| -------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| **A** (first)  | FFI ownership and pointer-lifetime contract; nested-array lvalue semantics; global static-storage ownership; runtime initializer-count mismatch; `unsafe decl` cleanup; owning/reference operation matrix; `inline` layout claim; stack-lowering escape; C-ABI baseline; the resize error model (`may fail`). |
| **B** (second) | `ref of T[]` variable semantics; `ref`-modifier vs. `ref`-type collision; `export C fn` parameter spelling; origin-dependent rebinding; moved-from enforcement; Annex A grow-on-materialised-view; error-stack hygiene; empty-array FFI marshalling; array equality.                                          |
| **C** (third)  | Sign-off review; elision-vs-abort policy; global buffer placement; FFI `const` exception (it grew into the general `readonly` capability); the resize mechanism for references; Part II deferral cues.                                                                                                        |

Where a claim was checked against the document or the compiler and found imprecise, it is corrected
in §6.

## 2. Consensus: what is working

- **One runtime-sized owning array type.** `i32[5]` is an initial count, `i32[]` the same type; no
  separate fixed-size type; no generics needed for `sum(a)`.
- **Deep-copy value semantics**, stated plainly and made the default.
- **The normative ownership table (§3.2)** — allocation source, freeing responsibility, moved-from,
  reassignment order — is the single strongest addition across all three reviews.
- **The Part I / Part II split.** Owned arrays plus a narrow whole-range reference in 1.0.0; ranged
  views deferred. This keeps 1.0.0 independent of range invalidation, global view tracking, and
  liveness all at once.
- **FFI ownership (`ref` / `copy` / `readonly ref`) and the two export forms** (`export fn` = Lale ABI,
  `export C fn` = C ABI), with the mangled/plain symbol-name tie-in.
- **"A foreign buffer is never freed by Lale"** with the FFTW example — a requirement, not a
  courtesy, for libraries with custom allocators.
- **Complete initialisation plus an explicitly `unsafe` escape hatch.**
- **The Annex B failure model**: abort by default, `may fail` for opt-in tolerance, no-op semantics
  so recovery is always from a consistent state, and an inspectable error stack.
- **§7.3's provisional flag** on the stack-lowering escape assumption — honest classification of a
  design risk.

## 3. Items already resolved (closed) or clarified

- **Capacity observability.** `capacity of` is removed; capacity is unobservable, yet kept in the
  ABI header so independently compiled modules agree on layout (§2.3, §13.4).
- **Moved-from.** The rule (§3.2) is present; only its enforcement remains open (§4.1.6).
- **The elision non-guarantee.** (§3.2) documents that copy elision is permitted but not
  behaviour-preserving under an observable OOM abort.
- **`view of text` / `view of binary`.** Indexing is by code point / byte; ranges are views — settled.
- **Annex B O1/O2/O3.** Dissolved; foreign-buffer release is now a requirement.
- **Documentation-only corrections applied:** §8.3 header-stability caveat; narrowed §4.1 `inline`
  layout claim; §11 regrouped; §2.6 static-buffer note; Part II deferral cues on the forward
  references in §2.5, §3.1, §3.2.
- **Global static-storage ownership — clarified, not exhaustive.** The §3.2 bullet specifies the
  transition on growth and that cleanup frees heap buffers but never static storage. It does not
  enumerate every case (for example reassignment of a static-backed global, or which storage is
  freed and by whom at program exit). Treat this as a design-level clarification, not a fully
  enumerated specification.

## 4. The approval gate — consolidated open decisions

These are the items recorded in the proposal's §11, regrouped here by priority. Numbers here are
local to this review.

### 4.1 References and the FFI boundary — resolve before 1.0.0 is approved

**4.1.1 Reference variables.** `ref of T[]` (§3.4) is a 1.0.0 value that may be **held in a
variable**, so its variable semantics must be stated: binding, assigning between two references,
passing one to an owning by-value parameter, and returning one. The hazard: a reference to a local,
or to an `inline` field, held in a variable that outlives its origin **dangles**. _Provenance: B;
overlaps A's operation-matrix item._ **Options:** (a) a full §3.2-style normative table for
references; (b) every reference bounded by its origin's valid lifetime for 1.0.0 (never stored, returned,
or passed to an owning parameter); (c) allow reference variables, with per-function restrictions —
§8.5's escape/freeze rules plus two syntactic checks (no returning a reference to a local, no global
storage), which suffices because Lale has only two scopes. **Chosen: (c)** — the design adopted it
(§11); this review had recommended (b) as the smaller 1.0.0 scope.

**4.1.2 Rebinding through a `ref` array parameter.** §4.3 rejects whole-array reassignment for an
`inline`-field reference, but says nothing for a `ref` parameter over an owning array, so the legality
of `xs = other` inside a callee currently depends on the call site's argument origin — which
separate compilation cannot see. _Provenance: B._ **Recommendation:** a uniform rule — an array
`ref` grants element mutation and (for owning origins) growth, but **never whole-variable
rebinding**.

**4.1.3 `ref` as a modifier vs. `ref of T` as a type — resolved.** The design now spells the type
`ref of T` (§3.4) and keeps `ref` as the parameter modifier (`fn f(ref xs as i32[])`), so the two are
distinguishable. _Provenance: B._ **Remaining:** confirm the grammar admits both without ambiguity.

**4.1.4 `export C fn` array-parameter spelling.** §5.1 and §5.3 say the ownership markings apply to
`export C fn` parameters, but the C-ABI rule bans a `T[]` parameter and the example uses a bare
`pointer` + `u64` — so an annotated `export C fn` array parameter has **no grammatical spelling**.
_Provenance: B._ **Recommendation:** define the descriptor form (reference + count binding) for
exports as it exists for imports, rather than dropping the claim.

**4.1.5 Runtime initializer-count mismatch.** §2.8 makes a _statically known_ mismatch between the
amount grown and the initialiser count a compile error, but `grow xs by n = f()` with a runtime `n`
and a runtime-sized initialiser is unspecified. _Provenance: A; re-flagged by B._ **Recommendation:**
an Annex B **failure** (abort by default, no-op + error under `may fail`).

**4.1.6 Moved-from detection.** §3.2's moved-from rule has no enforcement mechanism: reading a
moved-from variable is a compile error, a runtime abort, or undefined behaviour — unstated.
Conditional moves are likewise unaddressed. _Provenance: B, refining A._ **Recommendation:** a
definite-assignment-style linear check, small because `return` is the only move operation.

**4.1.7 `inline` element types.** Which element types are C-compatible for `inline T[n]` is unstated.
_Provenance: A._ **Recommendation:** name them in the ABI text (§13.3, §13.5).

**4.1.8 `may fail` error attribution and clearing.** The error stack is global and consuming
(`ARCHITECTURE.md §5.18`), but how a tolerated `may fail` error is attributed to its statement, and
whether it must be consumed before the function returns, is unspecified — a callee's unread
tolerated failure is otherwise visible to a caller. _Provenance: A; re-flagged with specifics by
B._

**4.1.9 Empty-array FFI marshalling.** §2.3 lets a zero-count array carry a null data pointer, and
§13.5's baseline passes that pointer to C — so an empty array marshals as `(NULL, 0)`, formally
undefined behaviour for `memcpy`-style APIs. _Provenance: B._ **Recommendation:** specify whether
the marshaller substitutes a non-null dummy or empties are excluded from the baseline shape.

**4.1.10 `copy` on a transferred FFI return (O1).** A C function that allocates and hands over
ownership loses its pointer once `returns copy i32[n]` deep-copies — so nothing releases the
original. _Provenance: A; re-flagged by B._ Both reviews reject "accept the leak". **Recommendation:**
restrict `copy` to the parameter direction in 1.0.0; revisit with 2.0.0's zero-copy adoption.

**4.1.11 Array equality.** Value semantics are stated, but `==` is undefined. _Provenance: B._
**Recommendation:** element-wise (deep) equality is the only value-consistent definition; record the
decision either way, since it is `O(n)` behind an innocent-looking operator.

**4.1.12 `unsafe decl` and destruction of uninitialised elements.** Element initialisation is the
programmer's obligation (§2.2), which works for values the program _reads_. Destruction is
different: the compiler frees a live array at scope exit, so for a resource-owning element type
(nested array, `text`, `binary`) recursive destruction could follow a garbage pointer.
_Provenance: A._ **Recommendation:** restrict `unsafe decl` to cleanup-free element types in 1.0.0,
or require every element to be valid before destruction.

**4.1.13 Resizing a fixed-capacity reference — the mechanism.** §4.3 says resizing is "unavailable" for
a reference (its `capacity` equals its `count`), but not whether the violation is a compile
error (plausible for `inline T[n]`, where `count` is the constant `n`) or a runtime failure (for an
FFI `ref` result with a runtime count). _Provenance: C._

**4.1.14 Elision vs. OOM-abort determinism.** §3.2 documents that elision is permitted but not
behaviour-preserving under an observable OOM abort. Review C argues this violates semantic
preservation and proposes restricting elision to non-failing allocations (or gating it on
`may fail`). _Provenance: C; the underlying tension was first raised by A._ This is a **policy
choice**, not an oversight. _Caveat:_ "allocation cannot fail (small arrays)" is not a decidable
predicate — see §6. **Options:** keep the documented non-determinism, or make elision
abort-preserving (restrict it / make OOM recoverable). Recorded in §11's cross-cutting group.

**4.1.15 Nested-array mutation target semantics.** §2.8 states the behaviour for growth ("growing a
nested array … leaves the outer array untouched"), but not the general rule: an indexed expression
used as an **assignment or mutation target** must designate the **stored** element, not a temporary
copy produced by an ordinary array-valued read. Without that rule, `grow m[1] by 2 = [7, 8]` and
`m[1] = other_row` could plausibly act on a copy. _Provenance: A._

**4.1.16 Owned / reference operation matrix.** The owning rules (§3.2) and the reference rules are not
interchangeable, and §3.2's table is owning-only. The reference side still needs normative decisions
for element mutation, growth and shrinkage, assignment between two reference values, whole-variable
rebinding, passing (to owning vs. non-owning parameters), returning, and destruction. §4.1.1 covers
binding, assignment, passing, and returning; §4.1.2 covers rebinding; the whole matrix should be
stated in one place. _Provenance: A._

**4.1.17 A declared `readonly` capability vs. "infer, don't declare".** `readonly` (§3.4) _declares_
that a holder may not write through a reference, whereas mutability is normally **inferred from SSA,
not declared** (`ARCHITECTURE.md §5.2`). The proposal justifies the declaration: `ARCHITECTURE.md §5.2` concerns a
**variable's** mutability, which SSA can see; a **reference type's capability** is a property of the
handle, with no single body to infer it from. _Provenance: this review, from the `readonly` design._
**Recommendation:** accept it explicitly, as a documented expansion of `ARCHITECTURE.md §5.2` — not a silent
exception.

### 4.2 The view layer (deferred — later version)

**4.2.1 Cross-call references.** A `ref` parameter may `grow` / `shrink` its array, and the per-function
freeze rule (§8.5) cannot see that: a caller's view can be dangled by a callee that shrinks the
array. _Provenance: A, B._

**4.2.2 Growth survival for nested-element views.** §8.3's "a view survives growth" holds only while
the origin's header keeps a stable address — true for a top-level variable, false for an
array-valued element inside a reallocating outer buffer. _Provenance: A, B._

**4.2.3 Unifying the two non-owning spellings.** Whether the whole-range `ref of T[]` (1.0) and the
ranged `view of T` (later) should become one type. _Provenance: A, B._

**4.2.4 Views of `text` / `binary`, and global origins.** Reassignment and storage invalidation of
`text` / `binary` views lack rules comparable to arrays; a view of a global can escape, so
view-taking on a global is cross-function. _Provenance: A, B._

**4.2.5 Annex A: the materialised by-value view parameter.** Neither Option P nor Option R states
whether a by-value view parameter (materialised to an owned array) may then `grow` / `shrink` — the
deciding evidence between P and R. _Provenance: B._

### 4.3 Cross-cutting

**4.3.1 `may fail` on other statements.** Whether the annotation stays specific to `grow` / `shrink`
or extends to other fallible statements. _Provenance: A; already in §11._

**4.3.2 The stack-lowering escape assumption (§7.3).** Validating that no escape route defeats "no
escape analysis"; a failure is a design risk, not merely an implementation task. _Provenance: A;
already in §11, with C concurring._

**4.3.3 IR spelling.** `i32[]` vs. `dyn<i32>` vs. a named fat-pointer struct. _Provenance: A;
already in §11._

**4.3.4 Compiler error-message vocabulary drift.** The operator is `count of`, the compile-time
analyzer says `size`, and the runtime message says `length`. _Provenance: B (minor), corrected and
confirmed against the compiler — see §6._ **Recommendation:** a compiler-wording follow-up, not a
proposal edit.

**4.3.5 Generalising `ref of T` beyond arrays, `text`, and `binary`.** In 1.0.0 a reference exists
only for the length-carrying types. Allowing `ref of T` for all types later (a scalar, a struct)
would be **additive, not breaking** — but it would turn `ref` into a general typed reference, undoing
`ARCHITECTURE.md §5.14`'s "Raw Pointers Only". _Provenance: this review._ **Recommendation:** keep it
open; treat it as its own design.

## 5. Documentation-level findings

- **Applied to V2:** the static-storage ownership bullet (§3.2) and its §7.3 cross-reference; the
  §8.3 header-stability caveat; the narrowed §4.1 `inline` claim; the §11 regroup; the §2.6
  static-buffer note; Part II deferral cues in §2.5, §3.1, §3.2.
- **Applied to V2 — the reference package:** a new **§3.4** defines the reference type `ref of T`,
  the value form `ref to x`, and the `readonly` capability modifier; the reference-array spelling
  became `ref of T[]` throughout; `const ref` became `readonly ref` (§5.1, §5.3, §9); `view of X`
  became source-typed (`view of f64[]`, §8.2); and §11 records the chosen option (c). The package also
  adds two terminology notes to §13: **§13.7**, a `lale.md` note that `readonly` names a withheld
  _permission_ (not the `const` immutability guarantee Lale cannot make), and **§13.9**, an
  `ARCHITECTURE.md` note on why the concept is a **reference**, not a _borrow_.
- **Now recorded in §11:** §4.1.13–§4.1.17, §4.2.4, §4.2.5, §4.3.4, and §4.3.5 — so the proposal's
  §11 and this review's §4 cover the same set.

## 6. Corrections to reviewer claims

- **"§2.4 says `length=5`, not `count`" (B, minor).** A **false positive against the proposal**: the
  text faithfully reproduces the compiler. `src/ir/builder.rs` (`bounds_check`) renders `", length="`,
  and `tests/runtime_error_format_tests.rs` asserts `array index out of bounds (index=5, length=3)`.
  The real, latent issue is a **three-way vocabulary drift** (operator `count of`; compile-time
  `size`; runtime `length`) — a compiler-wording decision, not a proposal edit.
- **"Read-only data segment" for globals (C).** Inaccurate: §2.6 requires only that a global
  _initialise_ to a constant; §2.8 lets it `grow` and §3.2 governs reassignment. A mutated global
  needs **writable** static storage and transitions to heap on growth. `.rodata` fits only a
  provably never-mutated global.
- **"Allow elision only when allocation cannot fail (small arrays)" (C).** Not a sound rule: whether
  an allocation can fail depends on the runtime, not the size. The honest choices are the two policy
  poles in §4.1.14.
- **"Add an `ARCHITECTURE.md §5.2` exception for FFI `const`" (C).** The design went further than the
  reviewer asked: rather than an FFI-only exception, `readonly` is now a general capability modifier
  on references and views (§3.4). §13.1 records the resulting `ARCHITECTURE.md §5.2` carve-out as "Declared
  capabilities", and the tension it creates is flagged in §11 (4.1.17).

## 7. Where the reviewers differ

- **Reference scope.** B preferred restricting references to rvalues and FFI bindings; A's
  operation-matrix item implied specifying reference variables fully. The design chose the wider
  option (c) (§4.1.1).
- **Option P vs. Option R.** B supplies the deciding evidence (the missing grow-on-materialised-view
  rule) and leans toward R plus an explicit materialisation spelling; A is silent. Deferred with the
  view layer (§4.2.5).
- **Elision policy.** C argues for semantic preservation; the document currently documents the
  non-guarantee. A design choice, recorded as §4.1.14.

## 8. Comparison: how Swift resolves the same questions

Swift is the closest widely shipped language to this proposal: one owning dynamic array type
(`Array<T>`, no size in the type) plus separate non-owning types. This section checks each open
question against Swift's answer. It is grounded in the evolution proposals **SE-0426**
(`BitwiseCopyable`), **SE-0377** (`borrowing` / `consuming`), **SE-0447** (`Span`), and **SE-0453**
(`InlineArray`); general `Array` / copy-on-write behaviour is from the language reference.

**The framing.** Swift's _owning_ array (`Array<T>`) is one dynamic type with no size in the type,
so the parallel is fair — but the two languages rest on different foundations, and that difference
decides how much of this section transfers.

- **Copy-on-write (COW).** A Swift array value is a small handle to a shared element buffer.
  Copying the _handle_ is cheap and shares the buffer; the elements are duplicated only when someone
  mutates a shared array. So `let b = a` copies nothing, and `b[0] = 1` makes `b` a private copy.
- **Automatic reference counting (ARC).** The runtime counts the handles pointing at each buffer and
  frees it automatically when the last one disappears — no garbage collector, no manual `free`.
- **Exclusivity.** Swift forbids two conflicting accesses to the same variable at once — for
  example, a mutation overlapping a live reference.

Together these mean a non-owning view can be shared safely: the buffer stays alive while any view
refers to it, and a later mutation of the origin copies instead of overwriting what the view sees.
Lale deliberately chose the opposite foundation: **deep-copy value semantics** (assignment and
parameter passing copy the elements eagerly, producing independent buffers, so `a = b` is `O(n)`)
and **no reference counting at run time** (a compiler-managed buffer is freed deterministically when
its owning scope ends, like a stack variable).

The consequence: Swift answers most of the questions below _in the runtime_, so they barely arise;
Lale must answer them _in the language_, because nothing at run time keeps a buffer alive for a
reference or copies on write. That is why Lale has many open decisions here and Swift has few.

(Swift also ships other array-like types — `ArraySlice`, `Substring`, `ContiguousArray`,
`InlineArray`, `Span` — so it is not literally a single array type; its _owning_ one is.)

**Swift is two layers.** Swift's array story splits in two, and only one half speaks to Lale:

| Swift layer                                 | Types and features                                                                                                                                                                                        | Where the answers live | Value to Lale                           |
| ------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ---------------------- | --------------------------------------- |
| **Safe value layer** (COW + ARC)            | `Array`, `String`, `Dictionary`, `ArraySlice`, `Substring`, `ContiguousArray`                                                                                                                             | in the runtime         | little — the questions barely arise     |
| **Explicit layer** (no COW/ARC convenience) | `Span` / `RawSpan` (`~Escapable`), `InlineArray`, `~Copyable` types, `borrowing` / `consuming`, `consume x` / `copy x`, `UnsafePointer`, `withUnsafeBufferPointer`, `Array(unsafeUninitializedCapacity:)` | in the language        | high — this is the layer Lale resembles |

So a point gains real help when Swift's answer comes from the **explicit layer** (or from a scope or
judgment decision, such as deferring ranged views); it gains little when the answer is "the runtime
handles it" — which is the case for the whole view-invalidation family (4.2.1, 4.2.2, and in part
4.2.4). That split is why the points that _do_ get help cluster where Swift steps outside its own
COW/ARC comfort zone.

### 8.1 References and the FFI boundary

| Open point                                 | How Swift solves it                                                                                                                                                                                                                                                                                                                                                                                                  | For Lale                                                                                                                                                                                                                                                         |
| ------------------------------------------ | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| 4.1.1 Reference variables; dangling        | (a) ARC-shared `ArraySlice` / `Substring` — storable and returnable, no dangle (the buffer is retained). (b) `Span<T>` is `~Escapable` (SE-0447), and in the shipping prototype spans are "constrained to closures from which they structurally cannot escape". `Array.span` awaits lifetime annotations.                                                                                                            | (a) needs ARC, unavailable. `Span` is `~Escapable`; it may be **stored** within its enclosing scope, it just cannot escape it — closer to Lale's chosen option **(c)** than to (b).                                                                              |
| 4.1.2 Rebinding through a `ref` param      | Parameter ownership modifiers (SE-0377): `borrowing` (shared, read-only), `consuming` (transfer), `inout` (exclusive; may replace). No "mutate elements but not rebind" middle ground; aliasing is policed by **exclusivity**.                                                                                                                                                                                       | No analogue for Lale's no-rebind middleware; it is _stricter_ than `inout`. Swift maps `ref` to `inout`, and the deferred read-only mode to `borrowing`.                                                                                                         |
| 4.1.3 `ref` modifier vs. type              | Ownership is both, kept distinct: the _type_ says who owns (`Array` owns; `ArraySlice` / `Span` / `UnsafeBufferPointer` are non-owning); the _modifier_ says how it is passed (`borrowing` / `consuming` / `inout`). There is no `ref of T[]`-style type spelling.                                                                                                                                                   | Swift matches 4.1.3: one spelling per role — modifier for parameters, a distinct type for stored non-owning values.                                                                                                                                              |
| 4.1.4 `export C fn` array-param spelling   | `Array` is not C-representable; C `T[N]` imports as an N-tuple (SE-0453 calls this painful). C calls use `UnsafePointer` / `UnsafeMutablePointer` + count; ownership lives in the pointer type and docs, with **no ownership-marking grammar**.                                                                                                                                                                      | Supports the pointer+count baseline, and suggests resolving 4.1.4 by **dropping the claim that markings apply to `export C fn` params** rather than inventing a spelling.                                                                                        |
| 4.1.5 Runtime initializer-count mismatch   | No such operation: `append(contentsOf:)` and `replaceSubrange` derive the count from the source; `reserveCapacity` allocates without creating elements. The amount and the data are never separated.                                                                                                                                                                                                                 | Swift's answer is "don't separate them" — a simpler surface. No precedent for the failure rule; it stays Lale-specific.                                                                                                                                          |
| 4.1.6 Moved-from detection                 | `Array` is COW, so no move; moves exist for `~Copyable` types (SE-0390), and after `consume x` / a `consuming` param, **use-after-consume is a compile error** (SE-0366, SE-0377).                                                                                                                                                                                                                                   | Swift backs the _shape_ of 4.1.6 (a linear compile error) but not its cost: conditional moves, control-flow joins, and initialization state still need Lale-specific rules.                                                                                      |
| 4.1.7 `inline` element types               | `InlineArray<let count, Element>` (SE-0453): layout = count × stride(Element), alignment of Element, never implicitly heap-allocated — Lale's `inline T[n]` exactly. The C-compat predicate is `BitwiseCopyable` (SE-0426): `memcpy`-copyable, no destroy.                                                                                                                                                           | Good model for the **trivial-destroy** half, but `BitwiseCopyable` is _not_ C-ABI compatibility: layout, size, alignment, stride, representation, and the enclosing struct need their own definition. The trivial-destroy half also settles 4.1.12 for `inline`. |
| 4.1.8 `may fail` attribution / clearing    | No error stack. Failure is control flow: `throws` / typed throws (SE-0413), `try?`, `try!`. Attribution is lexical; there is nothing to clear.                                                                                                                                                                                                                                                                       | No precedent. Only the policy maps: default-abort ≈ `try!`, `may fail` ≈ `try?`. The fork is errors-as-values (Swift) vs a diagnostic stack (Lale).                                                                                                              |
| 4.1.9 Empty-array FFI marshalling          | Empty `Array` through `withUnsafeBufferPointer` gives `count == 0`; `baseAddress` nil-ness is an implementation detail, **not guaranteed non-null**; well-written wrappers guard on count.                                                                                                                                                                                                                           | Swift-consistent choice: document that `(nil, 0)` may be passed; Lale does not promise a non-null dummy. (Verify against the current stdlib before quoting.)                                                                                                     |
| 4.1.10 `copy` on a transferred return (O1) | Importing `int *make_ints(size_t)` gives `UnsafeMutablePointer<Int32>?`; to own it you write `Array(UnsafeBufferPointer(start:count:))` — a copy — and **you must free the original yourself**. Zero-copy adoption with a custom deallocator exists (`Data(bytesNoCopy:count:deallocator: .custom)`).                                                                                                                | Strong support for both halves: copying never frees the original (matches the never-free rule); adoption is explicit, with a caller-supplied deallocator (Lale's deferred 2.0.0 mode). Swift avoids O1 by not hiding the pointer.                                |
| 4.1.11 Array equality                      | `Array: Equatable where Element: Equatable`; `==` is element-wise (deep), O(n); `Hashable` likewise.                                                                                                                                                                                                                                                                                                                 | Swift matches the recommended resolution: element-wise `==`, documenting O(n).                                                                                                                                                                                   |
| 4.1.12 `unsafe` declaration destruction    | Uninitialized memory must be `initialize`d then `deinitialize(count:)`; destroying uninitialized non-trivial memory is UB. `Array(unsafeUninitializedCapacity:)` forces reporting the initialized count (SE-0447 calls it error-prone). SE-0453 **refuses to ship `FixedCapacityArray`** (initialized prefix + uninitialized suffix) _because_ the compiler cannot track initialization and destroying it may crash. | Strong support for 4.1.12: track the initialized count and destroy only that many, or require trivially-destructible (`BitwiseCopyable`) element types; otherwise, do not provide the type.                                                                      |
| 4.1.13 Resizing a fixed-capacity reference | Reference types have no mutating-size API: `Span` has no append, `UnsafeBufferPointer` cannot resize, `InlineArray` has no `append` / `remove`. It is a compile error by construction.                                                                                                                                                                                                                               | Swift = compile error, via absence of the operation. Supports 4.1.13.                                                                                                                                                                                            |
| 4.1.14 Elision vs. OOM-abort determinism   | Allocation failure traps (unrecoverable); there is no promise that a given allocation occurs, so the OOM-trap point is not optimisation-invariant — effectively Lale's documented non-determinism. Copy control is explicit (`copy x`, `consume x`), not imposed on the optimizer.                                                                                                                                   | Swift supports Lale's current stance (document the non-guarantee) against C's proposed restriction. For determinism, the Swift route is explicit programmer-controlled copies.                                                                                   |
| 4.1.15 Nested-array mutation target        | Subscripts have separate read and mutating accessors, so an indexed expression used as a mutation target designates the stored element in place, while a plain read yields a value.                                                                                                                                                                                                                                  | Supports making the general lvalue rule normative: a read is a value copy, a mutation target is the stored element.                                                                                                                                              |
| 4.1.16 Owned / reference operation matrix  | Owning (`Array`) and non-owning (`Span`, `ArraySlice`, `UnsafeBufferPointer`) types expose different APIs; the non-owning types have no mutating-size or ownership operations at all.                                                                                                                                                                                                                                | Supports making the reference column normative, enforced by the type system.                                                                                                                                                                                     |

### 8.2 The view layer

| Open point                                      | How Swift solves it                                                                                                                                                                                                                                                                       | For Lale                                                                                                                                                                                                                   |
| ----------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| 4.2.1 Cross-call references                     | `ArraySlice` shares storage via ARC, so a callee's reallocation COWs and the caller's slice keeps the old storage (no dangle, a snapshot); **exclusivity** forbids overlapping `inout` access and a live reference; `Span`'s `~Escapable` stops a callee's reference outliving its scope. | No direct transfer (it needs ARC / COW / exclusivity). Closest reusable idea: **exclusivity** — a live reference cannot coexist with a mutating (`ref`) reference.                                                         |
| 4.2.2 Growth survival for nested-element views  | `ArraySlice` does not follow growth; it retains the old buffer (COW snapshot). A slice of an inner array shares the inner buffer via ARC.                                                                                                                                                 | No analogue — Swift chose the opposite model (snapshot + retain). Lale's "follow the header" is the more complex design; its nested-header problem vanishes under snapshot semantics (but COW conflicts with deep copies). |
| 4.2.3 Unifying the two non-owning spellings     | Swift keeps them separate: `ArraySlice`, `Substring`, `UnsafeBufferPointer`, `Span` / `RawSpan`. `Substring` vs `String` is the classic second-type precedent; SE-0447's appendix debates "Span is its own slice type".                                                                   | Supports keeping `ref of T[]` and `view of T` distinct.                                                                                                                                                                    |
| 4.2.4 Views of `text` / `binary`, globals       | `Substring` and `String.UTF8View` share storage; `String` is COW. `String.span` and `Array.bytes` are **deferred** pending lifetime annotations (SE-0447 future directions).                                                                                                              | Swift needed lifetime annotations before exposing text spans; Lale must define its own invalidation rules. A caution, not a solution.                                                                                      |
| 4.2.5 Annex A: materialised by-value view param | Views are distinct types; a by-value `Span` passes the reference (no materialisation); to own, you write `Array(slice)` — an explicit O(n) copy. No implicit view-to-owner conversion.                                                                                                    | Swift points to **Option R with an explicit materialisation spelling**. (In Swift the by-value param stays a reference; the materialisation question exists only because Lale's value semantics force a copy.)             |

### 8.3 Cross-cutting

| Open point                             | How Swift solves it                                                                                                                                                        | For Lale                                                                                                                                             |
| -------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------- |
| 4.3.1 `may fail` on other statements   | No statement-level fallibility; it lives in the signature (`throws`) and at the call site (`try?` / `try!`).                                                               | No precedent; supports keeping `may fail` narrow.                                                                                                    |
| 4.3.2 Stack-lowering escape assumption | Swift does stack-promote array / object allocations and relies on **escape analysis** to decide (arrays are individually heap-allocated, promoted only when non-escaping). | Contradicts Lale's "no escape analysis needed"; strengthens the §7.3 design-risk flag. (From knowledge of the optimizer, not the fetched proposals.) |
| 4.3.3 IR spelling                      | Nominal generic types (`Array<T>`, `InlineArray<N,T>`, `Span<T>`), backing type `_ContiguousArrayStorage<T>`; no `dyn<...>` syntax.                                        | Supports a named type over `dyn<i32>`.                                                                                                               |
| 4.3.4 Error-message vocabulary drift   | Consistent terms: "index out of range"; `count` for element count, `capacity` for capacity. Swift **exposes** `capacity`.                                                  | Pick one term (`count`) in the API and in diagnostics.                                                                                               |

### 8.4 What this tells us

- **Swift supports eight of the recommendations:** scope-restricted references (4.1.1b) — `Span` is
  closure-scoped by construction; a compile error for resizing a fixed-capacity reference (4.1.13) — via absence
  of the API; `BitwiseCopyable` as the model for the trivially-destructible half of the `inline`
  element set (4.1.7) — though C-ABI layout compatibility needs its own definition; track the
  initialized count, or require trivially-destructible elements, for `unsafe` declarations (4.1.12)
  — SE-0453 refuses to ship the unsafe variant for this reason; element-wise `==` (4.1.11); an
  explicit materialisation spelling for view-to-owner (4.2.5, i.e. Option R); a named IR type
  (4.3.3); a linear compile error for moved-from (4.1.6); and not hiding the original FFI pointer,
  with an explicit deallocator for adoption (4.1.10).
- **Swift contradicts or does not support four:** that escape analysis is unnecessary (the §7.3
  premise); a no-rebind middleware for `ref` (Swift uses the `inout` and `borrowing` poles plus
  exclusivity); a diagnostic error stack (Swift uses errors-as-values); and "a view survives growth"
  (Swift is snapshot + retain).
- **Observation.** Most of Lale's open points are _costs of the deep-copy, no-reference-counting
  value model_: Swift's copy-on-write, reference counting, and exclusivity design away the aliasing,
  lifetime, and dangling questions Lale must specify by hand. That does not argue against Lale's
  deliberate choice, but it explains why so many decisions are needed here. Supporting evidence:
  Swift shipped its first safe non-owning type (`Span`) only in **6.2 (2025)**, and even then deferred
  `Array.span` and sub-spans to future lifetime annotations — an argument for Lale deferring ranged
  views too.

## 9. Recommendation

The core design is **sound and approvable in direction**: one runtime-sized owning array type,
deep-copy defaults, an explicit and normative ownership table, a clean Part I / Part II split, and an
honest FFI boundary. All three reviewers endorse this shape; none disputes the architecture.

Approval of the **1.0.0 specification**, however, should be **conditional** on resolving the
reference-array and FFI-boundary decisions — in particular §4.1.1 (reference variables; now decided
as option (c)), §4.1.2
(uniform no-rebinding), §4.1.4 (`export C fn` spelling), §4.1.5 (runtime count mismatch), §4.1.6
(moved-from detection), §4.1.10 (O1), §4.1.15 (nested-array mutation), and §4.1.16
(owned/reference operation matrix). These are semantic choices the implementation cannot make
for itself without becoming a _de facto_ specification.

The deferred view layer (§4.2) can be approved in outline only; its lifetime questions belong with
the later proposal. The cross-cutting items (§4.3) do not gate 1.0.0 except the stack-lowering escape
assumption, whose outcome may force a language-level change and therefore deserves to be resolved
before the optimisation is relied upon.
