# Picea Physics Engine Production Milestones

> Date: 2026-04-25
>
> Status: active execution plan.
>
> Scope: this plan turns `todo.md` into milestone-sized execution batches for the
> current `World` + `SimulationPipeline` engine path. It is a planning document
> only; concrete implementation must still start from current repo facts, add a
> focused behavior lock first, and pass the milestone gate before moving on.

## Positioning

The original task list is too broad to land as one milestone. Broadphase productionization,
SAT manifolds, persistent contacts, mass/inertia, sequential impulses, island
sleep, CCD, API recipes, observability, and benchmarks depend on each other in a
specific order. The execution rule is therefore:

1. lock one behavior slice;
2. make the smallest production change for that slice;
3. expose enough facts to explain the result;
4. run the targeted gate and the milestone gate;
5. do not start the next milestone until the current one is accepted.

`picea-lab` is an evidence layer, not the core test oracle. Core correctness must
live in `crates/picea` unit, integration, and deterministic scenario tests.
`picea-lab` should make those results explainable through artifacts, snapshots,
debug render data, and later benchmark summaries.

## Common Acceptance Rules

Each implementation milestone must include:

- a failing behavior lock or a targeted known-red test before production edits;
- a narrow implementation that does not mix future milestone work;
- a `StepReport`, `DebugSnapshot`, event, or artifact fact when the behavior is
  hard to understand from final positions alone;
- a short residual-risk note when the milestone intentionally stops before the
  full production target;
- verification through `rtk proxy` commands.

Default gates, adjusted per milestone:

```bash
rtk proxy cargo fmt --all --check
rtk proxy cargo test -p picea --lib
rtk proxy cargo test -p picea --tests
rtk proxy cargo test -p picea-lab
rtk proxy ruby -e 'require "yaml"; YAML.load_file("docs/ai/doc-catalog.yaml"); puts "yaml ok"'
rtk proxy git diff --check
```

Use narrower targeted commands during red/green loops. Run the broader gates
before acceptance when the changed surface touches shared behavior.

## M1 Broadphase Productionization

> Status: completed 2026-04-25.
>
> Completion notes: `World` now owns persistent broadphase state; the broadphase
> maintains fat AABB proxies, handles move/reinsert/remove/stale cleanup,
> compacts tombstones, and rebuilds deterministically when tree depth exceeds
> the balance budget. `StepStats`, `DebugSnapshot`, and `picea-lab` artifacts
> now expose candidate count, update count, stale proxy drops, candidate drop
> reason counts, rebuild count, and tree depth. The lab includes a
> `broadphase_sparse` scenario for artifact evidence.

### Goal

Turn the current per-step rebuilt dynamic AABB tree into a production broadphase
with persistent collider proxies, fat AABBs, incremental updates, deterministic
candidate order, and useful debug facts.

### In Scope

- Store a broadphase proxy id for each live collider.
- Add persistent proxy insert, move, remove, and reinsert paths.
- Add fat AABB expansion so small movements do not force tree updates.
- Add tree balance, rebuild strategy, and tree-depth metrics.
- Reuse the tree for ray cast, AABB query, and region query when those query
  semantics are clear.
- Expose candidate count, drop reasons, update count, and tree depth through
  step/debug facts.

### Out Of Scope

- Criterion performance thresholds.
- SAT, clipping, contact manifold, solver, CCD, or API recipe changes.
- Replacing the public `World` API just to host broadphase state.

### Acceptance Method

- Add the smallest candidate-count behavior test first. It should prove that a
  sparse scene does not degenerate into all-pairs candidates and that candidate
  ordering is deterministic.
- Add lifecycle tests for proxy move, remove, reinsert, and stale proxy cleanup.
- Add at least one debug/snapshot assertion for candidate count and tree depth.
- Use `picea-lab` only as artifact evidence in this milestone: one named
  broadphase scenario should capture candidate count and tree depth. Do not add
  Criterion here.

Suggested targeted gates:

```bash
rtk proxy cargo test -p picea --lib pipeline::broadphase
rtk proxy cargo test -p picea --test query_debug_contract
rtk proxy cargo test -p picea-lab
```

Verified completion gates:

```bash
rtk proxy cargo fmt --all --check
rtk proxy cargo test -p picea --lib
rtk proxy cargo test -p picea --tests
rtk proxy cargo test -p picea-lab
rtk proxy ruby -e 'require "yaml"; YAML.load_file("docs/ai/doc-catalog.yaml"); puts "yaml ok"'
rtk proxy git diff --check
```

