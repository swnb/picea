# Solver Island Ordering Contract

M28 fixes the current single-threaded solver ordering contract. It does not
introduce a parallel solver and does not merge contact and joint rows into one
interleaved row stream.

## Contract

- Solver islands are built from dynamic body connectivity and are ordered by the
  deterministic island list produced by the sleep/island pipeline.
- Each active island owns a dense `body_slots` array for the current step.
  Public handles remain the external identity; dense slots are temporary row
  indices.
- Contact rows and joint rows share the same `body_slots`, so debug counters can
  explain all hot rows through one island-local body set.
- Contact rows keep contact-gathering order inside their island.
- Joint rows keep world joint iteration order inside their island.
- Contact rows and joint rows remain separate phase vectors. Current behavior is
  intentionally contact-solve first, joint-solve second, rather than a unified
  island row stream.
- Sleeping islands do not allocate hot contact or joint rows.
- Wake and sleep events stay in the step event stream; debug islands summarize
  the latest reason but do not invent solver ordering facts.

## Why This Contract

The current separate-phase solver is easier to validate against existing stack,
sleep, warm-start, and joint tests. A unified row stream may be useful before a
future parallel island solver, but it would change physics outcomes and needs a
dedicated rollback plan. M28 therefore documents and locks the current contract
instead of changing it quietly.

## Non-Goals

- No multithreaded solver.
- No contact manifold ordering changes.
- No public joint API changes.
- No broad material-system redesign.
- No claim that contact and joint impulses are solved in a single unified order.

