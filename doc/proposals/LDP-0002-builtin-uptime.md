---
number: LDP-0002
title: "`__lale_read_uptime()` Builtin Function"
status: implemented
scope: language
milestone: 1.0.0
charter: [] # charter not yet adopted; see LDP-0001 O1
author: Kağan Kayal
decided-by: Kağan Kayal
created: 2026-10-25
decided: null # decision predates the record system; see Status notes
implemented: null # implementation predates the record system; see Status notes
supersedes: null
superseded-by: null
docs-touched:
  - doc/ABI_SPECIFICATION.md
  - doc/lale.md
  - doc/ARCHITECTURE.md
  - AGENTS.md
  - src/builtins/builtins.lale
  - stdlib/src/core.lale
implementation:
  - target: linux
    state: yes
  - target: macos
    state: yes
    reason: mach_continuous_time + mach_timebase_info, exact-split conversion
  - target: windows
    state: yes
  - target: wasi-browser # WebAssembly System Interface (browser demo)
    state: partial
    reason: CLOCK_MONOTONIC — excludes host suspend and counts from runtime start (demo/README.md)
  - target: bare-metal
    state: partial
    reason: full conformance only where the BSP provides an always-on counter spanning suspend (§4.3, §5)
impl-ref: src/builtins/builtins.lale, stdlib/src/core.lale, src/interpreter.rs, tests/uptime_tests.rs
---

# Proposal: `__lale_read_uptime()` Builtin Function

## 1. Context

Provide a simple, honest source of boot-relative elapsed time for Lale targets — embedded, bare metal (running directly on the hardware, with no operating system underneath), and hosted (running under an operating system) — without requiring a complex time type, clocks, zones, or calendar policies.

**Goal:** `__lale_read_uptime()` returns "nanoseconds since boot" as a plain `u64 in <ns>`, usable with Lale's existing unit system and arithmetic.

**Availability.** `__lale_read_uptime()` depends on the capabilities of the underlying platform. Where the platform provides an always-on counter — a timer that keeps counting while the device sleeps — the implementation **must** satisfy the full contract in Section 4. Where the platform cannot — for example, a bare-metal chip with no such counter, or a web runtime whose only monotonic clock (one that never moves backwards) excludes host suspend — the function **may** provide a reduced implementation: readings remain monotonic within a boot session (one power-on to shutdown), but may exclude suspended time (the time spent in a low-power sleep state) and may count from the moment the runtime's clock became operational rather than from the device's boot.

A reduced implementation is permitted, but it may not be silent. Each platform declares its conformance level in `doc/ABI_SPECIFICATION.md`, and this proposal records it per target in its front matter. Code that must know whether suspend time is counted depends on the declaration, not on an assumption the compiler cannot enforce. Reduced conformance is for platforms whose hardware or runtime cannot expose suspended time — not a convenience option for platforms that can.

---

## 2. Definition

```lale
import fn signature __lale_read_uptime() returns u64 in <ns>
```

`__lale_read_uptime` is declared in `builtins.lale` — like `__lale_malloc` — as an `import fn` hook. An `import fn` is a function the compiler knows only by its signature; its body is supplied outside the compiled program and joined to it at link time (when the executable is assembled from its own code plus the libraries it calls). The platform supplies that body — the OS kernel, the language runtime (the library that starts and supports a Lale program), a vendor software development kit (SDK), or a board support package (BSP: the vendor-supplied startup code and hardware drivers for one specific board).

The contract between the compiled program and the platform — symbol names, how arguments are passed, and how data is laid out — is the _application binary interface_ (ABI); `doc/ABI_SPECIFICATION.md` records it for each target.

On hosted targets the standard library (`core.lale`) supplies that implementation as an `export fn` (a function compiled under a fixed, externally visible name) that wraps the platform's monotonic boot-time clock — the same "`import fn` hook + `export fn` wrapper" pattern as `__lale_malloc` wrapping `malloc` (see §4.4 for the per-OS clock). In no-stdlib/embedded mode, the `import fn signature` above is the hook the platform must satisfy at link time.

