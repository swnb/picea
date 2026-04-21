# picea-lab

`picea-lab` is the artifact-driven tooling crate for Picea observability.

It is the place to:

- capture example artifacts
- replay deterministic scenarios
- diff two artifact directories
- export verification markdown
- open the native artifact viewer
- serve and inspect the same artifacts in the browser viewer

It does not run a separate physics engine. It consumes the artifact shapes produced by `picea::tools::observability`.

## CLI Commands

Capture a one-step contact fixture:

```bash
cargo run -p picea-lab -- capture-contact target/picea-lab/runs/contact-smoke
```

Replay a deterministic contact scenario for several steps:

```bash
cargo run -p picea-lab -- replay-contact \
  target/picea-lab/runs/replay-a \
  replay-a \
  1.5 \
  3
```

Capture artifacts for a named benchmark-style scenario:

```bash
cargo run -p picea-lab -- capture-benchmark \
  target/picea-lab/runs/bench-contact \
  bench-contact \
  contact_refresh_transfer \
  2
```

Diff two artifact directories:

```bash
cargo run -p picea-lab -- diff \
  target/picea-lab/runs/replay-a \
  target/picea-lab/runs/replay-b
```

Open the native viewer:

```bash
cargo run -p picea-lab -- view target/picea-lab/runs/replay-a
```

Export a human-readable verification summary:

```bash
cargo run -p picea-lab -- export-verification \
  target/picea-lab/runs/replay-a \
  target/picea-lab/runs/replay-a/verification.md
```

## Artifact Layout

Each artifact directory uses the same filenames:

- `final_snapshot.json`
- `debug_render.json`
- `trace.jsonl`
- `perf.json`
- `trace.perfetto.json`

These files are shared across:

- headless replay/diff
- native `egui/eframe` viewer
- browser viewer under `web/`

## Native Viewer

The native viewer is implemented in `src/viewer.rs`.

It can inspect:

- shapes and contacts in the viewport
- manifolds
- phase timeline events
- element / pair / phase filters
- replay / benchmark recipe parameters with regenerate
- contact radius / normal scale / overlay visibility controls
- wheel zoom, drag pan, click selection for contact pairs or elements
- verification markdown export

## Browser Viewer

The browser viewer is a no-build static app in `web/`.

To serve it locally:

```bash
cd crates/picea-lab/web
python3 -m http.server 4177 --bind 127.0.0.1
```

Then open:

```text
http://127.0.0.1:4177/
```

It loads the bundled `web/fixtures/contact-smoke/` artifact set by default, and it can also read user-selected artifact files with the same schema.

## Tests

Run the crate tests:

```bash
cargo test -p picea-lab
```

This covers:

- deterministic replay and diff
- benchmark artifact capture
- native viewer model filtering/export
- web viewer asset and fixture discovery

Codex/agent sessions in this repository should prefix cargo commands with `rtk proxy`; see the root `AGENTS.md`.

## Related Docs

- `../../AGENTS.md`
- `../../docs/ai/repo-map.md`
- `../../docs/ai/debug-artifacts.md`
- `../../docs/design/picea-lab-observability-architecture.md`
