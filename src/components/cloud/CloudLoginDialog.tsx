import { useEffect, useState } from "react"
import { useTranslation } from "react-i18next"

import { Button } from "@/components/ui/button"
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog"
import { Input } from "@/components/ui/input"
import { useCloudSession } from "@/hooks/useCloudSession"
import { DEFAULT_SKILLHUB_SERVER_URL } from "@/types/skillhub"

interface CloudLoginDialogProps {
  open: boolean
  onOpenChange: (open: boolean) => void
  onLoggedIn?: () => void
}

export default function CloudLoginDialog({
  open,
  onOpenChange,
  onLoggedIn,
}: CloudLoginDialogProps) {
  const { t } = useTranslation()
  const { login, session } = useCloudSession()
  const [serverUrl, setServerUrl] = useState(DEFAULT_SKILLHUB_SERVER_URL)
  const [username, setUsername] = useState("")
  const [password, setPassword] = useState("")
  const [submitting, setSubmitting] = useState(false)
  const [error, setError] = useState<string | null>(null)

  useEffect(() => {
    if (!open) return
    // Prefer the last logged-in server; otherwise keep / restore the product default.
    setServerUrl((current) => session?.serverUrl || current || DEFAULT_SKILLHUB_SERVER_URL)
    if (session?.user?.username) {
      setUsername((current) => current || session.user.username)
    }
  }, [open, session])

  async function submit() {
    setSubmitting(true)
    setError(null)
    try {
      await login({ serverUrl, username, password })
      onLoggedIn?.()
      onOpenChange(false)
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e))
    } finally {
      setSubmitting(false)
    }
  }

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent>
        <DialogHeader>
          <DialogTitle>{t("cloud.loginTitle")}</DialogTitle>
          <DialogDescription className="sr-only">{t("cloud.loginDescription")}</DialogDescription>
        </DialogHeader>
        <div className="space-y-3">
          <Input
            value={serverUrl}
            onChange={(e) => setServerUrl(e.target.value)}
            placeholder={t("cloud.serverUrl")}
            autoComplete="url"
          />
          <Input
            value={username}
            onChange={(e) => setUsername(e.target.value)}
            placeholder={t("cloud.username")}
            autoComplete="username"
          />
          <Input
            value={password}
            onChange={(e) => setPassword(e.target.value)}
            type="password"
            placeholder={t("cloud.password")}
            autoComplete="current-password"
          />
          {error && <p className="text-xs text-destructive">{error}</p>}
          <Button
            className="w-full"
            onClick={submit}
            disabled={submitting || !serverUrl || !username || !password}
          >
            {submitting ? t("cloud.loggingIn") : t("cloud.login")}
          </Button>
        </div>
      </DialogContent>
    </Dialog>
  )
}
