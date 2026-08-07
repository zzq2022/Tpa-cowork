import { beforeEach, describe, expect, it, vi } from "vitest"

const { webviewWindowOptions } = vi.hoisted(() => ({
  webviewWindowOptions: vi.fn(),
}))

vi.mock("@/lib/transport", () => ({ isTauriMode: () => true }))
vi.mock("@/lib/logger", () => ({
  logger: { error: vi.fn() },
}))
vi.mock("@tauri-apps/api/webviewWindow", () => ({
  WebviewWindow: class {
    static getByLabel = vi.fn().mockResolvedValue(null)

    constructor(_label: string, options: unknown) {
      webviewWindowOptions(options)
    }

    once = vi.fn()
  },
}))

import { openHelpWindow } from "./openHelpWindow"

describe("openHelpWindow", () => {
  beforeEach(() => {
    webviewWindowOptions.mockClear()
  })

  it("uses the TPA CoWork product name for the desktop help window", async () => {
    await openHelpWindow()

    expect(webviewWindowOptions).toHaveBeenCalledWith(
      expect.objectContaining({ title: "TPA CoWork" }),
    )
  })
})
