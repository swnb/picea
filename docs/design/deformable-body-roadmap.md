# Deformable Body Roadmap

> Date: 2026-05-05
>
> Status: RFC boundary for future work. This document does not describe an
> implemented Picea feature.

This document fixes the product and engineering boundary for future soft-body,
particle, cloth, and deformable mesh work in Picea. The current engine is a 2D
rigid-body engine. M38 added a rigid-body lattice proxy for lab visualization;
it did not add true deformable physics.

## Decision

Picea should not treat soft-body support as a `picea-lab-web` UI feature.
True deformation requires a new core simulation model, artifact schema, and
verification lane.

If Picea pursues deformable physics, the recommended first real implementation
path is an experimental 2D XPBD/PBD particle-grid or particle-mesh body family,
kept separate from existing rigid `BodyDesc` / `ColliderDesc` / `JointDesc`
semantics until the behavior and debug facts are stable. FEM, cloth tearing,
runtime topology mutation, 3D tetrahedral meshes, and GPU solvers should remain
out of the first implementation slice.

M38's `lattice_grid` remains useful as a rigid-body approximation and UI
debugging surface, but it must keep using names such as "rigid-body lattice
proxy" or "joint-grid proxy". It must not be renamed to soft-body, cloth, FEM,
deformable mesh, or particle simulation.

## Current Picea Boundary

Current core facts:

- `BodyType` is `Static` / `Dynamic` / `Kinematic`, with rigid pose, linear
  velocity, angular velocity, mass, and inertia.
- `SharedShape` supports circle, rectangle, regular polygon, convex polygon,
  concave polygon authoring, and segment geometry. It does not model runtime
  mesh topology or per-vertex state.
- `JointDesc` supports distance and world-anchor joints. Joints constrain rigid
  body handles; they are not deformable elements.
- `SimulationPipeline::step` is organized around rigid body integration, joint
  constraints, contact generation/solving, CCD, sleep, events, and debug
  snapshots.
- `DebugSnapshot` exports bodies, colliders, joints, contacts, manifolds,
  broadphase tree, islands, and draw primitives. It has no particle set,
  element set, strain/stress field, deformation energy, or deformable material
  facts.

Current lab facts:

- `SceneShapeFixture` can author rigid collider shapes and rigid compound /
  static concave decomposition. It has no particle, cloth, flex, FEM, or mesh
  topology fixture type.
- `picea-lab-web` consumes exported frames. It must not recompute physics or
  invent authoritative deformable state in the browser.
- M38 `lattice_grid` uses ordinary dynamic rigid bodies, distance joints, and
  world-anchor joints. Its stretch display is a Web-derived view of exported
  joint anchors, not a material strain or stress fact.

## Industrial Reference Points

The point of these references is not to copy a 3D engine into Picea. They show
which surfaces industrial engines separate before claiming soft-body support.

- Box2D describes itself as a 2D rigid-body simulation library. That matches
  Picea's current body/collider/joint model and supports keeping M38 as a
  rigid-body approximation rather than pretending it is deformable physics.
  Source: <https://box2d.org/documentation/>
- PhysX soft bodies use FEM and separate simulation, collision, and render
  meshes. PhysX also has cooking/preprocessing steps for mesh acceleration
  structures, tetrahedra ordering, mass/volume facts, and remap tables.
  Sources: <https://nvidia-omniverse.github.io/PhysX/physx/5.3.1/docs/SoftBodies.html>,
  <https://nvidia-omniverse.github.io/PhysX/physx/5.1.2/docs/SoftBodies.html>
- PhysX PBD particle systems model fluid and deformable dynamics through
  particles and constraints, with GPU/CUDA requirements in current official
  documentation. This supports treating particle/PBD as a separate simulation
  surface, not as a hidden rigid-body patch.
  Source: <https://nvidia-omniverse.github.io/PhysX/physx/5.1.1/docs/ParticleSystem.html>
