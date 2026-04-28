import * as Tabs from "@radix-ui/react-tabs"
import { Gauge, Pause, RotateCcw, SkipForward } from "lucide-react"

import { Input } from "../ui/input"
import { PanelHeader } from "../ui/panel"
import { Checkbox, Slider, Tooltip } from "../ui/radix"
import { t, type Locale } from "../../i18n"
import type { FrameRecord, WorkbenchLog } from "../../types"
import { vec } from "./format"

export function BottomTimeline({
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
  onPlay,
  onPause,
  onStep,
  onReset,
}: {
  frames: FrameRecord[]
  frameIndex: number
  onFrameChange: (value: number) => void
  logs: WorkbenchLog[]
  frameCount: number
  setFrameCount: (value: number) => void
  useCustomGravity: boolean
  setUseCustomGravity: (value: boolean) => void
  gravityY: number
  setGravityY: (value: number) => void
  locale: Locale
  onPlay: () => void
  onPause: () => void
  onStep: () => void
  onReset: () => void
}) {
  const frame = frames[Math.min(frameIndex, Math.max(0, frames.length - 1))]
  return (
    <Tabs.Root defaultValue="timeline" className="flex h-full min-h-0 flex-col">
      <PanelHeader>
        <div className="flex items-center gap-3">
          <div className="flex items-center rounded-md bg-black/20 p-0.5 shadow-inner">
            <Tooltip label={t(locale, "tooltip.pausePlayback")}>
              <button
                type="button"
                className="flex h-6 w-7 items-center justify-center rounded-sm text-lab-muted transition-colors hover:bg-white/10 hover:text-lab-text"
                onClick={onPause}
              >
                <Pause className="h-3.5 w-3.5" />
              </button>
            </Tooltip>
            <Tooltip label={t(locale, "tooltip.playTimeline")}>
              <button
                type="button"
                className="flex h-6 w-7 items-center justify-center rounded-sm text-lab-muted transition-colors hover:bg-white/10 hover:text-lab-text"
                onClick={onPlay}
              >
                <Gauge className="h-3.5 w-3.5" />
              </button>
            </Tooltip>
            <Tooltip label={t(locale, "tooltip.advanceFrame")}>
              <button
                type="button"
                className="flex h-6 w-7 items-center justify-center rounded-sm text-lab-muted transition-colors hover:bg-white/10 hover:text-lab-text"
                onClick={onStep}
              >
                <SkipForward className="h-3.5 w-3.5" />
              </button>
            </Tooltip>
            <Tooltip label={t(locale, "tooltip.resetTimeline")}>
              <button
                type="button"
                className="flex h-6 w-7 items-center justify-center rounded-sm text-lab-muted transition-colors hover:bg-white/10 hover:text-lab-text"
                onClick={onReset}
              >
                <RotateCcw className="h-3.5 w-3.5" />
              </button>
            </Tooltip>
          </div>
          <div className="h-4 w-px bg-lab-line/80" />
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
        </div>
        <div className="flex items-center gap-2 text-xs text-lab-muted">
          <span className="tabular-nums">
            {frame?.simulated_time.toFixed(3)}s
          </span>
          <span className="tabular-nums">{frame?.state_hash}</span>
        </div>
      </PanelHeader>

      <Tabs.Content
        value="timeline"
        className="min-h-0 flex-1 p-3 outline-none"
      >
        <div className="mb-3 flex items-center gap-3">
          <span className="w-20 text-xs text-lab-muted">
            {t(locale, "timeline.frameAt", { frame: frameIndex })}
          </span>
          <Slider
            value={frameIndex}
            min={0}
            max={Math.max(0, frames.length - 1)}
            step={1}
            onValueChange={onFrameChange}
          />
          <span className="w-20 text-right text-xs text-lab-muted">
            {t(locale, "timeline.totalFrames", { count: frames.length })}
          </span>
        </div>
        <div className="grid grid-cols-4 gap-2">
          <Metric
            label={t(locale, "metric.step")}
            value={frame?.snapshot.stats.step_index ?? 0}
          />
          <Metric
            label={t(locale, "metric.simTime")}
            value={(frame?.snapshot.meta.simulated_time ?? 0).toFixed(3)}
          />
          <Metric
            label={t(locale, "metric.gravity")}
            value={vec(frame?.snapshot.meta.gravity ?? { x: 0, y: 0 })}
          />
          <Metric
            label={t(locale, "metric.manifolds")}
            value={frame?.snapshot.manifolds.length ?? 0}
          />
        </div>
      </Tabs.Content>

      <Tabs.Content
        value="logs"
        className="min-h-0 flex-1 overflow-auto p-2 outline-none"
      >
        <div className="space-y-1 font-mono text-xs">
          {logs.map((entry, index) => (
            <div
              key={`${entry.time}-${index}`}
              className="grid grid-cols-[74px_52px_1fr] gap-2 rounded px-2 py-1 hover:bg-white/5"
            >
              <span className="text-lab-muted">{entry.time}</span>
              <span
                className={
                  entry.level === "error"
                    ? "text-lab-danger"
                    : entry.level === "warn"
                      ? "text-lab-warn"
                      : "text-lab-accent"
                }
              >
                {entry.level}
              </span>
              <span className="min-w-0 truncate text-lab-text">
                {entry.message}
              </span>
            </div>
          ))}
        </div>
      </Tabs.Content>

      <Tabs.Content value="run" className="min-h-0 flex-1 p-3 outline-none">
        <div className="grid max-w-2xl grid-cols-[140px_1fr] items-center gap-3">
          <label className="text-sm text-lab-muted">
            {t(locale, "run.frameCount")}
          </label>
          <Input
            type="number"
            min={1}
            max={2000}
            value={frameCount}
            onChange={(event) =>
              setFrameCount(Math.max(1, Number(event.target.value) || 1))
            }
          />
          <label className="text-sm text-lab-muted">
            {t(locale, "run.gravityOverride")}
          </label>
          <Checkbox
            checked={useCustomGravity}
            onCheckedChange={setUseCustomGravity}
            label={t(locale, "run.sendOverride")}
          />
          <label className="text-sm text-lab-muted">
            {t(locale, "run.gravityY")}
          </label>
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
  )
}

function Metric({ label, value }: { label: string; value: string | number }) {
  return (
    <div className="rounded-md border border-lab-line bg-lab-panel2 px-2 py-1.5">
      <div className="truncate text-[11px] uppercase tracking-normal text-lab-muted">
        {label}
      </div>
      <div className="truncate font-mono text-sm tabular-nums text-lab-text">
        {value}
      </div>
    </div>
  )
}
