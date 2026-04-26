import { useEffect, useMemo, useRef, useState, type ReactNode } from "react";
import * as DropdownMenu from "@radix-ui/react-dropdown-menu";
import * as Tabs from "@radix-ui/react-tabs";
import {
  Activity,
  Box,
  Braces,
  ChevronRight,
  CircleDot,
  Gauge,
  Languages,
  Layers,
  MousePointer2,
  Pause,
  Play,
  RotateCcw,
  Settings2,
  SkipForward,
  Square,
  Waypoints,
} from "lucide-react";
import { Panel, PanelGroup, PanelResizeHandle } from "react-resizable-panels";
import { controlSession, createSession, fetchFrames, fetchScenarios, openSessionEvents } from "./api";
import { WorldCanvas } from "./components/workbench/WorldCanvas";
import { Badge } from "./components/ui/badge";
import { Button } from "./components/ui/button";
import { Input } from "./components/ui/input";
import { PanelHeader, PanelTitle } from "./components/ui/panel";
import { Checkbox, Select, Slider, Tooltip } from "./components/ui/radix";
import { demoScenarios, makeDemoFrames } from "./demo";
import {
  actionLabel,
  bodyTypeLabel,
  booleanLabel,
  detectInitialLocale,
  dynamicValueLabel,
  entityKindLabel,
  entityLabel,
  layerLabel,
  localeLabels,
  localizeScenario,
  sourceLabel,
  statusLabel,
  storeLocale,
  supportedLocales,
  t,
  type LayerKey,
  type Locale,
  type StatusKind,
} from "./i18n";
import type { DebugBody, DebugCollider, FrameRecord, ScenarioDescriptor, SelectedEntity, WorkbenchLog } from "./types";
import { cn } from "./lib/utils";

function warmStartTriplet(stats: FrameRecord["snapshot"]["stats"]) {
  return `${stats.warm_start_hit_count ?? 0}/${stats.warm_start_miss_count ?? 0}/${stats.warm_start_drop_count ?? 0}`;
}

type LayerState = {
  shapes: boolean;
  aabbs: boolean;
  contacts: boolean;
  velocities: boolean;
  trace: boolean;
};

const defaultLayers: LayerState = {
  shapes: true,
  aabbs: true,
  contacts: true,
  velocities: true,
  trace: true,
};