**Impurity.** Each evaluation obtains a fresh platform reading. The compiler must not eliminate, cache, merge, or reorder calls in a way that changes observable results or violates ordering semantics (Section 4.5).

---

## 3. Return Value and Type

| Property         | Value                                                                                                                                                                                                 |
| ---------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| **Return type**  | `u64 in <ns>` — unsigned 64-bit integer with unit `<ns>`                                                                                                                                              |
| **Meaning**      | A non-decreasing, boot-relative reading of elapsed time, expressed in nanoseconds per the platform's monotonic boot-time clock; includes suspended time where the platform can measure it (Section 1) |
| **Range**        | 0 to 2⁶⁴ − 1 (≈ 584 years)                                                                                                                                                                            |
| **Sign**         | Unsigned — boot is the origin; no pre-boot readings                                                                                                                                                   |
| **Monotonicity** | Non-decreasing within a boot session                                                                                                                                                                  |
| **Wraparound**   | Must not wrap within a boot session; behavior beyond the representable range is out of scope                                                                                                          |

**Units.** The `<ns>` is part of the return type and is enforced by the type checker; no caller-side annotation is needed.

**Resolution and accuracy.** The value may be quantized to the platform's tick — the period of the hardware counter, which only changes every so many nanoseconds — so two consecutive calls may return the same value. That is normal, not an error. The counter is **non-decreasing**, not strictly increasing. The language guarantees neither nanosecond resolution nor a specific accuracy relative to physical SI time (the international standard second): clocks drift and undergo frequency adjustment, so the `<ns>` unit is a _scale_, not a promise of measurement accuracy.

---

## 4. Semantics

### 4.1 Monotonicity

`__lale_read_uptime()` is **monotonic non-decreasing within a boot session**: successive readings never decrease. They may repeat when the platform's resolution is coarser than 1 ns. This guarantee holds across threads and cores — a later sequential reading must not return a lower value merely because execution migrated to another core.

### 4.2 The boot reference point

On hosted systems the OS exposes a boot-relative clock. On bare metal, the timebase — the hardware counter that supplies the tick — may be initialized by the runtime or BSP after reset, not at the instant the device began booting.

The reference point is the **platform's defined boot epoch** (the moment the count starts from), including early startup where the platform can measure it. If a target cannot account for the interval before its timebase becomes operational, that limitation is a platform-contract matter, not a redefinition of the feature.

### 4.3 Who keeps it monotonic

A fast-wrapping hardware counter cannot be returned directly — a 32-bit counter running at 1 MHz (one million ticks per second) reaches its maximum after ~71 minutes and then rolls back to zero. The **platform** (OS, SDK, or BSP) — not the compiler — hides the wrap by extending the counter to 64 bits, using one of these standard techniques:

- a **64-bit hardware counter** (effectively no wrap), or
- an **overflow interrupt** at the moment of wrap that increments a **software high word** — a counter kept in ordinary memory that forms the upper 32 bits of the 64-bit result, or
- a **periodic interrupt** (a timer interrupt that fires at a fixed interval) that samples more often than the wrap period and increments the high word whenever it sees the counter move backwards.

Because the high word is maintained in the background, user code may call `__lale_read_uptime()` hours apart and still receive a correct monotonic value.

A conceptual platform-side illustration (not Lale, not compiler-generated):

```text
// Platform/SDK side: a 32-bit timer plus a software high word.
// The snapshot must be coherent — no wrap may occur between the two reads.
u64 read_uptime_ns(void) {
    disable_timer_irq();                       // single-core: block the wrap mid-read
    u64 combined = ((u64)high << 32) | read_timer_counter();
    enable_timer_irq();
    return ticks_to_ns(combined);              // scale ticks to nanoseconds
}
```

This is illustrative only. The normative requirement is that the implementation obtain a coherent counter snapshot (a reading whose high and low halves belong to the same instant — no wrap may occur between the two reads), account for every wrap, and remain correct across concurrency and suspend/wake transitions; full guidance (multi-core synchronization, suspend reconciliation) belongs in the platform/ABI documentation.

