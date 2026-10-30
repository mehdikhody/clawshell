# 15: Sign-in routing

**What to build:** Browser sign-ins (`claude://` links) land in the account that asked for them.
- **First Add account:** clawshell offers to turn routing on.
- **Turning it on:**
  - registers clawshell as a `claude://` handler under the current user
  - backs up Claude's original handler
  - on the Store build, opens the Default Apps page with the one choice the user must make
- **Picking the target:** the account launched while signed out, then the only open signed-out account, then the account whose window was used last, then the original Claude install.
- **Handler upkeep:** clawshell re-takes the handler when Claude rewrites it.
- **Settings toggle:** turning it off restores the backup exactly.
- **Safety:** links are validated, and their contents are never logged.

See spec: Launching and windows (Sign-in routing).

**Blocked by:** 03, 09

**Status:** ready-for-agent

- [ ] The offer appears on the first Add; the Settings toggle turns routing on and off
- [ ] A sign-in link is routed according to the order above
- [ ] Turning it off leaves the registry exactly as before; the Store-build Default Apps step is guided
- [ ] Non-`claude://` or malformed links are refused; no link contents appear in any log
- [ ] Core tests cover the target-selection order and link validation; the registry and Default Apps steps are checked by hand on Windows 11
