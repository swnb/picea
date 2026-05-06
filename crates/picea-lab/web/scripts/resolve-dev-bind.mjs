import net from "node:net";

const bind = process.argv[2];
const separator = bind?.lastIndexOf(":") ?? -1;

if (!bind || separator <= 0 || separator === bind.length - 1) {
  throw new Error(`expected bind address in host:port form, got ${bind ?? "<missing>"}`);
}

const host = bind.slice(0, separator);
const preferredPort = Number(bind.slice(separator + 1));

if (!Number.isInteger(preferredPort) || preferredPort <= 0 || preferredPort > 65535) {
  throw new Error(`expected TCP port in host:port bind address, got ${bind}`);
}

async function canBind(port) {
  return await new Promise((resolve) => {
    const server = net.createServer();
    server.once("error", () => resolve(false));
    server.listen({ host, port }, () => {
      server.close((error) => resolve(!error));
    });
  });
}

for (let port = preferredPort; port <= 65535 && port < preferredPort + 100; port += 1) {
  if (await canBind(port)) {
    process.stdout.write(`${host}:${port}`);
    process.exit(0);
  }
}

throw new Error(`no free picea-lab API port found from ${bind}`);
