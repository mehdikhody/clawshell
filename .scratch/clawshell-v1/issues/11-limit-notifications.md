# 11: Limit notifications

**What to build:** clawshell sends Windows notifications when an account's 5-hour or weekly usage:
- crosses the warning threshold (default 80%)
- crosses the limit threshold (default 100%)
- resets after being Limited

Each notification fires at most once per account per window, keyed by that window's `resets_at`. Settings has an on/off switch for each notification type and editable warning and limit thresholds. See spec: Notifications.

**Blocked by:** 05, 09

**Status:** ready-for-agent

- [ ] Warning, limited and reset notifications fire at the right crossings
- [ ] There are no repeats within the same window, across polls or restarts
- [ ] Thresholds and on/off switches in Settings take effect immediately and persist
- [ ] Core tests (fake clock and HTTP sequence) cover crossings, dedup and resets; UI tests cover the settings controls
