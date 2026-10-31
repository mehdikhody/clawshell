# Spec: clawshell v1 — multi-account Claude Desktop manager

Status: ready-for-agent

## Problem Statement

I use several Claude subscriptions (personal, work, spare) on one Windows PC. Claude Desktop only holds one sign-in at a time, so switching means signing out and back in, and I lose track of which account still has headroom. I can't see, in one place, how much of each account's 5-hour and weekly limit is left, when the windows reset, or roughly what my Code-tab usage is costing. The existing PowerShell switcher (Claude-Code-Desktop-Switcher) solves running accounts side by side, but it's a dated WinForms window with no usage or cost view at all.

## Solution

clawshell is a small, phone-shaped Windows desktop app (Tauri v2 + SvelteKit) that runs any number of Claude Desktop accounts side by side, each in its own isolated Electron user-data-dir named `clawshell-{uuid}` directly under the local app-data folder. It has three bottom tabs:

- **Usage** — this month's dollar total across all accounts (Code-tab estimate plus real extra-usage spend), pooled 5-hour and weekly "remaining" bars with the soonest reset, and a 30-day git-style heatmap of daily usage.
- **Accounts** — every account's name, email, 5-hour and weekly usage, and status. Double-click opens that account's Claude window (or brings it to the front). An Add button creates a new account and opens Claude at its sign-in screen.
- **Settings** — launch at boot (shown or hidden), close to tray, language (English / Persian with RTL), sign-in routing, notification thresholds, and a Claude.exe path override.

It lives in the tray, keeps every account's window on its own badged taskbar button, and routes browser sign-ins back to the account that asked for them.

## User Stories

### Window and shell

1. As a user, I want clawshell to open as a fixed 400×780 frameless window with rounded corners, so that it feels like a compact phone app rather than a sprawling desktop window.
2. As a user, I want a custom top bar I can drag the window by, so that I can place it anywhere without a native title bar.
3. As a user, I want minimize and close buttons in the custom top bar (and no maximize), so that the window controls match its fixed size.
4. As a user, I want a bottom tab bar with Usage, Accounts and Settings, so that I can move between the three views with one click.
5. As a user, I want the app to use a dark theme in Claude Code's colours, so that it looks at home next to my Claude tools.
6. As a user, I want only one clawshell to run at a time, so that launching it again just brings the existing window forward.
7. As a user, I want closing the window to hide clawshell to the tray (when "close to tray" is on), so that usage keeps refreshing and notifications keep arriving.
8. As a user, I want closing the window to quit the app when "close to tray" is off, so that I can opt out of the background behaviour.
9. As a user, I want a tray icon whose menu offers Open clawshell, my accounts, Add account and Quit, so that I can act without opening the window.
10. As a user, I want clicking an account in the tray menu to open that account's Claude (or bring it forward), so that switching is one click from anywhere.
11. As a user, I want running accounts marked in the tray menu, so that I can see at a glance which are open.

### Accounts

12. As a user, I want to see every clawshell account as a row with its name, email, 5-hour usage, weekly usage and status, so that I know each account's state at a glance.
13. As a user, I want each account's status to be one of Running, Idle, Signed out or Limited, so that I know whether it's open, usable, needs sign-in, or is out of quota.
14. As a user, I want an account to show as Running when its Claude Desktop window is open, so that I know which accounts are live.
15. As a user, I want an account to show as Signed out when it has no valid sign-in, so that I know it needs attention.
16. As a user, I want an account to show as Limited when its 5-hour or weekly usage is at 100%, so that I don't open an account that can't do work.
17. As a user, I want to double-click an account to open its Claude Desktop window, so that I can start working in that account immediately.
18. As a user, I want double-clicking an account that is already open to bring its window to the front instead of starting a second copy, so that I don't end up with duplicate windows.
19. As a user, I want an Add button that creates a new account and opens Claude at the sign-in screen, so that adding an account takes one click plus a sign-in.
20. As a user, I want each new account to get its own `clawshell-{uuid}` folder directly under local app data, so that accounts are fully isolated and Cowork's local VM still works.
21. As a user, I want a new account to be named "Account N" by default, so that I don't have to name it before using it.
22. As a user, I want to rename an account from its ⋯ menu, so that I can label it "Work" or "Personal".
23. As a user, I want renaming to change only the label and never the folder, so that the account's sign-in and data stay intact.
24. As a user, I want to open an account's folder from its ⋯ menu, so that I can inspect or back up its data.
25. As a user, I want to delete an account from its ⋯ menu after a confirmation, so that I can remove accounts I no longer need.
26. As a user, I want delete to be blocked while that account's Claude is running, so that I don't corrupt a live profile.
27. As a user, I want deleting an account to also remove its shortcuts and taskbar identity, so that nothing stale is left behind.
28. As a user, I want to pick a badge colour per account, so that I can tell accounts apart on the taskbar and in the list.
29. As a user, I want accounts to get a distinct colour automatically until I choose one, so that they're distinguishable from the start.
30. As a user, I want to set per-account environment variables, so that an MCP server in one account can get its own configuration.
31. As a user, I want to set per-account extra launch flags, so that I can enable logging or change language for one account only.
32. As a user, I want to be told that env vars and flags only apply when that account's Claude starts fresh, so that I'm not confused when a running copy ignores them.
33. As a user, I want to add desktop and Start-menu shortcuts for an account, so that I can launch it without opening clawshell.
34. As a user, I want each account's shortcut to carry its coloured badge icon, so that I can recognise it in Start search and on the desktop.
35. As a user, I want each account's Claude window to get its own taskbar button with its badge, so that accounts don't all pile onto one Claude button.
36. As a user, I want clicking an account's shortcut when it's already open to bring it forward, so that shortcuts behave like double-click.
37. As a user, I want my original Claude Desktop sign-in and the old switcher's profile folders left alone and not listed, so that clawshell never touches data it didn't create.

