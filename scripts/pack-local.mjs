#!/usr/bin/env node

/**
 * Local packaging ladder for TPA CoWork / Hope Agent.
 *
 * Modes (cheapest → heaviest):
 *   pnpm pack:local              compile only (release-fast, no NSIS)
 *   pnpm pack:local:bundle       local installer (release, LTO off, NSIS)
 *   pnpm pack:local:ship         ship-like installer (release profile defaults, NSIS)
 *
 * Flags:
 *   --skip-frontend              reuse dist/ when frontend did not change
 *   --skip-host                  reuse src-tauri/resources/browser-host
 *   --skip-eval-sidecar          reuse src-tauri/binaries/hope-agent-eval-*
 *   --skip-venv                  reuse agent-venv.zip (bundle/ship only; compile skips by default)
 *   --jobs N                     cap cargo parallelism (helps 16GB machines)
 *   --help                       print usage
 *
 * Migration / daily tip: prefer pack:local + skip flags; run bundle once at the end.
 * pack-local owns prepare steps and clears tauri beforeBuildCommand so work is not doubled.
 */

import { copyFileSync, existsSync, mkdirSync, readdirSync, statSync, writeFileSync } from "node:fs"
import { cpus, freemem, totalmem } from "node:os"
import { dirname, join, resolve } from "node:path"
import { spawnSync } from "node:child_process"
import { fileURLToPath } from "node:url"

const repoRoot = resolve(dirname(fileURLToPath(import.meta.url)), "..")
const args = process.argv.slice(2)
const isWindows = process.platform === "win32"

if (args.includes("--help") || args.includes("-h")) {
  printHelp()
  process.exit(0)
}

const ship = args.includes("--ship")
const bundle = ship || args.includes("--bundle")
const skipFrontend = args.includes("--skip-frontend")
const skipHost = args.includes("--skip-host")
const skipEvalSidecar = args.includes("--skip-eval-sidecar")
// Compile mode never needs the zip; bundle/ship need it unless skipped.
const skipVenv = args.includes("--skip-venv") || !bundle
const jobsIndex = args.indexOf("--jobs")
const jobsExplicit =
  jobsIndex >= 0 && /^\d+$/.test(args[jobsIndex + 1] ?? "")
    ? Number(args[jobsIndex + 1])
    : null

const targetIndex = args.indexOf("--target")
const targetExplicit =
  targetIndex >= 0 && args[targetIndex + 1] && !args[targetIndex + 1].startsWith("-")
    ? args[targetIndex + 1]
    : null

const hostBinaryName =
  (targetExplicit ? targetExplicit.includes("windows") : isWindows)
    ? "ha-browser-host.exe"
    : "ha-browser-host"
const hostResourcePath = join(
  repoRoot,
  "src-tauri",
  "resources",
  "browser-host",
  hostBinaryName,
)
const frontendDistIndex = join(repoRoot, "dist", "index.html")
const agentVenvZip = join(repoRoot, "agent-venv.zip")
const binariesDir = join(repoRoot, "src-tauri", "binaries")

function printHelp() {
  console.log(`Usage: node scripts/pack-local.mjs [--bundle|--ship] [flags]

Modes:
  (default)              Compile main app with release-fast (no NSIS)
  --bundle               Build NSIS installer; release LTO forced off
  --ship                 Build NSIS installer with default release profile

Flags:
  --skip-frontend        Reuse dist/ (requires dist/index.html)
  --skip-host            Reuse browser-host resource binary
  --skip-eval-sidecar    Reuse hope-agent-eval externalBin (skips fat LTO)
  --skip-venv            Reuse agent-venv.zip (bundle/ship)
  --jobs N               Cap CARGO_BUILD_JOBS (auto-capped on low free RAM)
  --help, -h             Show this help

Examples:
  pnpm pack:local
  pnpm pack:local -- --skip-frontend --skip-host --skip-eval-sidecar
  pnpm pack:local:bundle -- --skip-eval-sidecar --jobs 2
  pnpm pack:local:ship
`)
}

function log(message) {
  console.log(`[pack-local] ${message}`)
}

function fail(message, code = 1) {
  console.error(`[pack-local] ${message}`)
  process.exit(code)
}

