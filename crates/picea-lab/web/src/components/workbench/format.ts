import { dynamicValueLabel, type Locale } from "../../i18n"
import type { FrameRecord, WorkbenchLog } from "../../types"

export function warmStartTriplet(stats: FrameRecord["snapshot"]["stats"]) {
  return `${stats.warm_start_hit_count ?? 0}/${stats.warm_start_miss_count ?? 0}/${stats.warm_start_drop_count ?? 0}`
}

export function ccdQuad(stats: FrameRecord["snapshot"]["stats"]) {
  return `${stats.ccd_candidate_count ?? 0}/${stats.ccd_hit_count ?? 0}/${stats.ccd_miss_count ?? 0}/${stats.ccd_clamp_count ?? 0}`
}

export function traceStage(
  locale: Locale,
  termination: string,
  iterations: number,
): string {
  return `${dynamicValueLabel(locale, termination)} / ${iterations}`
}

export function vec(value: { x: number; y: number }): string {
  return `${value.x.toFixed(3)}, ${value.y.toFixed(3)}`
}

export function log(level: WorkbenchLog["level"], message: string): WorkbenchLog {
  return {
    time: new Date().toLocaleTimeString("en-US", { hour12: false }),
    level,
    message,
  }
}

export function messageOf(error: unknown): string {
  return error instanceof Error ? error.message : String(error)
}