### Sign-in routing

38. As a user, I want clawshell to offer to turn on sign-in routing the first time I add an account, so that browser sign-ins reach the new account instead of my original one.
39. As a user, I want a browser sign-in to land in the account that asked for it, so that adding accounts doesn't keep signing in the wrong one.
40. As a user, I want sign-in routing to pick the target in a clear order (the account I launched while it was signed out, then the only open signed-out account, then the account whose window I used last), so that the choice is predictable.
41. As a user, I want to turn sign-in routing on or off in Settings, so that I can restore Claude's original link handling.
42. As a user, I want turning routing off to restore Claude's original `claude://` handler exactly, so that the registry is left as it was.
43. As a user on the Store build, I want clawshell to open the right Default Apps page and tell me the one choice I must make, so that routing actually takes effect.
44. As a user, I want the router never to log a sign-in link's contents, so that one-time codes aren't written to disk.

### Usage tab

45. As a user, I want to see this month's total dollar cost across all accounts, so that I know what my usage is worth.
46. As a user, I want that monthly total to combine the Code-tab estimate and real extra-usage spend, so that one number covers everything.
47. As a user, I want the Code-tab part labelled as an estimate, so that I don't mistake it for a bill.
48. As a user, I want a per-account breakdown under the monthly total, with each account's dollars and share, so that I see which account drives cost.
49. As a user, I want a pooled 5-hour bar showing the average remaining percentage across signed-in accounts, so that I know my combined short-term headroom.
50. As a user, I want a pooled weekly bar showing the average remaining percentage across signed-in accounts, so that I know my combined weekly headroom.
51. As a user, I want each pooled bar to say "N of M accounts available", so that I know how many accounts can still work.
52. As a user, I want each pooled bar to show the soonest reset time, so that I know when capacity comes back.
53. As a user, I want a git-style heatmap of the last 30 days, so that I can see my usage pattern by day.
54. As a user, I want to toggle the heatmap between daily Code-tab dollars and daily usage percentage, so that I can see both cost and load.
55. As a user, I want the usage-percentage view to show each day's peak weekly percentage averaged across accounts, so that the colour reflects overall load.
56. As a user, I want to tap or hover a heatmap day to see its date, total, and per-account breakdown, so that I can see who used what.
57. As a user, I want days with no data shown as empty rather than zero-cost, so that I can tell "no data" from "no usage".
58. As a user, I want no dollar estimate shown when prices can't be fetched (offline), with a clear note, so that I'm never shown made-up numbers.

### Usage data freshness

59. As a user, I want usage to refresh automatically on an adaptive schedule, so that numbers stay current without me doing anything.
60. As a user, I want polling to back off when the usage service rate-limits, so that my accounts aren't throttled further.
61. As a user, I want a manual refresh button on the Usage and Accounts tabs, so that I can force an update.
62. As a user, I want manual refresh to respect the minimum interval and tell me when the next refresh is allowed, so that I don't trigger rate limits.
63. As a user, I want stale usage marked with "updated X ago", so that I know when numbers might be out of date.
64. As a user, I want clawshell to fall back to the account's locally recorded usage history when it can't reach the usage service, so that I still see something useful.
65. As a user, I want clawshell never to refresh an account's sign-in token itself, so that it can never log out my running Claude.
66. As a user, I want an account whose token has expired to show its last known values as stale until Claude Desktop refreshes the token, so that nothing breaks while I'm away from that account.

