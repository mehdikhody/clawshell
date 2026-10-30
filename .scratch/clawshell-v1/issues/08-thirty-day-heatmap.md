# 08: 30-day heatmap

**What to build:** A git-style heatmap of the last 30 days on the Usage tab, hand-built as a grid with shadcn Tooltip. A toggle switches between two modes:
- **$:** each day's Code-tab estimate summed across accounts. Extra usage is excluded.
- **%:** each day's peak weekly utilisation, averaged across the accounts that have data that day. The data comes from `plan-usage-history.json` samples plus this session's polls.

Intensity is bucketed. Hover or tap shows the date, the total and the per-account breakdown. Days with no data look different from zero days. See spec: Cost (Heatmap).

**Blocked by:** 06, 07

**Status:** ready-for-agent

- [ ] 30 cells ending today, in local days, fit 400px without scrolling
- [ ] The $ / % toggle works; % mode uses peak weekly averaged across accounts
- [ ] The tooltip shows the date, the total and the per-account values
- [ ] No-data days render distinctly from zero
- [ ] Core tests cover both modes' daily values and empty-vs-zero; UI tests cover the toggle and tooltip
