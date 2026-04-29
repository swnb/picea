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

> Status: completed 2026-04-27.
>
> Completion notes: M10 is marked completed, the post-M10 upgrade plan now frames
> the remaining work as system-quality follow-up, `todo.md` keeps M1-M10 as a
> completed production baseline, and AI routing/catalog metadata points future
> implementation sessions to M11-M14.

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

> Status: completed 2026-04-27.
>
> Completion notes: the accepted M11 scope is the direct broadphase
> collider-handle to leaf lookup substrate. Broadphase proxy maintenance no
> longer searches leaves linearly, and the branch keeps existing public query
> semantics and benchmark posture unchanged.

### Goal

Turn the post-M10 engine toward a performance-ready system without changing
public behavior. The accepted M11 scope is the first substrate step: direct
broadphase indexing from collider handle to tree leaf. Broader query reuse,
shape geometry caches, and lower per-step allocation are Post-M14 deepening
items.

### Design Goals

- Add direct collider-handle to broadphase leaf lookup so proxy updates do not
  search leaves linearly.
- Preserve deterministic ordering and existing debug counters.
- Keep public query semantics and Criterion baseline posture unchanged while
  creating a clear substrate for later query/cache work.

### In Scope

- Broadphase leaf map / proxy lookup internals.
- Stale/recycled handle safety for the lookup cache.
- Focused behavior locks proving rebuilds, moves, removals, and recycled
  handles keep the lookup valid.

### Out Of Scope

- Public API redesign.
- Active island compact solver arrays; that is M12.
- CCD shape-cast generalization; that is M13.
- Absolute performance thresholds before baseline variance is reviewed.

### Acceptance Method

- Add behavior locks proving direct leaf lookup tracks moves, rebuilds, stale
  removals, and recycled handles.
- Keep existing query/debug behavior stable.
- Keep the Criterion bench targets buildable without introducing hard thresholds.

Suggested targeted gates:

```bash
rtk proxy cargo test -p picea --lib pipeline::broadphase
rtk proxy cargo test -p picea --test query_debug_contract
rtk proxy cargo bench -p picea --no-run
```

## M12 Active Island Solver

> Status: completed 2026-04-27.
>
> Completion notes: contact rows and joint rows now batch by active island, and
> sleeping islands no longer stay in the hottest solver rows. This gives the
> post-M10 solver an accepted island-owned execution slice while preserving
> current warm-start facts, handles, and debug/event surfaces.

### Goal

Move from a correct single-world contact solver toward an island-owned solver
execution model. The accepted M12 scope is active-island batching for contact
and joint rows, with sleeping islands skipped from hot solver rows while public
handles and existing debug facts stay stable.

### Design Goals

- Build deterministic active-island batches for dynamic bodies, contact rows,
  and joint rows during the step.
- Solve contacts and joints by island while preserving the current separate
  contact/joint phase boundary.
- Keep sleeping islands out of hot solver arrays unless a wake reason brings
  them back.
- Preserve contact ids, manifold ids, warm-start facts, and sleep/wake reasons
  across the internal layout change.
- Keep residual position correction in the solver phase and prove it does not
  overwrite solved velocity facts.

### In Scope

- Internal active-island solve batching.
- Contact and joint row batching by island.
- Debug facts that prove island membership and solved impulses still line up.
- Stack, friction, restitution threshold, jointed-island, and unrelated-island
  regressions.

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

> Status: completed 2026-04-27.
>
> Completion notes: CCD remains a named pose-clamping phase before contact
> generation. The phase now keeps the existing dynamic-circle analytic path and
> adds a translational dynamic-convex vs static-convex shape cast for rectangles,
> regular polygons, and convex polygons. Multi-hit behavior is still bounded:
> hits are sorted by TOI, one moving body receives at most one clamp in a step,
> and global CCD counters expose candidate / hit / miss / clamp facts so a
> later ignored hit is visible as `hit_count > clamp_count`. `ccd_trace` keeps
> its existing field set; comments now describe the swept collider reference
> point instead of circle-only center semantics. `picea-lab` adds a
> `ccd_fast_convex_walls` artifact scenario that records the selected earliest
> hit and the budgeted later hit through the existing stats + selected-contact
> `ccd_trace` path.
>
### Goal

Generalize CCD from the narrow dynamic-circle pose clamp into a staged TOI /
shape-cast system without making the step pipeline opaque. The goal is fewer
tunneling cases with explicit trace semantics and controlled cost.

### Design Goals

- Keep CCD as a named step phase that proposes or applies TOI advancement before
  contact generation.
- Extend dynamic-vs-static support beyond circles through convex shape casts or
  staged shape-cast helpers.
- Add multi-impact budgeting so one moving body can handle more than one
  relevant hit without unbounded substeps.
- Keep `ccd_trace` rich enough to explain swept start/end, candidate, TOI,
  advancement, clamp/slop, and chosen/ignored impacts.
- Gate dynamic-vs-dynamic CCD behind benchmark and behavior evidence.

### In Scope

- Dynamic-vs-static convex CCD beyond circles.
- Shape-cast helpers for the accepted translational convex slice.
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

> Status: completed 2026-04-27.
>
> Completion notes: core now has higher-level scene/asset recipe helpers,
> nested path context for recipe/command validation, and a serializable fixture
> path that `picea-lab` scenarios can consume directly. The low-level
> `World::create_*` APIs remain the stable fallback surface, and the accepted
> branch gate is `v1_api_smoke` plus lab fixture acceptance rather than public
> scene-schema freeze.

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

## M15 Performance Data Path

> Status: completed 2026-04-27.
>
> M15 is the first Post-M14 implementation milestone. It makes the accepted
> M11-M14 capability line faster and more reusable without changing the public
> authoring or physics semantics.
>
> Completion notes: `QueryPipeline` now builds an internal balanced
> broadphase-style index for AABB, point, and ray candidate traversal while
> preserving public hit ordering and filter semantics. `ColliderRecord` caches
> transform-derived AABBs and convex world vertices behind geometry revision and
> world-pose keys; contact gathering, CCD, and generic GJK/EPA fallback reuse
> those facts to reduce repeated geometry rebuilding, with conservative
> allocation pre-sizing where the current code makes it visible. Broadphase
> proxy/leaf ids remain private. Query allocation/perf counters and deeper
> solver allocation work remain Post-M15.

### Goal

Turn the landed broadphase, query, shape, contact, sleep, and solver facts into
a measurable hot data path. The main win should be less repeated work: queries
should reuse an indexed spatial structure where semantics match, shape/support
geometry should be cached behind transform revision, and step setup should
pre-size or reuse temporary collections only where the current behavior locks
and counters justify it.

### Design Goals

- Reuse broadphase-style spatial indexing for `QueryPipeline` candidate
  selection while keeping public query ordering and filtering stable.
- Keep broadphase proxy/leaf details internal; public callers still work with
  body/collider handles and query hit types, not proxy ids.
- Cache world-space vertices, AABBs, and convex support data behind explicit
  transform/revision invalidation.
- Reduce repeated geometry recomputation in contact gathering, CCD, and generic
  GJK/EPA fallback; keep deeper allocation work behind tests or counters.
- Preserve deterministic ordering, `DebugSnapshot`, `StepStats`, and
  `picea-lab` artifact semantics.
- Keep Criterion as baseline evidence first; add hard performance thresholds
  only after variance is understood.

### In Scope

- `QueryPipeline` / broadphase internal reuse for AABB, ray, and region-style
  candidate traversal.
- Shape/support/world-vertices cache storage, invalidation, and stale-cache
  behavior locks.
- Focused recomputation reductions and conservative collection pre-sizing in
  contact gathering, CCD, and support-map paths.
- Benchmark buildability plus counter/variance evidence that explains the data
  path change; stronger query allocation/perf counters remain follow-up.
- AI routing/doc updates if the implemented entry points change.

### Out Of Scope

- Dense island-local solver arrays; keep that for a later M16-style milestone.
- Dynamic-vs-dynamic CCD, rotational CCD, and all-shape CCD expansion.
- Public scene schema stabilization or live `picea-lab` editing.
- Public distance-query API stabilization.
- Absolute perf pass/fail thresholds before multiple local baselines exist.

### Acceptance Method

- Start with behavior locks for current query semantics, deterministic hit
  ordering, and stale-cache invalidation before changing the data path.
- Add targeted broadphase/query/cache tests that fail if transforms, removals,
  recycled handles, or rebuilds leave stale spatial data behind.
- Keep existing physics realism, debug contract, and recipe smoke tests stable
  unless the implementation explicitly extends their debug facts.
- Build Criterion benches without turning benchmark timing into a brittle gate;
  report counters or baseline variance instead.

Suggested targeted gates:

```bash
rtk proxy cargo test -p picea --lib pipeline::broadphase
rtk proxy cargo test -p picea --test query_debug_contract
rtk proxy cargo test -p picea --test world_step_review_regressions
rtk proxy cargo bench -p picea --no-run
rtk proxy ruby -e 'require "yaml"; YAML.load_file("docs/ai/doc-catalog.yaml"); puts "yaml ok"'
rtk proxy git diff --check
```

## M16 Dense Island Execution

