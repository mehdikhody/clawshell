# 07: This month's $

**What to build:** The Usage tab shows this month's dollar total, made of two parts:
- **Code-tab estimate (labelled "est."):** transcripts in the shared Claude Code `projects` folder are mapped to accounts via each profile's `claude-code-sessions` (`cliSessionId` / `priorCliSessionIds`); unmapped transcripts are ignored. Usage records are deduped (message id + request id, falling back to message id) and priced per model, covering input, output, cache-creation (5-minute and 1-hour) and cache-read tokens. Prices are always fetched online from LiteLLM.
- **Extra-usage spend:** month to date, in cents converted to dollars, from the usage endpoint.

A per-account list shows each account's dollars and share. If prices can't be fetched, no estimate is shown and a note explains why. See spec: Cost.

**Blocked by:** 03, 05

**Status:** done

- [x] Transcripts map to the right account; transcripts from other or unknown sessions are ignored
- [x] Duplicate streamed records are counted once
- [x] The month total = Code estimate + extra usage, with the estimate labelled
- [x] The per-account breakdown sums to the total
- [x] Offline or price-fetch failure shows "no estimate" rather than $0
- [x] Core tests cover mapping, dedup, pricing, month bucketing (local day) and the offline case; UI tests cover the headline and breakdown
