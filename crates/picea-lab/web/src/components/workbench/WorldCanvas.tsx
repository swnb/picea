import {
  useEffect,
  useMemo,
  useRef,
  useState,
  type PointerEvent,
  type WheelEvent,
} from "react";
import { Focus, Maximize2, RotateCcw, ZoomIn, ZoomOut } from "lucide-react";
import type {
  ActiveGrabRecord,
  DebugAabb,
  DebugBody,
  DebugBroadphaseTree,
  DebugCollider,
  DebugContact,
  DebugIsland,
  DebugPrimitive,
  DebugShape,
  FrameRecord,
  SelectedEntity,
  Vec2,
} from "../../types";
import { profileMeasure, profileStart } from "../../profile";
import { Button } from "../ui/button";
import { Tooltip } from "../ui/radix";
import type { CcdTrajectoryTrail, TrajectoryOverlay } from "./trajectory";
import type {
  CanvasDebugView,
  LatticeProxyEdge,
  LatticeProxyNode,
  LatticeProxySummary,
  LayerState,
  TrajectorySettings,
} from "./types";

type WorldCanvasProps = {
  frame: FrameRecord;
  frames: FrameRecord[];
  frameIndex: number;
  selected: SelectedEntity | null;
  layers: LayerState;
  trajectorySettings: TrajectorySettings;
  trajectoryOverlay: TrajectoryOverlay;
  latticeSummary: LatticeProxySummary;
  offlineWatermark: string | null;
  grabEnabled: boolean;
  activeGrab: ActiveGrabRecord | null;
  labels: {
    frame: string;
    colliders: string;
    contacts: string;
    zoom: string;
    scale: string;
    mode: string;
    modeFree: string;
    modeLockedCore: string;
    fit: string;
    lock: string;
    unlock: string;
    reset: string;
    zoomIn: string;
    zoomOut: string;
    targetActive: string;
    targetAllColliders: string;
    targetScene: string;
    trajectoryEmptySelectedBody: string;
    trajectoryEmptyAllDynamic: string;
    trajectoryEmptySelectedIsland: string;
    trajectoryEmptyContacts: string;
    trajectoryEmptyCcd: string;
    trajectoryEmptyUnsupportedSelection: string;
    entity: (kind: SelectedEntity["kind"], id: number) => string;
  };
  onSelect: (entity: SelectedEntity | null) => void;
  onGrabStart: (bodyHandle: number, point: Vec2) => void;
  onGrabMove: (point: Vec2) => void;
  onGrabEnd: () => void;
  onViewChange?: (view: CanvasDebugView) => void;
};

type Camera = {
  scale: number;
  origin: Vec2;
  width: number;
  height: number;
};

type CameraState = {
  mode: "free" | "locked_core";
  scale: number;
  origin: Vec2;
  lockedBounds: DebugAabb | null;
  lockedTargetDescription: string | null;
};

type PanGesture = {
  pointerId: number;
  start: Vec2;
  origin: Vec2;
  moved: boolean;
};

type GridSpacing = {
  minorStep: number;
  majorEvery: number;
};

type CameraFocusTarget = {
  description: string;
  bounds: DebugAabb;
};

type CameraFocusLabels = Pick<
  WorldCanvasProps["labels"],
  "targetActive" | "targetAllColliders" | "targetScene" | "entity"
>;

const CAMERA_DAMPING = 0.22;
const CAMERA_SETTLE_EPSILON = 0.0001;
const CAMERA_WHEEL_STEP = 1.16;

