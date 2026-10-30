# 05: Live usage per account

**What to build:** Each account row shows its email and its 5-hour and weekly usage with reset times, and the status gains Limited.
- **Fetching:** the core reads each profile's token in memory using the Windows token-reader adapter proven in 01. It gets the email from the OAuth profile endpoint and the 5-hour/7-day utilisation, `resets_at`, and extra-usage fields from the OAuth usage endpoint.
- **Polling:** claude-swap's policy: 180 s minimum interval, adaptive 60/300–600/600 s, ±10% jitter, `Retry-After` handling including the saturated-budget case, and exponential backoff.
- **Manual refresh:** a button on the Accounts tab, which respects the minimum interval and says when the next refresh is allowed.
- **Expired token or 401:** never refresh. Keep the last snapshot marked stale ("updated X ago"), and fall back to the profile's `plan-usage-history.json`.
- **Limited:** shown when either window is at or above 100%.
- **Logging:** tokens and emails are never logged.

See spec: Identity and usage.

**Blocked by:** 01, 03

**Status:** ready-for-agent

- [ ] Rows show the email, 5h % with reset, and weekly % with reset
- [ ] Limited status appears at ≥100% on either window, with precedence Signed out > Limited > Running > Idle
- [ ] Polling follows the policy; on a 429 it backs off as specified
- [ ] Manual refresh obeys the minimum interval
- [ ] An expired token or 401 shows a stale snapshot, then local-history values, and never triggers a token refresh
- [ ] Snapshot changes are pushed to the UI as events
- [ ] Core tests (fake HTTP and clock) cover fetch, polling and backoff decisions, stale fallback, and Limited; UI tests cover the row rendering and refresh button states
