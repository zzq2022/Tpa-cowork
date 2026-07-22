import { useState, useEffect, useRef } from "react"
import { getTransport } from "@/lib/transport-provider"
import { useTranslation } from "react-i18next"
import { cn } from "@/lib/utils"
import { logger } from "@/lib/logger"
import { Button } from "@/components/ui/button"
import { Check, Loader2, LogIn, LogOut } from "lucide-react"

import { type UserConfig, LANGUAGE_OPTIONS, PRESET_STYLES } from "./types"
import AvatarSection from "./AvatarSection"
import ProfileForm from "./ProfileForm"
import PersonalInfoSection from "./PersonalInfoSection"
import { useAvatarUpload } from "@/hooks/useAvatarUpload"
import { useCloudSession } from "@/hooks/useCloudSession"
import CloudLoginDialog from "@/components/cloud/CloudLoginDialog"

export default function UserProfilePanel({ onSaved }: { onSaved?: () => void } = {}) {
  const { t, i18n } = useTranslation()
  const [config, setConfig] = useState<UserConfig>({})
  const [saving, setSaving] = useState(false)
  const [saveStatus, setSaveStatus] = useState<"idle" | "saved" | "failed">("idle")
  const [customStyle, setCustomStyle] = useState(false)
  const [customGender, setCustomGender] = useState(false)
  const composingRef = useRef(false)

  const cloud = useCloudSession()
  const cloudUsername = cloud.session?.user.username
  const [showCloudLogin, setShowCloudLogin] = useState(false)

  const { cropSrc, handleAvatarPick, handleCropCancel, handleCropConfirm } =
    useAvatarUpload({
      fileName: () => `user_${Date.now()}.png`,
      logCategory: "UserProfilePanel",
      onSaved: (path) => update({ avatar: path }),
    })

  useEffect(() => {
    Promise.all([
      getTransport().call<UserConfig>("get_user_config"),
      getTransport().call<string>("get_system_timezone").catch(() => "UTC"),
    ])
      .then(([cfg, sysTz]) => {
        if (!cfg.timezone) cfg.timezone = sysTz
        if (!cfg.language) {
          const matched = LANGUAGE_OPTIONS.find((l) => i18n.language.startsWith(l.code))
          if (matched) cfg.language = matched.code
        }
        setConfig(cfg)
        if (cfg.responseStyle && !PRESET_STYLES.includes(cfg.responseStyle)) {
          setCustomStyle(true)
        }
        if (cfg.gender && !["male", "female"].includes(cfg.gender)) {
          setCustomGender(true)
        }
      })
      .catch((e: unknown) =>
        logger.error("settings", "UserProfilePanel::load", "Failed to load user config", e),
      )
  }, [i18n.language])

  const handleSave = async () => {
    setSaving(true)
    try {
      await getTransport().call("save_user_config", { config })
      setSaveStatus("saved")
      onSaved?.()
      setTimeout(() => setSaveStatus("idle"), 2000)
    } catch (e) {
      logger.error("settings", "UserProfilePanel::save", "Failed to save user config", e)
      setSaveStatus("failed")
      setTimeout(() => setSaveStatus("idle"), 2000)
    } finally {
      setSaving(false)
    }
  }

  const update = (patch: Partial<UserConfig>) => {
    setConfig((prev) => ({ ...prev, ...patch }))
  }

  /** Props for text inputs that handle IME composition correctly */
  const textInputProps = (field: keyof UserConfig) => ({
    value: (config[field] as string) ?? "",
    onChange: (e: React.ChangeEvent<HTMLInputElement | HTMLTextAreaElement>) => {
      update({ [field]: e.target.value })
    },
    onCompositionStart: () => {
      composingRef.current = true
    },
    onCompositionEnd: (e: React.CompositionEvent<HTMLInputElement | HTMLTextAreaElement>) => {
      composingRef.current = false
      update({ [field]: (e.target as HTMLInputElement).value })
    },
    onBlur: (e: React.FocusEvent<HTMLInputElement | HTMLTextAreaElement>) => {
      if (!e.target.value) update({ [field]: null })
    },
  })

  return (
    <div className="flex-1 flex flex-col min-h-0 overflow-hidden">
      {showCloudLogin && (
        <CloudLoginDialog
          open={showCloudLogin}
          onOpenChange={setShowCloudLogin}
          onLoggedIn={() => void cloud.reload()}
        />
      )}
      {/* Scrollable content */}
      <div className="flex-1 overflow-y-auto p-6">
        <div className="w-full">
          <h2 className="text-lg font-semibold text-foreground mb-1">{t("settings.profileSettings")}</h2>
          <p className="text-xs text-muted-foreground mb-5">{t("settings.profileDesc")}</p>

          <div className="space-y-5">
            {/* Cloud account (SkillHub session) */}
            <div className="rounded-xl border border-border-soft bg-secondary/10 p-4 text-sm">
              <div className="flex items-center justify-between">
                <div>
                  <h3 className="font-semibold text-foreground">{t("cloud.title", "Cloud account")}</h3>
                  <p className="text-xs text-muted-foreground mt-0.5">
                    {cloudUsername
                      ? t("cloud.signedInAs", { username: cloudUsername })
                      : t("cloud.notSignedIn", "Not signed in")}
                  </p>
                </div>
                {cloudUsername ? (
                  <Button
                    variant="outline"
                    size="sm"
                    className="text-destructive border-destructive/20 hover:bg-destructive/10 hover:text-destructive transition-colors"
                    onClick={() => void cloud.logout()}
                  >
                    <LogOut className="h-3.5 w-3.5 mr-1.5" />
                    {t("cloud.logout", "Log out")}
                  </Button>
                ) : (
                  <Button variant="default" size="sm" onClick={() => setShowCloudLogin(true)}>
                    <LogIn className="h-3.5 w-3.5 mr-1.5" />
                    {t("cloud.login", "Log in")}
                  </Button>
                )}
              </div>
            </div>

            <AvatarSection
              avatar={config.avatar}
              cropSrc={cropSrc}
              onAvatarPick={handleAvatarPick}
              onCropConfirm={handleCropConfirm}
              onCropCancel={handleCropCancel}
            />

            <ProfileForm
              config={config}
              customStyle={customStyle}
              customGender={customGender}
              onCustomStyleChange={setCustomStyle}
              onCustomGenderChange={setCustomGender}
              update={update}
              textInputProps={textInputProps}
            />

            <PersonalInfoSection
              config={config}
              update={update}
              textInputProps={textInputProps}
            />
          </div>
        </div>
      </div>

      {/* ── Save — fixed bottom-right ── */}
      <div className="shrink-0 flex justify-end px-6 py-3 border-t border-border/30">
        <Button
          className={cn(
            saveStatus === "saved" && "bg-green-500/10 text-green-600 hover:bg-green-500/20",
            saveStatus === "failed" && "bg-destructive/10 text-destructive hover:bg-destructive/20",
          )}
          onClick={handleSave}
          disabled={saving}
        >
          {saving ? (
            <span className="flex items-center gap-1.5">
              <Loader2 className="h-3.5 w-3.5 animate-spin" />
              {t("common.saving")}
            </span>
          ) : saveStatus === "saved" ? (
            <span className="flex items-center gap-1.5">
              <Check className="h-3.5 w-3.5" />
              {t("common.saved")}
            </span>
          ) : saveStatus === "failed" ? (
            t("common.saveFailed")
          ) : (
            t("common.save")
          )}
        </Button>
      </div>
    </div>
  )
}