### 4.4 Suspend time

**Include all elapsed time**, including time spent in sleep/suspend states, **where the platform can measure it**. A reduced platform (Section 1) excludes suspend time; the exclusion is declared per platform, never inferred by the caller.

> Rationale: if a device sleeps for an hour, that hour happened; a data logger measuring intervals across suspend should see it.

Implementation note: each operating system names its clock sources; choose the one that includes suspend — `CLOCK_BOOTTIME` on Linux/Android (not `CLOCK_MONOTONIC`, which excludes suspend). On macOS use `mach_continuous_time` — macOS has no `CLOCK_BOOTTIME`, and `mach_continuous_time` is its suspend-inclusive monotonic clock. On Windows use `QueryInterruptTimePrecise` (100-nanosecond units since boot, including suspend); the `QueryUnbiasedInterruptTime*` variants exclude sleep and must not be used. On bare metal, use an always-on counter, or reconcile the high word on wake-up.

### 4.5 Observation semantics

Each call performs one platform read. Because `__lale_read_uptime()` is an `import fn` call, Lale treats it as effectful and must not eliminate, cache, merge, or reorder calls in a way that changes observable results.

### 4.6 Boot-to-boot independence and wraparound

Values from different boots are **incomparable** — the counter resets to 0 on each boot. Comparison across boot boundaries is meaningless, and detecting reboot is out of scope for 1.0.0 (a library concern).

The 64-bit nanosecond counter must **not wrap within a boot session**. The range 2⁶⁴ ns (≈ 584 years) exceeds any realistic boot; behavior beyond it is out of scope and no provision is made.

---

## 5. Implementation Boundary

| Component    | Responsibility                                                                                                                                                                                                                                                                                                                                                                                                                                    |
| ------------ | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| **Compiler** | Type-checks `__lale_read_uptime()` as `u64 in <ns>`; treats it as an effectful `import fn` (no folding, CSE, or reordering); emits a normal call                                                                                                                                                                                                                                                                                                  |
| **Platform** | Implements `__lale_read_uptime()` — reads and extends the hardware counter, ensures monotonicity and a coherent snapshot, and includes suspend time where the platform can measure it. On hosted systems the primitive is the platform's suspend-inclusive clock — `clock_gettime(CLOCK_BOOTTIME, &ts)` on Linux/Android, `QueryInterruptTimePrecise` on Windows, and `mach_continuous_time` on macOS (a Mach call, not a clock id — see _macOS_) |
| **ABI**      | Declares each platform's conformance level (full or reduced; see below); specifies the symbol, calling convention, resolution, and suspend reconciliation per target                                                                                                                                                                                                                                                                              |

In the Compiler row, _folding_ means replacing a call with a fixed value at compile time, and _CSE_ (common subexpression elimination) means computing one reading and reusing it for two identical calls; either would make two reads return the same value.

**Stdlib wrapper.** `core.lale` selects the implementation at compile time: `#if #posix` wraps `clock_gettime(CLOCK_BOOTTIME, &ts)`, and `#if #windows` wraps `QueryInterruptTimePrecise`. A failed `clock_gettime` is not a degraded reading — the wrapper reports it and exits (see _Failure handling_).

**Failure handling.** `clock_gettime` returns `0` on success and `-1` on failure. For a constant, always-valid clock id and a stack out-parameter, a failure means the platform is misconfigured rather than that a reading is temporarily unavailable. Returning a value anyway would produce a silent zero — indistinguishable from "the device just booted" and enough to corrupt every interval computed from it — so the wrapper writes one line to fd 2 and exits with status 1.

It writes with `write(2, …)` rather than `alert` deliberately: every log line carries the uptime column, which calls back into this function, so a log statement inside the failure path would re-enter it. Windows needs no equivalent branch — `QueryInterruptTimePrecise` returns `void` and has no error channel.

