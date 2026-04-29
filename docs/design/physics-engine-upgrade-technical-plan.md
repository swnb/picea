# Picea Physics Engine Upgrade Technical Plan

This document records the direction agreed for Picea's next physics-engine upgrade. It is intentionally separate from milestone history: use the current `World` + `SimulationPipeline` code as the source of truth, and use this document to explain the target architecture, algorithm choices, and acceptance order.

## Positioning

Picea should not try to win by being a smaller clone of Box2D or Rapier. The useful niche is:

- an ergonomic 2D engine with a small, explicit Rust API;
- predictable fixed-step behavior that is easy to test and explain;
- data-oriented internals that can grow into good performance;
- first-class debug facts for broadphase, narrowphase, contact, solver, sleep, and CCD decisions.

The bias is ease of use first, with performance designed into the internal layout instead of patched on through hidden global state.

## Post-M20 Baseline And Remaining Gap

The current production line has moved Picea past the "missing core algorithms"
stage. Current `World` + `SimulationPipeline` already has persistent broadphase
proxies with direct leaf lookup, SAT + clipping manifolds, mass/inertia facts,
warm-started sequential impulses, island sleep/wake reasons, GJK/EPA fallback,
active-island batching for contact/joint rows, a dynamic-vs-static convex CCD
pose-clamping phase with multi-hit ordering/budget traces, the selected
dynamic-vs-dynamic convex CCD slice, scene/asset recipe helpers, nested recipe
error paths, serializable lab fixture flows, a versioned v1 scene schema, an
internal query spatial index, transform/revision-backed collider geometry
caches, debug facts, lab artifacts, and Criterion baseline benches.

The remaining gap to top production 2D engines is now system quality:

- broadphase now has direct collider-handle to leaf lookup, the M15 query path
  has an internal broadphase-style index, and M18 removed avoidable per-leaf
  root scans from candidate-pair traversal, but the persistent tree still needs
  stronger insertion/balancing heuristics and deeper query/allocation counters;
- queries are stable, ordered, and use indexed candidate traversal internally,
  and M21 has productized public distance/shape queries through `QueryShape`,
  `ShapeHit`, and `QueryPipeline::intersect_shape` / `closest_shape`;
- the contact solver now uses deterministic dense island-local body slots,
  contact row indices, and joint rows for active islands, while later solver
  work should focus on stronger island-owned ordering, deeper allocation
  evidence, or parallelism only after behavior locks justify it;
- CCD now covers staged dynamic-vs-static convex casts plus the selected
  translational dynamic-vs-dynamic convex slice, but not rotational casts,
  dynamic compound CCD, or all-shape coverage;
- shape AABBs and convex world vertices are now cached behind transform/revision
  keys and reused by query/contact/CCD/GJK paths, but broader support-map
  counters and allocation evidence remain follow-up;
- public authoring now has scene/asset recipe helpers, nested path context,
  serializable lab fixture flows, a versioned v1 scene schema, and the M22
  compound/concave authoring boundary; direct concave contact solving remains
  outside the core solver.

This means the next upgrade line should optimize and deepen the system around
the accepted M11-M22 capabilities, not treat those milestones as still open.
M21 and M22 are now completed user-facing query and authoring slices, with
deeper artifact provenance, decomposition, and performance work left staged.

## Broadphase Decision

Use a Box2D-style dynamic AABB tree as Picea's default broadphase, not Rapier's BVH as the first production path.

Why:

- Picea is 2D and mostly dynamic-body focused; dynamic proxy insert/move/remove operations map well to a dynamic AABB tree.
- Box2D's dynamic tree is a proven hierarchical AABB tree for broadphase queries, ray casts, and region queries.
- A persistent proxy tree gives us deterministic pair ordering, cheap debug snapshots, and a direct path to scene queries without coupling to a 3D-oriented BVH abstraction.
- Rapier's BVH broadphase is attractive, but its strengths are less aligned with Picea's current simple 2D shape set and API goals.

Tradeoff:

- Proxy storage remains internal to `World`; public users should not manage
  broadphase proxy IDs.
