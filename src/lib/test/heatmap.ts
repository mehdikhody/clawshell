import type { DayValues, Heatmap, HeatmapDay } from '$lib/heatmap';

/** The 9 weeks ending Monday 2026-06-15: Sunday 2026-04-19 on, oldest first. */
const DATES = Array.from({ length: 58 }, (_, i) => {
	const d = new Date(2026, 3, 19 + i);
	const pad = (n: number) => String(n).padStart(2, '0');
	return `${d.getFullYear()}-${pad(d.getMonth() + 1)}-${pad(d.getDate())}`;
});

/** A day's dollars or tokens from `[account, value]` pairs; the total is their sum. */
export const values = (rows: [string, number][]): DayValues => ({
	total: rows.reduce((s, [, v]) => s + v, 0),
	accounts: rows.map(([label, value]) => ({ id: label, label, colour: '#d97757', value }))
});

/** 9 weeks with no data anywhere; `days` fills in individual dates. */
export const heatmapOf = (
	days: Record<string, Partial<Pick<HeatmapDay, 'usd' | 'tokens'>>> = {},
	over: Partial<Heatmap> = {}
): Heatmap => ({
	days: DATES.map((date) => ({ date, usd: null, tokens: null, ...days[date] })),
	noEstimate: null,
	...over
});
