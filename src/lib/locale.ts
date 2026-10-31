/** Locale rules that need no browser: which locales exist, their direction and number formats. */

export const locales = ['en', 'fa'] as const;
export type Locale = (typeof locales)[number];

export const isLocale = (value: unknown): value is Locale =>
	typeof value === 'string' && (locales as readonly string[]).includes(value);

export const directionOf = (locale: Locale): 'ltr' | 'rtl' => (locale === 'fa' ? 'rtl' : 'ltr');

/**
 * The tag to hand to `Intl`. Persian keeps Gregorian dates, because the spec asks
 * only for Persian digits, and names the digits so no engine default can change them.
 */
export const intlLocale = (locale: Locale): string =>
	locale === 'fa' ? 'fa-u-ca-gregory-nu-arabext' : 'en';

/** A whole number with grouping, in the digits of the locale. */
export const formatInteger = (value: number, intl: string): string =>
	new Intl.NumberFormat(intl, { maximumFractionDigits: 0 }).format(value);

/** A percent from 0 to 100, rounded to a whole number. */
export const formatPercentValue = (percent: number, intl: string): string =>
	new Intl.NumberFormat(intl, { style: 'percent', maximumFractionDigits: 0 }).format(percent / 100);
