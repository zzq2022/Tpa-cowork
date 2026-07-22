import { cpSync, existsSync, mkdtempSync, readFileSync, realpathSync, rmSync } from "node:fs"
import { tmpdir } from "node:os"
import path from "node:path"
import { spawnSync } from "node:child_process"
import { fileURLToPath } from "node:url"

const repoRoot = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..")
const relocateSmoke = process.argv.includes("--relocate-smoke")
const runtimeArg = process.argv.find((arg) => !arg.startsWith("--"))
const runtimeDir = path.resolve(repoRoot, runtimeArg || "agent-venv")

if (process.platform !== "win32") {
  console.log("[verify-agent-venv] Non-Windows target; skipping portable runtime verification.")
  process.exit(0)
}

try {
  verifyRuntime(runtimeDir)
  if (relocateSmoke) verifyRelocatedRuntime(runtimeDir)
} catch (error) {
  console.error(`[verify-agent-venv] ${error instanceof Error ? error.message : String(error)}`)
  process.exit(1)
}

function verifyRuntime(root) {
  const scriptsDir = path.join(root, "Scripts")
  const required = [
    "python.exe",
    "python312.dll",
    "python312.zip",
    "python312._pth",
    path.join("Lib", "site-packages"),
  ]

  for (const relative of required) {
    const candidate = path.join(scriptsDir, relative)
    if (!existsSync(candidate)) {
      throw new Error(`portable runtime is missing ${path.relative(repoRoot, candidate)}`)
    }
  }

  const pthPath = path.join(scriptsDir, "python312._pth")
  const pth = readFileSync(pthPath, "utf8").replace(/\r\n/g, "\n")
  for (const line of ["python312.zip", ".", "Lib/site-packages", "import site"]) {
    if (!pth.split("\n").includes(line)) {
      throw new Error(`python312._pth is missing required line: ${line}`)
    }
  }

  const python = path.join(scriptsDir, "python.exe")
  const probe = [
    "import json, pathlib, sys",
    "import yaml, openpyxl, docx, pptx, pandas, numpy, PIL, pypdf, httpx, requests, rich",
    "print(json.dumps({'basePrefix': sys.base_prefix, 'executable': sys.executable, 'paths': sys.path}))",
  ].join("; ")
  const result = spawnSync(python, ["-I", "-c", probe], {
    cwd: scriptsDir,
    encoding: "utf8",
    env: minimalPythonEnv(scriptsDir),
  })
  if (result.status !== 0) {
    throw new Error(
      `bundled Python probe failed (${result.status ?? "spawn error"}): ${result.stderr || result.error || "unknown error"}`,
    )
  }

  const payload = JSON.parse(result.stdout.trim())
  const canonicalScripts = canonicalPath(scriptsDir)
  for (const [label, value] of [
    ["base prefix", payload.basePrefix],
    ["executable", path.dirname(payload.executable)],
  ]) {
    if (canonicalPath(value) !== canonicalScripts) {
      throw new Error(`${label} escapes bundled runtime: ${value}`)
    }
  }
  for (const entry of payload.paths) {
    if (entry && !isInside(canonicalScripts, canonicalPath(entry))) {
      throw new Error(`sys.path escapes bundled runtime: ${entry}`)
    }
  }

  console.log(`[verify-agent-venv] Portable runtime verified at ${root}`)
}

function verifyRelocatedRuntime(source) {
  const tempRoot = mkdtempSync(path.join(tmpdir(), "TPA CoWork portable 中文 "))
  const relocated = path.join(tempRoot, "agent venv 验收")
  try {
    cpSync(source, relocated, { recursive: true })
    verifyRuntime(relocated)
    console.log(`[verify-agent-venv] Relocation smoke passed at ${relocated}`)
  } finally {
    const canonicalTemp = canonicalPath(tempRoot)
    const canonicalSystemTemp = canonicalPath(tmpdir())
    if (!isInside(canonicalSystemTemp, canonicalTemp)) {
      throw new Error(`refusing to remove relocation path outside temp: ${tempRoot}`)
    }
    rmSync(tempRoot, { recursive: true, force: true })
  }
}

function minimalPythonEnv(scriptsDir) {
  const systemRoot = process.env.SystemRoot || process.env.WINDIR || "C:\\Windows"
  return {
    SystemRoot: systemRoot,
    WINDIR: systemRoot,
    TEMP: process.env.TEMP || tmpdir(),
    TMP: process.env.TMP || tmpdir(),
    PATH: `${scriptsDir};${path.join(systemRoot, "System32")}`,
  }
}

function canonicalPath(value) {
  const resolved = path.resolve(value)
  return (existsSync(resolved) ? realpathSync.native(resolved) : resolved).toLowerCase()
}

function isInside(parent, candidate) {
  const relative = path.relative(parent, candidate)
  return relative === "" || (!relative.startsWith("..") && !path.isAbsolute(relative))
}