> Status: completed 2026-04-28.
>
> Completion notes: `pipeline::island::IslandSolvePlan` now turns accepted M12
> active-island facts into deterministic island-local body slots, contact row
> indices, and joint rows. Contact solver velocity rows use dense slot indices
> during impulse iteration, joint batching uses the same solve-plan surface, and
> contact-impact wake recording now happens before sleeping-body velocity
> writeback so M12/M16 wake semantics stay stable. Public handles, contact
> events, `StepStats`, `DebugSnapshot`, lab artifact semantics, and the live
> separate contact/joint phase order are unchanged.

### Goal

Replace the current map/set-heavy active-island solver staging with a
deterministic per-island solve plan. Each active island should own compact body
slots, contact row indices, joint rows, and the temporary handle-to-slot lookup
needed by the solver. The public world model still uses stable handles; dense
slots are an internal hot-path representation.

### Design Goals

- Build a deterministic `IslandSolvePlan` from the existing `SolverIsland`
  facts, contact observations, and joint records.
- Use island-local body slots for contact solver velocity reads/writes instead
  of repeatedly looking up `BodyHandle` in map-heavy hot paths.
- Batch joint rows through the same island plan while preserving the current
  separate-phase behavior and live step order.
- Keep sleeping islands out of row construction and keep wake reasons explicit.
- Preserve warm-start impulse transfer, contact ids, manifold ids, `StepStats`,
  `DebugSnapshot`, and `picea-lab` artifact semantics.
- Add allocation/counter evidence only where it is cheap and directly tied to
  the new solver layout; do not set brittle timing thresholds yet.

### In Scope

- Internal island solve-plan data structures and deterministic ordering tests.
- Contact solver body slots, contact row batches, and velocity writeback through
  island-local slot indices.
- Joint row batching through the same island plan, without changing joint
  behavior or phase order.
- Tests proving sleeping islands do not build hot rows and unrelated islands do
  not affect each other.
- Focused stack, sleep, warm-start, and joint regression coverage.

### Out Of Scope

- Multithreaded island solving.
- A unified contact/joint solver phase or new joint types.
- Dynamic-vs-dynamic CCD, rotational CCD, or all-shape CCD expansion.
- Public API changes, public scene schema stabilization, or live lab editing.
- Absolute performance pass/fail thresholds before baseline variance is clear.

### Acceptance Method

- Start with behavior locks for current stack, friction, warm-start, sleep, and
  jointed-island behavior.
- Add layout-specific tests proving the solve plan has deterministic island
  order, stable body slot assignment, and no row construction for sleeping
  islands.
- Keep existing `StepStats`, `DebugSnapshot`, contact events, and lab artifacts
  semantically stable unless explicitly adding new counters.
- Build Criterion benches without making timing variance a pass/fail gate.

Suggested targeted gates:

```bash
rtk proxy cargo test -p picea --test physics_realism_acceptance stack
rtk proxy cargo test -p picea --test physics_realism_acceptance sleep
rtk proxy cargo test -p picea --test world_step_review_regressions
rtk proxy cargo test -p picea --lib pipeline::island
rtk proxy cargo test -p picea --lib pipeline::sleep
rtk proxy cargo bench -p picea --no-run
rtk proxy ruby -e 'require "yaml"; YAML.load_file("docs/ai/doc-catalog.yaml"); puts "yaml ok"'
rtk proxy git diff --check
```

## M17 Performance Evidence And Tuning Gate

> Status: completed 2026-04-28.
>
> M17 is the measurement gate after the accepted performance data-layout work.
> M15 made query and geometry reuse real, and M16 made island solver rows dense.
> M17 should turn those structural improvements into explicit evidence before
> the roadmap spends more complexity budget on CCD expansion, broadphase tuning,
> or public scene-schema work.
>
> Completion notes: core now exposes additive deterministic evidence counters
> without changing physics semantics. `QueryPipeline` keeps `QueryStats` for the
> most recent query call, covering traversal, candidate, prune, filter-drop, and
> hit counts. `StepStats` / `DebugStats` now include broadphase traversal/prune
> counts plus island/active-island/sleeping-skip, solver body slot, contact row,
> and joint row counts with serde defaults for older payloads. `picea-lab`
> propagates those counters through debug render frames and adds a
> `perf.json.counter_summary` aggregation so artifacts carry deterministic work
> shape separately from wall-clock timing. Criterion bench IDs now include
> counter summaries and cover query-heavy, many-small-islands, and one large
> island scenarios in addition to the existing sparse/dense broadphase, stack,
> CCD bullet, and recipe-heavy baselines.
>
> Residual risk: M17 records local evidence shape but does not tune the
> broadphase/query tree, expand CCD, or set hard timing thresholds. M18 should
> choose its first tuning slice from these counters and repeated local bench
> baselines.

### Background

Picea's current advantage is not just "has the same checklist as Box2D/Rapier".
The intended advantage is an engine that is easier to author, easier to inspect,
and fast for the right reasons. M11-M16 have moved the internals toward that:
direct broadphase lookup, indexed query candidates, transform/revision-backed
geometry caches, and dense island-local solver data. What is still missing is a
stable evidence layer that shows where time and work go.

Without that evidence, future sessions can make plausible but unproven
performance claims, or choose the wrong next optimization. M17 should make the
next choices measurable: broadphase/query tuning, CCD expansion, solver layout
deepening, and lab/schema work should all be prioritized from counters and
baseline variance rather than intuition.

### Goal

Create a performance evidence gate for Picea's post-M16 engine. The milestone
should add deterministic counters, artifact summaries, and Criterion baseline
coverage that explain the cost shape of query, broadphase, solver-island, and
contact/CCD-heavy scenarios. It should not turn timing into brittle CI pass/fail
thresholds yet.

### Design Goals

- Expose cheap, deterministic counters for query traversal, candidate pruning,
  contact/joint row construction, active/sleeping island work, and solver body
  slots where those facts already exist or are cheap to collect.
- Keep `picea-lab` as an evidence/artifact layer, not a timing oracle.
- Preserve `StepStats`, `DebugSnapshot`, and artifact compatibility; if new
  fields are serialized, older fixtures should still deserialize through
  defaulted fields.
- Extend Criterion coverage around representative scenarios: sparse/dense
  broadphase, query-heavy scenes, many small islands, one large island, stacked
  contacts, CCD bullets, and large recipe creation.
- Record baseline variance and expected counter shapes before setting hard
  performance thresholds.
- Use the evidence to decide the next optimization milestone instead of bundling
  broadphase tuning, CCD expansion, and scene schema work together.

### In Scope

- Query/broadphase counters such as candidate count, tree traversal count,
  pruned candidate count, filtered hit count, and hit ordering stability.
- Solver/island counters such as island count, active island count, sleeping
  island skip count, body slot count, contact row count, and joint row count.
- `picea-lab` artifact/perf summaries that surface the new counters for saved
  runs without treating wall-clock timing as authoritative.
- Criterion benchmark coverage for the accepted post-M16 hot paths.
- Documentation of baseline variance, counter interpretation, and which future
  milestone the evidence points toward.

### Out Of Scope

- Hard absolute timing thresholds in CI.
- Broadphase insertion/balancing rewrites; those belong to a later tuning
  milestone once M17 evidence says they are worth doing.
- Dynamic-vs-dynamic CCD, rotational CCD, or broader all-shape CCD.
- Public scene schema stabilization or live `picea-lab` editing.
- Public API changes beyond additive debug/stat fields.

### Acceptance Method

- Add deterministic tests for any new counters so small scenes prove the exact
  query, island, row, and skipped-work facts being reported.
- If serialized stats/artifacts change, add backward-compatible serde tests for
  older payloads and schema checks for new payloads.
- Build and, where practical, run focused Criterion scenarios to capture local
  baseline variance; do not fail the milestone on absolute timings.
- Update `picea-lab` artifact tests if new perf/debug summaries are emitted.
- Update routing docs if new counter fields, benchmark names, or artifact files
  become the preferred entry points for future optimization work.

Suggested targeted gates:

```bash
rtk proxy cargo test -p picea --test query_debug_contract
rtk proxy cargo test -p picea --test world_step_review_regressions
rtk proxy cargo test -p picea --test physics_realism_acceptance stack
rtk proxy cargo test -p picea --lib pipeline::island
rtk proxy cargo test -p picea-lab --test artifact_run
rtk proxy cargo bench -p picea --no-run
rtk proxy ruby -e 'require "yaml"; YAML.load_file("docs/ai/doc-catalog.yaml"); puts "yaml ok"'
rtk proxy git diff --check
```

## M18 Broadphase And Query Tuning

> Status: completed on 2026-04-28.
>
> M18 is the first optimization milestone that should be chosen from M17
> evidence. It should tune broadphase/query cost where counters and Criterion
> baselines show real pressure, rather than replacing the broadphase design or
> chasing micro-optimizations without proof.

### Background

M11 made broadphase proxy maintenance cheaper with direct handle-to-leaf lookup,
and M15 gave `QueryPipeline` an internal broadphase-style index. After M17, the
engine should have counter evidence for query traversal, candidate pruning,
dense/sparse broadphase behavior, and scene-level query-heavy workloads. M18
uses that evidence to improve the existing dynamic AABB tree path.

The design choice remains Box2D-style dynamic AABB tree first. M18 is not a
switch to Rapier's BVH or a public proxy-id API. It is a tuning pass on the
chosen substrate: insertion heuristics, balancing/rebuild policy, query
traversal cost, and debug/bench evidence.

### Goal

Reduce broadphase and query cost while preserving deterministic ordering and
public query semantics. The desired result is fewer unnecessary tree traversals,
fewer stale or imbalanced tree states, clearer rebuild/balance counters, and
benchmark evidence that explains the improvement.

