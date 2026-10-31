import { describe, expect, it } from 'vitest';
import { directionOf, formatInteger, formatPercentValue, intlLocale, isLocale } from './locale';

describe('isLocale', () => {
	it('accepts the two supported locales and nothing else', () => {
		expect(isLocale('en')).toBe(true);
		expect(isLocale('fa')).toBe(true);
		expect(isLocale('de')).toBe(false);
		expect(isLocale(null)).toBe(false);
	});
});

describe('directionOf', () => {
	it('is right-to-left for Persian and left-to-right for English', () => {
		expect(directionOf('fa')).toBe('rtl');
		expect(directionOf('en')).toBe('ltr');
	});
});

describe('formatInteger', () => {
	it('keeps Latin digits in English', () => {
		expect(formatInteger(1234, intlLocale('en'))).toBe('1,234');
	});

	it('uses Persian digits in Persian', () => {
		expect(formatInteger(30, intlLocale('fa'))).toBe('۳۰');
		expect(formatInteger(1234, intlLocale('fa'))).toBe('۱٬۲۳۴');
	});
});

describe('formatPercentValue', () => {
	it('writes a whole percent from 0 to 100', () => {
		expect(formatPercentValue(79.6, intlLocale('en'))).toBe('80%');
		expect(formatPercentValue(34.6, 'en-US')).toBe('35%');
		expect(formatPercentValue(0, 'en-US')).toBe('0%');
	});

	it('writes a 0-to-1 share once it is scaled to a percent', () => {
		expect(formatPercentValue(0.8182 * 100, 'en-US')).toBe('82%');
		expect(formatPercentValue(1 * 100, 'en-US')).toBe('100%');
		expect(formatPercentValue(0.8182 * 100, intlLocale('fa'))).toBe('۸۲٪');
	});

	it('uses Persian digits and the Persian percent sign', () => {
		expect(formatPercentValue(79.6, intlLocale('fa'))).toBe('۸۰٪');
	});
});
