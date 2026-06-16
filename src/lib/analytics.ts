/**
 * Telemetry was removed in the Windows fork. `track` is kept as a no-op so the
 * call sites don't need to change, but it never sends anything anywhere.
 */
export function track(
  _event: string,
  _props?: Record<string, string | number>,
): void {
  // Intentionally does nothing — no analytics in this build.
}
