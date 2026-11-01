# 10: List an account's Code sessions in the Copy sessions dialog

**What to build:** An account's context menu gets a "Copy sessions…" item. It opens a dialog that lists every Code session of that account, from every organisation, newest activity first. Each row shows the title, folder, relative last activity, and an Archived badge for archived sessions. The user can tick several sessions. The target picker and Copy button come in ticket 11. If Claude has changed its session layout, listing fails with a clear error and doesn't show a partial list. See spec: Session copy (Listing, UI, Scope).

**Blocked by:** None (can start immediately)

**Status:** resolved

- [x] The context menu item "Copy sessions…" is on every account and opens the dialog
- [x] The dialog lists the account's sessions from all organisations, skipping `deleted_*` folders, newest activity first
- [x] Each row shows title, folder, relative last activity, and an Archived badge when archived
- [x] Rows can be ticked and unticked; the dialog has a clear empty state
- [x] An unreadable layout shows a clear error, not a partial list
- [x] Core tests cover every-org reading, `deleted_*` skipping, ordering and the layout error; a component test checks the rows and ticking
- [x] A user-facing entry is added under `## [Unreleased]` in `CHANGELOG.md`
