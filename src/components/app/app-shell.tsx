import { useEffect, useRef } from "react"
import { getCurrentWindow } from "@tauri-apps/api/window"
import { LogicalPosition } from "@tauri-apps/api/dpi"
import { useShallow } from "zustand/react/shallow"
import { AppContent, type AppContentActionProps } from "@/components/app/app-content"
import { PanelFooter } from "@/components/panel-footer"
import { SideNav, type NavPlugin, type PluginContextAction } from "@/components/side-nav"
import type { DisplayPluginState } from "@/hooks/app/use-app-plugin-views"
import type { SettingsPluginState } from "@/hooks/app/use-settings-plugin-list"
import { useAppVersion } from "@/hooks/app/use-app-version"
import { usePanel } from "@/hooks/app/use-panel"
import { useAppUpdate } from "@/hooks/use-app-update"
import { useAppUiStore } from "@/stores/app-ui-store"

const ARROW_OVERHEAD_PX = 37

type AppShellProps = {
  onRefreshAll: () => void
  navPlugins: NavPlugin[]
  displayPlugins: DisplayPluginState[]
  settingsPlugins: SettingsPluginState[]
  autoUpdateNextAt: number | null
  selectedPlugin: DisplayPluginState | null
  onPluginContextAction: (pluginId: string, action: PluginContextAction) => void
  isPluginRefreshAvailable: (pluginId: string) => boolean
  onNavReorder: (orderedIds: string[]) => void
  appContentProps: AppContentActionProps
}

export function AppShell({
  onRefreshAll,
  navPlugins,
  displayPlugins,
  settingsPlugins,
  autoUpdateNextAt,
  selectedPlugin,
  onPluginContextAction,
  isPluginRefreshAvailable,
  onNavReorder,
  appContentProps,
}: AppShellProps) {
  const {
    activeView,
    setActiveView,
    showAbout,
    setShowAbout,
  } = useAppUiStore(
    useShallow((state) => ({
      activeView: state.activeView,
      setActiveView: state.setActiveView,
      showAbout: state.showAbout,
      setShowAbout: state.setShowAbout,
    }))
  )

  const {
    containerRef,
    scrollRef,
    canScrollDown,
    maxPanelHeightPx,
  } = usePanel({
    activeView,
    setActiveView,
    showAbout,
    setShowAbout,
    displayPlugins,
  })

  const appVersion = useAppVersion()
  const { updateStatus, triggerInstall, checkForUpdates } = useAppUpdate()

  // On Windows the panel is a borderless window with no title bar, so it can get
  // stuck partly off-screen with no way to move it. Give it a drag handle (the top
  // bar). We move the window ourselves with setPosition rather than the OS drag
  // loop (start_dragging): start_dragging briefly drops focus, which the panel's
  // hide-on-blur turns into an instant disappear the moment you grab it.
  // setPosition keeps focus (SWP_NOACTIVATE), so the panel stays put while moving.
  // Gated to Windows — the macOS build is an anchored NSPanel and must not move.
  const isWindows =
    typeof navigator !== "undefined" && /Windows/i.test(navigator.userAgent)

  const dragCleanupRef = useRef<(() => void) | null>(null)
  useEffect(() => () => dragCleanupRef.current?.(), [])

  const handlePanelDragStart = async (e: React.MouseEvent) => {
    if (!isWindows || e.button !== 0) return
    e.preventDefault()
    const appWindow = getCurrentWindow()
    // CSS screen coords (e.screenX/Y) are logical pixels, the same unit as Tauri's
    // LogicalPosition. Work entirely in logical space and let Tauri convert to
    // physical per the monitor the window currently sits on — so dragging across
    // monitors with different DPI scale factors stays correct (no stale dpr).
    const startCx = e.screenX
    const startCy = e.screenY
    let originX: number
    let originY: number
    try {
      const scale = await appWindow.scaleFactor()
      const phys = await appWindow.outerPosition()
      originX = phys.x / scale
      originY = phys.y / scale
    } catch {
      return
    }

    let rafId: number | null = null
    let lastCx = startCx
    let lastCy = startCy
    const applyMove = () => {
      rafId = null
      const x = originX + (lastCx - startCx)
      const y = originY + (lastCy - startCy)
      void appWindow.setPosition(new LogicalPosition(x, y)).catch(() => {})
    }
    const onMove = (ev: MouseEvent) => {
      lastCx = ev.screenX
      lastCy = ev.screenY
      if (rafId == null) rafId = requestAnimationFrame(applyMove)
    }
    const cleanup = () => {
      if (rafId != null) cancelAnimationFrame(rafId)
      window.removeEventListener("mousemove", onMove)
      window.removeEventListener("mouseup", cleanup)
      dragCleanupRef.current = null
    }
    dragCleanupRef.current?.()
    dragCleanupRef.current = cleanup
    window.addEventListener("mousemove", onMove)
    window.addEventListener("mouseup", cleanup)
  }

  return (
    <div
      ref={containerRef}
      tabIndex={-1}
      className="flex flex-col items-center p-6 pt-1.5 bg-transparent outline-none"
    >
      <div className="tray-arrow" />
      <div
        className="relative bg-card rounded-xl overflow-hidden select-none w-full border shadow-lg flex flex-col"
        style={maxPanelHeightPx ? { maxHeight: `${maxPanelHeightPx - ARROW_OVERHEAD_PX}px` } : undefined}
      >
        {isWindows && (
          <div
            onMouseDown={handlePanelDragStart}
            aria-hidden="true"
            title="Drag to move"
            className="flex h-5 w-full shrink-0 cursor-grab items-center justify-center active:cursor-grabbing"
          >
            <div className="pointer-events-none h-1 w-8 rounded-full bg-border" />
          </div>
        )}
        <div className="flex flex-1 min-h-0 flex-row">
          <SideNav
            activeView={activeView}
            onViewChange={setActiveView}
            plugins={navPlugins}
            onPluginContextAction={onPluginContextAction}
            isPluginRefreshAvailable={isPluginRefreshAvailable}
            onReorder={onNavReorder}
          />
          <div className="flex-1 flex flex-col px-3 pt-2 pb-1.5 min-w-0 bg-card dark:bg-muted/50">
            <div className="relative flex-1 min-h-0">
              <div ref={scrollRef} className="h-full overflow-y-auto scrollbar-none">
                <AppContent
                  {...appContentProps}
                  displayPlugins={displayPlugins}
                  settingsPlugins={settingsPlugins}
                  selectedPlugin={selectedPlugin}
                />
              </div>
              <div
                className={`pointer-events-none absolute inset-x-0 bottom-0 h-14 bg-gradient-to-t from-card dark:from-muted/50 to-transparent transition-opacity duration-200 ${canScrollDown ? "opacity-100" : "opacity-0"}`}
              />
            </div>
            <PanelFooter
              version={appVersion}
              autoUpdateNextAt={autoUpdateNextAt}
              updateStatus={updateStatus}
              onUpdateInstall={triggerInstall}
              onUpdateCheck={checkForUpdates}
              onRefreshAll={onRefreshAll}
              showAbout={showAbout}
              onShowAbout={() => setShowAbout(true)}
              onCloseAbout={() => setShowAbout(false)}
            />
          </div>
        </div>
      </div>
    </div>
  )
}
