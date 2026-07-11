# Picea Design Docs

This directory records design intent and future-facing engineering decisions.

## Documents

- `engine-design.md`: current engine design, goals, non-goals, component boundaries, and extension points.
- `physics-engine-upgrade-technical-plan.md`: current physics upgrade direction, algorithm choices, acceptance order, and landed slice.
- `debug-observability-design.md`: design for reproducible debug facts and stable read-side inspection.
- `picea-lab-observability-architecture.md`: current target design for artifacts, visualization, and benchmark evidence.
- `performance-stability-diagnostics-contract.md`: D1 contract for performance and stack-stability diagnostics ownership, schema tiers, source labels, and UI mapping.
- `stack-stability-repro-diagnostics.md`: D2 contract for stack repro roles, frame windows, first-bad-frame markers, and solver handoff shape.
- `picea-lab-live-session-semantics.md`: M25-B live session reset/patch/transaction semantics and product upgrade path.
- `deformable-body-roadmap.md`: RFC boundary for future soft-body, particle, cloth, and deformable mesh work; distinguishes true deformable physics from rigid-body lattice proxies.
- `solver-island-ordering-contract.md`: M28 single-threaded island-local contact/joint ordering contract.
- `performance-threshold-policy.md`: M29 benchmark baseline, warn/fail, and fallback policy.
- `../public-beta.md`: M30 public beta surface, migration notes, examples, and final verification matrix.
- `matrix-stack-stability-optimization-design.md`: D1 design for matrix-stack stability optimization order and solver boundaries.
- `matrix-stack-stability-acceptance.md`: D2 acceptance gates for stack and matrix-stack stability evidence.
- `stack-4-contact-block-solve-stability-design.md`: design note for the narrow `stack_4` block-solve stability guard.
- `dense-pressure-position-row-architecture.md`: completed dense pressure / pseudo-position / position-row architecture record.
- `scene-runtime-config-and-parameter-ui-design.md`: scene-owned runtime config and parameter UI architecture.
- `2026-06-17-physics-realism-vnext-architecture.md`: vNext architecture package for seven physics-realism workstreams, including the scenario capability matrix for picea-lab showcase routing.
- `architecture-refactor-requirements.md`: archived legacy `Scene`-path refactor requirements; not current default routing.

Design docs describe the intended direction. When implementing, still verify against current code and milestone gates.
