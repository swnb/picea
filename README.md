# Picea

Picea is a work-in-progress 2D physics engine written in Rust. The repository now centers on the `World`-based core crate. `crates/picea-lab` is a local C/S simulator wrapper for scenarios, artifacts, and visualization. `crates/macro-tools` remains a standalone proc-macro crate in the workspace and is verified separately; neither lab nor macro tooling is part of `crates/picea`'s core runtime dependency graph.

## Workspace

This repository is a Rust workspace with three crate families:

| Crate | Purpose |
| --- | --- |
| `crates/picea` | Core 2D physics engine centered on the `World` API, query/debug read models, and internal pipeline/solver modules. |
| `crates/picea-lab` | Local C/S simulator tooling: deterministic scenarios, artifact capture, HTTP/SSE server, CLI, and the React Canvas workbench under `crates/picea-lab/web`. |
| `crates/macro-tools` | Standalone proc-macro crate in the workspace for derive helpers such as `Accessors`, `Builder`, and `Deref`, validated separately from the current `crates/picea` dependency graph. |

## Public Beta Surface

The current beta path is the `World` API plus explicit stepping and read-side
inspection:

- `World`, `WorldDesc`, body/collider/joint descriptors, and opaque handles own
  authoritative state.
- `SimulationPipeline`, `StepConfig`, and `StepReport` own fixed-step cadence and
  per-step counters/events.
- `DebugSnapshot` and `QueryPipeline` provide stable read-side inspection.
- `WorldRecipe`, `BodyBundle`, `ColliderBundle`, and `JointBundle` provide
  reproducible setup for examples, tests, and lab scenarios.
- `picea-lab` hosts local scenarios, artifact capture, live request-driven
  sessions, and the React Canvas workbench. It explains core facts rather than
  recomputing physics in the browser.

For the beta scope, non-goals, migration notes, and final verification matrix,
see:

- `docs/public-beta.md`

The checked core example is:

- `crates/picea/examples/public_beta_smoke.rs`

## Quick Start

Run the core library tests:

```bash
rtk proxy cargo test -p picea --lib
```

Compile the public beta example:

```bash
rtk proxy cargo test -p picea --examples --no-run
```

Run the standalone proc-macro crate tests:

```bash
rtk proxy cargo test -p picea-macro-tools
```

Run the local simulator crate tests:

```bash
rtk proxy cargo test -p picea-lab
```

Start the local simulator server for the web workbench:

```bash
rtk proxy cargo run -p picea-lab -- serve --bind 127.0.0.1:18080
```

Or start the Rust API and `picea-lab-web` Vite workbench together:

```bash
just picea-lab-web
```

For a background service lifecycle, use:

```bash
just web-start
just web-stop
just status
just logs
```

The recipe uses `127.0.0.1:8080` for the Rust API and `127.0.0.1:5173`
for Vite by default. Override with `PICEA_LAB_BIND` or
`PICEA_LAB_WEB_PORT` when those ports are already occupied. Background service
state and logs live under `target/picea-lab-web/` by default; override that
with `PICEA_LAB_SERVICE_DIR` if needed.

Codex App users can select the local environment at
`.codex/environments/environment.toml`; it exposes the same Start, Stop,
Status, Logs, foreground workbench, API-only, UI-only, and web contract actions.

When working on `picea-lab-web`, start here first:

```bash
rtk proxy just picea-lab-web
```

For acceptance, prefer `browser-use:browser` against the actual local dev URL
and explicitly walk all three surfaces: demo fallback, Rust artifact replay,
and Rust live session. M40 closeout expects browser checks for scenario
grouping, overlay presets, stack stability, trajectory focus, paused
perturbation review, lattice proxy wording, and copy-debug-context output.

## Development Workflow

Start with current repo facts, not archived milestone notes:

1. Confirm current `git status`, `HEAD`, and recent verification output.
2. Read `crates/picea/src/lib.rs` for the current public crate-root surface.
3. Use `docs/ai/repo-map.md`, `docs/ai/index.md`, and `docs/architecture/system-overview.md` to route into the current modules.
4. Consult `docs/plans/2026-04-25-picea-physics-engine-production-milestones.md` for current production milestone scope and acceptance methods.
5. Consult `docs/plans/2026-04-18-picea-physics-engine-milestones.md` only when the task explicitly needs archived execution history.

For current production milestone scope and acceptance methods, see:

- `docs/plans/2026-04-25-picea-physics-engine-production-milestones.md`

For archived execution history, see:

- `docs/plans/2026-04-18-picea-physics-engine-milestones.md`

The archived plan mixes old milestone boundaries with execution records from removed `Scene`/`Context` paths and old `picea-web` / wasm gates. Treat those removed-path entries as historical notes, not default current routing or validation targets.

Before changing code, read:

- `AGENTS.md` for repository rules and authority order.
- `docs/public-beta.md` for the current beta surface and migration notes.
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

## Canonical Validation Profiles

Use the repository profiles as the canonical delivery gates:

Use stable Rust with `rustfmt` for fast and `clippy` plus `rust-src` for full.
The source component is needed for standard-library excerpts in the compile-fail
diagnostic snapshots; CI installs these components explicitly.
The full profile also needs `just` for its dev-server recipe contracts. The
hosted full job installs the locally validated `just 1.45.0` explicitly.
RTK is optional for Justfile tool recipes: when available they use `rtk proxy`,
otherwise they invoke the same tools directly. The dev-server contract also
runs real Vite with an isolated PATH containing no RTK, so a developer's local
installation cannot hide this hosted-runner prerequisite boundary.

```bash
just ci-fast
just ci-full
just ci-nightly
just ci-release
```

- `ci-fast` checks the release runner contract, formatting, core smoke tests,
  Web contracts, and the production bundle.
- `ci-full` runs the release runner contract, workspace all-target tests,
  strict Clippy, a high-severity audit of the complete Web build dependency
  tree, and the complete Web contract suite.
- `ci-nightly` runs the 600/1200-frame matrix-stack gates and verifies the exact
  nine-scenario Criterion counter manifest. Criterion wall-clock results remain
  review evidence, not a one-run correctness threshold.
- `ci-release` verifies package metadata and builds package archives. It never
  publishes. A dirty local run is labeled only as a buildable candidate; a
  clean hosted-CI receipt is required before claiming clean or reproducible
  packaging.

The runner contract exercises clean/dirty local and CI inputs, exact package
arguments, and failure propagation on the current Bash (including macOS Bash
3.2). It uses stub external tools and does not replace actual package builds.

The thin recipes call `scripts/ci/run.sh`, which uses `rtk proxy` locally and
the same underlying commands directly on a standard CI runner. GitHub workflow
results remain unknown until the branch containing them is pushed and actually
runs.

## Common Test Gates

Use current repo facts and live command output as the source of truth. When a task is explicitly milestone-scoped, consult the milestone plan only for the still-relevant boundary checks. Archived `Scene`/`Context` / `picea-web` / wasm gates in that file are historical, not current default targets.

These are the common workspace gates used across the current repository:

```bash
cargo test -p picea --lib
cargo test -p picea-lab
cargo test -p picea-macro-tools
cargo test -p picea --examples --no-run
cargo test --workspace --all-targets --no-run
```

`cargo test -p picea-macro-tools` is a standalone workspace/proc-macro gate. It does not by itself imply that `crates/picea` currently depends on `picea-macro-tools`.

Codex/agent sessions in this repository should prefix these commands with `rtk proxy`; see `AGENTS.md`.

`--no-run` is currently treated as a compile gate. It does not guarantee warning-clean output by itself; use dedicated warning cleanup when that is part of the goal.

## Debugging

Do not debug Picea by visual guessing alone. Prefer a minimal repro, a fixed `dt`, targeted behavior locks, and structured evidence.

Start with:

- `docs/ai/debug-playbook.md`

`debug-playbook.md` also defines the recommended repro, trace, snapshot,
render, and verification artifact shape for reviewable debugging.

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
