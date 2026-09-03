import { readFile, readdir, stat } from "node:fs/promises"
import { basename, dirname, join, resolve } from "node:path"
import { fileURLToPath } from "node:url"

const scriptDir = dirname(fileURLToPath(import.meta.url))
const repoRoot = resolve(scriptDir, "..", "..")
const criterionRoot = join(repoRoot, "target", "criterion")
const baselineName = process.argv[2]
const startedAtMs = Number(process.argv[3])

if (!baselineName || !Number.isFinite(startedAtMs)) {
  console.error(
    "Usage: verify-criterion-counters.mjs <baseline-name> <run-started-ms>",
  )
  process.exit(64)
}

const expected = JSON.parse(
  await readFile(join(scriptDir, "criterion-counters.json"), "utf8"),
).sort()

async function findCurrentBenchmarkFiles(directory) {
  const files = []
  for (const entry of await readdir(directory, { withFileTypes: true })) {
    const path = join(directory, entry.name)
    if (entry.isDirectory()) {
      files.push(...(await findCurrentBenchmarkFiles(path)))
    } else if (
      entry.name === "benchmark.json" &&
      basename(dirname(path)) === baselineName
    ) {
      files.push(path)
    }
  }
  return files
}

const benchmarkFiles = await findCurrentBenchmarkFiles(criterionRoot)
const actual = []
for (const path of benchmarkFiles) {
  const fileStats = await stat(path)
  if (fileStats.mtimeMs + 1_000 < startedAtMs) continue

  const benchmark = JSON.parse(await readFile(path, "utf8"))
  if (typeof benchmark.full_id !== "string") {
    throw new Error(`missing full_id in ${path}`)
  }
  actual.push(benchmark.full_id)
}
actual.sort()

const expectedSet = new Set(expected)
const actualSet = new Set(actual)
const missing = expected.filter((id) => !actualSet.has(id))
const unexpected = actual.filter((id) => !expectedSet.has(id))

if (
  actual.length !== actualSet.size ||
  expected.length !== expectedSet.size ||
  missing.length > 0 ||
  unexpected.length > 0
) {
  console.error("Criterion counter manifest mismatch")
  for (const id of missing) console.error(`- missing: ${id}`)
  for (const id of unexpected) console.error(`- unexpected: ${id}`)
  if (actual.length !== actualSet.size) {
    console.error("- duplicate benchmark full_id values were found")
  }
  if (expected.length !== expectedSet.size) {
    console.error("- criterion-counters.json contains duplicate values")
  }
  process.exit(1)
}

console.log(
  `Criterion counter manifest passed: ${actual.length} exact scenarios from ${baselineName}`,
)
