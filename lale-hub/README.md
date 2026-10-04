# lale-hub

**Status: not started.**

This crate is a placeholder for the Lale package hub — the `hub` dependency
origin (`use ... from hub.<package> version <x.y.z>`).

It will eventually provide package download, local caching, exact-version
selection, and integrity/checksum verification.

## Design

See `doc/ARCHITECTURE.md` §4.6 for the decisions behind the
`std` / `local` / `hub` dependency-origin system, and `doc/roadmap.md` §4.3
(item 9) for the 2.0 workpackage.