### Design Goals

- Use M17 counters to pick the narrowest broadphase/query bottleneck first.
- Improve dynamic AABB tree insertion, balancing, rebuild, or traversal
  heuristics without changing public `QueryPipeline` hit semantics.
- Keep broadphase proxy/leaf ids private to `World` and internal pipeline code.
- Preserve deterministic candidate and hit ordering across rebuilds, removals,
  and recycled handles.
- Keep debug counters interpretable enough for `picea-lab` and future tuning
  sessions.
- Avoid hard timing thresholds until M17 baseline variance says they are safe.

### In Scope

- Dynamic AABB tree insertion/balancing/rebuild heuristics.
- Query traversal pruning and region/ray/AABB candidate efficiency.
- Broadphase/query counters and benchmark scenario updates when they explain a
  tuning choice.
- Regression tests for stale removal, recycled handles, deterministic ordering,
  and query filters.

### Out Of Scope

- Replacing the broadphase with a different BVH implementation.
- Public broadphase proxy ids or public tree mutation APIs.
- Public distance-query stabilization.
- Solver data-layout work; that belongs to M16/Post-M16 solver follow-up.
- CCD expansion or scene schema work.

### Acceptance Method

- Start from M17 evidence and document which counter/benchmark motivated the
  tuning slice.
- Add behavior locks before tuning if the target behavior is not already locked.
- Prove deterministic ordering survives tree balance/rebuild, removals, and
  recycled handles.
- Build Criterion benches and report counter/baseline movement without turning
  timing into a brittle pass/fail gate.

Suggested targeted gates:

```bash
rtk proxy cargo test -p picea --lib pipeline::broadphase
rtk proxy cargo test -p picea --test query_debug_contract
rtk proxy cargo test -p picea --test world_step_review_regressions
rtk proxy cargo bench -p picea --no-run
rtk proxy ruby -e 'require "yaml"; YAML.load_file("docs/ai/doc-catalog.yaml"); puts "yaml ok"'
rtk proxy git diff --check
```

### Completion Notes

- M17 evidence showed the small three-collider step still paying the old
  per-leaf root scan cost in `DynamicAabbTree::candidate_pairs_with_stats()`,
  with `broadphase_traversal_count=15` and `broadphase_pruned_count=4`
  candidate-pair traversal work units for a single surviving pair.
- M18 replaced that path with subtree-pair traversal from internal child pairs,
  so each overlapping node pair is considered once from its lowest common
  ancestor instead of rescanning the whole root per leaf.
- Public behavior stayed fixed: candidate output remains sorted by live collider
  snapshot index, query AABB/point/ray ordering and semantics stay unchanged,
  stale removal/recycled handles still rebuild correctly, and broadphase
  proxy/leaf ids remain private.
- The accepted small-scene lock now reports
  `broadphase_traversal_count=4` and `broadphase_pruned_count=2` for the same
  one-pair overlap case. This is evidence that the old per-leaf scan did more
  candidate-pair traversal work, while the new subtree-pair traversal reports
  fewer work units for the same scene; these counts are not literal tree-node
  visit totals.

## M19 CCD And Realism Expansion

> Status: completed selected CCD slice 2026-04-28.
>
> M19 is the first post-M17 physical-behavior expansion milestone. It should use
> performance evidence and behavior locks to choose the next CCD/realism slice,
> rather than attempting all-shape continuous collision or broad material
> realism in one pass.
>
> Completion notes: the selected M19 slice is translational dynamic-vs-dynamic
> convex CCD. The CCD phase now reduces two moving convex colliders to relative
> translational motion, selects hits through the existing swept convex-convex
> TOI path, and clamps both dynamic bodies to one traceable advancement before
> contact generation. The first slice intentionally skips the dynamic-dynamic
> path when either body rotates during the step, and it still excludes circles,
> rotational casts, and broad all-shape CCD.
>
> `CcdTrace` keeps the older `static_body` / `static_collider` field names for
> compatibility, but adds serde-defaulted `target_kind`, `target_swept_start`,
> `target_swept_end`, and `target_clamp` facts so artifacts can distinguish
> static targets from dynamic targets. `picea-lab` includes a
> `ccd_dynamic_convex_pair` artifact scenario, and Criterion now has a
> `ccd_dynamic_pair` bench ID so the new CCD cost is observable.
>
> Residual risk: rotational CCD, dynamic circle-vs-dynamic CCD, broader
> all-shape CCD, public scene schema, and broad material-system work remain
> staged follow-up work. Ramp-specific friction now has a Post-M22 regression
> lock.

### Background

M13 landed translational dynamic-vs-static convex CCD with multi-hit ordering
and budget traces. That removed a major tunneling gap without hiding substeps or
event semantics. The remaining CCD risks are harder: dynamic-vs-dynamic motion,
rotational casts, and broader all-shape coverage can all become expensive or
ambiguous without strong behavior locks and counter evidence.

Picea also still needs focused realism regressions such as ramp-specific
friction. These are visible to users and cheaper to prove than broad material
systems, so they should be included as targeted behavior locks rather than
rolled into an open-ended solver rewrite.

### Goal

Expand simulation realism in the smallest evidence-backed slice. The preferred
CCD order is dynamic-vs-dynamic translational convex first, then rotational or
broader all-shape coverage only if tests and M17/M18 counters justify the cost.
The milestone should also add focused ramp/friction regressions where current
behavior is intentionally narrow.

### Design Goals

- Keep CCD as an explicit named phase with traceable selected/ignored TOI facts.
- Add dynamic-vs-dynamic CCD only with narrow behavior locks, counters, and
  budget semantics.
- Preserve existing dynamic-vs-static CCD behavior and contact event semantics.
- Keep rotational/all-shape CCD staged behind tests and performance evidence.
- Add focused ramp/friction realism tests without broad material-system churn.
- Keep `picea-lab` artifacts able to explain swept paths, TOI selection, and
  ignored/budgeted impacts.

### In Scope

- One evidence-backed CCD expansion slice, preferably translational
  dynamic-vs-dynamic convex CCD.
- No-false-positive, ordering, budget, and event/debug trace tests for the new
  CCD slice.
- Ramp-specific friction regression coverage if it can be locked without
  changing public API.
- Lab artifact updates for new CCD trace facts if the trace surface changes.

### Out Of Scope

- Full all-shape CCD in one milestone.
- Rotational CCD unless the selected M19 slice explicitly proves it is the
  smallest safe next step.
- Hidden unbounded substeps.
- Broad material-system redesign.
- Public scene schema or authoring UX work.

### Acceptance Method

- Add known-red tests for the selected CCD or realism gap before implementation.
- Prove selected/ignored TOI ordering and budget semantics remain deterministic.
- Keep existing M13 dynamic-vs-static CCD tests green.
- Use lab artifacts or trace snapshots to show the new CCD/realism facts.
- Build Criterion benches so the new CCD cost is at least observable.

Suggested targeted gates:

```bash
rtk proxy cargo test -p picea --test physics_realism_acceptance ccd
rtk proxy cargo test -p picea --test physics_realism_acceptance friction
rtk proxy cargo test -p picea --lib pipeline::ccd
rtk proxy cargo test -p picea-lab
rtk proxy cargo bench -p picea --no-run
rtk proxy ruby -e 'require "yaml"; YAML.load_file("docs/ai/doc-catalog.yaml"); puts "yaml ok"'
rtk proxy git diff --check
```

## M20 Scene Schema And Authoring UX

> Status: completed 2026-04-28.
>
> M20 is the next ease-of-use milestone. It should turn the accepted M14
> recipe/fixture surface into a clearer scene-authoring contract without
> weakening the low-level `World::create_*` APIs.

### Background

Picea should not compete only by matching physics algorithms. A major advantage
should be that users can create repeatable worlds, inspect failures, and share
examples without hand-threading every body, collider, material, collision layer,
and validation path. M14 added scene/asset recipe helpers, nested error paths,
and serializable lab fixture flows; M20 stabilizes the next public authoring
layer around that work.

This milestone should happen after M17-M19 because schema and UX choices should
be informed by real benchmark/artifact scenarios and by the CCD/realism facts
that users need to author and inspect.

### Goal

Stabilize a versioned scene schema and authoring workflow for examples,
benchmarks, and `picea-lab` scenarios. The target is a friendlier public setup
path that remains reproducible, validated, and easy to debug, while preserving
the low-level `World` lifecycle APIs for advanced users.

### Design Goals

- Keep low-level `World::create_body` / `create_collider` / `create_joint`
  stable as the control surface.
- Make scene schema additions versioned, additive, and explicit about defaults.
- Preserve nested validation paths for body/collider/joint/material/layer
  failures.
- Let examples, benches, and lab scenarios share the same authoring model where
  that reduces duplication.
- Define live `picea-lab` editing semantics separately from static scene
  loading; do not imply hot mutation behavior the engine does not support.
- Keep generated artifacts reproducible enough for AI and human debugging.

### In Scope

- Versioned public scene schema for accepted recipe/asset concepts.
- Serializable examples or fixtures that exercise common body/collider/joint
  setup without hiding handles from advanced users.
- Schema validation tests, nested error-path tests, and backward-compatible
  fixture loading where feasible.
- `picea-lab` scenario loading improvements that consume the stabilized schema.
- Documentation examples that show both high-level scene authoring and low-level
  `World` fallback.

### Out Of Scope