- The persistent tree now uses fat AABBs, incremental moves, stale cleanup,
  deterministic rebuild/compaction, step/debug metrics, and benchmark
  scenarios. The first performance-substrate slice added direct
  handle-to-leaf lookup, and M15 added internal query-index reuse. The next
  broadphase work is better balancing/insertion heuristics and stronger
  query/perf counters.

Sources:

- [Box2D Dynamic Tree](https://box2d.org/documentation/group__tree.html)
- [Box2D Collision: Dynamic Tree](https://box2d.org/documentation/md_collision.html)
- [Rapier `BroadPhaseBvh`](https://docs.rs/rapier3d/latest/rapier3d/geometry/struct.BroadPhaseBvh.html)

## Narrowphase Decision

Use specialized shape dispatch first, then generic convex fallback:

- circle-circle: analytic contact generation;
- circle-segment and circle-polygon: analytic closest-point paths;
- polygon-polygon and rectangle-rectangle: SAT to find the minimum penetration axis, then reference/incident edge clipping for a 1-2 point manifold;
- generic convex fallback: GJK for distance/intersection and CCD support, with EPA or closest-feature fallback only when SAT is not available;
- concave polygons: decompose or treat as static compound convex pieces; do not solve concave contact directly in the core solver.

SAT is the best default for Picea's 2D convex polygons because it is simple, debuggable, gives a usable minimum translation axis, and pairs naturally with clipping. GJK is still important, but more as a distance/CCD/fallback kernel than as the primary polygon manifold generator.

V-Clip is not the first choice. It is valuable for feature-tracking convex polyhedra, but it requires richer boundary-representation feature topology than Picea currently stores. We should revisit it only if SAT feature IDs plus GJK fallback cannot provide stable enough manifolds.

Sources:

- [Original GJK paper](https://graphics.stanford.edu/courses/cs164-09-spring/Handouts/paper_GJKoriginal.pdf)
- [V-Clip paper summary](https://www.sciweavers.org/node/294355)
- [Separating Axis Theorem overview](https://textbooks.cs.ksu.edu/cis580/04-collisions/04-separating-axis-theorem/index.html)

## Contact And Manifold Plan

Contact generation should produce a manifold, not only one point:

- analytic circle contacts produce one point;
- polygon contacts use SAT + clipping to produce up to two points;
- each point carries a feature key, normal, penetration depth, and local anchors;
- the contact manager persists matching feature keys across frames;
- warm-starting reuses previous normal and tangent impulses only when the feature key and normal orientation are compatible.

This is the path to stable stacks. A single position correction point can pass toy cases, but it cannot reliably handle resting contact, friction, or stacked bodies.

## Solver Plan

The first solver goal has landed: Picea now uses a warm-started sequential
impulse contact solver with effective mass, inverse inertia, Coulomb friction,
restitution thresholding, velocity iterations, and residual position correction.
The accepted M12 slice has also landed: contacts and joints are batched by
active island and sleeping islands stay out of hot solver rows. The remaining
solver deepening work is denser island-local execution:

- replace the current map/set-heavy batching with compact per-island
  body/contact/joint arrays for the active solve;
- move contact and joint solving closer to one island-owned ordering contract
  while preserving the current separate-phase behavior until it is proven safe;
- preserve stable public handles while keeping hot solver state dense;
- keep warm-start and debug facts attached to stable contact ids;
- keep residual position correction from overwriting solved velocity facts.

This follows the same family of ideas as Box2D's solver: iterative impulses plus
temporal coherence, with Picea keeping the solved impulse facts available for
events, debug snapshots, and lab artifacts.

Sources:

- [Erin Catto, Iterative Dynamics with Temporal Coherence](https://www.gamedevs.org/uploads/iterative-dynamics-with-temporal-coherence-slides.pdf)
- [Box2D Simulation: restitution, friction, sleeping, CCD](https://box2d.org/documentation/md_simulation.html)

## Sleep And Wake Plan

Sleeping is now stability-window and island based, not "one quiet frame":

- each dynamic body tracks low-motion time;
- a body may sleep only after sustained low linear and angular speed;
- production behavior evaluates deterministic contact/joint islands;
- wake reasons should be explicit: contact impulse, joint correction, user patch, transform edit, or velocity edit;
- sleeping data should eventually move out of hot active arrays for cache locality.

The next sleep-related work belongs with active-island storage: keep island
membership and sleeping bodies out of hot arrays without weakening deterministic
events and debug facts.

Source:

- [Box2D Simulation: island sleep](https://box2d.org/documentation/md_simulation.html)

## CCD Plan

CCD exists as a clearly named pose-clamping phase. It handles dynamic circles
and the accepted translational dynamic-convex vs static-convex slice, emits
`ccd_trace`, and feeds the normal contact path after clamping. The next CCD
work should broaden that staged model carefully:

- opt in per body or per collider first, then add automatic fast-body heuristics;
- generate swept AABBs in broadphase;
- use time of impact for more shape pairs, rotational casts, and eventual
  conservative-advancement fallback;
- advance to the earliest impact, clamp to a small slop before contact, then resolve through the normal solver;
- keep CCD event semantics explicit because hit/contact events may be delayed or split across substeps.

CCD should stay staged: dynamic-vs-static shape casts first, then multi-impact
conservative advancement, then dynamic-vs-dynamic only when benchmarks and
behavior locks justify the cost.

Source:

- [Erin Catto, Continuous Collision](https://box2d.org/files/ErinCatto_ContinuousCollision_GDC2013.pdf)

## API And Ergonomics Direction

The public API should be easier than traditional engine APIs where historical layers leak through:

- keep explicit low-level `World::create_body` and `World::create_collider` for control;
- keep `BodyBundle` / `ColliderBundle` / `JointBundle` for common object creation;
- keep `WorldRecipe` for declarative test worlds and examples;
- keep `WorldCommands` for batch creation, destruction, and patches with validation before mutation;
- keep named material and collision-layer assets so users do not hand-thread masks and coefficients everywhere;
- return structured creation results with handles, emitted events, and validation errors;
- expose `StepReport` and debug snapshots as first-class facts, not afterthought logs.

The advantage over copying Box2D/Rapier surface APIs is that Picea can design
around modern Rust data ownership and reproducible debugging from the start.
The accepted M14 ergonomic layer is additive: scene/asset recipes, better
nested error context, and serializable setup flows are in place for the current
milestone line. Future work should stabilize the public scene schema and
broaden authoring coverage without weakening the low-level `World` contract.

## Performance Direction

Performance work should now become the next main line. It should follow data
flow rather than micro-optimizing isolated helpers:

- persistent broadphase proxies and fat AABBs;
- direct broadphase leaf lookup by collider handle;
- public query reuse of broadphase-style spatial indexes where semantics match;
- stable arena handles with generation checks;
- separate hot simulation arrays from user metadata;
- compact active islands while leaving stable public handles intact;
- cache shape support data and world-space vertices when transforms change;
- reduce repeated geometry rebuilding and add conservative pre-sizing where the
  current counters or behavior locks justify it;
- keep scenario benchmarks for sparse broadphase, dense broadphase, stacked contacts, CCD bullets, and large batch creation;
- add thresholds only after several local baselines make variance and expected
  counter shapes clear.

## Acceptance Order

1. M10.5 closeout (completed 2026-04-27): make docs, backlog, and verification
   routing agree that the M1-M10 capability line is complete.
2. M11 performance substrate (completed 2026-04-27): the accepted scope is
   direct broadphase leaf lookup as the performance substrate entry point.
3. M12 active island solver (completed 2026-04-27): the accepted scope is
   active-island batching for contact/joint rows with sleeping islands removed
   from hot solve rows.
4. M13 CCD generalization (completed 2026-04-27): the accepted scope is
   dynamic-vs-static translational convex CCD with multi-hit ordering and
   budget traces.
5. M14 ergonomic API v2 (completed 2026-04-27): the accepted scope is
   scene/asset recipe helpers, nested error paths, and serializable lab
   fixture flows with `v1_api_smoke` and lab fixture acceptance.
6. M15 performance data path (completed 2026-04-27): `QueryPipeline` reuses an
   internal broadphase-style spatial index, collider-derived AABBs and convex
   world vertices are cached behind transform/revision keys, and contact/CCD/GJK
   paths now reduce repeated geometry rebuilding with conservative pre-sizing
   where behavior locks make it visible.
7. M16 dense island execution (completed 2026-04-28): replace the current map/set-heavy
   active-island solver staging with deterministic per-island dense body slots,
   contact row indices, and joint rows while preserving current separate-phase
   behavior, live step order, and public handles.
8. M17 performance evidence and tuning gate (completed 2026-04-28): add query,
   broadphase, island, solver-row, CCD-heavy, and recipe-heavy counter evidence
   before deeper tuning claims.
9. M18 broadphase and query tuning (completed 2026-04-28): reduce avoidable
   candidate-pair traversal work while preserving public ordering and private
   proxy ids.
10. M19 CCD and realism expansion (completed 2026-04-28): land the selected
   translational dynamic-vs-dynamic convex CCD slice with dynamic-target trace
   facts rather than claiming all-shape CCD.
11. M20 scene schema and authoring UX (completed 2026-04-28): stabilize the v1
   scene fixture schema, version errors, and joint authoring path while keeping
   low-level `World` APIs stable.
12. M21 public distance and shape query API (completed 2026-04-28): expose
   deterministic filterable distance/closest-shape query facts on top of the
   accepted query, cache, and GJK substrate.
13. M22 compound and concave authoring boundary (completed 2026-04-28): support
   safe compound convex or pre-decomposed authoring while keeping arbitrary
   concave contact solving outside the core solver.

## Post-M20 / M21-M22 Deepening

The next line is no longer "finish M11-M20". M15-M20 have landed as concrete
follow-ups:

- `QueryPipeline` now reuses an internal spatial index for semantic-match query
  candidates without exposing broadphase proxy or leaf implementation details;
- collider-derived geometry now uses transform/revision-backed AABB and convex
  world-vertex caches that contact, CCD, generic GJK/EPA fallback, and query
  paths can safely share;
- query allocation/perf counters and deeper solver allocation work remain
  follow-up and should move only with counter evidence;
- keep Criterion as baseline evidence until variance is understood.

M16 Dense Island Execution is now the accepted dense solver-layout slice:

- active-island batching now has deterministic island-local body slots,
  contact row indices, and joint rows;
- preserve the current separate-phase behavior and live step order until a
  unified island-owned ordering contract is proven safe;
- keep public handles stable while hot solver state uses island-local slot
  indices;
- carry warm-start, wake reason, and debug facts through the layout change.

M21/M22 landed the first user-facing deepening line:

- public distance/shape query now lets application code inspect
  geometry without rebuilding engine internals;
- compound/concave authoring now lets users express common
  concave-looking objects through convex pieces or stable validation errors;
- extend CCD toward rotational, dynamic compound, and broader all-shape coverage
  only when behavior locks and benchmarks justify it;
- add focused ramp-friction coverage and other realism regressions where the
  current accepted line is intentionally narrow;
- define any future live authoring semantics separately from static scene
  loading.

## Capability Line Landed

The M1-M10 line should be treated as the first production capability baseline:

- persistent broadphase state with fat AABBs, lifecycle cleanup, deterministic
  rebuild/compaction, and debug counters;
- SAT + clipping manifolds, analytic circle contacts, stable feature ids, and
  a GJK/EPA fallback for supported convex pairs;
- density-derived mass, center-of-mass, inertia, and static/kinematic/dynamic
  inverse-mass semantics;
- persistent contact identity, warm-start transfer, and a row-based sequential
  impulse contact solver;
- deterministic island sleep/wake reasons and explicit event/debug facts;
- staged CCD for dynamic circles and translational dynamic convex shapes against
  static convex geometry with `ccd_trace`;
- `BodyBundle`, `ColliderBundle`, `JointBundle`, `WorldRecipe`, and
  transactional `WorldCommands`, plus scene/asset recipe fixtures;
- `picea-lab` artifact/replay evidence and Criterion baseline scenarios.

The next high-risk work is not "finish the old algorithm checklist"; it is
making those capabilities faster, more reusable, and harder to misuse.