**macOS.** macOS has no `CLOCK_BOOTTIME`; its suspend-inclusive clock (`mach_continuous_time`) is a Mach call, not a `clock_gettime` clock id. The `#darwin` arm calls `mach_continuous_time` and `mach_timebase_info` directly and converts ticks with the exact split. Both are documented exceptions to the C-only boundary (`ABI_SPECIFICATION.md` §4, `ARCHITECTURE.md` §4.8), and the interpreter handles the same two calls, so interpreted and native behaviour agree.

**C ABI signedness.** On POSIX systems — the Unix-family interface standard implemented by Linux and macOS — the hosted wrapper uses `clock_gettime`, whose `struct timespec` has **signed** fields (`time_t tv_sec`, `long tv_nsec`). That signedness is an internal C-ABI detail — `time_t` must represent pre-epoch wall-clock times. For `CLOCK_BOOTTIME` the seconds and nanoseconds are always non-negative, so the wrapper's signed-to-unsigned conversion to the public `u64` is value-preserving. The public contract is the unsigned `u64` return (§3); the signed C type never crosses the Lale boundary. Windows has no such signed struct — `QueryInterruptTimePrecise` returns an unsigned value directly.

**No new type.** `__lale_read_uptime()` returns an ordinary `u64` with a unit annotation — no flags, no layout guarantee, no ABI stability beyond the function signature.

**Conforming targets.** Two levels are defined. Every platform declares which one it implements.

| Level       | Requirements                                                                                                                           |
| ----------- | -------------------------------------------------------------------------------------------------------------------------------------- |
| **Full**    | Non-decreasing within a boot session; includes suspended time; reference point is the platform's boot epoch                            |
| **Reduced** | Non-decreasing within a boot session; may exclude suspended time; reference point is the moment the runtime's clock became operational |

The level is declared per platform in the ABI specification and recorded in this proposal's front matter. The compiler cannot detect it, so it guarantees only that the call is effectful (§4.5); the monotonicity and suspend guarantees are platform-contract obligations, verified by platform tests (§10). A platform that can satisfy the full contract must do so; reduced conformance is for platforms whose hardware or runtime cannot expose suspended time.

---

## 6. Examples

### 6.1 Interval measurement (the canonical use)

```lale
var t0 as u64 in <ns> = __lale_read_uptime()
do_work()
var t1 as u64 in <ns> = __lale_read_uptime()
when t1 >= t0
    var dt as u64 in <ns> = t1 - t0  // elapsed per the platform's boot-time clock
end when
```

### 6.2 Deadline pattern

```lale
var started as u64 in <ns> = __lale_read_uptime()

loop
    when __lale_read_uptime() >= started + 1000000000 <ns>  // 1 s elapsed
        exit loop
    end when
    do_step()
end loop
```

### 6.3 Plain integer arithmetic and unit checking

```lale
// Valid: same units, guarded so the subtraction cannot underflow
var d1 as u64 in <ns> = __lale_read_uptime()
var d2 as u64 in <ns> = __lale_read_uptime()
when d2 >= d1
    var diff as u64 in <ns> = d2 - d1
end when

// Invalid: unit mismatch (caught by the type checker)
// var bad = __lale_read_uptime() + 5 <m>   // compile error
```

---

## 7. Relation to Other Time Concepts

`__lale_read_uptime()` measures elapsed time; it does not provide a wall-clock value.

- **Wall-clock time** (calendar time — what a person reads on a clock: UTC, calendars, zones, leap seconds) is a separate concern, provided by libraries.
- **`CLOCK_MONOTONIC`** is monotonic but excludes suspend; `__lale_read_uptime()` includes suspend on platforms that can measure it (matching `CLOCK_BOOTTIME`); a reduced platform behaves like `CLOCK_MONOTONIC` (Section 1).
- **`__lale_timestamp()`** — the pre-existing `builtins.lale` extern that returns the current UTC (coordinated universal time, the world time standard) wall-clock time as ISO-8601 text (the international date-and-time text standard; the common form is `YYYY-MM-DDThh:mm:ss`) — is a wall-clock concern, separate from `__lale_read_uptime()`. It moves to the standard library (a later, separately-scoped step).
- **`__lale_log_level()`** remains a builtin, because the `log`/`warn`/`alert` keywords lower directly to it as their runtime threshold hook.

