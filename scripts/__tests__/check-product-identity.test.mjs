import assert from "node:assert/strict"
import { execFileSync } from "node:child_process"
import { mkdtempSync, mkdirSync, writeFileSync } from "node:fs"
import { tmpdir } from "node:os"
import { join } from "node:path"
import test from "node:test"

const checker = join(process.cwd(), "scripts", "check-product-identity.mjs")

function fixture() {
  const root = mkdtempSync(join(tmpdir(), "tpa-product-check-"))
  mkdirSync(join(root, "src"), { recursive: true })
  mkdirSync(join(root, "tpa"), { recursive: true })
  return root
}

function run(root) {
  try {
    execFileSync(process.execPath, [checker, "--root", root], { encoding: "utf8", stdio: "pipe" })
    return { status: 0, output: "" }
  } catch (error) {
    return {
      status: error.status,
      output: `${error.stdout ?? ""}${error.stderr ?? ""}`,
    }
  }
}

test("reports an unapproved legacy product identity with file and line", () => {
  const root = fixture()
  writeFileSync(join(root, "src", "visible.ts"), 'export const title = "Hope Agent"\n')
  writeFileSync(join(root, "tpa", "product-identity-allowlist.json"), "[]\n")

  const result = run(root)

  assert.equal(result.status, 1)
  assert.match(result.output, /src\/visible\.ts:1/)
  assert.match(result.output, /Hope Agent/)
})

for (const legacyIdentity of ["hopeAgent", "hope_agent", "repo.hopeagent.ai"]) {
  test(`reports legacy identity variant ${legacyIdentity}`, () => {
    const root = fixture()
    writeFileSync(join(root, "src", "visible.ts"), `export const value = ${JSON.stringify(legacyIdentity)}\n`)
    writeFileSync(join(root, "tpa", "product-identity-allowlist.json"), "[]\n")

    const result = run(root)

    assert.equal(result.status, 1)
    assert.match(result.output, /src\/visible\.ts:1/)
  })
}

test("accepts an exact attribution allowlist entry with a reason", () => {
  const root = fixture()
  writeFileSync(join(root, "NOTICE.md"), "Upstream project: Hope Agent\n")
  writeFileSync(
    join(root, "tpa", "product-identity-allowlist.json"),
    `${JSON.stringify(
      [
        {
          path: "NOTICE.md",
          pattern: "Hope Agent",
          reason: "Required upstream attribution",
        },
      ],
      null,
      2,
    )}\n`,
  )

  const result = run(root)

  assert.equal(result.status, 0, result.output)
})

test("preserves legacy identities in dedicated release history", () => {
  const root = fixture()
  mkdirSync(join(root, "docs", "release-notes"), { recursive: true })
  writeFileSync(join(root, "docs", "release-notes", "v0.1.0.md"), "Released as Hope Agent\n")
  writeFileSync(join(root, "tpa", "product-identity-allowlist.json"), "[]\n")

  const result = run(root)

  assert.equal(result.status, 0, result.output)
})

test("rejects allowlist entries without a reason", () => {
  const root = fixture()
  writeFileSync(join(root, "NOTICE.md"), "Upstream project: Hope Agent\n")
  writeFileSync(
    join(root, "tpa", "product-identity-allowlist.json"),
    `${JSON.stringify([{ path: "NOTICE.md", pattern: "Hope Agent", reason: "" }], null, 2)}\n`,
  )

  const result = run(root)

  assert.equal(result.status, 1)
  assert.match(result.output, /non-empty reason/)
})

test("rejects TypeScript identity drift from product.toml", () => {
  const root = fixture()
  mkdirSync(join(root, "src", "tpa"), { recursive: true })
  writeFileSync(
    join(root, "tpa", "product.toml"),
    [
      'display_name = "TPA CoWork"',
      'slug = "tpa-cowork"',
      'compact_name = "TPACoWork"',
      'agent_name = "TPA-Agent"',
      'data_dir = ".tpa-cowork"',
      'env_prefix = "TPA_COWORK_"',
      'binary_name = "tpa-cowork"',
      'user_agent = "TPA-CoWork"',
    ].join("\n"),
  )
  writeFileSync(join(root, "src", "tpa", "product.ts"), 'export const PRODUCT_DISPLAY_NAME = "Wrong"\n')

  const result = run(root)

  assert.equal(result.status, 1)
  assert.match(result.output, /PRODUCT_DISPLAY_NAME must match/)
})