### Notifications

67. As a user, I want a Windows notification when an account reaches my warning threshold (80% by default), so that I can plan ahead.
68. As a user, I want a notification when an account becomes Limited (100%), so that I know to move to another account.
69. As a user, I want a notification when a Limited account's window resets, so that I know it's usable again.
70. As a user, I want to set the warning and limit thresholds and toggle each notification in Settings, so that alerts match how I work.
71. As a user, I want each threshold to notify once per window, so that I'm not spammed on every poll.

### Settings

72. As a user, I want a switch to launch clawshell when Windows starts, so that it's always running.
73. As a user, I want a switch for whether clawshell starts shown or hidden in the tray, so that boot-time launch doesn't put a window in my face.
74. As a user, I want a "close to tray" switch, so that I decide what the close button does.
75. As a user, I want to pick English or Persian, so that I can use the app in my language.
76. As a user choosing Persian, I want the whole layout to flip right-to-left with Persian digits, so that it reads naturally.
77. As a user, I want a sign-in routing switch, so that I can turn routing on or off later.
78. As a user, I want notification switches and threshold inputs, so that I control alerts.
79. As a user, I want to point clawshell at Claude.exe manually when detection fails, so that it still works on unusual installs.
80. As a user, I want clawshell to auto-detect both the Store and installer builds of Claude, so that I usually never need the override.
81. As a user, I want my settings to persist across restarts, so that I set them once.

### Errors and edge cases

82. As a user, I want a clear message when Claude Desktop can't be found, with a way to set the path, so that I'm not stuck.
83. As a user, I want an account whose folder was deleted outside clawshell to be detected and offered for removal, so that the list stays accurate.
84. As a user, I want fatal errors written to a log file and shown in a dialog, so that problems can be diagnosed.
85. As a user, I want no tokens, cookies or email addresses written to logs, so that my credentials stay private.

## Implementation Decisions

### Platform and stack
- Tauri v2 app with a SvelteKit (static adapter, SSR off) frontend. The existing scaffold already uses Svelte 5, Tailwind 4, shadcn-svelte (vega style, Tabler icons), Paraglide i18n with `en` and `fa`, and Vitest browser mode.
- **Component policy:** use shadcn-svelte components first, then shadcn-svelte-extra, then other libraries, and only then hand-build. The 30-day heatmap is hand-built as a grid using shadcn Tooltip, because shadcn has no heatmap chart.
- **Window:** one main window with decorations off, a transparent background for Windows 11 rounded corners, fixed at 400×780, not resizable, and no maximize. The drag region is in the custom top bar.
- **Tauri plugins:** single-instance, tray, autostart, notification, and opener. Settings are stored by the core in its own JSON file.
- **Theme:** dark only. Claude Code colour tokens (warm terracotta accent on a near-black neutral) replace the scaffold's shadcn theme tokens.
- **RTL:** when the locale is `fa`, the document direction becomes RTL and numbers are formatted with Persian digits.

### Domain vocabulary
- **Account:** one clawshell-managed Claude Desktop identity, backed by one profile folder.
- **Profile folder:** `clawshell-{uuid}` directly under the local app-data folder, passed to Claude as its user-data-dir. The uuid is the account's permanent id, so a rename changes only the label.
- **App data:** clawshell's own metadata (labels, colours, per-account env and flags, settings, logs, router state) lives in a `clawshell` folder under local app data. It is kept separate from the profile folders.
- **Status:** Running | Idle | Signed out | Limited.
  - Running: a Claude Desktop process exists whose user-data-dir is this profile folder.
  - Signed out: no usable token.
  - Limited: 5-hour or weekly utilisation ≥ 100%.
  - Idle: otherwise.
  - Precedence: Signed out > Limited > Running > Idle for the badge. A running, limited account shows both Running and Limited.
- **Usage snapshot:** per account, 5-hour and weekly utilisation, each with a reset time, a fetched-at time, and a source (live | local-history | stale).
- **Pooled remaining:** the average of (100 − utilisation) over signed-in accounts, plus "N of M available" (accounts below 100%) and the soonest reset.

