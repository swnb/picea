import { useEffect, useRef, useState } from "react"
import * as DropdownMenu from "@radix-ui/react-dropdown-menu"
import {
  Activity,
  Braces,
  Check,
  ChevronRight,
  Languages,
  Layers,
  LoaderCircle,
  RotateCcw,
  SlidersHorizontal,
} from "lucide-react"

import { Badge } from "../ui/badge"
import { Button } from "../ui/button"
import { Select, Tooltip } from "../ui/radix"
import { cn } from "../../lib/utils"
import {
  layerLabel,
  localeLabels,
  localizeScenario,
  overlayPresetDescription,
  overlayPresetLabel,
  scenarioGroupForId,
  scenarioGroupLabel,
  scenarioWatchFor,
  sourceLabel,
  statusLabel,
  t,
  type LayerKey,
  type Locale,
  type OverlayPresetId,
  type ScenarioGroupId,
  type StatusKind,
} from "../../i18n"
import type { ScenarioDescriptor } from "../../types"
import type { LayerState, RunMode, SourceKind } from "./types"

export function Toolbar({
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
  manifestArtifact,
  finalSnapshotArtifact,
  finalSnapshotStep,
  onRun,
  runMode,
  onRunModeChange,
  layers,
  onLayerChange,
  onApplyOverlayPreset,
}: {
  locale: Locale
  onLocaleChange: (locale: Locale) => void
  scenario: ScenarioDescriptor
  scenarios: ScenarioDescriptor[]
  selectedScenario: string
  onScenarioChange: (value: string) => void
  status: StatusKind
  source: SourceKind
  sessionId: string | null
  runId: string | null
  manifestArtifact: string | null
  finalSnapshotArtifact: string | null
  finalSnapshotStep: number | null
  onRun: () => void
  runMode: RunMode
  onRunModeChange: (value: RunMode) => void
  layers: LayerState
  onLayerChange: (key: keyof LayerState, value: boolean) => void
  onApplyOverlayPreset: (preset: OverlayPresetId) => void
}) {
  const groupedScenarios = buildScenarioGroups(locale, scenarios)
  const watchFor = scenarioWatchFor(locale, selectedScenario)
  const scenarioTooltip = watchFor
    ? `${scenario.description} ${t(locale, "scenario.watchForLabel")}: ${watchFor}`
    : scenario.description
  const isRunLoading = status === "loading"
  const runProgressLabel =
    runMode === "artifact_replay"
      ? t(locale, "run.generatingArtifact")
      : t(locale, "run.startingLiveSession")
  const runTriggerLabel =
    isRunLoading
      ? runProgressLabel
      : sessionId || runId || source !== "demo" || status !== "idle"
        ? t(locale, "tooltip.rerunScenario")
        : t(locale, "tooltip.startRunScenario")
  return (
    <header className="flex h-12 shrink-0 items-center gap-2 border-b border-lab-line bg-lab-panel px-3">
      <div className="flex min-w-0 flex-1 items-center gap-3">
        <Tooltip label={scenarioTooltip}>
          <div
            aria-label={scenarioTooltip}
            tabIndex={0}
            className="flex w-60 shrink-0 items-center gap-2 rounded-sm focus-visible:outline-none focus-visible:shadow-focus"
          >
            <Activity className="h-5 w-5 shrink-0 text-lab-accent" />
            <div className="min-w-0 leading-tight">
              <div className="truncate text-sm font-semibold text-lab-text">
                {t(locale, "app.name")}
              </div>
              <div className="truncate text-[11px] text-lab-muted">
                {scenario.description}
              </div>
            </div>
          </div>
        </Tooltip>
        <Select
          value={runMode}
          onValueChange={(value) => onRunModeChange(value as RunMode)}
          ariaLabel={t(locale, "run.mode")}
          items={[
            {
              value: "artifact_replay",
              label: t(locale, "run.modeArtifact"),
            },
            {
              value: "live_session",
              label: t(locale, "run.modeLive"),
            },
          ]}
          className="ml-2 w-40"
          disabled={isRunLoading}
        />
        <Select
          value={selectedScenario}
          onValueChange={onScenarioChange}
          ariaLabel={t(locale, "scenario.select")}
          items={groupedScenarios}
          className="w-52"
          disabled={isRunLoading}
        />
        <Badge
          tone={
            source === "live"
              ? "green"
              : source === "artifact"
                ? "accent"
                : "warn"
          }
        >
          {sourceLabel(locale, source)}
        </Badge>
        <Badge
          tone={
            status === "failed"
              ? "danger"
              : isRunLoading || status === "playing"
                ? "accent"
                : "neutral"
          }
        >
          {statusLabel(locale, status)}
        </Badge>
        <div
          role={isRunLoading ? "status" : undefined}
          aria-live="polite"
          aria-hidden={!isRunLoading}
          className={cn(
            "flex w-40 shrink-0 items-center gap-1.5 text-xs text-lab-muted",
            !isRunLoading && "invisible",
          )}
        >
          <LoaderCircle className="h-3.5 w-3.5 shrink-0 animate-spin text-lab-accent" />
          <span className="truncate">{runProgressLabel}</span>
        </div>
      </div>

      <div className="flex items-center gap-1">
        <Tooltip label={runTriggerLabel}>
          <Button
            size="icon"
            onClick={onRun}
            disabled={isRunLoading}
            aria-busy={isRunLoading}
            aria-label={runTriggerLabel}
          >
            {isRunLoading ? (
              <LoaderCircle className="h-4 w-4 animate-spin" />
            ) : (
              <RotateCcw className="h-4 w-4" />
            )}
          </Button>
        </Tooltip>
        <LayerMenu
          locale={locale}
          layers={layers}
          onLayerChange={onLayerChange}
        />
        <OverlayPresetMenu locale={locale} onApplyPreset={onApplyOverlayPreset} />
        <ArtifactMenu
          locale={locale}
          sessionId={sessionId}
          runId={runId}
          manifestArtifact={manifestArtifact}
          finalSnapshotArtifact={finalSnapshotArtifact}
          finalSnapshotStep={finalSnapshotStep}
        />
        <LanguageMenu locale={locale} onLocaleChange={onLocaleChange} />
      </div>
    </header>
  )
}