Lale's logger uses `__lale_read_uptime()` for elapsed-time columns and obtains wall-clock timestamps from a library.

Everything else is a library, platform, or ABI concern.

---

## 8. Alternatives Considered

| Alternative                                                     | Rejected because                                                                                                                           |
| --------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------ |
| **Keyword `uptime`**                                            | Consumes a scarce keyword namespace; a plain function achieves the same effect                                                             |
| **New type (`Uptime`, `duration`, …)**                          | Unnecessary; `u64 in <ns>` already carries the unit                                                                                        |
| **Bare `uptime()` (no `__lale_` prefix)**                       | Inconsistent with the existing `__lale_*` builtin convention                                                                               |
| **Exclude suspend (`CLOCK_MONOTONIC`)**                         | Undercounts intervals across suspend                                                                                                       |
| **Return `f64 in <ns>`**                                        | Floating point has rounding error; `u64` covers ~584 years exactly                                                                         |
| **A full time model (instant/timestamp/ruler types)**           | Prior proposal iterations proved the dead end; that complexity belongs in libraries                                                        |
| **Runtime capability query (e.g. `uptime_includes_suspend()`)** | Adds public API surface for a property that is fixed per platform; the conformance level is declared in the ABI and in this record instead |

---

## 9. Consequences

- **No new type and no new keyword.** The feature reuses `u64 in <ns>`, so every existing arithmetic and unit rule applies unchanged and the grammar is untouched (§3, §5).
- **The conformity burden moves to the platform.** Each platform declares full or reduced conformance; a platform capable of the full contract must implement it, and a reduced level is a declared limitation rather than an unannounced substitution (§1, §5).
- **A failed clock read is loud.** The wrapper aborts with a message and status 1 rather than returning 0, which would be indistinguishable from a fresh boot and would corrupt every interval computed from it; the trade-off is that a misconfigured platform kills the program instead of degrading it (§5).
- **Wrap handling is not the compiler's job.** Extending a narrow hardware counter to 64 bits and taking a coherent snapshot belong to the platform; the compiler's only obligation is to treat the call as effectful (§4.3, §4.5).
- **Readings are only comparable within one boot.** Comparing values across boots is meaningless, and reboot detection is deliberately left to a library-level `boot_id()` (§4.6, O2).
- **Wall-clock work is pushed out.** `__lale_timestamp()` becomes a standard-library concern in a separately scoped step, and zones, calendars, leap tables, and serialization stay in versioned library components (§7).
- **Downstream consumers depend on it.** Lale's logger derives its elapsed-time columns from this call, and wall-clock timestamps come from a library instead (§7).
- **Part of acceptance is manual.** The no-folding, suspend-inclusion, and no-stdlib criteria cannot be exercised by unit tests on a development host and need manual verification before 1.0.0.

---

## 10. Acceptance Criteria for 1.0.0

The following are normative requirements, each verified by a test rather than merely asserted:

| Requirement                                                                                                                                                                                  | Verification      |
| -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ----------------- |
| Compiles without the standard library                                                                                                                                                        | build test        |
| Returns `u64 in <ns>` (no new type)                                                                                                                                                          | type-checker test |
| Monotonic non-decreasing within a boot session                                                                                                                                               | platform test     |
| Includes suspend time where the platform can measure it                                                                                                                                      | platform test     |
| Hosted stdlib wrapper uses the platform's suspend-inclusive clock on the platforms it implements (`CLOCK_BOOTTIME` Linux/Android, `QueryInterruptTimePrecise` Windows; macOS via Mach calls) | platform test     |
| Never constant-folded, CSE'd, or reordered                                                                                                                                                   | optimizer test    |
| No new grammar keywords                                                                                                                                                                      | grammar test      |
| Type checker enforces unit compatibility                                                                                                                                                     | unit test         |

