# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.5.0] - 2026-11-04

### Added

- **Disable account** and **Enable account** in an account's right-click menu. A disabled account is greyed out, shows a **Disabled** badge (with **Running** beside it while its Claude is open), and is listed at the very bottom, below limited and missing accounts. It is hidden from the tray, left out of the pooled usage, can't receive copied sessions and sends no limit notifications. It still opens when you launch it, and disabling never closes a running Claude. Disabled accounts can be dragged among themselves, and each returns to its place in your order when enabled.

## [0.4.1] - 2026-11-02

### Changed

- The **Update available** badge in the title bar is smaller.
- The app's program file is now named `Clawshell.exe` instead of `clawshell.exe`.
- The update dialog lays out the release notes with their headings, paragraphs and bullet lists, instead of showing the raw `##` and `-` markers.
- Limited accounts and accounts whose folder is missing are now listed after the others, on the Accounts screen and in the tray menu. They keep their place in your order and return to it once usable again. They can't be dragged, and other rows can't be dropped among them.

### Fixed

- The update dialog's **What's new** was empty because the release notes never reached the update manifest. Releases now include them, and v0.4.0's manifest has been fixed too.

## [0.4.0] - 2026-11-01

### Changed

- Limited accounts and accounts whose folder is missing now look disabled on the Accounts screen: their icon, name, email and usage bars are greyed out and faded. Their badges and the Remove button keep their colour, and the row still launches and opens its menu as before.
- An account's weekly and 5-hour usage now share one thicker bar on the Accounts screen: an orange segment as wide as the weekly percentage, then an amber 5-hour segment that takes its percentage of the room the week leaves. As the week fills, the 5-hour segment shrinks, and once the week is used up it has no room. Underneath, a matching coloured dot labels each window with its real percentage and reset time.
- The Accounts screen's loading placeholders now look like the account rows they stand in for: thin lines where the name, email and usage labels go, a dot for each label, and a status badge.
- Hovering an account's usage bar shows a tooltip that follows the pointer, with both windows and how long until each resets, such as "resets in 2 days" or "resets in 45 minutes".
- The account usage bars keep their own colours and no longer turn orange or red at the warning and limit thresholds. The thresholds still decide when you are notified.

## [0.3.0] - 2026-11-01

### Added

- Arabic, Russian and Simplified Chinese join English and Persian in the window. The language picker shows each language in its own name, Arabic is laid out right to left, and Arabic and Chinese get their own fonts. The tray menu and desktop notifications are translated too.
- On first run, clawshell starts in your Windows display language when it is Persian, Arabic, Russian or Chinese, and in English otherwise. A language you have already chosen is never changed.
- **Calendar** setting under Language: Automatic, Gregorian, Jalali, Hijri or Chinese lunar. It decides how dates are written and which day the heatmap's weeks start on. Numbers keep the digits of the language (Persian digits in Persian, Arabic-Indic in Arabic, 0-9 otherwise), so a calendar never changes them. Automatic picks Jalali for Persian, Hijri for Arabic and Gregorian otherwise, so Persian looks the same as before, and a calendar you pick stays when you change language.
- **Quit Claude** in a running account's context menu. It ends all of that account's Claude processes at once, including a Claude hidden in its own tray, and works for the default account too. It reports a failure only when some of Claude is still running afterwards.
- **Copy sessions…** in every account's context menu. It lists the account's Code sessions from every organisation, newest first, including sessions whose Code conversation id Claude has reset, with title, folder, last activity and an Archived tag, and lets you tick several from a picker with **Select all**, and a search box once there are more than five. Pick a target account under **Copy to** and press **Copy**: the sessions are copied into the account and organisation that account is signed into now, the source keeps its own, archived ones stay archived, and a session the target already has is replaced. Accounts that can't receive copies are greyed out with what to do: sign in first, or open the Code tab once. The result reads "Sessions copied: 3", plus a reminder to restart the target when its Claude is running, since Claude lists new sessions only after a restart.
- A "Before you start" notice on first use (and again whenever its text changes), in your language: clawshell is unofficial, what it reads and changes, and that you use it at your own risk. Until you accept, only the window runs: no tray, usage polling, account watching, link handling or update checks, and the window is shown even with start hidden on. Exit, or closing the window, quits clawshell; shortcut clicks and sign-in links you open meanwhile are carried out once you accept. Everyone sees it once after updating.

### Changed

- "This month" is now the month of your calendar that contains today (a Jalali, Hijri or Chinese lunar month on those calendars). On a calendar other than Gregorian, the extra-usage line is labelled "billing month" because Anthropic reports it for its own month. Heatmap dates and the numbers in limit notifications follow your calendar too.
- The Calendar menu is wide enough for its longest name, and the English name for the Chinese lunar calendar is now **Lunar**.
- Placeholders shaped like the real content on the Usage tab (heatmap, This month card, pooled usage) and the Accounts tab (account rows) replace the blank space and the "Loading…" line while they first load. They stay still under reduced motion, and refreshes, like changing the language or calendar, never bring them back.
- After clawshell starts, each signed-in account shows placeholders for its email and usage bars until its first usage check, instead of "No usage data yet", and the Usage tab's pooled card waits for every account the same way. The loading placeholders for the pooled card, the heatmap and the usage part of account rows now match the size and layout of what replaces them, so they no longer jump when the numbers arrive.
- A signed-out account no longer shows its last usage and email. When it signs back in, possibly as someone else, it shows the placeholders until its first usage check.
- Dragging an account on the Accounts tab moves the rows live, so you see the new order before you let go. Release the row over the list to save it; release it off the list and the old order stays.
- Account drag and drop uses a new library: the other rows slide out of the way, and the dragged row is solid, so the rows under it no longer show through. From the keyboard, Space picks a focused row up and puts it down, the arrow keys move it, and Escape puts it back. Enter still launches the account.
- The default account (Claude) can be dragged to any place in the account list, like any other account. It stays first until you move it.
- **Quit Claude** sits directly above **Delete** in an account's context menu.
- Rename and badge colour are one **Edit account** item in an account's context menu. It opens a dialog like the Add dialog, with the name and the colour together; Save applies both at once. The default account can be renamed there but has no colour.

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
- An account whose status badges sit beside a long email no longer wraps them onto their own line, so "Running" and "Limited" stay on the name's row and line up across accounts.

## [0.1.0] - 2026-10-31

### Added

- Initial release.

[unreleased]: https://github.com/mehdikhody/clawshell/compare/v0.5.0...HEAD
[0.5.0]: https://github.com/mehdikhody/clawshell/compare/v0.4.1...v0.5.0
[0.4.1]: https://github.com/mehdikhody/clawshell/compare/v0.4.0...v0.4.1
[0.4.0]: https://github.com/mehdikhody/clawshell/compare/v0.3.0...v0.4.0
[0.3.0]: https://github.com/mehdikhody/clawshell/compare/v0.2.0...v0.3.0
[0.2.0]: https://github.com/mehdikhody/clawshell/compare/v0.1.0...v0.2.0
[0.1.0]: https://github.com/mehdikhody/clawshell/releases/tag/v0.1.0
