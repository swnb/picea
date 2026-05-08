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
- `running-world patch`: mutation of an already built world. The current narrow
  exception is live gravity apply, which only replaces `WorldDesc.gravity` and
  refreshes the current frame. Body, collider, or joint patch/destroy commands
  after live stepping has started remain outside the shipped live debugger.
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
- `live_session` accepts the narrow `POST /api/sessions/:id/gravity` runtime
  patch. It validates the supplied session epoch, updates world gravity, bumps
  world revision and session epoch, resyncs query state, clears velocity preview
  cache, refreshes the current authoritative frame, and truncates any future
  buffered frames invalidated by the edit.
- `play` / `run` on a live session only changes the session status. Physics moves
  forward only through backend `step` requests.
- `reset` on a live session rebuilds the world from scenario plus reset-time
  overrides and clears the live frame buffer. A successfully applied live
  gravity patch updates those gravity overrides so reset continues with the last
  applied gravity.

This is the conservative product choice. It keeps web debugging honest: the page
can drive backend physics, but it cannot yet edit a world in ways that imply
unsettled handle, contact-cache, sleep, and query-cache guarantees.

## Runtime Controls V1

The 2026-05-07 lab-web runtime controls keep session creation separate from
playback:

- The header run button creates a new session for the selected scenario and
  mode. If a session already exists, it is a restart/rerun command, not a
  play/resume toggle.
- The timeline play button is the only pause/resume toggle for the current
  session. Pausing then resuming continues that session instead of creating a
  new one.
- Live sessions may be unbounded at creation time. They continue stepping past
  the requested display frame count while retaining only a bounded frame window.
  The V1 default retains 600 frames; older evicted live frames are not
  authoritative replay facts and are reported as evicted instead of being
  silently reconstructed by the web client.
- Artifact replay remains finite-frame replay. It does not inherit live
  unbounded stepping or live retained-window behavior.
- The gravity dial has two modes. In artifact replay it remains a reset-time
  override for the next header rerun. In live session it edits a draft `[x, y]`
  vector; clicking Apply sends the narrow live gravity patch immediately.
  Reset restores the draft to default gravity, and Undo either discards an
  unapplied draft edit or applies the previous live gravity value.
- Live gravity apply is deliberately not a general editor. It does not create,
  destroy, or patch bodies, colliders, joints, contacts, sleep state, material,
  collision filters, or solver settings.

## Live Frame Detail Contract

Live playback may request either a lightweight summary frame or a full
`FrameRecord`. This is a transport/detail choice only; the Rust live runtime
remains authoritative and physics still advances only through backend `step`
requests.

- `summary` is the default for running live playback. It contains enough
  authoritative data to draw the latest frame and keep session freshness gates:
  frame index, simulated time, state hash, session id, session epoch, world
  revision, status, buffered frame counts, body/collider transforms, body type
  and compact stats.
- `full` returns the existing `FrameRecord` facts for inspection, diagnostics,
  evidence, copy-debug-context, and paused velocity perturbation gates.
- Summary frames must not be interpreted as full diagnostic facts. Missing
  `diagnostics`, `events`, contact lists, broadphase tree, island details, or
  provenance details means "not hydrated", not "zero issues".
- A full-frame response is usable only when its session id, session epoch, frame
  index, and world revision still match the active live session. Stale responses
  are discarded by the web client.
- Hydrating a summary frame into a full frame invalidates any marker,
  diagnostics, or missing-evidence cache derived from the summary-only view.

The expected routes are:

- `POST /api/sessions/:id/control` with `action = "step"` and
  `detail = "summary" | "full"`.
- `GET /api/sessions/:id/frames/:index` for on-demand full frame lookup from the
  live buffer.

`artifact replay` remains full-frame based. This avoids changing artifact schema
or weakening existing diagnostics contracts while still making live playback
cheap enough to degrade gracefully on dense scenes.

## M37-A Velocity Perturbation Preview

M37-A adds the first paused-world preview, but only as a server-side read model:

- `POST /api/sessions/:id/velocity-perturbations/preview` accepts an
  `action_id`, `world_revision`, `session_epoch`, `body_handle`, `frame_index`,
  `requested_delta`, and optional `wake_intent`.
- The endpoint is read-only. It does not call `step`, `run_scenario`, artifact
  writing, live query sync, or `BodyPatch`; it reads the current
  `latest_frame` snapshot after proving that snapshot matches the authoritative
  live `World` revision, then computes `before_velocity + requested_delta`.
- A live session starts with `session_epoch = 0`. Live `reset` increments the
  epoch because it replaces the authoritative world and clears the live frame
  buffer. Preview never increments the epoch.
- `paused` live sessions may return a preview when the supplied revision, epoch,
  frame index, handle, body type, and velocity delta are fresh and valid.
- V1 preview requires an authoritative handle source: the requested
  `body_handle` must be present in the current `latest_frame` snapshot, and that
  snapshot must match the session's current frame index and live world revision.
  `created` live sessions do not have a current frame snapshot yet, including
  immediately after `reset`, so they reject preview with `rejection_reason`
  rather than accepting a handle supplied only by the client.
- `running`, `completed`, stale revision, stale epoch, stale frame, stale or
  foreign body handle, missing current frame snapshot, static body, kinematic
  body, zero/invalid delta, and non-finite delta all return a preview-shaped
  response with `rejection_reason` set.
