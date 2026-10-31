# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- Arabic, Russian and Simplified Chinese join English and Persian in the window. The language picker shows each language in its own name, Arabic is laid out right to left, and Arabic and Chinese get their own fonts. Tray menu and notification text stay in English for these three for now.
- **Quit Claude** in a running account's context menu. It ends all of that account's Claude processes at once, including a Claude hidden in its own tray, and works for the default account too.

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
