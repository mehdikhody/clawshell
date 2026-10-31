import { describe, expect, it } from 'vitest';
import {
	directionOf,
	formatInteger,
	formatPercentValue,
	intlLocale,
	isLocale,
	locales,
	weekStartOf
} from './locale';

describe('isLocale', () => {
	it('lists the five supported locales', () => {
		expect([...locales]).toEqual(['en', 'fa', 'ar', 'ru', 'zh']);
	});

	it('accepts the supported locales and nothing else', () => {
		for (const locale of ['en', 'fa', 'ar', 'ru', 'zh']) expect(isLocale(locale)).toBe(true);
		expect(isLocale('de')).toBe(false);
		expect(isLocale(null)).toBe(false);
	});
});

describe('directionOf', () => {
	it('is right-to-left for Persian and Arabic and left-to-right otherwise', () => {
		expect(directionOf('fa')).toBe('rtl');
		expect(directionOf('ar')).toBe('rtl');
		expect(directionOf('en')).toBe('ltr');
		expect(directionOf('ru')).toBe('ltr');
		expect(directionOf('zh')).toBe('ltr');
	});
});

describe('formatInteger', () => {
	it('keeps Latin digits in English', () => {
		expect(formatInteger(1234, intlLocale('en'))).toBe('1,234');
	});

	it('starts the week on Saturday in Persian and on Sunday in English', () => {
		expect(weekStartOf('fa')).toBe(6);
		expect(weekStartOf('en')).toBe(0);
	});

	it('uses the Jalali calendar in Persian', () => {
		expect(intlLocale('fa')).toContain('-u-ca-persian');
		expect(intlLocale('en')).not.toContain('-u-ca-');
	});

	it('hands Intl the language tag, with zh-CN for Chinese', () => {
		expect(intlLocale('ar')).toMatch(/^ar(-|$)/);
		expect(intlLocale('ru')).toMatch(/^ru(-|$)/);
		expect(intlLocale('zh')).toMatch(/^zh-CN(-|$)/);
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
