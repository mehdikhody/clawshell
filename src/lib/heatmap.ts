import { invoke } from '@tauri-apps/api/core';
import type { NoEstimate } from '$lib/cost';

export type DayAccount = { id: string; label: string; value: number };

/** A day's total and each account's part, biggest first. */
export type DayValues = { total: number; accounts: DayAccount[] };

export type HeatmapDay = {
	/** The local calendar day, `YYYY-MM-DD`. */
	date: string;
	/** Dollars; null when nothing is known about the day: not the same as zero. */
	usd: DayValues | null;
	/** Tokens used that day across accounts; null when nothing is known about the day. */
	tokens: number | null;
};

export type Heatmap = {
	/** The last 60 local days, oldest first, today last. */
	days: HeatmapDay[];
	/** Why every dollar cell is empty, when prices are the reason. */
	noEstimate: NoEstimate | null;
};

/** The last 60 days; may fetch model prices, so it can take a moment. */
export const heatmap = () => invoke<Heatmap>('heatmap');

/** 0 is a real zero; 1 to 4 are increasingly busy. */
export type Level = 0 | 1 | 2 | 3 | 4;

/**
 * The bucket of a day's value, or null when the day has no data.
 * Values are relative to `maxUsd`, the busiest day shown.
 */
export function intensity(value: number | null, maxUsd: number): Level | null {
	if (value === null) return null;
	if (value <= 0) return 0;
	const ratio = maxUsd > 0 ? value / maxUsd : 1;
	if (ratio <= 0.25) return 1;
	if (ratio <= 0.5) return 2;
	if (ratio <= 0.75) return 3;
	return 4;
}

/** A calendar day as local midnight. */
export function parseDay(date: string): Date {
	const [year, month, day] = date.split('-').map(Number);
	return new Date(year, month - 1, day);
}

/**
 * The days laid out in week columns, Sunday on top: blanks (null) pad the first
 * week so every day sits under its weekday. Fill order is down each column.
 */
export function cellsFor(days: HeatmapDay[]): (HeatmapDay | null)[] {
	if (days.length === 0) return [];
	const lead = parseDay(days[0].date).getDay();
	return [...Array<null>(lead).fill(null), ...days];
}

/** A calendar day as a short date, such as "Jun 15". */
export const formatDay = (date: string, locale: string): string =>
	new Intl.DateTimeFormat(locale, { month: 'short', day: 'numeric' }).format(parseDay(date));