## M2 SAT And Clipping Manifold

> Status: completed 2026-04-26.
>
> Completion notes: convex rectangle / regular polygon / convex polygon pairs
> now use SAT minimum-penetration axes, deterministic reference/incident edge
> selection, clipping, duplicate point reduction, and stable per-point
> `ContactFeatureId`s. Circle-circle, circle-polygon, and circle-segment paths
> use analytic contacts. Contact events/debug snapshots expose manifold points,
> feature ids, normals, depths, and reduction reasons; contact identity is now
> collider pair + feature id while one `ManifoldId` is shared per collider pair.
> Concave polygons remain outside M2 and use an explicit `non_m2_fallback`
> residual-risk path rather than entering convex SAT.

### Goal

Make convex contact generation real: polygon and rectangle pairs should produce
a stable 1-2 point manifold instead of falling back to single-point AABB overlap.

### In Scope

- Implement SAT minimum penetration axis for rectangles and convex polygons.
- Select reference and incident edges.
- Clip incident edges to produce a 1-2 point contact manifold.
- Generate stable feature ids for contact points.
- Add conservative contact reduction for duplicate or jittery points.
- Add analytic circle-polygon and circle-segment narrowphase.

### Out Of Scope

- Generic GJK/EPA fallback.
- Solver impulse caching beyond the feature ids needed by the manifold.
- CCD time-of-impact behavior.

### Acceptance Method

- Add deterministic tests for polygon contact normal, penetration depth, contact
  point count, and feature-id stability under small movements.
- Add rotated rectangle and convex polygon cases; avoid accepting an AABB-only
  false positive as a contact.
- Add contact-reduction tests for near-duplicate clipping output.
- Add debug facts for manifold points, feature ids, normal, and reduction reason.
- Use `picea-lab` to render manifold points and normals for one saved scenario,
  but keep pass/fail assertions in `crates/picea`.

Suggested targeted gates:

```bash
rtk proxy cargo test -p picea --lib pipeline::narrowphase
rtk proxy cargo test -p picea --test physics_realism_acceptance sat_polygon
rtk proxy cargo test -p picea --test query_debug_contract
```

## M3 Mass And Inertia Model

### Goal

Define physical mass properties before the full solver depends on them. Dynamic
bodies need mass, center of mass, inverse mass, inertia, and inverse inertia with
clear static, kinematic, and dynamic semantics.

### In Scope

- Compute mass from collider shape density.
- Compute local center of mass for supported shapes.
- Compute moment of inertia for circles, rectangles, segments where applicable,
  and convex polygons.
- Define inverse mass and inverse inertia semantics for static, kinematic, and
  dynamic bodies.
- Validate illegal mass, density, and inertia inputs.
- Explain density, mass, center of mass, and inertia in code comments where the
  formula or domain term is not obvious.

### Out Of Scope

- Sequential impulse solving.
- Warm-start impulse transfer.
- CCD and island sleep.

### Acceptance Method

- Add formula-level tests for each supported shape.
- Add world-level tests proving static and kinematic bodies have zero inverse
  mass/inertia while dynamic bodies use density-derived values.
- Add validation tests for negative, non-finite, and degenerate mass inputs.
- Add debug snapshot facts only if needed to inspect mass properties; avoid
  expanding the public API more than necessary.

Suggested targeted gates:

```bash
rtk proxy cargo test -p picea --lib collider
rtk proxy cargo test -p picea --test core_model_world
rtk proxy cargo test -p picea --test physics_realism_acceptance
```

### Completion Notes

- Completed in the current workspace at `HEAD=6a8c3e1` without starting M4.
- Added public `MassProperties` facts on `BodyView` and debug snapshots, with
  `picea-lab` artifact and web inspector propagation.
- Collider density now contributes to mass for circles, rectangles, segments,
  regular polygons, convex polygons, and simple concave polygon loops; sensors
  still contribute mass because sensor status only controls contact response.
- Static and kinematic bodies retain density-derived `mass` / `inertia` facts
  but expose zero `inverse_mass` / `inverse_inertia`; dynamic bodies expose
  positive inverses only when derived values are positive.
