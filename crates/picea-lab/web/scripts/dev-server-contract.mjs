import assert from "node:assert/strict";
import { spawn } from "node:child_process";
import fs from "node:fs/promises";
import net from "node:net";
import os from "node:os";
import path from "node:path";
import { fileURLToPath } from "node:url";

const host = "127.0.0.1";
const scriptDir = path.dirname(fileURLToPath(import.meta.url));
const repoRoot = path.resolve(scriptDir, "../../../..");

async function bindPort(port) {
  return await new Promise((resolve) => {
    const server = net.createServer();
    server.once("error", () => resolve(null));
    server.listen(port, host, () => resolve(server));
  });
}

function closeServer(server) {
  return new Promise((resolve, reject) => {
    server.close((error) => {
      if (error) {
        reject(error);
      } else {
        resolve();
      }
    });
  });
}

async function findAdjacentPorts() {
  for (let offset = 0; offset < 200; offset += 1) {
    const basePort = 42000 + offset * 2;
    const occupied = await bindPort(basePort);
    if (!occupied) {
      continue;
    }

    const next = await bindPort(basePort + 1);
    if (next) {
      await closeServer(next);
      return { occupied, basePort, fallbackPort: basePort + 1 };
    }

    await closeServer(occupied);
  }

  throw new Error("could not find adjacent free ports for dev-server contract");
}

async function main() {
  await assertFullStackApiFallback();
  await assertWebUiPortFallback();
}

async function assertFullStackApiFallback() {
  const { occupied, basePort, fallbackPort } = await findAdjacentPorts();
  const tempDir = await fs.mkdtemp(path.join(os.tmpdir(), "picea-lab-dev-contract-"));
  const logPath = path.join(tempDir, "rtk-log.jsonl");
  const fakeRtkPath = path.join(tempDir, "rtk");
  await fs.writeFile(
    fakeRtkPath,
    `#!/usr/bin/env node
const fs = require("node:fs");
const logPath = process.env.CONTRACT_LOG;
let args = process.argv.slice(2);
if (args[0] === "proxy") args = args.slice(1);
const command = args[0];
const append = (record) => fs.appendFileSync(logPath, JSON.stringify(record) + "\\n");
process.on("SIGTERM", () => process.exit(0));

if (command === "cargo") {
  const bindIndex = args.indexOf("--bind");
  append({ kind: "cargo", bind: bindIndex === -1 ? null : args[bindIndex + 1] });
  setInterval(() => {}, 1000);
} else if (command === "curl") {
  append({ kind: "curl", url: args.find((arg) => arg.startsWith("http://")) ?? null });
  process.exit(0);
} else if (command === "npm") {
  append({ kind: "npm", apiBase: process.env.VITE_PICEA_LAB_API_BASE ?? null, args });
  process.exit(0);
} else {
  append({ kind: "other", args });
  process.exit(0);
}
`,
  );
  await fs.chmod(fakeRtkPath, 0o755);

  try {
    const output = await runCommand("just", ["picea-lab-web"], {
      cwd: repoRoot,
      env: {
        ...process.env,
        CI: "1",
        NO_COLOR: "1",
        CONTRACT_LOG: logPath,
        PATH: `${tempDir}${path.delimiter}${process.env.PATH}`,
        PICEA_LAB_BIND: `${host}:${basePort}`,
        PICEA_LAB_WEB_HOST: host,
        PICEA_LAB_WEB_PORT: String(fallbackPort + 100),
      },
      timeoutMs: 20_000,
    });
    const records = (await fs.readFile(logPath, "utf8"))
      .trim()
      .split("\n")
      .filter(Boolean)
      .map((line) => JSON.parse(line));
    const cargo = records.find((record) => record.kind === "cargo");
    const curl = records.find((record) => record.kind === "curl");
    const npm = records.find((record) => record.kind === "npm");

    assert.equal(cargo?.bind, `${host}:${fallbackPort}`, output);
    assert.equal(curl?.url, `http://${host}:${fallbackPort}/api/scenarios`, output);
    assert.equal(npm?.apiBase, `http://${host}:${fallbackPort}`, output);
  } finally {
    await closeServer(occupied);
    await fs.rm(tempDir, { recursive: true, force: true });
  }
}

async function assertWebUiPortFallback() {
  const { occupied, basePort, fallbackPort } = await findAdjacentPorts();
  const expectedUrl = `http://${host}:${fallbackPort}/`;

  let output = "";
  const child = spawn("just", ["picea-lab-web-ui"], {
    cwd: repoRoot,
    detached: true,
    env: {
      ...process.env,
      CI: "1",
      NO_COLOR: "1",
      PICEA_LAB_WEB_HOST: host,
      PICEA_LAB_WEB_PORT: String(basePort),
      VITE_PICEA_LAB_API_BASE: `http://${host}:8080`,
    },
    stdio: ["ignore", "pipe", "pipe"],
  });

  const stopChild = () => {
    if (child.pid && !child.killed) {
      try {
        process.kill(-child.pid, "SIGTERM");
      } catch {
        child.kill("SIGTERM");
      }
    }
  };

  try {
    await new Promise((resolve, reject) => {
      const timeout = setTimeout(() => {
        stopChild();
        reject(new Error(`timed out waiting for Vite fallback to ${expectedUrl}\n${output}`));
      }, 20_000);

      const onData = (chunk) => {
        output += chunk.toString();
        if (output.includes(expectedUrl)) {
          clearTimeout(timeout);
          stopChild();
          resolve();
        }
      };

      child.stdout.on("data", onData);
      child.stderr.on("data", onData);
      child.once("error", (error) => {
        clearTimeout(timeout);
        reject(error);
      });
      child.once("exit", (code, signal) => {
        if (!output.includes(expectedUrl)) {
          clearTimeout(timeout);
          reject(new Error(`dev server exited before port fallback: code=${code} signal=${signal}\n${output}`));
        }
      });
    });
  } finally {
    stopChild();
    await closeServer(occupied);
  }

  assert.match(output, new RegExp(`http://${host}:${fallbackPort}/`));
}

function runCommand(command, args, { cwd, env, timeoutMs }) {
  return new Promise((resolve, reject) => {
    let output = "";
    const child = spawn(command, args, {
      cwd,
      env,
      stdio: ["ignore", "pipe", "pipe"],
    });
    const timeout = setTimeout(() => {
      child.kill("SIGTERM");
      reject(new Error(`timed out running ${command} ${args.join(" ")}\n${output}`));
    }, timeoutMs);
    const onData = (chunk) => {
      output += chunk.toString();
    };

    child.stdout.on("data", onData);
    child.stderr.on("data", onData);
    child.once("error", (error) => {
      clearTimeout(timeout);
      reject(error);
    });
    child.once("exit", (code, signal) => {
      clearTimeout(timeout);
      if (code === 0) {
        resolve(output);
      } else {
        reject(new Error(`${command} ${args.join(" ")} failed: code=${code} signal=${signal}\n${output}`));
      }
    });
  });
}

await main();
