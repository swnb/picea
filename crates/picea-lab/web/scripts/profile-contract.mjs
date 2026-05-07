import assert from "node:assert/strict";
import fs from "node:fs";

function read(path) {
  return fs.readFileSync(new URL(path, import.meta.url), "utf8");
}

const profileSource = read("../src/profile.ts");
const apiSource = read("../src/api.ts");
const appSource = read("../src/App.tsx");
const canvasSource = read("../src/components/workbench/WorldCanvas.tsx");
const timelineSource = read("../src/components/workbench/Timeline.tsx");

assert.match(
  profileSource,
  /piceaProfileEnabled/,
  "Profile module should expose a guarded enable check instead of logging unconditionally.",
);
assert.match(
  profileSource,
  /picea-profile/,
  "Profile module should support the ?picea-profile=1 query switch.",
);
assert.match(
  profileSource,
  /picea\.lab\.profile/,
  "Profile module should support a durable localStorage opt-in for browser sessions.",
);
assert.match(
  profileSource,
  /__PICEA_PROFILE__/,
  "Profile samples should be inspectable from the browser global for Codex/browser debugging.",
);
assert.match(
  profileSource,
  /console\.table/,
  "Profile summaries should be readable from browser console logs.",
);

assert.match(
  apiSource,
  /profileJsonRequest/,
  "API JSON requests should be profiled so live step payload size and latency are visible.",
);
assert.match(
  apiSource,
  /export async function controlSession[\s\S]*requestJson<SessionControlResponse>\(`\/api\/sessions\/\$\{sessionId\}\/control`[\s\S]*detail/,
  "Live control requests should keep using requestJson while exposing the summary|full detail switch so api.json profiling covers the hot live step path.",
);
assert.match(
  apiSource,
  /export async function fetchLiveFrame[\s\S]*requestJson\(`\/api\/sessions\/\$\{sessionId\}\/frames\/\$\{frameIndex\}`/,
  "On-demand full frame hydration should also flow through requestJson so api.json profiling covers both summary step and full frame fetch.",
);
assert.match(
  appSource,
  /await controlSession\(sessionId, "step", detail\)/,
  "Live step playback should keep the summary|full detail choice inside the profiled request path.",
);
assert.match(
  appSource,
  /await fetchLiveFrame\(sessionId, frame\.frame_index\)/,
  "Paused inspection hydration should fetch full frames through the same profiled JSON path.",
);
assert.match(
  profileSource,
  /response\.text\(\)/,
  "Profiled API JSON requests should read response.text() so payload byte size remains observable.",
);
assert.match(
  profileSource,
  /JSON\.parse\(text\)/,
  "Profiled API JSON requests should parse from the captured text so transport and parse cost stay separable.",
);
assert.match(
  profileSource,
  /bytes: byteLength\(text\)/,
  "Profiled API JSON requests should record payload bytes for live payload regressions.",
);
assert.match(
  profileSource,
  /readTextMs:/,
  "Profiled API JSON requests should keep readText timing visible in samples.",
);
assert.match(
  profileSource,
  /parseJsonMs:/,
  "Profiled API JSON requests should keep JSON parse timing visible in samples.",
);
assert.match(
  appSource,
  /profileAsync\("live\.advanceFrame"/,
  "Live playback should profile the end-to-end frame advance path.",
);
assert.match(
  appSource,
  /profileMeasure\("live\.applyFrame"/,
  "Live frame application should be profiled separately from transport.",
);
assert.match(
  canvasSource,
  /profileMeasure\("canvas\.drawWorld"/,
  "Canvas redraw should be profiled with frame and scene counts.",
);
assert.match(
  timelineSource,
  /profileMeasure\("timeline\.derive"/,
  "Timeline derived markers should be profiled because they scale with buffered frames.",
);

assert.doesNotMatch(
  appSource + canvasSource + timelineSource,
  /<(?:Profile|ProfilePanel|ProfileBadge)\b|profile-panel|profile-badge/,
  "Profiling should stay opt-in and should not add permanent visible workbench chrome.",
);
