# Proposal: `__lale_read_uptime()` Builtin Function

**Document:** `doc/proposals/implemented/builtin_uptime.md`
**Status:** Implemented
**Scope:** Language core for Lale 1.0.0 — boot-time counter as a plain value, no new type

---

## 1. Purpose

Provide a simple, honest source of boot-relative elapsed time for Lale targets — embedded, bare-metal, hosted — without requiring a complex time type, clocks, zones, or calendar policies.

**Goal:** `__lale_read_uptime()` returns "nanoseconds since boot" as a plain `u64 in <ns>`, usable with Lale's existing unit system and arithmetic.

**Availability.** The interface is available on every Lale 1.0 target whose platform implementation can satisfy the contract in Section 4. A target that cannot — for example, a bare-metal chip with no always-on counter to measure suspended time — must not silently substitute a non-conforming clock; the platform ABI defines how such targets are handled.

---

## 2. Definition

```lale
import fn signature __lale_read_uptime() returns u64 in <ns>
```

`__lale_read_uptime` is declared in `builtins.lale` — like `__lale_malloc` — as an `import fn` hook. The compiler knows only its type and signature; the platform (OS kernel, runtime, SDK, or bare-metal BSP) provides the implementation.

On hosted targets the standard library (`core.lale`) supplies that implementation as an `export fn` that wraps the platform's monotonic boot-time clock — the same "`import fn` hook + `export fn` wrapper" pattern as `__lale_malloc` wrapping `malloc` (see §4.4 for the per-OS clock). In no-stdlib/embedded mode, the `import fn signature` above is the hook the platform must satisfy at link time.

**Impurity.** Each evaluation obtains a fresh platform reading. The compiler must not eliminate, cache, merge, or reorder calls in a way that changes observable results or violates ordering semantics (Section 4.5).

---

## 3. Return Value and Type

| Property         | Value                                                                                                                                                   |
| ---------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------- |
| **Return type**  | `u64 in <ns>` — unsigned 64-bit integer with unit `<ns>`                                                                                                |
| **Meaning**      | A non-decreasing, boot-relative reading of elapsed time, expressed in nanoseconds per the platform's monotonic boot-time clock; includes suspended time |
| **Range**        | 0 to 2⁶⁴ − 1 (≈ 584 years)                                                                                                                              |
| **Sign**         | Unsigned — boot is the origin; no pre-boot readings                                                                                                     |
| **Monotonicity** | Non-decreasing within a boot session                                                                                                                    |
| **Wraparound**   | Must not wrap within a boot session; behavior beyond the representable range is out of scope                                                            |

**Units.** The `<ns>` is part of the return type and is enforced by the type checker; no caller-side annotation is needed.

**Resolution and accuracy.** The value may be quantized to the platform's tick, so two consecutive calls may return the same value — that is normal, not an error. The counter is **non-decreasing**, not strictly increasing. The language guarantees neither nanosecond resolution nor a specific accuracy relative to physical SI time: clocks drift and undergo frequency adjustment, so the `<ns>` unit is a _scale_, not a promise of measurement accuracy.

---

## 4. Semantics

### 4.1 Monotonicity

`__lale_read_uptime()` is **monotonic non-decreasing within a boot session**: successive readings never decrease. They may repeat when the platform's resolution is coarser than 1 ns. This guarantee holds across threads and cores — a later sequential reading must not return a lower value merely because execution migrated to another core.

### 4.2 The boot reference point

On hosted systems the OS exposes a boot-relative clock. On bare metal, the timebase may be initialized by the runtime or BSP after reset, not at the instant the device began booting.

The reference point is the **platform's defined boot epoch**, including early startup where the platform can measure it. If a target cannot account for the interval before its timebase becomes operational, that limitation is a platform-contract matter, not a redefinition of the feature.

### 4.3 Who keeps it monotonic

A fast-wrapping hardware counter cannot be returned directly — a 32-bit counter at 1 MHz wraps every ~71 minutes. The **platform** (OS, SDK, or BSP) — not the compiler — hides the wrap by extending the counter to 64 bits, using one of these standard techniques:

- a **64-bit hardware counter** (effectively no wrap), or
- an **overflow interrupt** that fires exactly at wrap and increments a software high word, or
- a **periodic interrupt** that samples more often than the wrap period and increments the high word on a backwards jump.

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

This is illustrative only. The normative requirement is that the implementation obtain a coherent counter snapshot, account for every wrap, and remain correct across concurrency and suspend/wake transitions; full guidance (multi-core synchronization, suspend reconciliation) belongs in the platform/ABI documentation.

### 4.4 Suspend time

**Include all elapsed time**, including time spent in sleep/suspend states.

> Rationale: if a device sleeps for an hour, that hour happened; a data logger measuring intervals across suspend should see it.

Implementation note: use `CLOCK_BOOTTIME` on Linux/Android (not `CLOCK_MONOTONIC`, which excludes suspend). On macOS use `mach_continuous_time` — macOS has no `CLOCK_BOOTTIME`, and `mach_continuous_time` is its suspend-inclusive monotonic clock. On Windows use `QueryInterruptTimePrecise` (100-nanosecond units since boot, including suspend); the `QueryUnbiasedInterruptTime*` variants exclude sleep and must not be used. On bare metal, use an always-on counter, or reconcile the high word on wake-up.

### 4.5 Observation semantics

Each call performs one platform read. Because `__lale_read_uptime()` is an `import fn` call, Lale treats it as effectful and must not eliminate, cache, merge, or reorder calls in a way that changes observable results.

### 4.6 Boot-to-boot independence and wraparound

Values from different boots are **incomparable** — the counter resets to 0 on each boot. Comparison across boot boundaries is meaningless, and detecting reboot is out of scope for 1.0.0 (a library concern).

The 64-bit nanosecond counter must **not wrap within a boot session**. The range 2⁶⁴ ns (≈ 584 years) exceeds any realistic boot; behavior beyond it is out of scope and no provision is made.

---

## 5. Implementation Boundary

| Component    | Responsibility                                                                                                                                                                                                                                                                                                                                                         |
| ------------ | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| **Compiler** | Type-checks `__lale_read_uptime()` as `u64 in <ns>`; treats it as an effectful `import fn` (no folding, CSE, or reordering); emits a normal call                                                                                                                                                                                                                       |
| **Platform** | Implements `__lale_read_uptime()` — reads and extends the hardware counter, includes suspend time, ensures monotonicity and a coherent snapshot. On hosted systems the primitive is the platform's suspend-inclusive clock — `clock_gettime(CLOCK_BOOTTIME, &ts)` on Linux/Android, `mach_continuous_time` on macOS, `QueryInterruptTimePrecise` on Windows (see §4.4) |
| **ABI**      | Specifies the symbol, calling convention, resolution, and suspend reconciliation per target; defines how non-conforming targets are handled                                                                                                                                                                                                                            |

**C ABI signedness.** On POSIX systems (Linux/macOS) the hosted wrapper uses `clock_gettime`, whose `struct timespec` has **signed** fields (`time_t tv_sec`, `long tv_nsec`). That signedness is an internal C-ABI detail — `time_t` must represent pre-epoch wall-clock times. For `CLOCK_BOOTTIME` the seconds and nanoseconds are always non-negative, so the wrapper's signed-to-unsigned conversion to the public `u64` is value-preserving. The public contract is the unsigned `u64` return (§3); the signed C type never crosses the Lale boundary. Windows has no such signed struct — `QueryInterruptTimePrecise` returns an unsigned value directly.

**No new type.** `__lale_read_uptime()` returns an ordinary `u64` with a unit annotation — no flags, no layout guarantee, no ABI stability beyond the function signature.

**Conforming targets.** A conforming target must provide a non-decreasing reading and account for suspended time. A target that cannot meet the contract must not silently substitute a clock that excludes suspend time.

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