export function WorldCanvas({
  frame,
  frames,
  frameIndex,
  selected,
  layers,
  trajectorySettings,
  trajectoryOverlay,
  latticeSummary,
  offlineWatermark,
  grabEnabled,
  activeGrab,
  labels,
  onSelect,
  onGrabStart,
  onGrabMove,
  onGrabEnd,
  onViewChange,
}: WorldCanvasProps) {
  const canvasRef = useRef<HTMLCanvasElement | null>(null);
  const containerRef = useRef<HTMLDivElement | null>(null);
  const [size, setSize] = useState({ width: 900, height: 600 });
  const [isPanning, setIsPanning] = useState(false);
  const [canvasCameraState, setCanvasCameraState] = useState<CameraState | null>(null);
  const [cameraSmoothingDisabled, setCameraSmoothingDisabled] = useState(false);
  const panGesture = useRef<PanGesture | null>(null);
  const grabPointerId = useRef<number | null>(null);
  const cameraStateRef = useRef<CameraState | null>(null);
  const cameraTargetStateRef = useRef<CameraState | null>(null);
  const cameraAnimationFrameRef = useRef<number | null>(null);
  const fitCamera = useMemo(
    () => makeCamera(frame.snapshot.colliders, frame.snapshot.primitives, size),
    [frame, size],
  );
  const coreFocusTarget = useMemo(
    () => resolveCoreFocusTarget(frame, selected, labels),
    [frame, selected, labels],
  );
  const camera = useMemo(
    () => ({
      scale: canvasCameraState?.scale ?? fitCamera.scale,
      origin: canvasCameraState?.origin ?? fitCamera.origin,
      width: size.width,
      height: size.height,
    }),
    [canvasCameraState, fitCamera, size],
  );

  function cancelCameraAnimation() {
    if (cameraAnimationFrameRef.current !== null) {
      cancelAnimationFrame(cameraAnimationFrameRef.current);
      cameraAnimationFrameRef.current = null;
    }
  }

  function commitCameraState(next: CameraState) {
    cameraStateRef.current = next;
    setCanvasCameraState(next);
  }

  function setCameraImmediately(next: CameraState) {
    cancelCameraAnimation();
    cameraTargetStateRef.current = next;
    commitCameraState(next);
  }

  function readInteractionCameraState(): CameraState {
    return cameraTargetStateRef.current ?? cameraStateRef.current ?? buildFreeCameraState(fitCamera);
  }

  function stepCameraAnimation() {
    const current = cameraStateRef.current;
    const target = cameraTargetStateRef.current;
    if (!current || !target) {
      cameraAnimationFrameRef.current = null;
      return;
    }

    if (cameraSmoothingDisabled || cameraNearEqual(current, target)) {
      commitCameraState(target);
      cameraAnimationFrameRef.current = null;
      return;
    }

    const next: CameraState = {
      mode: target.mode,
      scale: lerp(current.scale, target.scale, CAMERA_DAMPING),
      origin: {
        x: lerp(current.origin.x, target.origin.x, CAMERA_DAMPING),
        y: lerp(current.origin.y, target.origin.y, CAMERA_DAMPING),
      },
      lockedBounds: target.lockedBounds,
      lockedTargetDescription: target.lockedTargetDescription,
    };

    if (cameraNearEqual(next, target, CAMERA_SETTLE_EPSILON)) {
      commitCameraState(target);
      cameraAnimationFrameRef.current = null;
      return;
    }

    commitCameraState(next);
    cameraAnimationFrameRef.current = requestAnimationFrame(stepCameraAnimation);
  }

  function scheduleCameraTransition(next: CameraState, options?: { immediate?: boolean }) {
    cameraTargetStateRef.current = next;
    if (options?.immediate || cameraSmoothingDisabled) {
      setCameraImmediately(next);
      return;
    }
    if (cameraAnimationFrameRef.current === null) {
      cameraAnimationFrameRef.current = requestAnimationFrame(stepCameraAnimation);
    }
  }

  useEffect(() => {
    const element = containerRef.current;
    if (!element) {
      return;
    }
    const observer = new ResizeObserver(([entry]) => {
      const rect = entry.contentRect;
      setSize({
        width: Math.max(320, Math.floor(rect.width)),
        height: Math.max(240, Math.floor(rect.height)),
      });
    });
    observer.observe(element);
    return () => observer.disconnect();
  }, []);

  useEffect(() => {
    if (!cameraStateRef.current) {
      setCameraImmediately(buildFreeCameraState(fitCamera));
    }
  }, [fitCamera]);

  useEffect(() => {
    cameraStateRef.current = canvasCameraState;
  }, [canvasCameraState]);

  useEffect(() => {
    const mediaQuery = window.matchMedia("(prefers-reduced-motion: reduce)");
    const syncPreference = () => {
      setCameraSmoothingDisabled(mediaQuery.matches);
    };
    syncPreference();
    mediaQuery.addEventListener("change", syncPreference);
    return () => mediaQuery.removeEventListener("change", syncPreference);
  }, []);

  useEffect(() => {
    if (cameraSmoothingDisabled && cameraTargetStateRef.current) {
      setCameraImmediately(cameraTargetStateRef.current);
    }
  }, [cameraSmoothingDisabled]);

  useEffect(() => {
    if (!coreFocusTarget) {
      return;
    }
    const previous = readInteractionCameraState();
    if (previous.mode !== "locked_core") {
      return;
    }
    const lockedBounds =
      previous.lockedTargetDescription !== coreFocusTarget.description
        ? coreFocusTarget.bounds
        : stabilizeLockedBounds(previous.lockedBounds, coreFocusTarget.bounds);
    const lockedCamera = makeLockedCoreCamera(lockedBounds, size);
    const nextState: CameraState = {
      mode: "locked_core",
      scale: lockedCamera.scale,
      origin: lockedCamera.origin,
      lockedBounds,
      lockedTargetDescription: coreFocusTarget.description,
    };
    if (
      previous.lockedTargetDescription === coreFocusTarget.description &&
      aabbNearEqual(previous.lockedBounds, lockedBounds) &&
      cameraNearEqual(previous, lockedCamera)
    ) {
      cameraTargetStateRef.current = nextState;
      return;
    }
    scheduleCameraTransition(nextState);
  }, [coreFocusTarget, size, cameraSmoothingDisabled]);

  useEffect(() => () => cancelCameraAnimation(), []);

  useEffect(() => {
    const canvas = canvasRef.current;
    if (!canvas) {
      return;
    }
    const ratio = window.devicePixelRatio || 1;
    canvas.width = Math.floor(size.width * ratio);
    canvas.height = Math.floor(size.height * ratio);
    canvas.style.width = `${size.width}px`;
    canvas.style.height = `${size.height}px`;

    const ctx = canvas.getContext("2d");
    if (!ctx) {
      return;
    }
    ctx.setTransform(ratio, 0, 0, ratio, 0, 0);
    const startedAt = profileStart();
    drawWorld(
      ctx,
      frame,
      frames.slice(0, frameIndex + 1),
      camera,
      layers,
      selected,
      trajectoryOverlay,
      trajectorySettings,
      latticeSummary,
      labels,
      activeGrab,
    );
    profileMeasure("canvas.drawWorld", startedAt, {
      frameIndex: frame.frame_index,
      bodies: frame.snapshot.bodies.length,
      colliders: frame.snapshot.colliders.length,
      contacts: frame.snapshot.contacts.length,
      previousFrames: frameIndex + 1,
      layers: Object.entries(layers)
        .filter(([, enabled]) => enabled)
        .map(([name]) => name)
        .join(","),
    });
  }, [
    activeGrab,
    camera,
    frame,
    labels,
    latticeSummary,
    layers,
    selected,
    size,
    trajectoryOverlay,
    trajectorySettings,
  ]);

  useEffect(() => {
    if (!onViewChange) {
      return;
    }
    const isLockedCore = canvasCameraState?.mode === "locked_core";
    const targetBounds = isLockedCore ? canvasCameraState.lockedBounds : null;
    const targetDescription = isLockedCore ? canvasCameraState.lockedTargetDescription : null;
    onViewChange({
      mode: canvasCameraState?.mode ?? "free",
      zoom: camera.scale,
      center: screenToWorld({ x: camera.width / 2, y: camera.height / 2 }, camera),
      targetDescription,
      targetBounds,
      dampingEnabled: !cameraSmoothingDisabled,
      grid: layers.grid,
      rulers: layers.rulers,
    });
  }, [camera, canvasCameraState, coreFocusTarget, cameraSmoothingDisabled, layers.grid, layers.rulers, onViewChange]);

  function handlePointerDown(event: PointerEvent<HTMLCanvasElement>) {
    if (grabEnabled && event.button === 0) {
      const rect = event.currentTarget.getBoundingClientRect();
      const point = screenToWorld(
        { x: event.clientX - rect.left, y: event.clientY - rect.top },
        camera,
      );
      const hit = hitTest(frame.snapshot.colliders, [], [], [], point);
      if (hit?.kind === "collider") {
        const collider = frame.snapshot.colliders.find(
          (entry) => entry.handle === hit.id,
        );
        const body = collider
          ? frame.snapshot.bodies.find((entry) => entry.handle === collider.body)
          : undefined;
        if (body && body.body_type === "dynamic") {
          grabPointerId.current = event.pointerId;
          event.currentTarget.setPointerCapture(event.pointerId);
          onGrabStart(body.handle, point);
          return;
        }
      }
    }
    const rect = event.currentTarget.getBoundingClientRect();
    const start = { x: event.clientX - rect.left, y: event.clientY - rect.top };
    panGesture.current = {
      pointerId: event.pointerId,
      start,
      origin: { ...camera.origin },
      moved: false,
    };
    event.currentTarget.setPointerCapture(event.pointerId);
    setIsPanning(true);
  }

  function handlePointerMove(event: PointerEvent<HTMLCanvasElement>) {
    if (grabPointerId.current === event.pointerId) {
      const rect = event.currentTarget.getBoundingClientRect();
      const point = screenToWorld(
        { x: event.clientX - rect.left, y: event.clientY - rect.top },
        camera,
      );
      onGrabMove(point);
      return;
    }
    const gesture = panGesture.current;
    if (!gesture || gesture.pointerId !== event.pointerId) {
      return;
    }
    const rect = event.currentTarget.getBoundingClientRect();
    const current = { x: event.clientX - rect.left, y: event.clientY - rect.top };
    const delta = { x: current.x - gesture.start.x, y: current.y - gesture.start.y };
    if (Math.hypot(delta.x, delta.y) > 3) {
      gesture.moved = true;
    }
    if (!gesture.moved) {
      return;
    }
    setCameraImmediately(
      buildFreeCameraState({
        ...camera,
        origin: {
          x: gesture.origin.x + delta.x,
          y: gesture.origin.y + delta.y,
        },
      }),
    );
  }

  function handlePointerUp(event: PointerEvent<HTMLCanvasElement>) {
    if (grabPointerId.current === event.pointerId) {
      grabPointerId.current = null;
      onGrabEnd();
      return;
    }
    const gesture = panGesture.current;
    panGesture.current = null;
    setIsPanning(false);
    event.currentTarget.releasePointerCapture(event.pointerId);
    if (gesture?.moved) {
      return;
    }
    const rect = event.currentTarget.getBoundingClientRect();
    const point = screenToWorld({ x: event.clientX - rect.left, y: event.clientY - rect.top }, camera);
    const latticeProxy = layers.lattice ? latticeSummary : null;
    const hit = hitTest(
      frame.snapshot.colliders,
      frame.snapshot.contacts,
      latticeProxy?.enabled ? latticeProxy.nodes : [],
      latticeProxy?.enabled ? latticeProxy.edges : [],
      point,
    );
    onSelect(hit);
  }

  function handlePointerCancel(event: PointerEvent<HTMLCanvasElement>) {
    if (grabPointerId.current === event.pointerId) {
      grabPointerId.current = null;
      onGrabEnd();
      return;
    }
    panGesture.current = null;
    setIsPanning(false);
    event.currentTarget.releasePointerCapture(event.pointerId);
  }

  function handleWheel(event: WheelEvent<HTMLCanvasElement>) {
    event.preventDefault();
    const rect = event.currentTarget.getBoundingClientRect();
    const focus = { x: event.clientX - rect.left, y: event.clientY - rect.top };
    zoomAt(focus, event.deltaY < 0 ? CAMERA_WHEEL_STEP : 1 / CAMERA_WHEEL_STEP);
  }

  function zoomAt(focus: Vec2, factor: number) {
    const interactionCamera = cameraStateToCamera(readInteractionCameraState(), size);
    const before = screenToWorld(focus, interactionCamera);
    const nextScale = clamp(interactionCamera.scale * factor, 12, 360);
    scheduleCameraTransition(
      buildFreeCameraState({
        ...interactionCamera,
        scale: nextScale,
        origin: {
          x: focus.x - before.x * nextScale,
          y: focus.y - before.y * nextScale,
        },
      }),
    );
  }

  function zoomCenter(factor: number) {
    zoomAt({ x: camera.width / 2, y: camera.height / 2 }, factor);
  }

  function fitCanvas() {
    setCameraImmediately(buildFreeCameraState(fitCamera));
  }

  function resetCanvas() {
    setCameraImmediately(buildFreeCameraState(fitCamera));
    onSelect(null);
  }

  function toggleCoreLock() {
    if (!coreFocusTarget) {
      setCameraImmediately(buildFreeCameraState(camera));
      return;
    }
    if (canvasCameraState?.mode === "locked_core") {
      setCameraImmediately(buildFreeCameraState(camera));
      return;
    }
    const lockedCamera = makeLockedCoreCamera(coreFocusTarget.bounds, size);
    scheduleCameraTransition({
      mode: "locked_core",
      scale: lockedCamera.scale,
      origin: lockedCamera.origin,
      lockedBounds: coreFocusTarget.bounds,
      lockedTargetDescription: coreFocusTarget.description,
    });
  }

  const cameraMode = canvasCameraState?.mode ?? "free";
  const modeLabel = cameraMode === "locked_core" ? labels.modeLockedCore : labels.modeFree;
  const lockTooltip = cameraMode === "locked_core" ? labels.unlock : labels.lock;

  return (
    <div ref={containerRef} className="relative h-full min-h-0 w-full overflow-hidden bg-lab-canvas">
      <canvas
        ref={canvasRef}
        className={`block h-full w-full touch-none ${isPanning ? "cursor-grabbing" : "cursor-grab"}`}
        onPointerDown={handlePointerDown}
        onPointerMove={handlePointerMove}
        onPointerUp={handlePointerUp}
        onPointerCancel={handlePointerCancel}
        onWheel={handleWheel}
      />
      {offlineWatermark ? (
        <div className="pointer-events-none absolute inset-0 z-10 flex items-center justify-center">
          <div className="-rotate-12 rounded-md border border-lab-line bg-lab-panel/60 px-6 py-3 text-2xl font-bold tracking-widest text-lab-muted">
            {offlineWatermark}
          </div>
        </div>
      ) : null}
      <div className="pointer-events-none absolute left-3 top-3 flex items-center gap-2 rounded border border-lab-line bg-lab-panel/90 px-2 py-1 text-xs text-lab-muted">
        <span>{labels.frame} {frame?.frame_index ?? 0}</span>
        <span className="h-3 w-px bg-lab-line" />
        <span>{frame?.snapshot.colliders.length ?? 0} {labels.colliders}</span>
        <span>{frame?.snapshot.contacts.length ?? 0} {labels.contacts}</span>
        <span className="h-3 w-px bg-lab-line" />
        <span>{labels.mode} {modeLabel}</span>
        <span className="h-3 w-px bg-lab-line" />
        <span>{labels.zoom} {(camera.scale / 48).toFixed(2)}x</span>
      </div>
      <div className="absolute right-3 top-3 flex items-center gap-1 rounded border border-lab-line bg-lab-panel/90 p-1 shadow-lg">
        <Tooltip label={lockTooltip}>
          <Button
            size="icon"
            variant={cameraMode === "locked_core" ? "default" : "ghost"}
            aria-label={lockTooltip}
            aria-pressed={cameraMode === "locked_core"}
            onClick={toggleCoreLock}
          >
            <Focus className="h-4 w-4" />
          </Button>
        </Tooltip>
        <Tooltip label={labels.zoomOut}>
          <Button size="icon" variant="ghost" aria-label={labels.zoomOut} onClick={() => zoomCenter(1 / 1.2)}>
            <ZoomOut className="h-4 w-4" />
          </Button>
        </Tooltip>
        <Tooltip label={labels.zoomIn}>
          <Button size="icon" variant="ghost" aria-label={labels.zoomIn} onClick={() => zoomCenter(1.2)}>
            <ZoomIn className="h-4 w-4" />
          </Button>
        </Tooltip>
        <Tooltip label={labels.fit}>
          <Button size="icon" variant="ghost" aria-label={labels.fit} onClick={fitCanvas}>
            <Maximize2 className="h-4 w-4" />
          </Button>
        </Tooltip>
        <Tooltip label={labels.reset}>
          <Button size="icon" variant="ghost" aria-label={labels.reset} onClick={resetCanvas}>
            <RotateCcw className="h-4 w-4" />
          </Button>
        </Tooltip>
      </div>
      {layers.rulers ? (
        <div className="pointer-events-none absolute bottom-3 left-3 rounded border border-lab-line bg-lab-panel/90 px-2 py-1 font-mono text-xs text-lab-muted">
          {labels.scale} 1u = {camera.scale.toFixed(0)}px
        </div>
      ) : null}
    </div>
  );
}

