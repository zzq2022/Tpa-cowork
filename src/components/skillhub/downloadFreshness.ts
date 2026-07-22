/** Compare local vs remote skill freshness for overwrite dialogs. */

export type SkillOverwriteFreshness =
  | "local-newer"
  | "remote-newer"
  | "same"
  | "unknown"

export interface SkillFreshnessInput {
  localVersion?: string | null
  remoteVersion?: string | null
  localUpdatedAt?: string | number | null
  remoteUpdatedAt?: string | number | null
}

function parseSemverLike(value?: string | null): number[] | null {
  if (!value) return null
  const cleaned = value.trim().replace(/^v/i, "")
  if (!cleaned) return null
  const parts = cleaned.split(/[.+-]/).filter(Boolean)
  if (parts.length === 0) return null
  const nums: number[] = []
  for (const part of parts) {
    if (!/^\d+$/.test(part)) return null
    nums.push(Number(part))
  }
  return nums.length > 0 ? nums : null
}

function compareSemver(a: number[], b: number[]): number {
  const len = Math.max(a.length, b.length)
  for (let i = 0; i < len; i += 1) {
    const left = a[i] ?? 0
    const right = b[i] ?? 0
    if (left > right) return 1
    if (left < right) return -1
  }
  return 0
}

function parseTimestamp(value?: string | number | null): number | null {
  if (value == null || value === "") return null
  if (typeof value === "number" && Number.isFinite(value)) {
    // Accept seconds or milliseconds.
    return value < 1_000_000_000_000 ? value * 1000 : value
  }
  const ms = Date.parse(String(value))
  return Number.isFinite(ms) ? ms : null
}

export function compareSkillFreshness(input: SkillFreshnessInput): SkillOverwriteFreshness {
  const localVersion = parseSemverLike(input.localVersion)
  const remoteVersion = parseSemverLike(input.remoteVersion)
  if (localVersion && remoteVersion) {
    const cmp = compareSemver(localVersion, remoteVersion)
    if (cmp > 0) return "local-newer"
    if (cmp < 0) return "remote-newer"
    return "same"
  }

  const localTs = parseTimestamp(input.localUpdatedAt)
  const remoteTs = parseTimestamp(input.remoteUpdatedAt)
  if (localTs != null && remoteTs != null) {
    if (localTs > remoteTs) return "local-newer"
    if (localTs < remoteTs) return "remote-newer"
    return "same"
  }

  return "unknown"
}
