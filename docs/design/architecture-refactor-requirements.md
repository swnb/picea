# Architecture Refactor Requirements

> Date: 2026-04-20
>
> Status: direction archive. This document records the next architecture refactor target, acceptance shape, example coverage, and performance benchmark policy. It is not an implementation plan.

## 1. Why This Document Exists

M0-M8 made Picea much more verifiable, but most work so far has been local hardening inside the existing structure. The engine still needs an architecture refactor before it can honestly claim high-performance, high-realism direction.

Current milestone work improved:

- deterministic fixed-step ticking
- safer element storage than the original `Rc<UnsafeCell>` model
- concave decomposition caching
- AABB broadphase and manifold lifecycle
- solver effective mass hardening
- wasm public API hardening
- stable contact identity and warm-start transfer

Those are prerequisites. They are not the final architecture.

The next architecture work should split responsibilities that are currently mixed across `Scene`, `collision`, and `constraints`.

## 2. Repository Research Summary

The current documentation layout already has useful boundaries:

- `docs/architecture/` describes observed/current module structure and runtime flow.
- `docs/design/` records future-facing design intent.
- `docs/plans/` records milestone execution scope and historical gates.
- `docs/ai/` records routing, evals, debug artifacts, and agent workflow.

Therefore this refactor direction belongs in `docs/design/`, because it describes future architecture intent and acceptance criteria. Once implementation changes land, `docs/architecture/` should be updated to describe the new actual structure.

Current examples live in `crates/picea/examples/` and are mostly visual/demo scenarios:

- `ground`, `stack`, `stack2`
- `friction`
- `newton_cradle`, `conservation_of_momentum`
- `bridge`, `link`, `cloth`
- `pit`, `accumulation`

They are useful for smoke and visual sanity, but not enough as architecture acceptance because most examples are window-oriented and do not assert deterministic outcomes.

There is currently no benchmark harness or `benches/` directory. Rust/Cargo has a standard `cargo bench` entrypoint, but built-in `#[bench]` is unstable/nightly-only; Cargo recommends stable-channel packages such as Criterion for benchmark workflows. Criterion reports confidence intervals and change estimates, which fits regression acceptance. Iai-Callgrind is stable-compatible and useful for instruction/memory profiling when wall-clock noise is too high.

References:

- [Cargo Book: cargo bench](https://doc.rust-lang.org/beta/cargo/commands/cargo-bench.html)
- [Criterion.rs documentation](https://criterion-rs.github.io/book/user_guide/command_line_output.html)
- [Iai-Callgrind docs](https://docs.rs/iai-callgrind/latest/iai_callgrind/)

## 3. Refactor Target

The next architecture refactor should create three explicit subsystems.

### 3.1 Contact Manager / Manifold Manager

Move pair lifecycle, manifold refresh, contact matching, contact reduction, and warm-start cache ownership out of `ContactConstraint` and mostly out of `Scene`.

Current smell:

- `Scene::collision_detective` owns pass lifecycle timing.
- `ContactConstraint` owns active/inactive flags, pending contact pairs, cached lambda, stable contact key, object pointers, and solver state.
- `collision` only returns `Vec<ContactPointPair>`, so contact identity has to be reconstructed later.

Target shape:

```text
BroadphasePair
  -> NarrowphaseResult
  -> ManifoldUpdate
  -> PersistentManifold
  -> SolverContactBatch
```

Acceptance:

- Re-contact after inactive pass cannot inherit stale lambda.
- Continuing contact can transfer cached impulses by stable identity.
- Duplicate, degenerate, ambiguous, or feature-drifted contacts drop cache safely.
- Contact matching can be tested without running the whole `Scene`.
- Manifold lifecycle has explicit states and drop reasons.

### 3.2 Solver World

Move solver execution away from direct long-lived `*mut Element` state inside constraints.

Current smell:

- contact, join, and point constraints store or receive raw object pointers during solve.
- `Scene::pre_solve_constraints` has to borrow world state through raw pointers to work around Rust borrowing.
- storage mutation clears manifolds because constraints may still contain stale element pointers.

Target shape:

```text
World/ElementStore
  -> SolverBodySet
  -> SolverConstraintBatch
  -> velocity solve / position solve
  -> writeback
```

Acceptance:

- persistent constraint state does not store raw `*mut Element`.
- solver can run from compact `SolverBody` / `SolverContact` records.
- writeback to `ElementStore` is explicit and centralized.
- storage mutation does not require clearing all contact cache purely for pointer safety.
- contact, join, and point constraints share a consistent solver input model.

### 3.3 Shape Proxy / Collider Proxy

Move hot-path collision data into explicit proxies rather than repeatedly going through trait-object shape APIs.

Current smell:

- `Element` stores `shape: Box<dyn ShapeTraitUnion>`.
- `Collider::sub_colliders()` returns boxed iterators.
- broadphase builds temporary AABBs each pass.
- narrowphase receives `dyn SubCollider` and loses concrete shape type information.

Target shape:

```text
Shape asset/local geometry
  -> ShapeProxy / ColliderProxy
  -> AABB/support/feature cache
  -> broadphase + narrowphase + manifold
```

Acceptance:

- shape transform invalidation updates explicit proxy data.
- AABB/support/contact feature data can be reused across collision stages.
- concave sub-collider proxies are stable enough to identify source pieces.
- specialized narrowphase can be added without changing public shape API.

## 4. Where Acceptance Material Should Live

Use separate locations for separate purposes:

| Material | Location | Reason |
| --- | --- | --- |
| Refactor direction and acceptance policy | `docs/design/architecture-refactor-requirements.md` | Future-facing design intent. |
| Current architecture after implementation | `docs/architecture/*.md` | Describes actual structure, diagrams, and module ownership. |
| Step-by-step implementation tasks | `docs/plans/YYYY-MM-DD-architecture-refactor-*.md` | Executable milestone plan. |
| Deterministic scenario tests | `crates/picea/tests/` or focused module tests | Asserts behavior without windows. |
| Human/visual demos | `crates/picea/examples/` | Manual visual smoke and public usage examples. |
| Performance benchmarks | `crates/picea/benches/` | Standard Cargo benchmark discovery. |
| Benchmark result archives | `docs/perf/YYYY-MM-DD-*.md` or `docs/perf/results/*.json` | Keeps machine/commit/command/result evidence out of code. |
| Debug/replay artifacts | shape in `docs/ai/debug-artifacts.md` | Shared trace/snapshot format for failures. |

Do not use examples as the only acceptance gate. Examples should compile and remain useful, but architecture acceptance needs deterministic tests and benchmarks.

## 5. Acceptance Standards

An architecture milestone is acceptable only if it passes all relevant gates below.

### 5.1 Correctness Gates

Minimum existing gate:

```bash
rtk proxy cargo fmt --all --check
rtk proxy cargo test -p picea --lib
rtk proxy cargo test -p picea --examples --no-run
rtk proxy cargo test -p picea-web --lib
rtk proxy cargo test -p picea-macro-tools
rtk proxy cargo test --workspace --all-targets --no-run -- -D warnings
rtk git diff --check
```

Architecture milestones must also add targeted tests for the subsystem being split:

- contact manager: lifecycle, matching, reduction, stale cache, feature drift
- solver world: body extraction, solve, writeback, invalid handle behavior
- shape proxy: transform invalidation, AABB/support cache, concave sub-piece stability

### 5.2 Behavioral Scenario Gates

Each architecture milestone should include at least one deterministic scenario test from the scenario family it affects.

Recommended scenario families:

| Scenario | Existing Example Inspiration | What It Proves |
| --- | --- | --- |
| Resting stack | `stack`, `stack2`, `ground` | contact manifold persistence, position correction, sleeping. |
| Broadphase scale | `accumulation` | candidate filtering, AABB/proxy cost, many-body scaling. |
| Concave terrain | `pit`, `accumulation` | concave decomposition/proxy stability, sub-collider identity. |
| Constraint chain | `bridge`, `link` | join/point solver writeback and graph correctness. |
| Cloth grid | `cloth` | many constraints, solver batch pressure, deterministic stepping. |
| Friction slope | `friction` | tangent impulse continuity and friction stability. |
| Elastic transfer | `newton_cradle`, `conservation_of_momentum` | restitution, energy drift, warm-start side effects. |
| Sleep/wakeup | no single current example | sleep threshold, wakeup from collision/constraint, no stale state. |

Scenario tests should assert numeric or structural outcomes, not screenshots.

Examples can still mirror these families, but examples should be treated as user-facing demos rather than pass/fail proof.

### 5.3 Performance Gates

Performance acceptance should use two layers.

Layer 1: Criterion wall-clock benchmarks

Recommended file:

```text
crates/picea/benches/physics_scenarios.rs
```

Recommended command:

```bash
rtk proxy cargo bench -p picea --bench physics_scenarios
```

Recommended benchmark groups:

- `step/stack_10_50_100`
- `step/circles_100_500_1000`
- `collision/broadphase_sparse_dense`
- `collision/concave_sub_colliders`
- `solver/contact_pile`
- `solver/constraint_chain`
- `shape/proxy_transform_sync`
- `manifold/contact_refresh_transfer`

Metrics to record:

- time per fixed step
- elements per second
- contacts per second
- active manifold count
- broadphase candidate count
- narrowphase contact count
- max penetration after solve
- deterministic state hash after N steps

Layer 2: optional instruction/allocation profiling

Use Iai-Callgrind or Valgrind-backed runs for noisy hot-path changes, especially storage/layout work. These are not required for every milestone because they may depend on local tooling, but they are valuable before accepting a data-layout claim.

Suggested use:

- storage/handle refactor
- shape proxy cache refactor
- broadphase replacement
- solver batching

### 5.4 Regression Thresholds

Until the first benchmark baseline is established, do not invent absolute performance numbers. Establish a baseline from current `main`, then compare the refactor branch against it.

Initial policy:

- A correctness-focused architecture milestone may accept up to 5% wall-clock regression in unrelated benchmark groups if documented.
- A hot-path architecture milestone should not regress its target benchmark group unless it removes a correctness bug and the regression is explicitly accepted.
- A performance-claiming milestone must improve at least one target group by a measured amount and must not hide regressions in another target group.
- Any benchmark delta above 5% needs a short explanation: expected, unexplained, or blocked by measurement noise.

Once the benchmark suite stabilizes, replace this with per-scenario thresholds.

## 6. How To Actually Accept A Refactor

Use this acceptance sequence.

1. Freeze the baseline commit.
   - Record branch, `HEAD`, machine, OS, Rust version, and relevant feature flags.
   - Run the correctness gate on baseline.

2. Run baseline benchmarks.
   - Run the selected `cargo bench` suite.
   - Save output or summary under `docs/perf/`.
   - Record machine load caveats.

3. Implement the architecture milestone with TDD.
   - First add failing tests for the subsystem boundary.
   - Keep scope inside the milestone.
   - Do not claim performance without benchmark evidence.

4. Run targeted tests and full correctness gate.
   - Include old regression tests from M0-M8.
   - Include new subsystem tests.
   - Include scenario tests.

5. Run benchmarks on the refactor branch.
   - Compare against the saved baseline.
   - Record improvements/regressions and expected reasons.

6. Review in two layers.
   - Spec review: boundary, acceptance, non-goals.
   - Code review: correctness, ownership, stale state, hot-path allocations, test holes.

7. Update docs.
   - Update `docs/architecture/` when the actual structure changes.
   - Update `docs/design/` if the direction changes.
   - Update `docs/plans/` with milestone execution evidence.

8. Supervisor acceptance.
   - Accept only if correctness, scenario, benchmark, and documentation gates are all accounted for.
   - If a gate is intentionally skipped, record why and whether it blocks merge.

## 7. Example Direction

Future examples should be split into two kinds.

### Demo Examples

Stay in `crates/picea/examples/`.

Purpose:

- show public API usage
- provide visual sanity checks
- help humans understand scenes

Recommended additions:

- `many_bodies.rs`: circles/rectangles at scale, shows broadphase pressure
- `sleep_wakeup.rs`: stacked bodies entering sleep and waking on impact
- `concave_terrain.rs`: named replacement or cleanup around `pit`
- `stable_contacts.rs`: visualizes persistent contact points and warm-start cache

### Deterministic Acceptance Scenarios

Prefer integration tests under `crates/picea/tests/`.

Purpose:

- fixed `dt`
- fixed initial state
- no window
- numeric assertions
- deterministic final state hash

Recommended scenario tests:

- `stack_settles_without_excess_penetration`
- `many_sparse_bodies_keep_broadphase_candidates_bounded`
- `concave_pit_keeps_sub_collider_contacts_stable`
- `bridge_chain_remains_connected`
- `friction_slope_stops_or_slides_with_expected_direction`
- `newton_cradle_preserves_directional_momentum_within_tolerance`
- `sleeping_stack_wakes_on_collision`

## 8. First Refactor Milestone Recommendation

Start with Contact Manager / Manifold Manager.

Reason:

- M5 and M8 already exposed the most fragile lifecycle bugs.
- Current contact identity is conservative but not feature-based.
- This boundary can be refactored before generation handles and solver batching.
- It gives immediate value to both correctness and performance measurements.

Suggested first architecture milestone:

```text
M9 Contact Manager Extraction
```

Scope:

- introduce `ContactManager` or equivalent
- move active/inactive/pending lifecycle out of `ContactConstraint`
- keep solver formulas unchanged
- keep public API unchanged
- preserve M5/M8 behavior locks
- expose contact update/drop reasons for tests/debug artifacts

Out of scope:

- generation arena
- broadphase replacement
- solver batching
- wasm API changes
- feature-id-quality rewrite beyond what contact manager needs

Acceptance:

- all M8 contact identity tests still pass
- new manager-level lifecycle tests pass without `Scene`
- scene-level stale warm-start tests pass
- no public API changes
- benchmark baseline added for `manifold/contact_refresh_transfer`

Execution evidence (2026-04-20, M9):

- `ContactManager` / manager entry now owns active, inactive, pending refresh, and warm-start eligibility state; `ContactConstraintManifold` remains the existing public wrapper and delegates to the manager.
- `ContactConstraint` no longer exposes collision-pass lifecycle methods to `Scene`; it retains solver contact infos, cached impulses, pre-solve state, and velocity/friction/position solve methods.
- `Scene::collision_detective` now starts a manager pass and ingests current contact pairs through manager APIs. Warm start, pre-solve, velocity solve, position fix, and debug collision view read manager active / warm-start eligible iterators.
- Behavior locks: `contact_manager_*`, `contact_identity`, continuing refresh, and stale warm-start tests passed under targeted commands recorded in the milestone plan.
- Benchmark baseline: `crates/picea/benches/physics_scenarios.rs` adds Criterion `manifold/contact_refresh_transfer`, runnable with `rtk proxy cargo bench -p picea --bench physics_scenarios -- --test`.

Residual risk after M9:

- Manager still wraps `ContactConstraint`, so raw object pointer pre-solve state remains until M10 SolverWorld.
- Manager lifecycle states are test-visible but not yet a formal debug/drop-reason artifact surface.
- Contact reduction, persistent allocation optimization, and feature-id-quality rewrite remain future work.
- `ContactConstraintManifold` preserves common map-like read/query/mutable-iteration methods, but the exact old `Deref<Target = BTreeMap<...>>` identity is not restored.