function drawWorld(
  ctx: CanvasRenderingContext2D,
  frame: FrameRecord,
  previousFrames: FrameRecord[],
  camera: Camera,
  layers: LayerState,
  selected: SelectedEntity | null,
  trajectoryOverlay: TrajectoryOverlay,
  trajectorySettings: TrajectorySettings,
  latticeSummary: LatticeProxySummary,
  labels: Pick<
    WorldCanvasProps["labels"],
    | "trajectoryEmptySelectedBody"
    | "trajectoryEmptyAllDynamic"
    | "trajectoryEmptySelectedIsland"
    | "trajectoryEmptyContacts"
    | "trajectoryEmptyCcd"
    | "trajectoryEmptyUnsupportedSelection"
  >,
  activeGrab: ActiveGrabRecord | null,
) {
  ctx.clearRect(0, 0, camera.width, camera.height);
  fillBackground(ctx, camera);
  if (layers.grid) {
    drawGrid(ctx, camera);
  }
  drawAxes(ctx, camera);

  if (layers.trace) {
    drawTrajectoryOverlay(ctx, trajectoryOverlay, camera, trajectorySettings, labels);
    drawPrimitives(ctx, frame.snapshot.primitives, camera);
  }

  if (layers.shapes) {
    for (const collider of frame.snapshot.colliders) {
      drawShape(ctx, collider, camera, isColliderSelected(collider, selected));
    }
  }

  if (layers.sleep) {
    drawSleepLayer(ctx, frame.snapshot.bodies, frame.snapshot.colliders, camera);
  }

  if (layers.aabbs) {
    for (const collider of frame.snapshot.colliders) {
      if (collider.aabb) {
        drawAabb(ctx, collider.aabb, camera, isColliderSelected(collider, selected));
      }
    }
  }

  if (layers.broadphaseTree) {
    drawBroadphaseTree(
      ctx,
      frame.snapshot.broadphase_tree,
      frame.snapshot.colliders,
      camera,
      selected,
    );
  }

  if (layers.islands) {
    drawIslands(ctx, frame.snapshot.islands ?? [], frame.snapshot.colliders, camera);
  }

  if (layers.provenance) {
    drawProvenance(ctx, frame, camera);
  }

  if (layers.stackStability) {
    drawStackStability(ctx, frame, previousFrames.slice(-24), camera);
  }

  if (layers.lattice) {
    drawLatticeProxy(ctx, latticeSummary, camera, selected);
  }

  if (activeGrab) {
    drawGrabOverlay(ctx, activeGrab, frame, camera);
  }

  if (selected?.kind === "body") {
    const body = frame.snapshot.bodies.find((entry) => entry.handle === selected.id);
    if (body) {
      const ownedColliders = frame.snapshot.colliders.filter((collider) => collider.body === body.handle);
      drawBodySelection(ctx, body, ownedColliders, camera);
    }
  }

  if (layers.velocities) {
    for (const body of frame.snapshot.bodies) {
      const start = body.transform.translation;
      drawArrow(ctx, camera, start, body.linear_velocity, "#7fb069", 0.35, body.body_type === "static" ? 0.15 : 1);
    }
  }

  if (layers.contacts) {
    for (const contact of frame.snapshot.contacts) {
      drawContact(ctx, contact, camera, selected?.kind === "contact" && selected.id === contact.id);
    }
  }

  if (layers.rulers) {
    drawRulers(ctx, camera);
  }
}

function isColliderSelected(collider: DebugCollider, selected: SelectedEntity | null): boolean {
  return (
    (selected?.kind === "collider" && selected.id === collider.handle) ||
    (selected?.kind === "body" && selected.id === collider.body)
  );
}

function fillBackground(ctx: CanvasRenderingContext2D, camera: Camera) {
  ctx.fillStyle = "#111418";
  ctx.fillRect(0, 0, camera.width, camera.height);
}

function drawGrid(ctx: CanvasRenderingContext2D, camera: Camera) {
  const worldTopLeft = screenToWorld({ x: 0, y: 0 }, camera);
  const worldBottomRight = screenToWorld({ x: camera.width, y: camera.height }, camera);
  const { minorStep, majorEvery } = pickGridSpacing(camera.scale);
  const startXIndex = Math.floor(worldTopLeft.x / minorStep);
  const endXIndex = Math.ceil(worldBottomRight.x / minorStep);
  const startYIndex = Math.floor(worldTopLeft.y / minorStep);
  const endYIndex = Math.ceil(worldBottomRight.y / minorStep);

  ctx.save();
  for (let index = startXIndex; index <= endXIndex; index += 1) {
    const screen = worldToScreen({ x: index * minorStep, y: 0 }, camera).x;
    drawGridLine(ctx, screen, 0, camera.height, index % majorEvery === 0);
  }

  for (let index = startYIndex; index <= endYIndex; index += 1) {
    const screen = worldToScreen({ x: 0, y: index * minorStep }, camera).y;
    drawGridLine(ctx, screen, 1, camera.width, index % majorEvery === 0);
  }
  ctx.restore();
}

function drawAxes(ctx: CanvasRenderingContext2D, camera: Camera) {
  const origin = worldToScreen({ x: 0, y: 0 }, camera);
  ctx.strokeStyle = "#556170";
  ctx.lineWidth = 1.5;
  ctx.beginPath();
  ctx.moveTo(0, origin.y);
  ctx.lineTo(camera.width, origin.y);
  ctx.moveTo(origin.x, 0);
  ctx.lineTo(origin.x, camera.height);
  ctx.stroke();
  ctx.fillStyle = "#8f9aaa";
  ctx.font = "11px ui-monospace, SFMono-Regular, monospace";
  ctx.fillText("x", camera.width - 18, origin.y - 8);
  ctx.fillText("y", origin.x + 8, camera.height - 14);
}

function drawRulers(ctx: CanvasRenderingContext2D, camera: Camera) {
  const band = 28;
  const worldTopLeft = screenToWorld({ x: 0, y: 0 }, camera);
  const worldBottomRight = screenToWorld({ x: camera.width, y: camera.height }, camera);
  const step = niceStep(72 / camera.scale);

  ctx.save();
  ctx.fillStyle = "rgba(17, 20, 24, 0.82)";
  ctx.fillRect(0, 0, camera.width, band);
  ctx.fillRect(0, 0, band, camera.height);
  ctx.strokeStyle = "#303844";
  ctx.lineWidth = 1;
  ctx.beginPath();
  ctx.moveTo(0, band);
  ctx.lineTo(camera.width, band);
  ctx.moveTo(band, 0);
  ctx.lineTo(band, camera.height);
  ctx.stroke();
  ctx.font = "10px ui-monospace, SFMono-Regular, monospace";
  ctx.fillStyle = "#8f9aaa";

  for (let x = Math.ceil(worldTopLeft.x / step) * step; x <= worldBottomRight.x; x += step) {
    const screen = worldToScreen({ x, y: 0 }, camera).x;
    ctx.strokeStyle = "#556170";
    ctx.beginPath();
    ctx.moveTo(screen, band - 8);
    ctx.lineTo(screen, band);
    ctx.stroke();
    ctx.fillText(formatTick(x), screen + 3, 11);
  }

  for (let y = Math.ceil(worldTopLeft.y / step) * step; y <= worldBottomRight.y; y += step) {
    const screen = worldToScreen({ x: 0, y }, camera).y;
    ctx.strokeStyle = "#556170";
    ctx.beginPath();
    ctx.moveTo(band - 8, screen);
    ctx.lineTo(band, screen);
    ctx.stroke();
    ctx.fillText(formatTick(y), 3, screen - 3);
  }
  ctx.restore();
}

// Grid step uses world units, but the user reads it in pixels: keep minor spacing
// near a readable on-screen density and derive major lines from that cadence.
function pickGridSpacing(scale: number): GridSpacing {
  const minorStep = niceStep(34 / Math.max(scale, 1));
  const minorPixels = minorStep * scale;
  const majorEvery = minorPixels >= 44 ? 2 : minorPixels >= 26 ? 4 : 5;
  return {
    minorStep,
    majorEvery,
  };
}

function drawGridLine(
  ctx: CanvasRenderingContext2D,
  screen: number,
  start: number,
  end: number,
  isMajor: boolean,
) {
  ctx.strokeStyle = isMajor ? "rgba(74, 86, 101, 0.62)" : "rgba(43, 51, 61, 0.8)";
  ctx.lineWidth = isMajor ? 1.15 : 1;
  ctx.beginPath();
  if (start === 0) {
    const x = alignToDevicePixel(screen);
    ctx.moveTo(x, start);
    ctx.lineTo(x, end);
  } else {
    const y = alignToDevicePixel(screen);
    ctx.moveTo(start, y);
    ctx.lineTo(end, y);
  }
  ctx.stroke();
}

