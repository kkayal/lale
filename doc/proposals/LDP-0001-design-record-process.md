---
number: LDP-0001
title: The Lale Design Record System
status: draft
scope: process
milestone: 1.0.0
author: Kağan Kayal
decided-by: null
created: 2026-10-25
decided: null
implemented: null
supersedes: null
superseded-by: null
docs-touched:
  - AGENTS.md
---

# The Lale Design Record System

## 1. Objective

The objective is to give every significant Lale design decision a permanent, citable
record that answers three questions:

- **What is the rule?**
- **Why is the rule that way?**
- **What did we reject, and why?**

Such a record prevents two failures: _design drift_, where the code, the specification, and
the rules the maintainers believe diverge; and _decision amnesia_, where a question settled
years ago is reopened from zero. This proposal defines the process that produces and
preserves that record.

An **LDP** (Lale Design Proposal) is a numbered Markdown file that states a decision, the
reasoning behind it, the alternatives that lost, and what it costs. The specification says
what is true; the proposal says why.

This process covers proposals only; broader project-level principles are outside its scope.

## 2. When a proposal is required

A proposal is required for a change to:

- language syntax, semantics, or the type system;
- the intermediate representation (IR) contract or a compiler invariant;
- the application binary interface (ABI) or an extern-boundary behaviour;
- the public standard-library API;
- a documented diagnostic or error-message contract;
- this process.

A proposal is not required for a bug fix that restores documented behaviour, a refactor
with no observable effect, a test that verifies existing behaviour, or a performance
improvement that stays inside an accepted design.

If it is unclear which side a change falls on, write the proposal. A short record is
cheaper than a lost argument.

## 3. The proposal record

### 3.1 One flat directory

Proposals live in one flat directory, regardless of status:

```text
doc/
  proposals/
    LDP-0001-design-record-process.md
    LDP-0002-builtin-uptime.md
  lale.md                 # language specification
  ARCHITECTURE.md         # compiler architecture
  ABI_SPECIFICATION.md    # ABI specification
  roadmap.md              # roadmap
  TODO.md                 # working backlog
```

The filename is `LDP-NNNN-short-title.md`. Numbers are assigned when a draft is first
added to the repository, start at 1, and are never reused or renumbered — even if the
draft is later deleted. A number is a permanent citation: `LDP-0002 §4.4` must keep
resolving, even after the proposal is superseded.

There are no status-named subdirectories. A record's location never contradicts its
`status:` field, and moving a record through review does not break links or Git history.

A draft may be edited, split, merged, or deleted. Its number is retired if the draft is
deleted; the record enters the proposal corpus when its status becomes `submitted`.

### 3.2 Status

Every proposal has exactly one status:

| Status        | Meaning                                               |
| ------------- | ----------------------------------------------------- |
| `draft`       | Being written. May be changed or deleted freely.      |
| `submitted`   | Under review. Editable; the record enters the corpus. |
| `accepted`    | The decision is approved. The text is frozen.         |
| `rejected`    | Considered and declined. Kept for the record.         |
| `withdrawn`   | Retired before a decision. Kept for the record.       |
| `implemented` | Accepted and delivered.                               |
| `superseded`  | Replaced by a later accepted proposal.                |

A proposal moves from `draft` to `submitted`. The decider then marks it `accepted` or
`rejected`, or the author withdraws it. An accepted proposal becomes `implemented` when
the work is delivered, and `superseded` when a later proposal replaces it.

A submitted proposal remains editable during review. The author may revise it in response
to discussion, and Git preserves the sequence of revisions. Review revisions must be
committed; the merge policy must not squash them away. The text becomes immutable only
when the proposal is accepted.

`accepted` and `implemented` are different facts. `accepted` means the design is settled;
`implemented` means the accepted change has been delivered, the affected documentation has
been updated, and the acceptance criteria have been verified. A proposal may stay
`accepted` for a long time before it is implemented.

`implemented` applies to the proposal as a whole. If only part of the work is complete,
the proposal remains `accepted`.

### 3.3 Front matter

Every record starts with a small YAML block:

```yaml
---
number: LDP-0002
title: `__lale_read_uptime()` builtin function
status: implemented
scope: language # language | compiler | tooling | process
milestone: 1.0.0
author: Kağan Kayal
decided-by: Kağan Kayal
created: 2025-11-03
decided: 2025-11-17
implemented: 2025-12-02
supersedes: null
superseded-by: null
docs-touched: [doc/ABI_SPECIFICATION.md, doc/lale.md]
---
```

- `number` is the permanent identifier.
- `title` is the canonical title and must match the body's H1 heading.
- `status` is one value from §3.2.
- `scope` is the primary area affected.
- `milestone` is the intended release; it is not a delivery promise.
- `author` maintains the text; `decided-by` records who accepted or rejected it.
- `created`, `decided`, and `implemented` are dates. Leave a date `null` until it
  happens, or while it is unknown.
- `supersedes` names the record this one replaces; the older record gets
  `superseded-by` once the replacement is accepted.
- `docs-touched` lists the specification and other documentation that must be updated to
  reflect the accepted decision. Source-code changes stay visible in their ordinary Git
  commits.

Extra fields are allowed when they help; only the fields above are required. One useful
example is an optional per-target `implementation` list:

```yaml
implementation:
  - target: hosted
    state: yes
  - target: bare-metal
    state: partial
    reason: no always-on counter spanning suspend
```

`state` is one of `yes`, `partial`, `no`, or `n/a`; a `reason` is required for `partial` or
`no` and may be added to any entry. Target names are free-form. The list is informational: it does not change the proposal's
whole-proposal status (§3.2). `n/a` means the target is deliberately outside the
proposal's scope. Use `partial` or `no` only when the proposal intends to support the
target but has not fully delivered it; in that case the proposal remains `accepted` rather
than `implemented` (§3.2).

