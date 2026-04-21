const state = {
  finalSnapshot: null,
  debugRender: null,
  traceEvents: [],
  perf: null,
  filters: {
    element: null,
    pair: null,
    phase: "",
  },
};

const dom = {
  runId: document.querySelector("#run-id"),
  files: document.querySelector("#artifact-files"),
  elementFilter: document.querySelector("#element-filter"),
  pairFilter: document.querySelector("#pair-filter"),
  phaseFilter: document.querySelector("#phase-filter"),
  clearFilters: document.querySelector("#clear-filters"),
  metricElements: document.querySelector("#metric-elements"),
  metricContacts: document.querySelector("#metric-contacts"),
  metricEvents: document.querySelector("#metric-events"),
  metricHash: document.querySelector("#metric-hash"),
  viewport: document.querySelector("#viewport"),
  contacts: document.querySelector("#contacts"),
  manifolds: document.querySelector("#manifolds"),
  timeline: document.querySelector("#timeline"),
  contactCount: document.querySelector("#contact-count"),
  manifoldCount: document.querySelector("#manifold-count"),
  timelineCount: document.querySelector("#timeline-count"),
};

const fixtureBase = new URL("./fixtures/contact-smoke/", window.location.href);

async function loadDefaultFixture() {
  const [finalSnapshot, debugRender, traceText, perf] = await Promise.all([
    fetchJson(new URL("final_snapshot.json", fixtureBase)),
    fetchJson(new URL("debug_render.json", fixtureBase)),
    fetchText(new URL("trace.jsonl", fixtureBase)),
    fetchJson(new URL("perf.json", fixtureBase)),
  ]);
  loadArtifacts({ finalSnapshot, debugRender, traceText, perf });
}

async function fetchJson(url) {
  const response = await fetch(url);
  if (!response.ok) throw new Error(`failed to load ${url}`);
  return response.json();
}

async function fetchText(url) {
  const response = await fetch(url);
  if (!response.ok) throw new Error(`failed to load ${url}`);
  return response.text();
}

function loadArtifacts({ finalSnapshot, debugRender, traceText, perf }) {
  state.finalSnapshot = finalSnapshot;
  state.debugRender = debugRender;
  state.traceEvents = parseTrace(traceText);
  state.perf = perf;
  populatePhaseFilter();
  render();
}

function parseTrace(text) {
  return text
    .split(/\r?\n/)
    .map((line) => line.trim())
    .filter(Boolean)
    .map((line) => JSON.parse(line));
}

function populatePhaseFilter() {
  const current = dom.phaseFilter.value;
  const phases = [...new Set(state.traceEvents.map((event) => event.phase))].sort();
  dom.phaseFilter.replaceChildren(option("", "Any phase"));
  for (const phase of phases) dom.phaseFilter.append(option(phase, phase));
  dom.phaseFilter.value = phases.includes(current) ? current : "";
}

function option(value, text) {
  const node = document.createElement("option");
  node.value = value;
  node.textContent = text;
  return node;
}

function render() {
  if (!state.finalSnapshot || !state.debugRender) return;
  const summary = summarize();
  dom.runId.textContent = summary.runId;
  dom.metricElements.textContent = String(summary.elements);
  dom.metricContacts.textContent = String(summary.contacts);
  dom.metricEvents.textContent = String(summary.events);
  dom.metricHash.textContent = summary.hash;

  drawViewport();
  renderContacts();
  renderManifolds();
  renderTimeline();
}

function summarize() {
  return {
    runId: state.finalSnapshot.run_id,
    hash: hashSnapshot(state.finalSnapshot).slice(0, 12),
    elements: state.finalSnapshot.elements.length,
    contacts: state.finalSnapshot.contacts.length,
    events: state.traceEvents.length,
  };
}

function filteredContacts() {
  return state.finalSnapshot.contacts.filter((contact) => {
    return matchesElement(contact.element_ids) && matchesPair(contact.element_ids);
  });
}

function visibleShapes() {
  return state.debugRender.shapes.filter((shape) => {
    if (state.filters.pair) return state.filters.pair.includes(shape.element_id);
    return state.filters.element == null || shape.element_id === state.filters.element;
  });
}

function filteredManifolds() {
  return state.finalSnapshot.manifolds.filter((manifold) => {
    return matchesElement(manifold.element_ids) && matchesPair(manifold.element_ids);
  });
}

function filteredTimeline() {
  return state.traceEvents.filter((event) => {
    const pair = event.pair_id || [];
    const phaseOk = !state.filters.phase || event.phase === state.filters.phase;
    return phaseOk && matchesElement(event.element_ids || []) && matchesPair(pair);
  });
}