### Rust core module (the main seam)
- A single domain module, independent of Tauri, exposes these operations:
  - list accounts, with status and snapshot
  - add account
  - rename
  - set colour
  - set env/flags
  - delete
  - refresh usage (forced or scheduled)
  - month cost summary
  - 30-day heatmap ($ or %)
  - get settings / update settings
- It is built from injected ports:
  - **Filesystem roots:** local app data, user profile home.
  - **HTTP client:** the profile, usage and pricing endpoints.
  - **Clock**
  - **Token reader:** profile folder → access token or none.
  - **Process inspector:** which profile folders have a running Claude Desktop.
  - **OS adapters:** launcher, window focus, taskbar identity, shortcuts, link-handler registry, autostart, notifier.
- Tauri commands are thin wrappers over these operations, and push events to the UI when snapshots change.

### Identity and usage (adapted from claude-swap)
- **Token reader (Windows adapter):**
  - Reads `oauth:tokenCacheV2` from the profile's `config.json`. It is base64 of a `v10`-prefixed AES-256-GCM ciphertext.
  - The AES key comes from the profile's `Local State` `os_crypt.encrypted_key`, unwrapped with DPAPI as the current user.
  - Decryption happens in memory only. Tokens are never persisted, logged or sent anywhere except the two Anthropic endpoints below.
  - The plaintext structure of the decrypted token cache is unverified. Working out how to extract the access token and its expiry is part of the feasibility task.
- **Signed-in check without decryption:** `windowSizeWasSignedIn`, falling back to `oauth:tokenCacheV2` length > 100. This is the reference repo's heuristic.
- **Identity:** GET the OAuth profile endpoint with a bearer token. The email comes from the account. The name is clawshell's own editable label (default "Account N"), not read from the account.
- **Usage:** GET the OAuth usage endpoint with a bearer token and the `oauth-2025-04-20` beta header.
  - The fields used are the 5-hour and 7-day `utilization` and `resets_at`.
  - Extra-usage `used_credits` and `monthly_limit` are cents, used only when the account has extra usage enabled.
- **No token refresh, ever.**
  - If the token has expired or the endpoint returns 401, the account shows its last snapshot as stale.
  - It falls back to the profile's `plan-usage-history.json` (samples of 5-hour and weekly % with no reset times) until Claude Desktop refreshes the token itself.
