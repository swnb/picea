import assert from "node:assert/strict"
import { spawnSync } from "node:child_process"
import {
  copyFileSync,
  mkdirSync,
  mkdtempSync,
  readFileSync,
  rmSync,
  writeFileSync,
} from "node:fs"
import { tmpdir } from "node:os"
import { join } from "node:path"
import { test } from "node:test"

// Run the real shell boundary, including local RTK and hosted-CI routing. Only
// external commands are fixtures; this cannot replace real cargo package checks.
function runRelease(t, fixture = {}) {
  const root = mkdtempSync(join(tmpdir(), "picea release contract-"))
  t.after(() => rmSync(root, { recursive: true, force: true }))
  const scripts = join(root, "scripts", "ci")
  const bin = join(root, "bin")
  const temporary = join(root, "tmp")
  for (const path of [scripts, bin, temporary]) mkdirSync(path, { recursive: true })
  for (const name of ["run.sh", "verify-release-metadata.mjs"]) {
    copyFileSync(new URL(name, import.meta.url), join(scripts, name))
  }

  const packages = ["picea", "picea-macro-tools"].map((name) => ({
    name,
    description: "Release control-flow fixture, not a real package",
    documentation: "https://example.invalid/docs",
    license_file: "LICENSE",
    readme: "README.md",
    repository: "https://example.invalid/repo",
    authors: ["Fixture"],
    categories: ["simulation"],
    keywords: ["fixture"],
  }))
  packages.push({ name: "picea-lab", publish: fixture.invalidMetadata ? null : [] })

  const log = join(root, "commands.jsonl")
  // The fixture protocol is private to these stub processes. Inherited CI is
  // deliberately overridden so a developer and hosted runner exercise all cases.
  const shim = `
const { appendFileSync } = require("node:fs")
const { spawnSync } = require("node:child_process")
const fixture = JSON.parse(process.env.PICEA_RELEASE_FIXTURE)
const args = process.argv.slice(2)
appendFileSync(fixture.log, JSON.stringify({ tool, args }) + "\\n")
if (tool === "rtk" && args[0] === "proxy") {
  const child = spawnSync(args[1], args.slice(2), { stdio: "inherit" })
  process.exit(child.status ?? 99)
}
if (tool === "git" && JSON.stringify(args) === '["status","--porcelain"]') {
  if (fixture.gitFails) process.exit(43)
  process.stdout.write(fixture.dirty ? " M tracked-file\\n?? new-file\\n" : "")
  process.exit(0)
}
if (tool === "cargo" && args[0] === "metadata") {
  if (fixture.metadataFails) process.exit(41)
  process.stdout.write(JSON.stringify({ packages: fixture.packages }))
  process.exit(0)
}
if (tool === "cargo" && args[0] === "package") {
  process.exit(args[2] === fixture.failPackage ? 42 : 0)
}
console.error("Unexpected fixture command", tool, args)
process.exit(99)
`
  for (const tool of ["cargo", "git", "rtk"]) {
    writeFileSync(
      join(bin, tool),
      `#!/usr/bin/env node\nconst tool = ${JSON.stringify(tool)}\n${shim}`,
      { mode: 0o755 },
    )
  }
  const result = spawnSync("bash", [join(scripts, "run.sh"), "release"], {
    cwd: root,
    env: {
      ...process.env,
      PATH: `${bin}:${process.env.PATH}`,
      CI: fixture.ci ? "true" : "",
      TMPDIR: temporary,
      PICEA_RELEASE_FIXTURE: JSON.stringify({ ...fixture, packages, log }),
    },
    encoding: "utf8",
    timeout: 30_000,
  })
  assert.ifError(result.error)
  const commands = readFileSync(log, "utf8").trim().split("\n").map(JSON.parse)
  return {
    ...result,
    commands,
    packageArgs: commands
      .filter(({ tool, args }) => tool === "cargo" && args[0] === "package")
      .map(({ args }) => args),
  }
}

for (const ci of [false, true]) {
  test(`clean ${ci ? "CI" : "local"} release preserves exact package arguments`, (t) => {
    const result = runRelease(t, { ci })
    assert.equal(result.status, 0, result.stdout + result.stderr)
    assert.match(result.stdout, /Package receipt class: clean checkout candidate/)
    assert.deepEqual(result.packageArgs, [
      ["package", "-p", "picea", "--locked"],
      ["package", "-p", "picea-macro-tools", "--locked"],
    ])
    assert.equal(result.commands.some(({ tool }) => tool === "rtk"), !ci)
  })
}

test("dirty local release is explicitly classified and allows both candidates", (t) => {
  const result = runRelease(t, { dirty: true })
  assert.equal(result.status, 0, result.stdout + result.stderr)
  assert.match(result.stdout, /dirty local candidate \(buildable, not clean\/reproducible\)/)
  assert.deepEqual(result.packageArgs, [
    ["package", "-p", "picea", "--locked", "--allow-dirty"],
    ["package", "-p", "picea-macro-tools", "--locked", "--allow-dirty"],
  ])
})

test("dirty CI release fails before packaging", (t) => {
  const result = runRelease(t, { ci: true, dirty: true })
  assert.equal(result.status, 1, result.stdout + result.stderr)
  assert.match(result.stderr, /CI package verification requires a clean checkout/)
  assert.deepEqual(result.packageArgs, [])
})

test("invalid metadata fails before packaging", (t) => {
  const result = runRelease(t, { invalidMetadata: true })
  assert.equal(result.status, 1, result.stdout + result.stderr)
  assert.match(result.stderr, /picea-lab.publish must be false/)
  assert.deepEqual(result.packageArgs, [])
})

for (const [failure, status] of [["metadataFails", 41], ["gitFails", 43]]) {
  test(`${failure} propagates before packaging`, (t) => {
    const result = runRelease(t, { [failure]: true })
    assert.equal(result.status, status, result.stdout + result.stderr)
    assert.deepEqual(result.packageArgs, [])
  })
}

for (const [index, name] of ["picea", "picea-macro-tools"].entries()) {
  test(`${name} package failure is not swallowed`, (t) => {
    const result = runRelease(t, { dirty: true, failPackage: name })
    assert.equal(result.status, 42, result.stdout + result.stderr)
    assert.equal(result.packageArgs.length, index + 1)
    assert.equal(result.packageArgs.at(-1)[2], name)
  })
}
