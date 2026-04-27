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

## Post-M9 Design Target

After M1-M9, Picea has most of the originally planned physical capability slices:
persistent broadphase, SAT manifolds, mass properties, warm-start facts,
sequential impulses, island sleep behavior, GJK/EPA fallback, narrow CCD, recipes,
debug facts, lab artifacts, and baseline benches. The next risk is no longer one
missing algorithm; it is architectural drift from several fast production slices.

The next design target is therefore architecture consolidation:

- Make step orchestration explicit. Each step should have one transient
  `StepContext`-style owner for previous poses, CCD results, wake reasons,
  broadphase/contact/solver facts, numeric warnings, and final stats.
- Keep phase responsibilities narrow. CCD should propose or apply TOI state in a
  clearly named phase; contact gathering, contact solving, and contact event
  emission should not live as one growing module.
- Pick a canonical source of truth for per-step facts, then derive `StepStats`,
  debug snapshots, events, and lab artifacts from that source instead of manually
  mirroring fields across layers.
- Treat public ergonomics as a product surface. `recipe` is exported from core,
  so bundles, command errors, and authoring helpers should be designed as stable
  setup APIs, not as hot-path mutation shortcuts.
- Treat `picea-lab` as a replay/evidence workbench unless and until live
  simulation semantics are intentionally designed. Its UI should make artifact
  provenance, final snapshots, joints, and backend/demo state explicit.

The concrete consequence is that M10 should come before broadening CCD coverage
or adding stricter benchmark thresholds.

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

> Status: completed 2026-04-27.
>
> Completion notes: sleep now operates on deterministic contact/joint islands
> rather than isolated body checks. Sleep events carry island ids and transition
> reasons; pending wake reasons are recorded for contact impulses, joint
> correction, user patches, transform edits, velocity edits, and impact-style
> wakeups. Core behavior tests cover stability-window sleep, island-level sleep,
> transform wake, static-contact non-bridging, impact wake, unrelated sleeping
> islands, and resting-contact stay-asleep behavior. Debug snapshots and lab
> artifacts expose island membership and the most recent island sleep/wake
> reason.
>
> Residual risk: island membership is currently rebuilt from contact/joint facts
> during the step rather than stored as a first-class retained island graph. That
> is acceptable for behavior correctness now, but M10 should make the per-step
> transient ownership explicit so event/debug ordering cannot silently become the
> island source of truth.

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

> Status: completed 2026-04-27.
>
> Completion notes: core now has an internal `pipeline::gjk` support-mapped
> convex kernel. Circles, rectangles, regular polygons, convex polygons, and
> segments expose crate-private support points; concave polygons remain outside
> the single-convex-shape path. GJK reports deterministic separation,
> touching/intersection, iteration count, and simplex size; EPA is the primary
> penetration route for the generic fallback and contains degenerate failures
> without panics or NaNs. Specialized SAT/analytic paths remain primary, while
> segment-vs-convex fallback contacts carry `generic_convex_fallback` plus
> GJK/EPA trace facts through events and debug snapshots with serde defaults.

Residual risks:

- The generic fallback exports a single conservative contact point; richer
  clipped manifolds remain owned by the specialized SAT path.
- Degenerate zero-area convex pairs can be contained before EPA converges; they
  do not fabricate a penetration face.

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

> Status: completed narrow core CCD slice 2026-04-27.
>
> Completion notes: the previous CCD known-red has been promoted into normal
> behavior tests. Core supports the intended first production slice:
> dynamic circle vs static thin wall / static convex, with swept AABB
> candidate filtering, TOI hit/miss accounting, pose clamping, and generated
> contact events carrying `ccd_trace`. Core exports CCD counters through
> `StepStats` / `DebugStats` and attaches `ccd_trace: Option<CcdTrace>` to
> contact events and debug contacts. `picea-lab` keeps those facts on the
> existing `frames.jsonl` / `final_snapshot.json` path, adds a
> `ccd_fast_circle_wall` builtin scenario, and the web workbench can inspect
> selected contact TOI, advancement, clamp/slop, swept start/end, and TOI point.
> The canvas renders generic `snapshot.primitives`, so core-provided swept path,
> TOI marker, or label primitives are visible without another lab schema change.
>
> Residual risks: dynamic-vs-dynamic CCD and full generic all-shape CCD remain
> out of scope. M10 clarified the current CCD boundary as a narrow
> pose-clamping phase before contact generation; broader CCD should build on
> that explicit phase contract rather than blur it back into contact gathering.

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