function niceStep(raw: number): number {
  const magnitude = 10 ** Math.floor(Math.log10(Math.max(raw, 0.0001)));
  const normalized = raw / magnitude;
  if (normalized > 5) {
    return 10 * magnitude;
  }
  if (normalized > 2) {
    return 5 * magnitude;
  }
  if (normalized > 1) {
    return 2 * magnitude;
  }
  return magnitude;
}

function formatTick(value: number): string {
  if (Math.abs(value) >= 100 || Number.isInteger(value)) {
    return value.toFixed(0);
  }
  if (Math.abs(value) >= 10) {
    return value.toFixed(1);
  }
  return value.toFixed(2).replace(/0+$/, "").replace(/\.$/, "");
}

function drawBroadphaseTree(
  ctx: CanvasRenderingContext2D,
  tree: DebugBroadphaseTree | undefined,
  colliders: DebugCollider[],
  camera: Camera,
  selected: SelectedEntity | null,
) {
  const nodes = tree?.nodes ?? [];
  if (nodes.length === 0) {
    return;
  }
  const sortedNodes = [...nodes].sort((left, right) => left.depth - right.depth);
  for (const node of sortedNodes) {
    const isLeaf = node.collider != null;
    const isSelected =
      isLeaf &&
      ((selected?.kind === "collider" && selected.id === node.collider) ||
        (selected?.kind === "body" &&
          colliders.some(
            (collider) => collider.handle === node.collider && collider.body === selected.id,
          )));
    ctx.save();
    ctx.strokeStyle = isSelected
      ? "#f0c36b"
      : isLeaf
        ? "rgba(127, 176, 105, 0.75)"
        : "rgba(240, 195, 107, 0.55)";
    ctx.lineWidth = isSelected ? 2.2 : isLeaf ? 1.2 : 1;
    ctx.setLineDash(isLeaf ? [] : [6, 5]);
    drawAabbOutline(ctx, node.aabb, camera);
    ctx.restore();

    const label = isLeaf ? `L${node.depth}` : `N${node.depth}`;
    const labelPoint = worldToScreen(node.aabb.min, camera);
    ctx.fillStyle = isLeaf ? "#7fb069" : "#f0c36b";
    ctx.font = "10px ui-monospace, SFMono-Regular, monospace";
    ctx.fillText(label, labelPoint.x + 4, labelPoint.y - 4);
  }
}

function drawIslands(
  ctx: CanvasRenderingContext2D,
  islands: DebugIsland[],
  colliders: DebugCollider[],
  camera: Camera,
) {
  for (const island of islands) {
    const bounds = mergeBounds(
      colliders
        .filter((collider) => island.bodies.includes(collider.body))
        .map((collider) => collider.aabb)
        .filter((aabb): aabb is DebugAabb => Boolean(aabb)),
    );
    if (!bounds) {
      continue;
    }

    ctx.save();
    ctx.strokeStyle = island.sleeping
      ? "rgba(143, 154, 170, 0.8)"
      : "rgba(86, 182, 194, 0.85)";
    ctx.lineWidth = 2;
    ctx.setLineDash([10, 6]);
    drawAabbOutline(ctx, inflateAabb(bounds, 0.08), camera);
    ctx.restore();

    const labelPoint = worldToScreen(bounds.max, camera);
    ctx.fillStyle = island.sleeping ? "#8f9aaa" : "#56b6c2";
    ctx.font = "10px ui-monospace, SFMono-Regular, monospace";
    ctx.fillText(`I${island.id}`, labelPoint.x + 4, labelPoint.y - 4);
  }
}

function drawSleepLayer(
  ctx: CanvasRenderingContext2D,
  bodies: DebugBody[],
  colliders: DebugCollider[],
  camera: Camera,
) {
  const sleepingBodies = new Set(
    bodies
      .filter((body) => body.body_type === "dynamic" && body.sleeping)
      .map((body) => body.handle),
  );
  if (sleepingBodies.size === 0) {
    return;
  }

  ctx.save();
  for (const collider of colliders) {
    if (!sleepingBodies.has(collider.body)) {
      continue;
    }
    ctx.fillStyle = "rgba(143, 154, 170, 0.26)";
    ctx.strokeStyle = "rgba(203, 213, 225, 0.92)";
    ctx.lineWidth = 2.2;
    ctx.setLineDash([6, 4]);
    shapePath(ctx, collider.shape, camera);
    ctx.fill();
    ctx.stroke();
  }
  ctx.restore();

  ctx.save();
  ctx.font = "10px ui-monospace, SFMono-Regular, monospace";
  for (const body of bodies) {
    if (!sleepingBodies.has(body.handle)) {
      continue;
    }
    const center = worldToScreen(body.transform.translation, camera);
    ctx.fillStyle = "rgba(17, 20, 24, 0.9)";
    ctx.fillRect(center.x + 8, center.y - 18, 24, 14);
    ctx.strokeStyle = "#cbd5e1";
    ctx.lineWidth = 1;
    ctx.strokeRect(center.x + 8, center.y - 18, 24, 14);
    ctx.fillStyle = "#cbd5e1";
    ctx.fillText("Zz", center.x + 11, center.y - 8);
  }
  ctx.restore();
}

function drawProvenance(ctx: CanvasRenderingContext2D, frame: FrameRecord, camera: Camera) {
  for (const entry of frame.compound_provenance ?? []) {
    for (const piece of entry.pieces) {
      const collider = frame.snapshot.colliders.find(
        (candidate) => candidate.handle === piece.collider_handle,
      );
      if (!collider) {
        continue;
      }
      const screen = worldToScreen(collider.world_transform.translation, camera);
      ctx.fillStyle = "rgba(17, 20, 24, 0.82)";
      ctx.fillRect(screen.x - 12, screen.y - 20, 24, 14);
      ctx.strokeStyle = "#f0c36b";
      ctx.lineWidth = 1;
      ctx.strokeRect(screen.x - 12, screen.y - 20, 24, 14);
      ctx.fillStyle = "#f0c36b";
      ctx.font = "10px ui-monospace, SFMono-Regular, monospace";
      ctx.fillText(`P${piece.generated_piece_index}`, screen.x - 9, screen.y - 10);
    }
  }
}

function drawStackStability(
  ctx: CanvasRenderingContext2D,
  frame: FrameRecord,
  previousFrames: FrameRecord[],
  camera: Camera,
) {
  const dynamicBodies = frame.snapshot.bodies.filter((body) => body.body_type === "dynamic")
  const palette = ["#7fb069", "#56b6c2", "#f0c36b", "#d3869b", "#8f9aaa"]

  for (const body of dynamicBodies) {
    const bodyColliders = frame.snapshot.colliders.filter((collider) => collider.body === body.handle)
    drawBodyStabilityEnvelope(ctx, body, bodyColliders, camera, palette[body.handle % palette.length])

    const trail = previousFrames
      .map(
        (pastFrame) =>
          pastFrame.snapshot.bodies.find((entry) => entry.handle === body.handle)?.transform
            .translation ?? null,
      )
      .filter((point): point is Vec2 => Boolean(point))
    if (trail.length > 1) {
      ctx.save()
      ctx.strokeStyle = `${palette[body.handle % palette.length]}aa`
      ctx.lineWidth = 1.4
      ctx.beginPath()
      trail.forEach((point, index) => {
        const screen = worldToScreen(point, camera)
        if (index === 0) {
          ctx.moveTo(screen.x, screen.y)
        } else {
          ctx.lineTo(screen.x, screen.y)
        }
      })
      ctx.stroke()
      ctx.restore()
    }

    const center = worldToScreen(body.transform.translation, camera)
    const badge = body.sleeping ? "Zz" : `I${body.island_id ?? "?"}`
    ctx.fillStyle = "rgba(17, 20, 24, 0.86)"
    ctx.fillRect(center.x + 8, center.y - 18, 24, 14)
    ctx.strokeStyle = body.sleeping ? "#8f9aaa" : "#56b6c2"
    ctx.lineWidth = 1
    ctx.strokeRect(center.x + 8, center.y - 18, 24, 14)
    ctx.fillStyle = body.sleeping ? "#8f9aaa" : "#56b6c2"
    ctx.font = "10px ui-monospace, SFMono-Regular, monospace"
    ctx.fillText(badge, center.x + 11, center.y - 8)
  }

  for (const contact of frame.snapshot.contacts) {
    const point = worldToScreen(contact.point, camera)
    const impulseEvidence = readContactImpulseEvidence(contact)
    ctx.save()
    if (impulseEvidence.totalImpulse == null) {
      ctx.strokeStyle = "rgba(143, 154, 170, 0.75)"
      ctx.setLineDash([3, 2])
      ctx.lineWidth = 1
      ctx.beginPath()
      ctx.arc(point.x, point.y, Math.max(3.5, 0.055 * camera.scale), 0, Math.PI * 2)
      ctx.stroke()
      ctx.restore()
      continue
    }
    const worldRadius = 0.055 + Math.min(0.22, Math.sqrt(impulseEvidence.totalImpulse) * 0.08)
    const radius = Math.max(4, Math.min(54, worldRadius * camera.scale))
    const alpha = Math.max(0.18, Math.min(0.72, impulseEvidence.totalImpulse / 2.4))
    ctx.fillStyle = `rgba(240, 195, 107, ${alpha})`
    ctx.beginPath()
    ctx.arc(point.x, point.y, radius, 0, Math.PI * 2)
    ctx.fill()
    ctx.strokeStyle =
      impulseEvidence.missingCount > 0 ? "rgba(240, 195, 107, 0.95)" : "#f0c36b"
    ctx.lineWidth = 1
    if (impulseEvidence.missingCount > 0) {
      ctx.setLineDash([4, 2])
    }
    ctx.stroke()
    ctx.restore()
  }
}

