#!/usr/bin/env node
// Patches the dev-only Electron.app bundle's Info.plist so macOS shows "Bitvue" (not "Electron")
// in the bold system menu bar during `npm run electron` / `scripts/dev.sh`.
//
// Why this is needed at all: macOS reads the bold menu-bar app name from the *running bundle's*
// Info.plist (CFBundleName/CFBundleDisplayName) at process launch, before any JS runs -- so
// `app.setName("Bitvue")` in electron/main.ts (which only changes what Electron's own APIs like
// app.getName() return) can never fix this on its own. A real `electron-builder` package already
// gets this right for free (productName: "Bitvue" in package.json controls the packaged bundle's
// Info.plist) -- this script exists only to fix the *dev* experience, which runs the raw
// node_modules/electron/dist/Electron.app.
//
// Re-run safe: npm reinstalls node_modules/electron on every `npm install`, overwriting this
// patch, so it's wired as a `postinstall` script rather than a one-time manual fix.

import { execFileSync } from "node:child_process";
import { existsSync } from "node:fs";
import { fileURLToPath } from "node:url";
import path from "node:path";

if (process.platform !== "darwin") {
  process.exit(0);
}

const here = path.dirname(fileURLToPath(import.meta.url));
const plistPath = path.join(
  here,
  "..",
  "node_modules",
  "electron",
  "dist",
  "Electron.app",
  "Contents",
  "Info.plist",
);

if (!existsSync(plistPath)) {
  // electron not installed yet (e.g. postinstall running before its own package finishes) --
  // nothing to patch, not an error.
  process.exit(0);
}

const APP_NAME = "Bitvue";

for (const key of ["CFBundleName", "CFBundleDisplayName"]) {
  execFileSync("/usr/libexec/PlistBuddy", [
    "-c",
    `Set :${key} ${APP_NAME}`,
    plistPath,
  ]);
}

console.log(`[patch-electron-app-name] Set ${plistPath} bundle name to "${APP_NAME}".`);

// The menu bar (a live query against the running process) picks up the Info.plist edit above
// immediately -- but the Dock's label/tooltip comes from Launch Services' own cached copy of
// CFBundleName, keyed by bundle path, which does NOT refresh just because the file on disk
// changed. Force a re-scan of just this bundle so the Dock stops showing the stale "Electron"
// name too.
const bundlePath = path.join(here, "..", "node_modules", "electron", "dist", "Electron.app");
const lsregister =
  "/System/Library/Frameworks/CoreServices.framework/Versions/A/Frameworks/LaunchServices.framework/Versions/A/Support/lsregister";
if (existsSync(lsregister)) {
  execFileSync(lsregister, ["-f", bundlePath]);
  console.log(`[patch-electron-app-name] Re-registered ${bundlePath} with Launch Services.`);
}
