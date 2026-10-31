import { setLocale } from '$lib/paraglide/runtime.js';
import { directionOf, intlLocale, type Locale } from '$lib/locale';

/** The language on screen. Screens re-render on a change by keying on `active.locale`. */
export const active = $state<{ locale: Locale }>({ locale: 'en' });

/** Switches every string, the document direction and `active` in one step, without a reload. */
export function switchLocale(locale: Locale) {
	setLocale(locale, { reload: false });
	document.documentElement.lang = locale;
	document.documentElement.dir = directionOf(locale);
	active.locale = locale;
}

/** The tag for `Intl` in the language on screen. */
export const formatTag = () => intlLocale(active.locale);
