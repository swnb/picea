# Picea Lab Live Session Semantics

This document fixes the M25-B product and engineering boundary for `picea-lab`
live debugging. It intentionally does not turn the web workbench into a visual
world editor yet.

## Terms

- `artifact replay`: the server runs a scenario to completion, writes artifacts
  such as `frames.jsonl`, and the web replays those recorded facts.
- `live session`: the server owns a long-lived `World + SimulationPipeline` and
  advances it only when the client asks for a backend `step`.
- `reset-time override`: input applied before a world is built, such as gravity
  or frame count. It is deterministic because the world is recreated from a
  known scenario boundary.
- `running-world patch`: mutation of an already built world. This includes body,
  collider, or joint patch/destroy commands after live stepping has started.
- `transaction`: a batch of mutations that either all validates and commits, or
  leaves the authoritative world unchanged.
- `handle invalidation`: the rule that destroyed body/collider/joint handles
  become stale and must not silently resolve to newly reused slots.
- `query sync`: `QueryPipeline::sync` rebuilds the read-side query cache from a
  world revision. Queries before sync may reflect the older cached revision.

## Current M25-B Decision

M25-B is a semantics and red-line milestone. The shipped server behavior remains:

- `artifact_replay` accepts reset-time overrides through
  `PATCH /api/sessions/:id/overrides`, then `reset` reruns the artifact.
- `live_session` rejects `PATCH /api/sessions/:id/overrides` with an explicit
  `400` error and does not mutate the authoritative live runtime.
- `play` / `run` on a live session only changes the session status. Physics moves
  forward only through backend `step` requests.
- `reset` on a live session rebuilds the world from scenario plus reset-time
  overrides and clears the live frame buffer.

This is the conservative product choice. It keeps web debugging honest: the page
can drive backend physics, but it cannot yet edit a world in ways that imply
unsettled handle, contact-cache, sleep, and query-cache guarantees.

## Why Not Implement Arbitrary Live Patch Now

Arbitrary live patch is not just a UI feature. It changes the core runtime
contract in several places:

- Body, collider, and joint handles can become stale or be recycled with new
  generations after destroy/recreate.
- Contact warm-start and retained contact state may be invalid after collider or
  body topology changes.
- Sleeping bodies must wake with an inspectable reason when a patch changes
  pose, velocity, sleep flags, mass, collider shape, or collision eligibility.
- Query results must be tied to a `WorldRevision`; otherwise web inspection can
  mix new world state with old query cache results.
- Artifact provenance must say whether a frame came from scenario construction,
  reset-time overrides, or an accepted transaction.

Committing to these rules through a half-editor would make future physics fixes
look like UI regressions. M25-B therefore locks the refusal boundary first.

## Future Paused Patch Contract

A later milestone may add a paused-only patch endpoint. The minimal acceptable
contract is:

1. The session must be `paused` or `created`; `running` and `completed` sessions
   reject patch.
2. The request carries a bounded transaction of `WorldCommand`-like operations,
   not arbitrary world mutation.
3. The server validates the transaction on a scratch clone, using the existing
   `WorldCommands` atomic apply path.
4. On failure, the real live world, live frame buffer, status, and latest frame
   stay unchanged.
5. On success, the server commits the new world, wakes affected bodies when
   needed, clears or rebuilds retained contact state, and requires query
   pipelines to resync to the new world revision before answering inspection
   queries.
6. The response includes transaction provenance and any handle invalidation facts
   needed by the web inspector.
7. Seeking remains read-only: it browses already buffered frames and never rolls
   back the authoritative world.

## Existing Behavior Locks

The current repository already covers the low-level contracts that future live
patch must reuse:

- `core_model_world::world_commands_patch_and_destroy_are_atomic_on_handle_errors`
  proves rejected batches do not leak earlier patches into the real world.
- `core_model_world::world_commands_cover_collider_joint_paths_and_step_after_batch`
  proves collider/joint patch and destroy events are structured and stale handles
  stay invalid after destroy.
- `query_debug_contract::query_pipeline_ray_and_aabb_queries_require_sync_to_drop_stale_and_recycled_handles`
  proves stale query caches stay revision-bound until `sync`.
- `query_debug_contract::query_pipeline_sync_invalidates_cached_geometry_after_pose_and_shape_changes`
  proves pose and shape patches require query resync before inspection uses new
  geometry.
- `world_step_review_regressions` covers rejected world mutations preserving
  revision and contact/sleep review regressions around retained runtime state.
- `server_routes::live_session_overrides_patch_is_rejected_without_mutating_live_runtime`
  proves live override rejection does not mutate session metadata or the next
  backend step.

## Product Upgrade Path

- M25-B, current: refuse live patch explicitly, document semantics, keep live
  physics request-driven, and rely on reset-time overrides for deterministic
  scenario changes.
- Next patch slice: add a paused-only transaction preview endpoint that validates
  and returns proposed handle/query/provenance effects without committing.
- Commit slice: allow a small transaction subset after preview is stable, with
  query resync and provenance visible in the web inspector.
- Editor slice: only after the transaction subset is stable, expose web controls
  for body/collider/joint edits.

