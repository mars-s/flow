#!/usr/bin/env bun
//
// The Tauri analogue of Flow's own scripts/dev.ts: rebuilds and relaunches
// a real, Spotlight-discoverable "Flow Debug.app" bundle on every source
// change, instead of the bare unbundled binary `bun run tauri dev` runs
// from target/debug/. Not the same kind of "live" as Vite's own HMR (that
// still runs underneath this via beforeDevCommand for the frontend) — this
// is "rebuild the whole app and relaunch it," the same full-cycle model
// scripts/dev.ts already uses for Flow itself, chosen for consistency
// rather than trying to hot-swap a running Tauri window.

import { $ } from "bun";
import { watch, type FSWatcher } from "node:fs";
import { join, resolve } from "node:path";

const root = resolve(import.meta.dir, "..");
const appName = "Flow Debug";
const appPath = join(root, "src-tauri/target/debug/bundle/macos", `${appName}.app`);
// The bundle's own CFBundleExecutable stays the Cargo package name
// (tauri.conf.json's `bundle.mainBinaryName` isn't a real schema field in
// this Tauri version — checked, not guessed, after `pkill -x "Flow Debug"`
// silently matched nothing and hung the whole watcher waiting for a
// process that was never actually going to exit) — `pkill` has to target
// the real running process name, not the bundle's display name.
const processName = "flow-tauri-prototype";
// The release build (installed at /Applications/Flow.app once shipped)
// shares this exact same binary name — Cargo's package name never
// changes with tauri.conf.json's productName override. A bare
// `pkill -x processName` matched *both* bundles and silently killed the
// user's real installed Flow.app on every dev rebuild — a real incident,
// not a hypothetical. `stopApp` below matches on the full executable
// path instead (`pkill -f`), scoped to this debug bundle specifically.
const debugExecutablePath = join(appPath, "Contents/MacOS", processName);
// A single recursive watch on the repo root, filtered by hand in the
// callback below — not one `fs.watch` per directory (`src`, `src-tauri/
// src`, ...), which is what this originally tried and is what actually
// caused a real, confirmed rebuild loop: instrumented directly (added a
// temporary console.log to every watcher callback and read the raw
// events), the "src" watcher was firing for changes under
// "src-tauri/target/..." — cargo's own build output — with the reported
// filename missing its "src" prefix entirely (e.g. "-tauri/target/debug/
// ...", i.e. "src-tauri/target/..." with a naive `slice(3)` applied). That
// pattern means "src-tauri" started with "src", the fs.watch layer under
// Bun did a prefix match without checking for a path-separator boundary
// after it, and cargo rewriting hundreds of files under target/ on every
// build kept re-triggering the exact watcher meant to catch *source*
// changes — a true infinite loop, not a one-off. (icon.icns's own mtime
// changing on every bundle step was a second, smaller real issue caught
// the same way, folded into this same fix rather than kept as two
// separate directory watches.) One watcher with correct boundary-checked
// prefixes sidesteps trusting that scoping again for a second directory.
const sourcePrefixes = ["src/", "src-tauri/src/"];
const watchedFiles = new Set([
  "src-tauri/Cargo.toml",
  "src-tauri/Cargo.lock",
  "src-tauri/tauri.conf.json",
  "package.json",
]);
const ignoredInfixes = ["/target/", "/node_modules/", "/dist/", "/.git/"];

function isWatchedChange(filename: string): boolean {
  if (ignoredInfixes.some((infix) => filename.includes(infix))) return false;
  if (watchedFiles.has(filename)) return true;
  return sourcePrefixes.some((prefix) => filename.startsWith(prefix));
}
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
  console.log("[flow-debug] Building Flow Debug.app...");
  // tauri build --debug runs a real (non-minified) frontend build plus an
  // unoptimized Rust build, then bundles both into a proper .app — the
  // full bundler, not the bare `cargo run` binary `tauri dev` launches,
  // which is what actually makes this discoverable from Spotlight at all.
  const result = await $`bun run tauri build --debug`.nothrow();
  if (result.exitCode !== 0) {
    console.error("[flow-debug] Build failed; keeping the current app open.");
    return false;
  }
  // Nudges Spotlight to index the freshly-(re)built bundle immediately
  // rather than waiting for its own periodic scan to notice the new mtime.
  await $`mdimport ${appPath}`.quiet().nothrow();
  return true;
}

async function stopApp(): Promise<void> {
  const waiter = app;
  app = undefined;
  await $`pkill -TERM -f ${debugExecutablePath}`.quiet().nothrow();
  if (waiter?.exitCode === null) {
    await waiter.exited;
  }
}

function launchApp(): ReturnType<typeof Bun.spawn> {
  console.log(`[flow-debug] Launching ${appPath}`);
  const launchedApp = Bun.spawn(["open", "-n", "-W", appPath], {
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
    console.log("[flow-debug] App exited; stopping the watcher.");
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
  console.error("[flow-debug] File watcher failed:", error);
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
  const watcher = watch(root, { recursive: true }, (_eventType, filename) => {
    if (filename && isWatchedChange(filename.toString())) scheduleBuild();
  });
  watcher.on("error", reportWatcherError);
  watchers.push(watcher);
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
  console.log("[flow-debug] Stopping watcher and app...");
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

console.log("[flow-debug] Watching for source changes.");
