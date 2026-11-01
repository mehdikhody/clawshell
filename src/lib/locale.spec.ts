import { describe, expect, it } from 'vitest';
import {
	calendars,
	digitsOf,
	directionOf,
	formatInteger,
	formatPercentValue,
	intlLocale,
	isCalendar,
	isLocale,
	locales,
	resolveCalendar,
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

describe('isCalendar', () => {
	it('lists Automatic and the four Intl calendars, and accepts nothing else', () => {
		expect([...calendars]).toEqual([
			'automatic',
			'gregory',
			'persian',
			'islamic-umalqura',
			'chinese'
		]);
		for (const calendar of calendars) expect(isCalendar(calendar)).toBe(true);
		expect(isCalendar('hebrew')).toBe(false);
		expect(isCalendar(undefined)).toBe(false);
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

describe('resolveCalendar', () => {
	it('lets Automatic follow the language: Jalali for Persian, Hijri for Arabic, else Gregorian', () => {
		expect(resolveCalendar('fa', 'automatic')).toBe('persian');
		expect(resolveCalendar('ar', 'automatic')).toBe('islamic-umalqura');
		for (const language of ['en', 'ru', 'zh'] as const) {
			expect(resolveCalendar(language, 'automatic')).toBe('gregory');
		}
	});

	it('keeps a pinned calendar whatever the language', () => {
		for (const language of locales) {
			for (const calendar of ['gregory', 'persian', 'islamic-umalqura', 'chinese'] as const) {
				expect(resolveCalendar(language, calendar)).toBe(calendar);
			}
		}
	});
});

describe('digitsOf', () => {
	it('is Persian digits for Jalali, Arabic-Indic for Hijri and 0-9 otherwise', () => {
		expect(digitsOf('persian')).toBe('arabext');
		expect(digitsOf('islamic-umalqura')).toBe('arab');
		expect(digitsOf('gregory')).toBe('latn');
		expect(digitsOf('chinese')).toBe('latn');
	});
});

describe('weekStartOf', () => {
	it('starts Jalali and Hijri weeks on Saturday and lunar weeks on Monday, in any language', () => {
		for (const language of locales) {
			expect(weekStartOf(language, 'persian')).toBe(6);
			expect(weekStartOf(language, 'islamic-umalqura')).toBe(6);
			expect(weekStartOf(language, 'chinese')).toBe(1);
		}
	});

	it('takes the Gregorian week start from the language', () => {
		expect(weekStartOf('en', 'gregory')).toBe(0);
		expect(weekStartOf('ru', 'gregory')).toBe(1);
		expect(weekStartOf('zh', 'gregory')).toBe(1);
		expect(weekStartOf('fa', 'gregory')).toBe(6);
		expect(weekStartOf('ar', 'gregory')).toBe(6);
	});

	it('resolves Automatic first', () => {
		expect(weekStartOf('fa', 'automatic')).toBe(6);
		expect(weekStartOf('ar', 'automatic')).toBe(6);
		expect(weekStartOf('en', 'automatic')).toBe(0);
		expect(weekStartOf('ru', 'automatic')).toBe(1);
		expect(weekStartOf('zh', 'automatic')).toBe(1);
	});
});

describe('intlLocale', () => {
	it('gives Persian with Automatic the tag it always had', () => {
		expect(intlLocale('fa', 'automatic')).toBe('fa-u-ca-persian-nu-arabext');
		expect(intlLocale('fa')).toBe('fa-u-ca-persian-nu-arabext');
	});

	it('names the language tag, the resolved calendar and its digits for every pair', () => {
		const tags = { en: 'en', fa: 'fa', ar: 'ar', ru: 'ru', zh: 'zh-CN' };
		const expected = {
			automatic: {
				en: 'gregory-nu-latn',
				fa: 'persian-nu-arabext',
				ar: 'islamic-umalqura-nu-arab',
				ru: 'gregory-nu-latn',
				zh: 'gregory-nu-latn'
			},
			gregory: 'gregory-nu-latn',
			persian: 'persian-nu-arabext',
			'islamic-umalqura': 'islamic-umalqura-nu-arab',
			chinese: 'chinese-nu-latn'
		};
		for (const language of locales) {
			for (const calendar of calendars) {
				const suffix = calendar === 'automatic' ? expected.automatic[language] : expected[calendar];
				expect(intlLocale(language, calendar)).toBe(`${tags[language]}-u-ca-${suffix}`);
			}
		}
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

	it('uses Arabic-Indic digits under the Hijri calendar, even in English', () => {
		expect(formatInteger(30, intlLocale('en', 'islamic-umalqura'))).toBe('٣٠');
	});

	it('uses 0-9 in Persian once the user pins Gregorian', () => {
		expect(formatInteger(30, intlLocale('fa', 'gregory'))).toBe('30');
	});

	it('writes a Jalali year, in Persian digits, for an English reader who pinned Jalali', () => {
		const year = new Intl.DateTimeFormat(intlLocale('en', 'persian'), { year: 'numeric' }).format(
			new Date(2026, 5, 15)
		);
		expect(year).toContain('۱۴۰۵');
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
