# Legacy Scene Runtime Archive

> Status: deprecated archive. This document merges the former
> `runtime-pipeline.md` and `collision-constraints.md` notes from the removed
> `Scene` / `Context` engine path. Current work should start from
> `crates/picea/src/lib.rs`, `World`, `SimulationPipeline`, and
> `docs/architecture/system-overview.md`.

This file is retained only for historical design context. It must not be used as
the default runtime, validation, or module-routing source for new work.

## Historical Runtime Shape

The old runtime centered on `Scene::tick(delta_time)`. It accumulated arbitrary
frame deltas, advanced fixed substeps, and drove a `step_frame(fixed_dt)` loop.

Historical stage order:

1. integrate velocity
2. detect collisions
3. warm-start old contacts
4. refresh contact points
5. pre-solve constraints
6. solve velocity/friction constraints
7. integrate positions
8. solve position corrections
9. apply sleep
10. sync transforms and post-solve

The current runtime entrypoint is instead `SimulationPipeline::step(&mut World,
StepConfig)`. Do not infer current step ordering from the list above without
checking `crates/picea/src/pipeline/*`.

## Historical Collision / Constraint Shape

The old path split collision and constraints roughly as:

- `collision` produced potential and accurate contact pairs.
- `constraints` stored solver-facing contact state.
- `Scene` orchestrated contact-manifold lifecycle and solver phases.

Important historical ideas that still explain some archived design decisions:

- conservative contact-key transfer is safer than inheriting the wrong cached
  impulse;
- re-contact after an inactive pass should not inherit pre-separation lambda;
- broadphase, narrowphase, manifold lifecycle, and solver rows are separate
  concerns even when a runtime implementation wires them together.

Current ownership is different. Use:

- `docs/ai/repo-map.md` for current module ownership;
- `docs/design/engine-design.md` for current design invariants;
- `docs/design/physics-engine-upgrade-technical-plan.md` for current upgrade
  direction;
- `crates/picea/src/pipeline/contacts.rs`,
  `crates/picea/src/pipeline/narrowphase.rs`, and
  `crates/picea/src/solver/contact.rs` for live implementation facts.

## Safe Use

Use this archive only when you are investigating why older plans mention
`Scene`, `Context`, `picea-web`, wasm gates, manifold lifecycle shims, or old
constraint terminology. For all current implementation, review, and validation
tasks, treat current code and fresh command output as higher authority.
