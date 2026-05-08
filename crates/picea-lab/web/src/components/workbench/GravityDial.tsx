import { useCallback, useMemo, useRef } from "react"
import { Check, RotateCcw, Undo2 } from "lucide-react"

import { Button } from "../ui/button"
import { Input } from "../ui/input"
import { Tooltip } from "../ui/radix"
import { t, type Locale } from "../../i18n"

export type GravityVector = {
  x: number
  y: number
}

export const DEFAULT_GRAVITY: GravityVector = { x: 0, y: 9.8 }
const MAX_GRAVITY_MAGNITUDE = 20
const KEYBOARD_STEP = 0.5

function finiteOrZero(value: number): number {
  return Number.isFinite(value) ? value : 0
}

function roundForInput(value: number): number {
  return Number(value.toFixed(3))
}

function magnitudeOf(vector: GravityVector): number {
  return Math.hypot(vector.x, vector.y)
}

function clampMagnitude(vector: GravityVector): GravityVector {
  const magnitude = magnitudeOf(vector)
  if (magnitude <= MAX_GRAVITY_MAGNITUDE || magnitude <= Number.EPSILON) {
    return vector
  }
  const scale = MAX_GRAVITY_MAGNITUDE / magnitude
  return {
    x: vector.x * scale,
    y: vector.y * scale,
  }
}

function vectorWithMagnitude(vector: GravityVector, nextMagnitude: number): GravityVector {
  const magnitude = magnitudeOf(vector)
  const direction =
    magnitude > Number.EPSILON
      ? { x: vector.x / magnitude, y: vector.y / magnitude }
      : { x: 0, y: 1 }
  return {
    x: direction.x * nextMagnitude,
    y: direction.y * nextMagnitude,
  }
}

