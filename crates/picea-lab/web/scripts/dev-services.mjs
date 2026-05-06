import { execFile, spawn } from "node:child_process";
import fs from "node:fs";
import fsp from "node:fs/promises";
import net from "node:net";
import os from "node:os";
import path from "node:path";
import { promisify } from "node:util";
import { fileURLToPath } from "node:url";

const execFileAsync = promisify(execFile);
const scriptDir = path.dirname(fileURLToPath(import.meta.url));
const repoRoot = path.resolve(scriptDir, "../../../..");
const command = process.argv[2];

const defaultApiBind = "127.0.0.1:8080";
const defaultWebHost = "127.0.0.1";
const defaultWebPort = "5173";
const serviceDir = path.resolve(
  repoRoot,
  process.env.PICEA_LAB_SERVICE_DIR ?? "target/picea-lab-web",
);
const statePath = path.join(serviceDir, "services.json");
const apiLogPath = path.join(serviceDir, "api.log");
const uiLogPath = path.join(serviceDir, "ui.log");

async function main() {
  if (command === "start") {
    await startServices();
    return;
  }

  if (command === "manager") {
    await runManager();
    return;
  }

  if (command === "stop") {
    await stopServices();
    return;
  }

  throw new Error(`expected start or stop, got ${command ?? "<missing>"}`);
}

async function startServices() {
  await fsp.mkdir(serviceDir, { recursive: true });

  const existingState = await readState();
  if (existingState && (isManagedProcessRunning(existingState.managerPid) || isManagedProcessRunning(existingState.apiPid) || isManagedProcessRunning(existingState.uiPid))) {
    throw new Error(`picea-lab-web is already running from ${serviceDir}. Run: just picea-lab-web-stop`);
  }

  await fsp.rm(statePath, { force: true });
  const managerLogPath = path.join(serviceDir, "manager.log");
  await appendLogHeader(managerLogPath, "Starting picea-lab-web service manager");
  const manager = spawnDetached(process.execPath, [fileURLToPath(import.meta.url), "manager"], {
    env: process.env,
    logPath: managerLogPath,
  });

  for (let attempt = 0; attempt < 240; attempt += 1) {
    const state = await readState();
    if (state?.error) {
      throw new Error(state.error);
    }
    if (state?.ready) {
      console.log(`picea-lab API is ready at ${state.readyUrl}`);
      console.log(`Starting picea-lab-web from ${state.webUrl}`);
      console.log("picea-lab-web services started.");
      console.log(`API log: ${path.relative(repoRoot, state.apiLogPath)}`);
      console.log(`Web log: ${path.relative(repoRoot, state.uiLogPath)}`);
      console.log("Stop with: just picea-lab-web-stop");
      return;
    }
    if (!isManagedProcessRunning(manager.pid)) {
      throw new Error(`picea-lab-web service manager exited during startup. Log: ${managerLogPath}\n${tailLog(managerLogPath)}`);
    }
    await sleep(500);
  }

  throw new Error(`Timed out waiting for picea-lab-web service manager. Log: ${managerLogPath}\n${tailLog(managerLogPath)}`);
}

async function runManager() {
  await fsp.mkdir(serviceDir, { recursive: true });

  const existingState = await readState();
  if (existingState && (isManagedProcessRunning(existingState.apiPid) || isManagedProcessRunning(existingState.uiPid))) {
    throw new Error(`picea-lab-web is already running from ${serviceDir}. Run: just picea-lab-web-stop`);
  }

  const requestedApiBind = process.env.PICEA_LAB_BIND ?? defaultApiBind;
  const apiBind = await resolveBind(requestedApiBind);
  const apiBase = `http://${apiBind}`;
  const readyUrl = process.env.PICEA_LAB_READY_URL ?? `${apiBase}/api/scenarios`;
  const webHost = process.env.PICEA_LAB_WEB_HOST ?? defaultWebHost;
  const webPort = await resolvePort(webHost, Number(process.env.PICEA_LAB_WEB_PORT ?? defaultWebPort));
  const webApiBase = process.env.VITE_PICEA_LAB_API_BASE ?? apiBase;
  const startedAt = new Date().toISOString();
  const socketPath = path.join(serviceDir, "control.sock");
  await fsp.rm(socketPath, { force: true });

  await appendLogHeader(apiLogPath, `Starting picea-lab API on ${apiBind}`);
  const apiProcess = spawnDetached(
    "cargo",
    ["run", "-p", "picea-lab", "--", "serve", "--bind", apiBind],
    { logPath: apiLogPath },
  );

  let state = {
    apiPid: apiProcess.pid,
    apiBind,
    apiBase,
    apiLogPath,
    readyUrl,
    startedAt,
    managerPid: process.pid,
    socketPath,
  };
  await writeState(state);

  try {
    await waitForApiReady({ pid: apiProcess.pid, readyUrl, logPath: apiLogPath });
    const apiListenerPid = await findListeningPid(portFromBind(apiBind));

    await appendLogHeader(uiLogPath, `Starting picea-lab-web on ${webHost}:${webPort}`);
    console.log(`Starting picea-lab-web from http://${webHost}:${webPort}`);
    const uiProcess = spawnDetached(
      "npm",
      [
        "--prefix",
        "crates/picea-lab/web",
        "run",
        "dev",
        "--",
        "--host",
        webHost,
        "--port",
        String(webPort),
      ],
      {
        env: { ...process.env, VITE_PICEA_LAB_API_BASE: webApiBase },
        logPath: uiLogPath,
      },
    );

    const webUrl = `http://${webHost}:${webPort}`;
    await waitForWebReady({ pid: uiProcess.pid, webUrl, logPath: uiLogPath });
    const uiListenerPid = await findListeningPid(webPort);

    state = {
      ...state,
      apiListenerPid,
      uiPid: uiProcess.pid,
      uiListenerPid,
      webHost,
      webPort,
      webUrl,
      webApiBase,
      uiLogPath,
      managerPid: process.pid,
      socketPath,
      ready: true,
    };
    await writeState(state);
    await runControlServer(state);
  } catch (error) {
    await writeState({ ...state, error: error.message, ready: false });
    await stopState(state, { removeState: true });
    throw error;
  }
}