- Replacing low-level `World::create_*` APIs.
- Full live editing of an actively stepping world unless a separate design locks
  reset/patch/transaction semantics first.
- Physics solver, broadphase, CCD, or material-model changes.
- A large visual editor; `picea-lab` remains an inspection and replay tool unless
  a later product milestone says otherwise.

### Acceptance Method

- Add schema/fixture tests before implementation where the desired authoring
  behavior is new.
- Prove old accepted fixtures still load or fail with clear versioned errors.
- Add `v1_api_smoke` coverage showing low-level APIs remain stable.
- Add or update at least one lab/example fixture that uses the stabilized schema.
- Update AI routing and docs so future sessions know whether a task belongs to
  schema authoring, lab scenario loading, or low-level `World` lifecycle.

Suggested targeted gates:

```bash
rtk proxy cargo test -p picea --test v1_api_smoke
rtk proxy cargo test -p picea --test core_model_world
rtk proxy cargo test -p picea-lab
rtk proxy cargo bench -p picea --no-run
rtk proxy ruby -e 'require "yaml"; YAML.load_file("docs/ai/doc-catalog.yaml"); puts "yaml ok"'
rtk proxy git diff --check
```

### 2026-04-28 Narrow Slice Notes

- `picea-lab` scene fixtures now treat `SCENE_RECIPE_SCHEMA_VERSION = 1` as the
  stable v1 authoring contract; legacy accepted JSON that omits
  `schema_version` still deserializes as v1.
- Unsupported non-v1 scene schema versions fail before world instantiation with
  a dedicated scene-schema error instead of falling through to lower-level world
  setup.
- Scene fixtures can now author recipe-indexed `distance` and `world_anchor`
  joints with additive optional fields (`rest_length`, anchors, `stiffness`,
  `damping`) while still delegating handle resolution and nested path reporting
  to `WorldRecipe::with_joint`.
- Verification for this slice uses `v1_api_smoke`, `core_model_world`,
  `picea-lab`, bench build, AI YAML parse, formatting, and diff hygiene gates.

## M21 Public Distance And Shape Query API

> Status: completed 2026-04-28; Plan Gate accepted.
>
> Completion notes: M21 adds the public `QueryShape`, `ShapeHit`, and
> `QueryShapeError` surface plus `QueryPipeline::intersect_shape` /
> `QueryPipeline::closest_shape`. The API returns semantic handles, distances,
> witness points, optional normals, and existing `QueryStats` counters without
> exposing broadphase proxy/leaf ids. The accepted slice covers circle,
> convex-polygon/rect, and segment query shapes, rejects direct concave query
> input, preserves AABB/point/ray semantics, and keeps query behavior locked for
> ordering, filters, stale sync, recycled handles, degenerate input, no-hit
> cases, and capsule snapshot radius facts.

### Background

After M20, users can author repeatable scenes, but inspection and gameplay-style
logic still need a stable way to ask geometric questions beyond AABB, point, and
ray hits. Internally, Picea already has ordered `QueryPipeline` traversal,
filtering, query stats, cached collider geometry, and GJK distance/fallback
kernels. The remaining gap is an ergonomic public contract for distance and
shape queries that does not leak broadphase proxy ids or mutate the `World`.

This milestone is also part of Picea's ease-of-use advantage: users should be
able to write "how far is this shape from the world?" code without rebuilding
engine internals in application space.

### Goal

Expose a deterministic, filterable public distance/shape-query API that returns
closest collider facts in a stable form, reuses the existing indexed query/cache
path, and gives enough debug/stat evidence to prove ordering, filtering, stale
sync, and handle-reuse behavior.

### Design Goals

- Build on `QueryPipeline`, existing collider geometry caches, and the internal
  GJK distance kernel instead of creating a second query engine.
- Return semantic handles and geometric facts only: collider/body handles,
  distance, closest points, normal/direction where well-defined, and query
  statistics. Do not expose broadphase proxy/leaf ids.
- Keep hit ordering deterministic across equivalent worlds and recycled handles.
- Make filters explicit and consistent with existing AABB/point/ray queries.
- Treat unsupported shapes or degenerate inputs as clear validation results, not
  panics or silent "no hit" outcomes.
- Keep concave decomposition and compound authoring for M22; M21 handles the
  supported convex/circle/segment/rectangle/polygon query surface.

### In Scope

- Public API types for distance/shape query results, filters, and options.
- Distance or closest-hit queries against supported existing collider shapes.
- Tests for ordering, filtering, stale pipeline sync, recycled handles,
  degenerate input, and no-hit cases.
- `QueryStats` or debug fact updates only where they explain the new query path.
- Documentation examples that show high-level query usage without bypassing
  `World`/`QueryPipeline` ownership.

### Out Of Scope

- World mutation, live editing, or automatic collider creation from query input.
- Concave polygon decomposition, compound scene authoring, or direct concave
  solver support.
- CCD, solver, material model, or broadphase balancing changes.
- Public exposure of internal proxy ids, leaf ids, cache revisions, or tree
  implementation details.
- Absolute performance thresholds; use Criterion and counters as evidence first.

### Acceptance Method

- Add behavior locks before implementation for the public distance/shape-query
  contract.
- Prove query results are deterministic, filter-aware, and stable after body
  transform patches and handle recycling.
- Prove the query path reuses the existing query/cache infrastructure instead of
  scanning all colliders in the accepted scenarios.
- Keep existing AABB/point/ray query behavior unchanged.
- Update AI routing so future tasks know public distance query belongs to M21,
  while concave/compound authoring belongs to M22.

Suggested targeted gates:

```bash
rtk proxy cargo test -p picea --test query_debug_contract
rtk proxy cargo test -p picea --test world_step_review_regressions
rtk proxy cargo test -p picea --lib pipeline::gjk
rtk proxy cargo test -p picea --test v1_api_smoke
rtk proxy cargo bench -p picea --no-run
rtk proxy ruby -e 'require "yaml"; YAML.load_file("docs/ai/doc-catalog.yaml"); puts "yaml ok"'
rtk proxy git diff --check
```

### Subagent Execution Plan

- `explorer` (leaf agent, no subagents): inspect `query.rs`, `pipeline/gjk.rs`,
  collider shape support, and existing query tests; report the smallest public
  API surface and behavior-lock locations.
- `worker` (leaf agent, no subagents; prefer `gpt-5.4` unless API risk calls for
  higher quality): implement the accepted API slice and tests in the query/API
  ownership files only.
- `reviewer` (leaf agent, no subagents): review public API compatibility,
  determinism, stale sync, filtering, and proxy-id leakage risk.
- `verifier` (leaf agent, no subagents): run the targeted gates and report any
  generated or modified files.

## M22 Compound And Concave Authoring Boundary

> Status: completed 2026-04-28; Plan Gate accepted.
>
> Completion notes: M22 lands the additive lab scene-fixture authoring boundary:
> `compound` means one body with ordered validated convex collider pieces
> (`circle`, `rect`, `convex_polygon`) and optional piece `local_pose`.
> Generated pieces inherit body-level material, filter, density, and sensor
> semantics, while existing low-level `World`/`Collider` lifecycle APIs stay
> unchanged. Direct `concave_polygon`, top-level/piece concave
> `convex_polygon`, zero-length polygon edges, empty compounds, invalid piece
> radii/sizes, and invalid piece local poses now fail before world
> instantiation with stable `scene.bodies[..].shape...` paths. M20 v1 fixture
> compatibility remains intact. Artifact/UI provenance, automatic polygon
> decomposition, per-piece material overrides, and broader dynamic concave
> support remain Post-M22 follow-up work; additive compound-piece mass/inertia
> now has a Post-M22 behavior lock.

### Background

Picea's narrowphase direction remains convex-first: SAT + clipping for supported
convex manifolds, analytic simple-shape paths, and GJK/EPA fallback for generic
convex cases. That is the right core-solver boundary. The product gap after M20
is different: users still need to express common concave-looking objects,
terrain pieces, sensor areas, or compound obstacles without hand-authoring a
fragile pile of low-level colliders.

M22 should define the safe authoring contract: concave input is either rejected
with a clear error or represented as validated compound convex pieces with
traceable provenance.

### Goal

Provide a clear compound/concave authoring boundary for scene recipes and lab
fixtures: supported compound convex shapes should be easy to author and inspect,
while unsupported direct concave solver usage should fail early with stable
errors before world mutation.

### Design Goals

- Preserve the core solver's convex-contact contract; do not add direct concave
  contact solving in this milestone.
- Represent concave-looking objects as compound convex pieces or validated
  pre-decomposed fixtures with explicit provenance.
- Keep body/collider handles, material, sensor, and collision-filter semantics
  understandable at the authored object and generated-piece levels.
- Prefer static or explicitly constrained compound authoring first; dynamic
  concave mass/inertia behavior requires a separate behavior lock before support.
- Use `picea-lab` fixture evidence to explain generated child pieces and
  validation failures in this slice; richer artifact/schema/UI provenance is a
  Post-M22 follow-up.
- Keep the schema additive and versioned so M20 fixtures remain compatible.

### In Scope

- Scene/recipe authoring support for compound convex collider groups or
  validated pre-decomposed concave fixtures.
- Validation and nested error paths for unsupported direct concave shapes,
  invalid pieces, and empty decomposition.
- Tests proving generated pieces preserve filters, materials, sensors, and
  deterministic ordering.
- At least one lab/example fixture behavior lock that demonstrates compound or
  concave-looking authoring boundaries.
