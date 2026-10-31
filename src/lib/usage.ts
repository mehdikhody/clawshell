/** Display helpers for usage numbers: pure, so they are easy to test. */

const DAY_MS = 86_400_000;

const sameDay = (a: Date, b: Date) =>
	a.getFullYear() === b.getFullYear() &&
	a.getMonth() === b.getMonth() &&
	a.getDate() === b.getDate();

/**
 * A window's reset time: the clock time when it is later today, the weekday
 * and time within the next week, and the date and time beyond that.
 * Empty when the time is missing or unreadable.
 */
export function formatReset(iso: string | null, now: Date, locale: string): string {
	if (!iso) return '';
	const at = new Date(iso);
	if (Number.isNaN(at.getTime())) return '';
	const time: Intl.DateTimeFormatOptions = { hour: 'numeric', minute: '2-digit' };
	const options: Intl.DateTimeFormatOptions = sameDay(at, now)
		? time
		: at.getTime() - now.getTime() < 7 * DAY_MS
			? { weekday: 'short', ...time }
			: { month: 'short', day: 'numeric', ...time };
	return new Intl.DateTimeFormat(locale, options).format(at);
}

/** "5 minutes ago", or "now" under a minute (or if the clock says future). */
export function formatAgo(thenMs: number, nowMs: number, locale: string): string {
	const rtf = new Intl.RelativeTimeFormat(locale, { numeric: 'auto' });
	const seconds = Math.max(0, Math.round((nowMs - thenMs) / 1000));
	if (seconds < 60) return rtf.format(0, 'second');
	const minutes = Math.floor(seconds / 60);
	if (minutes < 60) return rtf.format(-minutes, 'minute');
	const hours = Math.floor(minutes / 60);
	if (hours < 24) return rtf.format(-hours, 'hour');
	return rtf.format(-Math.floor(hours / 24), 'day');
}

/** A wait as m:ss, rounded up so a pending wait never reads 0:00. */
export function formatCountdown(ms: number): string {
	const total = Math.max(0, Math.ceil(ms / 1000));
	const seconds = String(total % 60).padStart(2, '0');
	return `${Math.floor(total / 60)}:${seconds}`;
}
