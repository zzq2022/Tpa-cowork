/// <reference types="node" />
import { readFileSync, readdirSync } from "node:fs"
import { dirname, join } from "node:path"
import { fileURLToPath } from "node:url"
import { test, expect } from "vitest"

const localesDir = join(dirname(fileURLToPath(import.meta.url)), "locales")

const expectedPlaceholders: Record<string, string> = {
  "en.json": "e.g. Always reply in English; keep the tone clear and friendly; start with the conclusion",
  "zh-TW.json": "例如：始終用繁體中文回覆；語氣清楚親切；先給結論再補充細節",
  "zh.json": "例如：始终用中文回复；语气清楚亲切；先给结论再补充细节",
}

test("project instruction placeholders use non-technical examples in every locale", () => {
  const localeFiles = readdirSync(localesDir)
    .filter((file) => file.endsWith(".json"))
    .sort()

  expect(Object.keys(expectedPlaceholders).sort()).toEqual(localeFiles)

  for (const file of localeFiles) {
    const locale = JSON.parse(readFileSync(join(localesDir, file), "utf8")) as {
      project?: { projectInstructionsPlaceholder?: string }
    }
    const placeholder = locale.project?.projectInstructionsPlaceholder

    expect(placeholder, file).toBe(expectedPlaceholders[file])
    expect(placeholder ?? "", file).not.toMatch(/Tauri|React|stack|技术栈|技術棧/i)
  }
})
