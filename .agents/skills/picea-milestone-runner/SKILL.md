---
name: picea-milestone-runner
description: Execute, review, or supervise one Picea physics-engine milestone under the repository milestone gates. Use when the user asks to implement, continue, validate, review, or strictly stop at a named Picea milestone, or when work must follow the docs/plans/2026-04-25-picea-physics-engine-production-milestones.md boundary, test-first behavior locks, and supervisor acceptance flow.
---

# Picea Milestone Runner

## Overview

Run exactly one Picea milestone without drifting into adjacent physics-engine work. This skill preserves the project's milestone boundaries, verification gates, and review chain.

## Read Order

1. `AGENTS.md`
2. `docs/plans/2026-04-25-picea-physics-engine-production-milestones.md`
3. `docs/ai/repo-map.md`
4. `docs/ai/debug-playbook.md` when the task is a bug or regression
5. The smallest code module needed for the named milestone

`docs/plans/2026-04-18-picea-physics-engine-milestones.md` is archive/history
only; do not use its old milestone labels as current routing authority.

## Workflow

1. Confirm the live repo state:
   - `rtk proxy git status --short --branch`
   - `rtk proxy git rev-parse --short HEAD`
2. Name the active milestone and quote its hard boundary from the plan.
3. State what is in scope and what is explicitly out of scope.
4. Start with a failing behavior lock or focused regression test.
5. Implement the smallest change that turns the behavior lock green.
6. Run the milestone-specific targeted tests, then the required broader gates.
7. Record residual risks and boundaries not touched.

## Review Chain

Use this order for milestone work:

1. Implementer: behavior lock, minimal implementation, targeted verification.
2. Spec Reviewer: check milestone scope, hard boundaries, and acceptance criteria.
3. Code Reviewer: findings-first review for bugs, panic, NaN, unsafe, stale handles, and weak tests.
4. Supervisor Acceptance: final decision after verification and review are closed.

If the user explicitly asks to use subagents, split implementer/spec/code review into separate agents. Keep the main thread responsible for integration and final acceptance.

## Guardrails

- Do not enter the next milestone unless the user explicitly asks.
- Do not reframe a red gate as "good enough"; diagnose or record the blocker.
- Do not commit or push unless the user asks.
- Do not overwrite unrelated dirty files.
- Do not use workaround-first fixes for physics behavior.
- Do not change public wasm/API contracts unless the target milestone includes it.

## Common Gates

Use the plan as source of truth, but these are common Picea gates:

- `rtk proxy cargo test -p picea --lib`
- `rtk proxy cargo test -p picea --examples --no-run`
- `rtk proxy cargo test -p picea-macro-tools`
- `rtk proxy cargo test -p picea-web --lib`
- `rtk proxy cargo test --workspace --all-targets --no-run -- -D warnings` when warning cleanliness is in scope