- Documentation that states the solver boundary in user terms: convex pieces are
  supported; arbitrary concave contact solving is not.

### Out Of Scope

- Direct arbitrary concave-vs-concave or concave-vs-convex contact solving in the
  core narrowphase.
- A broad polygon-decomposition algorithm unless the selected implementation
  slice first locks its input limits, determinism, and failure modes.
- Dynamic concave mass/inertia aggregation without explicit tests and acceptance.
- Visual editor work, live scene patching, or hidden world mutation semantics.
- CCD expansion for compound shapes beyond whatever existing per-piece collider
  behavior already guarantees.

### Acceptance Method

- Add behavior locks for accepted compound/concave authoring examples before
  implementation.
- Prove unsupported direct concave authoring fails before world instantiation or
  mutation, with nested path errors.
- Prove generated compound pieces preserve material/filter/sensor semantics and
  deterministic ordering.
- Prove existing M20 v1 scene fixtures remain compatible.
- Add lab artifact or fixture evidence showing generated piece provenance.
- Update AI routing so future tasks do not confuse M22 authoring support with
  direct core-solver concave support.

Suggested targeted gates:

```bash
rtk proxy cargo test -p picea --test core_model_world
rtk proxy cargo test -p picea --test v1_api_smoke
rtk proxy cargo test -p picea --test query_debug_contract
rtk proxy cargo test -p picea-lab
rtk proxy cargo bench -p picea --no-run
rtk proxy ruby -e 'require "yaml"; YAML.load_file("docs/ai/doc-catalog.yaml"); puts "yaml ok"'
rtk proxy git diff --check
```

### Subagent Execution Plan

- `explorer` (leaf agent, no subagents): inspect `recipe.rs`,
  `picea-lab/src/scenario.rs`, collider shape representation, and current
  schema tests; report the smallest additive authoring surface.
- `worker` (leaf agent, no subagents; prefer `gpt-5.4` unless schema/API risk
  calls for higher quality): implement the accepted M22 slice in recipe/scene
  files and focused tests only.
- `reviewer` (leaf agent, no subagents): review schema compatibility, solver
  boundary wording, error paths, deterministic ordering, and accidental direct
  concave-solver leakage.
- `verifier` (leaf agent, no subagents): run the targeted gates and report any
  generated or modified files.

## M23：Performance Hardening / Broadphase + Query Cost Gate

状态：已完成；Plan Gate 已确认 2026-04-29；Supervisor Acceptance 通过 2026-04-29。

目标：
让 M21 之后的 public query surface 既好用又可解释。M23 完成后，
`closest_shape`、`intersect_shape`、ray / AABB / point query 和 broadphase
candidate-pair traversal 的成本应该能通过 counters、debug facts 和 Criterion
baseline 解释；dynamic AABB tree 的插入、平衡、rebuild 策略应该有第一轮
行为锁和调优证据。

为什么现在做：
M21/M22 已经把易用 API 和 compound authoring 打开了。先做 M23 可以避免后续
CCD、live editing、automatic decomposition 把更多复杂场景压到尚未硬化的
query / broadphase 路径上。

执行拆分：
- M23-A 行为锁和事实盘点：先锁定 `closest_shape` / `intersect_shape` / ray /
  AABB / point query 的 ordering、filter、stale sync、recycled handle 和
  `last_stats()` reset/coherence；同时列出 M24 需要但当前 artifact/debug facts
  尚未承载的 tree/traversal 信息。
- M23-B Broadphase tree cost evidence：只在 M23-A 明确缺口后改 dynamic AABB
  tree 的插入、move、rebuild 或 debug carrier；每个调优都必须用 deterministic
  counter 或行为锁解释，不能用单次 wall-clock 作为结论。
- M23-C Benchmark / routing closeout：刷新或补齐 Criterion scenario labels 和
  counter summaries，更新计划/AI 路由中 M23 的完成证据；仍不设置 pass/fail
  性能阈值。

范围：
- 改进 dynamic AABB tree 插入、平衡、move / rebuild 策略，保持 public
  ordering 和 proxy id 私有性。
- 增加 query allocation / perf counters，覆盖 `closest_shape`、
  `intersect_shape`、ray、AABB、point query。
- 补充 M24 可视化需要消费的 broadphase tree / traversal facts，例如 tree depth、
  node AABB、leaf collider link、candidate traversal / prune reason；这些事实
  仍然不成为 public broadphase mutation API。
- 用 Criterion 多轮 baseline 解释 query-heavy、broadphase-heavy、compound
  authoring 后的成本形状。
- 补行为锁：candidate ordering、filter semantics、stale sync、recycled
  handles、tree rebuild、counter reset behavior。

不做：
- 不公开 broadphase proxy / leaf id。
- 不设置硬性能阈值；M29 才讨论 threshold gate。
- 不做 CCD expansion、concave solver、live scene editing。
- 不改 `World::create_*` 或 M21 public query 的语义。

所有权：
- 可能触及：`crates/picea/src/pipeline/broadphase.rs`、
  `crates/picea/src/query.rs`、`crates/picea/src/debug.rs`、
  `crates/picea/src/pipeline/step.rs`、`crates/picea/tests/query_debug_contract.rs`、
  `crates/picea/tests/world_step_review_regressions.rs`、
  `crates/picea/benches/physics_scenarios.rs`。
- 不应触及：`crates/picea/src/pipeline/ccd.rs` 的 TOI 语义、
  `crates/picea-lab/web` UI、public scene patch/live editing API。

验收标准：
- Query / broadphase counters 能解释新增和既有 query path 的主要工作量。
- Dynamic AABB tree 调整不改变 public hit ordering、filter semantics 或
  recycled-handle 行为。
- Criterion baseline 能展示成本形状，不把 wall-clock timing 当正确性 oracle。
- Existing M21 query tests 和 M18 broadphase tests 保持绿色。

验证方式：
- `rtk proxy cargo test -p picea --lib pipeline::broadphase`
- `rtk proxy cargo test -p picea --test query_debug_contract`
- `rtk proxy cargo test -p picea --test world_step_review_regressions`
- `rtk proxy cargo bench -p picea --no-run`
- `rtk proxy ruby -e 'require "yaml"; YAML.load_file("docs/ai/doc-catalog.yaml"); puts "yaml ok"'`
- `rtk proxy git diff --check`

Subagent 执行计划：
- explorer：叶子 agent；回答 broadphase/query counter 缺口、tree heuristics
  风险、M24 可消费 facts 缺口和最小行为锁位置。
- worker：叶子 agent；优先 `gpt-5.4`，只拥有 broadphase/query/debug/bench
  相关文件，按 M23-A -> M23-B -> M23-C 顺序推进，先补测试再做最小调优。
- reviewer：叶子 agent；重点审查 ordering、filter、stale sync、proxy id 泄漏、
  counter 语义和 benchmark 结论是否过度。
- verifier：叶子 agent；运行 M23 验证方式并报告验证过程中产生或修改的文件。

提交策略：
- auto-commit：否。
- message hint：`Add M23 performance hardening gate`。

风险 / 后续：
- Tree heuristic 容易过拟合当前 fixture；必须用 counters 和多场景 baseline
  解释，不用单一场景定结论。
- 如果 M23-A 发现需要公开新的 query 或 debug contract，必须先暂停确认 API
  边界；默认只加内部/serde-default debug facts。

进度记录：
- 2026-04-29：用户确认使用 `subagent-current-workspace` 依次执行 M23 -> M24；
  当前主 Codex 作为 supervisor，M23 先进入 M23-A 探索/行为锁执行。M24 完成后
  必须使用 `browser-use:browser` 对 `picea-lab-web` 进行最终可视验收。
- 2026-04-29：M23 完成。补齐 ray / AABB query 的 sensor/filter、stale sync、
  recycled handle、`last_stats()` reset 行为锁；`DebugSnapshot` 增加 additive /
  serde-default 的 `broadphase_tree` 读模型，导出 tree depth、node AABB、leaf
  collider link，并保持 broadphase proxy / leaf id 私有。`for_query()` 路径跳过
  M24 可视化 tree carrier，避免 query cache rebuild 先建后丢。
- 2026-04-29：M23 reviewer 发现的 prelude surface 和 transient node id 测试
  过拟合已修正；最终 verification 通过：
  `rtk proxy cargo test -p picea --lib pipeline::broadphase`、`rtk proxy cargo test -p picea --test query_debug_contract`、
  `rtk proxy cargo test -p picea --test world_step_review_regressions`、
  `rtk proxy cargo bench -p picea --no-run`、`rtk proxy ruby -e 'require "yaml"; YAML.load_file("docs/ai/doc-catalog.yaml"); puts "yaml ok"'`、
  `rtk proxy git diff --check`。

## M24：Lab-Web Visualization Foundation / Tree + Island + Provenance

状态：已完成；Plan Gate 已确认 2026-04-29；Supervisor Acceptance 通过 2026-04-29。

目标：
把 `picea-lab-web` 从“能看 replay / inspector”推进到“能解释物理过程”的
可视化工作台。M24 完成后，用户应该能在同一个案例中看到 broadphase tree
结构与遍历过程、island 形成与 sleep/wake 过程、以及 M22 compound provenance；
这些可视化必须来自 core / artifact facts，`picea-lab-web` 不重新计算物理。

