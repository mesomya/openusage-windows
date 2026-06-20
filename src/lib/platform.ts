/**
 * True when the app is running on Windows (WebView2). Used to swap macOS-isms
 * — "Cmd"/"Opt" modifiers, "menu bar" wording — for the Windows equivalents
 * ("Ctrl"/"Alt", "system tray"). Computed once from the user agent.
 */
export const isWindows =
  typeof navigator !== "undefined" && /Windows/i.test(navigator.userAgent)
