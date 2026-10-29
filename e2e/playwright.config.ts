import { defineConfig, devices } from "@playwright/test";

// Runs against an already-running app: `make up` locally (the default URL), or
// a pull request preview in CI (BASE_URL). It never starts a server itself.
export default defineConfig({
  testDir: "tests",
  // The tests share one account and run in order, like a person clicking through.
  fullyParallel: false,
  workers: 1,
  forbidOnly: !!process.env.CI,
  reporter: [
    ["list"],
    ["json", { outputFile: "results/results.json" }],
    ["html", { outputFolder: "results/report", open: "never" }],
  ],
  outputDir: "results/artifacts",
  use: {
    baseURL: process.env.BASE_URL ?? "http://localhost:3000",
    screenshot: "only-on-failure",
    trace: "retain-on-failure",
  },
  projects: [{ name: "chromium", use: { ...devices["Desktop Chrome"] } }],
});