> Status: minimal core/API and Criterion baseline slice implemented 2026-04-27.
>
> Completion notes: core now exposes `recipe` wrappers for `BodyBundle`,
> `ColliderBundle`, `WorldRecipe`, and transactional `WorldCommands` without
> changing the low-level `World::create_body` / `World::create_collider`
> contracts. Batch commands cover body, existing-body collider, joint, patch, and
> destroy paths by running against a cloned scratch `World` and committing only
> after every command succeeds, so validation and handle errors do not leak
> partial mutation into the caller's world. Material and collision layer presets
> cover the common recipe/benchmark cases. `crates/picea` now has Criterion
> baselines for sparse broadphase, dense broadphase, stack stability, CCD bullet,
> and API batch creation; benchmark IDs record deterministic Picea counters
> alongside timings and intentionally do not set thresholds. `picea-lab`
> artifact schema tests parse saved JSON and typed render artifacts to lock the
> final broadphase, contact/manifold, warm-start, sleep/island, and CCD fact
> carriers used by the viewer and saved run artifacts.
>
> Residual risk: Criterion lockfile ownership needs supervisor acceptance if the
> dependency update is committed.

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

## M10 Architecture Consolidation And Product Surface Cleanup

> Status: completed 2026-04-27.
>
> Completion notes: `SimulationPipeline::step` now routes through an explicit
> internal `StepContext` that owns previous poses, previous sleep state, wake
> reasons, CCD pose-clamp traces, broadphase stats, warm-start stats, contact
> counts, sleep counts, numeric warnings, and final `StepStats` assembly. Contact
> solver rows, effective mass, warm-start application, velocity writeback, and
> residual correction now live under `solver/contact.rs`, while
> `pipeline/contacts.rs` retains gather / warm-start / event emission. CCD is
> named as `run_pose_clamp_phase`; it keeps the M8 shape coverage but makes the
> mutating TOI clamp boundary explicit. Core `recipe` now supports
> `JointBundle` / `WorldRecipe::with_joint`, and `picea-lab` exposes replay
> provenance, final snapshots, selectable joints, backend/demo state, and
> non-failing empty SSE semantics.
>
> Residual risks: M10 preserved behavior rather than broadening physics. The
> broadphase/query path, shape geometry caches, active-island solver layout,
> generalized CCD, and higher-level ergonomic scene APIs remain follow-up
> milestones.

### Goal

Consolidate the post-M9 engine architecture before adding broader capabilities.
The target is not new physics behavior; it is clearer ownership, smaller phase
boundaries, less duplicated fact plumbing, and a more honest authoring/debugging
surface.

### Design Goals

- Make `SimulationPipeline::step` read like a real pipeline, with one explicit
  transient step context carrying all temporary facts between phases.
- Move contact solving toward the `solver` module so `pipeline/contacts.rs`
  stops owning solver rows, warm-start application, residual correction, and
  event emission at the same time.
- Make CCD phase ownership explicit: either output a `CcdResolution` proposal
  applied by the step context, or clearly name CCD as the pose-clamping phase.
- Reduce schema duplication by deriving `StepStats`, debug stats, contact debug
  facts, and lab artifact counters from one canonical step fact set.
- Keep `recipe` ergonomic but honest: it is an authoring/setup API with atomic
  clone-and-commit semantics, not a hot-path mutation API.
- Make `picea-lab` visibly replay/evidence-oriented unless live simulation is
  separately designed.
- Update routing docs and `todo.md` so future sessions do not reopen completed
  known-red items.

### In Scope

- Introduce an internal `StepContext` or equivalent transient step-state
  structure.
- Split `pipeline/contacts.rs` into smaller gather / solve / emit modules, or
  move solver-specific code under `solver`.
- Refactor CCD output/application boundaries without broadening CCD shape
  coverage.
- Centralize per-step fact aggregation for stats, debug, events, and artifacts.
- Add recipe joint authoring support if it fits without reopening low-level
  `World` semantics.
- Tighten `picea-lab` replay semantics: final snapshot visibility, joint
  selection/inspection, backend/demo state clarity, and misleading empty-SSE
  failure handling.
- Synchronize `todo.md`, `docs/ai/index.md`, and `docs/ai/repo-map.md` with the
  current milestone reality and verification commands.

### Out Of Scope

- Dynamic-vs-dynamic CCD.
- Full generic all-shape CCD.
- New physics features beyond preserving existing behavior.
- Absolute benchmark thresholds.
- Turning `picea-lab` into a real-time simulator.

### Acceptance Method

- Start with characterization tests or existing gates that lock current behavior
  before refactoring phase boundaries.
