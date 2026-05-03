# Picea Public Beta Guide

Status: public beta hardening snapshot for the current `World` +
`SimulationPipeline` engine path.

This guide is the user-facing entry point for the beta surface. It intentionally
describes the stable workflow instead of every internal milestone detail.

## What Is In The Beta

- Core world ownership through `World`, `WorldDesc`, opaque handles, and
  lifecycle methods such as `create_body` and `create_collider`.
- Explicit fixed-step simulation through `SimulationPipeline`, `StepConfig`, and
  `StepReport`.
- Read-side inspection through `DebugSnapshot` and `QueryPipeline`.
- Declarative setup through `WorldRecipe`, `BodyBundle`, `ColliderBundle`, and
  `JointBundle`.
- Local debugging through `picea-lab` scenarios, artifact capture, HTTP routes,
  SSE events, and the React Canvas workbench.
- Lab authoring helpers for validated convex compound pieces and static
  `concave_polygon` decomposition into deterministic convex pieces.

## What Is Not In The Beta

- No 1.0 semver freeze. The beta surface is the current supported entry point,
  but the project may still make reviewed breaking changes before 1.0.
- No direct arbitrary concave contact solver in core. Static concave polygon
  authoring is decomposed by `picea-lab` before core world instantiation.
- No broad all-shape CCD or rotational CCD. The current CCD evidence covers the
  accepted translational slices, including dynamic compound bodies.
- No multithreaded solver execution or unified contact/joint row stream.
- No server-owned background autoplay loop for live sessions. The current live
  lab session advances when the client sends request-driven step commands.
- No running-world patch endpoint. Live override/patch requests are rejected
  until a paused-only transaction contract is implemented in a later milestone.

## Minimal Core Flow

Use `picea::prelude::*` for the beta path:

1. Create a `World` from `WorldDesc`.
2. Add bodies, colliders, and joints through lifecycle APIs or a `WorldRecipe`.
3. Step the world through `SimulationPipeline::step`.
4. Inspect deterministic counters and events from `StepReport`.
5. Build a `DebugSnapshot` or sync a `QueryPipeline` when a read-side view is
   needed.

The checked example lives at:

- `crates/picea/examples/public_beta_smoke.rs`

Compile it with:

```bash
rtk proxy cargo test -p picea --examples --no-run
```

## Minimal Lab Flow

List and run scenarios with the local simulator:

```bash
rtk proxy cargo run -p picea-lab -- list
rtk proxy cargo run -p picea-lab -- run falling_box_contact
```

Start the HTTP/SSE server for the web workbench:

```bash
rtk proxy cargo run -p picea-lab -- serve --bind 127.0.0.1:18080
```

The lab is an evidence layer over core facts. It should explain artifacts,
snapshots, query results, provenance, and live-session state; it should not
recompute physics in the browser.

## Migration Notes

- Treat old `Scene` / `Context` / `picea-web` / wasm milestone notes as archived
  history. Current routing starts from `World`, `SimulationPipeline`,
  `QueryPipeline`, `DebugSnapshot`, and `WorldRecipe`.
- Prefer handles and public view APIs over assuming store indices or internal
  proxy ids. Broadphase and query internals are deliberately not public
  contracts.
- Prefer `WorldRecipe` for reproducible setup and examples. Prefer direct
  `World` lifecycle APIs when a test needs to lock handle invalidation,
  mutation, or error behavior.
- Use `picea-lab` scene fixtures for authored compound/concave examples, but
  keep core solver expectations explicit: generated convex pieces are normal
  colliders by the time they reach `World`.
- Use deterministic counters and targeted tests before reading benchmark timing
  as a regression signal. The beta performance policy starts at baseline/warn
  mode, not hard-fail mode.

## Beta Verification Matrix

Before calling the beta surface ready in this workspace, run:

```bash
rtk proxy cargo test -p picea --lib
rtk proxy cargo test -p picea --tests
rtk proxy cargo test -p picea-lab
rtk proxy cargo test -p picea --examples --no-run
rtk proxy cargo bench -p picea --no-run
rtk proxy ruby -e 'require "yaml"; YAML.load_file("docs/ai/doc-catalog.yaml"); puts "yaml ok"'
rtk proxy git diff --check
```

For browser-facing changes, also run the web build/contract checks and verify
the workbench against a live local `picea-lab` server.