为什么现在做：
M23 会补 query / broadphase 成本和 tree traversal 事实，M16/M17 已经有 island
和 solver row 事实，M22 已经定义 compound authoring boundary。下一步应该把这些
事实组织成过程型可视化，而不是继续只堆表格字段。这样后续 M26 CCD、M27
decomposition、M28 solver ordering 都能复用同一套 viewer 架构。

前置条件：
- M23 至少要完成 M23-A，并明确哪些 broadphase/query facts 已经稳定到 artifact
  或 debug carrier；否则 M24 只能先做 artifact/schema 盘点，不能先写 UI。
- 如果 M24 需要 core 侧新增 facts，必须保持 additive / serde default，并证明旧
  artifact、旧 scene fixture 和现有 web demo 数据仍能加载。

执行拆分：
- M24-A Artifact/schema foundation：先补 deterministic scenarios、debug render /
  final snapshot / perf artifact 中的 tree、island、compound provenance carriers，
  并用 serde-default tests 锁住旧 artifact 兼容。
- M24-B Web process panels：在现有 `WorkbenchLayout`、`SceneHierarchy`、
  `Inspector`、`WorldCanvas`、`Facts` 等边界内加过程视图和 overlay，不重写
  workbench 结构，不引入 live editor 语义。
- M24-C Contract / docs closeout：补 UI contract、i18n contract、web build 和
  路由文档证据，确保 broadphase tree / island lifecycle / compound provenance
  的入口可从 scenario gallery 到达。

范围：
- 建立第一版 lab-web 可视化面板结构：scenario gallery、timeline、overlay
  toggles、entity inspector、process facts panel。
- Broadphase tree 可视化：tree node AABB、leaf collider、tree depth、fat AABB、
  rebuild / move facts、candidate traversal 和 prune reason。
- Island 可视化：body/contact/joint graph、island id、awake/sleep state、
  wake reason、active solver rows、body slots、contact rows、joint rows。
- Compound provenance 可视化：authored object、generated convex pieces、piece
  ordering、material/filter/sensor 继承关系和 validation failure path。
- 增加第一批确定性案例：sparse/dense broadphase tree、query closest-shape、
  sleeping island split/wake、stack island solve、compound fixture。
- 扩展 artifact/debug-render/final snapshot 中缺失的可视化 facts；仅记录 core
  或 scene loader 已经确定的事实。
- 保持 M20 v1 scene fixture 兼容；新增字段必须 additive / serde default。

不做：
- 不让 lab-web 重新执行 broadphase traversal、island grouping、decomposition、
  mass/inertia 或 collision 计算。
- 不改变 core solver、narrowphase 或 contact manifold。
- 不做 visual editor 或 live patch。
- 不做完整 CCD visualization pack；M26 只为选中的 CCD slice 增加对应 overlay。
- 不做自动 polygon decomposition；M27 单独处理。
- 不做 per-piece material/filter override，除非后续 milestone 单独批准。

所有权：
- 可能触及：`crates/picea-lab/src/scenario.rs`、
  `crates/picea-lab/src/artifact.rs`、`crates/picea-lab/tests/artifact_run.rs`、
  `crates/picea-lab/web/src/*`、artifact schema docs；若 facts 缺失，可小范围
  触及 `crates/picea/src/debug.rs`、`crates/picea/src/pipeline/broadphase.rs`、
  `crates/picea/src/pipeline/island.rs` 的 debug carrier。
- 不应触及：solver row math、CCD algorithms、low-level `World::create_*` API。

验收标准：
- Lab-web 至少提供 broadphase tree、island graph / lifecycle、compound
  provenance 三类过程型视图。
- 每个可视化点都能追溯到 artifact/core facts；web 不制造新的物理判断。
- Scenario gallery 能从内置案例进入对应可视化，不暴露 `target/...` 作为主要
  用户心智模型。
- UI/schema 展示继承关系、piece ordering、tree traversal 和 island state，不
  混淆 debug facts 与 solver 结果。
- 旧 artifact / fixture 仍能加载或用清晰 serde default 兼容。
- M22 direct concave rejection 的错误路径仍然稳定。

验证方式：
- `rtk proxy cargo test -p picea-lab`
- `rtk proxy cargo test -p picea-lab --test artifact_run`
- `rtk proxy cargo test -p picea --test core_model_world`
- `rtk proxy cargo test -p picea --test world_step_review_regressions`
- `cd crates/picea-lab/web && npm run build`
- `cd crates/picea-lab/web && npm run test:ui-contract`
- `cd crates/picea-lab/web && npm run test:i18n`
- `rtk proxy git diff --check`

Subagent 执行计划：
- explorer：叶子 agent；梳理 artifact schema、web panel ownership、broadphase
  tree facts、island facts、M22 fixture provenance 缺口和旧 artifact 兼容风险。
- worker：叶子 agent；优先 `gpt-5.4`，负责 lab artifact/schema/UI 的 additive
  visualization foundation，按 M24-A -> M24-B -> M24-C 顺序推进，避免改 physics
  algorithms。
- reviewer：叶子 agent；重点审查 schema backward compatibility、web 是否重新
  计算物理事实、tree/island 标签是否误导用户、案例是否足够确定性。
- verifier：叶子 agent；运行 Rust lab tests、web build、UI contract 和 i18n
  contract。

提交策略：
- auto-commit：否。
- message hint：`Add M24 lab web visualization foundation`。

风险 / 后续：
- Artifact schema 一旦公开就会被依赖；字段命名和 default 行为要保守。
- Process visualization 很容易膨胀成编辑器；M24 只做解释和 replay，不做 live
  mutation。
- 如果 M24-A 发现 core facts 不足，不允许在 web 侧补算 broadphase/island/
  decomposition 结论；应退回 core/debug carrier 的最小 additive 变更。

最终验收补充：
- Rust / web / contract gates 全部通过后，必须启动本地 `picea-lab-web`，再用
  `browser-use:browser` 打开本地页面，实际检查 scenario gallery、process facts
  panel / overlays、broadphase tree、island lifecycle 和 compound provenance 入口
  是否可见且不需要用户理解 `target/...` 路径。

进度记录：
- 2026-04-29：M24 完成。新增 lab-owned `compound_provenance` artifact carrier
  和 `compound_provenance` 内置场景；`frames.jsonl` / `debug_render.json`
  保留 provenance，`debug_render` 携带 M23 `broadphase_tree`，新增字段均为
  additive / serde-default。Web 在既有 Inspector / layer menu / canvas 边界内展示
  process facts panel、broadphase tree overlay、island overlay、provenance overlay；
  可视化只消费 core / artifact facts，不重新计算 broadphase、island、decomposition、
  mass/inertia 或 collision。
- 2026-04-29：M24 verification 通过：
  `rtk proxy cargo test -p picea-lab --test artifact_run`、`rtk proxy cargo test -p picea-lab`、
  `rtk proxy cargo test -p picea --test core_model_world`、
  `rtk proxy cargo test -p picea --test world_step_review_regressions`、
  `cd crates/picea-lab/web && npm run build`、`cd crates/picea-lab/web && npm run test:ui-contract`、
  `cd crates/picea-lab/web && npm run test:i18n`、`rtk proxy git diff --check`。
- 2026-04-29：按最终验收补充启动 `picea-lab serve --bind 127.0.0.1:18080`
  和 `picea-lab-web` Vite dev server，并用 `browser-use:browser` 打开
  `http://127.0.0.1:5174/`。真实 Rust 回放中可从 scenario gallery 选择
  `compound_provenance`，process facts panel 显示 `7 nodes / 4 leaves / depth 4`、
  island lifecycle 和 3 个 generated pieces；layer menu 中 `宽阶段树` / `岛` /
  `来源` overlay 可打开，浏览器 console 无 warning/error。

## M25：Live Scene Patch Semantics

状态：计划中。

目标：
在实现 live editing 前先固定语义。M25 完成后，文档和行为锁应该明确 reset、
patch、transaction、handle invalidation、query pipeline sync、artifact
provenance 的边界；必要时只落最小静态 patch contract，不做完整 editor。

为什么现在做：
M20/M22 都是静态 scene loading。直接在 running `World` 上热改会影响 handle
生命周期、contact cache、sleep island、query sync 和 artifact reproducibility。

范围：
- 设计 live scene patch 的 reset / rebuild / transactional apply 选项。
- 明确哪些 patch 会保留 handles，哪些必须 invalidate / rebuild。
- 为 query pipeline sync、contact cache 清理、sleep wake reason 增加行为锁。
- 只实现最小可验证 patch path，或在探索后仅提交设计文档和红线测试。

不做：
- 不做完整视觉编辑器。
- 不承诺任意运行中 world mutation 都保持 handle 稳定。
- 不把 `WorldCommands` 变成高频 runtime mutation API。
- 不做 networking / collaborative editing。

所有权：
- 可能触及：`docs/design/*`、`crates/picea/src/recipe.rs`、
  `crates/picea/src/world/api.rs`、`crates/picea-lab/src/scenario.rs`、
  `crates/picea-lab/src/server.rs`、相关 tests。
- 不应触及：solver row math、CCD algorithms、broadphase heuristics。

验收标准：
- 文档能回答 reset vs patch vs transaction 的语义差异。
- 行为锁覆盖 handle invalidation、query sync、contact/sleep cleanup 的关键边界。
- 旧 static scene fixture path 不变。
- 如果实现最小 patch，失败模式必须是显式错误，不允许静默半更新。

