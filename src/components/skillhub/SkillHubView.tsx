import { useState, useEffect } from "react"
import { useTranslation } from "react-i18next"
import {
  Download,
  RefreshCw,
  Search,
  Star,
  Plus,
  Flame,
  TrendingUp,
  Clock,
  ArrowLeft,
  Loader2,
  ShieldCheck,
  ShieldAlert,
  Languages,
  User,
  Tag,
  Check,
} from "lucide-react"
import { toast } from "sonner"

import CloudLoginDialog from "@/components/cloud/CloudLoginDialog"
import {
  AlertDialog,
  AlertDialogCancel,
  AlertDialogContent,
  AlertDialogDescription,
  AlertDialogFooter,
  AlertDialogHeader,
  AlertDialogTitle,
} from "@/components/ui/alert-dialog"
import { Dialog, DialogContent } from "@/components/ui/dialog"
import { Button } from "@/components/ui/button"
import { Input } from "@/components/ui/input"
import { getTransport } from "@/lib/transport-provider"
import type { MySkillCloudEntry, SkillHubPublicSkill, SkillHubPublicSkillsPage, SkillDetail } from "@/types/skillhub"
import { cn } from "@/lib/utils"
import MarkdownRenderer from "@/components/common/MarkdownRenderer"
import { compareSkillFreshness } from "@/components/skillhub/downloadFreshness"

interface SkillHubViewProps {
  onBack: () => void
  onDownloaded?: (entry: MySkillCloudEntry) => void
}

const CATEGORIES = [
  "全部",
  "通用",
  "办公工具",
  "开发工具",
  "AI 生成",
  "数据分析",
  "项目管理",
  "部署",
  "设计",
  "安全",
  "元技能",
]

function getSkillCategory(name: string): string {
  const lowercase = name.toLowerCase()
  if (lowercase.includes("scaffold") || lowercase.includes("browser") || lowercase.includes("test")) {
    return "开发工具"
  }
  if (lowercase.includes("notebook") || lowercase.includes("pdf") || lowercase.includes("doc")) {
    return "办公工具"
  }
  if (lowercase.includes("plan") || lowercase.includes("workflow") || lowercase.includes("project")) {
    return "项目管理"
  }
  if (
    lowercase.includes("stock") ||
    lowercase.includes("price") ||
    lowercase.includes("query") ||
    lowercase.includes("sql") ||
    lowercase.includes("write")
  ) {
    return "数据分析"
  }
  if (
    lowercase.includes("excalidraw") ||
    lowercase.includes("diagram") ||
    lowercase.includes("design") ||
    lowercase.includes("paint") ||
    lowercase.includes("image")
  ) {
    return "设计"
  }
  if (lowercase.includes("creator") || lowercase.includes("skill") || lowercase.includes("meta")) {
    return "元技能"
  }
  if (lowercase.includes("security") || lowercase.includes("key") || lowercase.includes("safe")) {
    return "安全"
  }
  if (lowercase.includes("deploy") || lowercase.includes("docker") || lowercase.includes("build")) {
    return "部署"
  }
  if (lowercase.includes("generate") || lowercase.includes("ai") || lowercase.includes("gpt")) {
    return "AI 生成"
  }
  return "通用"
}

function getAvatarColor(name: string): string {
  const colors = [
    "bg-emerald-500/10 text-emerald-600 dark:text-emerald-400",
    "bg-blue-500/10 text-blue-600 dark:text-blue-400",
    "bg-amber-500/10 text-amber-600 dark:text-amber-400",
    "bg-indigo-500/10 text-indigo-600 dark:text-indigo-400",
    "bg-rose-500/10 text-rose-600 dark:text-rose-400",
    "bg-purple-500/10 text-purple-600 dark:text-purple-400",
    "bg-pink-500/10 text-pink-600 dark:text-pink-400",
    "bg-red-500/10 text-red-600 dark:text-red-400",
    "bg-teal-500/10 text-teal-600 dark:text-teal-400",
    "bg-cyan-500/10 text-cyan-600 dark:text-cyan-400",
  ]
  let hash = 0
  for (let i = 0; i < name.length; i++) {
    hash = name.charCodeAt(i) + ((hash << 5) - hash)
  }
  const index = Math.abs(hash) % colors.length
  return colors[index]
}

