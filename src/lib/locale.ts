/** Locale rules that need no browser: which locales exist, their direction and number formats. */

export const locales = ['en', 'fa', 'ar', 'ru', 'zh'] as const;
export type Locale = (typeof locales)[number];

export const isLocale = (value: unknown): value is Locale =>
	typeof value === 'string' && (locales as readonly string[]).includes(value);

export const directionOf = (locale: Locale): 'ltr' | 'rtl' =>
	locale === 'fa' || locale === 'ar' ? 'rtl' : 'ltr';

/** The day a week starts on, counted as `Date.getDay` counts: Saturday (6) in Persian, Sunday (0) otherwise. */
export const weekStartOf = (locale: Locale): number => (locale === 'fa' ? 6 : 0);

/**
 * The tag to hand to `Intl`. Persian uses the Jalali (Persian) calendar and Persian
 * digits; the digits and calendar are named so no engine default can change them.
 * Chinese is Simplified, so its tag is `zh-CN`.
 */
export const intlLocale = (locale: Locale): string =>
	locale === 'fa' ? 'fa-u-ca-persian-nu-arabext' : locale === 'zh' ? 'zh-CN' : locale;

/** A whole number with grouping, in the digits of the locale. */
export const formatInteger = (value: number, intl: string): string =>
	new Intl.NumberFormat(intl, { maximumFractionDigits: 0 }).format(value);

/** A percent from 0 to 100, rounded to a whole number. */
export const formatPercentValue = (percent: number, intl: string): string =>
	new Intl.NumberFormat(intl, { style: 'percent', maximumFractionDigits: 0 }).format(percent / 100);