- MuJoCo distinguishes older composite objects that emulate soft bodies in a
  rigid-body simulator from true deformable objects exposed through `flex` and
  authoring helpers such as `flexcomp`. This is the same distinction Picea must
  keep between M38 rigid lattice proxy and future deformable bodies.
  Source: <https://mujoco.readthedocs.io/en/stable/modeling.html#deformable-objects>

## Candidate Routes

| Route | Fit For Picea | Tradeoff |
| --- | --- | --- |
| Keep rigid-body lattice proxy only | Good short-term debugging and authoring approximation. | Does not produce true material deformation; must keep proxy wording. |
| 2D XPBD/PBD particle-grid body | Best first true-deformable candidate because it is 2D, incremental, and easier to observe with constraint residuals. | Requires new particle handles, constraint types, collision coupling, solver stats, and stability tests. |
| FEM / finite elements | More physically meaningful for elastic solids. | Too large for first slice; needs mesh generation, material model, stress/strain facts, and more difficult numerical validation. |
| External plugin / interop | Lets Picea keep rigid core small while experimenting outside core. | Requires clear data interchange and artifact schema before UI can compare results. |
| GPU soft bodies or cloth tearing | Not a near-term fit. | Hardware/runtime dependency and topology mutation would swamp current 2D public API and test lanes. |

Recommended next design slice: define a CPU 2D XPBD/PBD prototype contract on
paper first, with no public API commitment until tests can prove deterministic
step behavior, rigid coupling, and artifact stability.

## Minimum Future Core Surface

A real deformable implementation needs explicit owners rather than overloading
rigid bodies:

- `DeformableHandle`, `ParticleHandle`, and possibly `ElementHandle` /
  `ConstraintHandle`, all generation-safe like existing body/collider/joint
  handles.
- A `DeformableDesc` or recipe fixture carrying particles, rest positions,
  masses, pins/anchors, material parameters, and constraints.
- A material model that names solver behavior directly: stretch stiffness,
  bending/area/volume preservation when applicable, damping, collision
  thickness, friction, and iteration budget.
- A pipeline phase with deterministic ordering and numeric warnings. It must say
  whether deformable constraints solve before rigid contacts, after rigid
  contacts, or in an interleaved phase.
- Rigid coupling rules: particle-vs-rigid collision, pinned particles, anchor to
  body, wake/sleep interaction, CCD limitations, and how query caches resync.
- Stable failure modes for invalid meshes/particles, non-finite positions,
  duplicate topology, inverted elements, impossible constraints, and solver
  divergence.

Do not add this by extending `BodyPatch` or live-session velocity perturbation.
Those contracts are for rigid bodies.

## Query And Selection Boundary

Future deformable state must not silently appear in the existing rigid
`QueryPipeline` contract. Today's query API answers over rigid colliders and
returns rigid collider/body-oriented hits. A deformable implementation needs an
explicit query decision before it becomes public surface:

- whole-deformable queries: coarse AABB / shape / distance checks that return a
  deformable handle, useful for broad selection and editor picking;
- particle queries: point, radius, or nearest-particle checks that return a
  particle handle and local state for precise inspection;
- element or constraint queries: segment/triangle/edge hits that return an
  element or constraint handle when the authored topology is selectable;
- collision-proxy queries: hits against a reduced collision representation when
  simulation particles, collision proxies, and render meshes differ.

A future result type may look like a separate `DeformableHit` carrying the
deformable handle, optional particle/element/constraint handle, feature kind,
world-space point, normal or distance when available, and the revision used to
answer the query. That is only a candidate contract; it is not implemented now.

Query cache and selection state must be revision-aware. A deformable query cache
must resync from `WorldRevision` plus a deformable-specific topology/state
revision, not from Web-rendered positions. Lab-web selection must use handles
exported by Rust artifacts or live frames. Until those handles exist, the Web UI
may visualize M38 rigid lattice nodes and edges, but it must not claim particle,
element, or deformable-body selection semantics.

## Artifact And Debug Facts

Before any Web UI claims deformable support, `DebugSnapshot` / artifacts need
additive, serde-default facts such as:

- particles: handle, position, predicted position, velocity, inverse mass,
  pinned/active/sleeping flags;
- constraints/elements: endpoints or element vertices, rest length/area,
  current length/area, stretch ratio, residual, material id, iteration count;
- collisions: particle/collider contacts, normal, depth, friction, owning rigid
  collider, and whether the contact fed a rigid or deformable solve phase;
- solver stats: particle count, constraint row count, active set count,
  iterations used, max residual, numeric warning count, and broadphase/query
  counters;
- render mapping when a visual mesh differs from the simulation particles.

These facts must remain read-only. Web may derive summaries and overlays from
them, but Web must not invent authoritative particle positions, material strain,
or solver residuals.

## Lab Authoring And UI Requirements

Future lab support should add a versioned scene fixture boundary before any
viewer work:

- authoring schema for a small 2D particle-grid or particle-mesh fixture;
- deterministic built-in scenarios: pinned strip, hanging cloth-like grid,
  rigid collider interaction, and high-stiffness stability case;
- artifact replay and live session support driven by Rust server state;
- Web overlays for particles, constraints, pins, collision contacts, residual
  heat, and selected particle/constraint inspector facts;
- copyable debug context that records scenario id, solver settings, selected
  particle/constraint, frame, and residual summary.

UI copy must use:

- "rigid-body lattice proxy" for M38;
- "particle/PBD prototype" only after particles exist in Rust core artifacts;
- "soft-body" or "deformable body" only after core state, solver, collision,
  debug facts, artifacts, tests, and docs exist.

## Verification Gates For A Future Implementation

The first implementation milestone after this RFC should start with behavior
locks, not UI:

- deterministic fixed-step tests for a pinned particle strip or grid;
- validation tests for invalid topology and non-finite inputs;
- coupling tests for particle-vs-rigid collision if included in V1;
- serialization compatibility tests for additive debug/artifact fields;
- performance evidence based on deterministic counters before wall-clock gates;
- browser acceptance only after Rust artifacts expose authoritative deformable
  facts.

Suggested command families:

- `rtk proxy cargo test -p picea --test deformable_*`
- `rtk proxy cargo test -p picea-lab --test artifact_run deformable`
- `rtk proxy cargo test -p picea-lab --test server_routes deformable`
- `cd crates/picea-lab/web && rtk proxy npm run build`
- `cd crates/picea-lab/web && rtk proxy npm run test:ui-contract`
- `cd crates/picea-lab/web && rtk proxy npm run test:i18n`

## Open Questions

These questions must be answered before any implementation milestone claims a
public deformable body surface:

- Is V1 a 2D particle-grid, triangle mesh, edge/constraint network, or another
  representation?
- Does V1 interact with rigid bodies through contacts, pinned anchors, or both?
- Are solver iterations fixed per step, adaptive, or per-material?
- Which facts are authoritative core facts versus Web-derived display summaries?
- Does the future authoring schema store rest topology directly, generate it
  from a higher-level fixture, or import it from an external asset?
- What does determinism mean for this feature: stable state hash, stable
  residual counters, stable visual topology, or all of them?
- How are invalid topology, tearing/cutting requests, and runtime topology
  mutation rejected in V1?

## Decision Checklist

Before starting a deformable implementation milestone, the plan must name:

- chosen route: rigid proxy continuation, XPBD/PBD particles, FEM, or external
  plugin/interop;
- V1 non-goals, especially tearing, cutting, 3D, GPU, and runtime topology
  mutation;
- public handles and generation/revision semantics;
- authored rest state and validation errors;
- solver phase placement relative to rigid integration, joints, contacts, CCD,
  and sleep;
- debug/artifact fields and serde-default compatibility tests;
- lab scenarios and browser acceptance checks;
- performance evidence counters and any benchmark gates.

## Routing Rule

Future requests that mention soft body, cloth, particle system, deformable
mesh, FEM, XPBD/PBD, runtime mesh topology, tearing, or cutting should route
here first. Do not route those requests directly to `picea-lab-web` unless the
question is explicitly about visualizing already-exported deformable facts.
