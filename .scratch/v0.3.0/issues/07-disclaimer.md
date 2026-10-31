# 07: First-use disclaimer dialog and deferred start

**What to build:** Until the user accepts the disclaimer, clawshell shows a "Before you start" dialog with Accept and Exit. Only the window runs before acceptance: watchers, usage polling, the tray, link handling and the updater check and auto-refresh timers wait. Accept saves the disclaimer version and starts the deferred work once. Exit quits. Esc, clicking outside and closing the window do nothing, except that closing the window counts as Exit. Launch arguments and second-instance arguments received before acceptance are replayed in order after it. Even with start-hidden set, the window is shown until acceptance. The dialog text is in the detected language (ticket 03), with Persian and Arabic right to left. Storage is `disclaimerAccepted` in settings, with the constant `DISCLAIMER_VERSION` starting at 1. See spec: Disclaimer, and the disclaimer test list.

**Blocked by:** 03

**Status:** ready-for-agent

- [ ] The dialog shows while the disclaimer is unaccepted, with no close button; Esc, clicking outside and closing the window don't dismiss it, and closing counts as Exit
- [ ] Exit quits clawshell; Accept saves `DISCLAIMER_VERSION`, starts the background work once, and replays held arguments in order
- [ ] No settings file means unaccepted; an accepted current version means accepted; a later version bump shows the dialog again
- [ ] Before acceptance the window is shown even with start-hidden set, and no tray or background work runs
- [ ] The text is translated into all five languages; Persian and Arabic render right to left
- [ ] Core tests cover acceptance states, start-once behaviour and argument replay; component tests cover show/absence, Esc and outside click, and the Accept and Exit calls
- [ ] A user-facing entry is added under `## [Unreleased]` in `CHANGELOG.md`
