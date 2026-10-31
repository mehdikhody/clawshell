# Spec: auto-update from GitHub releases

Status: ready-for-agent

## Problem Statement

clawshell sits in the tray for days, and the only way to get a new version is to visit the GitHub releases page, download the installer and run it by hand. Users don't know a new version exists, so fixes (sign-in routing, usage bars) reach them late or never.

## Solution

clawshell checks the latest GitHub release quietly at startup and every 6 hours. When a newer version exists, a small badge appears next to the app name in the header. Clicking it opens a dialog showing the new version and its release notes, with an Update button. Update downloads the signed installer with visible progress, installs it, and restarts clawshell. It uses the official Tauri updater (`tauri-plugin-updater`), which checks the installer's signature before running it.

## User Stories

1. As a user, I want clawshell to check for a new version at startup and every 6 hours, so that I learn about updates even though the app lives in the tray for days.
2. As a user, I want the check to be silent, so that a failed or offline check never shows an error or interrupts me.
3. As a user, I want a badge next to "clawshell" in the header when a newer version is available, so that I notice it without being nagged.
4. As a user, I want no badge when I'm on the latest version, so that the header stays clean.
5. As a user, I want clicking the badge to open a dialog with the new version number and its release notes, so that I can decide whether to update now.
6. As a user, I want to close the dialog without updating, so that I can update later; the badge stays.
7. As a user, I want the Update button to show download progress, so that I know it is working.
8. As a user, I want clawshell to install the update and restart itself, so that I don't run an installer by hand.
9. As a user, I want my Claude windows to keep running during the update, so that only clawshell restarts.
10. As a user, I want a failed download or install to say so in the dialog and leave the current version running, so that a bad update never leaves me without the app.
11. As a user, I want only genuine releases installed, so that a tampered download is refused (signature check).
12. As a user, I want the update text in English and Persian, with right-to-left layout in Persian.

## Implementation Decisions

### Mechanism
- Add `tauri-plugin-updater` (Rust and JS) and `tauri-plugin-process` (for the restart).
- `tauri.conf.json`: `bundle.createUpdaterArtifacts: true`; `plugins.updater.pubkey` set to the public key; `plugins.updater.endpoints` = `["https://github.com/mehdikhody/clawshell/releases/latest/download/latest.json"]`; `plugins.updater.windows.installMode: "passive"`.
- Capabilities: allow `updater:default` and `process:allow-restart` for the main window.
- Release workflow: pass `TAURI_SIGNING_PRIVATE_KEY` and `TAURI_SIGNING_PRIVATE_KEY_PASSWORD` from repo secrets to `tauri-action`, which then uploads the signed installers and `latest.json` to the release.
- One-time owner step (outside code): run `npx tauri signer generate -w ~/.tauri/clawshell.key`, add the two secrets, and commit the public key to `tauri.conf.json`.
- Only stable releases count: `releases/latest` already skips drafts and prereleases.

### Checking
- A frontend module `src/lib/update.svelte.ts` holds the shared state: `idle | available(update) | downloading(progress) | installing | failed(message)`.
- It calls `check()` once on startup and then every 6 hours (`UPDATE_CHECK_MS = 6 * 60 * 60 * 1000`). Errors are swallowed and the state stays as it was.
- A check that finds nothing newer clears a previously shown badge only if no download is in progress.

### Header badge
- `top-bar.svelte` renders a small badge button after the app name while the state is `available` or further along. Accessible name: "Update available". Uses the existing `Badge` component and theme tokens.

### Dialog
- An `update-dialog.svelte` component built on the existing `Dialog` component: title with the new version, release notes (`update.body`, plain text, scrollable), and Cancel and Update buttons.
- Update calls `downloadAndInstall`, maps the progress events to a `Progress` bar (percent when the content length is known, indeterminate otherwise), then calls `relaunch()`.
- On failure: an inline error and a Retry button. The current version keeps running.

### Strings
- New message keys in `messages/en.json` and `messages/fa.json`: `update_available`, `update_title` (with version), `update_notes`, `update_now`, `update_later`, `update_downloading`, `update_installing`, `update_failed`, `update_retry`.

### Not in the core
- The updater is a Tauri/UI concern with no domain logic, so the Rust core and its ports stay untouched.

## Testing Decisions

- Unit-test `update.svelte.ts` with the updater and process APIs mocked: the startup check, the 6-hour interval (fake timers), a swallowed error, and the state moving through download progress to install and failure.
- Component tests (vitest browser, the existing `mockTauri` harness): the badge is hidden with no update and shown with one; clicking it opens the dialog with the version and notes; Update shows progress and then calls relaunch; a failure shows the error and Retry; Persian renders right to left.
- Manual release check: tag a test version, confirm `latest.json` and the `.sig` files are on the release, install the previous version, and confirm the badge, dialog, update and restart work end to end.

## Out of Scope

- Auto-installing without a click, and background downloads.
- Tray menu entries or Windows notifications for updates.
- A "Check for updates" button in Settings.
- Prerelease or beta channels, and rollback.
- Updating Claude Desktop itself.

## Further Notes

- v0.1.0 has no updater, so users must install the first updater-enabled release by hand. Every release after that updates itself.
- Losing the private key means existing installs can no longer verify updates. Keep a backup of it outside the repo.