export function GravityDial({
  locale,
  enabled,
  vector,
  onEnabledChange,
  onVectorChange,
  showNextRunHint,
  appliedVector,
  liveApplyAvailable = false,
  canApply = false,
  canUndo = false,
  applyBusy = false,
  applyError = null,
  onApply,
  onUndo,
  onReset,
}: {
  locale: Locale
  enabled: boolean
  vector: GravityVector
  onEnabledChange: (enabled: boolean) => void
  onVectorChange: (vector: GravityVector) => void
  showNextRunHint: boolean
  appliedVector?: GravityVector
  liveApplyAvailable?: boolean
  canApply?: boolean
  canUndo?: boolean
  applyBusy?: boolean
  applyError?: string | null
  onApply?: () => void
  onUndo?: () => void
  onReset?: () => void
}) {
  const dialRef = useRef<HTMLDivElement | null>(null)
  const magnitude = magnitudeOf(vector)
  const normalizedForDial = useMemo(() => {
    const clamped = clampMagnitude(vector)
    return {
      x: clamped.x / MAX_GRAVITY_MAGNITUDE,
      y: clamped.y / MAX_GRAVITY_MAGNITUDE,
    }
  }, [vector])

  const commitVector = useCallback(
    (next: GravityVector) => {
      onEnabledChange(true)
      onVectorChange(clampMagnitude({
        x: finiteOrZero(next.x),
        y: finiteOrZero(next.y),
      }))
    },
    [onEnabledChange, onVectorChange],
  )

  const updateFromPointer = useCallback(
    (clientX: number, clientY: number) => {
      const rect = dialRef.current?.getBoundingClientRect()
      if (!rect) {
        return
      }
      const radius = Math.max(1, Math.min(rect.width, rect.height) / 2 - 10)
      const centerX = rect.left + rect.width / 2
      const centerY = rect.top + rect.height / 2
      const dx = Math.max(-radius, Math.min(radius, clientX - centerX))
      const dy = Math.max(-radius, Math.min(radius, clientY - centerY))
      const scale = MAX_GRAVITY_MAGNITUDE / radius
      commitVector({ x: dx * scale, y: dy * scale })
    },
    [commitVector],
  )

  const handleMagnitudeChange = (value: number) => {
    commitVector(vectorWithMagnitude(vector, Math.max(0, finiteOrZero(value))))
  }
  const hintKey = !enabled
    ? "run.gravityPendingDisabled"
    : liveApplyAvailable
      ? canApply
        ? "run.gravityDirtyLive"
        : "run.gravityAppliedLive"
      : showNextRunHint
        ? "run.gravityPendingActive"
        : "run.gravityPendingIdle"

  return (
    <div className="grid gap-3">
      <div className="grid grid-cols-[11rem_1fr] gap-4">
        <div
          ref={dialRef}
          role="slider"
          tabIndex={0}
          aria-label={t(locale, "run.gravityDial")}
          aria-valuemin={0}
          aria-valuemax={MAX_GRAVITY_MAGNITUDE}
          aria-valuenow={roundForInput(magnitude)}
          className="relative aspect-square w-44 touch-none select-none rounded-md border border-lab-line bg-lab-canvas focus-visible:outline-none focus-visible:shadow-focus"
          onPointerDown={(event) => {
            event.currentTarget.setPointerCapture(event.pointerId)
            updateFromPointer(event.clientX, event.clientY)
          }}
          onPointerMove={(event) => {
            if (event.currentTarget.hasPointerCapture(event.pointerId)) {
              updateFromPointer(event.clientX, event.clientY)
            }
          }}
          onKeyDown={(event) => {
            if (event.key === "ArrowLeft") {
              event.preventDefault()
              commitVector({ x: vector.x - KEYBOARD_STEP, y: vector.y })
            } else if (event.key === "ArrowRight") {
              event.preventDefault()
              commitVector({ x: vector.x + KEYBOARD_STEP, y: vector.y })
            } else if (event.key === "ArrowUp") {
              event.preventDefault()
              commitVector({ x: vector.x, y: vector.y - KEYBOARD_STEP })
            } else if (event.key === "ArrowDown") {
              event.preventDefault()
              commitVector({ x: vector.x, y: vector.y + KEYBOARD_STEP })
            } else if (event.key === "+" || event.key === "=") {
              event.preventDefault()
              handleMagnitudeChange(Math.min(MAX_GRAVITY_MAGNITUDE, magnitude + KEYBOARD_STEP))
            } else if (event.key === "-" || event.key === "_") {
              event.preventDefault()
              handleMagnitudeChange(Math.max(0, magnitude - KEYBOARD_STEP))
            } else if (event.key === "Home") {
              event.preventDefault()
              commitVector({ x: 0, y: 0 })
            } else if (event.key === "End") {
              event.preventDefault()
              commitVector(DEFAULT_GRAVITY)
            }
          }}
        >
          <div className="absolute inset-4 rounded-full border border-lab-line" />
          <div className="absolute inset-10 rounded-full border border-dashed border-lab-line/80" />
          <div className="absolute left-1/2 top-3 bottom-3 w-px bg-lab-line/70" />
          <div className="absolute left-3 right-3 top-1/2 h-px bg-lab-line/70" />
          <div
            className="absolute left-1/2 top-1/2 h-0.5 origin-left rounded-full bg-lab-danger"
            style={{
              width: `${Math.min(50, Math.hypot(normalizedForDial.x, normalizedForDial.y) * 50)}%`,
              transform: `rotate(${Math.atan2(normalizedForDial.y, normalizedForDial.x)}rad)`,
            }}
          />
          <div
            className="absolute h-4 w-4 -translate-x-1/2 -translate-y-1/2 rounded-full border border-lab-danger bg-lab-danger/25 shadow"
            style={{
              left: `${50 + normalizedForDial.x * 50}%`,
              top: `${50 + normalizedForDial.y * 50}%`,
            }}
          />
          <span className="absolute left-2 top-1 text-[10px] text-lab-muted">y-</span>
          <span className="absolute bottom-1 left-2 text-[10px] text-lab-muted">y+</span>
          <span className="absolute right-2 top-1/2 -translate-y-1/2 text-[10px] text-lab-muted">x+</span>
        </div>

        <div className="grid content-start gap-2">
          <div className="grid grid-cols-[5.5rem_1fr] items-center gap-2">
            <label className="text-xs text-lab-muted" htmlFor="gravity-x">
              {t(locale, "run.gravityX")}
            </label>
            <Input
              id="gravity-x"
              type="number"
              step="0.1"
              value={roundForInput(vector.x)}
              onChange={(event) => commitVector({ x: Number(event.target.value), y: vector.y })}
            />
            <label className="text-xs text-lab-muted" htmlFor="gravity-y">
              {t(locale, "run.gravityY")}
            </label>
            <Input
              id="gravity-y"
              type="number"
              step="0.1"
              value={roundForInput(vector.y)}
              onChange={(event) => commitVector({ x: vector.x, y: Number(event.target.value) })}
            />
            <label className="text-xs text-lab-muted" htmlFor="gravity-magnitude">
              {t(locale, "run.gravityMagnitude")}
            </label>
            <Input
              id="gravity-magnitude"
              type="number"
              min={0}
              max={MAX_GRAVITY_MAGNITUDE}
              step="0.1"
              value={roundForInput(magnitude)}
              onChange={(event) => handleMagnitudeChange(Number(event.target.value))}
            />
          </div>
          <div className="flex items-center gap-2">
            {liveApplyAvailable ? (
              <Tooltip label={t(locale, "run.gravityApply")}>
                <Button
                  size="sm"
                  disabled={!enabled || !canApply || applyBusy}
                  onClick={onApply}
                >
                  <Check className="h-4 w-4" />
                  {applyBusy ? t(locale, "run.gravityApplying") : t(locale, "run.gravityApply")}
                </Button>
              </Tooltip>
            ) : null}
            <Tooltip label={t(locale, "run.gravityUndo")}>
              <Button
                size="icon"
                variant="outline"
                disabled={!canUndo || applyBusy}
                aria-label={t(locale, "run.gravityUndo")}
                onClick={onUndo}
              >
                <Undo2 className="h-4 w-4" />
              </Button>
            </Tooltip>
            <Tooltip label={t(locale, "run.gravityReset")}>
              <Button
                size="icon"
                variant="outline"
                aria-label={t(locale, "run.gravityReset")}
                disabled={applyBusy}
                onClick={() => {
                  if (onReset) {
                    onReset()
                  } else {
                    commitVector(DEFAULT_GRAVITY)
                  }
                }}
              >
                <RotateCcw className="h-4 w-4" />
              </Button>
            </Tooltip>
            <div className="min-w-0 text-xs text-lab-muted">
              {t(locale, hintKey)}
              {appliedVector ? (
                <span className="ml-2 font-mono text-lab-muted/80">
                  {t(locale, "run.gravityAppliedValue", {
                    x: roundForInput(appliedVector.x),
                    y: roundForInput(appliedVector.y),
                  })}
                </span>
              ) : null}
            </div>
          </div>
          {applyError ? (
            <div className="text-xs text-lab-danger">
              {t(locale, "run.gravityApplyError", { message: applyError })}
            </div>
          ) : null}
        </div>
      </div>
    </div>
  )
}
