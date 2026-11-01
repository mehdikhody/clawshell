# 02: Arabic, Russian and Simplified Chinese in the window

**What to build:** clawshell's window is available in Arabic, Russian and Simplified Chinese, next to English and Persian. The language picker lists each language in its own name: العربية, Русский, 简体中文. The Arabic window is laid out right to left. Arabic uses Vazirmatn; Russian keeps Roboto; Chinese uses Noto Sans SC, loaded through Fontsource. The Rust `Language` enum gains the three codes. Tray and notification text comes in ticket 03. See spec: Languages.

**Blocked by:** None (can start immediately)

**Status:** resolved

- [x] The picker lists the five languages, each in its own name
- [x] Every message key from English exists in the ar, ru and zh message files, with translations drafted by the implementer
- [x] Arabic renders right to left; `directionOf` returns `rtl` for `ar`
- [x] Arabic and Chinese use their fonts; Russian uses Roboto
- [x] The chosen language persists and applies straight away
- [x] Unit tests cover the locale list and `directionOf`; a component test shows Arabic rendering right to left
- [x] A user-facing entry is added under `## [Unreleased]` in `CHANGELOG.md`
