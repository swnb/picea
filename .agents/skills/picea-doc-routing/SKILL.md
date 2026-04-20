---
name: picea-doc-routing
description: Maintain Picea repository AI navigation and routing documents. Use when Codex needs to create or update AGENTS.md, docs/ai/index.md, docs/ai/repo-map.md, docs/ai/doc-catalog.yaml, docs/ai/skill-candidates.md, or repository-local skills after module layout, milestone scope, validation commands, or documentation authority changes.
---

# Picea Doc Routing

## Overview

Keep Picea's AI entrypoints small, routed, and current. This is a documentation governance skill, not a physics implementation skill.

## Workflow

1. Confirm the live repo facts before editing docs:
   - `rtk proxy git status --short --branch`
   - `rtk proxy git rev-parse --short HEAD`
   - `rtk proxy find . -maxdepth 4 -type f -print`
2. Read only the routing sources needed for the change:
   - `AGENTS.md`
   - `docs/ai/index.md`
   - `docs/ai/repo-map.md`
   - `docs/ai/doc-catalog.yaml`
   - `docs/ai/skill-candidates.md`
   - `docs/plans/2026-04-18-picea-physics-engine-milestones.md`
3. Update the smallest set of routing artifacts:
   - Root rules belong in `AGENTS.md`; keep it short.
   - Question-to-source routing belongs in `docs/ai/index.md`.
   - Module ownership and validation commands belong in `docs/ai/repo-map.md`.
   - Document/code discoverability belongs in `docs/ai/doc-catalog.yaml`.
   - Future workflow ideas belong in `docs/ai/skill-candidates.md`.
   - Stable repeated workflows can become `.agents/skills/<skill-name>/SKILL.md`.
4. Verify the route, authority, and freshness story:
   - `rtk rg -n "TODO|\\[TODO" AGENTS.md docs/ai .agents/skills -g "!**/picea-doc-routing/SKILL.md"`
   - `rtk proxy ruby -e 'require "yaml"; YAML.load_file("docs/ai/doc-catalog.yaml"); puts "yaml ok"'`
   - `rtk proxy git diff --check`

## Rules

- Do not add long-lived status snapshot docs unless the user explicitly asks for one.
- Treat current git status and command output as higher authority than docs.
- Do not duplicate large architecture text in `AGENTS.md`; point to routed docs instead.
- Do not create a project skill for one-off module details. Promote only repeated development norms.
- Do not change `crates/**` while doing routing-only work.
- Protect unrelated dirty files; never rewrite docs by regenerating everything.

## Skill Promotion Policy

Only promote a workflow into `.agents/skills/` when all are true:

- The workflow repeats across sessions.
- The failure mode is costly if forgotten.
- The skill can stay procedural rather than becoming a stale status summary.
- It has clear trigger language in frontmatter.
