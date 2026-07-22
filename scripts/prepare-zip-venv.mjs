import { execFileSync } from "node:child_process"
import fs from "node:fs"
import path from "node:path"
import { fileURLToPath } from "node:url"

const repoRoot = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..")
const venvDir = path.join(repoRoot, "agent-venv")
const zipPath = path.join(repoRoot, "agent-venv.zip")
const force = process.argv.includes("--force")

if (process.platform !== "win32") {
  console.log("[prepare-zip-venv] Non-Windows target; skipping agent-venv ZIP.")
  process.exit(0)
}

if (fs.existsSync(zipPath) && !force) {
  const sizeMb = (fs.statSync(zipPath).size / 1024 / 1024).toFixed(1)
  console.log(`[prepare-zip-venv] Using existing agent-venv.zip (${sizeMb} MB).`)
  process.exit(0)
}

if (!fs.existsSync(venvDir)) {
  console.error(
    [
      "[prepare-zip-venv] agent-venv.zip and ./agent-venv are both missing.",
      "Build the environment with Python 3.12, or provide a prebuilt ZIP.",
      "See docs/packaging-agent-venv.md.",
    ].join("\n"),
  )
  process.exit(1)
}

console.log("[prepare-zip-venv] Compressing agent-venv...")
try {
  fs.rmSync(zipPath, { force: true })
  try {
    execFileSync(
      "tar.exe",
      ["-a", "-c", "-f", "agent-venv.zip", "agent-venv"],
      { cwd: repoRoot, stdio: "inherit" },
    )
  } catch {
    fs.rmSync(zipPath, { force: true })
    execFileSync(
      "powershell.exe",
      [
        "-NoProfile",
        "-ExecutionPolicy",
        "Bypass",
        "-Command",
        "Compress-Archive -LiteralPath 'agent-venv' -DestinationPath 'agent-venv.zip' -Force",
      ],
      { cwd: repoRoot, stdio: "inherit" },
    )
  }
} catch (error) {
  fs.rmSync(zipPath, { force: true })
  console.error("[prepare-zip-venv] Compression failed:", error)
  process.exit(1)
}

const sizeMb = (fs.statSync(zipPath).size / 1024 / 1024).toFixed(1)
console.log(`[prepare-zip-venv] Created agent-venv.zip (${sizeMb} MB).`)