- At M3 completion, the interim contact velocity response read body inverse
  mass from `MassProperties`; angular response, effective mass rows,
  warm-starting, and sequential impulses were intentionally deferred to later
  solver milestones.
- Review follow-up closed the mutation-order edge cases: derived non-finite
  mass facts are rejected before collider create, patch, or destroy mutates
  authoritative world state.

Residual risks:

- Polygon mass formulas assume a simple, non-self-intersecting loop; there is
  no full polygon self-intersection validator yet.
- Inertia is computed and exported but not consumed by angular contact solving
  until later solver milestones.

Verified gates:

```bash
rtk proxy cargo fmt --all --check
rtk proxy cargo test -p picea --lib
rtk proxy cargo test -p picea --tests
rtk proxy cargo test -p picea-lab
rtk proxy cargo test -p picea --test physics_realism_acceptance
cd crates/picea-lab/web && rtk proxy npm run build
rtk proxy ruby -e 'require "yaml"; YAML.load_file("docs/ai/doc-catalog.yaml"); puts "yaml ok"'
rtk proxy git diff --check
```

## M4 Persistent Contact And Warm-Start Cache

> Status: completed 2026-04-26.
>
> Completion notes: contact records now retain per-point warm-start cache facts
> keyed by normalized collider pair plus `ContactFeatureId`. The pipeline
> transfers previous normal/tangent impulses only when feature id, normal
> orientation, and collider-relative contact anchors remain trustworthy. Re-contact
> after separation, A/B ordering, feature drift, normal flip, point drift,
> solid/sensor transitions, and older serde payload defaults are covered by core
> tests. `StepStats`, `DebugSnapshot`, and `picea-lab` artifacts expose
> warm-start hit/miss/drop counts and per-contact reasons for a stable manifold
> scenario.
>
> Residual risk: the exported impulse values are warm-start cache transfer facts
> only. Full sequential impulse application, angular contact rows, effective mass
> solve, and solver-owned normal/tangent impulse ranges remain M5 work.

### Goal

Persist contact manifolds across frames and transfer cached normal/tangent
impulses only when the contact identity is trustworthy.

### In Scope

- Establish stable contact keys and per-point feature keys.
- Persist manifold cache across steps.
- Transfer previous normal and tangent impulses when feature ids and normal
  orientation match.
- Handle A/B exchange, normal flip, and feature drift.
- Drop stale cache conservatively when points drift too far or identity becomes
  ambiguous.
- Expose warm-start hit, miss, and drop reasons.

### Out Of Scope

- Full sequential impulse solver math beyond storing and transferring cache
  values.
- Island sleep.
- CCD.

### Acceptance Method

- Add tests for continuing contacts, re-contact after separation, A/B swap,
  normal flip, feature drift, and conservative cache drop.
- Assert both final behavior and debug facts: hit/miss/drop reason matters here.
- Use `picea-lab` artifacts to show per-step manifold identity and warm-start
  eligibility for one stable-contact scenario.

Suggested targeted gates:

```bash
rtk proxy cargo test -p picea --lib pipeline::contacts
rtk proxy cargo test -p picea --test physics_realism_acceptance
rtk proxy cargo test -p picea-lab
```

## M5 Sequential Impulse Solver

> Status: completed 2026-04-26.
>
> Completion notes: contact resolution now builds solver rows for every
> non-sensor contact point instead of keeping only the deepest point per collider
> pair. The velocity solve uses per-row effective mass with inverse mass and
> inverse inertia, applies trusted warm-start impulses before configured velocity
> iterations, clamps accumulated normal impulses to be non-negative, clamps
> tangent impulses by the Coulomb friction budget, and applies restitution only
> when closing speed exceeds `StepConfig::restitution_velocity_threshold`.
> Residual position correction is driven by configured position iterations and
> no longer overwrites solved linear or angular velocities. `ContactEvent`,
> `DebugSnapshot`, and `picea-lab` artifacts expose solver normal/tangent
> impulse facts, clamp state, restitution threshold decisions, and warm-start
> usage; the lab stack artifact keeps contact impulses measured for inspection.
>
> Residual risk: this is still a single-world contact solver without island
> compaction or island-level sleeping. CCD, generic convex fallback, benchmark
> thresholds, and island wake/sleep semantics remain later milestones.

### Goal

Replace the temporary contact velocity response with an iterative sequential
impulse solver that uses effective mass, inverse inertia, warm-starting,
Coulomb friction, restitution threshold, velocity iterations, position
iterations, and angular contact response.

