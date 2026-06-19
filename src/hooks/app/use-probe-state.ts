import { useCallback, useEffect, useRef, useState } from "react"
import type { CachedUsageSnapshot, PluginOutput } from "@/lib/plugin-types"
import type { PluginState } from "@/hooks/app/types"

type UseProbeStateArgs = {
  onProbeResult?: () => void
}

export function useProbeState({ onProbeResult }: UseProbeStateArgs) {
  const [pluginStates, setPluginStates] = useState<Record<string, PluginState>>({})

  const pluginStatesRef = useRef(pluginStates)
  useEffect(() => {
    pluginStatesRef.current = pluginStates
  }, [pluginStates])

  const manualRefreshIdsRef = useRef<Set<string>>(new Set())

  const updatePluginStates = useCallback(
    (
      updater: (
        previousStates: Record<string, PluginState>
      ) => Record<string, PluginState>
    ) => {
      const nextStates = updater(pluginStatesRef.current)
      pluginStatesRef.current = nextStates
      setPluginStates(nextStates)
    },
    []
  )

  const getErrorMessage = useCallback((output: PluginOutput) => {
    // Treat an Error badge ANYWHERE in the output as an error (matching the
    // backend's .any() semantics), not only when it's the sole line — otherwise
    // a probe that emits an error alongside other lines looks like a fresh
    // success and shows a misleading "just updated" timestamp.
    const errorLine = output.lines.find(
      (line) => line.type === "badge" && line.label === "Error"
    )
    if (errorLine && errorLine.type === "badge") {
      return errorLine.text || "Couldn't update data. Try again?"
    }
    return null
  }, [])

  const setLoadingForPlugins = useCallback((ids: string[]) => {
    updatePluginStates((prev) => {
      const next = { ...prev }
      for (const id of ids) {
        const existing = prev[id]
        next[id] = {
          data: existing?.data ?? null,
          loading: true,
          error: null,
          lastManualRefreshAt: existing?.lastManualRefreshAt ?? null,
          lastUpdatedAt: existing?.lastUpdatedAt ?? null,
        }
      }
      return next
    })
  }, [updatePluginStates])

  const setErrorForPlugins = useCallback((ids: string[], error: string) => {
    updatePluginStates((prev) => {
      const next = { ...prev }
      for (const id of ids) {
        const existing = prev[id]
        next[id] = {
          data: existing?.data ?? null,
          loading: false,
          error,
          lastManualRefreshAt: existing?.lastManualRefreshAt ?? null,
          lastUpdatedAt: existing?.lastUpdatedAt ?? null,
        }
      }
      return next
    })
  }, [updatePluginStates])

  // Seed cards from the Rust cache (last successful probes, persisted across
  // restarts) when the panel opens, before the live refresh runs. This makes the
  // panel show known-good data immediately, and — combined with handleProbeResult
  // keeping existing data on error — means a transient probe failure never blanks
  // a card. Only seeds providers that have no live data/loading state yet.
  const hydrateFromCache = useCallback(
    (snapshots: CachedUsageSnapshot[]) => {
      if (!Array.isArray(snapshots) || !snapshots.length) return
      updatePluginStates((prev) => {
        const next = { ...prev }
        for (const snap of snapshots) {
          const existing = prev[snap.providerId]
          if (existing?.data || existing?.loading) continue
          const fetchedMs = Date.parse(snap.fetchedAt)
          next[snap.providerId] = {
            data: {
              providerId: snap.providerId,
              displayName: snap.displayName,
              plan: snap.plan,
              lines: snap.lines,
              iconUrl: "",
            },
            loading: false,
            error: null,
            lastManualRefreshAt: existing?.lastManualRefreshAt ?? null,
            lastUpdatedAt: Number.isFinite(fetchedMs) ? fetchedMs : null,
          }
        }
        return next
      })
    },
    [updatePluginStates]
  )

  const handleProbeResult = useCallback(
    (output: PluginOutput) => {
      const errorMessage = getErrorMessage(output)
      const isManual = manualRefreshIdsRef.current.has(output.providerId)
      if (isManual) {
        manualRefreshIdsRef.current.delete(output.providerId)
      }

      const now = Date.now()
      updatePluginStates((prev) => {
        const existing = prev[output.providerId]
        return {
          ...prev,
          [output.providerId]: {
            data: errorMessage ? (existing?.data ?? null) : output,
            loading: false,
            error: errorMessage,
            lastManualRefreshAt: !errorMessage && isManual
              ? now
              : existing?.lastManualRefreshAt ?? null,
            lastUpdatedAt: errorMessage ? (existing?.lastUpdatedAt ?? null) : now,
          },
        }
      })

      onProbeResult?.()
    },
    [getErrorMessage, onProbeResult, updatePluginStates]
  )

  return {
    pluginStates,
    pluginStatesRef,
    manualRefreshIdsRef,
    setLoadingForPlugins,
    setErrorForPlugins,
    handleProbeResult,
    hydrateFromCache,
  }
}
