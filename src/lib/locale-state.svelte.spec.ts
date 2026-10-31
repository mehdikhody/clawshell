import { afterEach, describe, expect, it } from 'vitest';
import { getLocale } from '$lib/paraglide/runtime.js';
import { m } from '$lib/paraglide/messages.js';
import { active, formatTag, switchLocale } from './locale-state.svelte';

afterEach(() => switchLocale('en'));

describe('switchLocale', () => {
	it('flips the document to right-to-left Persian and back', () => {
		switchLocale('fa');
		expect(document.documentElement.dir).toBe('rtl');
		expect(document.documentElement.lang).toBe('fa');
		expect(getLocale()).toBe('fa');
		expect(active.locale).toBe('fa');
		expect(m.tab_settings()).toBe('تنظیمات');

		switchLocale('en');
		expect(document.documentElement.dir).toBe('ltr');
		expect(document.documentElement.lang).toBe('en');
		expect(m.tab_settings()).toBe('Settings');
	});

	it('hands Intl a tag that gives Persian digits and Jalali dates', () => {
		switchLocale('fa');
		const tag = formatTag();
		expect(new Intl.NumberFormat(tag).format(15)).toBe('۱۵');
		expect(new Intl.DateTimeFormat(tag, { year: 'numeric' }).format(new Date(2026, 5, 15))).toBe(
			'۱۴۰۵'
		);
	});
});
