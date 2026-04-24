# Picea Physics Engine Upgrade Technical Plan

This document records the direction agreed for Picea's next physics-engine upgrade. It is intentionally separate from milestone history: use the current `World` + `SimulationPipeline` code as the source of truth, and use this document to explain the target architecture, algorithm choices, and acceptance order.

## Positioning

Picea should not try to win by being a smaller clone of Box2D or Rapier. The useful niche is:

- an ergonomic 2D engine with a small, explicit Rust API;
- predictable fixed-step behavior that is easy to test and explain;
- data-oriented internals that can grow into good performance;
- first-class debug facts for broadphase, narrowphase, contact, solver, sleep, and CCD decisions.

The bias is ease of use first, with performance designed into the internal layout instead of patched on through hidden global state.

## Current Gap

Picea's current `World` path has a clean lifecycle API and useful tests, but it is still below top production engines in several physics areas:

- broadphase was effectively all-pairs before this upgrade slice;
- narrowphase only had AABB-style overlap for most contact generation;
- contact solving was positional and did not model mass, friction, restitution, warm-starting, or stable manifolds deeply enough;
- sleeping lacked a stability window and island-level wake reasoning;
- CCD is still missing, so fast thin-wall cases can tunnel;
- scene creation is explicit but still verbose for real users.

## Broadphase Decision

Use a Box2D-style dynamic AABB tree as Picea's default broadphase, not Rapier's BVH as the first production path.

Why:

- Picea is 2D and mostly dynamic-body focused; dynamic proxy insert/move/remove operations map well to a dynamic AABB tree.
- Box2D's dynamic tree is a proven hierarchical AABB tree for broadphase queries, ray casts, and region queries.
- A persistent proxy tree gives us deterministic pair ordering, cheap debug snapshots, and a direct path to scene queries without coupling to a 3D-oriented BVH abstraction.
- Rapier's BVH broadphase is attractive, but its strengths are less aligned with Picea's current simple 2D shape set and API goals.

Tradeoff:

- First implementation may rebuild from live proxies per step, as done in the current slice, to lock behavior without a lifetime-heavy proxy store.
- The production target is a persistent dynamic tree with proxy IDs stored on collider records, fat AABBs, incremental moves, and optional rebuild metrics.

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

Move from correction-only contacts to a sequential impulse solver:

- compute effective inverse mass and inverse inertia per constraint row;
- warm-start cached normal and tangent impulses before velocity iterations;
- solve normal impulse with non-negative clamping;
- solve tangent impulse with Coulomb friction clamped by normal impulse;
- apply restitution only above a configurable velocity threshold;
- run a small position correction pass for residual penetration.

This follows the same family of ideas as Box2D's solver: iterative impulses plus temporal coherence. The current material velocity response is an interim slice to make restitution and friction observable before the full solver lands.

Sources:

- [Erin Catto, Iterative Dynamics with Temporal Coherence](https://www.gamedevs.org/uploads/iterative-dynamics-with-temporal-coherence-slides.pdf)
- [Box2D Simulation: restitution, friction, sleeping, CCD](https://box2d.org/documentation/md_simulation.html)

## Sleep And Wake Plan

Sleeping must be stability-window based, not "one quiet frame":

- each dynamic body tracks low-motion time;
- a body may sleep only after sustained low linear and angular speed;
- later production behavior should evaluate islands, not isolated bodies;
- wake reasons should be explicit: contact impulse, joint correction, user patch, transform edit, or velocity edit;
- sleeping data should eventually move out of hot active arrays for cache locality.

The current slice implements the first body-level timer and resets it on contact, joint correction, integration disable paths, and user wake-like edits.

Source:

- [Box2D Simulation: island sleep](https://box2d.org/documentation/md_simulation.html)

## CCD Plan

Add CCD after the first stable broadphase/narrowphase/solver path:

- opt in per body or per collider first, then add automatic fast-body heuristics;
- generate swept AABBs in broadphase;
- use time of impact for circle/segment, circle/polygon, and convex fallback;
- advance to the earliest impact, clamp to a small slop before contact, then resolve through the normal solver;
- keep CCD event semantics explicit because hit/contact events may be delayed or split across substeps.

CCD should be narrow in scope at first: fast circles against static thin walls, then dynamic-vs-static convex, then dynamic-vs-dynamic if benchmarks justify it.

Source:

- [Erin Catto, Continuous Collision](https://box2d.org/files/ErinCatto_ContinuousCollision_GDC2013.pdf)

## API And Ergonomics Direction

The public API should be easier than traditional engine APIs where historical layers leak through:

- keep explicit low-level `World::create_body` and `World::create_collider` for control;
- add `BodyBundle` / `ColliderBundle` for common body + shape + material creation;
- add `WorldRecipe` for declarative test worlds and examples;
- add `WorldCommands` for batch creation, destruction, and patches with validation before mutation;
- add named material and collision-layer assets so users do not hand-thread masks and coefficients everywhere;
- return structured creation results with handles, emitted events, and validation errors;
- expose `StepReport` and debug snapshots as first-class facts, not afterthought logs.

The advantage over copying Box2D/Rapier surface APIs is that Picea can design around modern Rust data ownership and reproducible debugging from the start.

## Performance Direction

Performance work should follow data flow:

- persistent broadphase proxies and fat AABBs;
- stable arena handles with generation checks;
- separate hot simulation arrays from user metadata;
- compact active islands while leaving stable public handles intact;
- cache shape support data and world-space vertices when transforms change;
- avoid per-step allocations in contact generation and event refresh;
- add scenario benchmarks for sparse broadphase, dense broadphase, stacked contacts, CCD bullets, and large batch creation.

## Acceptance Order

1. Physics realism baseline: keep acceptance tests for current known gaps.
2. Broadphase: dynamic AABB tree candidate filtering with deterministic pair order.
3. Narrowphase: circle analytic contacts, then polygon SAT + clipping.
4. Solver: sequential impulses, warm-start, effective mass, friction, restitution threshold.
5. Sleep: body stability window first, island sleep and wake reasons second.
6. CCD: fast circle vs thin static wall, then broader convex support.
7. API ergonomics: batch commands, bundles, recipes, and material/layer presets.
8. Observability: debug facts for candidate drops, contact reduction, impulses, and sleep/CCD decisions.

## Slice Landed In This Upgrade

This implementation slice intentionally stops short of a full production solver. It adds:

- an internal dynamic AABB tree candidate pass;
- circle-circle narrowphase rejection of AABB-only false positives;
- first restitution/friction velocity response so material values affect motion;
- body-level sleep stability window;
- acceptance tests that keep CCD visible as the remaining known-red gap.

Remaining high-risk work:

- persistent broadphase proxy storage;
- SAT + clipping manifolds;
- full sequential impulse solver with effective mass and warm-start;
- island sleep and wake reason reporting;
- CCD time-of-impact implementation;
- ergonomic creation bundles and command batches.
