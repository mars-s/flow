#!/usr/bin/env bun

import { $ } from "bun";
import { watch, type FSWatcher } from "node:fs";
import { join, resolve } from "node:path";

const root = resolve(import.meta.dir, "..");
const isMacOS = process.platform === "darwin";
const appName = "Flow Dev";
const targetDir = resolve(root, process.env.CARGO_TARGET_DIR || "target");
const appPath = isMacOS
  ? join(targetDir, "debug/Flow Dev.app")
  : join(targetDir, "debug/flow");
const watchedDirectories = ["src", "crates", "assets", "resources", "locales"];
const watchedFiles = ["Cargo.toml", "Cargo.lock", "build.rs"];
const rebuildDebounceMs = 1_000;

$.cwd(root);

let app: ReturnType<typeof Bun.spawn> | undefined;
let stopping = false;
let building = false;
let queuedBuild = false;
let debouncedBuild = false;
let rebuildTimer: ReturnType<typeof setTimeout> | undefined;
const watchers: FSWatcher[] = [];

async function build(): Promise<boolean> {
  console.log(`[flow-dev] Building ${isMacOS ? "app bundle" : "app"}...`);
  const result = isMacOS
    ? await $`${join(root, "scripts/bundle.sh")} debug`.nothrow()
    : await $`cargo build --package flow --bin flow`.nothrow();
  if (result.exitCode !== 0) {
    console.error("[flow-dev] Build failed; keeping the current app open.");
    return false;
  }
  return true;
}

async function stopApp(): Promise<void> {
  const waiter = app;
  app = undefined;
  if (isMacOS) {
    await $`pkill -TERM -x ${appName}`.quiet().nothrow();
  } else if (waiter?.exitCode === null) {
    waiter.kill("SIGTERM");
  }
  if (waiter?.exitCode === null) {
    await waiter.exited;
  }
}

function launchApp(): ReturnType<typeof Bun.spawn> {
  console.log(`[flow-dev] Launching ${appPath}`);
  const command = isMacOS ? ["open", "-n", "-W", appPath] : [appPath];
  const launchedApp = Bun.spawn(command, {
    cwd: root,
    stdout: "inherit",
    stderr: "inherit",
  });
  void launchedApp.exited.then((exitCode) => {
    if (stopping || app !== launchedApp) return;
    app = undefined;
    stopping = true;
    closeWatchers();
    clearRebuildTimer();
    console.log("[flow-dev] App exited; stopping the watcher.");
    process.exitCode = exitCode;
  });
  return launchedApp;
}

function clearRebuildTimer(): void {
  if (rebuildTimer === undefined) return;
  clearTimeout(rebuildTimer);
  rebuildTimer = undefined;
}

function closeWatchers(): void {
  for (const watcher of watchers.splice(0)) watcher.close();
}

function reportWatcherError(error: Error): void {
  console.error("[flow-dev] File watcher failed:", error);
  process.exitCode = 1;
  void cleanup();
}

function scheduleBuild(): void {
  if (stopping) return;
  debouncedBuild = true;
  clearRebuildTimer();
  rebuildTimer = setTimeout(() => {
    rebuildTimer = undefined;
    if (debouncedBuild) {
      queuedBuild = true;
      debouncedBuild = false;
    }
    void drainBuildQueue();
  }, rebuildDebounceMs);
}

function startWatchers(): void {
  for (const directory of watchedDirectories) {
    const watcher = watch(join(root, directory), { recursive: true }, () => scheduleBuild());
    watcher.on("error", reportWatcherError);
    watchers.push(watcher);
  }

  const rootWatcher = watch(root, (_eventType, filename) => {
    if (filename && watchedFiles.includes(filename.toString())) scheduleBuild();
  });
  rootWatcher.on("error", reportWatcherError);
  watchers.push(rootWatcher);
}

async function drainBuildQueue(): Promise<void> {
  if (building || stopping) return;
  building = true;
  try {
    while (queuedBuild && !stopping) {
      queuedBuild = false;
      if (!(await build()) || stopping) continue;
      await stopApp();
      if (!stopping) app = launchApp();
    }
  } finally {
    building = false;
    if (queuedBuild && !stopping) void drainBuildQueue();
  }
}

async function cleanup(): Promise<void> {
  if (stopping) return;
  stopping = true;
  console.log("[flow-dev] Stopping watcher and app...");
  closeWatchers();
  clearRebuildTimer();
  await stopApp();
}

process.on("SIGINT", () => void cleanup());
process.on("SIGTERM", () => void cleanup());

startWatchers();
building = true;
const initialBuildSucceeded = await build();
building = false;
if (!initialBuildSucceeded) {
  closeWatchers();
  process.exit(1);
}

await stopApp();
app = launchApp();

console.log("[flow-dev] Watching for source changes.");