验证方式：
- `rtk proxy cargo test -p picea --test v1_api_smoke`
- `rtk proxy cargo test -p picea --test core_model_world`
- `rtk proxy cargo test -p picea --test world_step_review_regressions`
- `rtk proxy cargo test -p picea-lab`
- `rtk proxy git diff --check`

Subagent 执行计划：
- explorer：叶子 agent；先给出 handle/contact/query/sleep 语义风险清单和最小
  design surface。
- worker：叶子 agent；若计划批准实现，仅落最小设计/测试/patch contract。
- reviewer：叶子 agent；重点审查 public API、兼容性、artifact reproducibility
  和隐式 world mutation。
- verifier：叶子 agent；运行 M25 验证方式。

提交策略：
- auto-commit：否。
- message hint：`Define M25 live scene patch semantics`。

风险 / 后续：
- 如果 patch 语义影响 public API 或 handle lifecycle，必须暂停让用户确认。

## M26：CCD Expansion Slice

状态：计划中。

目标：
选择 dynamic compound CCD 或 rotational CCD 中最小、最可验收的一片。M26
完成后，选中的 CCD slice 应该有 TOI ordering、budget、false-positive /
false-negative 行为锁和 trace facts；未选方向继续留在后续。

为什么现在做：
M19 已完成 translational dynamic-vs-dynamic convex CCD。M22 引入 compound
authoring 后，CCD 的下一步必须更谨慎：compound 和 rotation 都会显著增加
TOI 候选、排序和预算复杂度。

范围：
- 先探索 dynamic compound CCD vs rotational CCD 的最小可验收切片。
- 只选择一个方向实现；另一个方向记录为 residual risk。
- 为 selected/ignored TOI、budget clamp、no false positive、missed impact
  增加行为锁。
- 扩展 `CcdTrace` / debug facts 仅限解释选中 slice。

不做：
- 不做全 shape CCD。
- 不做 arbitrary concave CCD。
- 不做 hidden unbounded substeps。
- 不改 public scene authoring 或 query API。

所有权：
- 可能触及：`crates/picea/src/pipeline/ccd.rs`、
  `crates/picea/src/pipeline/gjk.rs`、`crates/picea/src/debug.rs`、
  `crates/picea/tests/physics_realism_acceptance.rs`、
  `crates/picea-lab/src/scenario.rs`、CCD bench scenarios。
- 不应触及：broadphase balancing、scene patch semantics、automatic polygon
  decomposition。

验收标准：
- 明确记录为什么选择 dynamic compound 或 rotational 作为 M26 slice。
- TOI ordering、budget 和 trace facts 对选中场景可观察。
- 已有 M13/M19 CCD 行为保持绿色。
- 未覆盖方向在文档中保留为后续，不伪装成已完成。

验证方式：
- `rtk proxy cargo test -p picea --test physics_realism_acceptance ccd`
- `rtk proxy cargo test -p picea --lib pipeline::ccd`
- `rtk proxy cargo test -p picea --lib pipeline::gjk`
- `rtk proxy cargo test -p picea-lab --test artifact_run`
- `rtk proxy cargo bench -p picea --no-run`
- `rtk proxy git diff --check`

Subagent 执行计划：
- explorer：叶子 agent；比较 dynamic compound CCD 与 rotational CCD 的当前
  数据结构成本、测试可锁定性和最小切片。
- worker：叶子 agent；高风险 physics slice，优先继承高质量模型或显式
  `gpt-5.5`，只实现获批方向。
- reviewer：叶子 agent；重点审查 TOI ordering、budget、false positive/negative、
  trace 解释和未选方向边界。
- verifier：叶子 agent；运行 CCD / lab / bench no-run gates。

提交策略：
- auto-commit：否。
- message hint：`Add M26 CCD expansion slice`。

风险 / 后续：
- 如果两个方向都无法形成小 slice，停止并把 M26 改为 design-only milestone。

## M27：Automatic Polygon Decomposition

状态：计划中。

目标：
把 M22 的“手写 compound pieces”推进到受限自动 polygon decomposition。
M27 完成后，静态或受限 authoring path 可以把合规 concave polygon 转换为
deterministic convex pieces；core solver 仍然不直接求解 arbitrary concave
contact。

为什么现在做：
只有在 M24 visualization/provenance foundation 和 M25 patch semantics 明确后，
自动分解才不会变成不可解释的黑箱。M27 应该提升用户易用性，但保持 solver
boundary 清楚。

范围：
- 探索并选择一个 deterministic decomposition strategy 或受限输入策略。
- 支持静态 / authoring-time decomposition 的最小场景。
- 锁定 invalid polygon、self-intersection、zero-area、winding、piece ordering、
  material/filter/sensor inheritance。
- 让 generated pieces 进入 M24 visualization/provenance surface。

不做：
- 不做 direct concave solver。
- 不承诺任意复杂 polygon 都能自动分解。
- 不做 dynamic concave mass/inertia 的广义支持。
- 不做 runtime high-frequency decomposition。

所有权：
- 可能触及：`crates/picea/src/collider.rs`、`crates/picea/src/recipe.rs`、
  `crates/picea-lab/src/scenario.rs`、`crates/picea-lab/src/artifact.rs`、
  decomposition tests / fixtures。
- 不应触及：contact solver row math、CCD expansion、broadphase tree heuristics。

验收标准：
- 合规输入 deterministic 地生成 convex pieces。
- 非合规输入以稳定 nested error path 失败。
- Piece ordering、继承关系和 provenance 可观察。
- Core solver boundary 文档仍声明不支持 arbitrary concave contact。

验证方式：
- `rtk proxy cargo test -p picea --test core_model_world`
- `rtk proxy cargo test -p picea --test v1_api_smoke`
- `rtk proxy cargo test -p picea-lab`
- `rtk proxy cargo test -p picea-lab --test artifact_run`
- `rtk proxy git diff --check`

Subagent 执行计划：
- explorer：叶子 agent；调研当前 shape/recipe/lab constraints，提出最小可确定
  decomposition strategy 和拒绝策略。
- worker：叶子 agent；高风险 authoring/geometry slice，优先 `gpt-5.4` 或更高，
  先测试后实现。
- reviewer：叶子 agent；重点审查 determinism、degenerate geometry、错误路径、
  solver boundary 泄漏。
- verifier：叶子 agent；运行 core/lab/artifact gates。

提交策略：
- auto-commit：否。
- message hint：`Add M27 polygon decomposition milestone`。

风险 / 后续：
- 自动分解算法选择可能需要更长 design review；如果无法小步落地，应降级为受限
  authoring helper。

## M28：Solver Ordering / Island Contract

状态：计划中。

目标：
评估并收敛 contact/joint 是否应该共享更强的 island-owned ordering contract。
M28 完成后，solver ordering、debug facts 和未来 parallel island solver 的边界
应该更清楚，但不要求本 milestone 引入多线程。

为什么现在做：
M16 已经把 active island solver 收敛到 dense island-local slots。后续如果要做
更稳定 joint/contact coupling 或并行 island solving，需要先把 ordering contract
写清楚并锁住行为。

范围：
- 梳理 current separate-phase behavior 与 unified island-owned ordering 的差异。
- 为 contact/joint ordering、warm-start、wake reason、sleep skip 增加行为锁。
- 必要时调整 debug/lab facts，使 ordering 可解释。
- 只做单线程 deterministic contract。

不做：
- 不做 multithreaded solver。
- 不改 contact manifold generation。
- 不改 public joint API。
- 不做 broad material system redesign。

所有权：
- 可能触及：`crates/picea/src/pipeline/island.rs`、
  `crates/picea/src/pipeline/joints.rs`、`crates/picea/src/solver/contact.rs`、
  `crates/picea/src/pipeline/sleep.rs`、相关 stack/sleep tests。
- 不应触及：query API、scene schema、CCD algorithms。

验收标准：
- Contact/joint ordering contract 在文档和测试中明确。
- Existing stack stability、sleep、wake reason、warm-start 语义保持稳定。
- Debug facts 足以解释 island-local ordering。
- 如果不改变实现，也要有行为锁证明当前 contract 是刻意选择。

验证方式：
- `rtk proxy cargo test -p picea --test physics_realism_acceptance stack`
- `rtk proxy cargo test -p picea --test physics_realism_acceptance sleep`
- `rtk proxy cargo test -p picea --test world_step_review_regressions`
- `rtk proxy cargo test -p picea --lib pipeline::island`
- `rtk proxy cargo test -p picea --lib pipeline::sleep`
- `rtk proxy cargo bench -p picea --no-run`

Subagent 执行计划：
- explorer：叶子 agent；读 island/joint/contact/sleep path，列出当前 ordering
  facts 和需要锁定的行为。
- worker：叶子 agent；高风险 solver slice，优先高质量模型；先补行为锁，再做
  最小 contract change。
- reviewer：叶子 agent；重点审查 warm-start、wake/sleep、joint/contact ordering
  和 debug facts。
- verifier：叶子 agent；运行 stack/sleep/island gates。

提交策略：
- auto-commit：否。
- message hint：`Define M28 solver island ordering contract`。

风险 / 后续：
- 如果 contract change 影响物理结果，必须提供可解释 diff 和 rollback plan。

## M29：Performance Threshold Gate

状态：计划中。

目标：
把 M17/M23 累积的 baseline 证据推进到初始 perf regression guard。M29 完成后，
关键场景可以有合理阈值或警戒线，但这些阈值必须来自多轮 baseline，而不是
一次本机跑分。

