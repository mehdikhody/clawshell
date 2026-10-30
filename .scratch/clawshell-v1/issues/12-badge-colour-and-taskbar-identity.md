# 12: Badge colour and taskbar identity

**What to build:** Each account gets a distinct colour, assigned automatically and changeable from the ⋯ menu. The colour and the account's badge letter appear on its row. Each account's Claude window gets its own taskbar button with its badged icon. This uses its own AppUserModelID, applied between window creation and first show, as in the reference switcher. A change of badge letter or colour gives the window a new identity. See spec: Launching and windows (Taskbar identity).

**Blocked by:** 04

**Status:** ready-for-agent

- [ ] New accounts get distinct auto colours; the picker in the ⋯ menu changes the colour, and it persists
- [ ] Rows show the badge colour and letter
- [ ] Each running account appears as its own taskbar button with its badge (checked by hand on Windows 11, Store build)
- [ ] Renaming to a new initial updates the badge
- [ ] Core tests cover colour assignment and persistence
