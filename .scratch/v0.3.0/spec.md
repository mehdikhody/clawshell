# Spec: v0.3.0

Status: ready-for-agent

Four features: Quit Claude, session copy, three new languages with a separate Calendar setting, and skeleton loading. Terms follow `GLOSSARY.md`; the decisions behind them are ADRs 0002, 0003 and 0004.

## Problem Statement

- **Quit Claude.** Claude Desktop's close button only hides it to its own tray. To actually end one account's Claude, users open Task Manager and guess which `Claude.exe` belongs to which account.
- **Session copy.** When one account hits its limit in the middle of a Code session, the work is stuck in that account. Its transcript is already on disk, but the other accounts' Code tabs don't list it, so there's no way to carry on elsewhere.
- **Languages.** clawshell speaks English and Persian only. The language also fixes the calendar: Persian always means Jalali, and English always means Gregorian, even for people who date things differently.
- **Loading.** The Usage tab shows nothing until each section arrives, then the sections pop in and push each other down. The Accounts tab shows one "Loading…" line.

## Solution

- **Quit Claude:** a context-menu item on any running account ends all of that account's Claude processes at once, hidden or not.
- **Copy sessions:** a context-menu item opens a dialog. You tick Code sessions, pick a target account, and clawshell writes those sessions' records into the target's profile. The source keeps them, and both accounts continue the same transcript.
- **Languages:** Arabic, Russian and Simplified Chinese join English and Persian. A new Calendar setting (Automatic, Gregorian, Jalali, Hijri, Chinese lunar) decides how dates are written, which digits every number uses, which day weeks start on, and what This month means.
- **Skeletons:** shadcn-svelte Skeleton placeholders, shaped like the content they stand for, fill every lazily loaded section until its first answer arrives.

## User Stories

### Quit Claude
1. As a user, I want a "Quit Claude" item in a running account's context menu, so that I can end that account's Claude without Task Manager.
2. As a user, I want it to end Claude even when Claude is hidden in its own tray, so that "closed" really means closed.
3. As a user, I want it to act at once, with no confirm dialog, so that quitting is one click.
4. As a user, I want the item hidden when the account isn't running, so that the menu only offers what can happen.
5. As a user, I want to quit the default account's Claude the same way.
6. As a user, I want the account's status and the tray tick to update right after quitting.

### Session copy
7. As a user, I want a "Copy sessions…" item in an account's context menu, so that I can carry Code sessions to another account.
8. As a user, I want the dialog to list every Code session of that account, from every organisation, newest activity first, with title, folder and last activity. Archived ones should be tagged.
9. As a user, I want to tick several sessions and pick one target account, so that I copy them in one go.
10. As a user, I want targets that are signed out, or have never opened the Code tab, greyed out with a hint, so that I know why I can't pick them.
11. As a user, I want copied sessions to stay in the source account, so that nothing is lost.
12. As a user, I want archived sessions to stay archived in the target.
13. As a user, I want a session the target already has to be replaced with the source's version.
14. As a user, I want a result line such as "3 sessions copied", plus a note to restart the target when its Claude is running, because the target only lists them after a restart.
15. As a user, I want to copy into and out of the default account.
16. As a user, I want a copied session's cost to stay with the account it was created in, so that the monthly totals don't jump around after a copy.

### Languages and calendar
17. As a user, I want clawshell in Arabic, Russian and Simplified Chinese: the window, the tray menu and the notifications.
18. As an Arabic user, I want the window laid out right to left.
19. As a user, I want the language list to show each language in its own name (العربية, Русский, 简体中文).
20. As a new user, I want clawshell to start in my Windows display language when it has it, and in English otherwise.
21. As a user, I want a Calendar setting under Language with Automatic, Gregorian, Jalali, Hijri and Chinese lunar.
22. As a user, I want Automatic to pick Jalali for Persian, Hijri for Arabic and Gregorian otherwise, so that the defaults fit without setup.
23. As a user, I want a calendar I picked myself to stay when I change language.
24. As a user, I want every number (dates, costs, percents, tokens, notifications) in my calendar's digits: Persian digits for Jalali, Arabic-Indic for Hijri, 0–9 for Gregorian and lunar.
25. As a user, I want the heatmap's weeks to start on my calendar's day. That is Saturday for Jalali and Hijri, and Monday for lunar. Gregorian takes it from the language: Sunday for English, Monday for Russian and Chinese, Saturday for Persian and Arabic.
26. As a user, I want This month to be my calendar's current month.
27. As a user on a non-Gregorian calendar, I want the extra-usage spend labelled as the billing month, so that I know it covers a different span from the rest of the card.

### Skeleton loading
28. As a user, I want placeholders shaped like the real content on the Usage tab: the heatmap, the This month card and the pooled usage block. Then nothing jumps when the data arrives.
29. As a user, I want placeholder account rows, with name, email and two usage bars, in place of "Loading…" on the Accounts tab.
30. As a user, I want automatic refreshes to keep the old data on screen instead of flashing placeholders.
31. As a user, I want a section with no data to disappear after its placeholder, as it does today.
32. As a user who prefers reduced motion, I want the placeholders to stay still.

