import { describe, expect, it } from 'vitest';
import { intlLocale } from './locale';
import { formatUsd } from './cost';

describe('formatUsd', () => {
	it('writes dollars with cents', () => {
		expect(formatUsd(14.5, 'en-US')).toBe('$14.50');
		expect(formatUsd(0, 'en-US')).toBe('$0.00');
	});

	it('groups thousands', () => {
		expect(formatUsd(1234.567, 'en-US')).toBe('$1,234.57');
	});
});

describe('Persian money', () => {
	const fa = intlLocale('fa');

	it('writes dollars in Persian digits', () => {
		expect(formatUsd(1234.5, fa)).toBe('‎$۱٬۲۳۴٫۵۰');
	});
});