- **Wall-clock time** (UTC, calendars, zones, leap seconds) is a separate concern, provided by libraries.
- **`CLOCK_MONOTONIC`** is monotonic but excludes suspend; `__lale_read_uptime()` includes suspend (matching `CLOCK_BOOTTIME`).
- **`__lale_timestamp()`** — the pre-existing `builtins.lale` extern that returns the current UTC wall-clock time as ISO-8601 text — is a wall-clock concern, separate from `__lale_read_uptime()`. It moves to the standard library (a later, separately-scoped step).
- **`__lale_log_level()`** remains a builtin, because the `log`/`warn`/`alert` keywords lower directly to it as their runtime threshold hook.

Lale's logger uses `__lale_read_uptime()` for elapsed-time columns and obtains wall-clock timestamps from a library.

Everything else is a library, platform, or ABI concern.

---

## 8. Alternatives Considered

| Alternative                                           | Rejected because                                                                    |
| ----------------------------------------------------- | ----------------------------------------------------------------------------------- |
| **Keyword `uptime`**                                  | Consumes a scarce keyword namespace; a plain function achieves the same effect      |
| **New type (`Uptime`, `duration`, …)**                | Unnecessary; `u64 in <ns>` already carries the unit                                 |
| **Bare `uptime()` (no `__lale_` prefix)**             | Inconsistent with the existing `__lale_*` builtin convention                        |
| **Exclude suspend (`CLOCK_MONOTONIC`)**               | Undercounts intervals across suspend                                                |
| **Return `f64 in <ns>`**                              | Floating point has rounding error; `u64` covers ~584 years exactly                  |
| **A full time model (instant/timestamp/ruler types)** | Prior proposal iterations proved the dead end; that complexity belongs in libraries |

---

## 9. Open Points

| #   | Open point                                                                                                             | Status                                                                                                                                 |
| --- | ---------------------------------------------------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------- |
| O1  | Platform ABI for `__lale_read_uptime()` (calling convention, resolution, multi-core coherence, suspend reconciliation) | Resolved — recorded in `doc/ABI_SPECIFICATION.md` (per-target clock, calling convention, multi-core coherence, suspend reconciliation) |
| O2  | Boot-ID / reboot detection                                                                                             | Resolved — `boot_id()` as a standard-library function (Linux `/proc/sys/kernel/random/boot_id`); not a core-language feature           |

---

## 10. Acceptance Criteria for 1.0.0

The following are normative requirements, each verified by a test rather than merely asserted:

| Requirement                                                                                                                                                           | Verification      |
| --------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ----------------- |
| Compiles without the standard library                                                                                                                                 | build test        |
| Returns `u64 in <ns>` (no new type)                                                                                                                                   | type-checker test |
| Monotonic non-decreasing within a boot session                                                                                                                        | platform test     |
| Includes suspend time where the platform can measure it                                                                                                               | platform test     |
| Hosted stdlib wrapper uses the platform's suspend-inclusive clock (`CLOCK_BOOTTIME` Linux/Android, `mach_continuous_time` macOS, `QueryInterruptTimePrecise` Windows) | platform test     |
| Never constant-folded, CSE'd, or reordered                                                                                                                            | optimizer test    |
| No new grammar keywords                                                                                                                                               | grammar test      |
| Type checker enforces unit compatibility                                                                                                                              | unit test         |

Non-conforming targets are out of scope for this contract and are handled by the platform ABI.

"Compiles without the standard library" means the `import fn` declaration lives in the compiler's embedded `builtins.lale`, so the `u64 in <ns>` signature is known even with the stdlib disabled. The _implementation_ is stdlib-supplied on hosted targets (or platform-supplied in embedded/no-stdlib builds); user Lale code does not call the raw hook directly in no-stdlib mode.

---

_Version 1.0.0 milestone. One function, two semantic rules (monotonicity, include-suspend), no new types, no keyword churn. All remaining time complexity (zones, calendars, leap tables, serialization, logging) lives in the standard library as versioned, replaceable components._