## Implementation Decisions

### Quit Claude
- New core operation `quit_account(id)`. It ends every PID that `ProcessInspector::pids_for(profile)` returns for the account's profile, the default account included. Behind a new port method (`ProcessKiller::end(pids)`), the Windows adapter calls `TerminateProcess` on each PID. It's a forced end with no graceful step, because Claude has no quit command and closing its window only hides it.
- New Tauri command `quit_account`. After it returns, the window reloads the list and the tray redraws (`tray::refresh`).
- In `accounts-screen.svelte`, a `ContextMenu.Item` "Quit Claude" appears only when `account.running`, placed after Open folder. A failure shows the screen's existing `failure` alert ("Couldn't quit Claude").
- The clawshell tray menu is unchanged.

### Session copy
- **Where sessions live.** Each profile keeps one record per Code session at `claude-code-sessions/{accountUuid}/{orgUuid}/local_{id}.json`. Folders named `deleted_*` hold removed sessions and are skipped. The transcript lives in the shared `~/.claude/projects` and isn't touched (ADR 0004).
- **Listing.** New core operation `list_sessions(account_id)`. It reads every `local_*.json` under every account and organisation folder of the profile, and returns `cliSessionId`, `title`, `cwd`, `lastActivityAt` and `isArchived`, newest activity first.
- **Target folder.** The target's current account and organisation come from the OAuth profile endpoint (`PROFILE_URL`), which the usage poller already calls for the email. Store `account.uuid` and `organization.uuid` beside the email. A target is eligible when it's signed in, those IDs are known, and `claude-code-sessions/{accountUuid}/{orgUuid}` exists. Otherwise the picker greys it out with "Open the Code tab in this account once first", or "Sign in to this account first".
- **Copying.** New core operation `copy_sessions(source_id, session_ids, target_id)`. It writes each source record into the target folder under the same file name. If a record with the same `cliSessionId` already exists anywhere in the target profile, that record is replaced. Fields that turn out to be tied to the source account are cleared (see Further Notes); everything else is copied as is, `isArchived` included. Each file is written to a temporary name, then renamed. The operation returns `{ copied, targetRunning }`.
- **Original account.** Before the first copy of a session, clawshell records `cliSessionId → original account id` in a new `clawshell/session-origins.json`. In `cost.rs`, `session_owners` uses that record first and falls back to today's first-found rule. Copies never overwrite an existing entry.
- **Default account.** It can be a source and a target. That's the one exception to ADR 0001's read-only rule (ADR 0002).
- **UI.** A "Copy sessions…" `ContextMenu.Item` opens `copy-sessions-dialog.svelte`, built on `Dialog`. It has a scrollable checklist (title, folder, relative last activity, an Archived badge), a target `Select`/dropdown listing the other accounts with disabled reasons, and Cancel and Copy buttons. Copy is disabled until at least one session and a target are chosen. The result appears as the screen's `notice` ("{count} sessions copied", plus "Restart {target} to see them" when `targetRunning`).
- **Scope.** Code tab sessions only. Chats live on claude.ai, and Cowork (`local-agent-mode-sessions`) is a separate store.

### Languages
- Add `ar`, `ru` and `zh` to `project.inlang/settings.json` `locales`, `src/lib/locale.ts` `locales`, and the Rust `Language` enum (serde lowercase). Add `messages/ar.json`, `messages/ru.json` and `messages/zh.json` with every key in `en.json`. Use `Intl` tag `zh-CN` for `zh`.
- `directionOf` returns `rtl` for `fa` and `ar`.
- Translate the Rust-side text in `Language::tray_labels` (`shell.rs`) and in notifications (`notifications.rs` `text` and `window_name`) for all three. The implementer drafts every translation, and they ship without a review step.
- Language picker names are endonyms: `language_ar` = العربية, `language_ru` = Русский, `language_zh` = 简体中文, identical in every message file, as `language_fa` is today.
- **First run.** When `settings.json` doesn't exist yet, the core reads the Windows UI language through a new port method (`GetUserDefaultUILanguage`, behind an `OsAdapters` trait). It maps the primary language to `en`/`fa`/`ar`/`ru`/`zh` (any Chinese maps to `zh`), and falls back to `en`. A saved language is never overridden.
- **Fonts.** Arabic uses Vazirmatn (`:root:lang(ar)`, as for `fa`). Russian keeps Roboto, which has Cyrillic. Chinese gets `@fontsource-variable/noto-sans-sc` on `:root:lang(zh)`. Fontsource splits it by unicode-range, so only the glyphs on screen are fetched.

