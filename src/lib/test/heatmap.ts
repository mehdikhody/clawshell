import type { DayValues, Heatmap, HeatmapDay } from '$lib/heatmap';

/** The 30 local days ending 2026-06-15, oldest first. */
const DATES = Array.from({ length: 30 }, (_, i) => {
	const d = new Date(2026, 4, 17 + i);
	const pad = (n: number) => String(n).padStart(2, '0');
	return `${d.getFullYear()}-${pad(d.getMonth() + 1)}-${pad(d.getDate())}`;
});

/** A day's values from `[account, value]` pairs; the total is their sum, or the mean for percent. */
export const values = (rows: [string, number][], mean = false): DayValues => {
	const sum = rows.reduce((s, [, v]) => s + v, 0);
	return {
		total: mean && rows.length > 0 ? sum / rows.length : sum,
		accounts: rows.map(([label, value]) => ({ id: label, label, value }))
	};
};

/** 30 days with no data anywhere; `days` fills in individual dates. */
export const heatmapOf = (
	days: Record<string, Partial<Pick<HeatmapDay, 'usd' | 'percent'>>> = {},
	over: Partial<Heatmap> = {}
): Heatmap => ({
	days: DATES.map((date) => ({ date, usd: null, percent: null, ...days[date] })),
	noEstimate: null,
	...over
});