### In Scope

- Compute effective mass for each contact row.
- Warm-start cached normal and tangent impulses before velocity iterations.
- Solve normal impulse with non-negative clamping.
- Solve tangent impulse with Coulomb friction clamped by normal impulse.
- Apply restitution only above a configurable threshold.
- Run velocity iterations and position iterations from `StepConfig`.
- Add angular contact response.
- Keep residual position correction, but ensure it does not overwrite velocity
  solve results.

### Out Of Scope

- CCD.
- Island sleep graph compaction.
- Benchmark thresholds.

### Acceptance Method

- Add deterministic scenario tests for stack stability, ramp friction, elastic
  restitution threshold, angular response, and low-speed no-bounce behavior.
- Assert impulse facts: normal/tangent impulse range, clamp state, restitution
  threshold decision, and warm-start usage.
- Compare behavior against broad physical expectations, not frame-perfect
  Box2D/Rapier output.
- Use `picea-lab` artifacts to inspect stack contact points, normals, impulses,
  and sleep state, but keep pass/fail in core tests.

Suggested targeted gates:

```bash
rtk proxy cargo test -p picea --lib pipeline::contacts
rtk proxy cargo test -p picea --test physics_realism_acceptance
rtk proxy cargo test -p picea --test world_step_review_regressions
```

## M6 Island Sleep And Wake Reasons

### Goal

Move from body-local sleep checks to island-level sleeping and explicit wake
reason reporting.

### In Scope

- Build contact/joint islands from the current graph.
- Sleep an island only when all eligible bodies are stable for the required
  window.
- Wake islands on contact impulse, joint correction, user patch, transform edit,
  velocity edit, and impact.
- Report sleep and wake reasons in events/debug facts.
- Add resting stack sleep and wake-on-impact regressions.

### Out Of Scope

- CCD time-of-impact.
- Active-island compact arrays for performance.
- API recipes.

### Acceptance Method

- Add tests for stack sleeps as one island, impact wakes the island, user edits
  wake the correct island, and unrelated sleeping islands stay asleep.
- Assert event ordering and wake reason payloads.
- Use `picea-lab` artifacts to label sleeping islands and wake reasons.

Suggested targeted gates:

```bash
rtk proxy cargo test -p picea --lib pipeline::sleep
rtk proxy cargo test -p picea --test physics_realism_acceptance sleep
rtk proxy cargo test -p picea --test world_step_review_regressions
```

## M7 GJK, EPA, And Generic Convex Fallback

### Goal

Add a generic convex kernel for shapes that are not covered by the specialized
2D SAT/analytic paths, and prepare the distance-query and CCD fallback route.

### In Scope

- Add support mapping for convex shapes.
- Implement GJK distance and intersection.
- Implement EPA or a closest-feature fallback for penetration information.
- Use the generic convex path only when specialized SAT or analytic narrowphase
  is not available or not appropriate.
- Reuse the kernel for distance queries and the later CCD fallback path.
- Add debug facts for simplex evolution, termination reason, fallback choice,
  and failure containment when a degenerate shape cannot produce a stable answer.

### Out Of Scope

- Replacing SAT as the primary rectangle/convex-polygon manifold path.
- Solving concave contacts directly.
- Full CCD event semantics.
- Benchmark thresholds.

### Acceptance Method

- Add unit tests for support mapping on circles, rectangles, convex polygons,
  and segments where applicable.
- Add GJK distance/intersection tests for separated, touching, overlapping, and
  degenerate convex inputs.
- Add EPA or closest-feature fallback tests that prove non-finite or ambiguous
  input is contained without panics or NaNs.
- Add one distance-query or generic fallback scenario, but do not make it the
  default polygon manifold path when SAT is available.
- Use `picea-lab` artifacts only to explain fallback decisions and simplex/debug
  facts; core pass/fail belongs in `crates/picea`.

Suggested targeted gates:

```bash
rtk proxy cargo test -p picea --lib pipeline::narrowphase
rtk proxy cargo test -p picea --test query_debug_contract
rtk proxy cargo test -p picea --test physics_realism_acceptance
```

## M8 CCD TOI

### Goal

Prevent fast bodies from tunneling through thin static geometry by adding a
time-of-impact path, starting narrow and expanding only after the first case is
stable.

### In Scope

