const { spawn } = require("node:child_process");

const env = {
  ...process.env,
  MUSICOS_SYNC_TARGET:
    process.env.MUSICOS_SYNC_TARGET || "http://127.0.0.1:18080",
  MUSICOS_RENDERER_URL: process.env.MUSICOS_RENDERER_URL || "http://127.0.0.1:5173"
};

const npmCommand = process.platform === "win32" ? "npm.cmd" : "npm";
const vite = spawn(npmCommand, ["run", "dev"], {
  stdio: "inherit",
  env
});

const electron = spawn(npmCommand, ["run", "sync:electron"], {
  stdio: "inherit",
  env
});

function shutdown() {
  vite.kill();
  electron.kill();
}

process.on("SIGINT", shutdown);
process.on("SIGTERM", shutdown);

electron.on("exit", (code, signal) => {
  vite.kill();
  if (signal) {
    process.kill(process.pid, signal);
    return;
  }
  process.exit(code || 0);
});
