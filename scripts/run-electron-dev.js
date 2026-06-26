const { spawn } = require("node:child_process");

const env = {
  ...process.env,
  MUSICOS_SYNC_TARGET:
    process.env.MUSICOS_SYNC_TARGET || "http://127.0.0.1:18080"
};

const npmCommand = process.platform === "win32" ? "npm.cmd" : "npm";
const child = spawn(npmCommand, ["run", "sync:electron"], {
  stdio: "inherit",
  env
});

child.on("exit", (code, signal) => {
  if (signal) {
    process.kill(process.pid, signal);
    return;
  }
  process.exit(code || 0);
});
