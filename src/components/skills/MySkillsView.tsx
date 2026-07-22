import { useMemo, useState } from "react"
import { useTranslation } from "react-i18next"

import CloudLoginDialog from "@/components/cloud/CloudLoginDialog"
import MySkillList from "@/components/skills/MySkillList"
import SkillDetailView from "@/components/settings/skills-panel/SkillDetailView"
import type { SkillDetail } from "@/components/settings/skills-panel/types"
import type { SkillStatusEntry } from "@/components/settings/types"
import { Button } from "@/components/ui/button"
import { useCloudSession } from "@/hooks/useCloudSession"
import { useMySkills } from "@/hooks/useMySkills"
import { logger } from "@/lib/logger"
import { getTransport } from "@/lib/transport-provider"

interface MySkillsViewProps {
  onBack: () => void
}

export default function MySkillsView({ onBack }: MySkillsViewProps) {
  const { t } = useTranslation()
  const cloud = useCloudSession()
  const authenticated = !!cloud.session?.authenticated
  const mine = useMySkills({ autoSyncCloud: authenticated })
  const [loginOpen, setLoginOpen] = useState(false)
  const [selectedSkill, setSelectedSkill] = useState<SkillDetail | null>(null)
  const [envStatus, setEnvStatus] = useState<Record<string, Record<string, boolean>>>({})
  const [detailStatuses, setDetailStatuses] = useState<SkillStatusEntry[]>([])
  const [envValues, setEnvValues] = useState<Record<string, string>>({})
  const [envDirty, setEnvDirty] = useState<Record<string, boolean>>({})
  const [envSaving, setEnvSaving] = useState<Record<string, boolean>>({})

  const skillStatuses = detailStatuses.length > 0 ? detailStatuses : mine.skillStatuses

  const skillStatusByName = useMemo(() => {
    const next: Record<string, SkillStatusEntry | undefined> = {}
    for (const status of skillStatuses) next[status.name] = status
    return next
  }, [skillStatuses])

  async function submitReview(localSkillName: string) {
    try {
      await mine.submitReview(localSkillName)
    } catch (e) {
      const message = e instanceof Error ? e.message : String(e)
      if (/auth|login|token|401|credential/i.test(message)) {
        setLoginOpen(true)
        return
      }
      throw e
    }
  }

  async function openSkillDetail(name: string) {
    try {
      const [detail, maskedEnv, envStatusSnapshot, statusSnapshot] = await Promise.all([
        getTransport().call<SkillDetail>("get_skill_detail", { name }),
        getTransport().call<Record<string, string>>("get_skill_env", { name }),
        getTransport().call<Record<string, Record<string, boolean>>>("get_skills_env_status"),
        getTransport().call<SkillStatusEntry[]>("get_skills_status"),
      ])
      setSelectedSkill(detail)
      setEnvValues(maskedEnv)
      setEnvStatus(envStatusSnapshot)
      setDetailStatuses(statusSnapshot)
      setEnvDirty({})
      setEnvSaving({})
    } catch (e) {
      logger.error("skills", "MySkillsView::detail", "Failed to load skill detail", e)
    }
  }

  async function openDir(path: string) {
    try {
      await getTransport().call("open_directory", { path })
    } catch (e) {
      logger.error("skills", "MySkillsView::openDir", "Failed to open directory", e)
    }
  }

  async function toggleSkill(name: string, enabled: boolean) {
    try {
      await getTransport().call("toggle_skill", { name, enabled })
      setSelectedSkill((prev) => (prev?.name === name ? { ...prev, enabled } : prev))
      setDetailStatuses((prev) => {
        const source = prev.length > 0 ? prev : mine.skillStatuses
        return source.map((status) =>
          status.name === name
            ? {
                ...status,
                disabled: !enabled,
                eligible:
                  enabled
                  && !status.blocked_by_allowlist
                  && !status.hard_blocked
                  && !status.needs_setup,
              }
            : status,
        )
      })
      await mine.refreshLocal()
    } catch (e) {
      logger.error("skills", "MySkillsView::toggle", "Failed to toggle skill", e)
    }
  }

  async function saveEnvVar(key: string) {
    if (!selectedSkill) return
    const value = envValues[key] ?? ""
    setEnvSaving((prev) => ({ ...prev, [key]: true }))
    try {
      await getTransport().call("set_skill_env_var", { skill: selectedSkill.name, key, value })
      const [maskedEnv, nextEnvStatus, nextSkillStatuses] = await Promise.all([
        getTransport().call<Record<string, string>>("get_skill_env", { name: selectedSkill.name }),
        getTransport().call<Record<string, Record<string, boolean>>>("get_skills_env_status"),
        getTransport().call<SkillStatusEntry[]>("get_skills_status"),
      ])
      setEnvValues(maskedEnv)
      setEnvStatus(nextEnvStatus)
      setDetailStatuses(nextSkillStatuses)
      setEnvDirty((prev) => ({ ...prev, [key]: false }))
      await mine.refreshLocal()
    } catch (e) {
      logger.error("skills", "MySkillsView::saveEnv", "Failed to save env var", e)
    } finally {
      setEnvSaving((prev) => ({ ...prev, [key]: false }))
    }
  }

  async function removeEnvVar(key: string) {
    if (!selectedSkill) return
    try {
      await getTransport().call("remove_skill_env_var", { skill: selectedSkill.name, key })
      const [nextEnvStatus, nextSkillStatuses] = await Promise.all([
        getTransport().call<Record<string, Record<string, boolean>>>("get_skills_env_status"),
        getTransport().call<SkillStatusEntry[]>("get_skills_status"),
      ])
      setEnvValues((prev) => {
        const next = { ...prev }
        delete next[key]
        return next
      })
      setEnvStatus(nextEnvStatus)
      setDetailStatuses(nextSkillStatuses)
      setEnvDirty((prev) => ({ ...prev, [key]: false }))
      await mine.refreshLocal()
    } catch (e) {
      logger.error("skills", "MySkillsView::removeEnv", "Failed to remove env var", e)
    }
  }

  function changeEnvValue(key: string, value: string) {
    setEnvValues((prev) => ({ ...prev, [key]: value }))
    setEnvDirty((prev) => ({ ...prev, [key]: true }))
  }

  if (selectedSkill) {
    return (
      <SkillDetailView
        skill={selectedSkill}
        envStatus={envStatus}
        status={skillStatusByName[selectedSkill.name]}
        envValues={envValues}
        envDirty={envDirty}
        envSaving={envSaving}
        onBack={() => setSelectedSkill(null)}
        onToggleSkill={toggleSkill}
        onOpenDir={openDir}
        onEnvValueChange={changeEnvValue}
        onSaveEnvVar={(key) => void saveEnvVar(key)}
        onRemoveEnvVar={(key) => void removeEnvVar(key)}
      />
    )
  }

  const username = cloud.session?.user?.username

  return (
    <div className="flex-1 min-h-0 overflow-hidden">
      {loginOpen && (
        <CloudLoginDialog
          open={loginOpen}
          onOpenChange={setLoginOpen}
          onLoggedIn={() => {
            void cloud.reload()
            void mine.refresh(true)
          }}
        />
      )}
      <div className="flex h-full flex-col">
        <div className="shrink-0 border-b border-border-soft px-6 py-4">
          <div className="flex flex-wrap items-start justify-between gap-3">
            <div className="min-w-0">
              <ButtonlessBack onBack={onBack} label={t("mySkills.title")} />
              <p className="mt-1 text-xs text-muted-foreground">
                {t("mySkills.count", { count: mine.skills.length })}
              </p>
            </div>
            <div className="flex items-center gap-2">
              {authenticated ? (
                <span className="text-xs text-muted-foreground">
                  {t("cloud.signedInAs", { username: username ?? "" })}
                </span>
              ) : (
                <Button
                  variant="outline"
                  size="sm"
                  className="h-8 text-xs"
                  onClick={() => setLoginOpen(true)}
                >
                  {t("cloud.login")}
                </Button>
              )}
            </div>
          </div>
        </div>
        <div className="min-h-0 flex-1 overflow-hidden p-6">
          <MySkillList
            rows={mine.skills}
            skillStatuses={mine.skillStatuses}
            loading={mine.loading || cloud.loading}
            canUseCloudActions={authenticated}
            onRefresh={() => mine.refresh(authenticated)}
            onSubmitReview={submitReview}
            onOpenSkill={(name) => void openSkillDetail(name)}
            onOpenDir={(path) => void openDir(path)}
            error={mine.error}
          />
        </div>
      </div>
    </div>
  )
}

function ButtonlessBack({ onBack, label }: { onBack: () => void; label: string }) {
  return (
    <button
      type="button"
      className="text-left text-lg font-semibold text-foreground hover:text-primary"
      onClick={onBack}
    >
      {label}
    </button>
  )
}
