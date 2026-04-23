# picea

`picea` is the core Rust crate for the Picea 2D physics engine.

It owns the engine runtime pieces:

- `math`: vectors, points, segments, matrices, axes, and numeric helpers.
- `world`: authoritative `World` state, store/runtime facts, handles, and public lifecycle APIs.
- `pipeline`: explicit simulation-step orchestration.
- `solver`: internal world-path solve helpers.
- `query` / `debug`: stable read-side APIs over world facts.

## Run Tests

```bash
cargo test -p picea --lib
```

Codex/agent sessions in this repository should prefix cargo commands with `rtk proxy`; see the root `AGENTS.md`.

## Development Notes

Use the repository root docs for milestone and AI-assisted development flow:

- `../../AGENTS.md`
- `../../docs/plans/2026-04-18-picea-physics-engine-milestones.md`
- `../../docs/ai/repo-map.md`
- `../../docs/ai/debug-playbook.md`

For code changes, keep the milestone boundary narrow and start with a behavior lock or focused regression test.
