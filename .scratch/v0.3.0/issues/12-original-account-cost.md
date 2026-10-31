# 12: Keep a copied session's cost with the account it was created in

**What to build:** Before the first copy of a session, clawshell records its session ID and the original account ID in `clawshell/session-origins.json`. Copies never overwrite an existing entry. Cost calculation uses this record first and falls back to today's first-found rule. Monthly totals don't move when a session is copied. See spec: Session copy (Original account).

**Blocked by:** 11

**Status:** ready-for-agent

- [ ] `session-origins.json` gets an entry the first time a session is copied, and never gets a second entry for the same session
- [ ] The cost calculation attributes a copied session to its recorded original account, falling back to the first-found rule when there's no entry
- [ ] Monthly totals are unchanged by a copy
- [ ] Core tests cover the single write, the no-overwrite rule and the attribution with and without an entry
- [ ] A user-facing entry is added under `## [Unreleased]` in `CHANGELOG.md`