function matchesElement(ids) {
  return state.filters.element == null || ids.includes(state.filters.element);
}

function matchesPair(ids) {
  if (!state.filters.pair) return true;
  return samePair(ids, state.filters.pair);
}

function samePair(left, right) {
  return (
    left.length === 2 &&
    ((left[0] === right[0] && left[1] === right[1]) ||
      (left[0] === right[1] && left[1] === right[0]))
  );
}

function renderContacts() {
  const contacts = filteredContacts();
  dom.contactCount.textContent = String(contacts.length);
  dom.contacts.replaceChildren(
    ...contacts.map((contact) =>
      item(
        `pair ${contact.element_ids.join("-")}`,
        `${contact.contact_id} · depth ${formatNumber(contact.depth)}`,
        "warn",
      ),
    ),
  );
}

function renderManifolds() {
  const manifolds = filteredManifolds();
  dom.manifoldCount.textContent = String(manifolds.length);
  dom.manifolds.replaceChildren(
    ...manifolds.map((manifold) =>
      item(
        `pair ${manifold.element_ids.join("-")}`,
        `active ${manifold.is_active} · points ${manifold.contact_point_count}`,
        "blue",
      ),
    ),
  );
}

function renderTimeline() {
  const events = filteredTimeline();
  dom.timelineCount.textContent = String(events.length);
  dom.timeline.replaceChildren(
    ...events.slice(0, 120).map((event) =>
      item(
        `tick ${event.tick} · ${event.phase}`,
        `${event.event_kind} · substep ${event.substep ?? "-"}`,
        "",
      ),
    ),
  );
}

function item(title, detail, className) {
  const node = document.createElement("div");
  node.className = `item ${className}`.trim();
  const strong = document.createElement("strong");
  strong.textContent = title;
  const span = document.createElement("span");
  span.textContent = detail;
  node.append(strong, span);
  return node;
}

function drawViewport() {
  const canvas = dom.viewport;
  const bounds = state.debugRender.world_bounds;
  const ctx = canvas.getContext("2d");
  const rect = canvas.getBoundingClientRect();
  const ratio = window.devicePixelRatio || 1;
  canvas.width = Math.max(1, Math.round(rect.width * ratio));
  canvas.height = Math.max(1, Math.round(rect.height * ratio));
  ctx.setTransform(ratio, 0, 0, ratio, 0, 0);
  ctx.clearRect(0, 0, rect.width, rect.height);
  ctx.fillStyle = "#fdfefb";
  ctx.fillRect(0, 0, rect.width, rect.height);
  drawGrid(ctx, rect.width, rect.height);
  if (!bounds) return;

  for (const shape of visibleShapes()) drawShape(ctx, rect, bounds, shape);
  for (const contact of filteredContacts()) drawContact(ctx, rect, bounds, contact);
  drawManifoldLabels(ctx, rect, bounds);
  drawOverlay(ctx, rect);
}

function drawGrid(ctx, width, height) {
  ctx.strokeStyle = "rgba(23,23,23,0.07)";
  ctx.lineWidth = 1;
  for (let x = 0; x < width; x += 32) line(ctx, x, 0, x, height);
  for (let y = 0; y < height; y += 32) line(ctx, 0, y, width, y);
}

function drawShape(ctx, rect, bounds, shape) {
  ctx.strokeStyle = "#1f2937";
  ctx.lineWidth = 2;
  for (const edge of shape.edges) {
    if (edge.kind === "line") {
      const start = worldToScreen(rect, bounds, edge.start);
      const end = worldToScreen(rect, bounds, edge.end);
      line(ctx, start.x, start.y, end.x, end.y);
    } else if (edge.kind === "circle") {
      const center = worldToScreen(rect, bounds, edge.center);
      ctx.beginPath();
      ctx.arc(center.x, center.y, edge.radius * viewportScale(rect, bounds), 0, Math.PI * 2);
      ctx.stroke();
    }
  }
}

function drawContact(ctx, rect, bounds, contact) {
  const point = worldToScreen(rect, bounds, contact.point);
  ctx.fillStyle = "#c73b31";
  ctx.beginPath();
  ctx.arc(point.x, point.y, 5, 0, Math.PI * 2);
  ctx.fill();
  ctx.strokeStyle = "#1d5fd1";
  ctx.lineWidth = 3;
  line(
    ctx,
    point.x,
    point.y,
    point.x + contact.normal_toward_a.x * 28,
    point.y - contact.normal_toward_a.y * 28,
  );
}

