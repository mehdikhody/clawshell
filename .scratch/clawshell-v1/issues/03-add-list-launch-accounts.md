# 03: Add, list and launch accounts

**What to build:** clawshell detects Claude Desktop (Store/MSIX build via its package, installer build via its install folder).
- **Add:** creates a `clawshell-{uuid}` profile folder directly under local app data, records the account in clawshell's app data with the label "Account N", and opens Claude Desktop with that folder as its user-data-dir so it lands on the sign-in screen.
- **Accounts tab:** each account shows as a row with its name and its status: Running, Idle or Signed out. Running means a Claude Desktop process uses that profile folder; Code CLI child processes are ignored. Signed out uses the `windowSizeWasSignedIn` / token-length heuristic.
- **Double-click:** launches the account, or focuses its window if it's already running.
- **Detection failure:** if Claude can't be found, a clear message is shown.

See spec: Domain vocabulary, Launching and windows.

**Blocked by:** 02

**Status:** done

- [x] Add creates a uniquely named `clawshell-{uuid}` folder and opens Claude at sign-in
- [x] New accounts are labelled Account 2, Account 3… with no duplicates
- [x] The Accounts tab lists accounts with Running / Idle / Signed out, and statuses update when a Claude window opens or closes
- [x] Double-click starts a stopped account and focuses a running one (no second copy)
- [x] Original Claude data and `ClaudeProfile-*` folders are never listed or touched
- [x] Core tests cover add, naming, status derivation and the running check via a faked process inspector
- [x] UI tests cover the row list, double-click invoking launch, and Add
