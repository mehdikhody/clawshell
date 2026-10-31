import { describe, expect, it } from 'vitest';
import { formatAgo, formatCountdown, formatReset } from './usage';

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
