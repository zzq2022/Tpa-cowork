import { existsSync, readFileSync, readdirSync, statSync } from "node:fs"
import { relative, resolve } from "node:path"
import process from "node:process"

const LEGACY_PATTERNS = [
  "Hope Agent",
  "Hope-Agent",
  "HopeAgent",
  "hopeAgent",
  "hope_agent",
  "hopeagent",
  "hope-agent",
  ".hope-agent",
  ["H", "A_"].join(""),
  ["HO", "PE_"].join(""),
]

const IGNORED_DIRECTORIES = new Set([
  ".git",
  ".claude",
  ".scratch",
  ".superpowers",
  ".vite-cache",
  "node_modules",
  "target",
  "dist",
  "coverage",
])

const AUDIT_INFRASTRUCTURE_FILES = new Set([
  ".git",
  "scripts/check-product-identity.mjs",
  "scripts/__tests__/check-product-identity.test.mjs",
  "tpa/product-identity-allowlist.json",
])

const HISTORICAL_IDENTITY_FILES = new Set(["CHANGELOG.md"])

function isHistoricalIdentityFile(path) {
  return HISTORICAL_IDENTITY_FILES.has(path) || path.startsWith("docs/release-notes/")
}

function parseArgs(argv) {
  let root = process.cwd()
  let artifact = null
  for (let i = 0; i < argv.length; i += 1) {
    if (argv[i] === "--root") root = resolve(argv[++i])
    else if (argv[i] === "--artifact") artifact = resolve(argv[++i])
    else throw new Error(`Unknown argument: ${argv[i]}`)
  }
  return { root: artifact ?? resolve(root), allowlistRoot: resolve(root) }
}

function normalizedPath(root, path) {
  return relative(root, path).replaceAll("\\", "/")
}

function loadAllowlist(root, artifactMode) {
  if (artifactMode) return []
  const path = resolve(root, "tpa", "product-identity-allowlist.json")
  if (!existsSync(path)) return []
  const entries = JSON.parse(readFileSync(path, "utf8"))
  if (!Array.isArray(entries)) throw new Error("Product identity allowlist must be an array")
  for (const [index, entry] of entries.entries()) {
    if (!entry || typeof entry.path !== "string" || typeof entry.pattern !== "string") {
      throw new Error(`Allowlist entry ${index + 1} must define path and pattern`)
    }
    if (typeof entry.reason !== "string" || entry.reason.trim() === "") {
      throw new Error(`Allowlist entry ${index + 1} must have a non-empty reason`)
    }
  }
  return entries
}

function filesUnder(root) {
  const files = []
  const visit = (path) => {
    const stats = statSync(path)
    if (stats.isDirectory()) {
      if (path !== root && IGNORED_DIRECTORIES.has(path.split(/[\\/]/).at(-1))) return
      for (const name of readdirSync(path)) visit(resolve(path, name))
    } else if (stats.isFile()) {
      files.push(path)
    }
  }
  visit(root)
  return files
}

function isAllowed(allowlist, path, pattern) {
  return allowlist.some((entry) => entry.path === path && entry.pattern === pattern)
}

function parseProductToml(path) {
  return Object.fromEntries(
    readFileSync(path, "utf8")
      .split(/\r?\n/)
      .map((line) => line.trim())
      .filter((line) => line && !line.startsWith("#"))
      .map((line) => {
        const separator = line.indexOf("=")
        if (separator < 0) throw new Error(`Invalid product identity line: ${line}`)
        return [line.slice(0, separator).trim(), line.slice(separator + 1).trim().replace(/^"|"$/g, "")]
      }),
  )
}

function validateTypeScriptIdentity(root, failures) {
  const tomlPath = resolve(root, "tpa", "product.toml")
  const typescriptPath = resolve(root, "src", "tpa", "product.ts")
  if (!existsSync(tomlPath) || !existsSync(typescriptPath)) return

  const product = parseProductToml(tomlPath)
  const source = readFileSync(typescriptPath, "utf8")
  const mappings = {
    PRODUCT_DISPLAY_NAME: "display_name",
    PRODUCT_SLUG: "slug",
    PRODUCT_COMPACT_NAME: "compact_name",
    PRODUCT_AGENT_NAME: "agent_name",
    PRODUCT_DATA_DIR: "data_dir",
    PRODUCT_ENV_PREFIX: "env_prefix",
    PRODUCT_BINARY_NAME: "binary_name",
    PRODUCT_USER_AGENT: "user_agent",
  }
  for (const [constant, key] of Object.entries(mappings)) {
    const match = source.match(new RegExp(`export const ${constant} = ["']([^"']*)["']`))
    if (!match || match[1] !== product[key]) {
      failures.push(`src/tpa/product.ts: ${constant} must match tpa/product.toml key ${key}`)
    }
  }
}

function main() {
  const args = parseArgs(process.argv.slice(2))
  const artifactMode = args.root !== args.allowlistRoot
  const allowlist = loadAllowlist(args.allowlistRoot, artifactMode)
  const failures = []

  if (!artifactMode) validateTypeScriptIdentity(args.root, failures)

  for (const file of filesUnder(args.root)) {
    const path = normalizedPath(args.root, file)
    if (!artifactMode && isHistoricalIdentityFile(path)) continue
    if (!artifactMode && AUDIT_INFRASTRUCTURE_FILES.has(path)) {
      continue
    }
    let content
    try {
      content = readFileSync(file, "utf8")
    } catch {
      continue
    }
    if (content.includes("\u0000")) continue
    for (const [index, line] of content.split(/\r?\n/).entries()) {
      for (const pattern of LEGACY_PATTERNS) {
        if (line.includes(pattern) && !isAllowed(allowlist, path, pattern)) {
          failures.push(`${path}:${index + 1}: legacy identity ${JSON.stringify(pattern)}`)
        }
      }
    }
  }

  if (failures.length > 0) {
    console.error(failures.join("\n"))
    process.exitCode = 1
  } else {
    console.log(`Product identity check passed (${args.root})`)
  }
}

try {
  main()
} catch (error) {
  console.error(error instanceof Error ? error.message : String(error))
  process.exitCode = 1
}
