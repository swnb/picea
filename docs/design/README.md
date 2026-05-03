# Picea Design Docs

This directory records design intent and future-facing engineering decisions.

## Documents

- `engine-design.md`: current engine design, goals, non-goals, component boundaries, and extension points.
- `physics-engine-upgrade-technical-plan.md`: current physics upgrade direction, algorithm choices, acceptance order, and landed slice.
- `debug-observability-design.md`: design for reproducible debug facts and stable read-side inspection.
- `picea-lab-observability-architecture.md`: current target design for artifacts, visualization, and benchmark evidence.
- `picea-lab-live-session-semantics.md`: M25-B live session reset/patch/transaction semantics and product upgrade path.
- `solver-island-ordering-contract.md`: M28 single-threaded island-local contact/joint ordering contract.
- `performance-threshold-policy.md`: M29 benchmark baseline, warn/fail, and fallback policy.
- `../public-beta.md`: M30 public beta surface, migration notes, examples, and final verification matrix.
- `architecture-refactor-requirements.md`: archived legacy `Scene`-path refactor requirements; not current default routing.

Design docs describe the intended direction. When implementing, still verify against current code and milestone gates.