### 3.4 Body

The body follows a fixed spine. Sections marked _mandatory_ are the ones that prevent bad
decisions from slipping through.

1. **Objective (or Context)** — the problem, constraints, and why a decision is needed.
2. **Definition** — the change, stated precisely.
3. **Rules** — every resulting rule, stated so it can be tested. A language proposal may
   call this **Semantics**.
4. **Implementation boundary** — what the change touches, and what it must not affect.
5. **Alternatives considered** _(mandatory)_ — the options that lost, and why.
6. **Consequences** _(mandatory)_ — what becomes easier, harder, or impossible.
7. **Acceptance criteria** _(mandatory)_ — how we know the change is correct.
8. **Open points** _(mandatory)_ — anything unresolved, with a disposition (§4.2).

## 4. Rules

### 4.1 Accepted and implemented records are frozen

An accepted or implemented record may be edited only for spelling, grammar, formatting, or
a broken link. Everything else requires a new proposal with `supersedes:`.

Metadata changes that record delivery or supersession are also allowed. When an accepted
proposal becomes `implemented`, its `status` and `implemented` fields may be updated. When
a later proposal is accepted, the older record's `status` becomes `superseded` and its
`superseded-by` field is filled in. Neither change alters the accepted decision content.

A trailing **Status notes** section is also permitted, for dated facts that do not change
the decision:

```text
- 2026-11-09 — bare-metal target now conforms; see LDP-0042.
```

If accepted records can be edited, the rationale slowly rewrites itself to agree with the
present. Freezing the text is what makes the corpus trustworthy.

### 4.2 Open points

Before a proposal is accepted, every open point must be in exactly one state:

| State            | Requires                                                             |
| ---------------- | -------------------------------------------------------------------- |
| **Resolved**     | A concrete answer, with a citation.                                  |
| **Out of scope** | An explicit statement that the matter is outside the contract.       |
| **Deferred**     | A numbered follow-up proposal with `submitted` status, cited inline. |

"To be determined later" is not a disposition.

### 4.3 Governance

- **Acceptance is a dated event.** One commit changes `status` to `accepted` (or
  `rejected`) and fills `decided` and `decided-by`.
- **Every decision has a named decider**, even when that is always the same person.
- **Anyone may write a draft.** Only the decider accepts or rejects.
- **Rejection is permanent.** Reopening requires a new proposal that cites the rejected
  one and supplies new facts, new constraints, or a changed premise.
- **Withdrawal is recorded.** The author withdraws a submitted proposal by changing its
  status to `withdrawn`; the Git commit records who and when, and a dated **Status notes**
  entry records the reason. A withdrawn proposal is not reopened; a renewed effort is
  submitted as a new proposal referencing the withdrawn record.
- **Supersession is a chain.** A new proposal names the old one in `supersedes`; the old
  record gets `superseded-by` and changes its status to `superseded` only after the new
  one is accepted.

## 5. Alternatives considered

| Alternative                              | Rejected because                                                                                                                                                                                           |
| ---------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| **Architecture decision records (ADRs)** | A suitable lightweight pattern. LDP adopts the same basic approach, but adds an explicit `accepted` vs `implemented` distinction and a consistent structure for language, compiler, and process decisions. |
| **Status-named subdirectories**          | They duplicate the status metadata, require file moves during the lifecycle, and can break links or add unnecessary Git history noise.                                                                     |
| **An issue tracker**                     | Not the canonical, repository-versioned record: it is vendor-owned and not part of the clone.                                                                                                              |
| **A wiki**                               | Not the canonical, repository-versioned record: it is not reviewed or cloned with the code.                                                                                                                |
| **Editable accepted records**            | The rationale rewrites itself to agree with the present.                                                                                                                                                   |
| **A single growing decision log**        | No stable citation per decision and no independent status.                                                                                                                                                 |

## 6. Consequences

The benefits are direct: every rule has a stable citation, rejected alternatives stay
available, and a new contributor can read why the language looks the way it does.

The costs are equally direct:

- **Front matter must be maintained.** The metadata is small, but it must be correct.
- **Immutability is friction.** Changing a decision means writing a new record, not
  editing a paragraph.
- **Discipline is required.** The process has no validator; it works only if the
  maintainers follow it.

Adopting this proposal migrates existing proposal documents under `doc/proposals/`, gives
them permanent numbers and front matter, and leaves their bodies unchanged. It does not
require creating new records for decisions that were never documented as proposals, and it
does not introduce a generated index or a validation tool.

## 7. Acceptance criteria

| Requirement                                                                                                   | Verification                                                     |
| ------------------------------------------------------------------------------------------------------------- | ---------------------------------------------------------------- |
| `doc/proposals/` contains only flat `LDP-NNNN-*.md` files                                                     | `find doc/proposals -type d`                                     |
| Every record has the front matter in §3.3 and an H1 matching `title`                                          | document review                                                  |
| LDP-0002 is present as `doc/proposals/LDP-0002-builtin-uptime.md`                                             | file inspection                                                  |
| A submitted proposal can be revised during review, and Git retains its revisions                              | Inspect the history of a proposal with multiple review revisions |
| An accepted proposal's decision content remains unchanged; supersession only updates its status and reference | Inspect a superseded proposal and its Git history                |

## 8. Open points

| #   | Open point                                                        | State                                                            |
| --- | ----------------------------------------------------------------- | ---------------------------------------------------------------- |
| O1  | Retroactive records for decisions made before this system existed | Out of scope — each backfilled decision would be its own record. |