function drawBodyStabilityEnvelope(
  ctx: CanvasRenderingContext2D,
  body: DebugBody,
  colliders: DebugCollider[],
  camera: Camera,
  color: string,
) {
  const bounds = aggregateBounds(colliders)
  if (!bounds) {
    return
  }
  const speed = Math.hypot(body.linear_velocity.x, body.linear_velocity.y)
  const angularSpeed = Math.abs(body.angular_velocity)
  const motionScore = Math.min(1, speed * 1.8 + angularSpeed * 0.8)
  const padding = 0.045 + motionScore * 0.08
  const rect = screenRectFromAabb(inflateAabb(bounds, padding), camera)

  ctx.save()
  ctx.strokeStyle = body.sleeping ? "rgba(143, 154, 170, 0.76)" : withAlpha(color, 0.76)
  ctx.fillStyle = body.sleeping ? "rgba(143, 154, 170, 0.08)" : withAlpha(color, 0.08 + motionScore * 0.13)
  ctx.lineWidth = Math.max(1.1, Math.min(3.6, padding * camera.scale * 0.18))
  ctx.setLineDash(body.sleeping ? [0.12 * camera.scale, 0.08 * camera.scale] : [])
  roundRect(ctx, rect.left, rect.top, rect.width, rect.height, Math.min(10, 0.05 * camera.scale))
  ctx.fill()
  ctx.stroke()
  ctx.restore()
}

function readContactImpulseEvidence(contact: DebugContact): {
  totalImpulse: number | null
  missingCount: number
} {
  let totalImpulse = 0
  let seen = 0
  let missingCount = 0
  for (const value of [
    contact.solver_normal_impulse,
    contact.solver_tangent_impulse,
  ]) {
    if (typeof value === "number") {
      totalImpulse += Math.abs(value)
      seen += 1
    } else {
      missingCount += 1
    }
  }
  return {
    totalImpulse: seen > 0 ? totalImpulse : null,
    missingCount,
  }
}

function drawShape(ctx: CanvasRenderingContext2D, collider: DebugCollider, camera: Camera, isSelected: boolean) {
  const isStatic = collider.density === 0;
  const fill = isStatic ? "rgba(143, 154, 170, 0.18)" : "rgba(86, 182, 194, 0.20)";
  const stroke = isSelected ? "#f0c36b" : isStatic ? "#8f9aaa" : "#56b6c2";
  ctx.fillStyle = fill;
  ctx.strokeStyle = stroke;
  ctx.lineWidth = isSelected ? 3 : 1.8;

  shapePath(ctx, collider.shape, camera);
  ctx.fill();
  ctx.stroke();
}

function shapePath(ctx: CanvasRenderingContext2D, shape: DebugShape, camera: Camera) {
  ctx.beginPath();
  if (shape.kind === "circle") {
    const center = worldToScreen(shape.center, camera);
    ctx.arc(center.x, center.y, shape.radius * camera.scale, 0, Math.PI * 2);
    return;
  }
  if (shape.kind === "segment") {
    const start = worldToScreen(shape.start, camera);
    const end = worldToScreen(shape.end, camera);
    ctx.moveTo(start.x, start.y);
    ctx.lineTo(end.x, end.y);
    return;
  }
  shape.vertices.forEach((point, index) => {
    const screen = worldToScreen(point, camera);
    if (index === 0) {
      ctx.moveTo(screen.x, screen.y);
    } else {
      ctx.lineTo(screen.x, screen.y);
    }
  });
  ctx.closePath();
}

function drawAabb(ctx: CanvasRenderingContext2D, aabb: DebugAabb, camera: Camera, isSelected: boolean) {
  ctx.strokeStyle = isSelected ? "#f0c36b" : "rgba(216, 173, 91, 0.65)";
  ctx.lineWidth = isSelected ? 2 : 1;
  ctx.setLineDash([5, 4]);
  drawAabbOutline(ctx, aabb, camera);
  ctx.setLineDash([]);
}

function drawAabbOutline(ctx: CanvasRenderingContext2D, aabb: DebugAabb, camera: Camera) {
  const rect = screenRectFromAabb(aabb, camera);
  ctx.strokeRect(rect.left, rect.top, rect.width, rect.height);
}

function drawBodySelection(ctx: CanvasRenderingContext2D, body: DebugBody, colliders: DebugCollider[], camera: Camera) {
  const center = worldToScreen(body.transform.translation, camera);
  ctx.save();
  ctx.strokeStyle = "#f0c36b";
  ctx.fillStyle = "#f0c36b";
  ctx.lineWidth = 2;
  ctx.shadowColor = "rgba(240, 195, 107, 0.45)";
  ctx.shadowBlur = 12;

  const bounds = aggregateBounds(colliders);
  if (bounds) {
    const pad = 7;
    const rect = screenRectFromAabb(bounds, camera, pad);
    ctx.setLineDash([7, 4]);
    ctx.strokeRect(rect.left, rect.top, rect.width, rect.height);
    ctx.setLineDash([]);
  }

  // Body 是质点/位姿容器，实际几何来自它拥有的 collider；这里同时标出位姿中心。
  ctx.beginPath();
  ctx.arc(center.x, center.y, 5, 0, Math.PI * 2);
  ctx.fill();
  ctx.beginPath();
  ctx.arc(center.x, center.y, 11, 0, Math.PI * 2);
  ctx.stroke();
  ctx.restore();
}

function drawLatticeProxy(
  ctx: CanvasRenderingContext2D,
  summary: LatticeProxySummary,
  camera: Camera,
  selected: SelectedEntity | null,
) {
  if (!summary.enabled || summary.edges.length === 0) {
    return
  }

  const { edges } = summary
  ctx.save()
  ctx.lineCap = "round"
  ctx.lineJoin = "round"
  for (const edge of edges) {
    const start = worldToScreen(edge.joint.anchors[0], camera)
    const end = worldToScreen(edge.joint.anchors[1], camera)
    const isSelected = selected?.kind === "joint" && selected.id === edge.joint.handle
    const stretchAlpha =
      edge.stretchRatio == null ? 0.58 : clamp(Math.abs(edge.stretchRatio - 1) * 2.8 + 0.48, 0.48, 0.96)
    ctx.strokeStyle = isSelected
      ? "#f8e3a2"
      : edge.isWorldAnchor
        ? `rgba(211, 134, 155, ${stretchAlpha})`
        : `rgba(240, 195, 107, ${stretchAlpha})`
    ctx.lineWidth = isSelected ? 3.1 : edge.isWorldAnchor ? 2.1 : 1.45
    ctx.setLineDash(edge.isWorldAnchor ? [5, 3] : [])
    ctx.beginPath()
    ctx.moveTo(start.x, start.y)
    ctx.lineTo(end.x, end.y)
    ctx.stroke()

    for (const anchor of edge.joint.anchors) {
      const point = worldToScreen(anchor, camera)
      ctx.fillStyle = edge.isWorldAnchor ? "#d3869b" : "#f0c36b"
      if (edge.isWorldAnchor) {
        ctx.fillRect(point.x - 3, point.y - 3, 6, 6)
      } else {
        ctx.beginPath()
        ctx.arc(point.x, point.y, 2.8, 0, Math.PI * 2)
        ctx.fill()
      }
    }

    if (isSelected && edge.stretchRatio != null) {
      drawLatticeStretchBadge(ctx, camera, edge.joint.anchors, edge.stretchRatio)
    }
  }

  const nodes = summary.nodes
  for (const node of nodes) {
    const center = worldToScreen(node.body.transform.translation, camera)
    const isSelected = selected?.kind === "body" && selected.id === node.body.handle
    ctx.fillStyle = isSelected ? "#f8e3a2" : node.body.sleeping ? "#8f9aaa" : "#56b6c2"
    ctx.strokeStyle = "rgba(15, 17, 20, 0.75)"
    ctx.lineWidth = isSelected ? 2 : 1
    ctx.beginPath()
    ctx.arc(center.x, center.y, isSelected ? 5.3 : 4, 0, Math.PI * 2)
    ctx.fill()
    ctx.stroke()
    if (summary.enabled && (isSelected || node.anchorEdgeCount > 0)) {
      ctx.fillStyle = "rgba(15, 17, 20, 0.76)"
      ctx.fillRect(center.x + 6, center.y - 16, 30, 13)
      ctx.fillStyle = node.anchorEdgeCount > 0 ? "#d3869b" : "#56b6c2"
      ctx.font = "10px ui-monospace, SFMono-Regular, monospace"
      ctx.fillText(`I${node.islandId ?? "?"}`, center.x + 9, center.y - 6)
    }
  }
  ctx.restore()
}

function drawLatticeStretchBadge(
  ctx: CanvasRenderingContext2D,
  camera: Camera,
  anchors: Vec2[],
  stretchRatio: number,
) {
  const bounds = boundsFromPoints(anchors)
  if (!bounds) {
    return
  }
  const center = worldToScreen(aabbCenter(bounds), camera)
  const label = `x${stretchRatio.toFixed(2)}`
  ctx.save()
  ctx.font = "10px ui-monospace, SFMono-Regular, monospace"
  const width = Math.max(34, ctx.measureText(label).width + 10)
  ctx.fillStyle = "rgba(15, 17, 20, 0.84)"
  ctx.strokeStyle = "rgba(248, 227, 162, 0.75)"
  ctx.lineWidth = 1
  roundRect(ctx, center.x - width / 2, center.y - 24, width, 16, 4)
  ctx.fill()
  ctx.stroke()
  ctx.fillStyle = "#f8e3a2"
  ctx.fillText(label, center.x - width / 2 + 5, center.y - 12)
  ctx.restore()
}

