import {fileURLToPath} from "node:url";
import { defineConfig } from "@playwright/test";
export default defineConfig({
  webServer: {
    command: "npm run dev",
    cwd:fileURLToPath(new URL("..",import.meta.url)),
    url: "http://127.0.0.1:1420",
    reuseExistingServer: !process.env.CI,
    timeout: 30000,
  },
  testDir: "./tests",
  testMatch: "**/*.spec.ts",
  fullyParallel: false,
  workers: 1,
  timeout: 30000,
  use: {
    baseURL: "http://127.0.0.1:1420",
    viewport: { width: 1280, height: 800 },
    screenshot: "only-on-failure",
    trace: "retain-on-failure",
  },
  outputDir: "../out/rebuild-qa/playwright",
  reporter: [
    ["list"],
    ["json", { outputFile: "../out/rebuild-qa/playwright-results.json" }],
  ],
});
