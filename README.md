# clawshell

clawshell is a small Windows app for people with more than one Claude subscription. It runs each Claude Desktop account in its own window at the same time, so you never sign out to switch. It also shows how much of each account's 5-hour and weekly limit is left and what your Code-tab usage costs.

> [!NOTE]
> clawshell is in the planning stage. The repo holds a Tauri + SvelteKit scaffold, a spec and 15 tickets. None of the features below work yet.

## What it will do

The window is phone-sized (400×780) with three tabs along the bottom.

**Usage** shows this month's dollar total across all accounts, split per account. The total adds an estimate for Code-tab sessions, priced from token counts, to any real extra-usage spend. Two bars show how much 5-hour and weekly capacity is left on average across your signed-in accounts, how many accounts can still work, and when the next one resets. A 30-day heatmap shows daily cost or daily load.

**Accounts** lists each account with its name, email, 5-hour and weekly usage, and a status of Running, Idle, Signed out or Limited. Double-click an account to open its Claude window, or bring it forward if it's already open. The Add button creates a new account and opens Claude at the sign-in screen. Each account can be renamed, deleted, given a badge colour, its own environment variables and launch flags, and desktop shortcuts.

**Settings** covers launch at boot (shown or hidden), close to tray, English or Persian (with a right-to-left layout), sign-in routing, limit notifications at thresholds you set, and a manual path to `Claude.exe` if detection fails.

clawshell lives in the tray. Each account gets its own taskbar button with a coloured badge, and browser sign-ins go back to the account that started them.

## How it works

Claude Desktop is an Electron app, and Electron accepts `--user-data-dir`. clawshell gives every account its own folder, `%LOCALAPPDATA%\clawshell-{uuid}`, and launches Claude with it. Each folder has its own cookies and sign-in, so the accounts run side by side. The folders sit directly under `%LOCALAPPDATA%` because Cowork's local VM only looks there. Your original Claude sign-in is never touched.

Usage numbers come from two undocumented Anthropic OAuth endpoints, one for the profile and one for usage. clawshell reads each account's token from its folder, in memory only, and never refreshes it. A refresh would rotate the token under the running Claude and sign it out. When a token has expired, clawshell shows the last values as stale and falls back to the usage history Claude Desktop keeps in each profile.

Cost comes from the transcripts Claude Code writes for Code-tab sessions. clawshell matches them to accounts by session id and prices the tokens with LiteLLM's public price list. Regular claude.ai chats leave no token data on disk, so they aren't counted. Claude Code deletes transcripts after 30 days, which is why the heatmap covers 30 days.

## Development

You need Node.js, Rust and the [Tauri prerequisites for Windows](https://tauri.app/start/prerequisites/).

```powershell
npm install
npm run tauri dev
```

Other scripts:

| Command | Does |
| --- | --- |
| `npm test` | Run the Vitest suite once |
| `npm run check` | Type-check with svelte-check |
| `npm run lint` | Prettier and ESLint |
| `npm run tauri build` | Build the Windows installer |

The stack is Tauri v2, SvelteKit with Svelte 5, Tailwind 4, shadcn-svelte and Paraglide for English and Persian.

## Planning docs

- `.scratch/clawshell-v1/spec.md` has the full spec, including user stories, implementation decisions and testing seams.
- `.scratch/clawshell-v1/issues/` has the tickets in dependency order. Start with 01 and 02.
- `docs/agents/` tells coding agents where issues live and how they're labelled.

Ticket 01 checks that a token read from a Claude Desktop profile is accepted by the usage endpoint. If it isn't, the usage design in the spec changes.

## Credits

clawshell follows two MIT-licensed projects. [Claude-Code-Desktop-Switcher](https://github.com/PriyanshuGeTRekT/Claude-Code-Desktop-Switcher) worked out the profile folders, taskbar identity and sign-in routing for Claude Desktop. [claude-swap](https://github.com/realiti4/claude-swap) worked out the usage endpoint and its polling limits.

## Disclaimer

clawshell is unofficial and not affiliated with Anthropic. It reads tokens that Claude Desktop stores on your machine and calls endpoints Anthropic doesn't document, which may change or break without notice. Anthropic's terms of service apply to using more than one account.
