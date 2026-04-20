# picea

`picea` is the core Rust crate for the Picea 2D physics engine.

It owns the engine runtime pieces:

- `scene`: fixed-step tick orchestration, callbacks, sleep/wakeup, and constraint pipeline flow.
- `element`: element identity, storage, handles, and data attachment.
- `math`: vectors, points, segments, matrices, axes, and numeric helpers.
- `shape`: circles, polygons, convex/concave geometry, projections, and transform sync.
- `collision`: broadphase/narrowphase entrypoints, AABB filtering, contact point pairs, and contact keys.
- `constraints`: contact, join, point constraints, warm start, lambda, velocity solve, and position correction.
- `meta`: mass, force, transform, inertia, and body metadata.
- `tools`: debug/helper tools that sit outside the core physics contract.

## Run Tests

```bash
cargo test -p picea --lib
```

Build examples:

```bash
cargo test -p picea --examples --no-run
```

Run an example:

```bash
cargo run -p picea --example ground
```

Codex/agent sessions in this repository should prefix cargo commands with `rtk proxy`; see the root `AGENTS.md`.

## Development Notes

Use the repository root docs for milestone and AI-assisted development flow:

- `../../AGENTS.md`
- `../../docs/plans/2026-04-18-picea-physics-engine-milestones.md`
- `../../docs/ai/repo-map.md`
- `../../docs/ai/debug-playbook.md`

For code changes, keep the milestone boundary narrow and start with a behavior lock or focused regression test.