function buildScenarioGroups(
  locale: Locale,
  scenarios: ScenarioDescriptor[],
): Array<{ label: string; items: Array<{ value: string; label: string }> }> {
  const groupOrder: ScenarioGroupId[] = [
    "basics",
    "stack",
    "ccd",
    "compound",
    "lattice",
    "diagnostics",
  ]
  const grouped = new Map<ScenarioGroupId, Array<{ value: string; label: string }>>()
  for (const scenario of scenarios) {
    const group = scenarioGroupForId(scenario.id)
    const items = grouped.get(group)
    const localized = localizeScenario(locale, scenario)
    const entry = { value: scenario.id, label: localized.name }
    if (items) {
      items.push(entry)
    } else {
      grouped.set(group, [entry])
    }
  }
  return groupOrder
    .map((group) => ({
      label: scenarioGroupLabel(locale, group),
      items: (grouped.get(group) ?? []).sort((left, right) =>
        left.label.localeCompare(right.label, locale),
      ),
    }))
    .filter((group) => group.items.length > 0)
}

function ArtifactMenu({
  locale,
  sessionId,
  runId,
  manifestArtifact,
  finalSnapshotArtifact,
  finalSnapshotStep,
}: {
  locale: Locale
  sessionId: string | null
  runId: string | null
  manifestArtifact: string | null
  finalSnapshotArtifact: string | null
  finalSnapshotStep: number | null
}) {
  const triggerRef = useRef<HTMLButtonElement | null>(null)

  function handleCloseAutoFocus(event: Event) {
    event.preventDefault()
    triggerRef.current?.blur()
  }

  return (
    <DropdownMenu.Root>
      <Tooltip label={t(locale, "app.runArtifact")}>
        <DropdownMenu.Trigger asChild>
          <Button
            ref={triggerRef}
            size="icon"
            variant="outline"
            aria-label={t(locale, "app.runArtifact")}
          >
            <Braces className="h-4 w-4" />
          </Button>
        </DropdownMenu.Trigger>
      </Tooltip>
      <DropdownMenu.Portal>
        <DropdownMenu.Content
          align="end"
          sideOffset={8}
          onCloseAutoFocus={handleCloseAutoFocus}
          className="z-50 w-72 rounded-md border border-lab-line bg-lab-panel2 p-2 text-xs text-lab-text shadow-xl"
        >
          <ArtifactRow
            label={t(locale, "app.session")}
            value={sessionId ?? t(locale, "app.noSession")}
          />
          <ArtifactRow
            label={t(locale, "app.runArtifact")}
            value={runId ?? t(locale, "app.noRunArtifact")}
          />
          <ArtifactRow
            label={t(locale, "app.manifestArtifact")}
            value={manifestArtifact ?? t(locale, "app.noRunArtifact")}
          />
          <ArtifactRow
            label={t(locale, "app.finalSnapshot")}
            value={`${finalSnapshotArtifact ?? t(locale, "app.noRunArtifact")} / ${
              finalSnapshotStep ?? t(locale, "common.unknown")
            }`}
          />
        </DropdownMenu.Content>
      </DropdownMenu.Portal>
    </DropdownMenu.Root>
  )
}