- Keep public API behavior stable unless a change is explicitly part of product
  surface cleanup.
- Show that `StepStats`, `DebugSnapshot`, contact events, and lab artifacts still
  expose the same facts after consolidation.
- Add focused tests for recipe joint authoring and lab UI/server semantics when
  those slices are touched.
- Document residual risks where the cleanup intentionally leaves a broader
  design decision for a later milestone.

Suggested targeted gates:

```bash
rtk proxy cargo test -p picea --lib
rtk proxy cargo test -p picea --tests
rtk proxy cargo test -p picea-lab
cd crates/picea-lab/web && rtk proxy npm run build
rtk proxy cargo bench -p picea --no-run
rtk proxy ruby -e 'require "yaml"; YAML.load_file("docs/ai/doc-catalog.yaml"); puts "yaml ok"'
rtk proxy git diff --check
```

## M10.5 Documentation And Backlog Closeout

> Status: planned.

### Goal

Make the repository documentation agree with the M10 reality before new
implementation work starts. This is a small hygiene milestone, but it prevents
future sessions from reopening completed M1-M10 work as if it were still
known-red.

### In Scope

- Mark M10 as completed in this production milestone document.
- Update `docs/design/physics-engine-upgrade-technical-plan.md` from the old
  algorithm-gap framing to a post-M10 system-upgrade framing.
- Update `todo.md` so completed M1-M10 items are not left as unchecked backlog.
- Refresh AI routing/catalog metadata when the active milestone names or
  verification commands change.

### Out Of Scope

- Code changes.
- New performance, solver, CCD, or API behavior.
- Commit or release work unless explicitly requested.

### Acceptance Method

- Run YAML and whitespace/diff checks.
- Confirm git status shows only documentation files.

Suggested targeted gates:

```bash
rtk proxy ruby -e 'require "yaml"; YAML.load_file("docs/ai/doc-catalog.yaml"); puts "yaml ok"'
rtk proxy git diff --check
```

## M11 Performance Substrate

> Status: planned.

### Goal

Turn the post-M10 engine into a performance-ready system without changing public
behavior. M11 should make hot data flow explicit: broadphase indexing, query
reuse, shape geometry caches, and lower per-step allocation.

### Design Goals

- Add direct collider-handle to broadphase leaf lookup so proxy updates do not
  search leaves linearly.
- Expose or reuse broadphase-style tree queries for ray/AABB/region queries
  where semantics match the stable `QueryPipeline` contract.
- Cache world-space vertices and support data behind transform/revision
  invalidation instead of rebuilding `Vec<Point>` in narrowphase, CCD, and GJK
  paths.
- Reduce per-step `Vec` / `BTreeMap` churn in broadphase, contact gathering,
  sleep islands, and solver setup.
- Preserve deterministic ordering and existing debug counters while adding new
  allocation/cache counters only when they explain behavior.
- Keep Criterion as baseline evidence first; add thresholds only after local
  variance is understood.

### In Scope

- Broadphase leaf map / proxy lookup internals.
- Query cache acceleration that preserves current public query semantics.
- Shape geometry/support caches for supported shapes.
- Focused allocation reductions in step hot paths.
- Benchmark IDs or artifact counters that explain candidate/contact/query/cache
  work.

### Out Of Scope

- Public API redesign.
- Active island compact solver arrays; that is M12.
- CCD shape-cast generalization; that is M13.
- Absolute performance thresholds before baseline variance is reviewed.

### Acceptance Method

- Add behavior locks proving query and contact ordering stay deterministic.
- Add focused tests for stale broadphase/query/shape-cache invalidation.
- Compare Criterion baseline IDs and counters before/after the change; treat
  unexplained counter regressions as blockers even without absolute thresholds.

Suggested targeted gates:

```bash
rtk proxy cargo test -p picea --lib pipeline::broadphase
rtk proxy cargo test -p picea --test query_debug_contract
rtk proxy cargo bench -p picea --no-run
```

## M12 Active Island Solver

> Status: planned.

### Goal

Move from a correct single-world contact solver to an island-owned solver
execution model. The target is better stability and performance for stacks,
friction, and jointed scenes while preserving public handles and existing debug
facts.

### Design Goals

- Build compact active-island arrays for dynamic bodies, contact rows, and joint
  rows during the step.
- Solve contacts and joints through one deterministic island ordering contract.
- Keep sleeping islands out of hot solver arrays unless a wake reason brings
  them back.
- Preserve contact ids, manifold ids, warm-start facts, and sleep/wake reasons
  across the internal layout change.