function run(command, commandArgs, env = process.env) {
  log(`$ ${command} ${commandArgs.join(" ")}`)
  const result = spawnSync(command, commandArgs, {
    cwd: repoRoot,
    env,
    stdio: "inherit",
    shell: isWindows,
  })
  if (result.error) fail(result.error.message)
  if (result.status !== 0) fail(`command exited with ${result.status}`, result.status ?? 1)
}

function pnpm(commandArgs, env) {
  run(isWindows ? "pnpm.cmd" : "pnpm", commandArgs, env)
}

function resolveJobs() {
  if (jobsExplicit !== null) return jobsExplicit
  if (process.env.CARGO_BUILD_JOBS) return null

  const freeGb = freemem() / 1024 ** 3
  const totalGb = totalmem() / 1024 ** 3
  const cores = cpus().length
  // Two release rustc processes easily exceed free RAM on 16GB hosts.
  // Cap parallelism so we swap less and finish sooner.
  let auto = null
  if (freeGb < 3) auto = Math.min(2, cores)
  else if (freeGb < 5) auto = Math.min(3, cores)
  else if (totalGb <= 18 && freeGb < 7) auto = Math.min(4, cores)

  if (auto !== null) {
    log(
      `auto CARGO_BUILD_JOBS=${auto} (free RAM ${freeGb.toFixed(1)} GB / total ${totalGb.toFixed(1)} GB; pass --jobs to override)`,
    )
  }
  return auto
}

function listEvalSidecars() {
  if (!existsSync(binariesDir)) return []
  return readdirSync(binariesDir).filter((name) => {
    if (!name.startsWith("hope-agent-eval")) return false
    try {
      return statSync(join(binariesDir, name)).isFile()
    } catch {
      return false
    }
  })
}

function requireExisting(label, path, hint) {
  if (existsSync(path)) return
  fail(`${label} missing at ${path}. ${hint}`)
}

function prepareHost(env, profile) {
  if (skipHost) {
    requireExisting(
      "browser-host",
      hostResourcePath,
      "Build once without --skip-host, or run pnpm prepare:browser-host.",
    )
    log(`skip browser-host (reusing ${hostResourcePath})`)
    return
  }
  if (profile !== "release-fast") {
    pnpm(["prepare:browser-host"], env)
    return
  }
  run("cargo", ["build", "-p", "ha-browser-host", "--profile", profile, "--locked"], env)
  const source = join(repoRoot, "target", profile, hostBinaryName)
  requireExisting("browser-host build output", source, "cargo build -p ha-browser-host failed to produce binary.")
  const destinationDir = join(repoRoot, "src-tauri", "resources", "browser-host")
  mkdirSync(destinationDir, { recursive: true })
  copyFileSync(source, join(destinationDir, hostBinaryName))
  log(`copied browser-host -> ${join(destinationDir, hostBinaryName)}`)
}

function prepareEvalSidecar(env) {
  if (skipEvalSidecar) {
    const sidecars = listEvalSidecars()
    if (sidecars.length === 0) {
      fail(
        "eval sidecar missing under src-tauri/binaries/. Build once with pnpm prepare:eval-sidecar, or drop --skip-eval-sidecar.",
      )
    }
    log(`skip eval-sidecar (reusing ${sidecars.join(", ")})`)
    return
  }
  pnpm(["prepare:eval-sidecar"], env)
}

function prepareFrontend(env) {
  if (skipFrontend) {
    requireExisting(
      "frontend dist",
      frontendDistIndex,
      "Run pnpm build once, or drop --skip-frontend.",
    )
    log(`skip frontend (reusing ${frontendDistIndex})`)
    return
  }
  pnpm(["build"], env)
}

function prepareVenv(env) {
  if (skipVenv) {
    if (bundle) {
      requireExisting(
        "agent-venv.zip",
        agentVenvZip,
        "Run pnpm prepare:zip-venv, or drop --skip-venv.",
      )
      log(`skip agent-venv (reusing ${agentVenvZip})`)
    }
    return
  }
  pnpm(["prepare:zip-venv"], env)
}

