import type { ReactNode } from "react"

import { Button } from "../../ui/button"
import { Badge } from "../../ui/badge"
import { Input } from "../../ui/input"
import { Checkbox, Select, Slider, Tooltip } from "../../ui/radix"
import {
  booleanLabel,
  dynamicValueLabel,
  parameterSourceLabel,
  scenarioParameterHelp,
  scenarioParameterLabel,
  t,
  type Locale,
} from "../../../i18n"
import type {
  ScenarioParameterDescriptor,
  ScenarioParameterValue,
} from "../../../types"

export type ParameterSourceKind = "default" | "current" | "effective" | "dirty"

type ParameterValueMap = Record<string, ScenarioParameterValue>

function formatParameterValue(
  locale: Locale,
  value: ScenarioParameterValue | null | undefined,
): string {
  if (typeof value === "boolean") {
    return booleanLabel(locale, value)
  }
  if (typeof value === "number") {
    return Number.isInteger(value) ? String(value) : value.toFixed(2).replace(/\.?0+$/, "")
  }
  if (typeof value === "string") {
    return dynamicValueLabel(locale, value)
  }
  return "-"
}

function clampNumber(
  value: number,
  descriptor: ScenarioParameterDescriptor,
): number {
  const min = descriptor.min ?? -Number.MAX_SAFE_INTEGER
  const max = descriptor.max ?? Number.MAX_SAFE_INTEGER
  let next = Math.min(max, Math.max(min, value))
  if (descriptor.type === "integer") {
    next = Math.round(next)
  }
  return next
}

function stepPrecision(step: number | null | undefined): number {
  if (step == null || !Number.isFinite(step)) {
    return 6
  }
  const text = String(step)
  if (!text.includes(".")) {
    return 0
  }
  return Math.min(6, text.length - text.indexOf(".") - 1)
}

function normalizeNumberForDescriptor(
  value: number,
  descriptor: ScenarioParameterDescriptor,
): number {
  const clamped = clampNumber(value, descriptor)
  if (descriptor.type === "integer") {
    return clamped
  }

  const step = descriptor.step
  if (step == null || step <= 0 || !Number.isFinite(step)) {
    return Number(clamped.toFixed(6))
  }

  const min = descriptor.min ?? 0
  const precision = stepPrecision(step)
  const snapped = min + Math.round((clamped - min) / step) * step
  const tolerance = Math.max(1e-6, Math.abs(step) * 1e-4)
  if (Math.abs(snapped - clamped) <= tolerance) {
    return Number(snapped.toFixed(precision))
  }
  return Number(clamped.toFixed(6))
}

function normalizeParameterValue(
  descriptor: ScenarioParameterDescriptor,
  value: ScenarioParameterValue | null | undefined,
): ScenarioParameterValue | null | undefined {
  if (typeof value !== "number") {
    return value
  }
  return normalizeNumberForDescriptor(value, descriptor)
}

function fallbackValueForDescriptor(
  descriptor: ScenarioParameterDescriptor,
): ScenarioParameterValue {
  if (descriptor.type === "boolean") {
    return false
  }
  if (descriptor.type === "select") {
    return descriptor.options?.[0]?.value ?? ""
  }
  return normalizeNumberForDescriptor(descriptor.min ?? 0, descriptor)
}

function valuesMatch(
  descriptor: ScenarioParameterDescriptor,
  left: ScenarioParameterValue | null | undefined,
  right: ScenarioParameterValue | null | undefined,
): boolean {
  if (typeof left === "number" && typeof right === "number") {
    const normalizedLeft = normalizeNumberForDescriptor(left, descriptor)
    const normalizedRight = normalizeNumberForDescriptor(right, descriptor)
    return Math.abs(normalizedLeft - normalizedRight) <= 1e-6
  }
  return left === right
}

function resolveParameterValues({
  descriptor,
  values,
  defaultValues,
  runningValues,
  effectiveValues,
}: {
  descriptor: ScenarioParameterDescriptor
  values: ParameterValueMap
  defaultValues: ParameterValueMap
  runningValues?: ParameterValueMap | null
  effectiveValues?: ParameterValueMap | null
}) {
  const defaultValue =
    normalizeParameterValue(
      descriptor,
      defaultValues[descriptor.key] ?? descriptor.default ?? null,
    ) ?? fallbackValueForDescriptor(descriptor)
  const runningValue = normalizeParameterValue(
    descriptor,
    runningValues?.[descriptor.key],
  )
  const effectiveValue =
    normalizeParameterValue(
      descriptor,
      effectiveValues?.[descriptor.key],
    ) ?? runningValue ?? defaultValue
  const currentValue =
    normalizeParameterValue(descriptor, values[descriptor.key]) ??
    effectiveValue
  const dirty = !valuesMatch(
    descriptor,
    currentValue,
    runningValue ?? defaultValue,
  )

  return {
    currentValue,
    defaultValue,
    runningValue,
    effectiveValue,
    dirty,
  }
}