- Keep residual position correction in the solver phase and prove it does not
  overwrite solved velocity facts.

### In Scope

- Internal island solve data structures.
- Contact and joint row batching by island.
- Debug facts that prove island membership and solved impulses still line up.
- Stack, ramp friction, restitution threshold, and jointed-island regressions.

### Out Of Scope

- New joint types.
- CCD generalization.
- Multithreaded island solving.
- Public handle or recipe API changes.

### Acceptance Method

- Start with characterization tests for existing stack/friction/joint behavior.
- Add active-island tests showing unrelated islands solve and sleep
  independently.
- Keep `StepStats`, `DebugSnapshot`, contact events, and lab artifacts
  semantically stable.

Suggested targeted gates:

```bash
rtk proxy cargo test -p picea --test physics_realism_acceptance stack
rtk proxy cargo test -p picea --test physics_realism_acceptance sleep
rtk proxy cargo test -p picea --test world_step_review_regressions
```

## M13 CCD Generalization

> Status: planned.

### Goal

Generalize CCD from the narrow dynamic-circle pose clamp into a staged TOI /
shape-cast system without making the step pipeline opaque. The goal is fewer
tunneling cases with explicit trace semantics and controlled cost.

### Design Goals

- Keep CCD as a named step phase that proposes or applies TOI advancement before
  contact generation.
- Extend dynamic-vs-static support beyond circles through convex shape casts or
  GJK-backed conservative advancement.
- Add multi-impact budgeting so one moving body can handle more than one
  relevant hit without unbounded substeps.
- Keep `ccd_trace` rich enough to explain swept start/end, candidate, TOI,
  advancement, clamp/slop, and chosen/ignored impacts.
- Gate dynamic-vs-dynamic CCD behind benchmark and behavior evidence.

### In Scope

- Dynamic-vs-static convex CCD beyond circles.
- Conservative advancement or shape-cast helpers.
- Multi-hit ordering and budget semantics.
- CCD trace/artifact updates.

### Out Of Scope

- Full all-shape CCD in one slice.
- Dynamic-vs-dynamic CCD unless a focused follow-up justifies it.
- Turning CCD into hidden substeps with no event/debug trace.

### Acceptance Method

- Add known-red tests for a non-circle dynamic convex body crossing thin static
  geometry.
- Add no-false-positive and multi-hit ordering tests.
- Use `picea-lab` artifacts to show swept paths and selected/ignored TOI hits.

Suggested targeted gates:

```bash
rtk proxy cargo test -p picea --test physics_realism_acceptance ccd
rtk proxy cargo test -p picea --lib pipeline::ccd
rtk proxy cargo test -p picea-lab
```

## M14 Ergonomic API V2

> Status: planned.

### Goal

Build on the M9/M10 recipe surface to make scene authoring easier than a direct
Box2D/Rapier-style object-by-object API, while keeping low-level `World`
controls stable for advanced users.

### Design Goals

- Add higher-level scene/asset recipes for common test, example, and benchmark
  worlds.
- Make recipe and command errors point to the nested body/collider/joint path
  that failed.
- Support serializable setup flows where that helps examples, fixtures, and
  lab scenarios.
- Keep `WorldCommands` atomic clone-and-commit semantics honest; do not present
  it as a hot-path mutation API.
- Keep public API additions additive and small.

### In Scope

- Scene-level recipe helpers.
- Better recipe/command error context.
- Serializable recipe fixtures if the schema can stay stable.
- Documentation examples and smoke tests.

### Out Of Scope

- Replacing low-level `World::create_*` APIs.
- Live editing semantics in `picea-lab`.
- Runtime solver or CCD changes.

### Acceptance Method

- Add `v1_api_smoke` tests for the new ergonomic path.
- Add one lab/example fixture if serialization is introduced.
- Prove low-level APIs still compile and behave unchanged.

Suggested targeted gates:

```bash
rtk proxy cargo test -p picea --test v1_api_smoke
rtk proxy cargo test -p picea --test core_model_world
rtk proxy cargo test -p picea-lab
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
- M10: make replay provenance, final snapshots, joints, and backend/demo state
  explicit while preserving the same artifact fact surface.
- M11: surface performance evidence through candidate/query/cache counters and
  benchmark scenario IDs without making the lab the timing oracle.
- M12: show active island membership, solved contact/joint rows, wake reasons,
  and sleep state after solver compaction.
- M13: visualize generalized CCD sweeps, selected/ignored TOI hits, and
  advancement budgets.
- M14: host ergonomic scene/recipe examples as evidence that the public setup
  flow stays easy and reproducible.

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
