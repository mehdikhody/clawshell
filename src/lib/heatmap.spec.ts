import { describe, expect, it } from 'vitest';
import { intlLocale } from './locale';
import { cellsFor, formatDay, intensity, parseDay, type HeatmapDay } from './heatmap';

const day = (date: string): HeatmapDay => ({ date, usd: null, tokens: null });

describe('intensity', () => {
	it('has no level for a day without data', () => {
		expect(intensity(null, 10)).toBeNull();
	});

	it('gives a real zero level 0, apart from no data', () => {
		expect(intensity(0, 10)).toBe(0);
	});

	it('buckets dollars against the busiest day', () => {
		expect(intensity(0.01, 10)).toBe(1);
		expect(intensity(2.5, 10)).toBe(1);
		expect(intensity(2.6, 10)).toBe(2);
		expect(intensity(5, 10)).toBe(2);
		expect(intensity(7.5, 10)).toBe(3);
		expect(intensity(7.6, 10)).toBe(4);
		expect(intensity(10, 10)).toBe(4);
	});
});

describe('cellsFor', () => {
	it('pads the first week so each day sits under its weekday (Sunday first)', () => {
		// 2026-06-10 is a Wednesday.
		const cells = cellsFor([day('2026-06-10'), day('2026-06-11')]);
		expect(cells.map((c) => c?.date ?? null)).toEqual([
			null,
			null,
			null,
			'2026-06-10',
			'2026-06-11'
		]);
	});

	it('does not pad a window that starts on a Sunday', () => {
		expect(cellsFor([day('2026-06-14')])[0]?.date).toBe('2026-06-14');
	});
});

describe('parseDay', () => {
	it('reads a calendar day as local midnight', () => {
		const d = parseDay('2026-06-10');
		expect([d.getFullYear(), d.getMonth(), d.getDate(), d.getHours()]).toEqual([2026, 5, 10, 0]);
	});
});

describe('formatDay', () => {
	it('writes a short date', () => {
		expect(formatDay('2026-06-15', 'en-US')).toBe('Jun 15');
	});

	it('writes a Gregorian date in Persian digits', () => {
		expect(formatDay('2026-06-15', intlLocale('fa'))).toBe('۱۵ ژوئن');
	});
});
