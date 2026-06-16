# Retirement Notice

The overview shows a banner at the top, above all providers, letting users know this
build is retired and pointing them to the new app.

## Behavior
- Appears at the top of the overview (above the provider list and the empty state).
- Has a "Get the New App" link and a dismiss (x) button.
- Dismissing stores the current time in `settings.json` under
  `retirementNoticeDismissedAt`.
- The banner reappears once 7 days have passed since the last dismissal.
- If it has never been dismissed, it is shown.
- If the stored state can't be read, it fails open (shows the banner) and logs the error.

## Where it lives
- UI: `src/components/retirement-notice.tsx`, rendered by `src/pages/overview.tsx`.
- Persistence + 7-day logic: `src/lib/settings.ts`
  (`loadRetirementNoticeDismissedAt`, `saveRetirementNoticeDismissedAt`,
  `shouldShowRetirementNotice`).
- The "Get the New App" button opens `NEW_APP_URL` (https://www.openusage.ai).