export function App() {
  const [locale, setLocale] = useState<Locale>(() => detectInitialLocale());
  const [scenarios, setScenarios] = useState<ScenarioDescriptor[]>(demoScenarios);
  const [selectedScenario, setSelectedScenario] = useState("falling_box_contact");
  const [frameCount, setFrameCount] = useState(120);
  const [frames, setFrames] = useState<FrameRecord[]>(() => makeDemoFrames("falling_box_contact", 120));
  const [frameIndex, setFrameIndex] = useState(0);
  const [selectedEntity, setSelectedEntity] = useState<SelectedEntity | null>({ kind: "collider", id: 2 });
  const [sessionId, setSessionId] = useState<string | null>(null);
  const [runId, setRunId] = useState<string | null>(null);
  const [source, setSource] = useState<"server" | "demo">("demo");
  const [status, setStatus] = useState<StatusKind>("idle");
  const [logs, setLogs] = useState<WorkbenchLog[]>([
    log("warn", t(locale, "log.serverNotConfirmed")),
  ]);
  const [layers, setLayers] = useState<LayerState>(defaultLayers);
  const [useCustomGravity, setUseCustomGravity] = useState(false);
  const [gravityY, setGravityY] = useState(9.8);
  const playTimer = useRef<number | null>(null);

  const currentFrame = frames[Math.min(frameIndex, Math.max(0, frames.length - 1))];
  const scenario = localizeScenario(locale, scenarios.find((entry) => entry.id === selectedScenario) ?? scenarios[0]);

  useEffect(() => {
    document.documentElement.lang = locale;
    storeLocale(locale);
  }, [locale]);

  useEffect(() => {
    let cancelled = false;
    fetchScenarios()
      .then((next) => {
        if (cancelled) {
          return;
        }
        setScenarios(next);
        setSource("server");
        setLogs((prev) => [log("info", t(locale, "log.connectedScenarios")), ...prev].slice(0, 80));
      })
      .catch((error: Error) => {
        if (cancelled) {
          return;
        }
        setSource("demo");
        setLogs((prev) => [log("warn", t(locale, "log.serverUnavailable", { message: error.message })), ...prev].slice(0, 80));
      });
    return () => {
      cancelled = true;
    };
  }, [locale]);

  useEffect(() => {
    if (status !== "playing") {
      if (playTimer.current !== null) {
        window.clearInterval(playTimer.current);
        playTimer.current = null;
      }
      return;
    }

    playTimer.current = window.setInterval(() => {
      setFrameIndex((next) => {
        if (next >= frames.length - 1) {
          setStatus("paused");
          return next;
        }
        return next + 1;
      });
    }, 1000 / 30);

    return () => {
      if (playTimer.current !== null) {
        window.clearInterval(playTimer.current);
        playTimer.current = null;
      }
    };
  }, [frames.length, status]);

  const selectedDetails = useMemo(
    () => resolveSelection(currentFrame, selectedEntity),
    [currentFrame, selectedEntity],
  );

  async function runScenario() {
    setStatus("loading");
    setFrameIndex(0);
    setSelectedEntity(null);
    const gravity = useCustomGravity ? ([0, gravityY] as [number, number]) : null;

    try {
      const session = await createSession(selectedScenario, frameCount, gravity);
      if (!session.run_id) {
        throw new Error(t(locale, "error.sessionWithoutRun"));
      }
      const completedRunId = session.run_id;
      const nextFrames = await fetchFrames(completedRunId);
      if (nextFrames.length === 0) {
        throw new Error(t(locale, "error.emptyFrames"));
      }
      setSessionId(session.id);
      setRunId(completedRunId);
      setFrames(nextFrames);
      setSource("server");
      setStatus("paused");
      setLogs((prev) =>
        [
          log("info", t(locale, "log.loadedFrames", { count: nextFrames.length, runId: completedRunId })),
          log("info", t(locale, "log.sessionStatus", { sessionId: session.id, status: statusLabel(locale, session.status) })),
          ...prev,
        ].slice(0, 80),
      );
      subscribeToEvents(session.id);
    } catch (error) {
      const nextFrames = makeDemoFrames(selectedScenario, frameCount);
      setFrames(nextFrames);
      setSessionId(null);
      setRunId(null);
      setSource("demo");
      setStatus("paused");
      setLogs((prev) =>
        [
          log("warn", t(locale, "log.serverRunFailed", { message: messageOf(error) })),
          log("info", t(locale, "log.generatedDemoFrames", { count: nextFrames.length, scenarioId: selectedScenario })),
          ...prev,
        ].slice(0, 80),
      );
    }
  }

  function subscribeToEvents(nextSessionId: string) {
    try {
      const events = openSessionEvents(nextSessionId);
      events.addEventListener("frame", (event) => {
        setLogs((prev) => [log("info", t(locale, "log.sseFrame", { data: event.data })), ...prev].slice(0, 80));
      });
      events.addEventListener("failed", (event) => {
        setLogs((prev) => [log("error", t(locale, "log.sseFailed", { data: event.data })), ...prev].slice(0, 80));
      });
      window.setTimeout(() => events.close(), 2000);
    } catch (error) {
      setLogs((prev) => [log("warn", t(locale, "log.sseUnavailable", { message: messageOf(error) })), ...prev].slice(0, 80));
    }
  }

  function changeScenario(nextScenario: string) {
    setSelectedScenario(nextScenario);
    const nextFrames = source === "demo" ? makeDemoFrames(nextScenario, frameCount) : frames;
    setFrames(nextFrames);
    setFrameIndex(0);
    setSelectedEntity(null);
  }

  async function handleControl(action: "play" | "pause" | "step" | "reset") {
    if (action === "pause") {
      setStatus("paused");
    } else if (action === "play") {
      setStatus("playing");
    } else if (action === "step") {
      setStatus("paused");
      setFrameIndex((value) => Math.min(frames.length - 1, value + 1));
    } else {
      setStatus("paused");
      setFrameIndex(0);
    }

    if (source !== "server" || !sessionId) {
      return;
    }

    try {
      const session = await controlSession(sessionId, action);
      setLogs((prev) =>
        [
          log("info", t(locale, "log.serverAccepted", { action: actionLabel(locale, action), status: statusLabel(locale, session.status) })),
          ...prev,
        ].slice(0, 80),
      );
      setFrameIndex(Math.min(session.current_frame_index, Math.max(0, frames.length - 1)));
      if (session.run_id && action === "reset") {
        const nextFrames = await fetchFrames(session.run_id);
        if (nextFrames.length > 0) {
          setFrames(nextFrames);
          setRunId(session.run_id);
          setFrameIndex(session.current_frame_index);
        }
      }
    } catch (error) {
      setLogs((prev) =>
        [log("warn", t(locale, "log.serverControlFailed", { action: actionLabel(locale, action), message: messageOf(error) })), ...prev].slice(0, 80),
      );
    }
  }

  function updateLayer(key: keyof LayerState, value: boolean) {
    setLayers((prev) => ({ ...prev, [key]: value }));
  }

  return (
    <div className="flex h-screen min-h-[720px] flex-col overflow-hidden bg-lab-canvas text-lab-text">
      <Toolbar
        locale={locale}
        onLocaleChange={setLocale}
        scenario={scenario}
        scenarios={scenarios}
        selectedScenario={selectedScenario}
        onScenarioChange={changeScenario}
        status={status}
        source={source}
        sessionId={sessionId}
        runId={runId}
        onRun={runScenario}
        onPlay={() => void handleControl("play")}
        onPause={() => void handleControl("pause")}
        onStep={() => void handleControl("step")}
        onReset={() => void handleControl("reset")}
        layers={layers}
        onLayerChange={updateLayer}
      />

      <PanelGroup direction="horizontal" className="min-h-0 flex-1">
        <Panel defaultSize={20} minSize={16} maxSize={30} className="min-w-[240px] border-r border-lab-line bg-lab-panel">
          <SceneHierarchy frame={currentFrame} selected={selectedEntity} onSelect={setSelectedEntity} locale={locale} />
        </Panel>
        <ResizeHandle />
        <Panel defaultSize={56} minSize={35} className="min-w-[420px]">
          <PanelGroup direction="vertical">
            <Panel defaultSize={72} minSize={45} className="min-h-[320px]">
              <WorldCanvas
                frames={frames}
                frameIndex={frameIndex}
                selected={selectedEntity}
                layers={layers}
                labels={{
                  frame: t(locale, "canvas.frame"),
                  colliders: t(locale, "canvas.colliders"),
                  contacts: t(locale, "canvas.contacts"),
                }}
                onSelect={setSelectedEntity}
              />
            </Panel>
            <ResizeHandle vertical />
            <Panel defaultSize={28} minSize={18} className="min-h-[180px] border-t border-lab-line bg-lab-panel">
              <BottomTimeline
                frames={frames}
                frameIndex={frameIndex}
                onFrameChange={setFrameIndex}
                logs={logs}
                frameCount={frameCount}
                setFrameCount={setFrameCount}
                useCustomGravity={useCustomGravity}
                setUseCustomGravity={setUseCustomGravity}
                gravityY={gravityY}
                setGravityY={setGravityY}
                locale={locale}
              />
            </Panel>
          </PanelGroup>
        </Panel>
        <ResizeHandle />
        <Panel defaultSize={24} minSize={18} maxSize={34} className="min-w-[280px] border-l border-lab-line bg-lab-panel">
          <Inspector frame={currentFrame} selected={selectedDetails} locale={locale} />
        </Panel>
      </PanelGroup>
    </div>
  );
}

