# Picea Physics Engine Upgrade Technical Plan

This document records the direction agreed for Picea's next physics-engine upgrade. It is intentionally separate from milestone history: use the current `World` + `SimulationPipeline` code as the source of truth, and use this document to explain the target architecture, algorithm choices, and acceptance order.

## Positioning

Picea should not try to win by being a smaller clone of Box2D or Rapier. The useful niche is:

- an ergonomic 2D engine with a small, explicit Rust API;
- predictable fixed-step behavior that is easy to test and explain;
- data-oriented internals that can grow into good performance;
- first-class debug facts for broadphase, narrowphase, contact, solver, sleep, and CCD decisions.

The bias is ease of use first, with performance designed into the internal layout instead of patched on through hidden global state.

## Post-M10 Baseline And Remaining Gap

The first production line has moved Picea past the "missing core algorithms"
stage. Current `World` + `SimulationPipeline` already has persistent broadphase
proxies, SAT + clipping manifolds, mass/inertia facts, warm-started sequential
impulses, island sleep/wake reasons, GJK/EPA fallback, a narrow CCD
pose-clamping phase, recipe APIs, debug facts, lab artifacts, and Criterion
baseline benches.

The remaining gap to top production 2D engines is now system quality:

- broadphase is production-shaped but still needs a hotter internal data path:
  handle-to-leaf indexing, tree query reuse, stronger insertion/balancing
  heuristics, and fewer per-step temporary collections;
- queries are stable and easy to use, but still rebuild from debug snapshots and
  scan cached colliders rather than reusing an indexed spatial structure;
- the contact solver has the right sequential impulse model, but not yet an
  active-island compact-array execution path for contacts and joints;
- CCD covers the first important dynamic-circle vs static-convex slice, but not
  dynamic-vs-dynamic motion, generic shape casts, or multi-impact advancement;
- shape geometry and support data are recomputed in several hot paths instead
  of being cached behind transform revision;
- public authoring is much better through recipes, but still lacks a higher
  level scene/asset layer and richer error paths for complex setup flows.

This means the next upgrades should optimize the system around the algorithms
that now exist, not immediately chase a broad new algorithm surface.

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
  scenarios. The next step is performance substrate work: direct handle-to-leaf
  lookup, query reuse, and better balancing/insertion heuristics.

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
The next solver goal is active-island execution:

- build compact per-island body/contact/joint arrays for the active solve;
- solve contacts and joints through the same island ordering contract;
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

The first CCD slice exists as a clearly named pose-clamping phase. It handles
dynamic circles against static thin walls / static convex geometry, emits
`ccd_trace`, and feeds the normal contact path after clamping. The next CCD work
should generalize carefully:

- opt in per body or per collider first, then add automatic fast-body heuristics;
- generate swept AABBs in broadphase;
- use time of impact for circle/segment, circle/polygon, and convex fallback;
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
The next ergonomic layer should be additive: scene/asset recipes, better nested
error context, and serializable setup flows without weakening the low-level
`World` contract.

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
- avoid per-step allocations in contact generation and event refresh;
- keep scenario benchmarks for sparse broadphase, dense broadphase, stacked contacts, CCD bullets, and large batch creation;
- add thresholds only after several local baselines make variance and expected
  counter shapes clear.

## Acceptance Order

1. M10.5 closeout (completed 2026-04-27): make docs, backlog, and verification
   routing agree that the M1-M10 capability line is complete.
2. M11 performance substrate: broadphase leaf indexing, query reuse, shape
   geometry caches, allocation reduction, and benchmark counter baselines.
3. M12 active island solver: compact active arrays, island-level contact/joint
   solve, warm-start preservation, and debug fact continuity.
4. M13 CCD generalization: dynamic-vs-static shape casts, conservative
   advancement, multi-impact budgeting, and trace semantics.
5. M14 ergonomic API v2: higher-level scene/asset recipes, serializable setup,
   and richer command error context.

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
- narrow CCD for dynamic circle vs static convex geometry with `ccd_trace`;
- `BodyBundle`, `ColliderBundle`, `JointBundle`, `WorldRecipe`, and
  transactional `WorldCommands`;
- `picea-lab` artifact/replay evidence and Criterion baseline scenarios.

The next high-risk work is not "finish the old algorithm checklist"; it is
making those capabilities faster, more reusable, and harder to misuse.
