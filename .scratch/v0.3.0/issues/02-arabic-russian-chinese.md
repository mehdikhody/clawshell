# 02: Arabic, Russian and Simplified Chinese in the window

**What to build:** clawshell's window is available in Arabic, Russian and Simplified Chinese, next to English and Persian. The language picker lists each language in its own name: العربية, Русский, 简体中文. The Arabic window is laid out right to left. Arabic uses Vazirmatn; Russian keeps Roboto; Chinese uses Noto Sans SC, loaded through Fontsource. The Rust `Language` enum gains the three codes. Tray and notification text comes in ticket 03. See spec: Languages.

**Blocked by:** None (can start immediately)

**Status:** ready-for-agent

- [ ] The picker lists the five languages, each in its own name
- [ ] Every message key from English exists in the ar, ru and zh message files, with translations drafted by the implementer
- [ ] Arabic renders right to left; `directionOf` returns `rtl` for `ar`
- [ ] Arabic and Chinese use their fonts; Russian uses Roboto
- [ ] The chosen language persists and applies straight away
- [ ] Unit tests cover the locale list and `directionOf`; a component test shows Arabic rendering right to left
- [ ] A user-facing entry is added under `## [Unreleased]` in `CHANGELOG.md`