function ArtifactRow({ label, value }: { label: string; value: string }) {
  return (
    <div className="grid grid-cols-[96px_1fr] gap-2 rounded px-2 py-1.5">
      <span className="text-lab-muted">{label}</span>
      <span className="truncate text-right font-mono tabular-nums">
        {value}
      </span>
    </div>
  )
}

function LanguageMenu({
  locale,
  onLocaleChange,
}: {
  locale: Locale
  onLocaleChange: (locale: Locale) => void
}) {
  const localeOptions: Locale[] = ["zh-CN", "en-US"]
  const triggerRef = useRef<HTMLButtonElement | null>(null)

  function handleCloseAutoFocus(event: Event) {
    event.preventDefault()
    triggerRef.current?.blur()
  }

  return (
    <DropdownMenu.Root>
      <Tooltip label={t(locale, "app.language")}>
        <DropdownMenu.Trigger asChild>
          <Button
            ref={triggerRef}
            size="icon"
            variant="outline"
            aria-label={t(locale, "app.language")}
            className="ml-1"
          >
            <Languages className="h-4 w-4" />
          </Button>
        </DropdownMenu.Trigger>
      </Tooltip>
      <DropdownMenu.Portal>
        <DropdownMenu.Content
          align="end"
          sideOffset={8}
          onCloseAutoFocus={handleCloseAutoFocus}
          className="z-50 w-36 rounded-md border border-lab-line bg-lab-panel2 p-2 text-sm text-lab-text shadow-xl"
        >
          {localeOptions.map((option) => (
            <DropdownMenu.Item
              key={option}
              onSelect={() => onLocaleChange(option)}
              className="flex h-7 cursor-default select-none items-center gap-2 rounded px-2 outline-none data-[highlighted]:bg-lab-accent/[0.18]"
            >
              <span className="flex h-4 w-4 items-center justify-center">
                {locale === option ? (
                  <Check className="h-3.5 w-3.5 text-lab-accent" />
                ) : null}
              </span>
              <span>{localeLabels[option]}</span>
            </DropdownMenu.Item>
          ))}
        </DropdownMenu.Content>
      </DropdownMenu.Portal>
    </DropdownMenu.Root>
  )
}