- Add swept AABB broadphase support.
- Support fast circle vs static thin wall TOI first.
- Extend to circle vs static convex once the first path is green.
- Add conservative advancement where needed.
- Define substep and contact event semantics.
- Turn the existing ignored CCD known-red test into a normal passing test.
- Expose CCD trace facts: swept candidate, TOI, advancement, clamp/slop, and
  generated contact event.

### Out Of Scope

- Dynamic-vs-dynamic CCD unless a later benchmark justifies it.
- Generic CCD for every shape in the first slice.
- Performance thresholds.

### Acceptance Method

- Start by unignoring or duplicating the fast circle vs thin wall known-red as
  the failing behavior lock.
- Add tests for no false positives when the sweep misses and for event semantics
  when the sweep hits.
- Use `picea-lab` artifacts to visualize the swept path, TOI point, and final
  clamped pose.

Suggested targeted gates:

```bash
rtk proxy cargo test -p picea --test physics_realism_acceptance ccd
rtk proxy cargo test -p picea --lib pipeline
rtk proxy cargo test -p picea-lab
```

## M9 API Recipes, Observability Closure, And Benchmarks

### Goal

After the core physics path is stable, make scenario creation easier, close the
observability story, and add performance baselines without pretending early
numbers are production thresholds.

### In Scope

- Add `BodyBundle` and `ColliderBundle`.
- Add `WorldRecipe` for declarative test worlds and examples.
- Add `WorldCommands` for batch create, destroy, and patch with validation before
  mutation.
- Add material presets and collision layer presets.
- Return structured creation results with handles, events, and validation
  errors.
- Ensure debug snapshots can explain broadphase, narrowphase, solver, sleep, and
  CCD decisions.
- Add Criterion benchmark baselines for sparse broadphase, dense broadphase,
  stack stability, CCD bullet, and API batch creation.

### Out Of Scope

- Reopening the low-level `World::create_body` and `World::create_collider`
  contract unless implementation evidence proves the existing shape cannot
  support bundles.
- Setting absolute performance thresholds before collecting local baselines.

### Acceptance Method

- Add API recipe and batch scenario tests that create real worlds and run at
  least one deterministic step.
- Add artifact schema checks for the final debug fact set.
- Add Criterion benches as baselines and record Picea counters alongside timing.
- Treat unexplained regressions after baselines exist as investigation triggers;
  do not invent threshold numbers in this milestone document.

Suggested targeted gates:

```bash
rtk proxy cargo test -p picea --test v1_api_smoke
rtk proxy cargo test -p picea --test core_model_world
rtk proxy cargo test -p picea-lab
rtk proxy cargo bench -p picea --no-run
```

## Picea Lab Role Across Milestones

`picea-lab` should help AI and humans inspect real behavior. Its role grows by
milestone:

- M1: capture broadphase candidate count, tree depth, update count, and drop
  reasons for a sparse broadphase scenario.
- M2: render manifold points, normals, feature ids, and contact reduction
  reasons.
- M3: expose mass, center-of-mass, and inertia facts when they are needed to
  explain solver input.
- M4: show manifold identity, warm-start eligibility, hit/miss/drop reasons, and
  feature drift.
- M5: show normal/tangent impulses, clamp state, restitution threshold decisions,
  and stack contact stability.
- M6: show sleeping islands, wake reasons, and related event ordering.
- M7: explain generic convex fallback decisions, simplex/debug facts, and
  failure containment.
- M8: show swept paths, TOI points, advancement/clamp decisions, and generated
  contact events.
- M9: host benchmark scenario definitions, artifact schema checks, and Criterion
  baseline summaries.

The lab should not run physics independently from `crates/picea`, and it should
not be the only pass/fail signal for physics correctness.

## AI Correctness Strategy

AI-assisted implementation should not rely on code inspection alone. For each
milestone, the agent must connect its claim to executable evidence:

- local unit tests for small algorithms and formulas;
- deterministic scenario tests for integrated physics behavior;
- debug facts and artifacts for explainability;
- golden or replay-style comparisons where event sequences and counters matter;
- optional fuzz/property tests for degenerate input containment;
- real command output from `rtk proxy` gates before saying the milestone is done.

Unit tests are necessary, but not sufficient. The engine needs behavior locks,
scenario acceptance, and artifact evidence because physics bugs often appear in
phase ordering, temporal coherence, and state transfer rather than in one helper
function.
