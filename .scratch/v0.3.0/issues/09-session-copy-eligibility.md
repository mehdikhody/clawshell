# 09: Remember each account's UUIDs and mark ineligible copy targets

**What to build:** clawshell stores each signed-in account's account UUID and organisation UUID, taken from the profile lookup the usage poller already makes for the email. An account is an eligible copy target when it's signed in, both IDs are known, and its `claude-code-sessions/{accountUuid}/{orgUuid}` folder exists. Otherwise the account carries a reason: "Sign in to this account first", or "Open the Code tab in this account once first". Ticket 11 shows these reasons in the target picker. See spec: Session copy (Target folder).

**Blocked by:** None (can start immediately)

**Status:** ready-for-agent

- [ ] The account UUID and organisation UUID are stored beside the email when the profile is read
- [ ] Each account reports whether it can receive copies, with the reason when it can't
- [ ] Core tests cover signed-out, IDs unknown, folder missing, and eligible
- [ ] A user-facing entry is added under `## [Unreleased]` in `CHANGELOG.md` only if this ticket changes what the user sees; otherwise note in the commit that it doesn't
