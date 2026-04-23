# Picea

Picea is a work-in-progress 2D physics engine written in Rust. The repository now centers on the `World`-based core crate plus an internal proc-macro helper crate.

## Workspace

This repository is a Rust workspace with two crates:

| Crate | Purpose |
| --- | --- |
| `crates/picea` | Core 2D physics engine centered on the `World` API, query/debug read models, and internal pipeline/solver modules. |
| `crates/macro-tools` | Internal proc macro helpers used by the engine. |

## Quick Start

Run the core library tests:

```bash
cargo test -p picea --lib
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
cargo test -p picea-macro-tools
cargo test --workspace --all-targets --no-run
```

Codex/agent sessions in this repository should prefix these commands with `rtk proxy`; see `AGENTS.md`.

`--no-run` is currently treated as a compile gate. It does not guarantee warning-clean output by itself; use dedicated warning cleanup when that is part of the goal.

## Debugging

Do not debug Picea by visual guessing alone. Prefer a minimal repro, a fixed `dt`, targeted behavior locks, and structured evidence.

Start with:

- `docs/ai/debug-playbook.md`
- `docs/ai/debug-artifacts.md`

The debug route usually starts from one of these modules:

- `crates/picea/src/world.rs` and `crates/picea/src/world/*` for authoritative state ownership and lifecycle APIs.
- `crates/picea/src/pipeline/*` and `crates/picea/src/solver/*` for stepping orchestration and internal solve phases.
- `crates/picea/src/query.rs` and `crates/picea/src/debug.rs` for stable read-side behavior.

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
