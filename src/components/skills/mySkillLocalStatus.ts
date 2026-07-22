import type { SkillStatusEntry, SkillSummary } from "@/components/settings/types"
import type { MySkillCloudEntry, SkillPublishState } from "@/types/skillhub"

/** Local runtime filter keys for unauthenticated My Skills workbench. */
export type MySkillLocalFilter =
  | "all"
  | "enabled"
  | "disabled"
  | "needs-config"
  | "draft"

/** Publish workflow filter keys aligned with AgentWork when logged in. */
export type MySkillPublishFilter =
  | "all"
  | "local"
  | "pending-review"
  | "published"

export type MySkillLocalRuntimeState =
  | "enabled"
  | "disabled"
  | "needs-config"
  | "draft"

export function deriveLocalRuntimeState(
  skill: SkillSummary,
  status?: SkillStatusEntry,
): MySkillLocalRuntimeState {
  if (skill.status === "draft") return "draft"
  if (!skill.enabled || status?.disabled) return "disabled"
  // Prefer get_skills_status hard blockers / setup flags. Without a status
  // snapshot, keep the skill enabled rather than guessing missing env.
  if (status?.hard_blocked || status?.needs_setup) return "needs-config"
  return "enabled"
}

export function matchesLocalFilter(
  skill: SkillSummary,
  status: SkillStatusEntry | undefined,
  filter: MySkillLocalFilter,
): boolean {
  if (filter === "all") return true
  return deriveLocalRuntimeState(skill, status) === filter
}

export function matchesPublishFilter(
  entry: MySkillCloudEntry,
  filter: MySkillPublishFilter,
): boolean {
  if (filter === "all") return true
  if (filter === "local") return entry.cloud.publishState === "not-published"
  return entry.cloud.publishState === filter
}

export function matchesLocalSearch(
  skill: SkillSummary,
  query: string,
): boolean {
  const q = query.trim().toLowerCase()
  if (!q) return true

  const haystack = [
    skill.name,
    skill.description,
    skill.display?.author,
    skill.display?.license,
    skill.display?.version,
    skill.display?.license_label,
    ...(skill.display?.tags ?? []),
  ]
    .filter(Boolean)
    .join("\n")
    .toLowerCase()

  return haystack.includes(q)
}

export function countLocalFilters(
  skills: SkillSummary[],
  statusByName: Record<string, SkillStatusEntry | undefined>,
): Record<MySkillLocalFilter, number> {
  const counts: Record<MySkillLocalFilter, number> = {
    all: skills.length,
    enabled: 0,
    disabled: 0,
    "needs-config": 0,
    draft: 0,
  }
  for (const skill of skills) {
    const state = deriveLocalRuntimeState(skill, statusByName[skill.name])
    counts[state] += 1
  }
  return counts
}

export function countPublishFilters(
  rows: MySkillCloudEntry[],
): Record<MySkillPublishFilter, number> {
  const counts: Record<MySkillPublishFilter, number> = {
    all: rows.length,
    local: 0,
    "pending-review": 0,
    published: 0,
  }
  for (const row of rows) {
    if (row.cloud.publishState === "not-published") counts.local += 1
    else if (row.cloud.publishState === "pending-review") counts["pending-review"] += 1
    else if (row.cloud.publishState === "published") counts.published += 1
  }
  return counts
}

/** Primary badge when logged in: prefer publish workflow states. */
export function primaryPublishBadge(
  publishState: SkillPublishState,
  lastError?: string | null,
): SkillPublishState | "cloud-failed" | null {
  if (lastError && /fail|error|timeout|offline|auth|401|credential/i.test(lastError)) {
    if (publishState === "unknown" || !publishState) return "cloud-failed"
  }
  if (publishState === "unknown") return null
  return publishState
}

/** Secondary cloud badge shown only when a concrete remote result is cached. */
export function cloudBadgeState(
  publishState: SkillPublishState,
  lastError?: string | null,
): SkillPublishState | "cloud-failed" | null {
  return primaryPublishBadge(publishState, lastError)
}