function drawGrabOverlay(
  ctx: CanvasRenderingContext2D,
  grab: ActiveGrabRecord,
  frame: FrameRecord,
  camera: Camera,
) {
  const grabJoint =
    grab.joint_handle != null
      ? frame.snapshot.joints.find((joint) => joint.handle === grab.joint_handle)
      : undefined
  const body = frame.snapshot.bodies.find(
    (entry) => entry.handle === grab.body_handle,
  )
  const anchorWorld = grabJoint
    ? grabJoint.anchors[0]
    : body
      ? {
          x:
            body.transform.translation.x +
            grab.local_anchor.x * Math.cos(body.transform.rotation) -
            grab.local_anchor.y * Math.sin(body.transform.rotation),
          y:
            body.transform.translation.y +
            grab.local_anchor.x * Math.sin(body.transform.rotation) +
            grab.local_anchor.y * Math.cos(body.transform.rotation),
        }
      : null
  const targetWorld = grabJoint ? grabJoint.anchors[1] : grab.target
  if (!anchorWorld) {
    return
  }
  const start = worldToScreen(anchorWorld, camera)
  const end = worldToScreen(targetWorld, camera)
  ctx.strokeStyle = grab.mode === "spring" ? "#8ec07c" : "#83a598"
  ctx.lineWidth = 2.2
  ctx.setLineDash(grab.mode === "spring" ? [] : [6, 4])
  ctx.beginPath()
  ctx.moveTo(start.x, start.y)
  ctx.lineTo(end.x, end.y)
  ctx.stroke()
  ctx.setLineDash([])
  ctx.fillStyle = "#8ec07c"
  ctx.beginPath()
  ctx.arc(end.x, end.y, 4, 0, Math.PI * 2)
  ctx.fill()
}

function drawContact(ctx: CanvasRenderingContext2D, contact: DebugContact, camera: Camera, isSelected: boolean) {
  const point = worldToScreen(contact.point, camera);
  ctx.fillStyle = isSelected ? "#f0c36b" : "#d06464";
  ctx.beginPath();
  ctx.arc(point.x, point.y, isSelected ? 5 : 3.5, 0, Math.PI * 2);
  ctx.fill();
  drawArrow(ctx, camera, contact.point, contact.normal, isSelected ? "#f0c36b" : "#d06464", 0.8, 1);
}

function drawTrajectoryOverlay(
  ctx: CanvasRenderingContext2D,
  overlay: TrajectoryOverlay,
  camera: Camera,
  settings: TrajectorySettings,
  labels: Pick<
    WorldCanvasProps["labels"],
    | "trajectoryEmptySelectedBody"
    | "trajectoryEmptyAllDynamic"
    | "trajectoryEmptySelectedIsland"
    | "trajectoryEmptyContacts"
    | "trajectoryEmptyCcd"
    | "trajectoryEmptyUnsupportedSelection"
  >,
) {
  for (const trail of overlay.bodyTrails) {
    drawPolylineTrail(ctx, trail.points, camera, trail.color, settings.fade, trail.emphasis, 1.65);
  }
  for (const trail of overlay.contactTrails) {
    drawPolylineTrail(ctx, trail.points, camera, trail.color, settings.fade, false, 1.25, [4, 3]);
  }
  for (const trail of overlay.ccdTrails) {
    drawCcdTrajectory(ctx, trail, camera, settings.fade);
  }

  if (
    overlay.bodyTrails.length === 0 &&
    overlay.contactTrails.length === 0 &&
    overlay.ccdTrails.length === 0 &&
    overlay.emptyState
  ) {
    ctx.save();
    ctx.fillStyle = "rgba(15, 17, 20, 0.8)";
    ctx.strokeStyle = "rgba(130, 139, 150, 0.4)";
    ctx.lineWidth = 1;
    const label = emptyTrajectoryLabel(labels, overlay.emptyState);
    ctx.font = "12px Inter, ui-sans-serif, system-ui";
    // Keep the hint below the top-left HUD strip so it never sits under the
    // DOM canvas controls anchored to the top-right corner.
    const width = Math.min(camera.width - 24, ctx.measureText(label).width + 24);
    const height = 34;
    const left = 12;
    const top = 44;
    roundRect(ctx, left, top, width, height, 6);
    ctx.fill();
    ctx.stroke();
    ctx.fillStyle = "#cfd4db";
    ctx.fillText(label, left + 12, top + 21);
    ctx.restore();
  }
}

function drawPolylineTrail(
  ctx: CanvasRenderingContext2D,
  points: Vec2[],
  camera: Camera,
  color: string,
  fade: number,
  emphasis: boolean,
  width: number,
  lineDash: number[] = [],
) {
  if (points.length < 2) {
    return;
  }
  ctx.save();
  ctx.lineCap = "round";
  ctx.lineJoin = "round";
  ctx.setLineDash(lineDash);
  const minAlpha = Math.max(0.12, 1 - fade);
  for (let index = 1; index < points.length; index += 1) {
    const start = worldToScreen(points[index - 1], camera);
    const end = worldToScreen(points[index], camera);
    const progress = index / (points.length - 1);
    const alpha = minAlpha + (1 - minAlpha) * progress;
    ctx.strokeStyle = withAlpha(color, emphasis ? Math.min(1, alpha + 0.15) : alpha);
    ctx.lineWidth = emphasis ? width + 0.85 : width;
    ctx.beginPath();
    ctx.moveTo(start.x, start.y);
    ctx.lineTo(end.x, end.y);
    ctx.stroke();
  }
  const latest = worldToScreen(points[points.length - 1], camera);
  ctx.fillStyle = withAlpha(color, emphasis ? 0.96 : 0.88);
  ctx.beginPath();
  ctx.arc(latest.x, latest.y, emphasis ? 3.5 : 2.8, 0, Math.PI * 2);
  ctx.fill();
  ctx.restore();
}

function drawCcdTrajectory(
  ctx: CanvasRenderingContext2D,
  trail: CcdTrajectoryTrail,
  camera: Camera,
  fade: number,
) {
  ctx.save();
  drawPolylineTrail(
    ctx,
    [trail.sweptStart, trail.sweptEnd],
    camera,
    trail.color,
    fade,
    true,
    1.55,
    [6, 4],
  );
  if (trail.targetSweptStart && trail.targetSweptEnd) {
    drawPolylineTrail(
      ctx,
      [trail.targetSweptStart, trail.targetSweptEnd],
      camera,
      trail.targetKind === "dynamic" ? "#f0c36b" : "#8f9aaa",
      Math.max(0.45, fade - 0.2),
      false,
      1.1,
      [2, 4],
    );
  }
  const toi = worldToScreen(trail.toiPoint, camera);
  ctx.fillStyle = withAlpha(trail.color, 0.95);
  ctx.strokeStyle = "rgba(15, 17, 20, 0.7)";
  ctx.lineWidth = 1.2;
  ctx.beginPath();
  ctx.arc(toi.x, toi.y, 4, 0, Math.PI * 2);
  ctx.fill();
  ctx.stroke();
  ctx.restore();
}

function emptyTrajectoryLabel(
  labels: Pick<
    WorldCanvasProps["labels"],
    | "trajectoryEmptySelectedBody"
    | "trajectoryEmptyAllDynamic"
    | "trajectoryEmptySelectedIsland"
    | "trajectoryEmptyContacts"
    | "trajectoryEmptyCcd"
    | "trajectoryEmptyUnsupportedSelection"
  >,
  emptyState: TrajectoryOverlay["emptyState"],
): string {
  switch (emptyState) {
    case "selectedBody":
      return labels.trajectoryEmptySelectedBody;
    case "allDynamic":
      return labels.trajectoryEmptyAllDynamic;
    case "selectedIsland":
      return labels.trajectoryEmptySelectedIsland;
    case "contacts":
      return labels.trajectoryEmptyContacts;
    case "ccd":
      return labels.trajectoryEmptyCcd;
    case "unsupportedSelection":
      return labels.trajectoryEmptyUnsupportedSelection;
    case null:
      return "";
  }
}

function drawPrimitives(ctx: CanvasRenderingContext2D, primitives: DebugPrimitive[], camera: Camera) {
  for (const primitive of primitives) {
    ctx.save();
    if (primitive.kind === "line") {
      const start = worldToScreen(primitive.start, camera);
      const end = worldToScreen(primitive.end, camera);
      ctx.strokeStyle = colorToCss(primitive.color);
      ctx.lineWidth = 1.7;
      ctx.beginPath();
      ctx.moveTo(start.x, start.y);
      ctx.lineTo(end.x, end.y);
      ctx.stroke();
    } else if (primitive.kind === "polyline") {
      drawPrimitivePolyline(ctx, primitive.points, primitive.closed, colorToCss(primitive.color), camera);
    } else if (primitive.kind === "polygon") {
      drawPrimitivePolygon(ctx, primitive, camera);
    } else if (primitive.kind === "circle") {
      const center = worldToScreen(primitive.center, camera);
      ctx.strokeStyle = colorToCss(primitive.color);
      ctx.lineWidth = 1.7;
      ctx.beginPath();
      ctx.arc(center.x, center.y, Math.max(2, primitive.radius * camera.scale), 0, Math.PI * 2);
      ctx.stroke();
    } else if (primitive.kind === "arrow") {
      drawPrimitiveArrow(ctx, primitive.origin, primitive.direction, colorToCss(primitive.color), camera);
    } else if (primitive.kind === "label") {
      const position = worldToScreen(primitive.position, camera);
      ctx.fillStyle = colorToCss(primitive.color);
      ctx.font = "12px ui-monospace, SFMono-Regular, monospace";
      ctx.fillText(primitive.text, position.x + 6, position.y - 6);
    }
    ctx.restore();
  }
}

function drawPrimitivePolyline(
  ctx: CanvasRenderingContext2D,
  points: Vec2[],
  closed: boolean,
  color: string,
  camera: Camera,
) {
  if (points.length < 2) {
    return;
  }
  ctx.strokeStyle = color;
  ctx.lineWidth = 1.7;
  ctx.beginPath();
  points.forEach((point, index) => {
    const screen = worldToScreen(point, camera);
    if (index === 0) {
      ctx.moveTo(screen.x, screen.y);
    } else {
      ctx.lineTo(screen.x, screen.y);
    }
  });
  if (closed) {
    ctx.closePath();
  }
  ctx.stroke();
}

