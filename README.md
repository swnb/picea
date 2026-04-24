# Picea

Picea is a work-in-progress 2D physics engine written in Rust. The repository now centers on the `World`-based core crate. `crates/macro-tools` remains a standalone proc-macro crate in the workspace and is verified separately; it is not part of `crates/picea`'s current direct dependency graph.

## Workspace

This repository is a Rust workspace with two crates:

| Crate | Purpose |
| --- | --- |
| `crates/picea` | Core 2D physics engine centered on the `World` API, query/debug read models, and internal pipeline/solver modules. |
| `crates/macro-tools` | Standalone proc-macro crate in the workspace for derive helpers such as `Accessors`, `Builder`, and `Deref`, validated separately from the current `crates/picea` dependency graph. |

## Quick Start

Run the core library tests:

```bash
cargo test -p picea --lib
```

Run the standalone proc-macro crate tests:

```bash
cargo test -p picea-macro-tools
```

## Development Workflow

Start with current repo facts, not archived milestone notes:

1. Confirm current `git status`, `HEAD`, and recent verification output.
2. Read `crates/picea/src/lib.rs` for the current public crate-root surface.
3. Use `docs/ai/repo-map.md`, `docs/ai/index.md`, and `docs/architecture/system-overview.md` to route into the current modules.
4. Consult `docs/plans/2026-04-18-picea-physics-engine-milestones.md` only when the task explicitly needs a still-relevant milestone boundary or archived execution history.

For active milestone boundaries and archived execution history, see:

- `docs/plans/2026-04-18-picea-physics-engine-milestones.md`

That plan mixes still-useful milestone boundaries with archived execution records from removed `Scene`/`Context` paths and old `picea-web` / wasm gates. Treat those removed-path entries as historical notes, not default current routing or validation targets.

Before changing code, read:

- `AGENTS.md` for repository rules and authority order.
- `crates/picea/src/lib.rs` for the current public crate-root surface.
- `docs/ai/repo-map.md` for module ownership and test routing.
- `docs/architecture/system-overview.md` for the current crate/module boundary map.
- `docs/architecture/README.md` for the broader architecture doc index, including archived legacy runtime docs when needed.
- `docs/design/README.md` for design intent and future-facing decisions.
- `docs/ai/index.md` for question-to-source routing.
- `docs/ai/debug-playbook.md` when investigating bugs or regressions.

When reading milestone history, treat `cargo test -p picea-macro-tools` as a separate workspace verification gate for the proc-macro crate, not as evidence that `crates/picea` currently depends on it.

Milestone work should stay narrow:

1. Confirm current git status and `HEAD`.
2. Read the relevant milestone boundary.
3. Add a failing behavior lock or focused regression test.
4. Make the smallest implementation change.
5. Run the targeted tests and then the milestone gate.
6. Record residual risks instead of silently widening scope.

## Common Test Gates

Use current repo facts and live command output as the source of truth. When a task is explicitly milestone-scoped, consult the milestone plan only for the still-relevant boundary checks. Archived `Scene`/`Context` / `picea-web` / wasm gates in that file are historical, not current default targets.

These are the common workspace gates used across the current repository:

```bash
cargo test -p picea --lib
cargo test -p picea-macro-tools
cargo test --workspace --all-targets --no-run
```

`cargo test -p picea-macro-tools` is a standalone workspace/proc-macro gate. It does not by itself imply that `crates/picea` currently depends on `picea-macro-tools`.

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
- `docs/architecture/system-overview.md`
- `docs/architecture/README.md`
- `docs/design/README.md`
- `docs/ai/index.md`
- `docs/ai/repo-map.md`
- `docs/ai/doc-catalog.yaml`
- `docs/ai/evals.md`
- `.agents/skills/picea-doc-routing/SKILL.md`
- `.agents/skills/picea-milestone-runner/SKILL.md`

These files are for keeping future AI-assisted work scoped and reproducible. They do not replace the current code or test output as the source of truth.
