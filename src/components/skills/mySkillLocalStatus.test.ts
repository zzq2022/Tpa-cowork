import { describe, expect, test } from "vitest"

import type { SkillStatusEntry, SkillSummary } from "@/components/settings/types"
import type { MySkillCloudEntry } from "@/types/skillhub"

import {
  cloudBadgeState,
  countLocalFilters,
  countPublishFilters,
  deriveLocalRuntimeState,
  matchesLocalSearch,
  matchesPublishFilter,
} from "./mySkillLocalStatus"

function skill(partial: Partial<SkillSummary> & Pick<SkillSummary, "name">): SkillSummary {
  return {
    description: "",
    source: "user",
    base_dir: `/tmp/${partial.name}`,
    enabled: true,
    requires_env: [],
    ...partial,
  }
}

function entry(
  name: string,
  publishState: MySkillCloudEntry["cloud"]["publishState"],
): MySkillCloudEntry {
  return {
    local: skill({ name }),
    cloud: {
      registrySlug: `alice/${name}`,
      publishState,
    },
  }
}

describe("mySkillLocalStatus", () => {
  test("derives local runtime states", () => {
    expect(deriveLocalRuntimeState(skill({ name: "a", status: "draft" }))).toBe("draft")
    expect(deriveLocalRuntimeState(skill({ name: "b", enabled: false }))).toBe("disabled")
    expect(
      deriveLocalRuntimeState(skill({ name: "c" }), {
        name: "c",
        source: "user",
        eligible: false,
        disabled: false,
        blocked_by_allowlist: false,
        has_install: false,
        always: false,
        needs_setup: true,
      } satisfies SkillStatusEntry),
    ).toBe("needs-config")
    expect(deriveLocalRuntimeState(skill({ name: "d" }))).toBe("enabled")
  })

  test("search ignores registry-like internal fields", () => {
    const target = skill({
      name: "code-review",
      description: "Review code changes",
      display: { tags: ["quality"], author: "alice" },
    })
    expect(matchesLocalSearch(target, "quality")).toBe(true)
    expect(matchesLocalSearch(target, "alice")).toBe(true)
    expect(matchesLocalSearch(target, "alice/code-review")).toBe(false)
  })

  test("counts local filters", () => {
    const skills = [
      skill({ name: "a" }),
      skill({ name: "b", enabled: false }),
      skill({ name: "c", status: "draft" }),
    ]
    const counts = countLocalFilters(skills, {})
    expect(counts.all).toBe(3)
    expect(counts.enabled).toBe(1)
    expect(counts.disabled).toBe(1)
    expect(counts.draft).toBe(1)
  })

  test("counts publish filters for AgentWork-like states", () => {
    const rows = [
      entry("a", "not-published"),
      entry("b", "pending-review"),
      entry("c", "published"),
      entry("d", "unknown"),
    ]
    const counts = countPublishFilters(rows)
    expect(counts.all).toBe(4)
    expect(counts.local).toBe(1)
    expect(counts["pending-review"]).toBe(1)
    expect(counts.published).toBe(1)
    expect(matchesPublishFilter(rows[0], "local")).toBe(true)
    expect(matchesPublishFilter(rows[1], "pending-review")).toBe(true)
    expect(matchesPublishFilter(rows[2], "published")).toBe(true)
    expect(matchesPublishFilter(rows[3], "local")).toBe(false)
  })

  test("cloud badge hides unknown without error", () => {
    expect(cloudBadgeState("unknown")).toBeNull()
    expect(cloudBadgeState("pending-review")).toBe("pending-review")
    expect(cloudBadgeState("unknown", "cloud authentication failed")).toBe("cloud-failed")
  })
})