function drawPrimitivePolygon(
  ctx: CanvasRenderingContext2D,
  primitive: Extract<DebugPrimitive, { kind: "polygon" }>,
  camera: Camera,
) {
  if (primitive.points.length < 3) {
    return;
  }
  ctx.beginPath();
  primitive.points.forEach((point, index) => {
    const screen = worldToScreen(point, camera);
    if (index === 0) {
      ctx.moveTo(screen.x, screen.y);
    } else {
      ctx.lineTo(screen.x, screen.y);
    }
  });
  ctx.closePath();
  if (primitive.fill) {
    ctx.fillStyle = colorToCss(primitive.fill);
    ctx.fill();
  }
  ctx.strokeStyle = colorToCss(primitive.stroke);
  ctx.lineWidth = 1.7;
  ctx.stroke();
}

function drawPrimitiveArrow(
  ctx: CanvasRenderingContext2D,
  origin: Vec2,
  direction: Vec2,
  color: string,
  camera: Camera,
) {
  const start = worldToScreen(origin, camera);
  const end = worldToScreen({ x: origin.x + direction.x, y: origin.y + direction.y }, camera);
  const length = Math.hypot(end.x - start.x, end.y - start.y);
  if (!Number.isFinite(length) || length <= 0.5) {
    return;
  }
  const angle = Math.atan2(end.y - start.y, end.x - start.x);
  ctx.strokeStyle = color;
  ctx.fillStyle = color;
  ctx.lineWidth = 1.7;
  ctx.beginPath();
  ctx.moveTo(start.x, start.y);
  ctx.lineTo(end.x, end.y);
  ctx.stroke();
  ctx.beginPath();
  ctx.moveTo(end.x, end.y);
  ctx.lineTo(end.x - 8 * Math.cos(angle - Math.PI / 6), end.y - 8 * Math.sin(angle - Math.PI / 6));
  ctx.lineTo(end.x - 8 * Math.cos(angle + Math.PI / 6), end.y - 8 * Math.sin(angle + Math.PI / 6));
  ctx.closePath();
  ctx.fill();
}

function colorToCss(color: { r: number; g: number; b: number; a: number }): string {
  return `rgba(${color.r}, ${color.g}, ${color.b}, ${Math.max(0, Math.min(1, color.a / 255))})`;
}

function withAlpha(color: string, alpha: number): string {
  if (!color.startsWith("#")) {
    return color;
  }
  const hex = color.slice(1);
  const normalized =
    hex.length === 3
      ? hex
          .split("")
          .map((part) => `${part}${part}`)
          .join("")
      : hex;
  const value = Number.parseInt(normalized, 16);
  const red = (value >> 16) & 255;
  const green = (value >> 8) & 255;
  const blue = value & 255;
  return `rgba(${red}, ${green}, ${blue}, ${clamp(alpha, 0, 1)})`;
}

function roundRect(
  ctx: CanvasRenderingContext2D,
  x: number,
  y: number,
  width: number,
  height: number,
  radius: number,
) {
  const r = Math.min(radius, width / 2, height / 2);
  ctx.beginPath();
  ctx.moveTo(x + r, y);
  ctx.lineTo(x + width - r, y);
  ctx.arcTo(x + width, y, x + width, y + r, r);
  ctx.lineTo(x + width, y + height - r);
  ctx.arcTo(x + width, y + height, x + width - r, y + height, r);
  ctx.lineTo(x + r, y + height);
  ctx.arcTo(x, y + height, x, y + height - r, r);
  ctx.lineTo(x, y + r);
  ctx.arcTo(x, y, x + r, y, r);
  ctx.closePath();
}

function drawArrow(
  ctx: CanvasRenderingContext2D,
  camera: Camera,
  origin: Vec2,
  direction: Vec2,
  color: string,
  scale = 1,
  alpha = 1,
) {
  const magnitude = Math.hypot(direction.x, direction.y);
  if (!Number.isFinite(magnitude) || magnitude <= 0.001) {
    return;
  }
  const start = worldToScreen(origin, camera);
  const unit = { x: direction.x / magnitude, y: direction.y / magnitude };
  const worldEnd = {
    x: origin.x + unit.x * Math.min(1.4, magnitude * scale),
    y: origin.y + unit.y * Math.min(1.4, magnitude * scale),
  };
  const end = worldToScreen(worldEnd, camera);
  const angle = Math.atan2(end.y - start.y, end.x - start.x);
  ctx.save();
  ctx.globalAlpha = alpha;
  ctx.strokeStyle = color;
  ctx.fillStyle = color;
  ctx.lineWidth = 1.7;
  ctx.beginPath();
  ctx.moveTo(start.x, start.y);
  ctx.lineTo(end.x, end.y);
  ctx.stroke();
  ctx.beginPath();
  ctx.moveTo(end.x, end.y);
  ctx.lineTo(end.x - 8 * Math.cos(angle - Math.PI / 6), end.y - 8 * Math.sin(angle - Math.PI / 6));
  ctx.lineTo(end.x - 8 * Math.cos(angle + Math.PI / 6), end.y - 8 * Math.sin(angle + Math.PI / 6));
  ctx.closePath();
  ctx.fill();
  ctx.restore();
}

function hitTest(
  colliders: DebugCollider[],
  contacts: DebugContact[],
  latticeNodes: LatticeProxyNode[],
  latticeEdges: LatticeProxyEdge[],
  point: Vec2,
): SelectedEntity | null {
  const contactHit = contacts.find((contact) => distance(contact.point, point) < 0.18);
  if (contactHit) {
    return { kind: "contact", id: contactHit.id };
  }

  const latticeNodeHit = latticeNodes.find(
    (node) => distance(node.body.transform.translation, point) < 0.16,
  )
  if (latticeNodeHit) {
    return { kind: "body", id: latticeNodeHit.body.handle }
  }

  const colliderHit = [...colliders].reverse().find((collider) => colliderContains(collider, point));
  if (colliderHit) {
    return { kind: "collider", id: colliderHit.handle };
  }

  const latticeEdgeHit = latticeEdges.find((edge) => {
    if (edge.joint.anchors.length < 2) {
      return false
    }
    return distanceToSegment(point, edge.joint.anchors[0], edge.joint.anchors[1]) < 0.1
  })
  if (latticeEdgeHit) {
    return { kind: "joint", id: latticeEdgeHit.joint.handle }
  }

  return null;
}

function colliderContains(collider: DebugCollider, point: Vec2): boolean {
  if (collider.aabb && !pointInAabb(point, collider.aabb)) {
    return false;
  }
  if (collider.shape.kind === "circle") {
    return distance(point, collider.shape.center) <= collider.shape.radius;
  }
  if (collider.shape.kind === "polygon") {
    return pointInPolygon(point, collider.shape.vertices);
  }
  return collider.aabb ? pointInAabb(point, collider.aabb) : false;
}

function pointInAabb(point: Vec2, aabb: DebugAabb): boolean {
  return point.x >= aabb.min.x && point.x <= aabb.max.x && point.y >= aabb.min.y && point.y <= aabb.max.y;
}

function pointInPolygon(point: Vec2, vertices: Vec2[]): boolean {
  let inside = false;
  for (let i = 0, j = vertices.length - 1; i < vertices.length; j = i++) {
    const a = vertices[i];
    const b = vertices[j];
    const intersects = a.y > point.y !== b.y > point.y && point.x < ((b.x - a.x) * (point.y - a.y)) / (b.y - a.y) + a.x;
    if (intersects) {
      inside = !inside;
    }
  }
  return inside;
}

function buildFreeCameraState(camera: Camera): CameraState {
  return {
    mode: "free",
    scale: camera.scale,
    origin: camera.origin,
    lockedBounds: null,
    lockedTargetDescription: null,
  };
}

function cameraStateToCamera(
  state: Pick<CameraState, "scale" | "origin">,
  size: { width: number; height: number },
): Camera {
  return {
    scale: state.scale,
    origin: state.origin,
    width: size.width,
    height: size.height,
  };
}

// "core area" 是当前最值得盯住的调试主体：优先用户选中的实体，其次是仍在活动的动态体/
// 接触区域，再退到全局碰撞体与场景边界。这里用 AABB（axis-aligned bounding box，轴对齐包围盒）
// 做聚焦边界，避免前端重新计算物理过程。
function resolveCoreFocusTarget(
  frame: FrameRecord | undefined,
  selected: SelectedEntity | null,
  labels: CameraFocusLabels,
): CameraFocusTarget | null {
  if (!frame) {
    return null;
  }

  const selectedTarget = resolveSelectedFocusTarget(frame, selected, labels);
  if (selectedTarget) {
    return selectedTarget;
  }

  const activeDynamicBodies = new Set(
    frame.snapshot.bodies
      .filter((body) => body.body_type === "dynamic" && !body.sleeping)
      .map((body) => body.handle),
  );
  const activeBounds = mergeBounds([
    ...frame.snapshot.colliders
      .filter((collider) => activeDynamicBodies.has(collider.body))
      .map((collider) => collider.aabb)
      .filter((aabb): aabb is DebugAabb => Boolean(aabb)),
    ...frame.snapshot.contacts.map((contact) => pointBounds(contact.point, 0.18)),
  ]);
  if (activeBounds) {
    return {
      description: labels.targetActive,
      bounds: activeBounds,
    };
  }

  const colliderBounds = aggregateBounds(frame.snapshot.colliders);
  if (colliderBounds) {
    return {
      description: labels.targetAllColliders,
      bounds: colliderBounds,
    };
  }

  return {
    description: labels.targetScene,
    bounds:
      aggregateSceneBounds(frame.snapshot.colliders, frame.snapshot.primitives) ??
      defaultSceneBounds(),
  };
}

