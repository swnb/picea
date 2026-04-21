# picea-web

`picea-web` is the wasm-bindgen facade over the core `picea` physics engine.

It does not implement a second physics runtime. Instead, it exposes a JS/TS-facing API for:

- creating a scene
- creating and cloning elements
- stepping the scene with `tick(dt)`
- reading back shape and element state
- creating point and join constraints
- iterating visible shapes with `forEachElement`

## Entry Point

The crate exports a top-level `createScene()` function and the `WebScene` API described in `src/type.d.ts`.

Typical browser-side flow:

```ts
import init, { createScene, setPanicConsoleHook } from "picea-web";

await init();
setPanicConsoleHook();

const scene = createScene();
scene.setGravity({ x: 0, y: -9.8 });

const groundId = scene.createRect(-20, 0, 40, 2, { isFixed: true });
const ballId = scene.createCircle(0, 10, 1, { mass: 1 });

scene.tick(1 / 60);

scene.forEachElement((shape) => {
  console.log(shape.id, shape.shapeType, shape.centerPoint);
});
```

## API Shape

The public surface intentionally keeps two styles:

- legacy-compatible methods such as `createRect`, `createCircle`, `updateElementMeta`
- strict `try*` methods such as `tryCreateRect`, `tryCreateCircle`, `tryUpdateElementMeta`

The difference is:

- legacy methods return fallback values like `0`, `undefined`, `null`, or `[]`
- `try*` methods return JS errors for invalid input or missing objects

When in doubt, prefer the `try*` methods in new code.

Useful operations include:

- `setGravity` / `trySetGravity`
- `createRect`, `createCircle`, `createRegularPolygon`, `createPolygon`, `createLine`
- `cloneElement`
- `tick`
- `getElementVertices`, `getElementCenterPoint`
- `isElementCollide`
- `createPointConstraint`, `createJoinConstraint`
- `forEachElement`

## Development Notes

- `picea-web` owns the JS/Rust boundary, validation, fallback semantics, and TypeScript-facing shape.
- It should stay thin. Core solver and collision behavior still belongs in `crates/picea`.
- `src/type.d.ts` is the canonical TypeScript-facing contract summary for this crate.

## Run Tests

Native-facing library tests:

```bash
cargo test -p picea-web --lib
```

When the wasm-bindgen runner is installed and version-compatible, run wasm-target smoke:

```bash
CARGO_TARGET_WASM32_UNKNOWN_UNKNOWN_RUNNER=wasm-bindgen-test-runner \
  cargo test -p picea-web --lib --target wasm32-unknown-unknown
```

Codex/agent sessions in this repository should prefix cargo commands with `rtk proxy`; see the root `AGENTS.md`.

## Related Docs

- `../../AGENTS.md`
- `../../docs/ai/repo-map.md`
- `../../docs/ai/index.md`
- `../../docs/plans/2026-04-18-picea-physics-engine-milestones.md`
