import { useCallback, useEffect, useRef, useState } from "react"

import type { SkillStatusEntry } from "@/components/settings/types"
import { getTransport } from "@/lib/transport-provider"
import type { MySkillCloudEntry } from "@/types/skillhub"

export function useMySkills(options?: { autoSyncCloud?: boolean }) {
  const autoSyncCloud = options?.autoSyncCloud ?? false
  const [skills, setSkills] = useState<MySkillCloudEntry[]>([])
  const [skillStatuses, setSkillStatuses] = useState<SkillStatusEntry[]>([])
  const [loading, setLoading] = useState(true)
  const [error, setError] = useState<string | null>(null)
  const syncedOnceRef = useRef(false)

  const loadLocal = useCallback(async () => {
    const [nextSkills, nextStatuses] = await Promise.all([
      getTransport().call<MySkillCloudEntry[]>("my_skills_list"),
      getTransport()
        .call<SkillStatusEntry[]>("get_skills_status")
        .catch(() => [] as SkillStatusEntry[]),
    ])
    setSkills(nextSkills)
    setSkillStatuses(nextStatuses)
    return nextSkills
  }, [])

  const reload = useCallback(async () => {
    setLoading(true)
    setError(null)
    try {
      return await loadLocal()
    } catch (e) {
      // Keep the previous list on local refresh failure so the page stays usable.
      setError(e instanceof Error ? e.message : String(e))
      return null
    } finally {
      setLoading(false)
    }
  }, [loadLocal])

  /** Explicit cloud sync for the current logged-in account. */
  const refreshCloud = useCallback(async () => {
    setLoading(true)
    setError(null)
    try {
      const [next, nextStatuses] = await Promise.all([
        getTransport().call<MySkillCloudEntry[]>("my_skills_refresh_cloud"),
        getTransport()
          .call<SkillStatusEntry[]>("get_skills_status")
          .catch(() => [] as SkillStatusEntry[]),
      ])
      setSkills(next)
      setSkillStatuses(nextStatuses)
      return next
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e))
      // Fall back to local list so the page remains usable.
      try {
        await loadLocal()
      } catch {
        // ignore secondary failure
      }
      return null
    } finally {
      setLoading(false)
    }
  }, [loadLocal])

  /**
   * Page refresh:
   * - logged-out / autoSyncCloud=false → local only
   * - logged-in My Skills page → local + cloud publish state sync
   */
  const refresh = useCallback(
    async (syncCloud: boolean) => {
      if (syncCloud) return refreshCloud()
      return reload()
    },
    [refreshCloud, reload],
  )

  const submitReview = useCallback(async (localSkillName: string) => {
    const updated = await getTransport().call<MySkillCloudEntry>("my_skills_submit_review", {
      localSkillName,
    })
    setSkills((prev) => prev.map((row) => (row.local.name === localSkillName ? updated : row)))
    return updated
  }, [])

  useEffect(() => {
    let cancelled = false
    ;(async () => {
      setLoading(true)
      setError(null)
      try {
        await loadLocal()
        if (!cancelled && autoSyncCloud && !syncedOnceRef.current) {
          syncedOnceRef.current = true
          try {
            const next = await getTransport().call<MySkillCloudEntry[]>("my_skills_refresh_cloud")
            if (!cancelled) setSkills(next)
          } catch (e) {
            if (!cancelled) {
              setError(e instanceof Error ? e.message : String(e))
            }
          }
        }
      } catch (e) {
        if (!cancelled) setError(e instanceof Error ? e.message : String(e))
      } finally {
        if (!cancelled) setLoading(false)
      }
    })()
    return () => {
      cancelled = true
    }
  }, [autoSyncCloud, loadLocal])

  return {
    skills,
    skillStatuses,
    loading,
    error,
    reload,
    /** Local status refresh. */
    refreshLocal: reload,
    refreshCloud,
    /** Context-aware refresh used by the page refresh button. */
    refresh,
    submitReview,
  }
}