export function ParameterSourceBadge({
  locale,
  source,
}: {
  locale: Locale
  source: ParameterSourceKind
}) {
  const tone =
    source === "dirty"
      ? "warn"
      : source === "effective"
        ? "accent"
        : source === "current"
          ? "green"
          : "neutral"
  return (
    <Badge tone={tone}>
      {parameterSourceLabel(locale, source)}
    </Badge>
  )
}

export function NumberField({
  value,
  min,
  max,
  step,
  onChange,
}: {
  value: number
  min?: number | null
  max?: number | null
  step?: number | null
  onChange: (value: number) => void
}) {
  return (
    <Input
      type="number"
      value={value}
      min={min ?? undefined}
      max={max ?? undefined}
      step={step ?? undefined}
      onChange={(event) => {
        const next = Number(event.target.value)
        if (Number.isFinite(next)) {
          onChange(next)
        }
      }}
    />
  )
}

export function SliderField({
  value,
  min,
  max,
  step,
  onChange,
}: {
  value: number
  min: number
  max: number
  step?: number | null
  onChange: (value: number) => void
}) {
  return (
    <Slider
      value={value}
      min={min}
      max={max}
      step={step ?? undefined}
      onValueChange={onChange}
    />
  )
}

export function RangeField({
  descriptor,
  value,
  onChange,
}: {
  descriptor: ScenarioParameterDescriptor
  value: number
  onChange: (value: number) => void
}) {
  const min = descriptor.min ?? 0
  const max = descriptor.max ?? Math.max(1, value)
  return (
    <div className="grid gap-2">
      <SliderField
        value={value}
        min={min}
        max={max}
        step={descriptor.step}
        onChange={(next) => onChange(clampNumber(next, descriptor))}
      />
      <NumberField
        value={value}
        min={descriptor.min}
        max={descriptor.max}
        step={descriptor.step}
        onChange={(next) => onChange(clampNumber(next, descriptor))}
      />
    </div>
  )
}

export function ToggleField({
  locale,
  checked,
  onCheckedChange,
}: {
  locale: Locale
  checked: boolean
  onCheckedChange: (checked: boolean) => void
}) {
  return (
    <Checkbox
      checked={checked}
      onCheckedChange={onCheckedChange}
      label={checked ? t(locale, "common.true") : t(locale, "common.false")}
    />
  )
}

export function SegmentedField({
  locale,
  value,
  options,
  onChange,
}: {
  locale: Locale
  value: string
  options: Array<{ value: ScenarioParameterValue; label: string }>
  onChange: (value: string) => void
}) {
  if (options.length <= 3) {
    return (
      <div className="flex flex-wrap gap-2">
        {options.map((option) => {
          const optionValue = String(option.value)
          const label = dynamicValueLabel(locale, optionValue) || option.label
          return (
            <Button
              key={optionValue}
              size="sm"
              variant={optionValue === value ? "default" : "outline"}
              onClick={() => onChange(optionValue)}
            >
              {label}
            </Button>
          )
        })}
      </div>
    )
  }

  return (
    <Select
      value={value}
      onValueChange={onChange}
      ariaLabel={t(locale, "run.sceneParameters")}
      items={options.map((option) => ({
        value: String(option.value),
        label: dynamicValueLabel(locale, String(option.value)) || option.label,
      }))}
    />
  )
}

