# 01: Quit Claude from an account's context menu

**What to build:** A running account's context menu on the Accounts tab gets a "Quit Claude" item, placed after Open folder. Choosing it ends every Claude process that belongs to that account at once, including Claude windows hidden in Claude's own tray. The default account works the same way. It runs with no confirm dialog. Afterwards the account's status and the clawshell tray update straight away. If quitting fails, the screen's existing failure alert says "Couldn't quit Claude". See spec: Quit Claude.

**Blocked by:** None (can start immediately)

**Status:** resolved

- [x] The item appears only when the account is running, for the default account too
- [x] Choosing it ends exactly the Claude processes of that account's profile, hidden ones included
- [x] No confirm dialog; the account status and tray refresh right after
- [x] A failure shows "Couldn't quit Claude" in the existing alert
- [x] Core tests: only that profile's PIDs are ended; the default account works
- [x] Component test: the item shows only for running accounts and calls `quit_account`
- [x] A user-facing entry is added under `## [Unreleased]` in `CHANGELOG.md`
