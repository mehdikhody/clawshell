# 06: Skeleton placeholders on the Usage and Accounts tabs

**What to build:** Placeholders shaped like the real content replace the blank or "Loading…" states on first load. On the Usage tab, the heatmap grid, the This month card and the pooled usage block each show a skeleton with their real layout until their first answer arrives. On the Accounts tab, two placeholder rows (icon, name, email line, two usage bars) replace the "Loading…" line. A section that comes back empty or failed disappears after its placeholder, as it does today. Automatic and manual refreshes keep the old data on screen. Placeholders stay still under reduced motion. Adds the shadcn-svelte Skeleton component. See spec: Skeleton loading.

**Blocked by:** None (can start immediately)

**Status:** resolved

- [x] Each lazily loaded Usage section shows its skeleton until its first answer, then its content
- [x] The Accounts tab shows two account-shaped placeholder rows while the list is not loaded
- [x] A section with no data disappears after its placeholder
- [x] Refreshes never bring placeholders back
- [x] Placeholders don't animate when reduced motion is preferred
- [x] Component tests (`mockTauri`) cover: placeholders before the first answer, removal on empty data, no return on refresh
- [x] A user-facing entry is added under `## [Unreleased]` in `CHANGELOG.md`
