import { describe, expect, test } from "vitest"

import { compareSkillFreshness } from "./downloadFreshness"

describe("compareSkillFreshness", () => {
  test("prefers semver when both sides parse", () => {
    expect(
      compareSkillFreshness({
        localVersion: "1.2.0",
        remoteVersion: "1.3.0",
        localUpdatedAt: "2026-01-01",
        remoteUpdatedAt: "2020-01-01",
      }),
    ).toBe("remote-newer")
    expect(
      compareSkillFreshness({
        localVersion: "2.0.0",
        remoteVersion: "1.9.9",
      }),
    ).toBe("local-newer")
    expect(
      compareSkillFreshness({
        localVersion: "v1.0.0",
        remoteVersion: "1.0.0",
      }),
    ).toBe("same")
  })

  test("falls back to timestamps", () => {
    expect(
      compareSkillFreshness({
        localUpdatedAt: "2026-07-01T00:00:00Z",
        remoteUpdatedAt: "2026-06-01T00:00:00Z",
      }),
    ).toBe("local-newer")
    expect(
      compareSkillFreshness({
        localUpdatedAt: 1_700_000_000,
        remoteUpdatedAt: 1_800_000_000,
      }),
    ).toBe("remote-newer")
  })

  test("returns unknown when incomparable", () => {
    expect(compareSkillFreshness({})).toBe("unknown")
    expect(
      compareSkillFreshness({
        localVersion: "latest",
        remoteVersion: "stable",
      }),
    ).toBe("unknown")
  })
})
