# 03: First-run language from Windows, and translated tray and notifications

**What to build:** On first run, when no settings file exists, clawshell reads the Windows UI language. Persian, Arabic, Russian and any Chinese map to their language. Anything else falls back to English. A saved language is never overridden. The tray menu and desktop notifications are translated into Arabic, Russian and Chinese as well, including the window names used in notifications. See spec: Languages (First run) and Languages (Rust-side text).

**Blocked by:** 02

**Status:** ready-for-agent

- [ ] With no settings file, the language comes from the Windows UI language; unsupported languages fall back to English
- [ ] An existing settings file is never changed by this
- [ ] Tray labels and notification text are translated into ar, ru and zh
- [ ] Core tests cover the UI-language mapping, the English fallback, and notification text per language
- [ ] A user-facing entry is added under `## [Unreleased]` in `CHANGELOG.md`