function artifact(label, path) {
  if (!existsSync(path)) {
    log(`${label}: missing (${path})`)
    return
  }
  const stat = statSync(path)
  log(
    `${label}: ${stat.isDirectory() ? "present" : `${(stat.size / 1024 / 1024).toFixed(1)} MB`} (${path})`,
  )
}

function report(profile) {
  const executable = isWindows ? "tpa-cowork.exe" : "tpa-cowork"
  artifact("main executable", join(repoRoot, "target", profile, executable))
  artifact("browser host", join(repoRoot, "src-tauri", "resources", "browser-host"))
  artifact("eval sidecar", binariesDir)
  artifact("embedded extension source", join(repoRoot, "extensions", "chrome"))
  artifact("embedded skills source", join(repoRoot, "skills"))
  artifact("agent-venv archive", agentVenvZip)
  const nsisDir = join(repoRoot, "target", "release", "bundle", "nsis")
  if (existsSync(nsisDir)) {
    const installers = readdirSync(nsisDir).filter((name) => name.endsWith(".exe"))
    log(`NSIS installers: ${installers.length ? installers.join(", ") : "none"}`)
  }
}

function modeLabel() {
  if (ship) return "ship"
  if (bundle) return "bundle"
  return "compile"
}

const env = { ...process.env }
if (targetExplicit) {
  env.TAURI_ENV_TARGET_TRIPLE = targetExplicit
  env.CARGO_BUILD_TARGET = targetExplicit
  env.HA_BROWSER_HOST_TARGET = targetExplicit
  env.HA_EVAL_SIDECAR_TARGET = targetExplicit
}
const jobs = resolveJobs()
if (jobs !== null) env.CARGO_BUILD_JOBS = String(jobs)
if (bundle && !ship) {
  env.CARGO_PROFILE_RELEASE_LTO = "false"
  env.CARGO_PROFILE_RELEASE_CODEGEN_UNITS = "16"
}

const startedAt = Date.now()
log(
  `mode=${modeLabel()} target=${targetExplicit || "default"} cores=${cpus().length} jobs=${env.CARGO_BUILD_JOBS ?? "cargo-default"} skips={frontend:${skipFrontend},host:${skipHost},eval:${skipEvalSidecar},venv:${skipVenv}}`,
)

const isTargetWindows = targetExplicit ? targetExplicit.includes("windows") : isWindows
const bundleTargetFormat = isTargetWindows ? "nsis" : "deb,appimage"

if (bundle) {
  // Own the prepare pipeline so --skip-* actually works. tauri.conf.json
  // beforeBuildCommand would otherwise re-run host/sidecar/venv/frontend.
  prepareVenv(env)
  prepareHost(env, "release")
  prepareEvalSidecar(env)
  prepareFrontend(env)
  // Write config to a file so Windows shell:true spawn does not mangle JSON quotes.
  const tauriConfigPath = join(repoRoot, ".scratch", "pack-local-tauri-config.json")
  mkdirSync(dirname(tauriConfigPath), { recursive: true })
  const tauriConfig = { build: { beforeBuildCommand: "" } }
  if (!process.env.TAURI_SIGNING_PRIVATE_KEY) {
    tauriConfig.bundle = { createUpdaterArtifacts: false }
  }
  writeFileSync(
    tauriConfigPath,
    JSON.stringify(tauriConfig, null, 2) + "\n",
    "utf8",
  )
  const tauriBuildArgs = ["tauri", "build", "--bundles", bundleTargetFormat, "--config", tauriConfigPath]
  if (targetExplicit) {
    tauriBuildArgs.push("--target", targetExplicit)
  }
  pnpm(tauriBuildArgs, env)
  report("release")
} else {
  const profile = "release-fast"
  log(`compile mode on ${cpus().length} logical CPUs (profile ${profile})`)
  prepareHost(env, profile)
  prepareEvalSidecar(env)
  prepareFrontend(env)
  const cargoBuildArgs = [
    "build",
    "-p",
    "tpa-cowork",
    "--profile",
    profile,
    "--features",
    "tauri/custom-protocol",
    "--locked",
  ]
  if (targetExplicit) {
    cargoBuildArgs.push("--target", targetExplicit)
  }
  run("cargo", cargoBuildArgs, env)
  report(profile)
}

log(`elapsed ${((Date.now() - startedAt) / 1000).toFixed(1)}s`)
