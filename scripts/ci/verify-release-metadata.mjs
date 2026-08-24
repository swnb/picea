import { readFile } from "node:fs/promises"

const metadataPath = process.argv[2]
if (!metadataPath) {
  console.error("Usage: verify-release-metadata.mjs <cargo-metadata.json>")
  process.exit(64)
}

const metadata = JSON.parse(await readFile(metadataPath, "utf8"))
const byName = new Map(metadata.packages.map((pkg) => [pkg.name, pkg]))
const failures = []

for (const name of ["picea", "picea-macro-tools"]) {
  const pkg = byName.get(name)
  if (!pkg) {
    failures.push(`missing package metadata for ${name}`)
    continue
  }
  for (const field of [
    "description",
    "documentation",
    "license_file",
    "readme",
    "repository",
  ]) {
    if (!pkg[field]) failures.push(`${name}.${field} must be set`)
  }
  if (!Array.isArray(pkg.authors) || pkg.authors.length === 0) {
    failures.push(`${name}.authors must not be empty`)
  }
  if (!Array.isArray(pkg.categories) || pkg.categories.length === 0) {
    failures.push(`${name}.categories must not be empty`)
  }
  if (!Array.isArray(pkg.keywords) || pkg.keywords.length === 0) {
    failures.push(`${name}.keywords must not be empty`)
  }
}

const lab = byName.get("picea-lab")
if (!lab) {
  failures.push("missing package metadata for picea-lab")
} else if (!Array.isArray(lab.publish) || lab.publish.length !== 0) {
  failures.push("picea-lab.publish must be false")
}

if (failures.length > 0) {
  console.error("Release metadata contract failed:")
  for (const failure of failures) console.error(`- ${failure}`)
  process.exit(1)
}

console.log("Release metadata contract passed (package verification only; no publish)")
