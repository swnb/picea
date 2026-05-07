type ProfileMeta = Record<string, unknown>

type ProfileSample = {
  name: string
  durationMs: number
  atMs: number
  meta?: ProfileMeta
}

type ProfileSummary = {
  count: number
  totalMs: number
  maxMs: number
  lastMs: number
}

type ProfileGlobal = {
  samples: ProfileSample[]
  summary: Record<string, ProfileSummary>
  reset: () => void
  table: () => void
}

declare global {
  interface Window {
    __PICEA_PROFILE__?: ProfileGlobal
  }
}

const SAMPLE_LIMIT = 600
const SUMMARY_INTERVAL = 30

export function piceaProfileEnabled(): boolean {
  if (typeof window === "undefined") {
    return false
  }
  try {
    const params = new URLSearchParams(window.location.search)
    const queryValue = params.get("picea-profile") ?? params.get("profile")
    return (
      queryValue === "1" ||
      queryValue === "true" ||
      queryValue === "on" ||
      window.localStorage.getItem("picea.lab.profile") === "1"
    )
  } catch {
    return false
  }
}

export function profileStart(): number {
  return profileNow()
}

export function profileMeasure(
  name: string,
  startMs: number,
  meta?: ProfileMeta,
): void {
  if (!piceaProfileEnabled()) {
    return
  }
  recordSample({
    name,
    durationMs: profileNow() - startMs,
    atMs: profileNow(),
    meta,
  })
}

export async function profileAsync<T>(
  name: string,
  action: () => Promise<T>,
  meta?: ProfileMeta | (() => ProfileMeta),
): Promise<T> {
  if (!piceaProfileEnabled()) {
    return action()
  }
  const startMs = profileStart()
  try {
    return await action()
  } finally {
    profileMeasure(name, startMs, typeof meta === "function" ? meta() : meta)
  }
}

export async function profileJsonRequest<T>(
  url: string,
  meta: ProfileMeta,
  request: () => Promise<Response>,
): Promise<T> {
  const startMs = profileStart()
  const response = await request()
  if (!response.ok) {
    profileMeasure("api.json", startMs, {
      ...meta,
      status: response.status,
      ok: false,
    })
    throw new Error(`${response.status} ${response.statusText}`)
  }

  if (!piceaProfileEnabled()) {
    return response.json() as Promise<T>
  }

  const textStartMs = profileStart()
  const text = await response.text()
  const parseStartMs = profileStart()
  const data = JSON.parse(text) as T
  profileMeasure("api.json", startMs, {
    ...meta,
    status: response.status,
    ok: true,
    bytes: byteLength(text),
    readTextMs: parseStartMs - textStartMs,
    parseJsonMs: profileNow() - parseStartMs,
  })
  return data
}

function profileNow(): number {
  return typeof performance === "undefined" ? Date.now() : performance.now()
}

function recordSample(sample: ProfileSample): void {
  const state = profileState()
  state.samples.push(sample)
  if (state.samples.length > SAMPLE_LIMIT) {
    state.samples.splice(0, state.samples.length - SAMPLE_LIMIT)
  }

  const summary = state.summary[sample.name] ?? {
    count: 0,
    totalMs: 0,
    maxMs: 0,
    lastMs: 0,
  }
  summary.count += 1
  summary.totalMs += sample.durationMs
  summary.maxMs = Math.max(summary.maxMs, sample.durationMs)
  summary.lastMs = sample.durationMs
  state.summary[sample.name] = summary

  if (summary.count === 1 || summary.count % SUMMARY_INTERVAL === 0) {
    console.info("[picea-profile]", sample.name, JSON.stringify({
      durationMs: round(sample.durationMs),
      ...sample.meta,
    }))
    const rows = profileRows(state.summary)
    console.info("[picea-profile-summary]", JSON.stringify(rows))
    console.table(rows)
  }
}

function profileState(): ProfileGlobal {
  if (!window.__PICEA_PROFILE__) {
    window.__PICEA_PROFILE__ = {
      samples: [],
      summary: {},
      reset() {
        this.samples = []
        this.summary = {}
      },
      table() {
        console.table(profileRows(this.summary))
      },
    }
  }
  return window.__PICEA_PROFILE__
}

function profileRows(summary: Record<string, ProfileSummary>) {
  return Object.entries(summary).map(([name, entry]) => ({
    name,
    count: entry.count,
    avgMs: round(entry.totalMs / Math.max(1, entry.count)),
    maxMs: round(entry.maxMs),
    lastMs: round(entry.lastMs),
  }))
}

function byteLength(text: string): number {
  return new Blob([text]).size
}

function round(value: number): number {
  return Math.round(value * 100) / 100
}