function Toolbar({
  locale,
  onLocaleChange,
  scenario,
  scenarios,
  selectedScenario,
  onScenarioChange,
  status,
  source,
  sessionId,
  runId,
  onRun,
  onPlay,
  onPause,
  onStep,
  onReset,
  layers,
  onLayerChange,
}: {
  locale: Locale;
  onLocaleChange: (locale: Locale) => void;
  scenario: ScenarioDescriptor;
  scenarios: ScenarioDescriptor[];
  selectedScenario: string;
  onScenarioChange: (value: string) => void;
  status: StatusKind;
  source: "server" | "demo";
  sessionId: string | null;
  runId: string | null;
  onRun: () => void;
  onPlay: () => void;
  onPause: () => void;
  onStep: () => void;
  onReset: () => void;
  layers: LayerState;
  onLayerChange: (key: keyof LayerState, value: boolean) => void;
}) {
  return (
    <header className="flex h-12 shrink-0 items-center gap-2 border-b border-lab-line bg-lab-panel px-3">
      <div className="flex min-w-0 flex-1 items-center gap-3">
        <div className="flex items-center gap-2">
          <Activity className="h-5 w-5 text-lab-accent" />
          <div className="leading-tight">
            <div className="text-sm font-semibold text-lab-text">{t(locale, "app.name")}</div>
            <div className="truncate text-[11px] text-lab-muted">{scenario.description}</div>
          </div>
        </div>
        <Select
          value={selectedScenario}
          onValueChange={onScenarioChange}
          items={scenarios.map((entry) => ({ value: entry.id, label: localizeScenario(locale, entry).name }))}
          className="ml-2 w-52"
        />
        <Badge tone={source === "server" ? "green" : "warn"}>{sourceLabel(locale, source)}</Badge>
        <Badge tone={status === "failed" ? "danger" : status === "playing" ? "accent" : "neutral"}>{statusLabel(locale, status)}</Badge>
      </div>

      <div className="flex items-center gap-1">
        <Tooltip label={t(locale, "tooltip.runScenario")}>
          <Button size="icon" onClick={onRun} disabled={status === "loading"}>
            <Play className="h-4 w-4" />
          </Button>
        </Tooltip>
        <Tooltip label={t(locale, "tooltip.pausePlayback")}>
          <Button size="icon" variant="outline" onClick={onPause}>
            <Pause className="h-4 w-4" />
          </Button>
        </Tooltip>
        <Tooltip label={t(locale, "tooltip.playTimeline")}>
          <Button size="icon" variant="outline" onClick={onPlay}>
            <Gauge className="h-4 w-4" />
          </Button>
        </Tooltip>
        <Tooltip label={t(locale, "tooltip.advanceFrame")}>
          <Button size="icon" variant="outline" onClick={onStep}>
            <SkipForward className="h-4 w-4" />
          </Button>
        </Tooltip>
        <Tooltip label={t(locale, "tooltip.resetTimeline")}>
          <Button size="icon" variant="outline" onClick={onReset}>
            <RotateCcw className="h-4 w-4" />
          </Button>
        </Tooltip>
        <LayerMenu locale={locale} layers={layers} onLayerChange={onLayerChange} />
        <Tooltip label={t(locale, "app.language")}>
          <div className="ml-1 flex items-center gap-1">
            <Languages className="h-4 w-4 text-lab-muted" />
            <Select
              value={locale}
              onValueChange={(value) => onLocaleChange(value as Locale)}
              items={supportedLocales.map((entry) => ({ value: entry, label: localeLabels[entry] }))}
              ariaLabel={t(locale, "app.language")}
              className="w-24"
            />
          </div>
        </Tooltip>
      </div>

      <div className="hidden min-w-[220px] flex-col text-right text-[11px] text-lab-muted xl:flex">
        <span>{sessionId ?? t(locale, "app.noSession")}</span>
        <span>{runId ?? t(locale, "app.noRunArtifact")}</span>
      </div>
    </header>
  );
}

