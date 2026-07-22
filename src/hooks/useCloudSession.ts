import { useCallback, useEffect, useState } from "react"

import { getTransport } from "@/lib/transport-provider"
import type { CloudLoginRequest, CloudSession } from "@/types/skillhub"

const CLOUD_SESSION_CHANGED = "hope:cloud-session-changed"

function publishCloudSession(session: CloudSession | null) {
  window.dispatchEvent(new CustomEvent<CloudSession | null>(CLOUD_SESSION_CHANGED, { detail: session }))
}

export function useCloudSession() {
  const [session, setSession] = useState<CloudSession | null>(null)
  const [loading, setLoading] = useState(true)

  const reload = useCallback(async () => {
    setLoading(true)
    try {
      const next = await getTransport().call<CloudSession | null>("cloud_get_session")
      setSession(next)
      publishCloudSession(next)
    } finally {
      setLoading(false)
    }
  }, [])

  const login = useCallback(async (request: CloudLoginRequest) => {
    const next = await getTransport().call<CloudSession>("cloud_login", { request })
    setSession(next)
    publishCloudSession(next)
    return next
  }, [])

  const logout = useCallback(async () => {
    await getTransport().call("cloud_logout")
    setSession(null)
    publishCloudSession(null)
  }, [])

  useEffect(() => {
    const handleCloudSessionChanged = (event: Event) => {
      setSession((event as CustomEvent<CloudSession | null>).detail)
    }

    window.addEventListener(CLOUD_SESSION_CHANGED, handleCloudSessionChanged)
    void reload()
    return () => {
      window.removeEventListener(CLOUD_SESSION_CHANGED, handleCloudSessionChanged)
    }
  }, [reload])

  return { session, loading, reload, login, logout }
}