- The response includes `action_id`, `session_id`, `world_revision`,
  `session_epoch`, `body_handle`, `frame_index`, `before_velocity`,
  `requested_delta`, `computed_target_velocity`, `wake_intent`, and
  `rejection_reason`.

This is intentionally not a commit endpoint. It gives the web or another client
enough deterministic information to render a proposed velocity change later,
without changing live world state, buffered frames, latest frame, status,
selection/query state, events, artifacts, or run ids.

## M37-B Velocity Perturbation Commit

M37-B adds the matching paused-only commit gate:

- `POST /api/sessions/:id/velocity-perturbations/commit` is a distinct route,
  not an arm of `control_session`. The request must carry back the preview
  `action_id`, `world_revision`, `session_epoch`, `body_handle`, `frame_index`,
  `requested_delta`, and may include `computed_target_velocity` for consistency
  checking.
- A successful preview writes a live-runtime transient transaction cache keyed
  by `action_id`. This cache is not physics state: it does not change the world,
  frame buffer, latest frame, status, events, run id, query cache, or session
  epoch. Commit consumes the cached action exactly once on success; rejected
  commits leave it available unless the action was already successfully used.
- Commit revalidates against the current authoritative live `World` and current
  `latest_frame`. It does not trust the client-provided velocity fields. The
  handle must still be present in the current frame snapshot, the session must
  still be paused, and the supplied revision, epoch, and frame index must still
  match.
- Rejection reasons include `not_live_session`, `session_created`,
  `session_running`, `session_completed`, `session_failed`,
  `missing_frame_snapshot`, `stale_world_revision`, `stale_session_epoch`,
  `stale_frame`, `invalid_body_handle`, `static_body`, `kinematic_body`,
  `invalid_velocity_delta`, `missing_preview_action`, `stale_preview_action`,
  `reused_action`, and `body_patch_failed`. Rejected commits do not mutate the
  live world, frame buffer, latest frame, status, run id, events, session epoch,
  query cache, or preview cache.
- Accepted commit is intentionally narrow: it calls
  `World::apply_body_patch` with only `linear_velocity` and `wake`. It does not
  add continuous force integration, torque accumulation, a drag constraint,
  mouse joint behavior, solver changes, contact changes, or CCD changes.
- On success, `apply_body_patch` bumps the world revision and the server bumps
  `session_epoch`. The live query cache is resynced after the patch. The server
  refreshes the current frame at the same `frame_index`, replaces that frame in
  the live buffer, and truncates any future buffered frames. This refreshed frame
  is not a simulation step: it preserves the frame index and simulated time while
  clearing step events/counters for the edit frame. If `/events` already has a
  queued frame event for that edit frame, the server updates it to the refreshed
  state hash; queued future frame events are dropped because their frames were
  invalidated by the edit.
- `FrameRecord.perturbation_provenance` is additive and separate from
  `compound_provenance`. Each accepted perturbation records action id, session
  id, accepted world revision, accepted session epoch, body handle, frame index,
  before velocity, requested delta, computed target velocity, wake intent,
  commit outcome, and query sync status. Later live step frames carry the same
  perturbation provenance history so the downstream trajectory can be traced
  back to the paused edit.

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
- `server_routes::live_velocity_perturbation_preview_is_read_only_for_paused_session`
  proves velocity preview returns computed target velocity without changing
  status, frame buffer, latest frame, run id, epoch, or queued events.
- `server_routes::live_velocity_perturbation_preview_rejects_stale_or_mutating_requests`
  proves the M37-A rejection reasons for running/completed sessions, stale
  revision/epoch/frame, invalid handles, static bodies, and invalid deltas, and
  proves rejected preview paths do not change status, epoch, frame buffers,
  latest frame, run id, or event queues.
- `server_routes::live_session_epoch_starts_at_zero_and_reset_increments_without_preview_increment`
  proves `session_epoch` starts at zero, live reset increments it, and preview
  leaves it unchanged. It also proves `created` preview after reset rejects until
  a new authoritative frame snapshot exists.
- `server::tests::live_velocity_preview_rejects_kinematic_body_from_current_snapshot`
  proves the kinematic body rejection branch against a current frame snapshot
  without adding a broader scenario or solver change.
- `server_routes::live_velocity_perturbation_commit_applies_previewed_velocity_and_provenance`
  proves previewed velocity commit is one-shot, bumps world revision and session
  epoch, refreshes the current frame without advancing its frame index, writes
  perturbation provenance, and lets the next backend step continue from the
  committed velocity.
- `server_routes::live_velocity_perturbation_commit_rejects_stale_or_unpreviewed_transactions`
  proves commit rejection for created status, missing preview action,
  stale revision, stale epoch, stale frame, invalid handle, static body, invalid
  delta, tampered preview payloads, running session, and completed session while
  preserving session state and leaving a rejected cached preview reusable.

## Product Upgrade Path

- M25-B, current: refuse live patch explicitly, document semantics, keep live
  physics request-driven, and rely on reset-time overrides for deterministic
  scenario changes.
- M37-A, current preview slice: expose a server-only paused velocity
  perturbation preview with revision/epoch/frame/latest-frame handle-source
  gates and no live mutation.
- M37-B, current commit slice: accept the previewed paused velocity
  perturbation as a one-shot absolute velocity patch, with query resync and
  additive frame provenance.
- Next patch slice: generalize this pattern toward other paused-only
  transactions that validate and return handle/query/provenance effects before
  committing.
- Editor slice: only after the transaction subset is stable, expose web controls
  for body/collider/joint edits.
