# picea

`picea` is the core Rust crate for the Picea 2D physics engine.

It owns the public beta engine runtime pieces:

- `math`: vectors, points, segments, matrices, axes, and numeric helpers.
- `world`: authoritative `World` state, store/runtime facts, handles, and public lifecycle APIs.
- `pipeline`: explicit simulation-step orchestration.
- `solver`: internal world-path solve helpers.
- `query` / `debug`: stable read-side APIs over world facts.
- `recipe`: declarative setup through bundles, commands, and world recipes.

## Public Beta Flow

The intended beta entry point is `picea::prelude::*`.

Use `World` and descriptors for authoritative state:

- `World::new(WorldDesc::default())`
- `World::create_body(BodyDesc { ... })`
- `World::create_collider(body, ColliderDesc { ... })`

Use `SimulationPipeline::step` for fixed-step simulation. The returned
`StepReport` contains the step index, world revision, deterministic counters,
and ordered world events.

Use `DebugSnapshot` when a stable read model is needed, and `QueryPipeline` when
point/ray/AABB/shape queries should be run against an owned query cache. The
query cache never exposes internal broadphase proxy ids as public contract.

Use `WorldRecipe`, `BodyBundle`, `ColliderBundle`, and `JointBundle` for
repeatable examples, fixtures, and benchmarks.

Checked example:

```bash
rtk proxy cargo test -p picea --examples --no-run
```

The example source is `examples/public_beta_smoke.rs`.

## Run Tests

```bash
rtk proxy cargo test -p picea --lib
```

Codex/agent sessions in this repository should prefix cargo commands with `rtk proxy`; see the root `AGENTS.md`.

## Development Notes

Use the repository root docs for milestone and AI-assisted development flow:

- `../../AGENTS.md`
- `../../docs/public-beta.md`
- `../../docs/plans/2026-04-25-picea-physics-engine-production-milestones.md`
- `../../docs/ai/repo-map.md`
- `../../docs/ai/debug-playbook.md`

For code changes, keep the milestone boundary narrow and start with a behavior lock or focused regression test.