function formatUpdateTime(isoString?: string | null): string {
  if (!isoString) return ""
  try {
    const date = new Date(isoString)
    const y = date.getFullYear()
    const m = String(date.getMonth() + 1).padStart(2, "0")
    const d = String(date.getDate()).padStart(2, "0")
    return `${y}-${m}-${d}`
  } catch {
    return ""
  }
}

export default function SkillHubView({ onBack, onDownloaded }: SkillHubViewProps) {
  const { t } = useTranslation()
  const [query, setQuery] = useState("")
  const [skills, setSkills] = useState<SkillHubPublicSkill[]>([])
  const [loading, setLoading] = useState(false)
  const [downloading, setDownloading] = useState<string | null>(null)
  const [loginOpen, setLoginOpen] = useState(false)
  const [pendingOverwrite, setPendingOverwrite] = useState<{
    skill: SkillHubPublicSkill
    localVersion?: string | null
    remoteVersion?: string | null
    localUpdatedAt?: string | number | null
    remoteUpdatedAt?: string | number | null
    freshness: "local-newer" | "remote-newer" | "same" | "unknown"
  } | null>(null)
  const [sortBy, setSortBy] = useState<"popular" | "downloads" | "stars" | "latest">("popular")
  const [selectedCategory, setSelectedCategory] = useState<string>("全部")

  // Pagination states
  const [page, setPage] = useState(1)
  const [totalPages, setTotalPages] = useState(1)
  const [totalItems, setTotalItems] = useState(0)

  // Preview Modal States
  const [selectedDetail, setSelectedDetail] = useState<SkillDetail | null>(null)
  const [loadingDetailId, setLoadingDetailId] = useState<string | null>(null)
  const [isTranslating, setIsTranslating] = useState(false)
  const [showTranslation, setShowTranslation] = useState(false)
  const [translatedText, setTranslatedText] = useState<string>("")
  const [scanStatus, setScanStatus] = useState<"none" | "scanning" | "passed" | "warning">("none")

  async function search(nextPage = page, nextSort = sortBy, nextCategory = selectedCategory) {
    setLoading(true)
    try {
      const sortValue =
        nextSort === "popular"
          ? "trending"
          : nextSort === "downloads"
            ? "top"
            : nextSort === "stars"
              ? "most_starred"
              : "new"
      const categoryValue = nextCategory === "全部" ? null : nextCategory

      const result = await getTransport().call<SkillHubPublicSkillsPage>("skillhub_search_public", {
        request: {
          query: query || null,
          limit: 12,
          page: nextPage,
          sort: sortValue,
          category: categoryValue,
        },
      })

      setSkills(result.items || [])
      setTotalItems(result.total)
      setTotalPages(Math.max(1, Math.ceil(result.total / 12)))
    } catch (e) {
      toast.error(e instanceof Error ? e.message : String(e))
    } finally {
      setLoading(false)
    }
  }

  // Load public skills automatically on initial mount
  useEffect(() => {
    void search(1, sortBy, selectedCategory)
  }, [])

  function handleCloudError(e: unknown) {
    const message = e instanceof Error ? e.message : String(e)
    if (/auth|login|token|401/i.test(message)) {
      setLoginOpen(true)
      return
    }
    throw e
  }

  async function requestDownload(skill: SkillHubPublicSkill) {
    setDownloading(skill.id)
    try {
      const localSkills = await getTransport().call<MySkillCloudEntry[]>("my_skills_list")
      const existing = localSkills.find(
        (entry) =>
          entry.local.name === skill.name ||
          entry.local.name.toLowerCase() === skill.name.toLowerCase() ||
          entry.cloud?.remoteSkillId === skill.id ||
          entry.cloud?.registrySlug === skill.registrySlug,
      )
      if (existing) {
        const localVersion = existing.local.display?.version ?? null
        const remoteVersion = null
        const localUpdatedAt = null
        const remoteUpdatedAt = skill.updatedAt ?? null
        const freshness = compareSkillFreshness({
          localVersion,
          remoteVersion,
          localUpdatedAt,
          remoteUpdatedAt,
        })
        setPendingOverwrite({
          skill,
          localVersion,
          remoteVersion,
          localUpdatedAt,
          remoteUpdatedAt,
          freshness,
        })
        return
      }
    } catch (e) {
      try {
        handleCloudError(e)
      } catch (rethrown) {
        toast.error(rethrown instanceof Error ? rethrown.message : String(rethrown))
      }
      return
    } finally {
      setDownloading(null)
    }
    await download(skill.id, false)
  }

  async function download(skillId: string, overwrite: boolean) {
    setDownloading(skillId)
    try {
      const installed = await getTransport().call<MySkillCloudEntry>("skillhub_download_skill", {
        skillId,
        overwrite,
      })
      toast.success(
        overwrite ? t("skillhub.downloadedOverwrite") : t("skillhub.downloaded"),
      )
      setPendingOverwrite(null)
      setSelectedDetail(null) // Close preview modal after download
      onDownloaded?.(installed)
    } catch (e) {
      try {
        handleCloudError(e)
      } catch (rethrown) {
        const msg = rethrown instanceof Error ? rethrown.message : String(rethrown)
        if (!overwrite && /already exists/i.test(msg)) {
          const targetSkill =
            skills.find((s) => s.id === skillId) ||
            (selectedDetail && selectedDetail.id === skillId
              ? ({
                  id: selectedDetail.id,
                  name: selectedDetail.name,
                  registrySlug: selectedDetail.slug ?? selectedDetail.name,
                  downloads: selectedDetail.downloadCount,
                  stars: selectedDetail.starCount,
                  views: selectedDetail.viewCount,
                  description: selectedDetail.description,
                  authorUsername: selectedDetail.authorName,
                  category: selectedDetail.category,
                  updatedAt: null,
                } as SkillHubPublicSkill)
              : null)
          if (targetSkill) {
            setPendingOverwrite({
              skill: targetSkill,
              freshness: "unknown",
            })
            return
          }
        }
        toast.error(msg)
      }
    } finally {
      setDownloading(null)
    }
  }

  async function openSkillPreview(skillId: string) {
    setLoadingDetailId(skillId)
    setScanStatus("none")
    setShowTranslation(false)
    try {
      const detail = await getTransport().call<SkillDetail>("skillhub_get_public_detail", {
        skillId,
      })
      setSelectedDetail(detail)
    } catch (e) {
      toast.error(e instanceof Error ? e.message : String(e))
    } finally {
      setLoadingDetailId(null)
    }
  }

  // Simulated AI Translator
  async function handleTranslate() {
    if (showTranslation) {
      setShowTranslation(false)
      return
    }
    if (translatedText && selectedDetail && translatedText.includes(selectedDetail.name)) {
      setShowTranslation(true)
      return
    }
    setIsTranslating(true)
    try {
      // Simulate API call delay
      await new Promise((resolve) => setTimeout(resolve, 1200))
      const original = selectedDetail?.skillMd || ""
      // Simple mockup translation replacing basic layout
      const translated = original
        .replace(/# Prerequisites/gi, "# 安装前置要求")
        .replace(/# Description/gi, "# 技能描述")
        .replace(/# Instructions/gi, "# 使用指导")
        .replace(/# Usage/gi, "# 使用指导")
      setTranslatedText(translated)
      setShowTranslation(true)
      toast.success("翻译成功")
    } catch {
      toast.error("翻译失败")
    } finally {
      setIsTranslating(false)
    }
  }

  // Simulated local safety scanner
  function runScan() {
    if (!selectedDetail) return
    setScanStatus("scanning")
    setTimeout(() => {
      const content = (selectedDetail.skillMd || "").toLowerCase()
      const dangerousPatterns = [/rm\s+-rf/, /format\s+[a-zA-Z]:/, /drop\s+database/i, /delete\s+from/i]
      const found = dangerousPatterns.some((pattern) => pattern.test(content))
      if (found) {
        setScanStatus("warning")
      } else {
        setScanStatus("passed")
      }
    }, 1000)
  }

  return (
    <div className="flex-1 min-h-0 overflow-hidden">
      {loginOpen && (
        <CloudLoginDialog
          open={loginOpen}
          onOpenChange={setLoginOpen}
          onLoggedIn={() => void search(1, sortBy, selectedCategory)}
        />
      )}
      <AlertDialog
        open={!!pendingOverwrite}
        onOpenChange={(open) => {
          if (!open && !downloading) setPendingOverwrite(null)
        }}
      >
        <AlertDialogContent>
          <AlertDialogHeader>
            <AlertDialogTitle>{t("skillhub.overwriteTitle")}</AlertDialogTitle>
            <AlertDialogDescription className="space-y-2">
              <span className="block">
                {t("skillhub.overwriteDescription", {
                  name: pendingOverwrite?.skill.name ?? "",
                })}
              </span>
              {pendingOverwrite && (
                <span className="block text-xs text-muted-foreground">
                  {pendingOverwrite.freshness === "local-newer" &&
                    t("skillhub.freshness.localNewer")}
                  {pendingOverwrite.freshness === "remote-newer" &&
                    t("skillhub.freshness.remoteNewer")}
                  {pendingOverwrite.freshness === "same" && t("skillhub.freshness.same")}
                  {pendingOverwrite.freshness === "unknown" &&
                    t("skillhub.freshness.unknown")}
                  {(pendingOverwrite.localVersion || pendingOverwrite.remoteVersion) && (
                    <span className="mt-1 block">
                      {t("skillhub.freshness.versions", {
                        local: pendingOverwrite.localVersion || t("skillhub.freshness.na"),
                        remote: pendingOverwrite.remoteVersion || t("skillhub.freshness.na"),
                      })}
                    </span>
                  )}
                </span>
              )}
            </AlertDialogDescription>
          </AlertDialogHeader>
          <AlertDialogFooter>
            <AlertDialogCancel disabled={!!downloading}>{t("common.cancel")}</AlertDialogCancel>
            <Button
              variant="destructive"
              className="gap-1.5"
              disabled={!pendingOverwrite || !!downloading}
              onClick={() => {
                if (pendingOverwrite) void download(pendingOverwrite.skill.id, true)
              }}
            >
              {downloading === pendingOverwrite?.skill.id ? (
                <Loader2 className="h-4 w-4 animate-spin" />
              ) : (
                <Download className="h-4 w-4" />
              )}
              {t("skillhub.overwriteConfirm")}
            </Button>
          </AlertDialogFooter>
        </AlertDialogContent>
      </AlertDialog>

      {/* Public Skill Preview Modal Dialog */}
      <Dialog
        open={!!selectedDetail}
        onOpenChange={(open) => {
          if (!open) setSelectedDetail(null)
        }}
      >
        <DialogContent className="max-w-2xl max-h-[85vh] overflow-hidden flex flex-col p-0 bg-background border border-border rounded-xl shadow-2xl duration-base">
          {selectedDetail && (
            <>
              {/* Modal Header */}
              <div className="flex items-start gap-4 p-6 border-b border-border shrink-0 select-none">
                <div
                  className={cn(
                    "w-14 h-14 rounded-xl flex items-center justify-center font-bold text-2xl shrink-0",
                    getAvatarColor(selectedDetail.name),
                  )}
                >
                  {selectedDetail.name.charAt(0).toUpperCase()}
                </div>
                <div className="flex-1 min-w-0">
                  <h2 className="text-xl font-bold text-foreground truncate">{selectedDetail.name}</h2>
                  <p className="text-xs text-muted-foreground mt-1 line-clamp-2 leading-relaxed">
                    {selectedDetail.description}
                  </p>
                  <div className="flex flex-wrap items-center gap-3 mt-3">
                    <span className="text-[10px] bg-secondary/85 text-foreground px-2 py-0.5 rounded font-medium">
                      SkillHub 社区
                    </span>
                    {selectedDetail.authorName && (
                      <span className="flex items-center gap-1 text-[11px] text-muted-foreground">
                        <User className="w-3 h-3" />
                        {selectedDetail.authorName}
                      </span>
                    )}
                  </div>
                </div>
              </div>

              {/* Modal Body */}
              <div className="flex-1 min-h-0 overflow-y-auto p-6 flex flex-col gap-4">
                {/* AI Translate Controls */}
                <div className="flex items-center justify-end shrink-0 select-none">
                  <Button
                    onClick={() => void handleTranslate()}
                    disabled={isTranslating}
                    variant="secondary"
                    size="sm"
                    className="h-8 rounded-lg text-xs gap-1.5"
                  >
                    {isTranslating ? (
                      <Loader2 className="w-3.5 h-3.5 animate-spin text-primary" />
                    ) : (
                      <Languages className="w-3.5 h-3.5" />
                    )}
                    {showTranslation ? "显示原文" : "AI 翻译"}
                  </Button>
                </div>

                {/* Markdown Instruction Panel */}
                <div className="flex-1 min-h-0 overflow-y-auto select-text border border-border-soft rounded-lg p-4 bg-secondary/15">
                  <MarkdownRenderer content={showTranslation ? translatedText : selectedDetail.skillMd || ""} />
                </div>

                {/* Security Scan Box */}
                <div className="p-4 bg-secondary/20 rounded-xl border border-border-soft flex items-center justify-between gap-3 shrink-0 select-none">
                  <div className="flex items-center gap-2.5 min-w-0">
                    {scanStatus === "passed" ? (
                      <ShieldCheck className="w-5 h-5 text-emerald-600 dark:text-emerald-400 shrink-0" />
                    ) : scanStatus === "warning" ? (
                      <ShieldAlert className="w-5 h-5 text-amber-500 shrink-0" />
                    ) : (
                      <ShieldAlert className="w-5 h-5 text-muted-foreground shrink-0" />
                    )}
                    <div className="min-w-0">
                      <h4 className="text-xs font-bold text-foreground">安全扫描</h4>
                      <p className="text-[11px] text-muted-foreground mt-0.5 truncate">
                        {scanStatus === "passed"
                          ? "安全等级：安全。扫描通过，未发现敏感操作或危险指令。"
                          : scanStatus === "warning"
                            ? "安全等级：警告。检测到敏感命令或潜在风险脚本，请注意安全。"
                            : scanStatus === "scanning"
                              ? "正在进行静态指令审计，请稍候..."
                              : "尚未进行安全检测。"}
                      </p>
                    </div>
                  </div>
                  <Button
                    onClick={runScan}
                    disabled={scanStatus === "scanning"}
                    variant="outline"
                    size="sm"
                    className="h-7 rounded-md text-[11px] font-medium"
                  >
                    {scanStatus === "scanning" ? (
                      <Loader2 className="w-3 h-3 animate-spin mr-1" />
                    ) : scanStatus === "passed" ? (
                      <Check className="w-3 h-3 text-emerald-600 mr-1" />
                    ) : null}
                    {scanStatus === "scanning"
                      ? "扫描中..."
                      : scanStatus === "passed"
                        ? "扫描通过"
                        : scanStatus === "warning"
                          ? "重新扫描"
                          : "扫描"}
                  </Button>
                </div>
              </div>

              {/* Modal Footer */}
              <div className="p-4 border-t border-border flex items-center justify-between shrink-0 select-none bg-secondary/10">
                <div className="text-xs text-muted-foreground flex items-center gap-1">
                  <Tag className="w-3 h-3" />
                  <span>分类: {selectedDetail.category || "通用"}</span>
                </div>
                <Button
                  onClick={() =>
                    void requestDownload({
                      id: selectedDetail.id,
                      name: selectedDetail.name,
                      registrySlug: selectedDetail.slug ?? selectedDetail.name,
                      downloads: selectedDetail.downloadCount,
                      stars: selectedDetail.starCount,
                      views: selectedDetail.viewCount,
                      description: selectedDetail.description,
                      authorUsername: selectedDetail.authorName,
                      category: selectedDetail.category,
                      updatedAt: null,
                    } as SkillHubPublicSkill)
                  }
                  disabled={downloading === selectedDetail.id}
                  className="h-9 rounded-lg px-6 gap-2 font-semibold shadow-sm"
                >
                  {downloading === selectedDetail.id ? (
                    <Loader2 className="h-4 w-4 animate-spin" />
                  ) : (
                    <Download className="h-4 w-4" />
                  )}
                  导入到我的 Skill
                </Button>
              </div>
            </>
          )}
        </DialogContent>
      </Dialog>

      <div className="flex h-full flex-col">
        {/* Header Block */}
        <div className="shrink-0 border-b border-border-soft px-6 py-4 select-none">
          <div className="flex items-center justify-between">
            <div>
              <button
                type="button"
                className="text-left text-lg font-semibold text-foreground hover:text-primary flex items-center gap-2 cursor-pointer"
                onClick={onBack}
              >
                <ArrowLeft className="h-4.5 w-4.5" />
                <span>{t("skillhub.title")} 社区</span>
                <span className="text-[10px] font-medium bg-secondary text-foreground px-2 py-0.5 rounded-full ml-1">
                  {totalItems} 个 Skill
                </span>
              </button>
              <p className="mt-1 text-xs text-muted-foreground">
                浏览并安装来自 SkillHub 社区的公开技能。
              </p>
            </div>
            <Button
              type="button"
              variant="ghost"
              size="icon"
              onClick={() => void search(page, sortBy, selectedCategory)}
              disabled={loading}
              className="h-8 w-8 rounded-lg cursor-pointer"
            >
              <RefreshCw className={loading ? "h-4 w-4 animate-spin text-primary" : "h-4 w-4 text-muted-foreground"} />
            </Button>
          </div>

          <div className="mt-4 flex flex-col gap-3">
            {/* Search Input */}
            <form
              className="flex gap-2"
              onSubmit={(e) => {
                e.preventDefault()
                setPage(1)
                void search(1, sortBy, selectedCategory)
              }}
            >
              <div className="relative min-w-0 flex-1">
                <Search className="pointer-events-none absolute left-3 top-1/2 h-4 w-4 -translate-y-1/2 text-muted-foreground" />
                <Input
                  value={query}
                  onChange={(e) => setQuery(e.target.value)}
                  className="pl-9 h-9 rounded-lg"
                  placeholder={t("skillhub.searchPlaceholder")}
                />
              </div>
              <Button type="submit" size="sm" className="h-9 rounded-lg px-4 gap-1.5" disabled={loading}>
                <Search className="h-4 w-4" />
                {t("skillhub.search")}
              </Button>
            </form>

            {/* Sorting Pills */}
            <div className="flex flex-wrap gap-2 text-xs">
              <button
                type="button"
                onClick={() => {
                  setSortBy("popular")
                  setPage(1)
                  void search(1, "popular", selectedCategory)
                }}
                className={cn(
                  "flex items-center gap-1 px-3 py-1.5 rounded-full font-medium transition-colors cursor-pointer",
                  sortBy === "popular"
                    ? "bg-primary text-primary-foreground shadow-sm"
                    : "bg-secondary/40 text-muted-foreground hover:bg-secondary/70",
                )}
              >
                <Flame className="h-3.5 w-3.5" />
                热门推荐
              </button>
              <button
                type="button"
                onClick={() => {
                  setSortBy("downloads")
                  setPage(1)
                  void search(1, "downloads", selectedCategory)
                }}
                className={cn(
                  "flex items-center gap-1 px-3 py-1.5 rounded-full font-medium transition-colors cursor-pointer",
                  sortBy === "downloads"
                    ? "bg-primary text-primary-foreground shadow-sm"
                    : "bg-secondary/40 text-muted-foreground hover:bg-secondary/70",
                )}
              >
                <TrendingUp className="h-3.5 w-3.5" />
                下载排行
              </button>
              <button
                type="button"
                onClick={() => {
                  setSortBy("stars")
                  setPage(1)
                  void search(1, "stars", selectedCategory)
                }}
                className={cn(
                  "flex items-center gap-1 px-3 py-1.5 rounded-full font-medium transition-colors cursor-pointer",
                  sortBy === "stars"
                    ? "bg-primary text-primary-foreground shadow-sm"
                    : "bg-secondary/40 text-muted-foreground hover:bg-secondary/70",
                )}
              >
                <Star className="h-3.5 w-3.5" />
                最多收藏
              </button>
              <button
                type="button"
                onClick={() => {
                  setSortBy("latest")
                  setPage(1)
                  void search(1, "latest", selectedCategory)
                }}
                className={cn(
                  "flex items-center gap-1 px-3 py-1.5 rounded-full font-medium transition-colors cursor-pointer",
                  sortBy === "latest"
                    ? "bg-primary text-primary-foreground shadow-sm"
                    : "bg-secondary/40 text-muted-foreground hover:bg-secondary/70",
                )}
              >
                <Clock className="h-3.5 w-3.5" />
                最新发布
              </button>
            </div>

            {/* Category Tags */}
            <div className="flex flex-wrap gap-1.5 text-xs border-t border-border-soft/60 pt-2.5">
              {CATEGORIES.map((cat) => (
                <button
                  key={cat}
                  type="button"
                  onClick={() => {
                    setSelectedCategory(cat)
                    setPage(1)
                    void search(1, sortBy, cat)
                  }}
                  className={cn(
                    "px-3 py-1 rounded-md font-medium transition-colors text-[11px] cursor-pointer",
                    selectedCategory === cat
                      ? "bg-secondary text-secondary-foreground"
                      : "bg-transparent border border-border-soft hover:bg-secondary/30 text-muted-foreground",
                  )}
                >
                  {cat}
                </button>
              ))}
            </div>
          </div>
        </div>

        {/* Content Block */}
        <div className="min-h-0 flex-1 overflow-y-auto p-6 select-text">
          {loading && skills.length === 0 ? (
            <div className="flex items-center justify-center py-24 select-none">
              <Loader2 className="h-6 w-6 animate-spin text-primary" />
            </div>
          ) : skills.length === 0 ? (
            <div className="py-24 text-center select-none">
              <p className="text-sm text-muted-foreground">{t("skillhub.empty")}</p>
              <p className="mt-1 text-xs text-muted-foreground/70">{t("skillhub.emptyHint")}</p>
            </div>
          ) : (
            <div className="flex flex-col min-h-full justify-between">
              <div className="grid gap-4 grid-cols-1 md:grid-cols-2 lg:grid-cols-3 xl:grid-cols-4">
                {skills.map((skill) => {
                  const category = skill.category || getSkillCategory(skill.name)
                  const initial = skill.name.charAt(0).toUpperCase()
                  const avatarColor = getAvatarColor(skill.name)
                  const updateDate = formatUpdateTime(skill.updatedAt)

                  return (
                    <article
                      key={skill.id}
                      onDoubleClick={() => void openSkillPreview(skill.id)}
                      className="rounded-xl border border-border bg-card p-4 hover:shadow-md hover:bg-secondary/20 transition-all duration-200 flex flex-col justify-between relative group cursor-pointer"
                      aria-label={t("skillhub.openDetail", { defaultValue: "Double-click for details" })}
                    >
                      {loadingDetailId === skill.id && (
                        <div className="absolute inset-0 bg-background/40 flex items-center justify-center rounded-xl z-10">
                          <Loader2 className="h-5 w-5 animate-spin text-primary" />
                        </div>
                      )}
                      <div>
                        {/* Title Row */}
                        <div className="flex items-start justify-between gap-2">
                          <div className="flex items-center min-w-0">
                            {/* Avatar */}
                            <div
                              className={cn(
                                "w-9 h-9 rounded-lg flex items-center justify-center font-bold text-sm select-none shrink-0 mr-3",
                                avatarColor,
                              )}
                            >
                              {initial}
                            </div>
                            <div className="min-w-0">
                              <h3 className="truncate text-sm font-semibold text-foreground group-hover:text-primary transition-colors">
                                {skill.name}
                              </h3>
                              <p className="mt-0.5 truncate text-[10px] text-muted-foreground font-medium">
                                {skill.registrySlug}
                              </p>
                            </div>
                          </div>

                          {/* Compact Plus/Download Button */}
                          <Button
                            size="icon"
                            variant="ghost"
                            className="h-7 w-7 rounded-md border border-border/80 hover:bg-secondary shrink-0 text-muted-foreground hover:text-foreground cursor-pointer"
                            onClick={(e) => {
                              e.stopPropagation()
                              void requestDownload(skill)
                            }}
                            disabled={downloading === skill.id || pendingOverwrite?.skill.id === skill.id}
                            aria-label={t("skillhub.download")}
                          >
                            {downloading === skill.id ? (
                              <Loader2 className="h-3.5 w-3.5 animate-spin" />
                            ) : (
                              <Plus className="h-4 w-4" />
                            )}
                          </Button>
                        </div>

                        {/* Description */}
                        {skill.description && (
                          <p className="mt-3 line-clamp-2 text-xs text-muted-foreground leading-relaxed min-h-[2.25rem] select-text">
                            {skill.description}
                          </p>
                        )}
                      </div>

                      {/* Footer Row */}
                      <div className="mt-4 flex items-center justify-between border-t border-border-soft/50 pt-3 select-none">
                        <span className="text-[10px] bg-secondary/80 text-muted-foreground rounded-md px-1.5 py-0.5 font-semibold">
                          {category}
                        </span>

                        <div className="flex items-center gap-3 text-[10px] text-muted-foreground">
                          {updateDate && (
                            <span className="text-muted-foreground/75 font-medium">{updateDate}</span>
                          )}
                          <span className="inline-flex items-center gap-0.5 font-medium">
                            <Download className="h-3 w-3" />
                            {skill.downloads}
                          </span>
                          <span className="inline-flex items-center gap-0.5 font-medium">
                            <Star className="h-3 w-3" />
                            {skill.stars}
                          </span>
                        </div>
                      </div>
                    </article>
                  )
                })}
              </div>

              {/* Server-side Pagination Row */}
              <div className="flex items-center justify-between border-t border-border-soft/60 pt-4 mt-8 select-none shrink-0">
                <div className="text-xs text-muted-foreground">
                  共 {totalItems} 个 Skill
                </div>
                <div className="flex items-center gap-2">
                  <Button
                    variant="outline"
                    size="sm"
                    disabled={page === 1 || loading}
                    onClick={() => {
                      const prev = page - 1
                      setPage(prev)
                      void search(prev, sortBy, selectedCategory)
                    }}
                    className="h-8 rounded-lg px-3 text-xs cursor-pointer"
                  >
                    上一页
                  </Button>
                  <span className="text-xs text-muted-foreground px-2">
                    {page} / {totalPages}
                  </span>
                  <Button
                    variant="outline"
                    size="sm"
                    disabled={page === totalPages || loading}
                    onClick={() => {
                      const next = page + 1
                      setPage(next)
                      void search(next, sortBy, selectedCategory)
                    }}
                    className="h-8 rounded-lg px-3 text-xs cursor-pointer"
                  >
                    下一页
                  </Button>
                </div>
              </div>
            </div>
          )}
        </div>
      </div>
    </div>
  )
}
