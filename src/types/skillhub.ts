import type { SkillSummary } from "@/components/settings/types"

/** Default AgentWork / SkillHub base URL (local/dev). Mirrors ha-core DEFAULT_SKILLHUB_SERVER_URL. */
export const DEFAULT_SKILLHUB_SERVER_URL = "http://127.0.0.1:3000"

export type SkillPublishState = "published" | "pending-review" | "not-published" | "unknown"

export interface CloudUser {
  id: string
  username: string
  displayName?: string | null
}

export interface CloudSession {
  serverUrl: string
  user: CloudUser
  authenticated: boolean
}

export interface CloudLoginRequest {
  serverUrl: string
  username: string
  password: string
}

export interface SkillCloudMetadata {
  registrySlug: string
  remoteSkillId?: string | null
  upstreamSlug?: string | null
  publishState: SkillPublishState
  publishStateReason?: string | null
  lastSyncedAt?: number | null
  lastError?: string | null
}

export interface MySkillCloudEntry {
  local: SkillSummary
  cloud: SkillCloudMetadata
}

export interface SkillHubPublicSkill {
  id: string
  name: string
  description?: string | null
  registrySlug: string
  authorUsername?: string | null
  downloads: number
  stars: number
  views: number
  category?: string | null
  updatedAt?: string | null
}

export interface SkillHubPublicSkillsPage {
  items: SkillHubPublicSkill[]
  total: number
  pageSize: number
}

export interface SkillDetail {
  id: string
  name: string
  description: string
  visibility: string
  ownerUserId?: string | null
  skillMd?: string | null
  skillMdAvailable: boolean
  slug?: string | null
  approvalStatus?: string | null
  authorName?: string | null
  category?: string | null
  starCount: number
  downloadCount: number
  viewCount: number
}