function LayerMenu({
  locale,
  layers,
  onLayerChange,
}: {
  locale: Locale;
  layers: LayerState;
  onLayerChange: (key: keyof LayerState, value: boolean) => void;
}) {
  return (
    <DropdownMenu.Root>
      <Tooltip label={t(locale, "tooltip.canvasLayers")}>
        <DropdownMenu.Trigger asChild>
          <Button size="icon" variant="outline">
            <Layers className="h-4 w-4" />
          </Button>
        </DropdownMenu.Trigger>
      </Tooltip>
      <DropdownMenu.Portal>
        <DropdownMenu.Content
          align="end"
          sideOffset={8}
          className="z-50 w-52 rounded-md border border-lab-line bg-lab-panel2 p-2 text-sm text-lab-text shadow-xl"
        >
          {(Object.keys(layers) as Array<LayerKey>).map((key) => (
            <DropdownMenu.CheckboxItem
              key={key}
              checked={layers[key]}
              onCheckedChange={(value) => onLayerChange(key, value)}
              className="flex h-7 cursor-default select-none items-center gap-2 rounded px-2 outline-none data-[highlighted]:bg-lab-accent/[0.18]"
            >
              <DropdownMenu.ItemIndicator>
                <ChevronRight className="h-3.5 w-3.5 rotate-90 text-lab-accent" />
              </DropdownMenu.ItemIndicator>
              <span>{layerLabel(locale, key)}</span>
            </DropdownMenu.CheckboxItem>
          ))}
        </DropdownMenu.Content>
      </DropdownMenu.Portal>
    </DropdownMenu.Root>
  );
}

