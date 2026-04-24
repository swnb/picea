import type { FrameRecord, ScenarioDescriptor, SessionRecord } from "./types";

const apiBase = import.meta.env.VITE_PICEA_LAB_API_BASE ?? "";

async function requestJson<T>(path: string, init?: RequestInit): Promise<T> {
  const response = await fetch(`${apiBase}${path}`, {
    ...init,
    headers: {
      "content-type": "application/json",
      ...init?.headers,
    },
  });

  if (!response.ok) {
    throw new Error(`${response.status} ${response.statusText}`);
  }

  return response.json() as Promise<T>;
}

export async function fetchScenarios(): Promise<ScenarioDescriptor[]> {
  const data = await requestJson<{ scenarios: ScenarioDescriptor[] }>("/api/scenarios");
  return data.scenarios;
}

export async function createSession(
  scenarioId: string,
  frameCount: number,
  gravity?: [number, number] | null,
): Promise<SessionRecord> {
  const data = await requestJson<{ session: SessionRecord }>("/api/sessions", {
    method: "POST",
    body: JSON.stringify({
      scenario_id: scenarioId,
      frame_count: frameCount,
      overrides: {
        frame_count: frameCount,
        gravity: gravity ?? undefined,
      },
    }),
  });
  return data.session;
}

export async function controlSession(
  sessionId: string,
  action: "play" | "run" | "reset" | "step" | "pause",
): Promise<SessionRecord> {
  const data = await requestJson<{ session: SessionRecord }>(`/api/sessions/${sessionId}/control`, {
    method: "POST",
    body: JSON.stringify({ action }),
  });
  return data.session;
}

export async function fetchFrames(runId: string): Promise<FrameRecord[]> {
  const response = await fetch(`${apiBase}/api/runs/${runId}/artifacts/frames.jsonl`);
  if (!response.ok) {
    throw new Error(`${response.status} ${response.statusText}`);
  }
  const text = await response.text();
  return text
    .split("\n")
    .map((line) => line.trim())
    .filter(Boolean)
    .map((line) => JSON.parse(line) as FrameRecord);
}

export async function fetchFinalSnapshot(runId: string) {
  return requestJson(`/api/runs/${runId}/artifacts/final_snapshot.json`);
}

export function openSessionEvents(sessionId: string): EventSource {
  return new EventSource(`${apiBase}/api/sessions/${sessionId}/events`);
}
