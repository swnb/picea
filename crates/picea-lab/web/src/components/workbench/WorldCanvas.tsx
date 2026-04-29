import { useEffect, useMemo, useRef, useState, type PointerEvent } from "react";
import type {
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
import type { LayerState } from "./types";

type WorldCanvasProps = {
  frames: FrameRecord[];
  frameIndex: number;
  selected: SelectedEntity | null;
  layers: LayerState;
  labels: {
    frame: string;
    colliders: string;
    contacts: string;
  };
  onSelect: (entity: SelectedEntity | null) => void;
};

type Camera = {
  scale: number;
  origin: Vec2;
  width: number;
  height: number;
};

export function WorldCanvas({ frames, frameIndex, selected, layers, labels, onSelect }: WorldCanvasProps) {
  const canvasRef = useRef<HTMLCanvasElement | null>(null);
  const containerRef = useRef<HTMLDivElement | null>(null);
  const [size, setSize] = useState({ width: 900, height: 600 });
  const frame = frames[Math.min(frameIndex, Math.max(0, frames.length - 1))];
  const camera = useMemo(
    () => makeCamera(frame?.snapshot.colliders ?? [], frame?.snapshot.primitives ?? [], size),
    [frame, size],
  );

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
    const canvas = canvasRef.current;
    if (!canvas || !frame) {
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
    drawWorld(ctx, frame, frames.slice(0, frameIndex + 1), camera, layers, selected);
  }, [camera, frame, frameIndex, frames, layers, selected, size]);

  function handlePointerDown(event: PointerEvent<HTMLCanvasElement>) {
    if (!frame) {
      return;
    }
    const rect = event.currentTarget.getBoundingClientRect();
    const point = screenToWorld({ x: event.clientX - rect.left, y: event.clientY - rect.top }, camera);
    const hit = hitTest(frame.snapshot.colliders, frame.snapshot.contacts, point);
    onSelect(hit);
  }

  return (
    <div ref={containerRef} className="relative h-full min-h-0 w-full overflow-hidden bg-lab-canvas">
      <canvas ref={canvasRef} className="block h-full w-full cursor-crosshair" onPointerDown={handlePointerDown} />
      <div className="pointer-events-none absolute left-3 top-3 flex items-center gap-2 rounded border border-lab-line bg-lab-panel/90 px-2 py-1 text-xs text-lab-muted">
        <span>{labels.frame} {frame?.frame_index ?? 0}</span>
        <span className="h-3 w-px bg-lab-line" />
        <span>{frame?.snapshot.colliders.length ?? 0} {labels.colliders}</span>
        <span>{frame?.snapshot.contacts.length ?? 0} {labels.contacts}</span>
      </div>
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
) {
  ctx.clearRect(0, 0, camera.width, camera.height);
  fillBackground(ctx, camera);
  drawGrid(ctx, camera);
  drawAxes(ctx, camera);

  if (layers.trace) {
    drawTrace(ctx, previousFrames, camera);
    drawPrimitives(ctx, frame.snapshot.primitives, camera);
  }

  if (layers.shapes) {
    for (const collider of frame.snapshot.colliders) {
      drawShape(ctx, collider, camera, isColliderSelected(collider, selected));
    }
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

  for (const joint of frame.snapshot.joints) {
    if (joint.anchors.length >= 2) {
      const start = worldToScreen(joint.anchors[0], camera);
      const end = worldToScreen(joint.anchors[1], camera);
      ctx.strokeStyle = "#d8ad5b";
      ctx.lineWidth = selected?.kind === "joint" && selected.id === joint.handle ? 3 : 1.5;
      ctx.beginPath();
      ctx.moveTo(start.x, start.y);
      ctx.lineTo(end.x, end.y);
      ctx.stroke();
    }
  }

  if (layers.contacts) {
    for (const contact of frame.snapshot.contacts) {
      drawContact(ctx, contact, camera, selected?.kind === "contact" && selected.id === contact.id);
    }
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
  const minor = 0.5;
  const major = 2;

  for (let step = Math.floor(worldTopLeft.x / minor) * minor; step <= worldBottomRight.x; step += minor) {
    const screen = worldToScreen({ x: step, y: 0 }, camera).x;
    ctx.strokeStyle = Math.abs(step % major) < 0.001 ? "#2b333d" : "#20262e";
    ctx.lineWidth = 1;
    ctx.beginPath();
    ctx.moveTo(screen, 0);
    ctx.lineTo(screen, camera.height);
    ctx.stroke();
  }

  for (let step = Math.floor(worldTopLeft.y / minor) * minor; step <= worldBottomRight.y; step += minor) {
    const screen = worldToScreen({ x: 0, y: step }, camera).y;
    ctx.strokeStyle = Math.abs(step % major) < 0.001 ? "#2b333d" : "#20262e";
    ctx.lineWidth = 1;
    ctx.beginPath();
    ctx.moveTo(0, screen);
    ctx.lineTo(camera.width, screen);
    ctx.stroke();
  }
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
  ctx.fillText("y", origin.x + 8, 16);
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
  const min = worldToScreen(aabb.min, camera);
  const max = worldToScreen(aabb.max, camera);
  ctx.strokeRect(min.x, max.y, max.x - min.x, min.y - max.y);
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
    const min = worldToScreen(bounds.min, camera);
    const max = worldToScreen(bounds.max, camera);
    const pad = 7;
    ctx.setLineDash([7, 4]);
    ctx.strokeRect(min.x - pad, max.y - pad, max.x - min.x + pad * 2, min.y - max.y + pad * 2);
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

function drawContact(ctx: CanvasRenderingContext2D, contact: DebugContact, camera: Camera, isSelected: boolean) {
  const point = worldToScreen(contact.point, camera);
  ctx.fillStyle = isSelected ? "#f0c36b" : "#d06464";
  ctx.beginPath();
  ctx.arc(point.x, point.y, isSelected ? 5 : 3.5, 0, Math.PI * 2);
  ctx.fill();
  drawArrow(ctx, camera, contact.point, contact.normal, isSelected ? "#f0c36b" : "#d06464", 0.8, 1);
}

function drawTrace(ctx: CanvasRenderingContext2D, frames: FrameRecord[], camera: Camera) {
  const points = frames
    .map((frame) => frame.snapshot.bodies.find((body) => body.body_type === "dynamic")?.transform.translation)
    .filter((point): point is Vec2 => Boolean(point));
  if (points.length < 2) {
    return;
  }
  ctx.strokeStyle = "rgba(127, 176, 105, 0.75)";
  ctx.lineWidth = 1.5;
  ctx.beginPath();
  points.slice(-96).forEach((point, index) => {
    const screen = worldToScreen(point, camera);
    if (index === 0) {
      ctx.moveTo(screen.x, screen.y);
    } else {
      ctx.lineTo(screen.x, screen.y);
    }
  });
  ctx.stroke();
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

function hitTest(colliders: DebugCollider[], contacts: DebugContact[], point: Vec2): SelectedEntity | null {
  const contactHit = contacts.find((contact) => distance(contact.point, point) < 0.18);
  if (contactHit) {
    return { kind: "contact", id: contactHit.id };
  }

  const colliderHit = [...colliders].reverse().find((collider) => colliderContains(collider, point));
  if (colliderHit) {
    return { kind: "collider", id: colliderHit.handle };
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

function makeCamera(
  colliders: DebugCollider[],
  primitives: DebugPrimitive[],
  size: { width: number; height: number },
): Camera {
  const bounds = aggregateSceneBounds(colliders, primitives) ?? { min: { x: -5, y: -3 }, max: { x: 5, y: 3 } };
  const padding = 1.2;
  const width = Math.max(2, bounds.max.x - bounds.min.x + padding * 2);
  const height = Math.max(2, bounds.max.y - bounds.min.y + padding * 2);
  const scale = Math.min(size.width / width, size.height / height);
  return {
    scale,
    origin: {
      x: size.width / 2 - ((bounds.min.x + bounds.max.x) / 2) * scale,
      y: size.height / 2 + ((bounds.min.y + bounds.max.y) / 2) * scale,
    },
    width: size.width,
    height: size.height,
  };
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

function worldToScreen(point: Vec2, camera: Camera): Vec2 {
  return {
    x: camera.origin.x + point.x * camera.scale,
    y: camera.origin.y - point.y * camera.scale,
  };
}

function screenToWorld(point: Vec2, camera: Camera): Vec2 {
  return {
    x: (point.x - camera.origin.x) / camera.scale,
    y: (camera.origin.y - point.y) / camera.scale,
  };
}

function distance(a: Vec2, b: Vec2): number {
  return Math.hypot(a.x - b.x, a.y - b.y);
}
