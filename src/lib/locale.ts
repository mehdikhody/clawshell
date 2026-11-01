/** Locale rules that need no browser: which locales exist, their direction and number formats. */

export const locales = ['en', 'fa', 'ar', 'ru', 'zh'] as const;
export type Locale = (typeof locales)[number];

export const isLocale = (value: unknown): value is Locale =>
	typeof value === 'string' && (locales as readonly string[]).includes(value);

export const directionOf = (locale: Locale): 'ltr' | 'rtl' =>
	locale === 'fa' || locale === 'ar' ? 'rtl' : 'ltr';

/** The calendar setting: Automatic follows the language, the rest pin an `Intl` calendar id. */
export const calendars = [
	'automatic',
	'gregory',
	'persian',
	'islamic-umalqura',
	'chinese'
] as const;
export type CalendarSetting = (typeof calendars)[number];
/** A calendar Automatic has been resolved to. */
export type Calendar = Exclude<CalendarSetting, 'automatic'>;
/** The `Intl` numbering systems the calendars use. */
export type Digits = 'latn' | 'arab' | 'arabext';

export const isCalendar = (value: unknown): value is CalendarSetting =>
	typeof value === 'string' && (calendars as readonly string[]).includes(value);

/** The calendar in use: Automatic gives Jalali for Persian, Hijri for Arabic and Gregorian otherwise; a pinned one stays. */
export const resolveCalendar = (locale: Locale, calendar: CalendarSetting): Calendar =>
	calendar !== 'automatic'
		? calendar
		: locale === 'fa'
			? 'persian'
			: locale === 'ar'
				? 'islamic-umalqura'
				: 'gregory';

/** Persian digits for Persian, Arabic-Indic for Arabic and 0-9 otherwise. The calendar never changes them. */
export const digitsOf = (locale: Locale): Digits =>
	locale === 'fa' ? 'arabext' : locale === 'ar' ? 'arab' : 'latn';

/**
 * The day a week starts on, counted as `Date.getDay` counts. Jalali and Hijri start on
 * Saturday (6) and Chinese lunar on Monday (1); Gregorian takes it from the language.
 */
export const weekStartOf = (locale: Locale, calendar: CalendarSetting = 'automatic'): number => {
	switch (resolveCalendar(locale, calendar)) {
		case 'persian':
		case 'islamic-umalqura':
			return 6;
		case 'chinese':
			return 1;
		default:
			return locale === 'fa' || locale === 'ar' ? 6 : locale === 'ru' || locale === 'zh' ? 1 : 0;
	}
};

/**
 * The tag to hand to `Intl`: the language, then the resolved calendar and the language's digits,
 * both named so no engine default can change them. Chinese is Simplified, so its tag is `zh-CN`.
 */
export const intlLocale = (locale: Locale, calendar: CalendarSetting = 'automatic'): string => {
	const resolved = resolveCalendar(locale, calendar);
	return `${locale === 'zh' ? 'zh-CN' : locale}-u-ca-${resolved}-nu-${digitsOf(locale)}`;
};

/** A whole number with grouping, in the digits of the locale. */
export const formatInteger = (value: number, intl: string): string =>
	new Intl.NumberFormat(intl, { maximumFractionDigits: 0 }).format(value);

/** A percent from 0 to 100, rounded to a whole number. */
export const formatPercentValue = (percent: number, intl: string): string =>
	new Intl.NumberFormat(intl, { style: 'percent', maximumFractionDigits: 0 }).format(percent / 100);
