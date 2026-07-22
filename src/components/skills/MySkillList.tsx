import { useMemo, useState } from "react"
import { useTranslation } from "react-i18next"
import {
  AlertTriangle,
  CheckCircle2,
  CircleDashed,
  Cloud,
  FilePenLine,
  FolderOpen,
  PauseCircle,
  RefreshCw,
  Search,
  Send,
  UploadCloud,
  Wrench,
} from "lucide-react"
import { toast } from "sonner"

import { Button } from "@/components/ui/button"
import { SearchInput } from "@/components/ui/search-input"
import { IconTip } from "@/components/ui/tooltip"
import type { SkillStatusEntry } from "@/components/settings/types"
import { cn } from "@/lib/utils"
import type { MySkillCloudEntry, SkillPublishState } from "@/types/skillhub"

import {
  countLocalFilters,
  countPublishFilters,
  deriveLocalRuntimeState,
  matchesLocalFilter,
  matchesLocalSearch,
  matchesPublishFilter,
  primaryPublishBadge,
  type MySkillLocalFilter,
  type MySkillLocalRuntimeState,
  type MySkillPublishFilter,
} from "./mySkillLocalStatus"

interface MySkillListProps {
  rows: MySkillCloudEntry[]
  skillStatuses?: SkillStatusEntry[]
  loading: boolean
  canUseCloudActions: boolean
  /** Page refresh: local-only when logged out; local+cloud when logged in. */
  onRefresh: () => Promise<unknown>
  onSubmitReview: (localSkillName: string) => Promise<unknown>
  onOpenSkill: (name: string) => void
  onOpenDir: (path: string) => void
  /** Optional compact mode for Settings embedding. */
  compact?: boolean
  error?: string | null
}

const LOCAL_FILTERS: MySkillLocalFilter[] = [
  "all",
  "enabled",
  "disabled",
  "needs-config",
  "draft",
]

const PUBLISH_FILTERS: MySkillPublishFilter[] = [
  "all",
  "local",
  "pending-review",
  "published",
]

function publishStateKey(state: SkillPublishState | "cloud-failed"): string {
  switch (state) {
    case "published":
      return "mySkills.state.published"
    case "pending-review":
      return "mySkills.state.pendingReview"
    case "not-published":
      return "mySkills.state.notPublished"
    case "unknown":
      return "mySkills.state.unknown"
    case "cloud-failed":
      return "mySkills.state.cloudFailed"
  }
}

function publishStateClass(state: SkillPublishState | "cloud-failed"): string {
  switch (state) {
    case "published":
      return "bg-emerald-500/10 text-emerald-600"
    case "pending-review":
      return "bg-amber-500/10 text-amber-600"
    case "not-published":
      return "bg-secondary text-muted-foreground"
    case "unknown":
      return "bg-muted text-muted-foreground"
    case "cloud-failed":
      return "bg-destructive/10 text-destructive"
  }
}

function localStateKey(state: MySkillLocalRuntimeState): string {
  switch (state) {
    case "enabled":
      return "mySkills.localState.enabled"
    case "disabled":
      return "mySkills.localState.disabled"
    case "needs-config":
      return "mySkills.localState.needsConfig"
    case "draft":
      return "mySkills.localState.draft"
  }
}

function localStateClass(state: MySkillLocalRuntimeState): string {
  switch (state) {
    case "enabled":
      return "bg-emerald-500/10 text-emerald-600"
    case "disabled":
      return "bg-secondary text-muted-foreground"
    case "needs-config":
      return "bg-amber-500/10 text-amber-700"
    case "draft":
      return "bg-sky-500/10 text-sky-700"
  }
}

function localFilterIcon(filter: MySkillLocalFilter) {
  switch (filter) {
    case "all":
      return CircleDashed
    case "enabled":
      return CheckCircle2
    case "disabled":
      return PauseCircle
    case "needs-config":
      return Wrench
    case "draft":
      return FilePenLine
  }
}

function publishFilterIcon(filter: MySkillPublishFilter) {
  switch (filter) {
    case "all":
      return CircleDashed
    case "local":
      return Cloud
    case "pending-review":
      return UploadCloud
    case "published":
      return CheckCircle2
  }
}

