# Session copies share one transcript, and cost stays with the original account

Claude Desktop keeps a small record per session in each profile and the transcript itself in one place shared by every profile. A session copy therefore copies only the record into the target profile, pointed at the target's signed-in account and organisation; the source keeps its record, and both accounts continue the same transcript. The transcript doesn't say which account sent each message, so we can't split a copied session's cost between accounts. All of it, including turns taken later in a copy, stays with the original account, the one the session was first created in. We picked that over splitting by copy time, which goes wrong as soon as both accounts keep using the session.

## Consequences

- The cost per account on This month and the heatmap is slightly off for sessions continued in a copy. Usage limits are unaffected: they come from Anthropic per account.
- clawshell has to remember which account a session was created in, because after a copy the records alone can't tell.
- Copying a session to an account that already has it replaces that account's record with the source's.
