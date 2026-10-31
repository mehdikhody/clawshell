# 06: Pooled remaining bars

**What to build:** The Usage tab shows two pooled bars, one for 5-hour and one for weekly. Each bar shows the average remaining percentage (100 − utilisation) across signed-in accounts, the caption "N of M accounts available" (counting accounts below 100%), and the soonest reset time. Stale inputs are indicated. A manual refresh button sits on the Usage tab, as on Accounts. See spec: Domain vocabulary (Pooled remaining).

**Blocked by:** 05

**Status:** done

- [x] For 3 accounts at 60/0/0% used, the bar shows 80% remaining
- [x] Signed-out accounts are excluded from both the average and M
- [x] "N of M available" and the soonest reset are correct and update live
- [x] The empty state (no signed-in accounts) is handled
- [x] Core tests cover the pooling maths; UI tests cover the bars and captions
