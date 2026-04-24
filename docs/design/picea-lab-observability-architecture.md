# Picea Lab Observability Architecture

> Date: 2026-04-24
>
> Status: active target design. No `picea-lab` crate, viewer, or benchmark harness exists in the current workspace yet.

Picea Lab is the proposed toolchain for visualization, reproducible artifacts, and performance measurement around the current `World` + `SimulationPipeline` core.

## Goal

The lab should answer questions that plain tests and screenshots cannot:

- Which broadphase candidates were generated?
- Which candidates survived narrowphase?
- Which contacts, normals, depths, and sleep transitions were produced?
- Did two deterministic runs diverge, and at which step?
- Did a physics change improve or regress a named benchmark scenario?

The core engine should expose facts. The lab should capture, render, compare, and summarize those facts.

## Boundary

`crates/picea` may expose:

- `StepReport`
- `StepStats`
- `WorldEvent`
- `DebugSnapshot`
- `DebugPrimitive`
- optional future artifact/counter structs

`crates/picea` must not own:

- windows or UI framework state
- camera controls
- screenshot approval
- benchmark interpretation
- viewer-specific layout
- browser or native app packaging

## Recommended First Slice

Start artifact-first, not UI-first.

1. Add a headless scenario runner outside the core hot path.
2. Run deterministic scenarios through `World` and `SimulationPipeline`.
3. Export readable artifacts under `target/picea-lab/runs/<run_id>/`.
4. Add a minimal viewer that reads artifacts and does not run physics.
5. Add Criterion benchmarks only after scenario names and counters are stable.

This lets visualization and benchmarks share the same scenario definitions.

## Artifact Set

| File | Purpose |
| --- | --- |
| `frames.jsonl` | One record per captured step, with step index, events, counters, and selected snapshot facts. |
| `debug_render.json` | Shape outlines, AABBs, contact points, normals, sleep labels, and optional broadphase candidate lines. |
| `final_snapshot.json` | Final `DebugSnapshot` for deterministic comparison. |
| `perf.json` | Scenario metadata, counters, timing summary, and state hash. |
| `trace.perfetto.json` | Optional timeline export for Perfetto or Chrome trace viewers. |

JSON/JSONL should be canonical for the first slice because it is easy to diff, attach to bug reports, and inspect in code review. Binary replay can come later if schemas stabilize.

## Viewer Choice

The first viewer should be small and artifact-only:

- draw shapes and AABBs;
- draw contact points and normals;
- show sleeping bodies differently;
- expose a step/timeline selector;
- show counters from `StepStats` and future broadphase/narrowphase counters.

A static HTML Canvas viewer is the fastest first step. It avoids adding a heavy UI dependency before the artifact model is proven.

An `egui` / `eframe` native viewer is still a good second step once artifact schemas and scenario names are stable. It is better for inspectors, filters, and richer interaction, but it should not be the first dependency added just to see shapes.

## Benchmark Plan

Use Criterion for wall-clock benchmark claims and keep lab counters as domain context.

Initial benchmark groups:

- `broadphase/sparse_64`
- `broadphase/dense_64`
- `step/circles_64`
- `contacts/resting_overlap`
- `sleep/quiet_bodies`
- `create/batch_1000`

Future groups:

- `narrowphase/polygon_sat`
- `solver/stack_16`
- `ccd/fast_circle_wall`

Do not invent absolute thresholds before the first local baseline. Once baselines exist, any unexplained regression over 5% in a relevant scenario needs investigation or explicit acceptance.

## Metrics

Minimum counters:

- step time
- body count
- collider count
- broadphase candidate count
- narrowphase contact count
- manifold count
- sleep transition count
- max penetration
- deterministic state hash

Later counters:

- candidate reject reason counts
- contact feature-key transfers and drops
- solver iterations used
- normal/tangent impulse ranges
- CCD time-of-impact count

## Target Architecture

```mermaid
flowchart TD
    Scenario["Scenario definition"] --> Runner["Headless runner"]
    Runner --> World["picea::World"]
    Runner --> Pipeline["SimulationPipeline"]
    Pipeline --> Report["StepReport"]
    World --> Snapshot["DebugSnapshot"]
    Report --> Capture["Artifact capture"]
    Snapshot --> Capture
    Capture --> Frames["frames.jsonl"]
    Capture --> Render["debug_render.json"]
    Capture --> Final["final_snapshot.json"]
    Capture --> Perf["perf.json"]
    Frames --> Viewer["Artifact viewer"]
    Render --> Viewer
    Perf --> Bench["Criterion summary / regression notes"]
```

## Acceptance

L1 artifact capture:

- capture disabled has no behavior change;
- one deterministic contact scenario writes all required files;
- artifact schema has tests;
- state hash is stable for identical input.

L2 static viewer:

- opens saved artifacts without running physics;
- renders colliders, AABBs, contacts, normals, and sleep state;
- has at least one saved fixture test or screenshot check.

L3 benchmark baseline:

- `cargo bench` discovers named scenarios;
- benchmark output records Criterion results and Picea counters;
- broadphase candidate count is wired into stats or artifact counters.

## Non-Goals

- No live editor in the first slice.
- No screenshot-only correctness gate.
- No always-on tracing in the core hot path.
- No UI dependency inside `crates/picea`.
- No browser or native packaging before artifact capture is stable.