- **Polling policy (claude-swap's):**
  - 180 s minimum interval and serve TTL.
  - 60 s when an account is near a limit, 300–600 s when idle, 600 s when exhausted, ±10% jitter.
  - On a 429, honour `Retry-After`. `Retry-After: 0` means the budget is saturated: back off 300 s, with a 360 s floor for an hour. Otherwise back off exponentially up to 1800 s.
  - Budget of about 30 requests per hour.
  - Manual refresh is subject to the same minimum interval.
- ⚠️ **Feasibility gate (first task):** confirm that a token decrypted from a Desktop profile is accepted by the profile and usage endpoints. If it isn't, fall back to local usage history for the limits and to claude.ai cache data for identity. That would mean revisiting the spec.

### Cost
- **Source:** Claude Desktop's Code tab writes transcripts to the user's shared Claude Code `projects` folder, not per profile. Each profile's `claude-code-sessions` folder lists its `cliSessionId` and `priorCliSessionIds`, and those ids map transcripts to accounts. Transcripts that match no account are ignored.
- **Per message:** cost = tokens × price, where tokens are input, output, cache-creation (5-minute and 1-hour) and cache-read, priced by model.
- **Dedup:** on message id plus request id, falling back to message id alone. Bucket by local calendar day of the message timestamp.
- **Prices:** always fetched online from LiteLLM's public model price JSON. With no prices, there is no dollar estimate and the UI says so.
- **This month:**
  - Code estimate (month to date) plus extra-usage spend (month to date, from the usage endpoint).
  - Shown as a total and per account, with the estimate portion labelled.
- **Heatmap:**
  - Last 30 days.
  - $ mode = daily Code estimate only; extra usage is excluded because it has no daily breakdown.
  - % mode = each day's peak weekly %, averaged across accounts that have data that day. The data comes from local usage-history samples plus clawshell's own polls held in memory for the session.
- **No history store of its own:** clawshell shows only what's still on disk. Claude Code deletes transcripts after about 30 days.

### Launching and windows (from the reference switcher)
- **Detection:** the Store/MSIX build is found via its package (WindowsApps), the installer build via its install folder. An optional Claude.exe path override is remembered.
- **Launch:**
  - Start Claude.exe with the profile folder as its user-data-dir, plus that account's extra flags.
  - The account's env vars apply only to that child process.
  - If the profile is already running, focus its window instead of starting another.
- **Running detection:** only the Desktop app's own processes count. Code CLI child processes are ignored.
- **Taskbar identity:** each account's window gets its own AppUserModelID and badged icon, applied between window creation and first show, as the reference does. The badge letter and colour are part of the identity.
- **Shortcuts:** Desktop and Start-menu shortcuts per account, each with its own icon file. They launch through clawshell, which focuses the account if it's already running.
- **Sign-in routing:**
  - **Turning it on:** registers clawshell as a `claude://` handler under the current user and backs up Claude's original handler. On the Store build, it opens the Default Apps page for the user's one-time pick.
  - **Picking the target:** the account launched while signed out, then the only open signed-out account, then the account whose window was used last, then the original Claude install.
  - Re-takes the handler when Claude rewrites it.
  - **Turning it off:** restores the backup exactly.
  - Never logs links.

### Settings (persisted by the core)
- Launch at boot (bool) and start hidden (bool)
- Close to tray (bool)
- Language (en | fa)
- Sign-in routing (bool)
- Notifications: warning on/off with threshold (default 80), limited on/off with threshold (default 100), reset on/off
- Claude.exe path override (optional)
- Per-account: label, colour, env map, flags list

### Notifications
- Fired by the core when a snapshot crosses a threshold, at most once per account per window (5-hour or weekly), keyed by that window's `resets_at`.
- A reset notification fires when a previously Limited window resets.

## Testing Decisions

- **What makes a good test:** it exercises external behaviour through a seam: given these files on disk and these HTTP responses at this time, the operation returns this result. Tests never reach into private functions or assert on how something is computed internally. They must survive refactors of the internals.
- **Seam 1, the Rust core (most tests):**
  - **Setup:** each test builds a temp folder with fixture profile folders (`config.json`, `Local State`, `plan-usage-history.json`, `claude-code-sessions` session files), a fixture shared `projects` folder with JSONL transcripts, and a scripted fake HTTP client and clock. The token reader and process inspector are faked at their ports.
  - **Covers:**
    - account lifecycle (add, rename, delete blocked while running, folder naming)
    - status derivation and precedence
    - pooled remaining and "N of M available" with soonest reset
    - stale and local-history fallback on 401 or expired token
    - polling and backoff decisions on 429 with and without `Retry-After`
    - minimum-interval enforcement for manual refresh
    - transcript-to-account mapping
    - dedup and per-model pricing, and no estimate when prices fail
    - month total including extra usage
    - 30-day heatmap in both modes, with empty days distinct from zero
    - notification threshold crossings firing once per window
    - settings persistence
- **Seam 2, the UI at the IPC boundary:**
  - Svelte screens rendered in Vitest browser mode, with Tauri's IPC mocked to return canned core results.
  - **Covers:**
    - tab navigation
    - Accounts rows and status badges
    - double-click invoking launch
    - Add, Rename and Delete (with confirmation) flows
    - Usage tab totals, pooled bars and heatmap toggle and tooltip
    - Settings toggles and threshold inputs calling update
    - Persian locale switching the document to RTL with Persian digits
- **Not automated:** DPAPI/AES token decryption, launching and focusing Claude.exe, taskbar AppUserModelID and badge icons, `claude://` registry routing and Default Apps, autostart, shortcut creation, toast notifications. These are thin adapters behind ports, verified by hand on Windows 11 against the Store build. The token path is also covered by the first-task feasibility check.
- **Prior art:** the scaffold's Vitest examples, a plain unit spec and a vitest-browser-svelte component spec, are the pattern for seam 2. There is no Rust test prior art yet; seam 1 introduces the core's integration tests.

## Out of Scope

- The Claude Code CLI as a managed target (no per-account `CLAUDE_CONFIG_DIR` handling).
- Listing or managing the original Claude Desktop sign-in, or the reference switcher's existing `ClaudeProfile-*` folders (no import or migration).
- Refreshing OAuth tokens on any account's behalf.
- A clawshell-owned usage or cost history database. Data beyond what's on disk (about 30 days) isn't shown.
- Cost data for regular claude.ai chats (none exists locally).
- Chat transfer between accounts.
- Light theme or a theme picker.
- Resizable or maximisable window.
- macOS and Linux.
- Routing non-sign-in `claude://` links to a chosen account (the reference's "open links in" option).

## Further Notes

- All usage and profile endpoints are undocumented and may change or be rate-limited. Automated access with credentials read from Claude Desktop may conflict with Anthropic's consumer terms; the user has accepted this trade-off for v1.
- The reference implementation is Claude-Code-Desktop-Switcher (MIT). Its README and script document the Desktop-specific details: the Cowork folder constraint, the taskbar identity trick, routing order, and running detection. claude-swap (MIT) is the reference for the usage and profile endpoints, headers and polling policy.
- `plan-usage-history.json` only gains samples while that account's Claude usage tray has been opened within the last 24 hours, so the local fallback can lag.
- Each running account is a full Claude Desktop instance, roughly one Claude's worth of memory per account.

### Feasibility findings: Desktop token (ticket 01)

Probe: `cargo run --example token_probe -- "<profile folder>"` in `src-tauri`, run once per endpoint against a signed-in profile (a second run repeated both calls to capture nested key names). No refresh was attempted and no secret was printed or written.

- **Verdict: the design holds.** A token decrypted from a Desktop profile is accepted by both endpoints. The local-history and claude.ai-cache fallback is not needed for identity or usage while the token is valid. The existing fallback for an expired token or a 401 (last snapshot stale, then `plan-usage-history.json`) still applies.
- **Decryption works as specified:** `os_crypt.encrypted_key` is base64 of `DPAPI` + the DPAPI blob. DPAPI as the current user yields a 32-byte AES-256 key. `oauth:tokenCacheV2` is base64 of `v10` + 12-byte nonce + ciphertext with the 16-byte GCM tag last.
- **Plaintext is a JSON object keyed by cache key, not a single token record.** Key shape: `acct:{id}:{id}:{auth base url}:{scopes}`, with scopes space-separated. This profile has three entries, with scope sets `user:inference user:file_upload user:profile`, the same plus `user:sessions:claude_code`, and `user:profile` alone. Each entry holds these fields (names only): `token`, `refreshToken`, `expiresAt`, `identityChecked`, `rateLimitTier`, `subscriptionType`, and in one entry `pluginsScope`.
- **Access token and expiry:** `token` is the access token. `expiresAt` is a 13-digit number, i.e. epoch milliseconds. The token had not expired at probe time.
- **Which entry to use:** every entry carried `user:profile`, which both endpoints need. `extract_token` prefers entries with that scope, then the latest `expiresAt`. Which of the three entries the endpoints accept individually was not tested; the chosen one was accepted.
- **HTTP statuses (once each, `Authorization: Bearer`, usage with `anthropic-beta: oauth-2025-04-20`):** `GET /api/oauth/profile` 200, `GET /api/oauth/usage` 200.
- **Profile response keys:** top level `account`, `application`, `enabled_plugins`, `organization`. `account` has `created_at`, `display_name`, `email`, `full_name`, `has_claude_max`, `has_claude_pro`, `uuid`. `organization` has `billing_type`, `has_extra_usage_enabled`, `name`, `organization_type`, `rate_limit_tier`, `subscription_created_at`, `subscription_status`, `uuid`, and a few feedback and onboarding flags.
- **Usage response keys:** `five_hour` and `seven_day` each carry `utilization` (number) and `resets_at` (string), as the spec assumed. Also present: `extra_usage` (`is_enabled`, `credits_ever_enabled`, `spend_limit_reached`, `user_disabled`, `utilization`), `limits` (array), `spend` (`percent`, `severity`, `enabled`, `used`{`amount_minor`, `currency`, `exponent`}, and others), `seven_day_breakdown` (`as_of`, `window_started_at`, `rows`), `member_dashboard_available`, and many per-model or experimental keys that were null here (`seven_day_opus`, `seven_day_sonnet`, and others).
- **Spec adjustment for the usage ticket:** `extra_usage.used_credits` and `monthly_limit` were not present in this response (the account's extra usage looks disabled). The `spend.used` money object (`amount_minor`, `currency`, `exponent`) is a candidate source for extra-usage spend. Re-check against an account with extra usage enabled before relying on either shape, and parse every field as optional.
- **Fallback decision:** none needed. If a later call is rejected (401, or `expiresAt` in the past), apply the existing stale-snapshot then `plan-usage-history.json` fallback.