export function ParameterRow({
  locale,
  descriptor,
  currentValue,
  defaultValue,
  runningValue,
  effectiveValue,
  dirty,
  children,
}: {
  locale: Locale
  descriptor: ScenarioParameterDescriptor
  currentValue: ScenarioParameterValue
  defaultValue: ScenarioParameterValue | null | undefined
  runningValue: ScenarioParameterValue | null | undefined
  effectiveValue: ScenarioParameterValue | null | undefined
  dirty: boolean
  children: ReactNode
}) {
  const label = scenarioParameterLabel(locale, descriptor.key, descriptor.label)
  const help = scenarioParameterHelp(locale, descriptor.key)

  return (
    <div className="grid gap-2 border-b border-lab-line/70 py-3 last:border-b-0">
      <div className="flex items-start justify-between gap-3">
        <div className="min-w-0">
          <div className="flex items-center gap-2">
            <span className="truncate text-sm font-medium text-lab-text">{label}</span>
            <Tooltip label={help}>
              <span
                aria-label={help}
                className="inline-flex h-4 w-4 shrink-0 items-center justify-center rounded-full border border-lab-line text-[10px] text-lab-muted"
                tabIndex={0}
              >
                ?
              </span>
            </Tooltip>
          </div>
          <p className="mt-1 text-xs text-lab-muted">{help}</p>
        </div>
        <div className="flex shrink-0 flex-wrap items-center justify-end gap-1.5">
          <ParameterSourceBadge locale={locale} source="default" />
          {runningValue != null ? (
            <ParameterSourceBadge locale={locale} source="current" />
          ) : null}
          {effectiveValue != null ? (
            <ParameterSourceBadge locale={locale} source="effective" />
          ) : null}
          {dirty ? <ParameterSourceBadge locale={locale} source="dirty" /> : null}
        </div>
      </div>
      {children}
      <div className="grid gap-1 text-[11px] text-lab-muted sm:grid-cols-3">
        <span className="truncate">
          {t(locale, "run.parameterDefault")}: {formatParameterValue(locale, defaultValue)}
        </span>
        <span className="truncate">
          {t(locale, "run.parameterCurrent")}: {formatParameterValue(locale, runningValue)}
        </span>
        <span className="truncate">
          {t(locale, "run.parameterEffective")}: {formatParameterValue(locale, currentValue ?? effectiveValue)}
        </span>
      </div>
    </div>
  )
}

export function ParameterPanel({
  locale,
  schema,
  values,
  defaultValues,
  runningValues,
  effectiveValues,
  onChange,
  onResetDefaults,
  onRevertRunning,
}: {
  locale: Locale
  schema: ScenarioParameterDescriptor[]
  values: ParameterValueMap
  defaultValues: ParameterValueMap
  runningValues?: ParameterValueMap | null
  effectiveValues?: ParameterValueMap | null
  onChange: (key: string, value: ScenarioParameterValue) => void
  onResetDefaults: () => void
  onRevertRunning: () => void
}) {
  if (schema.length === 0) {
    return (
      <div className="grid gap-2 rounded-md border border-dashed border-lab-line/80 px-3 py-4 text-sm text-lab-muted">
        <div className="font-medium text-lab-text">{t(locale, "run.sceneParameters")}</div>
        <div>{t(locale, "run.parameterNoSchema")}</div>
      </div>
    )
  }

  const hasRunningConfig = Object.keys(runningValues ?? {}).length > 0
  const hasDirtyValues = schema.some(
    (descriptor) =>
      resolveParameterValues({
        descriptor,
        values,
        defaultValues,
        runningValues,
        effectiveValues,
      }).dirty,
  )

  // Reset defaults / Revert running config actions stay localized through i18n.
  return (
    <section className="grid gap-3">
      <div className="flex flex-wrap items-center justify-between gap-2">
        <div className="min-w-0">
          <h3 className="text-sm font-medium text-lab-text">
            {t(locale, "run.sceneParameters")}
          </h3>
          <p className="mt-1 text-xs text-lab-muted">
            {hasDirtyValues ? t(locale, "run.parameterDirty") : t(locale, "run.parameterEffective")}
          </p>
        </div>
        <div className="flex flex-wrap items-center gap-2">
          <Button size="sm" variant="outline" onClick={onResetDefaults}>
            {t(locale, "run.parameterResetDefaults")}
          </Button>
          <Button
            size="sm"
            variant="outline"
            disabled={!hasRunningConfig}
            onClick={onRevertRunning}
          >
            {t(locale, "run.parameterRevertRunning")}
          </Button>
        </div>
      </div>
      <div className="grid gap-1">
        {schema.map((descriptor) => {
          const {
            currentValue,
            defaultValue,
            runningValue,
            effectiveValue,
            dirty,
          } = resolveParameterValues({
            descriptor,
            values,
            defaultValues,
            runningValues,
            effectiveValues,
          })

          return (
            <ParameterRow
              key={descriptor.key}
              locale={locale}
              descriptor={descriptor}
              currentValue={currentValue}
              defaultValue={defaultValue}
              runningValue={runningValue}
              effectiveValue={effectiveValue}
              dirty={dirty}
            >
              {descriptor.type === "boolean" ? (
                <ToggleField
                  locale={locale}
                  checked={Boolean(currentValue)}
                  onCheckedChange={(next) => onChange(descriptor.key, next)}
                />
              ) : descriptor.type === "select" ? (
                <SegmentedField
                  locale={locale}
                  value={String(currentValue)}
                  options={descriptor.options ?? []}
                  onChange={(next) => onChange(descriptor.key, next)}
                />
              ) : (
                <RangeField
                  descriptor={descriptor}
                  value={Number(currentValue)}
                  onChange={(next) => onChange(descriptor.key, next)}
                />
              )}
            </ParameterRow>
          )
        })}
      </div>
    </section>
  )
}
