import assert from "node:assert/strict";
import fs from "node:fs";

const contractSources = [
  "../src/types.ts",
  "../src/App.tsx",
  "../src/api.ts",
  "../src/demo.ts",
  "../src/components/workbench/Toolbar.tsx",
  "../src/components/workbench/WorkbenchLayout.tsx",
  "../src/components/workbench/Timeline.tsx",
  "../src/components/workbench/RunSettingsPanel.tsx",
  "../src/components/workbench/GravityDial.tsx",
  "../src/components/workbench/parameters/ParameterPanel.tsx",
  "../src/components/workbench/SceneHierarchy.tsx",
  "../src/components/workbench/Inspector.tsx",
  "../src/components/workbench/WorldCanvas.tsx",
  "../src/components/workbench/types.ts",
  "../src/components/workbench/stackStability.ts",
  "../src/components/workbench/trajectory.ts",
  "../src/components/ui/radix.tsx",
  "../src/styles.css",
].map((path) => fs.readFileSync(new URL(path, import.meta.url), "utf8"));
const appSource = contractSources.join("\n");
const liveControlSource =
  appSource.match(/async function handleLiveControl[\s\S]*?\n  async function handleArtifactControl/)?.[0] ?? "";
const advanceLiveFrameSource =
  appSource.match(/async function advanceLiveFrame[\s\S]*?\n  function applyLiveSessionFrame/)?.[0] ?? "";
const applyLiveSessionFrameSource =
  appSource.match(/function applyLiveSessionFrame[\s\S]*?\n  function activateLiveSession/)?.[0] ?? "";
const artifactControlSource =
  appSource.match(/async function handleArtifactControl[\s\S]*?\n  function updateLayer/)?.[0] ?? "";
const perturbationCommitSource =
  appSource.match(/async function handleVelocityPerturbationCommit\(\)[\s\S]*?\n  function updateLayer/)?.[0] ?? "";
const overlayPresetSource =
  appSource.match(/function overlayPresetState[\s\S]*?\n}\n\nfunction selectTrajectoryContextMarkers/)?.[0] ?? "";
const timelineSource = fs.readFileSync(
  new URL("../src/components/workbench/Timeline.tsx", import.meta.url),
  "utf8",
);
const runSettingsSource = fs.readFileSync(
  new URL("../src/components/workbench/RunSettingsPanel.tsx", import.meta.url),
  "utf8",
);
const workbenchLayoutSource = fs.readFileSync(
  new URL("../src/components/workbench/WorkbenchLayout.tsx", import.meta.url),
  "utf8",
);
const parameterPanelSource = fs.readFileSync(
  new URL("../src/components/workbench/parameters/ParameterPanel.tsx", import.meta.url),
  "utf8",
);
const timelineHeaderSource =
  timelineSource.match(/const TimelineHeader = memo\([\s\S]*?\n\}\)/)?.[0] ?? "";
const toolbarSource = fs.readFileSync(
  new URL("../src/components/workbench/Toolbar.tsx", import.meta.url),
  "utf8",
);
const worldCanvasSource = fs.readFileSync(
  new URL("../src/components/workbench/WorldCanvas.tsx", import.meta.url),
  "utf8",
);
const i18nSource = fs.readFileSync(new URL("../src/i18n.ts", import.meta.url), "utf8");

assert.match(
  appSource,
  /kind:\s*"distance"\s*\|\s*"world_anchor"\s*\|\s*"revolute"/,
  "Web debug joint types must explicitly accept authoritative revolute facts.",
);
assert.match(
  i18nSource,
  /revolute:\s*"旋转铰链"[\s\S]*revolute:\s*"revolute joint"/,
  "Both locales must explicitly label revolute joints instead of falling back to a raw kind.",
);
assert.match(
  appSource,
  /dynamicValueLabel\(locale,\s*joint\.kind\)/,
  "Scene hierarchy must render the explicit revolute label from Rust joint facts.",
);
assert.match(
  appSource,
  /selected\.entity\.anchors\.map/,
  "Inspector must render both authoritative world anchors for a selected revolute joint.",
);
assert.match(
  timelineSource,
  /selectedEntity\?\.kind === "joint"[\s\S]*frame\?\.snapshot\.joints\.find[\s\S]*dynamicValueLabel\(locale, selectedJoint\.kind\)[\s\S]*join\(" · "\)/,
  "Timeline evidence must identify a selected joint by its explicit localized kind.",
);

