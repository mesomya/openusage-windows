import { useEffect, useState } from "react"
import { openUrl } from "@tauri-apps/plugin-opener"
import { X } from "lucide-react"
import { Alert, AlertDescription, AlertTitle } from "@/components/ui/alert"
import { Button } from "@/components/ui/button"
import {
  loadRetirementNoticeDismissedAt,
  saveRetirementNoticeDismissedAt,
  shouldShowRetirementNotice,
} from "@/lib/settings"

const NEW_APP_URL = "https://www.openusage.ai"

export function RetirementNotice() {
  const [visible, setVisible] = useState(false)

  useEffect(() => {
    let isMounted = true

    const evaluate = async () => {
      try {
        const dismissedAt = await loadRetirementNoticeDismissedAt()
        if (!isMounted) return
        setVisible(shouldShowRetirementNotice(dismissedAt, Date.now()))
      } catch (error) {
        console.error("Failed to load retirement notice state:", error)
        // Fail open: a load error shouldn't silently swallow the retirement
        // reminder, so show it rather than hiding it.
        if (isMounted) setVisible(true)
      }
    }

    evaluate()

    return () => {
      isMounted = false
    }
  }, [])

  if (!visible) return null

  const handleGetNewApp = () => {
    openUrl(NEW_APP_URL).catch(console.error)
  }

  const handleDismiss = async () => {
    setVisible(false)
    try {
      await saveRetirementNoticeDismissedAt(Date.now())
    } catch (error) {
      console.error("Failed to save retirement notice dismissal:", error)
    }
  }

  return (
    <Alert className="relative mb-3 p-3 border-destructive/50 dark:border-destructive">
      <Button
        type="button"
        variant="ghost"
        size="icon-xs"
        onClick={handleDismiss}
        aria-label="Dismiss"
        className="absolute right-1.5 top-1.5 text-muted-foreground"
      >
        <X />
      </Button>
      <AlertTitle className="pr-6 text-sm">OpenUsage Has Moved</AlertTitle>
      <AlertDescription className="mt-1 text-xs text-muted-foreground">
        This version is retired and won't receive any updates. Switch to the new
        app by clicking the button below.
      </AlertDescription>
      <Button
        type="button"
        size="xs"
        onClick={handleGetNewApp}
        className="mt-2 w-full"
      >
        Get the New App
      </Button>
    </Alert>
  )
}