### Calendar (ADR 0003)
- New setting `calendar`: `automatic` (default) | `gregory` | `persian` | `islamic-umalqura` | `chinese`, using `Intl` calendar ids. It's stored in `settings.json` and shown in the Settings tab directly below Language, with the same dropdown pattern.
- `automatic` resolves to `persian` for `fa`, `islamic-umalqura` for `ar`, and `gregory` otherwise. Choosing any other value pins it; changing the language never changes `calendar`.
- **One resolver**, in the frontend's `locale.ts` and mirrored in the core: `resolveCalendar(language, calendar)`, `digitsOf(calendar)`, `weekStartOf(language, calendar)`.
  - Digits: `persian` → `arabext`, `islamic-umalqura` → `arab`, `gregory`/`chinese` → `latn`.
  - Week start (`Date.getDay` numbering): `persian` and `islamic-umalqura` → 6. `chinese` → 1. `gregory` → by language: `en` 0, `ru`/`zh` 1, `fa`/`ar` 6.
  - `intlLocale` becomes `{languageTag}-u-ca-{calendar}-nu-{digits}`. Persian with Automatic produces today's `fa-u-ca-persian-nu-arabext`, so existing Persian users see no change.
- **This month.** `cost.rs` `local_month` becomes the calendar month containing today, in the resolved calendar. Use `icu_calendar` (ICU4X) in the core for Persian, Hijri Umm al-Qura and Chinese month boundaries. The month's start and end are converted to local-time epoch ms and used where the Gregorian `(year, month)` is used today.
- **Extra usage.** The spend stays the endpoint's billing-month figure. When the resolved calendar isn't `gregory`, `month-cost.svelte` labels the extra-usage line "billing month". The total still adds both parts.
- **Heatmap.** It keeps nine full weeks, now aligned to `weekStartOf(language, calendar)`. Tooltip dates are written in the resolved calendar.
- **Notifications.** The core formats the percent (and the "5" in "5-hour") in the resolved calendar's digits.

### Skeleton loading
- Add the shadcn-svelte Skeleton component (`npx shadcn-svelte@latest add skeleton`) under `src/lib/components/ui/skeleton/`. Add `motion-reduce:animate-none` to it.
- **Usage tab.** Each lazily loaded section (`heatmap`, `cost`, and the pooled block from `list`) gets three states: loading, loaded, or failed/empty. While a section is loading for the first time, it shows a skeleton with the real layout: a 9-week grid of day cells, the card's title, total and per-account rows, and the pooled bars. Loaded shows today's content. Failed or empty hides the section, as today.
- **Accounts tab.** While `!loaded`, show two skeleton rows shaped like an account row (icon, name, email line, two usage bars) instead of `m.loading()`. Once the list loads, accounts without usage show "No usage yet" as today; the core isn't changed.
- Refreshes (auto-refresh, the core pushing a new snapshot, manual refresh) never go back to skeletons.

## Testing Decisions

- **Core (Rust, `src-tauri/tests/`):**
  - `quit_account` ends exactly the PIDs of that profile and works for the default account.
  - `list_sessions` reads every account and organisation folder, skips `deleted_*`, and sorts by activity.
  - `copy_sessions` writes into the target's current account and organisation folder, replaces an existing `cliSessionId`, keeps `isArchived`, refuses ineligible targets, and reports `targetRunning`.
  - `session-origins.json` is written once per session, and `session_owners` honours it.
  - First-run language detection maps UI languages and falls back to `en`.
  - `local_month` boundaries are right for each calendar, including a Chinese leap month and a Hijri month edge.
  - Notification text uses each language's strings and each calendar's digits.
- **Frontend unit tests (`locale.spec.ts`):**
  - `resolveCalendar`, `digitsOf`, `weekStartOf` and `intlLocale` work for every language × calendar pair.
  - Persian with Automatic gives the same tag as today.
- **Component tests (vitest browser, `mockTauri`):**
  - Quit Claude shows only for running accounts and calls `quit_account`.
  - In the Copy sessions dialog: the list shows title, folder and Archived; disabled targets show their reason; Copy is disabled until a session and a target are chosen; the result notice appears, including the restart hint.
  - Arabic renders right to left.
  - The Calendar dropdown sits under Language and keeps a pinned value across language changes.
  - Extra usage is labelled under Jalali.
  - Skeletons show before the first answer, disappear on empty data, and don't come back on refresh.
- **Manual:** copy a real session between two signed-in accounts and into and out of the default account. Restart the target and continue the session there. Check the This month figure in each calendar.

## Out of Scope

- Moving sessions (copy only), and copying regular chats or Cowork sessions.
- Restarting the target's Claude automatically after a copy.
- Choosing the target organisation.
- Quit Claude in clawshell's tray menu, and a "Quit all".
- A graceful quit attempt before ending processes.
- Traditional Chinese, and calendars beyond the five listed.
- Native-speaker review of the new translations before release.
- Telling "usage not fetched yet" apart from "no usage" on the Accounts tab.

## Further Notes

- **Session-record fields (first task of the session-copy work).** Compare session records of the same project across two real accounts. List the fields that point at the account, such as `remoteMcpServersConfig` connector UUIDs and `spawnSeed`, and decide for each whether to clear it or keep it. Record the result in the session-copy ticket. The reference tool (`PriyanshuGeTRekT/Claude-Code-Desktop-Switcher`) says only that it "re-points" the record, without details.
- Claude can change its session layout in any update. If it does, listing and copying must fail with a clear error and must not write half a record.
- Release: add every user-facing change under `## [Unreleased]` in `CHANGELOG.md`, then release as 0.3.0.
