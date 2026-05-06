# Stack Stability Repro Diagnostics

> Date: 2026-05-06
>
> Status: D2 design output for
> `docs/plans/2026-05-06-performance-stability-observability-milestones.md`.

This document fixes the first stack-stability reproduction window and
first-bad-frame marker policy. It is a diagnostics contract, not a solver
correctness test and not a physics fix.

## Scenario Roles

| Scenario | Role | Why |
| --- | --- | --- |
| `stack_4` | Primary clean repro | Four same-size dynamic boxes above a static floor. It has fewer fixture confounders and should be the first place to inspect stack drift, contact churn, sleep convergence, and solver row stability. |
| `stack_stability_tower` | Secondary stress repro | Dense tower on a narrow static support plus a side perturbation body. It is useful for proving the diagnostics surface can explain complicated failures, but it must not become a correctness oracle by itself. |

Do not add a new scenario in D2. If E1/V1 proves that both existing scenes are
too noisy for a future behavior lock, create a later milestone for a minimal
stable-stack fixture and a failing test.

## Frame Windows

Use deterministic fixed-step artifacts.

| Scenario | Frame count | Main window | Rolling window | Notes |
| --- | --- | --- | --- | --- |
| `stack_4` | 180 | `0..180` | 24 frames | Primary first-bad-frame source. |
| `stack_stability_tower` | 240 | `0..240` | 24 frames | Stress evidence only. |

The 24-frame rolling window matches the existing Web stack stability window.
Sleep convergence should be interpreted with the current sleep stability window
of 0.5 seconds in mind, but D2 does not turn that into a new test threshold.

## Marker Payload

Every first-bad-frame marker should carry:

- `kind`: stable marker id;
- `severity`: `info`, `warning`, or `severe`;
- `frame_index`;
- `score`;
- `threshold_name`;
- `source`: `rust_authoritative`, `lab_derived`, `web_derived`, or `missing`;
- `body_handles`, `contact_ids`, `island_ids` when available;
- `evidence_fields`: the fields used to compute the marker;
- `missing_evidence`: facts that would make the marker more precise.

This lets the Web UI highlight a frame while the copied debug context still
explains what was measured and what was missing.

## First-Bad-Frame Policy

For a selected scenario run, compute all markers in frame order. The
`first_bad_frame` is the earliest frame with a `warning` or `severe` marker. If
multiple markers happen on the same frame, keep all of them and choose the
highest severity as the frame severity.

Severity order for triage:

1. `numeric_warning`
2. `penetration_spike`
3. `sleep_never_converged`
4. `contact_churn_spike`
5. `warm_start_drop_spike`
6. `solver_row_spike`
7. `angular_drift_spike`
8. `body_drift_spike`
9. `performance_counter_spike`

The order is for debugging priority, not blame assignment.

## Marker Definitions

These are first-pass debug thresholds. They help find a frame worth inspecting.
They do not define solver correctness.

| Marker | Source | Warning | Severe | Attribution |
| --- | --- | --- | --- | --- |
| `numeric_warning` | `WorldEvent` / `StepStats.numeric_warnings` | Any numeric warning. | Any repeated numeric warning in the rolling window. | Event phase/detail when available. |
| `penetration_spike` | contacts / manifolds | `penetration_max > 0.02` or `penetration_sum > 0.08`. | `penetration_max > 0.08` or `penetration_sum > 0.25`. | Deepest contact ids, body handles, manifold ids. |
| `contact_churn_spike` | adjacent-frame contact ids | `enter + exit >= 3` in a frame. | `enter + exit >= 6` or repeated warnings in 24-frame window. | Entered/exited contact ids and involved bodies. |
| `warm_start_drop_spike` | warm-start counters / reasons | Any warm-start drop in a stack contact. | `warm_start_drop_count >= 2` in one frame or repeated drops in a rolling window. | Drop reasons, contact ids, bodies. |
| `solver_row_spike` | `StepStats.contact_row_count + joint_row_count` | Current row count exceeds previous rolling median by at least 50 percent and at least 4 rows. | Same condition repeated across 3 frames or paired with penetration/contact churn warning. | Island ids when available, row counts. |
| `sleep_never_converged` | body velocities, sleeping flags, sleep events | After frame 90 in `stack_4`, dynamic bodies stay awake while the rolling window is otherwise quiet. | Same condition after frame 120 or paired with contact/penetration warning. | Awake body handles, island ids, missing sleep blocker facts. |
| `angular_drift_spike` | body transforms over 24-frame window | Max angular drift `> 0.08` rad in the rolling window. | Max angular drift `> 0.25` rad or repeated warnings. | Body handle and drift value. |
| `body_drift_spike` | body transforms over 24-frame window | Max translation drift `> 0.08` world units in the rolling window. | Max translation drift `> 0.25` or repeated warnings. | Body handle and drift value. |
| `performance_counter_spike` | counter deltas | Broadphase/contact/solver/CCD counter exceeds rolling median by at least 50 percent and at least a small absolute delta. | Spike repeats or coincides with stability marker. | Counter name and frame delta. |

Threshold rationale:

- `0.02` penetration is four times the current residual correction slop
  (`0.005`), so it is useful as a warning without claiming correctness failure.
- `0.08` drift / angular drift keeps continuity with the existing Web marker
  threshold.
- Sleep markers wait until frame 90 in `stack_4` to avoid flagging ordinary
  early settling.

## Debug Vs Behavior Lock

Debug markers can later become behavior locks only after a separate milestone
adds a focused failing test. The first behavior-lock candidates are:

- a smaller stable-stack fixture that should keep bounded rotation and enter
  sleep;
- a sleep-specific test proving residual position correction does not keep
  otherwise quiet stacks awake forever;
- a warm-start continuity test across multi-body stack contacts.

The tower stress scenario should remain a stress/debug artifact unless a future
design milestone deliberately extracts a smaller correctness fixture from it.

## Solver Handoff Template

V1 should produce a handoff with this shape:

```markdown
## Stack Diagnostics Handoff

- Scenario:
- Run path:
- Frame window:
- First bad frame:
- Markers:
- Selected body/contact/island:
- Rust authoritative facts:
- Lab-derived diagnostics:
- Web-derived observations:
- Missing evidence:
- Suspected next failing test:
- Candidate implementation area:
- Explicit non-goals:
- Verification commands:
```

The handoff should identify the next test and likely code area, but it must not
claim that the stack has been fixed.

## Execution Inputs

### E1 Inputs

- Use `stack_4` for primary marker and first-bad-frame tests.
- Use `stack_stability_tower` for stress marker coverage.
- Serialize marker payloads with source labels and missing evidence.
- Do not add a new scenario unless D2 is revised.

### E3 Inputs

- UI should show `stack_4` as the clean repro path and tower as stress evidence.
- Existing Web drift / jitter / quiet-window display may remain, but source must
  be labeled.
- Timeline marker priority should follow the policy above.

### V1 Inputs

- Generate one `stack_4` run and one tower run if E1/E3 supports both.
- Report first bad frame for `stack_4` first.
- Use tower evidence only as stress context.
