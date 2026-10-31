# Copy sessions into the default account

Amends ADR 0001. The default account was only ever read by clawshell. With session copies, it can now be a target as well as a source: clawshell writes the copied session's record into the default account's profile, under the account and organisation that profile is signed into now. Most users keep their main subscription in the default account, so refusing it as a target would block the most common copy (out of a limited account, into the one with headroom). Nothing else changes: its token is still never refreshed, it still can't be deleted, and it still has no badge, shortcuts or taskbar identity of its own.

## Consequences

- A session copy is the only write clawshell makes into Claude's own profile. Anything else that wants to write there needs its own decision.
- If Claude changes how its profile stores sessions, a copy can now damage the default account's session list too, not only a clawshell-made one.
