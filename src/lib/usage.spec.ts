import { describe, expect, it } from 'vitest';
import { intlLocale } from './locale';
import { formatAgo, formatCountdown, formatReset, formatResetIn } from './usage';

// Local-time dates keep these independent of the machine's time zone.
const at = (y: number, mo: number, d: number, h: number, mi = 0) => new Date(y, mo - 1, d, h, mi);

describe('formatReset', () => {
	const now = at(2026, 6, 15, 12);

	it('shows just the time for a reset later today', () => {
		const iso = at(2026, 6, 15, 15, 0).toISOString();
		expect(formatReset(iso, now, 'en-US')).toMatch(/^3:00\s?PM$/);
	});

	it('adds the weekday for a reset on another day this week', () => {
		const iso = at(2026, 6, 19, 9, 30).toISOString();
		expect(formatReset(iso, now, 'en-US')).toMatch(/^Fri,? 9:30\s?AM$/);
	});

	it('shows the date for a reset a week or more away', () => {
		const iso = at(2026, 6, 25, 9, 30).toISOString();
		expect(formatReset(iso, now, 'en-US')).toMatch(/^Jun 25,? 9:30\s?AM$/);
	});

	it('returns an empty string for a missing or unreadable time', () => {
		expect(formatReset(null, now, 'en-US')).toBe('');
		expect(formatReset('not a date', now, 'en-US')).toBe('');
	});
});

describe('formatReset in Persian', () => {
	const fa = intlLocale('fa');
	const now = at(2026, 6, 15, 12);

	it('writes the clock time in Persian digits', () => {
		expect(formatReset(at(2026, 6, 15, 15, 0).toISOString(), now, fa)).toBe('۱۵:۰۰');
	});

	it('writes the date in the Jalali calendar, with Persian digits', () => {
		// 25 June 2026 is 4 Tir 1405.
		const text = formatReset(at(2026, 6, 25, 9, 30).toISOString(), now, fa);
		expect(text).toBe('۴ تیر، ۹:۳۰');
	});
});

describe('formatAgo', () => {
	const now = 1_781_524_800_000;

	it('says now for data under a minute old', () => {
		expect(formatAgo(now - 10_000, now, 'en-US')).toBe('now');
	});

	it('counts minutes, hours and days', () => {
		expect(formatAgo(now - 5 * 60_000, now, 'en-US')).toBe('5 minutes ago');
		expect(formatAgo(now - 3 * 3_600_000, now, 'en-US')).toBe('3 hours ago');
		expect(formatAgo(now - 2 * 86_400_000, now, 'en-US')).toBe('2 days ago');
	});

	it('never reports the future', () => {
		expect(formatAgo(now + 60_000, now, 'en-US')).toBe('now');
	});
});

describe('formatResetIn', () => {
	const now = 1_781_524_800_000;
	const MIN = 60_000;
	const iso = (ms: number) => new Date(now + ms).toISOString();
	const soon = 'in less than a minute';

	it('rounds down to one unit, so it never reads later than the reset', () => {
		expect(formatResetIn(iso((24 + 20) * 60 * MIN), now, 'en-US', soon)).toBe('in 1 day');
		expect(formatResetIn(iso((5 * 60 + 50) * MIN), now, 'en-US', soon)).toBe('in 5 hours');
		expect(formatResetIn(iso(45 * MIN + 30_000), now, 'en-US', soon)).toBe('in 45 minutes');
	});

	it('says in less than a minute under a minute', () => {
		expect(formatResetIn(iso(20_000), now, 'en-US', soon)).toBe(soon);
	});

	it('returns an empty string for a missing, unreadable or past time', () => {
		expect(formatResetIn(null, now, 'en-US', soon)).toBe('');
		expect(formatResetIn('not a date', now, 'en-US', soon)).toBe('');
		expect(formatResetIn(iso(-2 * 60 * MIN), now, 'en-US', soon)).toBe('');
	});
});

describe('formatCountdown', () => {
	it('renders minutes and zero-padded seconds', () => {
		expect(formatCountdown(151_000)).toBe('2:31');
		expect(formatCountdown(59_000)).toBe('0:59');
		expect(formatCountdown(180_000)).toBe('3:00');
	});

	it('rounds up so it never shows 0:00 while still waiting', () => {
		expect(formatCountdown(200)).toBe('0:01');
	});

	it('clamps negative waits to zero', () => {
		expect(formatCountdown(-5)).toBe('0:00');
	});
});

describe('formatAgo in Persian', () => {
	it('uses Persian digits', () => {
		const now = 1_781_524_800_000;
		expect(formatAgo(now - 5 * 60_000, now, intlLocale('fa'))).toBe('۵ دقیقه پیش');
	});
});

describe('formatCountdown in Persian', () => {
	it('uses Persian digits and keeps the zero padding', () => {
		expect(formatCountdown(151_000, intlLocale('fa'))).toBe('۲:۳۱');
		expect(formatCountdown(180_000, intlLocale('fa'))).toBe('۳:۰۰');
	});
});