function SceneHierarchy({
  frame,
  selected,
  onSelect,
  locale,
}: {
  frame: FrameRecord;
  selected: SelectedEntity | null;
  onSelect: (entity: SelectedEntity) => void;
  locale: Locale;
}) {
  return (
    <div className="flex h-full min-h-0 flex-col">
      <PanelHeader>
        <PanelTitle>{t(locale, "panel.sceneHierarchy")}</PanelTitle>
        <Badge>{frame.snapshot.stats.step_index}</Badge>
      </PanelHeader>
      <div className="min-h-0 flex-1 overflow-auto p-2">
        <TreeGroup icon={<Box className="h-4 w-4" />} label={t(locale, "tree.bodies")} count={frame.snapshot.bodies.length} locale={locale}>
          {frame.snapshot.bodies.map((body) => (
            <TreeRow
              key={body.handle}
              active={selected?.kind === "body" && selected.id === body.handle}
              icon={<Square className="h-3.5 w-3.5" />}
              label={entityLabel(locale, "body", body.handle)}
              meta={bodyTypeLabel(locale, body.body_type)}
              onClick={() => onSelect({ kind: "body", id: body.handle })}
            />
          ))}
        </TreeGroup>
        <TreeGroup icon={<Layers className="h-4 w-4" />} label={t(locale, "tree.colliders")} count={frame.snapshot.colliders.length} locale={locale}>
          {frame.snapshot.colliders.map((collider) => (
            <TreeRow
              key={collider.handle}
              active={selected?.kind === "collider" && selected.id === collider.handle}
              icon={<CircleDot className="h-3.5 w-3.5" />}
              label={entityLabel(locale, "collider", collider.handle)}
              meta={entityLabel(locale, "body", collider.body)}
              onClick={() => onSelect({ kind: "collider", id: collider.handle })}
            />
          ))}
        </TreeGroup>
        <TreeGroup icon={<Waypoints className="h-4 w-4" />} label={t(locale, "tree.contacts")} count={frame.snapshot.contacts.length} locale={locale}>
          {frame.snapshot.contacts.map((contact) => (
            <TreeRow
              key={contact.id}
              active={selected?.kind === "contact" && selected.id === contact.id}
              icon={<MousePointer2 className="h-3.5 w-3.5" />}
              label={entityLabel(locale, "contact", contact.id)}
              meta={`${t(locale, "fact.depth")} ${contact.depth.toFixed(3)}`}
              onClick={() => onSelect({ kind: "contact", id: contact.id })}
            />
          ))}
        </TreeGroup>
      </div>
    </div>
  );
}

function TreeGroup({
  icon,
  label,
  count,
  children,
  locale,
}: {
  icon: ReactNode;
  label: string;
  count: number;
  children: ReactNode;
  locale: Locale;
}) {
  return (
    <section className="mb-3">
      <div className="mb-1 flex h-7 items-center gap-2 rounded px-1.5 text-xs font-semibold uppercase tracking-normal text-lab-muted">
        {icon}
        <span className="flex-1">{label}</span>
        <span>{count}</span>
      </div>
      <div className="space-y-0.5">{count > 0 ? children : <div className="px-8 py-1 text-xs text-lab-muted">{t(locale, "tree.empty")}</div>}</div>
    </section>
  );
}

function TreeRow({
  active,
  icon,
  label,
  meta,
  onClick,
}: {
  active: boolean;
  icon: ReactNode;
  label: string;
  meta: string;
  onClick: () => void;
}) {
  return (
    <button
      type="button"
      onClick={onClick}
      className={cn(
        "flex h-8 w-full items-center gap-2 rounded px-2 text-left text-sm transition-colors",
        active ? "bg-lab-accent/[0.16] text-lab-text" : "text-lab-muted hover:bg-white/[0.06] hover:text-lab-text",
      )}
    >
      {icon}
      <span className="min-w-0 flex-1 truncate">{label}</span>
      <span className="truncate text-[11px]">{meta}</span>
    </button>
  );
}

function Inspector({
  frame,
  selected,
  locale,
}: {
  frame: FrameRecord;
  locale: Locale;
  selected:
    | { kind: "body"; entity: DebugBody }
    | { kind: "collider"; entity: DebugCollider; body?: DebugBody }
    | { kind: "contact"; entity: FrameRecord["snapshot"]["contacts"][number] }
    | { kind: "joint"; entity: FrameRecord["snapshot"]["joints"][number] }
    | null;
}) {
  return (
    <div className="flex h-full min-h-0 flex-col">
      <PanelHeader>
        <PanelTitle>{t(locale, "panel.inspector")}</PanelTitle>
        <Badge tone="warn">{t(locale, "panel.firstSliceFacts")}</Badge>
      </PanelHeader>
      <div className="min-h-0 flex-1 overflow-auto p-3">
        <div className="mb-3 grid grid-cols-3 gap-2">
          <Metric label={t(locale, "metric.bodies")} value={frame.snapshot.bodies.length} />
          <Metric label={t(locale, "metric.contacts")} value={frame.snapshot.contacts.length} />
          <Metric label={t(locale, "metric.dt")} value={frame.snapshot.meta.dt.toFixed(4)} />
        </div>

        <section className="mb-4 rounded-md border border-lab-line bg-lab-panel2 p-3">
          <div className="mb-2 flex items-center justify-between">
            <h3 className="text-xs font-semibold uppercase tracking-normal text-lab-muted">{t(locale, "inspector.measurementStatus")}</h3>
            <Badge tone="warn">{t(locale, "inspector.unmeasured")}</Badge>
          </div>
          <Fact label={t(locale, "inspector.forces")} value={t(locale, "inspector.unmeasured")} muted />
          <Fact label={t(locale, "inspector.torques")} value={t(locale, "inspector.unmeasured")} muted />
          <Fact label={t(locale, "inspector.broadphaseCandidates")} value={String(frame.snapshot.stats.broadphase_candidate_count)} />
          <Fact label={t(locale, "inspector.warmStart")} value={warmStartTriplet(frame.snapshot.stats)} />
        </section>

        {selected ? (
          <EntityInspector selected={selected} locale={locale} />
        ) : (
          <div className="rounded-md border border-lab-line bg-lab-panel2 p-4 text-sm text-lab-muted">
            {t(locale, "inspector.emptySelection")}
          </div>
        )}
      </div>
    </div>
  );
}

