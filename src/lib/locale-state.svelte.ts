import { setLocale } from '$lib/paraglide/runtime.js';
import { directionOf, intlLocale, type CalendarSetting, type Locale } from '$lib/locale';

/** The language and calendar on screen. Screens re-render on a change by keying on both. */
export const active = $state<{ locale: Locale; calendar: CalendarSetting }>({
	locale: 'en',
	calendar: 'automatic'
});

/** Switches every string, the document direction and `active` in one step, without a reload. */
export function switchLocale(locale: Locale) {
	setLocale(locale, { reload: false });
	document.documentElement.lang = locale;
	document.documentElement.dir = directionOf(locale);
	active.locale = locale;
}

/** Switches the calendar on screen. A pinned one stays when the language changes. */
export function switchCalendar(calendar: CalendarSetting) {
	active.calendar = calendar;
}

/** The tag for `Intl` in the language and calendar on screen. */
export const formatTag = () => intlLocale(active.locale, active.calendar);
