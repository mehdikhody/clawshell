# 11: Copy ticked sessions into a target account

**What to build:** The Copy sessions dialog gets a target picker listing the other accounts, the default account included. Ineligible targets are greyed out with their reason (ticket 09). Copy stays disabled until at least one session and a target are chosen. Copy writes each ticked session's record into the target's current account and organisation folder under the same file name. A record with the same `cliSessionId` already in the target profile is replaced. Archived sessions stay archived. Fields tied to the source account are cleared or kept as decided in ticket 08. Each file is written to a temporary name, then renamed. The source keeps its records. The result line reads "{count} sessions copied", plus "Restart {target} to see them" when the target's Claude is running. The operation refuses ineligible targets, and an unknown layout leaves no half-written record. See spec: Session copy (Copying, UI).

**Blocked by:** 08 (ready-for-human), 09, 10

**Status:** ready-for-agent

- [ ] The target picker lists the other accounts; ineligible ones are disabled with their reason
- [ ] Copy is disabled until a session and a target are chosen
- [ ] Copied files land in the target's current account and organisation folder under the same file name
- [ ] An existing record with the same `cliSessionId` is replaced, anywhere in the target profile
- [ ] Source records are untouched; `isArchived` is kept; source-tied fields are handled as ticket 08 decided
- [ ] Writes use a temporary name and rename; an unknown layout or ineligible target fails without writing
- [ ] The result notice shows the count, plus the restart hint when `targetRunning` is true
- [ ] Core tests cover the write, replacement, archived flag, ineligible refusal and `targetRunning`; component tests cover the disabled states, the Copy gating and the result notice
- [ ] A user-facing entry is added under `## [Unreleased]` in `CHANGELOG.md`
