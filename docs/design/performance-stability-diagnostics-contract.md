# Performance And Stack Stability Diagnostics Contract

> Date: 2026-05-06
>
> Status: D1 design output for
> `docs/plans/2026-05-06-performance-stability-observability-milestones.md`.

This document defines the first diagnostics contract for performance and stack
stability investigation. It does not implement new fields. It decides ownership,
schema tiers, missing-evidence behavior, and the UI mapping that later
milestones must preserve.

## Goal

The diagnostics surface should answer two questions before any solver fix:

- Performance: did this frame or run get expensive because deterministic work
  increased, because artifact / UI processing got expensive, or because local
  wall-clock timing is noisy?
- Stability: which frame, body, contact, island, or sleep transition first
  explains a stacked-body failure?

The core engine exposes facts. `picea-lab` captures and summarizes those facts.
`picea-lab-web` displays them and may derive UI-only aggregates, but it must not
pretend those aggregates are solver-owned facts.

## Ownership

| Tier | Owner | Allowed contents | Not allowed |
| --- | --- | --- | --- |
| Core authoritative facts | `crates/picea` | Stable counters, events, debug snapshots, handles, reason enums, read-only facts that come from the simulation step. | UI layout, artifact paths, browser state, benchmark interpretation, wall-clock pass/fail policy. |
| Lab-owned diagnostics | `crates/picea-lab` | Per-frame / per-run summaries derived from `StepReport`, `DebugSnapshot`, `WorldEvent`, and artifact frame sequences. | Recomputed physics, changed solver behavior, hidden assumptions presented as core facts. |
| Web-derived observations | `crates/picea-lab/web` | UI window aggregates, marker selection, copy context, visual ranking, missing evidence display. | New physics truth, broadphase/island/contact reconstruction that is not exported by Rust. |

## Current Authoritative Inputs

The first implementation should reuse these facts before adding anything:

| Source | Existing facts | First use |
| --- | --- | --- |
| `StepStats` | body/collider/joint counts, broadphase candidate/traversal/prune/depth, contact/manifold counts, island counts, solver row counts, warm-start counts, CCD counts, sleep transition count, numeric warnings. | Performance counters, solver work spikes, sleep convergence context. |
| `WorldEvent` | contact lifecycle facts, contact ids, body/collider handles, contact depth, warm-start reason, solver impulses, sleep/wake reason, numeric warnings. | Contact churn, impulse range, warm-start explanation, sleep/wake timeline. |
| `DebugSnapshot` | body transforms/velocities/sleeping/island id, contacts, manifolds, islands, broadphase tree, debug stats. | Stability windows, body drift, angular drift, penetration summary, island membership. |
| `FrameRecord` | `report`, `stats`, `events`, `snapshot`, provenance, state hash. | Frame diagnostics and copy-debug-context source of truth. |
| `PerfArtifact` | elapsed micros plus run-level counter summary. | Run-level performance summary, not a correctness oracle. |

## Schema Tiers

### Always-On Cheap Counters

Use existing `StepStats` and `DebugStats` wherever possible. These are the
lowest-risk performance facts because they are already carried by the step and
artifact path.

Allowed examples:

- broadphase candidate / traversal / prune counts;
- contact and manifold counts;
- active / sleeping island counts;
- solver body slots and contact / joint rows;
- warm-start hit / miss / drop counts;
- CCD candidate / hit / miss / clamp counts;
- sleep transition and numeric warning counts.

### Lab-Owned Frame Diagnostics

E1 should prefer a lab-owned additive carrier, tentatively named
`FrameDiagnostics`, built from exported frame facts. This keeps the first slice
out of the core hot path.

Required source tags:

- `rust_authoritative`: direct Rust facts copied from `StepStats`,
  `DebugSnapshot`, or `WorldEvent`.
- `lab_derived`: lab summary derived from Rust facts or a frame sequence.
- `web_derived`: UI-only observation; Web may display it, but it should not be
  serialized as authoritative Rust evidence.
- `missing`: field was not present in the artifact or was not computable.

Recommended first fields:

| Group | Field | Source | Missing behavior |
| --- | --- | --- | --- |
| Performance | `counter_delta` for candidate, traversal, prune, contact, row, island, warm-start, CCD counters | `lab_derived` from adjacent frames | Show missing when previous frame is absent. |
| Performance | `performance_markers` such as `counter_spike`, `solver_row_spike`, `ccd_spike`, `numeric_warning` | `lab_derived` from frame/run counters | No marker means no detected marker, not zero cost. |
| Stability | `penetration_max`, `penetration_sum`, `penetrating_contact_count` | `lab_derived` from contacts / manifolds | Missing if contacts are absent from artifact. |
| Stability | `contact_churn` with enter / persist / exit counts by contact id | `lab_derived` from adjacent frames | Missing on first frame or when contact ids are absent. |
| Stability | `warm_start_summary` hit / miss / drop plus top drop reasons | `rust_authoritative` counters and event reasons, summarized by lab | Missing if event reasons are absent. |
| Stability | `impulse_summary` normal / tangent totals and max values | `lab_derived` from contact events / snapshot contacts | Missing if solver impulse facts are absent. |
| Stability | `sleep_summary` awake / sleeping dynamic bodies, transition count, reason counts | `rust_authoritative` plus lab summary | Missing only for reason counts when events are absent. |
| Stability | `island_summary` active / sleeping islands, body slots, row counts | `rust_authoritative` counters and `DebugIsland` | Missing if island facts are absent. |
| Stability | `stability_markers` such as `penetration_spike`, `contact_churn_spike`, `warm_start_drop_spike`, `sleep_never_converged`, `angular_drift_spike` | `lab_derived` | Marker thresholds come from D2. |

