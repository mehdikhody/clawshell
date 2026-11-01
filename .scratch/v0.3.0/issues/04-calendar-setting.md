# 04: Calendar setting for dates, digits and week start

**What to build:** The Settings tab has a Calendar setting directly below Language, with the same dropdown pattern: Automatic, Gregorian, Jalali, Hijri, Chinese lunar. Automatic picks Jalali for Persian, Hijri for Arabic, and Gregorian otherwise. The resolved calendar decides how dates are written, which digits every number uses (Persian digits for Jalali, Arabic-Indic for Hijri, 0–9 for Gregorian and lunar), and the first day of each heatmap week. A calendar the user picked stays when the language changes. Persian with Automatic looks exactly as it does today. The resolver exists in the frontend and mirrored in the core. See spec: Calendar (ADR 0003), and ADR 0003.

**Blocked by:** 02

**Status:** resolved

- [x] The Calendar dropdown sits under Language with the five options, Automatic by default, and is stored in settings
- [x] Automatic resolves per language; a pinned value survives language changes
- [x] Every number on screen uses the resolved digits; dates use the resolved calendar
- [x] The heatmap's weeks start on the day given by the spec's week-start rule
- [x] Frontend unit tests cover `resolveCalendar`, `digitsOf`, `weekStartOf` and `intlLocale` for every language and calendar pair; Persian with Automatic gives today's tag
- [x] The core resolver matches the frontend and has tests
- [x] A user-facing entry is added under `## [Unreleased]` in `CHANGELOG.md`