function EntityInspector({
  selected,
  locale,
}: {
  locale: Locale;
  selected:
    | { kind: "body"; entity: DebugBody }
    | { kind: "collider"; entity: DebugCollider; body?: DebugBody }
    | { kind: "contact"; entity: FrameRecord["snapshot"]["contacts"][number] }
    | { kind: "joint"; entity: FrameRecord["snapshot"]["joints"][number] };
}) {
  const title = entityLabel(locale, selected.kind, entityId(selected));
  return (
    <section className="rounded-md border border-lab-line bg-lab-panel2 p-3">
      <div className="mb-3 flex items-center justify-between">
        <h3 className="text-sm font-semibold text-lab-text">{title}</h3>
        <Badge tone="accent">{entityKindLabel(locale, selected.kind)}</Badge>
      </div>
      {selected.kind === "body" && (
        <>
          <Fact label={t(locale, "fact.type")} value={bodyTypeLabel(locale, selected.entity.body_type)} />
          <Fact label={t(locale, "fact.position")} value={vec(selected.entity.transform.translation)} />
          <Fact label={t(locale, "fact.mass")} value={selected.entity.mass_properties.mass.toFixed(3)} />
          <Fact label={t(locale, "fact.inverseMass")} value={selected.entity.mass_properties.inverse_mass.toFixed(3)} />
          <Fact label={t(locale, "fact.centerOfMass")} value={vec(selected.entity.mass_properties.local_center_of_mass)} />
          <Fact label={t(locale, "fact.inertia")} value={selected.entity.mass_properties.inertia.toFixed(3)} />
          <Fact label={t(locale, "fact.inverseInertia")} value={selected.entity.mass_properties.inverse_inertia.toFixed(3)} />
          <Fact label={t(locale, "fact.linearVelocity")} value={vec(selected.entity.linear_velocity)} />
          <Fact label={t(locale, "fact.angularVelocity")} value={selected.entity.angular_velocity.toFixed(3)} />
          <Fact label={t(locale, "fact.sleeping")} value={booleanLabel(locale, selected.entity.sleeping)} />
        </>
      )}
      {selected.kind === "collider" && (
        <>
          <Fact label={t(locale, "fact.body")} value={String(selected.entity.body)} />
          <Fact label={t(locale, "fact.shape")} value={dynamicValueLabel(locale, selected.entity.shape.kind)} />
          <Fact label={t(locale, "fact.center")} value={vec(selected.entity.world_transform.translation)} />
          <Fact label={t(locale, "fact.friction")} value={selected.entity.material.friction.toFixed(3)} />
          <Fact label={t(locale, "fact.restitution")} value={selected.entity.material.restitution.toFixed(3)} />
          <Fact label={t(locale, "fact.sensor")} value={booleanLabel(locale, selected.entity.is_sensor)} />
          <Fact label={t(locale, "fact.ownerVelocity")} value={selected.body ? vec(selected.body.linear_velocity) : t(locale, "common.unknown")} />
        </>
      )}
      {selected.kind === "contact" && (
        <>
          <Fact label={t(locale, "fact.point")} value={vec(selected.entity.point)} />
          <Fact label={t(locale, "fact.normal")} value={vec(selected.entity.normal)} />
          <Fact label={t(locale, "fact.depth")} value={selected.entity.depth.toFixed(4)} />
          <Fact label={t(locale, "fact.feature")} value={String(selected.entity.feature_id)} />
          <Fact label={t(locale, "fact.reduction")} value={dynamicValueLabel(locale, selected.entity.reduction_reason)} />
          <Fact label={t(locale, "inspector.warmStart")} value={dynamicValueLabel(locale, selected.entity.warm_start_reason ?? "miss_no_previous")} />
          <Fact label={t(locale, "fact.warmStartNormal")} value={selected.entity.normal_impulse.toFixed(4)} />
          <Fact label={t(locale, "fact.warmStartTangent")} value={selected.entity.tangent_impulse.toFixed(4)} />
          <Fact label={t(locale, "fact.solverNormal")} value={(selected.entity.solver_normal_impulse ?? 0).toFixed(4)} />
          <Fact label={t(locale, "fact.solverTangent")} value={(selected.entity.solver_tangent_impulse ?? 0).toFixed(4)} />
          <Fact label={t(locale, "fact.normalClamped")} value={booleanLabel(locale, selected.entity.normal_impulse_clamped ?? false)} />
          <Fact label={t(locale, "fact.tangentClamped")} value={booleanLabel(locale, selected.entity.tangent_impulse_clamped ?? false)} />
          <Fact label={t(locale, "fact.restitution")} value={selected.entity.restitution_applied ? t(locale, "contact.applied") : t(locale, "contact.suppressed")} />
        </>
      )}
      {selected.kind === "joint" && (
        <>
          <Fact label={t(locale, "fact.kind")} value={dynamicValueLabel(locale, selected.entity.kind)} />
          <Fact label={t(locale, "tree.bodies")} value={selected.entity.bodies.join(", ")} />
          <Fact label={t(locale, "fact.anchors")} value={selected.entity.anchors.map(vec).join(" -> ")} />
        </>
      )}
    </section>
  );
}

