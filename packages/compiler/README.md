# @tez/compiler

Phase 1 deliverable. Rust crate (oxc-based parser/semantic analysis), exposed to
the Vite plugin via napi-rs bindings. See `docs/superpowers/specs/2026-07-05-tez-architecture-design.md` §3.

Implemented so far (no pipeline/driver yet — each is a public library function):

- `parse()` — oxc-based TSX parsing (`src/lib.rs`).
- `extract_structure()` — structural summary: functions, JSX tags/attributes,
  expression containers (`src/lib.rs`).
- `find_reactive_bindings()` — import-resolved `signal()`/`computed()` binding
  detection via `oxc_semantic` (`src/semantic.rs`).
- `classify_jsx_expressions()` — per-component static vs. signal-driven
  classification of JSX expressions (`src/reactivity.rs`).
- `check_body_signal_writes()` — `TEZ101`: signal write during component body
  execution (`src/tez101.rs`), producing `Diagnostic` values (`src/diagnostics.rs`)
  that carry code + span + cause + fix + docs URL and render to a snapshot-tested
  plain-text form. Docs URL convention: `https://tez.dev/errors/<CODE>`.
- `serialize_static()` — static JSX element tree → escaped HTML template
  string; owns void elements, boolean attributes, and the reserved `v-*` /
  `use:` directive namespaces (`src/template_html.rs`).
- `compile_dom()` — DOM codegen entry point (sub-cycle 1: static components
  only): hoists `const _tN = template("…")`, replaces JSX with `_tN()` clone
  calls, injects the `@tez/runtime-dom` import, prints via `oxc_codegen`.
  Everything dynamic is an explicit `Unsupported` error until its sub-cycle
  lands (`src/codegen.rs`).

All oxc crates (including `oxc_codegen`) are pinned to 0.116.0 (rustc 1.91.1
compatibility) — see `Cargo.toml` before touching dependencies.