Platforms that cannot provide the full contract implement the reduced level (§5); each platform declares its level in the ABI specification.

"Compiles without the standard library" means the `import fn` declaration lives in the compiler's embedded `builtins.lale`, so the `u64 in <ns>` signature is known even with the stdlib disabled. The _implementation_ is stdlib-supplied on hosted targets (or platform-supplied in embedded/no-stdlib builds); user Lale code does not call the raw hook directly in no-stdlib mode.

---

## 11. Open Points

| #   | Open point                                                                                                                                                                | State                                                                                                                                                                                   |
| --- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| O1  | Platform ABI for `__lale_read_uptime()` (calling convention, resolution, multi-core coherence, suspend reconciliation)                                                    | Resolved — recorded in `doc/ABI_SPECIFICATION.md` (per-target clock, calling convention, multi-core coherence, suspend reconciliation)                                                  |
| O2  | Boot-ID / reboot detection                                                                                                                                                | Resolved — `boot_id()` as a standard-library function (Linux `/proc/sys/kernel/random/boot_id`); not a core-language feature                                                            |
| O5  | macOS stdlib branch: `#posix` also matches macOS, which has no `CLOCK_BOOTTIME`; a correct branch needs `mach_continuous_time`, a Mach call rather than a C stdlib extern | Resolved — the `#darwin` arm calls `mach_continuous_time` + `mach_timebase_info`; both handlers are a documented boundary exception (`ABI_SPECIFICATION.md` §4, `ARCHITECTURE.md` §4.8) |

---

_Version 1.0.0 milestone. One function, two semantic rules (monotonicity, include-suspend), no new types, no keyword churn. All remaining time complexity (zones, calendars, leap tables, serialization, logging) lives in the standard library as versioned, replaceable components._

---

## Status notes

- 2026-10-25 — Retroactive migration: front matter added; file moved from `doc/proposals/implemented/builtin_uptime.md` to `doc/proposals/LDP-0002-builtin-uptime.md`. The body is unchanged apart from section renumbering (Open Points moved from §9 to §11) and the removal of the old `Document`/`Status`/`Scope` header block, whose contents now live in the front matter. The decision and the implementation predate the design-record system, so `decided` and `implemented` are recorded as `null` rather than guessed.
- 2026-10-25 — Availability rule revised. The unconditional "must not silently substitute a non-conforming clock" requirement was replaced by the two conformance levels in §5: degradation is now permitted where the platform lacks the required capability, provided it is declared in the ABI specification and recorded per target here. §1, §3, §4.4, §5, §7, and §9 were updated; the delivery list now names concrete platforms, including the WASI browser build.
- 2026-10-25 — Body revised to match the implementation. The record system is still a draft (`LDP-0001`) and Lale is pre-1.0, so the record was updated in place rather than superseded: §5 now records the compile-time platform branch (`#if #posix`, `#if #windows`), the abort-on-failure policy and why it writes to fd 2 instead of using `alert`, and the macOS gap (O5); the delivery list marks macOS `partial` for the same reason.
- 2026-10-25 — macOS policy decided. `mach_continuous_time` and `mach_timebase_info` are accepted as the only documented non-C entries in the platform boundary, so AOT and interpreted uptime remain identical on Darwin. Recorded in `AGENTS.md` (handler constraints), `ARCHITECTURE.md` §4.8, and `ABI_SPECIFICATION.md` §4; O5 narrows to the pending stdlib branch.
- 2026-10-25 — Platform arms implemented. `core.lale` now selects the clock with a top-level `#match`: `#linux`/`#darwin`/`#windows`/`#wasi`, `#else: #fail`. O5 is resolved: the Darwin arm calls the two Mach functions and the interpreter mirrors them. `docs-touched` also covers `AGENTS.md`. The interpreter's 8-byte struct-slot layout for sub-8-byte fields is a known divergence, handled with a documented dual write in the `mach_timebase_info` handler.