function localFilterLabelKey(filter: MySkillLocalFilter): string {
  switch (filter) {
    case "all":
      return "mySkills.filter.all"
    case "enabled":
      return "mySkills.filter.enabled"
    case "disabled":
      return "mySkills.filter.disabled"
    case "needs-config":
      return "mySkills.filter.needsConfig"
    case "draft":
      return "mySkills.filter.draft"
  }
}

function publishFilterLabelKey(filter: MySkillPublishFilter): string {
  switch (filter) {
    case "all":
      return "mySkills.filter.all"
    case "local":
      return "mySkills.filter.local"
    case "pending-review":
      return "mySkills.filter.pendingReview"
    case "published":
      return "mySkills.filter.published"
  }
}

function skillIconFallback(name: string): string {
  const trimmed = name.trim()
  if (!trimmed) return "S"
  return trimmed.slice(0, 1).toUpperCase()
}

export default function MySkillList({
  rows,
  skillStatuses = [],
  loading,
  canUseCloudActions,
  onRefresh,
  onSubmitReview,
  onOpenSkill,
  onOpenDir,
  compact = false,
  error = null,
}: MySkillListProps) {
  const { t } = useTranslation()
  const [refreshing, setRefreshing] = useState(false)
  const [submitting, setSubmitting] = useState<string | null>(null)
  const [query, setQuery] = useState("")
  const [localFilter, setLocalFilter] = useState<MySkillLocalFilter>("all")
  const [publishFilter, setPublishFilter] = useState<MySkillPublishFilter>("all")

  const statusByName = useMemo(() => {
    const next: Record<string, SkillStatusEntry | undefined> = {}
    for (const status of skillStatuses) next[status.name] = status
    return next
  }, [skillStatuses])

  const localCounts = useMemo(
    () => countLocalFilters(
      rows.map((row) => row.local),
      statusByName,
    ),
    [rows, statusByName],
  )

  const publishCounts = useMemo(() => countPublishFilters(rows), [rows])

  const visibleRows = useMemo(() => {
    return rows.filter((row) => {
      const status = statusByName[row.local.name]
      const matchesSearch = matchesLocalSearch(row.local, query)
      if (!matchesSearch) return false
      if (canUseCloudActions) {
        return matchesPublishFilter(row, publishFilter)
      }
      return matchesLocalFilter(row.local, status, localFilter)
    })
  }, [rows, statusByName, query, canUseCloudActions, publishFilter, localFilter])

  async function refresh() {
    setRefreshing(true)
    try {
      await onRefresh()
    } finally {
      setRefreshing(false)
    }
  }

  async function submitReview(name: string) {
    if (!canUseCloudActions) {
      toast.warning(t("mySkills.loginToSubmitPrompt"))
      return
    }
    setSubmitting(name)
    try {
      await onSubmitReview(name)
      toast.success(t("mySkills.submitSuccess"))
    } catch (e) {
      toast.error(e instanceof Error ? e.message : String(e))
    } finally {
      setSubmitting(null)
    }
  }

  if (loading && rows.length === 0) {
    return (
      <div className="flex items-center justify-center py-10">
        <div className="h-5 w-5 animate-spin rounded-full border-2 border-foreground border-t-transparent" />
      </div>
    )
  }

  return (
    <div className={cn("flex h-full min-h-0 flex-col", compact ? "gap-3" : "gap-4")}>
      <div className="shrink-0 space-y-3">
        <div className="flex flex-wrap items-center gap-2">
          <div className="relative min-w-[180px] flex-1 max-w-md">
            <Search className="pointer-events-none absolute left-2.5 top-1/2 h-3.5 w-3.5 -translate-y-1/2 text-muted-foreground" />
            <SearchInput
              value={query}
              onChange={(e) => setQuery(e.target.value)}
              placeholder={t("mySkills.searchPlaceholder")}
              className="h-8 pl-8 text-xs"
              aria-label={t("mySkills.searchPlaceholder")}
            />
          </div>
          <Button
            variant="ghost"
            size="sm"
            className="h-8 gap-1.5 px-2 text-xs"
            onClick={() => void refresh()}
            disabled={refreshing}
          >
            <RefreshCw className={cn("h-3.5 w-3.5", refreshing && "animate-spin")} />
            {t(canUseCloudActions ? "mySkills.refreshStatus" : "mySkills.refreshLocal")}
          </Button>
        </div>

        <div className="flex flex-wrap items-center gap-1.5">
          {canUseCloudActions
            ? PUBLISH_FILTERS.map((item) => {
                const Icon = publishFilterIcon(item)
                const active = publishFilter === item
                return (
                  <Button
                    key={item}
                    type="button"
                    variant="ghost"
                    size="sm"
                    onClick={() => setPublishFilter(item)}
                    className={cn(
                      "h-7 gap-1 rounded-md px-2 text-xs font-medium",
                      active
                        ? "bg-primary/10 text-primary hover:bg-primary/15 hover:text-primary"
                        : "bg-secondary/40 text-muted-foreground hover:bg-secondary/60",
                    )}
                    aria-pressed={active}
                  >
                    <Icon className="h-3.5 w-3.5" />
                    <span>{t(publishFilterLabelKey(item))}</span>
                    <span className="tabular-nums opacity-80">{publishCounts[item]}</span>
                  </Button>
                )
              })
            : LOCAL_FILTERS.map((item) => {
                const Icon = localFilterIcon(item)
                const active = localFilter === item
                return (
                  <Button
                    key={item}
                    type="button"
                    variant="ghost"
                    size="sm"
                    onClick={() => setLocalFilter(item)}
                    className={cn(
                      "h-7 gap-1 rounded-md px-2 text-xs font-medium",
                      active
                        ? "bg-primary/10 text-primary hover:bg-primary/15 hover:text-primary"
                        : "bg-secondary/40 text-muted-foreground hover:bg-secondary/60",
                    )}
                    aria-pressed={active}
                  >
                    <Icon className="h-3.5 w-3.5" />
                    <span>{t(localFilterLabelKey(item))}</span>
                    <span className="tabular-nums opacity-80">{localCounts[item]}</span>
                  </Button>
                )
              })}
        </div>

        {error && (
          <div className="flex items-start gap-2 rounded-lg border border-destructive/20 bg-destructive/5 px-3 py-2 text-xs text-destructive">
            <AlertTriangle className="mt-0.5 h-3.5 w-3.5 shrink-0" />
            <div className="min-w-0 flex-1">
              <p className="truncate">{error}</p>
              <Button
                variant="ghost"
                size="sm"
                className="mt-1 h-6 px-1.5 text-[11px]"
                onClick={() => void refresh()}
              >
                {t("mySkills.retry")}
              </Button>
            </div>
          </div>
        )}
      </div>

      {rows.length === 0 ? (
        <div className="py-10 text-center">
          <p className="text-sm text-muted-foreground">{t("mySkills.empty")}</p>
          <p className="mt-1 text-xs text-muted-foreground/70">{t("mySkills.emptyHint")}</p>
        </div>
      ) : visibleRows.length === 0 ? (
        <div className="py-10 text-center">
          <p className="text-sm text-muted-foreground">{t("mySkills.noMatches")}</p>
        </div>
      ) : (
        <div className="min-h-0 flex-1 overflow-y-auto">
          <div
            className="grid gap-3"
            style={{
              gridTemplateColumns: "repeat(auto-fill, minmax(min(100%, 220px), 1fr))",
            }}
          >
            {visibleRows.map((row) => {
              const status = statusByName[row.local.name]
              const localState = deriveLocalRuntimeState(row.local, status)
              const cloudState = canUseCloudActions
                ? primaryPublishBadge(row.cloud.publishState, row.cloud.lastError)
                : null
              const tags = (row.local.display?.tags ?? []).slice(0, 3)
              const pending = row.cloud.publishState === "pending-review"
              const published = row.cloud.publishState === "published"
              const hideSubmit = pending || published
              const submitDisabled =
                hideSubmit || submitting === row.local.name || !canUseCloudActions
              const submitLabel = hideSubmit
                ? t(publishStateKey(row.cloud.publishState))
                : canUseCloudActions
                  ? t("mySkills.submitReview")
                  : t("mySkills.loginToSubmit")
              const emoji = row.local.display?.emoji

              return (
                <div
                  key={row.local.name}
                  className={cn(
                    "group relative flex min-h-[220px] flex-col rounded-lg border border-border-soft bg-card p-3 text-left transition-colors",
                    "hover:bg-secondary/30 focus-within:ring-1 focus-within:ring-ring",
                    "aspect-square",
                  )}
                >
                  <button
                    type="button"
                    className="flex min-h-0 flex-1 flex-col text-left outline-none cursor-pointer"
                    onDoubleClick={() => onOpenSkill(row.local.name)}
                    aria-label={t("mySkills.openDetail", {
                      name: row.local.name,
                      defaultValue: `Open ${row.local.name}`,
                    })}
                  >
                    <div className="flex items-start justify-between gap-2">
                      <div className="flex h-10 w-10 shrink-0 items-center justify-center rounded-lg bg-secondary/70 text-lg">
                        {emoji ? (
                          <span aria-hidden>{emoji}</span>
                        ) : (
                          <span className="text-sm font-semibold text-muted-foreground">
                            {skillIconFallback(row.local.name)}
                          </span>
                        )}
                      </div>
                      <div className="flex min-w-0 flex-wrap justify-end gap-1">
                        {canUseCloudActions ? (
                          cloudState ? (
                            <span
                              className={cn(
                                "rounded px-1.5 py-0.5 text-[10px] font-medium",
                                publishStateClass(cloudState),
                              )}
                            >
                              {t(publishStateKey(cloudState))}
                            </span>
                          ) : null
                        ) : (
                          <span
                            className={cn(
                              "rounded px-1.5 py-0.5 text-[10px] font-medium",
                              localStateClass(localState),
                            )}
                          >
                            {t(localStateKey(localState))}
                          </span>
                        )}
                      </div>
                    </div>

                    <div className="mt-3 min-w-0 flex-1">
                      <div
                        className={cn(
                          "truncate text-sm font-medium text-foreground",
                          !row.local.enabled && "line-through opacity-70",
                        )}
                      >
                        {row.local.name}
                      </div>
                      <p className="mt-1 line-clamp-2 text-xs leading-5 text-muted-foreground">
                        {row.local.description || t("mySkills.noDescription")}
                      </p>
                      {tags.length > 0 && (
                        <div className="mt-3 flex flex-wrap gap-1">
                          {tags.map((tag) => (
                            <span
                              key={tag}
                              className="max-w-full truncate rounded bg-secondary/70 px-1.5 py-0.5 text-[10px] text-muted-foreground"
                            >
                              {tag}
                            </span>
                          ))}
                        </div>
                      )}
                    </div>
                  </button>

                  <div className="mt-3 flex items-center justify-end gap-1 border-t border-border-soft/70 pt-2 opacity-100 sm:opacity-0 sm:transition-opacity sm:group-hover:opacity-100 sm:group-focus-within:opacity-100">
                    {!hideSubmit && (
                      <IconTip label={submitLabel}>
                        <Button
                          variant="ghost"
                          size="icon"
                          className="h-7 w-7 text-muted-foreground"
                          onClick={() => void submitReview(row.local.name)}
                          disabled={submitDisabled}
                          aria-label={submitLabel}
                        >
                          <Send className="h-3.5 w-3.5" />
                        </Button>
                      </IconTip>
                    )}
                    <IconTip label={t("mySkills.openDirectory")}>
                      <Button
                        variant="ghost"
                        size="icon"
                        className="h-7 w-7 text-muted-foreground"
                        onClick={() => onOpenDir(row.local.base_dir)}
                        aria-label={t("mySkills.openDirectory")}
                      >
                        <FolderOpen className="h-3.5 w-3.5" />
                      </Button>
                    </IconTip>
                  </div>
                </div>
              )
            })}
          </div>
        </div>
      )}
    </div>
  )
}
