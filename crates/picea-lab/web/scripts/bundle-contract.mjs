import { readdir, readFile, stat } from "node:fs/promises"
import { dirname, join, resolve } from "node:path"
import { fileURLToPath } from "node:url"

const MAX_JS_BYTES = 500_000
const scriptDir = dirname(fileURLToPath(import.meta.url))
const distDir = resolve(scriptDir, "..", "dist")
const assetsDir = join(distDir, "assets")

const assetNames = (await readdir(assetsDir)).sort()
const jsAssets = assetNames.filter((name) => name.endsWith(".js"))
const failures = []

if (jsAssets.length < 2) {
  failures.push(`expected at least 2 JavaScript chunks, found ${jsAssets.length}`)
}

for (const name of jsAssets) {
  const { size } = await stat(join(assetsDir, name))
  console.log(`${name}: ${size} bytes`)
  if (size > MAX_JS_BYTES) {
    failures.push(`${name} is ${size} bytes (budget ${MAX_JS_BYTES})`)
  }
}

const html = await readFile(join(distDir, "index.html"), "utf8")
const localAssetRefs = [
  ...html.matchAll(/(?:src|href)="\/?(assets\/[^"]+)"/g),
].map((match) => match[1])

if (localAssetRefs.length === 0) {
  failures.push("index.html does not reference any local assets")
}

for (const assetRef of localAssetRefs) {
  try {
    await stat(join(distDir, assetRef))
  } catch {
    failures.push(`index.html references missing asset: ${assetRef}`)
  }
}

if (failures.length > 0) {
  console.error("Bundle contract failed:")
  for (const failure of failures) {
    console.error(`- ${failure}`)
  }
  process.exit(1)
}

console.log(
  `Bundle contract passed: ${jsAssets.length} JavaScript chunks, each <= ${MAX_JS_BYTES} bytes`,
)
