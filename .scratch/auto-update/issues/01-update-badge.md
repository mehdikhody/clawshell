# 01: Update badge appears when a newer release exists

**What to build:** clawshell quietly checks the latest GitHub release at startup and then every 6 hours. When a newer version exists, a small "Update available" badge appears next to the app name in the header. When the app is on the latest version, or the check fails or runs offline, there is no badge and no error. This ticket adds both updater plugins (updater and process) on the Rust and JS sides. It also adds the updater config: the releases endpoint, passive Windows install mode, and a clearly marked placeholder public key until ticket 04 supplies the real one. It allows the capabilities ticket 02 will need (`updater:default`, `process:allow-restart`). The shared update state module covers the full state shape (`idle | available | downloading | installing | failed`) so ticket 02 can build on it. The badge uses the existing Badge component and theme tokens. Its label is in English and Persian. The Rust core and its ports stay untouched. See spec: Mechanism, Checking, Header badge, Strings.

**Blocked by:** None (can start immediately)

**Status:** ready-for-agent

- [ ] Both plugins are registered, the updater config and capabilities are in place, and the app still builds and runs locally without a signing key
- [ ] A check runs once at startup and then every 6 hours; a failed check is swallowed and leaves the state unchanged
- [ ] A check that finds nothing newer clears an earlier badge, except while a download is in progress
- [ ] The header shows a badge button named "Update available" only while an update is available or further along
- [ ] `update_available` exists in English and Persian
- [ ] Unit tests (updater mocked, fake timers) cover the startup check, the 6-hour interval, a swallowed error and the badge-clearing rule
- [ ] Component tests (existing `mockTauri` harness) show the badge hidden with no update and shown with one