function entityId(
  selected:
    | { kind: "body"; entity: DebugBody }
    | { kind: "collider"; entity: DebugCollider; body?: DebugBody }
    | { kind: "contact"; entity: FrameRecord["snapshot"]["contacts"][number] }
    | { kind: "joint"; entity: FrameRecord["snapshot"]["joints"][number] },
): number {
  return selected.kind === "contact" ? selected.entity.id : selected.entity.handle;
}

function BottomTimeline({
  frames,
  frameIndex,
  onFrameChange,
  logs,
  frameCount,
  setFrameCount,
  useCustomGravity,
  setUseCustomGravity,
  gravityY,
  setGravityY,
  locale,
}: {
  frames: FrameRecord[];
  frameIndex: number;
  onFrameChange: (value: number) => void;
  logs: WorkbenchLog[];
  frameCount: number;
  setFrameCount: (value: number) => void;
  useCustomGravity: boolean;
  setUseCustomGravity: (value: boolean) => void;
  gravityY: number;
  setGravityY: (value: number) => void;
  locale: Locale;
}) {
  const frame = frames[Math.min(frameIndex, Math.max(0, frames.length - 1))];
  return (
    <Tabs.Root defaultValue="timeline" className="flex h-full min-h-0 flex-col">
      <PanelHeader>
        <Tabs.List className="flex items-center gap-1">
          <Tabs.Trigger value="timeline" className="tab-trigger">
            {t(locale, "timeline.timeline")}
          </Tabs.Trigger>
          <Tabs.Trigger value="logs" className="tab-trigger">
            {t(locale, "timeline.logs")}
          </Tabs.Trigger>
          <Tabs.Trigger value="run" className="tab-trigger">
            {t(locale, "timeline.runSetup")}
          </Tabs.Trigger>
        </Tabs.List>
        <div className="flex items-center gap-2 text-xs text-lab-muted">
          <span>{frame?.simulated_time.toFixed(3)}s</span>
          <span>{frame?.state_hash}</span>
        </div>
      </PanelHeader>

      <Tabs.Content value="timeline" className="min-h-0 flex-1 p-3 outline-none">
        <div className="mb-3 flex items-center gap-3">
          <span className="w-20 text-xs text-lab-muted">{t(locale, "timeline.frameAt", { frame: frameIndex })}</span>
          <Slider value={frameIndex} min={0} max={Math.max(0, frames.length - 1)} step={1} onValueChange={onFrameChange} />
          <span className="w-20 text-right text-xs text-lab-muted">{t(locale, "timeline.totalFrames", { count: frames.length })}</span>
        </div>
        <div className="grid grid-cols-4 gap-2">
          <Metric label={t(locale, "metric.step")} value={frame?.snapshot.stats.step_index ?? 0} />
          <Metric label={t(locale, "metric.simTime")} value={(frame?.snapshot.meta.simulated_time ?? 0).toFixed(3)} />
          <Metric label={t(locale, "metric.gravity")} value={vec(frame?.snapshot.meta.gravity ?? { x: 0, y: 0 })} />
          <Metric label={t(locale, "metric.manifolds")} value={frame?.snapshot.manifolds.length ?? 0} />
        </div>
      </Tabs.Content>

      <Tabs.Content value="logs" className="min-h-0 flex-1 overflow-auto p-2 outline-none">
        <div className="space-y-1 font-mono text-xs">
          {logs.map((entry, index) => (
            <div key={`${entry.time}-${index}`} className="grid grid-cols-[74px_52px_1fr] gap-2 rounded px-2 py-1 hover:bg-white/5">
              <span className="text-lab-muted">{entry.time}</span>
              <span className={entry.level === "error" ? "text-lab-danger" : entry.level === "warn" ? "text-lab-warn" : "text-lab-accent"}>
                {entry.level}
              </span>
              <span className="min-w-0 truncate text-lab-text">{entry.message}</span>
            </div>
          ))}
        </div>
      </Tabs.Content>

      <Tabs.Content value="run" className="min-h-0 flex-1 p-3 outline-none">
        <div className="grid max-w-2xl grid-cols-[140px_1fr] items-center gap-3">
          <label className="text-sm text-lab-muted">{t(locale, "run.frameCount")}</label>
          <Input
            type="number"
            min={1}
            max={2000}
            value={frameCount}
            onChange={(event) => setFrameCount(Math.max(1, Number(event.target.value) || 1))}
          />
          <label className="text-sm text-lab-muted">{t(locale, "run.gravityOverride")}</label>
          <Checkbox checked={useCustomGravity} onCheckedChange={setUseCustomGravity} label={t(locale, "run.sendOverride")} />
          <label className="text-sm text-lab-muted">{t(locale, "run.gravityY")}</label>
          <Input
            type="number"
            step="0.1"
            value={gravityY}
            disabled={!useCustomGravity}
            onChange={(event) => setGravityY(Number(event.target.value) || 0)}
          />
        </div>
      </Tabs.Content>
    </Tabs.Root>
  );
}