为什么现在做：
M23 先补 counter 和 baseline，M29 再设置 threshold。这样可以避免过早把不稳定
wall-clock 数据变成阻塞开发的假警报。

范围：
- 选择少量关键场景：query-heavy、broadphase-heavy、stack、CCD、recipe/lab
  authoring。
- 定义 baseline 采样策略、variance 记录和 threshold policy。
- 增加 CI/local guard 文档或脚本，明确何时 fail、何时 warn。
- 把 counters 与 wall-clock 结论一起记录，避免只看耗时。

不做：
- 不为所有 benchmark 设置阈值。
- 不把 lab wall-clock 当 correctness oracle。
- 不做大规模性能重写；M29 是 gate，不是 tuning milestone。

所有权：
- 可能触及：`crates/picea/benches/physics_scenarios.rs`、benchmark docs、
  CI/local scripts、`docs/ai` routing。
- 不应触及：core physics behavior、public API、artifact schema。

验收标准：
- Threshold policy 明确区分 fail / warn / informational。
- Baseline 数据来源、重复次数、variance 解释可复现。
- 至少覆盖 query/broadphase/CCD/stack/recipe 中的关键场景。
- 不稳定环境下有明确 fallback，不阻塞正确性验证。

验证方式：
- `rtk proxy cargo bench -p picea --no-run`
- 按文档执行一次 local baseline smoke，并记录输出位置。
- `rtk proxy cargo test -p picea --test query_debug_contract`
- `rtk proxy git diff --check`

Subagent 执行计划：
- explorer：叶子 agent；梳理现有 Criterion 场景和 M17/M23 counters，提出最小
  threshold policy。
- worker：叶子 agent；优先 `gpt-5.4`，只改 bench/docs/scripts。
- reviewer：叶子 agent；重点审查阈值是否过早、是否可复现、是否误把性能当正确性。
- verifier：叶子 agent；运行 bench no-run 和指定 smoke。

提交策略：
- auto-commit：否。
- message hint：`Add M29 performance threshold gate`。

风险 / 后续：
- 本地机器差异会让硬阈值脆弱；默认先 warn，只有高置信场景才 fail。

## M30：Public Beta Hardening

状态：计划中。

目标：
把 M1-M29 的能力收束成可试用的 public beta：API surface、docs、examples、
error messages、migration notes 和 verification checklist 足够稳定，让外部用户
能在不读内部代码的情况下开始使用 Picea。

为什么现在做：
M30 是产品化收口，而不是继续扩功能。它应该发生在性能、query、authoring、
CCD、solver contract 都有足够证据之后。

范围：
- Public API review：`prelude`、`World`、query、recipe、scene/lab entrypoints。
- README / crate docs / examples / migration notes。
- Error message audit：query errors、scene authoring errors、world lifecycle
  errors。
- Final beta verification matrix：core tests、lab tests、examples no-run、
  docs/routing freshness、bench no-run。

不做：
- 不新增 major physics feature。
- 不做 breaking API rename，除非 beta freeze 前明确批准。
- 不承诺 1.0 semver；M30 是 public beta hardening。

所有权：
- 可能触及：`README.md`、`crates/picea/README.md`、`docs/*`、
  examples/tests、`crates/picea/src/lib.rs` re-export docs。
- 不应触及：solver algorithms、CCD algorithms、broadphase heuristics，除非只是
  修复 beta-blocking bug。

验收标准：
- 新用户能从 README / examples 完成 world creation、query、scene recipe、
  lab artifact inspection 的最小流程。
- Public API smoke 覆盖主要 beta surface。
- AI routing 和 milestone docs 不再指向过期能力标签。
- Beta 风险、非目标和下一阶段路线写清楚。

验证方式：
- `rtk proxy cargo test -p picea --lib`
- `rtk proxy cargo test -p picea --tests`
- `rtk proxy cargo test -p picea-lab`
- `rtk proxy cargo test -p picea --examples --no-run`
- `rtk proxy cargo bench -p picea --no-run`
- `rtk proxy ruby -e 'require "yaml"; YAML.load_file("docs/ai/doc-catalog.yaml"); puts "yaml ok"'`
- `rtk proxy git diff --check`

Subagent 执行计划：
- explorer：叶子 agent；审阅 public beta surface、docs gaps 和 stale routing。
- worker：叶子 agent；优先 `gpt-5.4`，只做 docs/examples/API-smoke hardening。
- reviewer：叶子 agent；重点审查 public API stability、docs accuracy、示例可运行性。
- verifier：叶子 agent；运行完整 beta hardening gates。

提交策略：
- auto-commit：否。
- message hint：`Prepare M30 public beta hardening`。

风险 / 后续：
- 如果发现 beta-blocking API 设计问题，必须暂停并让用户确认是否允许 breaking
  change。

## Post-M30 Follow-Up / Remaining Risks

These items are real follow-up work. M11-M22 are completed in the current
milestone line; M23-M30 are planned gates and are not completed claims.

- M17 completed: query/broadphase/solver-island counters, artifact summaries,
  and Criterion scenario coverage now exist as the evidence gate before tuning.
- M18 completed: broadphase candidate-pair traversal now walks subtree pairs
  once from internal child pairs, reducing avoidable candidate-pair
  traversal/prune work units while preserving candidate/query ordering and
  private proxy ids.
- M19 completed: expanded CCD/realism in the smallest evidence-backed slice
  rather than attempting full all-shape CCD.
- M20 completed: stabilized public scene authoring without replacing the
  low-level `World` lifecycle APIs.
- M21 completed: public distance/shape query now sits on top of the accepted
  query/cache/GJK substrate without exposing proxy/cache internals.
- M22 completed: compound/concave authoring is explicit for lab scene fixtures,
  while direct concave contact solving stays outside the core solver.
- Post-M22 completed slice: ramp-specific friction now has a signed downhill
  regression, and authored dynamic compound pieces have an additive
  mass/inertia behavior lock that distinguishes overlapping pieces from boolean
  concave union semantics.
- M23 planned: harden broadphase/query performance evidence before larger feature
  expansion.
- M24 planned: upgrade `picea-lab-web` into a process visualization foundation
  for broadphase tree traversal, island lifecycle, and compound provenance
  without letting lab recompute physics.
- M25 planned: define live scene patch semantics before editor-style mutation.
- M26 planned: choose one CCD expansion slice with TOI ordering and budget locks.
- M27 planned: consider automatic polygon decomposition behind strict authoring
  constraints.
- M28 planned: clarify solver ordering / island contract before parallel or
  tighter joint/contact coupling.
- M29 planned: convert repeated baseline evidence into initial threshold policy.
- M30 planned: harden public beta API/docs/examples/migration.
- Beyond M30: direct arbitrary concave contact solving, broad all-shape CCD,
  multithreaded solver execution, full visual editor, and 1.0 semver freeze all
  remain separate roadmap decisions.

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
- M11: surface broadphase candidate/update/rebuild facts and benchmark scenario
  IDs without making the lab the timing oracle.
- M12: show active island membership, solved contact/joint rows, wake reasons,
  and sleep state after active-island batching.
- M13: visualize generalized CCD sweeps, selected/ignored TOI hits, and
  advancement budgets.
- M14: host ergonomic scene/recipe examples as evidence that the public setup
  flow stays easy and reproducible.
- M15: surface available query/cache/recomputation facts and benchmark variance
  evidence for the performance data path; stronger allocation counters remain
  Post-M15.
- M16: show island-local body slots, contact row indices, joint rows, and skipped
  sleeping-island row construction when those facts are added to debug/lab
  artifacts.
- M17: show performance evidence summaries for query, broadphase, island, solver
  row, CCD-heavy, and recipe-heavy scenarios while keeping wall-clock benchmark
  timing outside lab pass/fail semantics.
- M18: show broadphase/query tuning counters, tree/rebuild facts, and
  query-heavy scenario summaries without exposing internal proxy ids.
- M19: visualize the selected CCD/realism expansion slice, including
  selected/ignored TOI facts, budget decisions, and ramp/friction evidence where
  applicable.
- M20: host stabilized scene-schema fixtures and authoring examples with clear
  validation errors, replay provenance, and low-level `World` fallback examples.
- M21: show public distance/shape-query results, filters, closest-point facts,
  and query stats without exposing internal broadphase proxy ids.
- M22: show supported compound/concave authoring examples, generated convex
  piece ordering and validation facts, and stable validation errors for
  unsupported direct concave solver usage; richer artifact/schema/UI
  provenance is planned in M24.
- M23: show query/broadphase counter summaries and benchmark-cost facts without
  treating wall-clock timing as the lab correctness oracle.
- M24: show broadphase tree nodes/traversal/prune facts, island graph and
  sleep/wake lifecycle, plus compound authored objects, generated convex pieces,
  inheritance facts, and validation paths in artifacts/UI.
- M25: show reset/patch/transaction provenance if live scene patching reaches
  artifact or server workflows.
- M26: show the selected CCD expansion slice through TOI ordering, budget
  decisions, and selected/ignored impact facts.
- M27: show polygon decomposition provenance, generated piece ordering, and
  rejected-input diagnostics.
- M28: show solver island ordering facts when contact/joint ordering becomes part
  of the accepted contract.
- M29: show benchmark scenario ids, counter summaries, baseline variance, and
  threshold/warn/fail status where applicable.
- M30: host public beta examples and docs-backed replay paths that demonstrate
  the stable user-facing workflow.

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
