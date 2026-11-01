# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- Arabic, Russian and Simplified Chinese join English and Persian in the window. The language picker shows each language in its own name, Arabic is laid out right to left, and Arabic and Chinese get their own fonts. The tray menu and desktop notifications are translated too.
- On first run, clawshell starts in your Windows display language when it is Persian, Arabic, Russian or Chinese, and in English otherwise. A language you have already chosen is never changed.
- **Calendar** setting under Language: Automatic, Gregorian, Jalali, Hijri or Chinese lunar. It decides how dates are written and which day the heatmap's weeks start on. Numbers keep the digits of the language (Persian digits in Persian, Arabic-Indic in Arabic, 0-9 otherwise), so a calendar never changes them. Automatic picks Jalali for Persian, Hijri for Arabic and Gregorian otherwise, so Persian looks the same as before, and a calendar you pick stays when you change language.
- **Quit Claude** in a running account's context menu. It ends all of that account's Claude processes at once, including a Claude hidden in its own tray, and works for the default account too.
- **Copy sessions…** in every account's context menu. It lists the account's Code sessions from every organisation, newest first, with title, folder, last activity and an Archived tag, and lets you tick several. Copying them to another account comes in a later update.
- A "Before you start" notice on first use (and again whenever its text changes), in your language: clawshell is unofficial, what it reads and changes, and that you use it at your own risk. Until you accept, only the window runs: no tray, usage polling, account watching, link handling or update checks, and the window is shown even with start hidden on. Exit, or closing the window, quits clawshell; shortcut clicks and sign-in links you open meanwhile are carried out once you accept. Everyone sees it once after updating.

### Changed

- "This month" is now the month of your calendar that contains today (a Jalali, Hijri or Chinese lunar month on those calendars). On a calendar other than Gregorian, the extra-usage line is labelled "billing month" because Anthropic reports it for its own month. Heatmap dates and the numbers in limit notifications follow your calendar too.
- The Calendar menu is wide enough for its longest name, and the English name for the Chinese lunar calendar is now **Lunar**.
- Placeholders shaped like the real content on the Usage tab (heatmap, This month card, pooled usage) and the Accounts tab (account rows) replace the blank space and the "Loading…" line while they first load. They stay still under reduced motion, and refreshes, like changing the language or calendar, never bring them back.
- Dragging an account on the Accounts tab moves the rows live, so you see the new order before you let go. Release the row over the list to save it; release it off the list and the old order stays.
- Account drag and drop uses a new library: the other rows slide out of the way, and the dragged row is solid, so the rows under it no longer show through. From the keyboard, Space picks a focused row up and puts it down, the arrow keys move it, and Escape puts it back. Enter still launches the account.
- The default account (Claude) can be dragged to any place in the account list, like any other account. It stays first until you move it.

## [0.2.0] - 2026-11-01

### Added

- Automatic updates. clawshell checks GitHub releases when it starts and every 6 hours after that, shows an **Update available** badge, and downloads, verifies and installs the update with a progress bar. Failed downloads can be retried, and the current version keeps running.
- Update dialog that shows the release notes before installing.
- Signed updater artifacts in release builds, with `latest.json` uploaded alongside the installers.
- Drag and drop to reorder accounts. The order is used on the Accounts tab, the tray menu, the This month card and the heatmap tooltip. New accounts go to the end.
- **Add account** item in the tray menu, which opens the Accounts tab with the Add dialog open.
- Warning on the Settings tab when Windows sends `claude://` sign-in links to another app, with an **Open Default Apps** button.

### Changed

- Reset alerts fire on every 5-hour and weekly window reset, not only after a limit was hit. The alert says how much of the window was used. The first reading after launch is silent.
- The heatmap always shows nine full weeks, starting on Sunday (Saturday in Persian), and is titled **Last 9 weeks**. Days still to come are drawn as faint slots.
- The heatmap tooltip shows the day's dollars and tokens in short form and breaks them down per account, with badge colours. Accounts with no tokens that day are left out.
- Usage bars and percentages on the Usage tab show how much of each limit is used, matching the Accounts tab.
- The This month card and the heatmap tooltip show each account's badge colour as a dot before its name.

### Fixed

- Tooltips are laid out right to left in Persian.

## [0.1.0] - 2026-10-31

### Added

- Initial release.

[unreleased]: https://github.com/mehdikhody/clawshell/compare/v0.2.0...HEAD
[0.2.0]: https://github.com/mehdikhody/clawshell/compare/v0.1.0...v0.2.0
[0.1.0]: https://github.com/mehdikhody/clawshell/releases/tag/v0.1.0
