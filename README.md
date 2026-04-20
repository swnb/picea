# Picea

Picea is a work-in-progress 2D physics engine written in Rust, with a core engine crate and a wasm binding crate for JavaScript/WebAssembly consumers.

The project is currently being rebuilt milestone by milestone: first making the baseline verifiable, then tightening geometry contracts, deterministic stepping, storage/handles, shape caching, collision/manifold behavior, solver realism, and wasm API hardening.

## Workspace

This repository is a Rust workspace with three crates:

| Crate | Purpose |
| --- | --- |
| `crates/picea` | Core 2D physics engine: scene stepping, elements, math, shapes, collision, constraints, metadata, and debug tools. |
| `crates/picea-web` | wasm-bindgen public API over the core engine. |
| `crates/macro-tools` | Internal proc macro helpers used by the engine. |

## Quick Start

Run the core library tests:

```bash
cargo test -p picea --lib
```

Build all examples without running their windows:

```bash
cargo test -p picea --examples --no-run
```

Run one example:

```bash
cargo run -p picea --example ground
```

Run wasm API tests:

```bash
cargo test -p picea-web --lib
```

Run macro helper tests:

```bash
cargo test -p picea-macro-tools
```

## Development Workflow

The authoritative milestone plan is:

- `docs/plans/2026-04-18-picea-physics-engine-milestones.md`

Before changing code, read:

- `AGENTS.md` for repository rules and authority order.
- `docs/ai/repo-map.md` for module ownership and test routing.
- `docs/architecture/README.md` for architecture diagrams and runtime flow.
- `docs/design/README.md` for design intent and future-facing decisions.
- `docs/ai/index.md` for question-to-source routing.
- `docs/ai/debug-playbook.md` when investigating bugs or regressions.

Milestone work should stay narrow:

1. Confirm current git status and `HEAD`.
2. Read the relevant milestone boundary.
3. Add a failing behavior lock or focused regression test.
4. Make the smallest implementation change.
5. Run the targeted tests and then the milestone gate.
6. Record residual risks instead of silently widening scope.

## Common Test Gates

Use the milestone plan as the source of truth. These are the common gates used across milestones:

```bash
cargo test -p picea --lib
cargo test -p picea --examples --no-run
cargo test -p picea-macro-tools
cargo test -p picea-web --lib
cargo test --workspace --all-targets --no-run -- -D warnings
```

Codex/agent sessions in this repository should prefix these commands with `rtk proxy`; see `AGENTS.md`.

For wasm smoke tests, use the wasm-bindgen runner when it is installed and version-compatible:

```bash
CARGO_TARGET_WASM32_UNKNOWN_UNKNOWN_RUNNER=wasm-bindgen-test-runner \
  cargo test -p picea-web --lib --target wasm32-unknown-unknown
```

## Debugging

Do not debug Picea by visual guessing alone. Prefer a minimal repro, a fixed `dt`, targeted behavior locks, and structured evidence.

Start with:

- `docs/ai/debug-playbook.md`
- `docs/ai/debug-artifacts.md`

The debug route usually starts from one of these modules:

- `crates/picea/src/scene/mod.rs` for tick order, fixed-step behavior, callbacks, sleep/wakeup.
- `crates/picea/src/collision/mod.rs` for broadphase, narrowphase, contact pairs, and contact keys.
- `crates/picea/src/constraints/` for contact/join/point solving, warm start, lambda, effective mass.
- `crates/picea/src/shape/` for local/world geometry, convex/concave decomposition, projections, and shape cache behavior.
- `crates/picea-web/src/` for JS/Rust boundary and wasm public API issues.

## AI Context

This repository includes local AI routing and workflow docs:

- `AGENTS.md`
- `docs/architecture/README.md`
- `docs/design/README.md`
- `docs/ai/index.md`
- `docs/ai/repo-map.md`
- `docs/ai/doc-catalog.yaml`
- `docs/ai/evals.md`
- `.agents/skills/picea-doc-routing/SKILL.md`
- `.agents/skills/picea-milestone-runner/SKILL.md`

These files are for keeping future AI-assisted work scoped and reproducible. They do not replace the current code or test output as the source of truth.