function Metric({ label, value }: { label: string; value: string | number }) {
  return (
    <div className="rounded-md border border-lab-line bg-lab-panel2 px-2 py-1.5">
      <div className="truncate text-[11px] uppercase tracking-normal text-lab-muted">{label}</div>
      <div className="truncate font-mono text-sm text-lab-text">{value}</div>
    </div>
  );
}

function Fact({ label, value, muted = false }: { label: string; value: string; muted?: boolean }) {
  return (
    <div className="grid grid-cols-[132px_1fr] gap-2 border-t border-lab-line/70 py-1.5 first:border-t-0">
      <span className="text-xs text-lab-muted">{label}</span>
      <span className={cn("min-w-0 break-words font-mono text-xs", muted ? "text-lab-warn" : "text-lab-text")}>{value}</span>
    </div>
  );
}

function ResizeHandle({ vertical = false }: { vertical?: boolean }) {
  return (
    <PanelResizeHandle
      className={cn(
        "shrink-0 bg-lab-line transition-colors hover:bg-lab-accent/70",
        vertical ? "h-1 w-full" : "h-full w-1",
      )}
    />
  );
}

function resolveSelection(frame: FrameRecord, selected: SelectedEntity | null) {
  if (!selected) {
    return null;
  }
  if (selected.kind === "body") {
    const entity = frame.snapshot.bodies.find((body) => body.handle === selected.id);
    return entity ? { kind: selected.kind, entity } : null;
  }
  if (selected.kind === "collider") {
    const entity = frame.snapshot.colliders.find((collider) => collider.handle === selected.id);
    const body = entity ? frame.snapshot.bodies.find((entry) => entry.handle === entity.body) : undefined;
    return entity ? { kind: selected.kind, entity, body } : null;
  }
  if (selected.kind === "contact") {
    const entity = frame.snapshot.contacts.find((contact) => contact.id === selected.id);
    return entity ? { kind: selected.kind, entity } : null;
  }
  const entity = frame.snapshot.joints.find((joint) => joint.handle === selected.id);
  return entity ? { kind: selected.kind, entity } : null;
}

function vec(value: { x: number; y: number }): string {
  return `${value.x.toFixed(3)}, ${value.y.toFixed(3)}`;
}

function log(level: WorkbenchLog["level"], message: string): WorkbenchLog {
  return {
    time: new Date().toLocaleTimeString("en-US", { hour12: false }),
    level,
    message,
  };
}

function messageOf(error: unknown): string {
  return error instanceof Error ? error.message : String(error);
}