function drawManifoldLabels(ctx, rect, bounds) {
  const labels = state.debugRender.manifold_labels || [];
  ctx.fillStyle = "#171717";
  ctx.font = "12px monospace";
  for (const label of labels) {
    if (!matchesElement(label.element_ids || []) || !matchesPair(label.element_ids || [])) continue;
    const relatedContact = state.debugRender.contacts.find((contact) =>
      samePair(contact.element_ids, label.element_ids),
    );
    if (!relatedContact) continue;
    const point = worldToScreen(rect, bounds, relatedContact.point);
    ctx.fillText(label.text, point.x + 9, point.y - 9);
  }
}

function drawOverlay(ctx, rect) {
  const lines = state.debugRender.overlay_text || [];
  ctx.fillStyle = "rgba(255,255,255,0.88)";
  ctx.fillRect(12, 12, Math.min(420, rect.width - 24), Math.max(32, lines.length * 18 + 12));
  ctx.fillStyle = "#171717";
  ctx.font = "12px monospace";
  lines.forEach((lineText, index) => {
    ctx.fillText(lineText, 22, 32 + index * 18);
  });
}

function line(ctx, x1, y1, x2, y2) {
  ctx.beginPath();
  ctx.moveTo(x1, y1);
  ctx.lineTo(x2, y2);
  ctx.stroke();
}

function worldToScreen(rect, bounds, point) {
  const scale = viewportScale(rect, bounds);
  const min = bounds.min;
  const max = bounds.max;
  const worldWidth = Math.max(1, Math.abs(max.x - min.x));
  const worldHeight = Math.max(1, Math.abs(max.y - min.y));
  const offsetX = (rect.width - worldWidth * scale) * 0.5;
  const offsetY = (rect.height - worldHeight * scale) * 0.5;
  return {
    x: offsetX + (point.x - min.x) * scale,
    y: rect.height - offsetY - (point.y - min.y) * scale,
  };
}

function viewportScale(rect, bounds) {
  const worldWidth = Math.max(1, Math.abs(bounds.max.x - bounds.min.x));
  const worldHeight = Math.max(1, Math.abs(bounds.max.y - bounds.min.y));
  return Math.min(rect.width / worldWidth, rect.height / worldHeight) * 0.82;
}

function formatNumber(value) {
  return Number(value).toFixed(5);
}

function hashSnapshot(snapshot) {
  const raw = JSON.stringify(snapshot);
  let hash = 2166136261;
  for (let index = 0; index < raw.length; index += 1) {
    hash ^= raw.charCodeAt(index);
    hash = Math.imul(hash, 16777619);
  }
  return (hash >>> 0).toString(16).padStart(8, "0");
}

dom.elementFilter.addEventListener("input", () => {
  const value = dom.elementFilter.value.trim();
  state.filters.element = value ? Number(value) : null;
  render();
});

dom.pairFilter.addEventListener("input", () => {
  const value = dom.pairFilter.value.trim();
  const [left, right] = value.split(",").map((part) => Number(part.trim()));
  state.filters.pair = Number.isFinite(left) && Number.isFinite(right) ? [left, right] : null;
  render();
});

dom.phaseFilter.addEventListener("change", () => {
  state.filters.phase = dom.phaseFilter.value;
  render();
});

dom.clearFilters.addEventListener("click", () => {
  state.filters.element = null;
  state.filters.pair = null;
  state.filters.phase = "";
  dom.elementFilter.value = "";
  dom.pairFilter.value = "";
  dom.phaseFilter.value = "";
  render();
});

dom.files.addEventListener("change", async () => {
  const files = Object.fromEntries([...dom.files.files].map((file) => [file.name, file]));
  const finalSnapshot = await readJsonFile(files["final_snapshot.json"]);
  const debugRender = await readJsonFile(files["debug_render.json"]);
  const perf = await readJsonFile(files["perf.json"]);
  const traceText = await readTextFile(files["trace.jsonl"]);
  loadArtifacts({ finalSnapshot, debugRender, traceText, perf });
});

async function readJsonFile(file) {
  if (!file) throw new Error("missing artifact json file");
  return JSON.parse(await file.text());
}

async function readTextFile(file) {
  if (!file) throw new Error("missing artifact text file");
  return file.text();
}

window.addEventListener("resize", drawViewport);

loadDefaultFixture().catch((error) => {
  dom.runId.textContent = error.message;
  console.error(error);
});