function LayerMenu({
  locale,
  layers,
  onLayerChange,
}: {
  locale: Locale
  layers: LayerState
  onLayerChange: (key: keyof LayerState, value: boolean) => void
}) {
  const [open, setOpen] = useState(false)
  const closeTimer = useRef<number | null>(null)
  const triggerRef = useRef<HTMLButtonElement | null>(null)

  function clearCloseTimer() {
    if (closeTimer.current !== null) {
      window.clearTimeout(closeTimer.current)
      closeTimer.current = null
    }
  }

  function scheduleClose() {
    clearCloseTimer()
    closeTimer.current = window.setTimeout(() => {
      setOpen(false)
      closeTimer.current = null
    }, 1200)
  }

  function handleOpenChange(nextOpen: boolean) {
    clearCloseTimer()
    setOpen(nextOpen)
  }

  function handleCloseAutoFocus(event: Event) {
    event.preventDefault()
    triggerRef.current?.blur()
  }

  useEffect(() => clearCloseTimer, [])

  return (
    <DropdownMenu.Root open={open} onOpenChange={handleOpenChange}>
      <Tooltip label={t(locale, "tooltip.canvasLayers")}>
        <DropdownMenu.Trigger asChild>
          <Button
            ref={triggerRef}
            size="icon"
            variant="outline"
            aria-label={t(locale, "tooltip.canvasLayers")}
          >
            <Layers className="h-4 w-4" />
          </Button>
        </DropdownMenu.Trigger>
      </Tooltip>
      <DropdownMenu.Portal>
        <DropdownMenu.Content
          align="end"
          sideOffset={8}
          onPointerEnter={clearCloseTimer}
          onPointerLeave={scheduleClose}
          onFocusCapture={clearCloseTimer}
          onBlurCapture={scheduleClose}
          onCloseAutoFocus={handleCloseAutoFocus}
          className="z-50 w-52 rounded-md border border-lab-line bg-lab-panel2 p-2 text-sm text-lab-text shadow-xl"
        >
          {(Object.keys(layers) as Array<LayerKey>).map((key) => (
            <DropdownMenu.CheckboxItem
              key={key}
              checked={layers[key]}
              onSelect={(event) => event.preventDefault()}
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
  )
}

function OverlayPresetMenu({
  locale,
  onApplyPreset,
}: {
  locale: Locale
  onApplyPreset: (preset: OverlayPresetId) => void
}) {
  const triggerRef = useRef<HTMLButtonElement | null>(null)
  const presetOrder: OverlayPresetId[] = [
    "stackStability",
    "trajectoryFocus",
    "perturbationReview",
    "latticeGrid",
  ]

  function handleCloseAutoFocus(event: Event) {
    event.preventDefault()
    triggerRef.current?.blur()
  }

  return (
    <DropdownMenu.Root>
      <Tooltip label={t(locale, "tooltip.overlayPresets")}>
        <DropdownMenu.Trigger asChild>
          <Button
            ref={triggerRef}
            size="icon"
            variant="outline"
            aria-label={t(locale, "tooltip.overlayPresets")}
          >
            <SlidersHorizontal className="h-4 w-4" />
          </Button>
        </DropdownMenu.Trigger>
      </Tooltip>
      <DropdownMenu.Portal>
        <DropdownMenu.Content
          align="end"
          sideOffset={8}
          onCloseAutoFocus={handleCloseAutoFocus}
          className="z-50 w-64 rounded-md border border-lab-line bg-lab-panel2 p-2 text-sm text-lab-text shadow-xl"
        >
          {presetOrder.map((preset) => (
            <DropdownMenu.Item
              key={preset}
              onSelect={() => onApplyPreset(preset)}
              className="grid cursor-default select-none gap-0.5 rounded px-2 py-1.5 outline-none data-[highlighted]:bg-lab-accent/[0.18]"
            >
              <span>{overlayPresetLabel(locale, preset)}</span>
              <span className="text-[11px] text-lab-muted">
                {overlayPresetDescription(locale, preset)}
              </span>
            </DropdownMenu.Item>
          ))}
        </DropdownMenu.Content>
      </DropdownMenu.Portal>
    </DropdownMenu.Root>
  )
}
