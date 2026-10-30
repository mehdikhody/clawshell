# 13: Shortcuts per account

**What to build:** An "Add shortcuts" action in the ⋯ menu creates a "Claude - <name>" shortcut on the Desktop and in the Start menu. Each shortcut carries the account's badged icon file and launches through clawshell, which focuses the account if it's already running. Shortcuts are updated on rename and colour change, and removed on delete. See spec: Launching and windows (Shortcuts).

**Blocked by:** 12

**Status:** ready-for-agent

- [ ] Desktop and Start-menu shortcuts are created with the badged icon
- [ ] Clicking a shortcut launches the account, or focuses it if it's open
- [ ] Rename and colour change update the shortcuts; delete removes them
- [ ] Shortcuts keep working after Claude updates itself (they don't point at a versioned exe path)
- [ ] Checked by hand on Windows 11