assert.doesNotMatch(
  appSource,
  /items=\{supportedLocales\.map/,
  "Toolbar language control should be a single toggle button, not a locale Select dropdown.",
);
assert.doesNotMatch(
  appSource,
  /function toggleLocale\(/,
  "Toolbar language control should open an option popup instead of directly toggling locales.",
);
assert.match(
  appSource,
  /function LanguageMenu\(/,
  "Toolbar should use the same DropdownMenu popup pattern for language options as other header popups.",
);
assert.match(
  toolbarSource,
  /const runTriggerLabel =[\s\S]*(tooltip\.startRunScenario[\s\S]*tooltip\.rerunScenario|tooltip\.rerunScenario[\s\S]*tooltip\.startRunScenario)/,
  "Top toolbar run control should distinguish between starting a new run and rerunning an existing scenario instead of reading like resume playback.",
);
assert.doesNotMatch(
  toolbarSource,
  /tooltip\.runScenario/,
  "Top toolbar run control should stop using the generic runScenario copy once M41 clarifies new-run versus resume semantics.",
);
assert.match(
  toolbarSource,
  /<RotateCcw className="h-4 w-4" \/>/,
  "Top toolbar run control should use a rerun/restart icon instead of the playback icon so it does not look like timeline resume.",
);
assert.match(
  toolbarSource,
  /className="flex w-60 shrink-0 items-center gap-2[\s\S]*<div className="min-w-0 leading-tight">[\s\S]*scenario\.description/,
  "Top toolbar identity block should reserve a stable width so scenario descriptions cannot resize the header.",
);
assert.match(
  toolbarSource,
  /<Tooltip label=\{scenarioTooltip\}>[\s\S]*aria-label=\{scenarioTooltip\}[\s\S]*tabIndex=\{0\}[\s\S]*<div className="min-w-0 leading-tight">[\s\S]*scenario\.description[\s\S]*<\/Tooltip>/,
  "Top toolbar identity block should show the full scenario description (plus watch-for copy when available) in a tooltip when hovered.",
);
assert.match(
  toolbarSource,
  /const watchFor = scenarioWatchFor\(locale, selectedScenario\)[\s\S]*const scenarioTooltip = watchFor[\s\S]*scenario\.watchForLabel[\s\S]*: scenario\.description/,
  "Toolbar scenario tooltip should append localized watch-for copy after the description when the scenario ships one, and fall back to the description alone otherwise.",
);
assert.match(
  toolbarSource,
  /const isRunLoading = status === "loading"/,
  "Top toolbar should name the loading state so long artifact generation has an explicit UI contract.",
);
assert.match(
  toolbarSource,
  /runMode === "artifact_replay"[\s\S]*run\.generatingArtifact[\s\S]*run\.startingLiveSession/,
  "Top toolbar loading copy should distinguish artifact generation from live-session startup.",
);
assert.match(
  toolbarSource,
  /aria-busy=\{isRunLoading\}/,
  "Top toolbar run control should expose busy semantics while a new run is being created.",
);
assert.match(
  toolbarSource,
  /LoaderCircle[\s\S]*animate-spin/,
  "Top toolbar run control should show a spinner while artifact generation or live-session startup is pending.",
);
assert.match(
  toolbarSource,
  /className=\{cn\([\s\S]*"flex w-40 shrink-0 items-center gap-1\.5 text-xs text-lab-muted"[\s\S]*!isRunLoading && "invisible"/,
  "Top toolbar should reserve a stable progress slot instead of inserting and removing width during runs.",
);
assert.match(
  appSource,
  /ariaLabel=\{t\(locale, "scenario\.select"\)\}/,
  "Toolbar scenario selector should have its own accessible label instead of reusing the run action label.",
);
assert.match(
  appSource,
  /const \[gravityVector, setGravityVector\]/,
  "Run settings should keep a pending gravity vector instead of a y-only scalar.",
);
assert.doesNotMatch(
  appSource,
  /const \[gravityY, setGravityY\]/,
  "Gravity override should no longer be limited to a y-only pending value.",
);
assert.match(
  appSource,
  /\[gravityVector\.x, gravityVector\.y\]/,
  "New sessions should send the full pending gravity vector to the server override.",
);
assert.match(
  appSource,
  /export type ScenarioRuntimeConfig = \{/,
  "Web types should expose ScenarioRuntimeConfig for backend default/effective runtime config payloads.",
);
assert.match(
  appSource,
  /substeps_per_frame: number/,
  "Scenario runtime config should include substeps_per_frame from the backend contract.",
);
assert.match(
  appSource,
  /export type StepConfig = \{/,
  "Web types should expose StepConfig so runtime-config step fields stay typed.",
);
assert.match(
  appSource,
  /export type ScenarioParameterDescriptor = \{/,
  "Web types should expose ScenarioParameterDescriptor for scene parameter schemas.",
);
assert.match(
  appSource,
  /export type ScenarioParameterValue = number \| boolean \| string/,
  "Web types should model scene parameter values as scalar number/boolean/string overrides.",
);
assert.match(
  appSource,
  /default_runtime_config\?: ScenarioRuntimeConfig/,
  "Scenario descriptors should surface default_runtime_config from the server.",
);
assert.match(
  appSource,
  /parameter_schema\?: ScenarioParameterDescriptor\[\]/,
  "Scenario descriptors should surface parameter_schema from the server.",
);
assert.match(
  appSource,
  /scene_params\?: Record<string, ScenarioParameterValue> \| null/,
  "Session overrides and runtime config should carry typed scene_params maps.",
);
assert.match(
  appSource,
  /effective_runtime_config\?: ScenarioRuntimeConfig \| null/,
  "Session records should expose effective_runtime_config so the UI can show authoritative running values.",
);
assert.match(
  appSource,
  /sceneParams\?: Record<string, ScenarioParameterValue> \| null/,
  "createSession should accept optional scene params from the current scenario draft.",
);
assert.match(
  appSource,
  /scene_params: sceneParams \?\? undefined/,
  "createSession should serialize sceneParams into overrides.scene_params.",
);
assert.match(
  appSource,
  /const \[scenarioParamDrafts, setScenarioParamDrafts\]/,
  "App state should retain per-scenario scene parameter drafts instead of a single shared mutable map.",
);
assert.match(
  appSource,
  /default_runtime_config\?\.scene_params/,
  "App should seed parameter drafts from the selected scenario default_runtime_config.scene_params.",
);
assert.match(
  appSource,
  /effective_runtime_config\?\.scene_params/,
  "App should read back effective runtime config from session responses for running/revert behavior.",
);
assert.match(
  appSource,
  /const sceneParams =[\s\S]*createSession\([\s\S]*sceneParams,/,
  "Top-level run action should pass the current scenario scene param draft into createSession.",
);
assert.match(
  appSource,
  /function GravityDial\(/,
  "Run settings should expose a dedicated gravity dial component for direction and strength.",
);
assert.match(
  appSource,
  /role="slider"[\s\S]*aria-valuenow/,
  "Gravity dial should provide a keyboard/focusable accessibility surface.",
);
assert.match(
  appSource,
  /updateFromPointer\(event\.clientX, event\.clientY\)/,
  "Gravity dial should map pointer drag direction and length into the pending gravity vector.",
);
assert.match(
  appSource,
  /run\.gravityPendingActive/,
  "Run settings should tell users gravity edits apply on the next restart while a session exists.",
);
assert.match(
  appSource,
  /function applyLiveGravity[\s\S]*\/api\/sessions\/\$\{sessionId\}\/gravity/,
  "Run settings should call the dedicated live gravity endpoint instead of mutating generic overrides.",
);
assert.match(
  appSource,
  /async function handleApplyGravityPatch[\s\S]*applyLiveGravity[\s\S]*applyLiveSessionFrame/,
  "Apply should patch live gravity immediately and refresh the authoritative live frame.",
);
assert.match(
  appSource,
  /function handleResetGravityDraft[\s\S]*setGravityVector\(DEFAULT_GRAVITY\)/,
  "Reset should restore the pending gravity draft to the default vector without hiding the apply step.",
);
assert.match(
  appSource,
  /async function handleUndoGravityChange[\s\S]*gravityUndoVector[\s\S]*handleApplyGravityPatch/,
  "Undo should be able to apply the previous live gravity value after a committed Apply.",
);
assert.match(
  appSource,
  /run\.gravityDirtyLive/,
  "Run settings should distinguish unapplied live gravity edits from replay-only next-run edits.",
);
assert.match(
  runSettingsSource,
  /export function RunSettingsPanel\(/,
  "Run settings should be encapsulated behind a dedicated right-sidebar panel component.",
);
assert.match(
  runSettingsSource,
  /<PanelHeader[^>]*>[\s\S]*timeline\.runSetup/,
  "Run settings panel should keep its own localized right-sidebar header.",
);
assert.match(
  runSettingsSource,
  /max=\{6000\}/,
  "Run settings panel should keep the long-run frame cap visible in the right sidebar.",
);
assert.match(
  runSettingsSource,
  /<ParameterPanel[\s\S]*schema=\{scenario\.parameter_schema \?\? \[\]\}/,
  "Run settings panel should render the shared ParameterPanel using the selected scenario parameter schema.",
);
assert.match(
  runSettingsSource,
  /<GravityDial[\s\S]*\/>[\s\S]*<ParameterPanel/s,
  "Scene parameter controls should live in the right-sidebar run-settings layout with GravityDial instead of a separate detached surface.",
);
assert.doesNotMatch(
  timelineSource,
  /panel="run"/,
  "Bottom timeline should not keep a run-settings tab after moving runtime controls to the right sidebar.",
);
assert.doesNotMatch(
  timelineSource,
  /id="bottom-panel-run"/,
  "Bottom timeline should not render a run-settings tab panel after the sidebar migration.",
);
assert.match(
  workbenchLayoutSource,
  /rightSidebarMode[\s\S]*<Inspector[\s\S]*<RunSettingsPanel/s,
  "Workbench right sidebar should switch between Inspector and RunSettingsPanel.",
);
assert.match(
  parameterPanelSource,
  /export function ParameterPanel\(/,
  "Parameter UI should be encapsulated behind a dedicated ParameterPanel component.",
);
assert.match(
  parameterPanelSource,
  /export function ParameterRow\(/,
  "ParameterPanel should expose a reusable ParameterRow wrapper for dense settings layout.",
);
assert.match(
  parameterPanelSource,
  /export function NumberField\(/,
  "ParameterPanel should export NumberField for numeric parameter editing.",
);
assert.match(
  parameterPanelSource,
  /export function SliderField\(/,
  "ParameterPanel should export SliderField for numeric slider controls.",
);
assert.match(
  parameterPanelSource,
  /export function RangeField\(/,
  "ParameterPanel should export RangeField for slider plus numeric input composition.",
);
assert.match(
  parameterPanelSource,
  /export function ToggleField\(/,
  "ParameterPanel should export ToggleField for boolean parameters.",
);
assert.match(
  parameterPanelSource,
  /export function SegmentedField\(/,
  "ParameterPanel should export SegmentedField for enum/select parameters such as contact_position_correction.",
);
assert.match(
  parameterPanelSource,
  /export function ParameterSourceBadge\(/,
  "ParameterPanel should export ParameterSourceBadge for default/current/effective provenance labels.",
);
assert.match(
  parameterPanelSource,
  /Reset defaults/,
  "ParameterPanel should offer a reset-to-defaults action.",
);
assert.match(
  parameterPanelSource,
  /Revert running config/,
  "ParameterPanel should offer a revert-to-running-config action when authoritative runtime values are known.",
);
assert.match(
  appSource,
  /newton_cradle[\s\S]*default_runtime_config[\s\S]*parameter_schema/s,
  "Demo fallback should provide runtime defaults and parameter schema for newton_cradle.",
);
assert.match(
  appSource,
  /matrix_stack[\s\S]*default_runtime_config[\s\S]*parameter_schema/s,
  "Demo fallback should provide runtime defaults and parameter schema for matrix stack scenes.",
);
assert.match(
  appSource,
  /lattice_grid[\s\S]*default_runtime_config[\s\S]*parameter_schema/s,
  "Demo fallback should provide runtime defaults and parameter schema for lattice_grid.",
);
assert.match(
  appSource,
  /substeps_per_frame/,
  "Newton cradle scene parameter support should include substeps_per_frame through types, demo fallback, and UI wiring.",
);
assert.match(
  appSource,
  /SelectPrimitive\.Group/,
  "Scenario selector should support visible grouped sections for industrial debugger scenario categories.",
);
assert.match(
  appSource,
  /scenarioGroupForId/,
  "Scenario selector grouping should stay derived in the frontend instead of requiring server schema changes.",
);
assert.match(
  appSource,
  /tooltip\.overlayPresets/,
  "Toolbar should expose a compact overlay preset control for M40 closeout.",
);
assert.match(
  appSource,
  /overlayPresetLabel/,
  "Overlay preset menu should use localized preset labels.",
);
assert.match(
  appSource,
  /overlayPresetDescription/,
  "Overlay preset menu should keep short localized descriptions for each preset.",
);
assert.match(
  appSource,
  /function applyOverlayPreset\(/,
  "App should own a single overlay preset application path that updates layers and trajectory settings together.",
);
assert.match(
  appSource,
  /stackStability["']?,\s*\n?\s*"trajectoryFocus"[\s\S]*"perturbationReview"[\s\S]*"latticeGrid"/,
  "Overlay preset support should keep all four M40 closeout presets wired: stack stability, trajectory focus, perturbation review, and lattice grid.",
);
assert.match(
  overlayPresetSource,
  /preset === "stackStability"[\s\S]*stackStability:\s*true[\s\S]*islands:\s*true[\s\S]*mode:\s*"selectedIsland"[\s\S]*colorMode:\s*"island"/,
  "Stack-stability preset should enable the stability/island review layers and switch trajectories to selected-island island coloring.",
);
assert.match(
  overlayPresetSource,
  /preset === "trajectoryFocus"[\s\S]*contacts:\s*false[\s\S]*velocities:\s*false[\s\S]*mode:\s*"selectedBody"[\s\S]*historyLength:\s*128[\s\S]*samplingStride:\s*1/,
  "Trajectory-focus preset should reduce non-trajectory chrome and keep dense selected-body sampling.",
);
assert.match(
  overlayPresetSource,
  /preset === "perturbationReview"[\s\S]*contacts:\s*true[\s\S]*velocities:\s*true[\s\S]*provenance:\s*true[\s\S]*stackStability:\s*true[\s\S]*mode:\s*"selectedBody"/,
  "Perturbation-review preset should show provenance, contacts, velocities, and recent selected-body trajectory facts.",
);
assert.match(
  overlayPresetSource,
  /return \{[\s\S]*contacts:\s*false[\s\S]*velocities:\s*false[\s\S]*islands:\s*true[\s\S]*lattice:\s*true[\s\S]*mode:\s*"selectedIsland"/,
  "Lattice-grid preset should emphasize the rigid-body lattice proxy and island context without contact/velocity clutter.",
);
assert.match(
  appSource,
  /position="popper"/,
  "Shared Select popup should use stable popper positioning near the fixed header.",
);
assert.match(
  appSource,
  /sideOffset=\{6\}/,
  "Shared Select popup should keep a small offset from the header trigger.",
);
assert.match(
  appSource,
  /--radix-select-trigger-width/,
  "Shared Select popup should keep its width anchored to the trigger instead of shifting during scroll.",
);
assert.match(
  appSource,
  /inline-grid h-8 min-w-40 grid-cols-\[minmax\(0,1fr\)_1rem\][^"]*overflow-hidden[^"]*whitespace-nowrap/,
  "Shared Select trigger should reserve a fixed icon column and prevent long labels from wrapping out of the trigger height.",
);
assert.match(
  appSource,
  /SelectPrimitive\.Value className="min-w-0 overflow-hidden truncate whitespace-nowrap"/,
  "Shared Select trigger value should stay single-line and truncate inside its own grid cell instead of resizing the trigger chrome.",
);
assert.doesNotMatch(
  appSource,
  /SelectPrimitive\.Scroll(?:Up|Down)Button/,
  "Shared Select should use native viewport scrolling instead of extra up/down arrow rows that destabilize long menus.",
);
assert.match(
  appSource,
  /\[scrollbar-gutter:stable\]/,
  "Shared Select viewport should reserve scrollbar gutter so long option lists do not shift text or trigger chrome.",
);
assert.match(
  appSource,
  /\[data-radix-select-viewport\][\s\S]*scrollbar-width:\s*thin !important/,
  "Shared Select viewport should restore a thin native scrollbar instead of relying on unstable arrow affordances.",
);
assert.match(
  appSource,
  /overscroll-contain/,
  "Shared Select viewport should contain wheel/touch scroll so upward scroll does not bubble into the workbench.",
);
assert.match(
  appSource,
  /<Button[^>]*\bsize="icon"[^>]*\bvariant="outline"[^>]*\baria-label=\{t\(locale, "app\.language"\)\}/,
  "Toolbar language trigger should be icon-only while keeping an accessible label.",
);
assert.match(
  appSource,
  /onCloseAutoFocus=\{handleCloseAutoFocus\}/,
  "Language popup should not return focus to the trigger after a mouse selection closes the menu.",
);
assert.match(
  appSource,
  /triggerRef\.current\?\.blur\(\)/,
  "Language trigger should clear the active focus highlight after the popup closes.",
);
assert.ok(
  (appSource.match(/onCloseAutoFocus=\{handleCloseAutoFocus\}/g) ?? []).length >= 2,
  "Every header popup button should prevent Radix from returning focus to its trigger after close.",
);
assert.ok(
  (appSource.match(/triggerRef\.current\?\.blur\(\)/g) ?? []).length >= 2,
  "Every header popup button should clear its trigger focus highlight after close.",
);
assert.match(
  appSource,
  /onSelect=\{\(event\) => event\.preventDefault\(\)\}/,
  "Layer checkbox items should prevent Radix DropdownMenu's default select-close behavior.",
);
assert.match(
  appSource,
  /generic_convex_trace/,
  "Contact inspector should render M7 generic convex GJK/EPA trace facts.",
);
assert.match(
  appSource,
  /fact\.genericFallback/,
  "Contact inspector should label the generic convex fallback decision.",
);
assert.match(
  appSource,
  /fact\.gjk/,
  "Contact inspector should label GJK termination and iteration facts.",
);
assert.match(
  appSource,
  /fact\.epa/,
  "Contact inspector should label EPA termination and iteration facts.",
);
assert.match(
  appSource,
  /ccd_trace\.target_kind/,
  "Contact inspector should expose CCD target kind for dynamic target traces.",
);
assert.match(
  appSource,
  /ccd_trace\.target_swept_start/,
  "Contact inspector should expose CCD target swept start.",
);
assert.match(
  appSource,
  /ccd_trace\.target_swept_end/,
  "Contact inspector should expose CCD target swept end.",
);
assert.match(
  appSource,
  /ccd_trace\.target_clamp/,
  "Contact inspector should expose CCD target clamp distance.",
);
assert.match(
  appSource,
  /tree\.joints/,
  "Scene hierarchy should expose joints as selectable replay artifacts.",
);
assert.match(
  appSource,
  /frame\.snapshot\.joints\.map/,
  "Scene hierarchy should render joint rows from the debug snapshot.",
);
assert.match(
  appSource,
  /log\.sseIdle/,
  "Workbench logs should distinguish an empty SSE queue from a failed run.",
);
assert.match(
  appSource,
  /final_snapshot_artifact/,
  "Workbench should surface final_snapshot artifact provenance from the server session.",
);
assert.match(
  appSource,
  /live_session/,
  "Workbench should expose a distinct Rust live session path alongside artifact replay.",
);
assert.match(
  appSource,
  /source === "artifact" && sessionId[\s\S]*handleArtifactControl\(action, sessionId\)/,
  "Rust artifact replay controls should keep using the server session instead of falling back to local demo scrubbing.",
);
assert.match(
  appSource,
  /async function handleArtifactControl[\s\S]*controlSession\(activeSessionId, action\)[\s\S]*fetchFrames\(session\.run_id\)/,
  "Artifact reset should ask the Rust server for a new run and reload its frames.",
);
assert.doesNotMatch(
  artifactControlSource,
  /setFrameIndex\(\s*Math\.min\(session\.current_frame_index/,
  "Artifact pause/play/step should not overwrite the local timeline with stale server current_frame_index.",
);
assert.match(
  liveControlSource,
  /if \(action === "reset"\)[\s\S]*else if \(action === "play"\) \{[\s\S]*applyLiveSessionFrame\(result, guard\)[\s\S]*setFrameIndex\(session\.current_frame_index\)[\s\S]*setStatus\("playing"\)/,
  "Live play should merge any server-returned authoritative frame before re-anchoring the local cursor and entering playing state.",
);
assert.match(
  liveControlSource,
  /else if \(action === "pause"\) \{[\s\S]*applyLiveSessionFrame\(result, guard\)[\s\S]*setStatus\("paused"\)/,
  "Live pause should merge any server-returned authoritative frame before showing the paused cursor.",
);
assert.match(
  appSource,
  /liveGenerationRef/,
  "Live controls should track a session generation so reset can invalidate older live responses.",
);
assert.match(
  appSource,
  /liveRequestTokenRef/,
  "Live controls should track request tokens so older step/reset responses cannot overwrite the current buffer.",
);
assert.match(
  appSource,
  /if\s*\(\s*guard\.generation\s*!==\s*liveGenerationRef\.current\s*\|\|\s*guard\.token\s*!==\s*liveRequestTokenRef\.current\s*\)/,
  "Workbench should discard stale live responses before mutating the buffered frame list.",
);
assert.match(
  appSource,
  /timeline\.stack/,
  "Bottom timeline should expose a dedicated Stack/Stability panel tab.",
);
assert.match(
  appSource,
  /stackStability/,
  "Workbench layer controls should expose a stack stability overlay toggle.",
);
assert.match(
  appSource,
  /stability\.derived/,
  "Stack stability summary should be labeled as web-derived display instead of authoritative physics fact.",
);
assert.match(
  appSource,
  /stability\.missing/,
  "Stack stability views should surface missing evidence explicitly instead of silently reporting zero.",
);
assert.match(
  appSource,
  /stability\.marker\./,
  "Stack stability panel should expose labeled marker categories for timeline jumps.",
);
assert.match(
  appSource,
  /export type FrameDiagnostics/,
  "Web types should model the lab-owned FrameDiagnostics artifact schema.",
);
assert.match(
  appSource,
  /diagnostics\?:\s*FrameDiagnostics/,
  "FrameRecord should carry optional lab diagnostics so older artifacts remain readable.",
);
assert.match(
  timelineSource,
  /buildDiagnosticTimelineMarkers\(/,
  "Timeline should include exported lab diagnostics markers as first-bad-frame jump targets.",
);
assert.match(
  timelineSource,
  /diagnosticSourceLabel\(/,
  "Diagnostics UI should label Rust authoritative facts separately from lab-derived facts.",
);
assert.match(
  timelineSource,
  /diagnostics\.missingEvidence/,
  "Diagnostics UI should expose missing evidence instead of showing absent facts as zero.",
);
assert.match(
  timelineSource,
  /<TimelineMarkerRail[\s\S]*markers=\{railMarkers\}/,
  "Timeline tab should render a combined marker rail near the slider instead of hiding stack and diagnostics markers in subpanels.",
);
assert.match(
  timelineSource,
  /const railMarkers = pickRailMarkers\(\[[\s\S]*diagnosticMarkers[\s\S]*stackMarkers[\s\S]*trajectoryMarkers/,
  "Timeline marker rail should combine exported diagnostics markers with existing stack and trajectory markers.",
);
assert.match(
  timelineSource,
  /pickRailMarkers\(/,
  "Main timeline marker rail should use an explicit balanced picker instead of a raw combined slice.",
);
assert.doesNotMatch(
  timelineSource,
  /\.sort\(\(left, right\) => left\.frameIndex - right\.frameIndex \|\| right\.score - left\.score\)\s*\.slice\(0, 12\)/,
  "Main timeline marker rail should not drop later important markers by slicing a time-sorted combined list.",
);
assert.match(
  appSource,
  /stability\.marker\.angularDrift/,
  "Timeline marker labels should include large angular-drift support.",
);
assert.match(
  appSource,
  /stability\.contribution/,
  "Inspector should explain selected body/contact contribution to recent stack stability.",
);
assert.match(
  appSource,
  /timeline\.trajectory/,
  "Bottom timeline should expose a dedicated trajectory panel tab.",
);
assert.match(
  appSource,
  /scenario:\s*\{\s*id:\s*selectedScenario[\s\S]*name:\s*scenario\.name[\s\S]*description:\s*scenario\.description/,
  "Copy debug context should include active scenario id, localized name, and description.",
);
assert.match(
  appSource,
  /stackStability: hasUnhydratedLiveFrames \? null : stackSummary/,
  "Copy debug context should include the derived stack stability summary only after live summary frame history is hydrated.",
);
assert.match(
  appSource,
  /frame: currentFrameNotHydrated \? null : currentFrame\.diagnostics/,
  "Copy debug context should include exported diagnostics only after live summary frames are hydrated.",
);
assert.match(
  appSource,
  /available:\s*boolean[\s\S]*marker_count:\s*number \| null/,
  "Diagnostics debug context summary should distinguish missing diagnostics from a real zero-marker frame.",
);
assert.match(
  appSource,
  /available:\s*false[\s\S]*marker_count:\s*null[\s\S]*frame_diagnostics/,
  "Old artifacts without diagnostics should stay explicitly missing in copy debug context instead of becoming m0.",
);
assert.match(
  appSource,
  /trajectory:\s*\{\s*settings:\s*trajectorySettings[\s\S]*overlay:/,
  "Copy debug context should include trajectory settings plus overlay summary.",
);
assert.match(
  appSource,
  /latticeProxy:\s*\{[\s\S]*enabled: !hasUnhydratedLiveFrames && latticeSummary\.enabled/,
  "Copy debug context should include an explicit lattice proxy summary only after live summary frames are hydrated.",
);
assert.match(
  appSource,
  /currentFrameProvenance:\s*currentFramePerturbationProvenance/,
  "Copy debug context should include accepted current-frame perturbation provenance summaries.",
);
assert.match(
  timelineSource,
  /evidence\.trajectory/,
  "Evidence panel should surface a trajectory summary row before the raw JSON preview.",
);
assert.match(
  timelineSource,
  /evidence\.stack/,
  "Evidence panel should surface a stack summary row before the raw JSON preview.",
);
assert.match(
  timelineSource,
  /evidence\.diagnostics/,
  "Evidence panel should summarize exported diagnostics before the raw JSON preview.",
);
assert.match(
  timelineSource,
  /summary\.available === false[\s\S]*"missing"/,
  "Evidence diagnostics preview should show missing diagnostics explicitly instead of rendering marker_count null as zero.",
);
assert.match(
  timelineSource,
  /severityRank\([\s\S]*marker\.severity/,
  "Timeline marker priority should preserve D2 first-bad-frame severity before later high-score spikes.",
);
assert.doesNotMatch(
  timelineSource,
  /diagnosticMarkerLabel\("en-US"/,
  "Diagnostics missing-evidence details should use the active locale instead of hardcoding English.",
);
assert.match(
  timelineSource,
  /evidence\.lattice/,
  "Evidence panel should surface a lattice proxy summary row before the raw JSON preview.",
);
assert.match(
  timelineSource,
  /evidence\.perturbation/,
  "Evidence panel should surface a perturbation summary row before the raw JSON preview.",
);
assert.match(
  appSource,
  /trajectorySettings/,
  "App should own a separate trajectorySettings state instead of overloading layer toggles.",
);
assert.match(
  appSource,
  /layers\.trace/,
  "Trace layer visibility should remain the master toggle for trajectory rendering.",
);
assert.match(
  appSource,
  /"selectedBody"/,
  "Trajectory settings should include a selected-body mode.",
);
assert.match(
  appSource,
  /"allDynamic"/,
  "Trajectory settings should include an all-dynamic mode.",
);
assert.match(
  appSource,
  /trajectory\.empty\.allDynamic/,
  "Trajectory empty-state labels should expose a dedicated all-dynamic copy path.",
);
assert.match(
  appSource,
  /case "allDynamic":[\s\S]*trajectory\.empty\.allDynamic/,
  "All-dynamic trajectory mode should render its own empty state instead of falling through to unsupported selection.",
);
assert.match(
  appSource,
  /"selectedIsland"/,
  "Trajectory settings should include a selected-island mode.",
);
assert.match(
  appSource,
  /"contacts"/,
  "Trajectory settings should include a contact trail mode.",
);
assert.match(
  appSource,
  /"ccd"/,
  "Trajectory settings should include a CCD trail mode.",
);
assert.match(
  appSource,
  /historyLength/,
  "Trajectory workbench should expose a history-length control.",
);
assert.match(
  appSource,
  /samplingStride/,
  "Trajectory workbench should expose a sampling-stride control.",
);
assert.match(
  appSource,
  /fade/,
  "Trajectory workbench should expose a fade control.",
);
assert.match(
  appSource,
  /colorMode/,
  "Trajectory workbench should expose a trajectory color-mode control.",
);
assert.match(
  appSource,
  /trajectory\.summary\.derived/,
  "Inspector trajectory summary should be explicitly labeled as web-derived.",
);
assert.match(
  appSource,
  /trajectory\.summary\.distance/,
  "Inspector trajectory summary should include traveled distance.",
);
assert.match(
  appSource,
  /trajectory\.summary\.maxSpeed/,
  "Inspector trajectory summary should include max speed.",
);
assert.match(
  appSource,
  /trajectory\.summary\.awakeSleep/,
  "Inspector trajectory summary should report awake/sleep coverage.",
);
assert.match(
  appSource,
  /trajectory\.summary\.contactCount/,
  "Inspector trajectory summary should report contact counts over the trail window.",
);
assert.match(
  appSource,
  /trajectory\.summary\.ccdEvidence/,
  "Inspector trajectory summary should report CCD evidence when present.",
);
assert.match(
  appSource,
  /trajectory\.marker\.bodyMotion/,
  "Timeline markers should include large body-motion jumps.",
);
assert.match(
  appSource,
  /trajectory\.marker\.contactAppeared/,
  "Timeline markers should include contact-appeared jumps.",
);
assert.match(
  appSource,
  /trajectory\.marker\.contactDisappeared/,
  "Timeline markers should include contact-disappeared jumps.",
);
assert.match(
  appSource,
  /trajectory\.marker\.contactBurst/,
  "Timeline markers should include contact-burst jumps.",
);
assert.match(
  appSource,
  /trajectory\.marker\.ccdClamp/,
  "Timeline markers should include CCD clamp jumps.",
);
assert.match(
  appSource,
  /trajectory\.marker\.ccdHit/,
  "Timeline markers should include CCD hit jumps.",
);
assert.match(
  appSource,
  /feature_id/,
  "Contact trajectory lineage should use exported feature identifiers when available.",
);
assert.match(
  appSource,
  /findSelectedContactLineage\(/,
  "Selected-contact trajectory summary should recover lineage from nearby exported frames when the current contact id rotated away.",
);
assert.match(
  appSource,
  /contact\.id === selected\.id/,
  "Contact trajectory summary should still prefer exact contact id matches when the current frame still exports the selected contact.",
);
assert.match(
  appSource,
  /approximateNormalBucket\(/,
  "Recovered contact lineage should derive an approximate normal bucket for conservative matching.",
);
assert.match(
  appSource,
  /normalBucket/,
  "Recovered contact lineage should keep a normal-direction guard to avoid merging unrelated contacts.",
);
assert.match(
  appSource,
  /target_swept_start/,
  "CCD trajectory rendering should use exported target sweep start facts when available.",
);
assert.match(
  appSource,
  /target_swept_end/,
  "CCD trajectory rendering should use exported target sweep end facts when available.",
);
assert.match(
  appSource,
  /panel\.processFacts/,
  "Inspector should expose a process facts panel for broadphase tree, island lifecycle, and compound provenance.",
);
assert.match(
  appSource,
  /frame\.snapshot\.contacts[\s\S]*solver_normal_impulse/,
  "Stack stability overlay or panel should read impulse heat from exported contact impulses instead of recomputing physics.",
);
assert.doesNotMatch(
  timelineSource,
  /const value = Number\(current\[key\] \?\? 0\)[\s\S]*const previousValue = Number\(previous\?\.\[key\] \?\? 0\)[\s\S]*const delta = previous \? value - previousValue : 0/,
  "Diagnostics rows should not fabricate zero current values or deltas when optional counters are absent.",
);
assert.doesNotMatch(
  timelineSource,
  /summary\.total_broadphase_candidate_count \?\? 0|summary\.total_broadphase_traversal_count \?\? 0|summary\.total_broadphase_pruned_count \?\? 0|summary\.max_broadphase_tree_depth \?\? 0|summary\.total_island_count \?\? 0|summary\.total_active_island_count \?\? 0|summary\.total_sleeping_island_skip_count \?\? 0|summary\.total_solver_body_slot_count \?\? 0|summary\.total_contact_row_count \?\? 0|summary\.total_joint_row_count \?\? 0|summary\.total_ccd_candidate_count \?\? 0|summary\.total_ccd_hit_count \?\? 0|perfArtifact\?\.elapsed_micros \?\? 0/,
  "Perf summary should surface missing evidence instead of authoritative zeroes for optional counters.",
);
assert.doesNotMatch(
  appSource,
  /Math\.abs\(contact\.solver_normal_impulse \?\? 0\)\s*\+\s*Math\.abs\(contact\.solver_tangent_impulse \?\? 0\)/,
  "Stack stability overlay should not render missing solver impulses as weak zero-heat contacts.",
);
assert.match(
  appSource,
  /contact\.id[\s\S]*contact\.feature_id/,
  "Selected-contact stability contribution should scope by contact identity and feature id, not just body pair.",
);
assert.match(
  appSource,
  /inspector\.broadphaseTree/,
  "Process facts should label the exported broadphase tree read model.",
);
assert.match(
  appSource,
  /inspector\.islandLifecycle/,
  "Process facts should label island lifecycle facts.",
);
assert.match(
  appSource,
  /inspector\.compoundProvenance/,
  "Process facts should label compound provenance facts.",
);
assert.match(
  appSource,
  /broadphaseTree/,
  "Layer controls should expose a broadphase tree overlay toggle.",
);
assert.match(
  appSource,
  /layers\.islands/,
  "World canvas should expose an island overlay toggle and draw path.",
);
assert.match(
  appSource,
  /layers\.provenance/,
  "World canvas should expose a provenance overlay toggle and draw path.",
);
assert.match(
  appSource,
  /layers\.lattice/,
  "World canvas should expose a lattice proxy overlay toggle and draw path.",
);
assert.match(
  appSource,
  /lattice_grid/,
  "Workbench should expose the lattice_grid scenario across Rust and demo paths.",
);
assert.match(
  appSource,
  /rigid-body joint lattice/i,
  "M38 copy should explicitly call the feature a rigid-body joint lattice/grid proxy.",
);
assert.match(
  appSource,
  /not a true soft-body|不是 true soft-body/u,
  "M38 copy should explicitly say the proxy is not a true soft-body solver.",
);
assert.match(
  appSource,
  /deriveLatticeProxy|buildLatticeProxy/,
  "Workbench should derive lattice proxy facts from existing snapshot joints/bodies instead of inventing new solver state.",
);
assert.match(
  appSource,
  /drawLatticeProxy|drawLatticeOverlay/,
  "World canvas should render a dedicated lattice proxy overlay for nodes, edges, and anchors.",
);
assert.match(
  appSource,
  /stretchRatio|maxStretchRatio/,
  "Inspector and timeline should surface derived lattice stretch ratios.",
);
assert.match(
  appSource,
  /panel\.latticeProxy|timeline\.lattice|inspector\.latticeProxy/,
  "Workbench should expose lattice proxy summary labels in inspector and timeline surfaces.",
);
assert.match(
  appSource,
  /liveControlBusy/,
  "Live controls should expose an in-flight busy state so step/play/reset cannot race stale backend responses.",
);
assert.match(
  liveControlSource,
  /async function handleLiveControl[\s\S]*const result = await controlSession\(activeSessionId, action\)[\s\S]*const session = result\.session[\s\S]*if \(action === "play"\) \{[\s\S]*setFrameIndex\(session\.current_frame_index\)[\s\S]*setStatus\("playing"\)/,
  "Live play should re-anchor to the Rust-authoritative current frame and enter playing only after the control endpoint acknowledges play.",
);
assert.doesNotMatch(
  liveControlSource,
  /async function handleLiveControl[\s\S]*if \(action === "play"\) \{\s*setStatus\("playing"\)[\s\S]*const result = await controlSession\(activeSessionId, action\)/,
  "Live play should not start the frame interval before the Rust play acknowledgement returns.",
);
assert.match(
  appSource,
  /async function startLiveSessionFromRun[\s\S]*controlSession\(nextSessionId, "play"\)[\s\S]*setStatus\("playing"\)/,
  "Top-level Run selected scenario should start Rust live sessions instead of leaving them in the created state.",
);
assert.match(
  appSource,
  /} else if \(action === "play"\) \{[\s\S]*setFrameIndex\(\(value\) => \{[\s\S]*const lastFrameIndex = frames\[frames\.length - 1\]\?\.frame_index \?\? 0[\s\S]*if \(value >= lastFrameIndex && lastFrameIndex > 0\) \{[\s\S]*return frames\[0\]\?\.frame_index \?\? 0[\s\S]*}\s*return value[\s\S]*}\)/,
  "Finite playback should restart from the first retained frame when play is triggered after the viewer is already parked at the end.",
);
assert.match(
  timelineSource,
  /const TimelineHeader = memo\(/,
  "Bottom timeline should split stable playback chrome into a memoized TimelineHeader.",
);
assert.doesNotMatch(
  timelineHeaderSource,
  /state_hash|simulated_time|toFixed\(3\)/,
  "TimelineHeader should not render per-frame time/hash values that make the tab chrome flash during live playback.",
);
assert.doesNotMatch(
  timelineHeaderSource,
  /Tabs\.(List|Trigger)/,
  "TimelineHeader should not consume Radix Tabs context because live frame updates can still repaint the tab chrome.",
);
assert.match(
  timelineSource,
  /type BottomPanelId =[\s\S]*"timeline"[\s\S]*"logs"[\s\S]*"diagnostics"[\s\S]*"evidence"/,
  "Bottom timeline should keep its active panel in local primitive state outside Radix Tabs context.",
);
assert.doesNotMatch(
  timelineSource,
  /type BottomPanelId =[\s\S]*"run"/,
  "Bottom timeline primitive panel ids should not include run settings after the sidebar migration.",
);
assert.match(
  timelineSource,
  /function BottomPanelTabButton\(/,
  "Bottom timeline should render stable plain tab buttons so the header is isolated from per-frame content updates.",
);
assert.match(
  timelineSource,
  /function FrameIdentityRow\(/,
  "Per-frame time/hash identity should live inside the timeline content instead of the tab chrome.",
);
assert.match(
  timelineSource,
  /type PlaybackToggleState = "play" \| "pause" \| "replay"/,
  "Timeline playback chrome should model a single toggle state machine for play, pause, and finite-end replay.",
);
assert.match(
  timelineSource,
  /function resolvePlaybackToggleState\(/,
  "Timeline should derive one playback toggle state from source, status, frame index, and buffered length instead of rendering separate play and pause buttons.",
);
assert.doesNotMatch(
  timelineHeaderSource,
  /label=\{t\(locale, "tooltip\.pausePlayback"\)\}[\s\S]*label=\{t\(locale, "tooltip\.playTimeline"\)\}/,
  "Timeline header should not render separate pause and play buttons after M41 merges them into one toggle.",
);
assert.match(
  timelineSource,
  /const playbackAction =[\s\S]*playbackState === "pause"[\s\S]*onPause[\s\S]*playbackState === "replay"[\s\S]*onReset[\s\S]*onPlay/,
  "Timeline header toggle should pause while running, resume the current session while paused, and switch to reset-to-replay at a finite end.",
);
assert.match(
  timelineSource,
  /const playbackLabel =[\s\S]*playbackState === "pause"[\s\S]*tooltip\.pausePlayback[\s\S]*tooltip\.replayTimeline[\s\S]*tooltip\.playTimeline/,
  "Timeline header toggle should expose localized pause, resume-current-session, and replay-from-start labels.",
);
assert.match(
  appSource,
  /if \(shouldLogLiveFrameBuffer\(nextFrame\.frame_index\)\) \{\s*pushLogs\(/,
  "Live frame buffering logs should be sampled so the hidden log tab is not updated every frame.",
);
assert.match(
  advanceLiveFrameSource,
  /detail = "summary"/,
  "Automatic live playback should default to summary detail instead of shipping full frame payloads every step.",
);
assert.match(
  appSource,
  /await advanceLiveFrame\(\{ logAccepted: true, detail: "full" \}\)/,
  "Explicit manual live step should request a full frame so paused inspection keeps authoritative detail.",
);
assert.match(
  appSource,
  /function frameFromLiveSummary\(/,
  "App should materialize live summary payloads into a renderable frame shape without pretending they are fully hydrated artifacts.",
);
assert.match(
  timelineSource,
  /function isLiveSummaryFrame\(/,
  "Timeline should recognize live summary frames before running derived evidence helpers.",
);
assert.match(
  timelineSource,
  /hasUnhydratedFrames[\s\S]*skipped: "live-summary-not-hydrated"/,
  "Timeline derived evidence should skip stack/trajectory/lattice derivation while live summary frames are not hydrated.",
);
assert.match(
  appSource,
  /hasUnhydratedLiveFrames \? null : trajectorySummary/,
  "Copy debug context should not export trajectory summaries derived from unhydrated live summary frame history.",
);
assert.match(
  appSource,
  /stackStability: hasUnhydratedLiveFrames \? null : stackSummary/,
  "Copy debug context should not export stack stability summaries derived from unhydrated live summary frame history.",
);
assert.match(
  appSource,
  /missing_evidence:\s*\["live_summary_not_hydrated"\]/,
  "Copy debug context diagnostics should report unhydrated live summaries as missing evidence.",
);
assert.match(
  appSource,
  /contactCount: hasUnhydratedLiveFrames \? null : latticeSummary\.contactCount/,
  "Copy debug context should not export lattice/contact counts derived from unhydrated live summary frame history.",
);
assert.match(
  appSource,
  /current\.frame_index !== result\.frame_index[\s\S]*current\.state_hash !== result\.frame\.state_hash[\s\S]*current\.live_authority\.session_id !== result\.session_id[\s\S]*current\.live_authority\.session_epoch !== result\.session_epoch[\s\S]*current\.live_authority\.world_revision !== result\.world_revision/,
  "Full-frame hydration should keep frame index, state hash, session id, epoch, and world revision freshness guards together.",
);
assert.match(
  appSource,
  /async function hydrateLiveFrameIfNeeded\(/,
  "Paused live inspection should have an on-demand full frame hydration path.",
);
assert.match(
  appSource,
  /await fetchLiveFrame\(sessionId, frame\.frame_index\)/,
  "On-demand hydration should fetch the authoritative full frame from the live buffer.",
);
assert.doesNotMatch(
  advanceLiveFrameSource,
  /setLiveControlBusy\(/,
  "Automatic live frame advancement should not toggle header controls every frame.",
);
assert.doesNotMatch(
  appSource,
  /window\.setInterval\(\(\) => \{\s*void advanceLiveFrame\(\)/,
  "Live playback should not use a fixed interval that can keep firing while a backend step is still in flight.",
);
assert.match(
  appSource,
  /async function tick\(\)[\s\S]*await advanceLiveFrame\(\)[\s\S]*window\.setTimeout\(tick, nextDelayMs\)/,
  "Live playback should schedule the next step only after the previous backend response has completed.",
);
assert.match(
  appSource,
  /profileMeasure\("live\.cadence"[\s\S]*actualFps[\s\S]*nextDelayMs[\s\S]*degraded/,
  "Adaptive live cadence should be visible in profile output with actual fps and degraded realtime state.",
);
assert.match(
  appSource,
  /LIVE_CADENCE_DEGRADED_STREAK[\s\S]*liveCadenceSlowFrameStreakRef[\s\S]*>= LIVE_CADENCE_DEGRADED_STREAK/,
  "Live cadence warning should require a sustained slow-frame streak instead of flickering on a single frame.",
);
assert.match(
  timelineSource,
  /function LiveCadenceBadge\(/,
  "Timeline chrome should expose actual live cadence so degraded realtime is visible to users.",
);
assert.match(
  appSource,
  /canvasCameraState/,
  "World canvas should keep explicit camera state instead of recomputing an auto-fit camera for every frame.",
);
assert.match(
  appSource,
  /"locked_core"/,
  "World canvas should expose a persistent locked_core camera mode alongside free mode.",
);
assert.match(
  appSource,
  /targetDescription/,
  "Canvas debug view should expose the current camera focus target description for evidence handoff.",
);
assert.match(
  appSource,
  /targetBounds/,
  "Canvas debug view should expose the current camera focus target bounds for evidence handoff.",
);
assert.match(
  appSource,
  /const isLockedCore = canvasCameraState\?\.mode === "locked_core"/,
  "World canvas evidence should distinguish an active locked target from the passive core-focus candidate.",
);
assert.match(
  appSource,
  /const targetDescription = isLockedCore \? canvasCameraState\.lockedTargetDescription : null/,
  "Free-mode evidence should not present the passive core candidate as an active lock target.",
);
assert.match(
  appSource,
  /dampingEnabled/,
  "Canvas debug view should expose whether camera damping is currently enabled for evidence handoff.",
);
assert.match(
  appSource,
  /cameraTargetStateRef/,
  "World canvas should keep a separate target camera state so damped zoom can coalesce repeated wheel input.",
);
assert.match(
  appSource,
  /cameraAnimationFrameRef/,
  "World canvas should keep a single requestAnimationFrame handle for damped camera motion cleanup.",
);
assert.match(
  appSource,
  /requestAnimationFrame\(/,
  "World canvas should drive smooth camera damping with requestAnimationFrame instead of discrete wheel jumps.",
);
assert.match(
  appSource,
  /cancelAnimationFrame\(/,
  "World canvas should cancel pending camera animation frames when mode changes or the component unmounts.",
);
assert.match(
  appSource,
  /prefers-reduced-motion: reduce/,
  "World canvas should respect reduced-motion users by checking the prefers-reduced-motion media query.",
);
assert.match(
  appSource,
  /cameraSmoothingDisabled/,
  "World canvas should branch to a reduced-motion path that snaps or minimizes smoothing.",
);
assert.match(
  appSource,
  /scheduleCameraTransition\(/,
  "World canvas should centralize camera target updates so fit/reset/lock can supersede stale wheel animation.",
);
assert.match(
  appSource,
  /readInteractionCameraState\(/,
  "World canvas should derive wheel zoom from the latest interaction camera target so continuous input stays coherent.",
);
assert.match(
  appSource,
  /labels\.lock/,
  "World canvas should expose a lock-core control with label-driven tooltip text.",
);
assert.match(
  appSource,
  /targetActive/,
  "World canvas focus target labels should be injected from localized UI labels.",
);
assert.doesNotMatch(
  appSource,
  /active dynamic bodies \/ contacts|all collider bounds|scene bounds fallback|selected body|selected collider|selected contact|selected joint/,
  "World canvas should not hard-code English focus target labels in rendered evidence.",
);
assert.match(
  appSource,
  /layers\.grid/,
  "World canvas should let users toggle the background grid layer.",
);
assert.match(
  appSource,
  /layers\.rulers/,
  "World canvas should let users toggle coordinate rulers separately from the grid.",
);
assert.match(
  appSource,
  /pickGridSpacing\(camera\.scale\)/,
  "World canvas grid spacing should adapt to zoom instead of staying on a fixed world-unit step.",
);
assert.match(
  appSource,
  /majorEvery/,
  "World canvas grid should preserve readable major lines alongside minor lines.",
);
assert.ok(
  (appSource.match(/drawGridLine\(/g) ?? []).length >= 2,
  "World canvas grid should explicitly draw both vertical and horizontal grid lines.",
);
assert.match(
  appSource,
  /function worldToScreen[\s\S]*y:\s*camera\.origin\.y \+ point\.y \* camera\.scale/,
  "World canvas should render picea's +y-down world coordinates without flipping gravity or velocity upward.",
);
assert.match(
  appSource,
  /function screenToWorld[\s\S]*y:\s*\(point\.y - camera\.origin\.y\) \/ camera\.scale/,
  "World canvas hit testing should invert the same +y-down transform used for rendering.",
);
assert.match(
  appSource,
  /y:\s*focus\.y - before\.y \* nextScale/,
  "World canvas zoom-at-point math should preserve focus under the +y-down transform.",
);
assert.match(
  appSource,
  /screenRectFromAabb/,
  "World canvas AABB drawing should normalize screen rectangles instead of relying on a flipped y axis.",
);
assert.match(
  appSource,
  /<WorldCanvas[\s\S]*frame=\{currentFrame\}/,
  "World canvas should render the current frame fallback so an empty live buffer clears stale canvas pixels.",
);
assert.doesNotMatch(
  worldCanvasSource,
  /buildTrajectoryOverlay\(/,
  "World canvas should consume the App-derived trajectory overlay instead of rebuilding it from the absolute frame index, which desyncs trails after live buffer trimming.",
);
assert.match(
  workbenchLayoutSource,
  /<WorldCanvas[\s\S]*?trajectoryOverlay=\{trajectoryOverlay\}[\s\S]*?\/>/,
  "Workbench layout should pass the shared trajectory overlay down to the world canvas so canvas and timeline render the same derived facts.",
);
assert.match(
  appSource,
  /async function handleGrabEnd\(\) \{\n\s*grabReleaseRequested\.current = true/,
  "Grab end must park a release request before checking the active grab, so a pointer-up that races the create round-trip is not silently dropped.",
);
assert.match(
  appSource,
  /await createGrab\([\s\S]*?if \(grabReleaseRequested\.current\) \{[\s\S]*?releaseGrab\(sessionId, result\.grab\.id\)/,
  "Grab create must honor a release requested while the create round-trip was in flight, so a quick click cannot leave an orphaned spring joint.",
);
assert.match(
  appSource,
  /FrameDiagnostics/,
  "Workbench should expose frame diagnostics from exported report/stats/events facts.",
);
assert.match(
  appSource,
  /fetchPerf/,
  "Workbench should fetch perf.json as artifact evidence instead of asking users to inspect files manually.",
);
assert.match(
  appSource,
  /copyDebugContext/,
  "Workbench should expose a copyable debug context for source/frame/selection/layers/camera evidence.",
);
assert.match(
  appSource,
  /copyTextToClipboard[\s\S]*document\.execCommand\("copy"\)/,
  "Debug context copy should fall back when embedded browser clipboard permissions reject navigator.clipboard.writeText.",
);
assert.match(
  appSource,
  /debugContextText[\s\S]*debug\.contextPreview/,
  "Debug context should remain visible in the evidence panel even when clipboard writes are blocked.",
);
assert.match(
  appSource,
  /const buildDebugContextText = useCallback\(\(\): string => \{[\s\S]*const payload: DebugContextPayload = \{/,
  "Debug context should be rebuilt from current workbench state so the preview stays aligned with source, frame, camera, layers, and perf evidence.",
);
assert.match(
  appSource,
  /function selectTrajectoryContextMarkers\(/,
  "Debug context should choose trajectory markers around the observed frame/window instead of blindly copying late-run markers.",
);
assert.doesNotMatch(
  appSource,
  /trajectoryMarkers\.slice\(-6\)/,
  "Debug context should not copy the last trajectory markers from the whole run when the user is inspecting an earlier frame.",
);
assert.match(
  appSource,
  /liveAuthority:\s*\{[\s\S]*latestFrameIndex[\s\S]*latestStateHash[\s\S]*latestWorldRevision[\s\S]*viewingLatestAuthoritativeFrame/,
  "Live debug context should record latest-frame authority so scrubbed live frames are not confused with perturbable authoritative frames.",
);
assert.match(
  appSource,
  /return JSON\.stringify\(payload, null, 2\)/,
  "Debug context preview should serialize directly from the current derived payload instead of waiting for a stale manual refresh.",
);
assert.match(
  appSource,
  /evidence\.session/,
  "Timeline evidence panel should include session provenance in the compact evidence rows.",
);
assert.match(
  appSource,
  /evidence\.frame/,
  "Timeline evidence panel should include the current frame index in the compact evidence rows.",
);
assert.match(
  appSource,
  /evidence\.target/,
  "Timeline evidence panel should include the current camera target description.",
);
assert.match(
  appSource,
  /evidence\.targetBounds/,
  "Timeline evidence panel should include the current camera target bounds.",
);
assert.match(
  appSource,
  /evidence\.damping/,
  "Timeline evidence panel should include whether damping is enabled.",
);
assert.match(
  appSource,
  /perf\.liveUnavailable/,
  "Evidence labels should distinguish live-session perf unavailability from a missing artifact file.",
);
assert.match(
  appSource,
  /previewVelocityPerturbation/,
  "Live workbench should call the paused-only velocity perturbation preview endpoint.",
);
assert.match(
  appSource,
  /commitVelocityPerturbation/,
  "Live workbench should call the paused-only velocity perturbation commit endpoint.",
);
assert.match(
  appSource,
  /panel\.velocityPerturbation/,
  "Inspector should expose a dedicated velocity perturbation group instead of hiding the control in generic run setup fields.",
);
assert.match(
  appSource,
  /perturbation\.absoluteVelocityHelp/,
  "Velocity perturbation UI should explicitly describe the feature as an absolute velocity edit.",
);
assert.match(
  appSource,
  /perturbation\.runningBlocked/,
  "Velocity perturbation UI should explain why submit is unavailable while the live session is running.",
);
assert.match(
  appSource,
  /perturbation\.missingFrame/,
  "Velocity perturbation UI should surface that created or reset live sessions cannot preview without a current authoritative frame.",
);
assert.match(
  appSource,
  /perturbation\.currentFrameOnly/,
  "Velocity perturbation UI should block preview and submit while the user is scrubbed to a historical frame instead of waiting for a server stale_frame rejection.",
);
assert.match(
  appSource,
  /const latestAuthoritativeLiveFrame =[\s\S]*const isAuthoritativeLiveFrameSelected =[\s\S]*frameIndex === latestAuthoritativeLiveFrame\.frame_index[\s\S]*currentFrame\.state_hash === latestAuthoritativeLiveFrame\.state_hash/,
  "Velocity perturbation gating should derive an authoritative latest-frame guard from the live frame buffer before enabling preview or submit.",
);
assert.match(
  appSource,
  /const perturbationAvailability = useMemo(?:<[^>]+>)?\(\(\) => \{[\s\S]*if \(source !== "live"\)[\s\S]*if \(!sessionId\)[\s\S]*if \(status === "playing" \|\| status === "running"\)[\s\S]*if \(status === "completed"\)[\s\S]*if \(status === "failed"\)[\s\S]*if \(!latestAuthoritativeLiveFrame \|\| worldRevision == null\)[\s\S]*if \(!isAuthoritativeLiveFrameSelected\)[\s\S]*if \(!selectedPerturbationTarget\)[\s\S]*if \(selectedPerturbationTarget\.bodyType !== "dynamic"\)/,
  "Velocity perturbation availability should gate on live-session authority first, then selected dynamic body eligibility.",
);
assert.match(
  appSource,
  /if \(status === "failed"\)[\s\S]*reasonKey: "perturbation\.failedBlocked"/,
  "Failed live sessions should surface a failed/session_failed-specific disabled reason instead of falling through to missing-frame or paused-only copy.",
);
assert.match(
  appSource,
  /function invalidatePerturbationResponses\(\)[\s\S]*function issuePerturbationGuard\(/,
  "Velocity perturbation preview should have its own request guard lifecycle so stale responses cannot write back after session/frame changes.",
);
assert.match(
  appSource,
  /async function handleVelocityPerturbationPreview\(\)[\s\S]*const guard = issuePerturbationGuard\([\s\S]*await previewVelocityPerturbation[\s\S]*if \(shouldIgnorePerturbationResponse\(guard, preview\)\) \{\s*return\s*\}[\s\S]*setPerturbationPreview\(preview\)/,
  "Velocity perturbation preview should ignore late responses from older guards instead of reviving a stale preview.",
);
assert.match(
  appSource,
  /useEffect\(\(\) => \{\s*invalidatePerturbationResponses\(\)[\s\S]*source,[\s\S]*sessionId,[\s\S]*frameIndex,[\s\S]*status,[\s\S]*sessionEpoch,[\s\S]*currentFrame\.state_hash,[\s\S]*selectedPerturbationTarget\?\.bodyHandle,[\s\S]*\]\)/,
  "Velocity perturbation guard tokens should be invalidated whenever the live session/frame/selection context changes.",
);
assert.match(
  appSource,
  /perturbation_provenance/,
  "Inspector should render accepted perturbation provenance from the current frame.",
);
assert.match(
  appSource,
  /action_id[\s\S]*requested_delta[\s\S]*computed_target_velocity[\s\S]*query_sync_status/,
  "Perturbation UI should expose action id, delta, target velocity, and query sync facts.",
);
assert.match(
  appSource,
  /rejection_reason/,
  "Perturbation preview or commit UI should surface server rejection reasons instead of silently failing.",
);
assert.match(
  applyLiveSessionFrameSource,
  /const retainedStart = session\.retained_frame_start \?\? 0[\s\S]*const retainedEnd =[\s\S]*session\.retained_frame_end_exclusive \?\?[\s\S]*Math\.max\(session\.buffered_frame_count, nextFrame\.frame_index \+ 1\)[\s\S]*byFrameIndex\.set\(nextFrame\.frame_index, nextFrame\)[\s\S]*for \(let index = retainedStart; index < retainedEnd; index \+= 1\)/,
  "Accepted live responses should rebuild the retained live frame window from absolute frame numbers instead of using array indexes as frame indexes.",
);
assert.match(
  appSource,
  /async function handleVelocityPerturbationCommit\(\)[\s\S]*await commitVelocityPerturbation[\s\S]*applyLiveSessionFrame\(\{ session: result\.session \}, liveGuard\)/,
  "Accepted perturbation commits should refresh the local latest frame from the server session payload rather than mutating the preview locally.",
);
assert.match(
  perturbationCommitSource,
  /const liveGuard = issueLiveGuard\(sessionId, true\)[\s\S]*const perturbationGuard = issuePerturbationGuard\(/,
  "Perturbation commit should invalidate older live responses while also minting a strict submit-lane guard for the current preview context.",
);
assert.match(
  perturbationCommitSource,
  /if \(shouldIgnorePerturbationCommitResponse\(perturbationGuard, result\)\) \{\s*return\s*\}[\s\S]*applyLiveSessionFrame\(\{ session: result\.session \}, liveGuard\)/,
  "Perturbation commit should reject stale success payloads before they can refresh the live frame buffer.",
);
assert.match(
  timelineSource,
  /frame\?\.kind === "summary" \|\| frame\?\.live_authority\?\.not_hydrated/,
  "Diagnostics panel should explicitly treat live summary frames as not hydrated instead of implying clean diagnostics.",
);
assert.match(
  perturbationCommitSource,
  /catch \(error\) \{\s*if \(isCurrentPerturbationGuard\(perturbationGuard\)\) \{\s*setPerturbationRequestError\(messageOf\(error\)\)\s*\}\s*\}\s*finally \{\s*if \(isCurrentPerturbationGuard\(perturbationGuard\)\) \{\s*setPerturbationBusy\("idle"\)\s*\}\s*\}/,
  "Perturbation commit catch/finally should stay behind the submit-lane guard so a late response cannot revive requestError or reset busy back to idle.",
);

// --- Select-to-run contract: selecting a scenario runs real physics ---
assert.match(
  appSource,
  /const \[runMode, setRunMode\] = useState<RunMode>\(\s*"live_session",?\s*\)/,
  "Workbench should default to live_session so the first scenario selection is interactive real physics.",
);
const changeScenarioSource =
  appSource.match(/function changeScenario\(nextScenario: string\) \{[\s\S]*?\n  \}/)?.[0] ?? "";
assert.doesNotMatch(
  changeScenarioSource,
  /makeDemoFrames/,
  "Scenario selection must not seed the canvas with fabricated demo frames; demo is an explicit offline fallback only.",
);
assert.match(
  changeScenarioSource,
  /void runScenario\(nextScenario\)/,
  "Selecting a scenario must trigger a real backend run (select-to-run).",
);
assert.match(
  appSource,
  /async function runScenario\(scenarioIdOverride\?: string\)/,
  "runScenario must accept a scenario override so changeScenario can run the newly selected scenario without stale state.",
);
assert.match(
  appSource,
  /initialRunTriggeredRef\.current = true[\s\S]*?void runScenario\(\)/,
  "First successful scenario fetch should auto-run once so the initial canvas shows real physics, not demo frames.",
);

// --- Offline demo watermark contract ---
assert.match(
  appSource,
  /offlineWatermark=\{source === "demo" \? t\(locale, "canvas\.offlineWatermark"\) : null\}/,
  "Demo-sourced frames must surface an explicit offline watermark instead of silently posing as real simulation.",
);
assert.match(
  appSource,
  /offlineWatermark: string \| null/,
  "WorldCanvas should accept the watermark as a nullable string prop so it stays locale-agnostic.",
);
assert.match(
  appSource,
  /offlineWatermark \?[\s\S]*?pointer-events-none[\s\S]*?\{offlineWatermark\}/,
  "The watermark overlay must render as a non-interactive overlay showing the provided label.",
);

// --- Live grab interaction contract ---
const canvasSource = fs.readFileSync(
  new URL("../src/components/workbench/WorldCanvas.tsx", import.meta.url),
  "utf8",
);
assert.match(
  canvasSource,
  /function handlePointerDown[\s\S]*?grabEnabled[\s\S]*?hitTest/,
  "Pointer-down must attempt a body grab before falling back to camera panning when grabbing is enabled.",
);
assert.match(
  canvasSource,
  /body_type === "dynamic"/,
  "Only dynamic bodies are grabbable from the canvas; static hits fall through to panning.",
);
assert.match(
  canvasSource,
  /onGrabMove\(/,
  "Pointer moves while grabbing must forward the world-space target upward.",
);
assert.match(
  canvasSource,
  /onGrabEnd\(\)/,
  "Pointer up/cancel while grabbing must release the grab.",
);
assert.match(
  appSource,
  /grabEnabled=\{source === "live" && sessionId != null\}/,
  "Grabbing is gated to live sessions only; artifact/demo sources stay pan-only.",
);
assert.match(
  appSource,
  /async function handleGrabStart[\s\S]*?if \(status === "paused"\)[\s\S]*?handleControl\("play"\)/,
  "Grabbing a paused live session must auto-resume playing so the drag has physical feedback.",
);
assert.match(
  appSource,
  /grabMoveInFlight[\s\S]*pendingGrabTarget/,
  "Grab target updates must coalesce to the latest pointer position with a single in-flight request.",
);

// --- Grab settings contract ---
assert.match(
  appSource,
  /const \[grabSettings, setGrabSettings\] = useState<GrabSettings>/,
  "Grab mode and spring strength must be user-adjustable state, not hardcoded constants.",
);
assert.match(
  runSettingsSource,
  /"run\.grabMode"/,
  "Run settings should expose the grab mode selector with localized labels.",
);
assert.match(
  runSettingsSource,
  /min=\{2\}[\s\S]{0,220}max=\{420\}/,
  "Grab spring strength slider should span the scene-calibrated stiffness range (2..420).",
);
