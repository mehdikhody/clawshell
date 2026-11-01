# 05: This month, extra usage and notifications in the chosen calendar

**What to build:** "This month" is the calendar month that contains today, in the resolved calendar. The Jalali, Hijri and Chinese lunar month boundaries are right, including a Chinese leap month and the Hijri month edge. On a non-Gregorian calendar the extra-usage line is labelled "billing month", and the total still adds both parts. The heatmap tooltip writes its dates in the resolved calendar. Notification numbers, including the "5" in "5-hour", use the resolved digits. See spec: Calendar (This month, Extra usage, Heatmap, Notifications) and ADR 0003.

**Blocked by:** 04

**Status:** resolved

- [x] "This month" covers the calendar month containing today, in the resolved calendar; `icu_calendar` supplies the boundaries
- [x] Extra usage is labelled "billing month" on a non-Gregorian calendar
- [x] Heatmap tooltip dates use the resolved calendar
- [x] Notification numbers use the resolved digits
- [x] Core tests cover month boundaries for each calendar, including a Chinese leap month and a Hijri month edge; notification digit tests
- [x] A component test shows the extra-usage label under Jalali
- [x] A user-facing entry is added under `## [Unreleased]` in `CHANGELOG.md`