async function stopServices() {
  const state = await readState();
  if (!state) {
    console.log("picea-lab-web services: not running");
    return;
  }

  if (!state.socketPath) {
    console.log("picea-lab-web services: missing control socket; remove stale state after manual cleanup");
    return;
  }

  await requestStop(state.socketPath);
}

async function stopState(state, { removeState }) {
  await stopProcess("picea-lab-web UI", { pid: state.uiListenerPid, groupPid: state.uiPid });
  await stopProcess("picea-lab API", { pid: state.apiListenerPid, groupPid: state.apiPid });

  if (removeState) {
    await fsp.rm(statePath, { force: true });
  }
}

async function runControlServer(state) {
  const server = net.createServer((socket) => {
    socket.setEncoding("utf8");
    socket.once("data", async (data) => {
      if (data.trim() !== "stop") {
        socket.end("unknown command\n");
        return;
      }

      await stopState(state, { removeState: true });
      socket.end("stopped\n");
      server.close(() => process.exit(0));
    });
  });

  await new Promise((resolve, reject) => {
    server.once("error", reject);
    server.listen(state.socketPath, resolve);
  });

  const shutdown = async () => {
    await stopState(state, { removeState: true });
    process.exit(0);
  };
  process.once("SIGTERM", shutdown);
  process.once("SIGINT", shutdown);

  await new Promise(() => {});
}

async function requestStop(socketPath) {
  await new Promise((resolve, reject) => {
    const socket = net.createConnection(socketPath);
    let output = "";

    socket.setEncoding("utf8");
    socket.once("error", reject);
    socket.on("data", (chunk) => {
      output += chunk;
    });
    socket.once("end", () => {
      process.stdout.write(output);
      resolve();
    });
    socket.once("connect", () => {
      socket.write("stop\n");
    });
  });
}

async function stopProcess(label, { pid, groupPid }) {
  if (!pid && !groupPid) {
    console.log(`${label}: not running`);
    return;
  }

  if (!isPidRunning(pid) && !isManagedProcessRunning(groupPid)) {
    console.log(`${label}: not running`);
    return;
  }

  console.log(`${label}: stopping pid ${pid ?? groupPid}`);
  await signalManagedProcess({ pid, groupPid, signal: "SIGTERM" });

  for (let attempt = 0; attempt < 40; attempt += 1) {
    await sleep(250);
    if (!isPidRunning(pid) && !isManagedProcessRunning(groupPid)) {
      console.log(`${label}: stopped`);
      return;
    }
  }

  console.log(`${label}: forcing pid ${pid ?? groupPid}`);
  await signalManagedProcess({ pid, groupPid, signal: "SIGKILL" });
}

function spawnDetached(program, args, { env = process.env, logPath }) {
  const stdout = fs.openSync(logPath, "a");
  const stderr = fs.openSync(logPath, "a");
  const child = spawn(program, args, {
    cwd: repoRoot,
    detached: true,
    env,
    stdio: ["ignore", stdout, stderr],
  });
  child.unref();
  fs.closeSync(stdout);
  fs.closeSync(stderr);
  return child;
}

async function waitForApiReady({ pid, readyUrl, logPath }) {
  for (let attempt = 0; attempt < 120; attempt += 1) {
    if (await isUrlOk(readyUrl)) {
      console.log(`picea-lab API is ready at ${readyUrl}`);
      return;
    }

    if (!isManagedProcessRunning(pid)) {
      throw new Error(`picea-lab API exited before it became ready. Log: ${logPath}\n${tailLog(logPath)}`);
    }

    await sleep(500);
  }

  throw new Error(`Timed out waiting for ${readyUrl}. Log: ${logPath}\n${tailLog(logPath)}`);
}

