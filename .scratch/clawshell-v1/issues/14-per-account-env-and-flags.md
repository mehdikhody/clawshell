# 14: Per-account env vars and launch flags

**What to build:** The ⋯ menu offers an editor for the account's environment variables (name/value pairs) and extra launch flags. Env vars apply only to that account's Claude process; flags go on every launch from the window, the tray and shortcuts. Invalid env names (anything other than letters, digits and `_`, or starting with a digit) are rejected. The editor explains that changes apply only when the account's Claude starts fresh. See spec: user stories 30–32.

**Blocked by:** 04

**Status:** resolved

- [x] Env vars and flags can be added, edited and removed per account, and persist
- [x] Invalid env names are rejected with a message
- [x] Launch passes the flags and sets the env vars only for that child process
- [x] The editor shows the "applies on fresh start" note
- [x] Core tests cover validation and launch-argument assembly via a faked launcher; UI tests cover the editor