### Conditional Core Read-Model Carrier

E2 should run only when D1/E1 prove that lab-owned diagnostics cannot explain a
key gap. The first likely gaps are not general tracing; they are narrow facts
that current events do not expose.

Allowed conditional candidates:

| Candidate | Why lab may need it | Owner if approved |
| --- | --- | --- |
| `position_correction_count` / `position_correction_max_translation` / `position_correction_sum_translation` | Lab can see penetration and drift, but not whether residual position correction moved bodies in the step. | Core read model or step stats. |
| `sleep_idle_reset_count_by_reason` | Lab can see bodies fail to sleep, but cannot reliably know whether idle time was reset by contact correction, velocity edit, or another phase. | Core sleep / step facts. |
| `sleep_blocker_reason_counts` | Useful when a stack remains awake despite low visible velocity. | Core sleep facts. |
| `phase_timing` | Useful only after deterministic counters fail to explain performance. | Prefer lab runner timing; core timing only as opt-in debug carrier. |

Not allowed in E2:

- changing contact response, sleep policy, or solver row math;
- exposing private storage ids;
- adding a generic always-on tracing framework;
- using wall-clock timing as correctness evidence.

## Missing Evidence Semantics

Consumers must distinguish:

- zero: the fact was measured and the value is zero;
- missing: the fact was not exported or cannot be derived safely;
- not applicable: the scenario/frame does not exercise that fact;
- web-derived: the value is useful for UI inspection but is not authoritative.

Web panels must show missing evidence explicitly instead of rendering missing
numeric fields as `0`.

## UI Mapping

### Performance Panel

The panel should show:

- run-level `PerfArtifact.counter_summary`;
- frame-level `counter_delta` when available;
- top markers for solver row spikes, broadphase counter spikes, CCD spikes, and
  numeric warnings;
- source labels: `Rust stats`, `Lab derived`, `Missing`, or `Web derived`;
- elapsed wall-clock only as informational evidence.

It should not show a hard pass/fail performance badge unless a future threshold
policy explicitly approves it.

### Stability Panel

The panel should show:

- penetration max / sum and penetrating contact count;
- contact churn enter / persist / exit;
- warm-start hit / miss / drop summary;
- normal / tangent impulse totals and max values;
- awake / sleeping body counts, sleep transition reasons, island counts;
- first-bad-frame markers from D2.

If Web keeps existing window metrics such as body drift, angular drift, jitter
proxy, or quiet window, it must label them as `web_derived` unless E1/E2 adds a
Rust/lab-owned equivalent.

### Timeline Markers

Markers should be derived from diagnostics facts, not from visual impression:

- `counter_spike`;
- `solver_row_spike`;
- `penetration_spike`;
- `contact_churn_spike`;
- `warm_start_drop_spike`;
- `sleep_transition`;
- `sleep_never_converged`;
- `angular_drift_spike`;
- `numeric_warning`.

D2 owns thresholds and the selected stack frame window. E1/E3 should consume
those decisions rather than inventing thresholds in implementation.

### Copy Debug Context

The copied context should include:

- scenario id/name and source mode;
- run id or session id;
- frame index and D2 window;
- state hash;
- selected body/collider/contact/island when any;
- performance diagnostics with source labels;
- stability diagnostics with source labels;
- first-bad-frame marker and threshold name;
- missing evidence summary;
- recommended verification commands when available.

## Design Decisions

1. Start with lab-owned diagnostics.
   This reuses existing authoritative facts and avoids putting new observation
   points inside solver internals before we know the precise gap.

2. Keep E2 conditional.
   Core read-model additions are appropriate only when a required fact cannot
   be reconstructed safely from exported `StepReport`, `DebugSnapshot`, or
   `WorldEvent` data.

3. Treat wall-clock as explanatory, not decisive.
   Deterministic counters and repeated benchmark baselines remain the
   performance evidence path. A single local run may guide investigation, but
   it should not fail correctness work.

4. Preserve the M35/M40 Web surface.
   Existing stack panels, markers, and copy context should be extended rather
   than replaced. The product surface stays familiar while the evidence gets
   sharper.

## Execution Inputs

### E1 Inputs

- Add a lab-owned diagnostics carrier only.
- Derive it from existing `FrameRecord` / `DebugSnapshot` / `WorldEvent` facts.
- Add source tags and missing evidence semantics.
- Add artifact schema tests for old/missing fields.
- Do not touch `crates/picea/src/*`.

### E2 Inputs

- Execute only if E1 records a concrete missing authoritative fact.
- Keep the field set minimal and additive.
- Prefer `StepStats`, `DebugStats`, `DebugSnapshot`, or `WorldEvent`.
- Do not change physics behavior.

### E3 Inputs

- Reuse existing Web panels and copy context.
- Show lab/core facts before Web-derived fallback.
- Label every value source.
- Missing evidence must stay visible.

### V1 Inputs

- Run at least one stack scenario selected by D2.
- Confirm artifacts and browser UI expose the same diagnostic story.
- Produce a solver handoff, not a solver fix.