function resolveSelectedFocusTarget(
  frame: FrameRecord,
  selected: SelectedEntity | null,
  labels: CameraFocusLabels,
): CameraFocusTarget | null {
  if (!selected) {
    return null;
  }

  if (selected.kind === "body") {
    const body = frame.snapshot.bodies.find((entry) => entry.handle === selected.id);
    if (!body) {
      return null;
    }
    const ownedColliders = frame.snapshot.colliders.filter((collider) => collider.body === body.handle);
    return {
      description: labels.entity(selected.kind, selected.id),
      bounds: aggregateBounds(ownedColliders) ?? pointBounds(body.transform.translation, 0.55),
    };
  }

  if (selected.kind === "collider") {
    const collider = frame.snapshot.colliders.find((entry) => entry.handle === selected.id);
    if (!collider) {
      return null;
    }
    return {
      description: labels.entity(selected.kind, selected.id),
      bounds: collider.aabb ?? shapeBounds(collider.shape) ?? pointBounds(collider.world_transform.translation, 0.45),
    };
  }

  if (selected.kind === "contact") {
    const contact = frame.snapshot.contacts.find((entry) => entry.id === selected.id);
    if (!contact) {
      return null;
    }
    return {
      description: labels.entity(selected.kind, selected.id),
      bounds: pointBounds(contact.point, 0.18),
    };
  }

  const joint = frame.snapshot.joints.find((entry) => entry.handle === selected.id);
  if (!joint) {
    return null;
  }
  return {
    description: labels.entity(selected.kind, selected.id),
    bounds: boundsFromPoints(joint.anchors) ?? pointBounds({ x: 0, y: 0 }, 0.45),
  };
}

function makeLockedCoreCamera(bounds: DebugAabb, size: { width: number; height: number }): Camera {
  const width = Math.max(bounds.max.x - bounds.min.x, 0.8);
  const height = Math.max(bounds.max.y - bounds.min.y, 0.8);
  const padding = Math.max(0.35, Math.max(width, height) * 0.18);
  return cameraFromBounds(inflateAabb(bounds, padding), size, 0.8);
}

function stabilizeLockedBounds(previous: DebugAabb | null, next: DebugAabb): DebugAabb {
  if (!previous) {
    return next;
  }
  const previousCenter = aabbCenter(previous);
  const nextCenter = aabbCenter(next);
  const previousSize = aabbSize(previous);
  const nextSize = aabbSize(next);
  const centerDelta = distance(previousCenter, nextCenter);
  const sizeDelta = Math.max(
    Math.abs(previousSize.x - nextSize.x),
    Math.abs(previousSize.y - nextSize.y),
  );
  if (centerDelta < 0.02 && sizeDelta < 0.02) {
    return previous;
  }
  const alpha = centerDelta > 0.45 || sizeDelta > 0.45 ? 0.34 : 0.2;
  return aabbFromCenterSize(
    {
      x: lerp(previousCenter.x, nextCenter.x, alpha),
      y: lerp(previousCenter.y, nextCenter.y, alpha),
    },
    {
      x: lerp(previousSize.x, nextSize.x, alpha),
      y: lerp(previousSize.y, nextSize.y, alpha),
    },
  );
}

function makeCamera(
  colliders: DebugCollider[],
  primitives: DebugPrimitive[],
  size: { width: number; height: number },
): Camera {
  const bounds = aggregateSceneBounds(colliders, primitives) ?? defaultSceneBounds();
  return cameraFromBounds(inflateAabb(bounds, 1.2), size);
}

function aggregateSceneBounds(colliders: DebugCollider[], primitives: DebugPrimitive[]): DebugAabb | null {
  const bounds = [
    ...colliders.map((collider) => collider.aabb),
    ...primitives.map(primitiveBounds),
  ].filter((aabb): aabb is DebugAabb => Boolean(aabb));
  return mergeBounds(bounds);
}

function aggregateBounds(colliders: DebugCollider[]): DebugAabb | null {
  const aabbs = colliders.map((collider) => collider.aabb).filter((aabb): aabb is DebugAabb => Boolean(aabb));
  return mergeBounds(aabbs);
}

function shapeBounds(shape: DebugShape): DebugAabb | null {
  if (shape.kind === "circle") {
    return {
      min: { x: shape.center.x - shape.radius, y: shape.center.y - shape.radius },
      max: { x: shape.center.x + shape.radius, y: shape.center.y + shape.radius },
    };
  }
  if (shape.kind === "segment") {
    return boundsFromPoints([shape.start, shape.end]);
  }
  return boundsFromPoints(shape.vertices);
}

function mergeBounds(aabbs: DebugAabb[]): DebugAabb | null {
  if (aabbs.length === 0) {
    return null;
  }
  return aabbs.reduce((acc, aabb) => ({
    min: { x: Math.min(acc.min.x, aabb.min.x), y: Math.min(acc.min.y, aabb.min.y) },
    max: { x: Math.max(acc.max.x, aabb.max.x), y: Math.max(acc.max.y, aabb.max.y) },
  }));
}

function inflateAabb(aabb: DebugAabb, amount: number): DebugAabb {
  return {
    min: { x: aabb.min.x - amount, y: aabb.min.y - amount },
    max: { x: aabb.max.x + amount, y: aabb.max.y + amount },
  };
}

function pointBounds(point: Vec2, radius: number): DebugAabb {
  return {
    min: { x: point.x - radius, y: point.y - radius },
    max: { x: point.x + radius, y: point.y + radius },
  };
}

function primitiveBounds(primitive: DebugPrimitive): DebugAabb | null {
  if (primitive.kind === "line") {
    return boundsFromPoints([primitive.start, primitive.end]);
  }
  if (primitive.kind === "polyline" || primitive.kind === "polygon") {
    return boundsFromPoints(primitive.points);
  }
  if (primitive.kind === "circle") {
    return {
      min: { x: primitive.center.x - primitive.radius, y: primitive.center.y - primitive.radius },
      max: { x: primitive.center.x + primitive.radius, y: primitive.center.y + primitive.radius },
    };
  }
  if (primitive.kind === "arrow") {
    return boundsFromPoints([
      primitive.origin,
      { x: primitive.origin.x + primitive.direction.x, y: primitive.origin.y + primitive.direction.y },
    ]);
  }
  return boundsFromPoints([primitive.position]);
}

function boundsFromPoints(points: Vec2[]): DebugAabb | null {
  if (points.length === 0) {
    return null;
  }
  return points.reduce(
    (acc, point) => ({
      min: { x: Math.min(acc.min.x, point.x), y: Math.min(acc.min.y, point.y) },
      max: { x: Math.max(acc.max.x, point.x), y: Math.max(acc.max.y, point.y) },
    }),
    { min: { ...points[0] }, max: { ...points[0] } },
  );
}

function cameraFromBounds(
  bounds: DebugAabb,
  size: { width: number; height: number },
  minSpan = 2,
): Camera {
  const width = Math.max(minSpan, bounds.max.x - bounds.min.x);
  const height = Math.max(minSpan, bounds.max.y - bounds.min.y);
  const scale = Math.min(size.width / width, size.height / height);
  return {
    scale,
    origin: {
      x: size.width / 2 - ((bounds.min.x + bounds.max.x) / 2) * scale,
      y: size.height / 2 - ((bounds.min.y + bounds.max.y) / 2) * scale,
    },
    width: size.width,
    height: size.height,
  };
}

function worldToScreen(point: Vec2, camera: Camera): Vec2 {
  return {
    x: camera.origin.x + point.x * camera.scale,
    y: camera.origin.y + point.y * camera.scale,
  };
}

function screenToWorld(point: Vec2, camera: Camera): Vec2 {
  return {
    x: (point.x - camera.origin.x) / camera.scale,
    y: (point.y - camera.origin.y) / camera.scale,
  };
}

function screenRectFromAabb(aabb: DebugAabb, camera: Camera, padding = 0) {
  const min = worldToScreen(aabb.min, camera);
  const max = worldToScreen(aabb.max, camera);
  return {
    left: Math.min(min.x, max.x) - padding,
    top: Math.min(min.y, max.y) - padding,
    width: Math.abs(max.x - min.x) + padding * 2,
    height: Math.abs(max.y - min.y) + padding * 2,
  };
}

function alignToDevicePixel(value: number): number {
  return Math.round(value) + 0.5;
}

function distance(a: Vec2, b: Vec2): number {
  return Math.hypot(a.x - b.x, a.y - b.y);
}

function distanceToSegment(point: Vec2, start: Vec2, end: Vec2): number {
  const dx = end.x - start.x
  const dy = end.y - start.y
  const lengthSquared = dx * dx + dy * dy
  if (lengthSquared <= 1e-8) {
    return distance(point, start)
  }
  const rawT = ((point.x - start.x) * dx + (point.y - start.y) * dy) / lengthSquared
  const t = clamp(rawT, 0, 1)
  return distance(point, { x: start.x + dx * t, y: start.y + dy * t })
}

function clamp(value: number, min: number, max: number): number {
  return Math.max(min, Math.min(max, value));
}

function lerp(start: number, end: number, alpha: number): number {
  return start + (end - start) * alpha;
}

function aabbCenter(aabb: DebugAabb): Vec2 {
  return {
    x: (aabb.min.x + aabb.max.x) / 2,
    y: (aabb.min.y + aabb.max.y) / 2,
  };
}

function aabbSize(aabb: DebugAabb): Vec2 {
  return {
    x: Math.max(aabb.max.x - aabb.min.x, 0.0001),
    y: Math.max(aabb.max.y - aabb.min.y, 0.0001),
  };
}

function aabbFromCenterSize(center: Vec2, size: Vec2): DebugAabb {
  return {
    min: { x: center.x - size.x / 2, y: center.y - size.y / 2 },
    max: { x: center.x + size.x / 2, y: center.y + size.y / 2 },
  };
}

function aabbNearEqual(left: DebugAabb | null, right: DebugAabb | null): boolean {
  if (!left || !right) {
    return left === right;
  }
  return (
    distance(left.min, right.min) < 0.0001 &&
    distance(left.max, right.max) < 0.0001
  );
}

function cameraNearEqual(
  left: Pick<Camera, "scale" | "origin">,
  right: Pick<Camera, "scale" | "origin">,
  epsilon = 0.0001,
): boolean {
  return Math.abs(left.scale - right.scale) < epsilon && distance(left.origin, right.origin) < epsilon;
}

function defaultSceneBounds(): DebugAabb {
  return { min: { x: -5, y: -3 }, max: { x: 5, y: 3 } };
}