async function waitForWebReady({ pid, webUrl, logPath }) {
  for (let attempt = 0; attempt < 120; attempt += 1) {
    if (await isUrlOk(webUrl)) {
      return;
    }

    if (!isManagedProcessRunning(pid)) {
      throw new Error(`picea-lab-web exited before it became ready. Log: ${logPath}\n${tailLog(logPath)}`);
    }

    await sleep(500);
  }

  throw new Error(`Timed out waiting for ${webUrl}. Log: ${logPath}\n${tailLog(logPath)}`);
}

async function resolveBind(bind) {
  const separator = bind.lastIndexOf(":");
  if (separator <= 0 || separator === bind.length - 1) {
    throw new Error(`expected bind address in host:port form, got ${bind}`);
  }

  const host = bind.slice(0, separator);
  const preferredPort = Number(bind.slice(separator + 1));
  if (!Number.isInteger(preferredPort) || preferredPort <= 0 || preferredPort > 65535) {
    throw new Error(`expected TCP port in host:port bind address, got ${bind}`);
  }

  for (let port = preferredPort; port <= 65535 && port < preferredPort + 100; port += 1) {
    if (await canBind(host, port)) {
      return `${host}:${port}`;
    }
  }

  throw new Error(`no free picea-lab API port found from ${bind}`);
}

async function resolvePort(host, preferredPort) {
  if (!Number.isInteger(preferredPort) || preferredPort <= 0 || preferredPort > 65535) {
    throw new Error(`expected TCP port, got ${preferredPort}`);
  }

  for (let port = preferredPort; port <= 65535 && port < preferredPort + 100; port += 1) {
    if (await canBind(host, port)) {
      return port;
    }
  }

  throw new Error(`no free picea-lab-web port found from ${host}:${preferredPort}`);
}

function portFromBind(bind) {
  const separator = bind.lastIndexOf(":");
  return Number(bind.slice(separator + 1));
}

async function canBind(host, port) {
  return await new Promise((resolve) => {
    const server = net.createServer();
    server.once("error", () => resolve(false));
    server.listen({ host, port }, () => {
      server.close((error) => resolve(!error));
    });
  });
}

async function isUrlOk(url) {
  const controller = new AbortController();
  const timeout = setTimeout(() => controller.abort(), 1000);

  try {
    const response = await fetch(url, { signal: controller.signal });
    return response.ok;
  } catch (error) {
    return error.code === "EPERM";
  } finally {
    clearTimeout(timeout);
  }
}

function isPidRunning(pid) {
  if (!pid) {
    return false;
  }

  try {
    process.kill(pid, 0);
    return true;
  } catch (error) {
    return error.code === "EPERM";
  }
}

function isProcessGroupRunning(pid) {
  if (!pid) {
    return false;
  }

  try {
    process.kill(-pid, 0);
    return true;
  } catch {
    return false;
  }
}

function isManagedProcessRunning(pid) {
  return isPidRunning(pid) || isProcessGroupRunning(pid);
}

async function signalManagedProcess({ pid, groupPid, signal }) {
  if (groupPid) {
    await sendSignal(-groupPid, signal, { ignorePermissionDenied: true });
    await sendSignal(groupPid, signal, { ignorePermissionDenied: true });
  }

  if (pid) {
    await sendSignal(pid, signal, { ignorePermissionDenied: false });
  }
}

async function sendSignal(target, signal, { ignorePermissionDenied }) {
  try {
    await execFileAsync("kill", [`-${signal.replace(/^SIG/, "")}`, String(target)]);
  } catch (error) {
    const text = `${error.stderr ?? ""}${error.message ?? ""}`;
    if (!text.includes("No such process") && !(ignorePermissionDenied && text.includes("Operation not permitted"))) {
      throw error;
    }
  }
}

async function findListeningPid(port) {
  try {
    const { stdout } = await execFileAsync("lsof", [`-tiTCP:${port}`, "-sTCP:LISTEN"]);
    const pid = Number(stdout.trim().split(/\s+/)[0]);
    return Number.isInteger(pid) ? pid : null;
  } catch {
    return null;
  }
}

async function readState() {
  try {
    return JSON.parse(await fsp.readFile(statePath, "utf8"));
  } catch (error) {
    if (error.code === "ENOENT") {
      return null;
    }
    throw error;
  }
}

async function writeState(state) {
  await fsp.writeFile(statePath, `${JSON.stringify(state, null, 2)}${os.EOL}`);
}

async function appendLogHeader(logPath, message) {
  await fsp.appendFile(logPath, `${os.EOL}[dev-services] ${new Date().toISOString()} ${message}${os.EOL}`);
}

function tailLog(logPath) {
  try {
    return fs.readFileSync(logPath, "utf8").trim().split(/\r?\n/).slice(-40).join(os.EOL);
  } catch {
    return "";
  }
}

function sleep(ms) {
  return new Promise((resolve) => setTimeout(resolve, ms));
}

await main();
