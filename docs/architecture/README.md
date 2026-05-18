# Picea Architecture

This directory contains architecture-level documentation for Picea.

Read these documents when you need to understand how the repository is structured, how the runtime advances simulation, or where collision and constraint responsibilities live.

## Documents

- `system-overview.md`: workspace, crate boundaries, core module ownership, and high-level dependency graph.
- `legacy-scene-runtime.md`: merged archive for the removed `Scene` / `Context` runtime and collision/constraint path.

## Authority

Architecture docs describe the intended and observed structure at the time they were written. When these docs conflict with current code or test output, trust the current code and update the docs in the same change.
