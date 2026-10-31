import { page } from 'vitest/browser';
import { afterEach, describe, expect, it, vi } from 'vitest';
import { render } from 'vitest-browser-svelte';
import { emit } from '@tauri-apps/api/event';
import '../../routes/layout.css';
import { switchLocale } from '$lib/locale-state.svelte';
import type { Settings } from '$lib/settings';
import { mockTauri } from '$lib/test/tauri-ipc';
import { UPDATE_CHECK_MS, updateState } from '$lib/update.svelte';
import AppShell from './app-shell.svelte';

let restore: () => void;

// The Accounts tab asks the core for its list as soon as it mounts.
const core = { list_accounts: () => ({ claudeFound: true, accounts: [] }) };
afterEach(() => {
	restore?.();
	switchLocale('en');
	updateState.current = { status: 'idle' };
});

const saved: Settings = {
	launchAtBoot: false,
	startHidden: false,
	closeToTray: true,
	language: 'en',
	claudePath: null,
	notifyWarning: true,
	notifyLimited: true,
	notifyReset: true,
	warningThreshold: 80,
	limitThreshold: 100,
	signInRouting: false
};
const leftOf = (locator: { element(): Element }) => locator.element().getBoundingClientRect().left;

describe('AppShell', () => {
	it('shows the Usage, Accounts and Settings tabs and switches screens', async () => {
		({ restore } = mockTauri(core));
		render(AppShell);

		const usage = page.getByRole('tab', { name: 'Usage' });
		const accounts = page.getByRole('tab', { name: 'Accounts' });
		const settings = page.getByRole('tab', { name: 'Settings' });

		await expect.element(accounts).toHaveAttribute('aria-selected', 'true');
		await expect.element(page.getByRole('tabpanel', { name: 'Accounts' })).toBeVisible();

		await usage.click();
		await expect.element(usage).toHaveAttribute('aria-selected', 'true');
		await expect.element(page.getByRole('tabpanel', { name: 'Usage' })).toBeVisible();

		await settings.click();
		await expect.element(settings).toHaveAttribute('aria-selected', 'true');
		await expect.element(page.getByRole('tabpanel', { name: 'Settings' })).toBeVisible();
	});

	it('shows the app name and minimize and close buttons, but no maximize', async () => {
		({ restore } = mockTauri(core));
		render(AppShell);

		await expect.element(page.getByText('clawshell', { exact: true })).toBeVisible();
		await expect.element(page.getByRole('button', { name: 'Minimize' })).toBeVisible();
		await expect.element(page.getByRole('button', { name: 'Close' })).toBeVisible();
		await expect
			.element(page.getByRole('button', { name: /maximi[sz]e/i }))
			.not.toBeInTheDocument();
	});

	it('minimizes and closes the window through the top bar', async () => {
		const tauri = mockTauri(core);
		({ restore } = tauri);
		render(AppShell);

		await page.getByRole('button', { name: 'Minimize' }).click();
		await page.getByRole('button', { name: 'Close' }).click();

		const commands = tauri.calls.map((c) => c.cmd);
		expect(commands).toContain('plugin:window|minimize');
		expect(commands).toContain('plugin:window|close');
	});
});

describe('AppShell update badge', () => {
	const release = { rid: 1, currentVersion: '0.1.0', version: '0.2.0', body: 'Notes', rawJson: {} };
	const badgeName = 'Update available';

	it('shows no badge when the app is on the latest version', async () => {
		const tauri = mockTauri({ ...core, 'plugin:updater|check': () => null });
		({ restore } = tauri);
		render(AppShell);
		await expect.element(page.getByRole('tab', { name: 'Usage' })).toBeVisible();

		await vi.waitFor(() => expect(tauri.calls.map((c) => c.cmd)).toContain('plugin:updater|check'));
		await expect.element(page.getByRole('button', { name: badgeName })).not.toBeInTheDocument();
	});

	it('shows no badge and no error when the check fails', async () => {
		const tauri = mockTauri({
			...core,
			'plugin:updater|check': () => {
				throw new Error('offline');
			}
		});
		({ restore } = tauri);
		render(AppShell);
		await expect.element(page.getByRole('tab', { name: 'Usage' })).toBeVisible();

		await vi.waitFor(() => expect(tauri.calls.map((c) => c.cmd)).toContain('plugin:updater|check'));
		await expect.element(page.getByRole('button', { name: badgeName })).not.toBeInTheDocument();
	});

	it('shows the badge after the app name when a newer release exists', async () => {
		({ restore } = mockTauri({ ...core, 'plugin:updater|check': () => release }));
		render(AppShell);

		const badge = page.getByRole('button', { name: badgeName });
		await expect.element(badge).toBeVisible();
		expect(leftOf(badge)).toBeGreaterThan(leftOf(page.getByText('clawshell', { exact: true })));
	});

	it('stops checking for updates once the shell is gone', async () => {
		vi.useFakeTimers({ toFake: ['setInterval', 'clearInterval'] });
		try {
			const tauri = mockTauri({ ...core, 'plugin:updater|check': () => null });
			({ restore } = tauri);
			const checks = () => tauri.calls.filter((c) => c.cmd === 'plugin:updater|check').length;
			const screen = render(AppShell);
			await vi.waitFor(() => expect(checks()).toBe(1));

			screen.unmount();
			await vi.advanceTimersByTimeAsync(UPDATE_CHECK_MS * 2);

			expect(checks()).toBe(1);
		} finally {
			vi.useRealTimers();
		}
	});

	it('names the badge in Persian when the saved language is Persian', async () => {
		({ restore } = mockTauri({
			...core,
			get_settings: () => ({ ...saved, language: 'fa' }),
			'plugin:updater|check': () => release
		}));
		render(AppShell);

		await expect
			.element(page.getByRole('button', { name: 'به‌روزرسانی در دسترس است' }))
			.toBeVisible();
	});
});

describe('AppShell sign-in routing offer', () => {
	it('offers routing on any tab when the core announces the first Add', async () => {
		({ restore } = mockTauri(core));
		render(AppShell);
		await expect.element(page.getByRole('tab', { name: 'Usage' })).toBeVisible();

		await emit('routing-offer');

		await expect
			.element(page.getByText('Route browser sign-ins to the right account?'))
			.toBeVisible();
	});
});

describe('AppShell tray Add account', () => {
	it('switches to the Accounts tab and opens the Add dialog when the tray asks', async () => {
		({ restore } = mockTauri({
			...core,
			new_account_defaults: () => ({ label: 'Account 2', colour: '#2563eb' })
		}));
		render(AppShell);
		await page.getByRole('tab', { name: 'Usage' }).click();
		await expect.element(page.getByRole('tab', { name: 'Usage' })).toHaveAttribute('aria-selected', 'true');

		await emit('add-account-requested');

		await expect.element(page.getByRole('tab', { name: 'Accounts' })).toHaveAttribute('aria-selected', 'true');
		await expect.element(page.getByRole('dialog', { name: 'Add account' })).toBeVisible();
	});
});

describe('AppShell in Persian', () => {
	it('opens in the language the core saved, right-to-left', async () => {
		({ restore } = mockTauri({ ...core, get_settings: () => ({ ...saved, language: 'fa' }) }));
		render(AppShell);

		await expect.element(page.getByRole('tab', { name: 'تنظیمات' })).toBeVisible();
		await expect.element(page.getByRole('tab', { name: 'مصرف' })).toBeVisible();
		expect(document.documentElement.dir).toBe('rtl');
		expect(document.documentElement.lang).toBe('fa');
	});

	it('opens in English, left-to-right, when the settings cannot be read', async () => {
		document.documentElement.dir = '%paraglide.dir%';
		({ restore } = mockTauri({
			...core,
			get_settings: () => {
				throw { code: 'io', message: 'unreadable' };
			}
		}));
		render(AppShell);

		await expect.element(page.getByRole('tab', { name: 'Settings' })).toBeVisible();
		expect(document.documentElement.dir).toBe('ltr');
		expect(document.documentElement.lang).toBe('en');
	});

	it('mirrors the top bar and the tab bar', async () => {
		({ restore } = mockTauri({ ...core, get_settings: () => ({ ...saved, language: 'fa' }) }));
		render(AppShell);

		const usage = page.getByRole('tab', { name: 'مصرف' });
		const accounts = page.getByRole('tab', { name: 'حساب‌ها' });
		const settings = page.getByRole('tab', { name: 'تنظیمات' });
		await expect.element(usage).toBeVisible();
		// The first tab sits on the right and the last on the left.
		expect(leftOf(usage)).toBeGreaterThan(leftOf(accounts));
		expect(leftOf(accounts)).toBeGreaterThan(leftOf(settings));
		// The name starts on the right and the window buttons end on the left.
		expect(leftOf(page.getByText('clawshell', { exact: true }))).toBeGreaterThan(
			leftOf(page.getByRole('button', { name: 'کوچک کردن' }))
		);
	});

	it('switches the whole window to Persian from Settings and stays on Settings', async () => {
		const tauri = mockTauri({
			...core,
			get_settings: () => saved,
			update_settings: (args) => (args as { settings: Settings }).settings
		});
		({ restore } = tauri);
		render(AppShell);

		await page.getByRole('tab', { name: 'Settings' }).click();
		await page.getByRole('button', { name: 'Language' }).click();
		await page.getByRole('menuitemradio', { name: 'فارسی' }).click();

		const settings = page.getByRole('tab', { name: 'تنظیمات' });
		await expect.element(settings).toHaveAttribute('aria-selected', 'true');
		await expect.element(page.getByRole('tab', { name: 'Usage' })).not.toBeInTheDocument();
		await expect.element(page.getByRole('button', { name: 'بستن' })).toBeVisible();
		await expect.element(page.getByText('زبان')).toBeVisible();
		expect(document.documentElement.dir).toBe('rtl');
	});

	it('goes back to English and left-to-right', async () => {
		({ restore } = mockTauri({
			...core,
			get_settings: () => ({ ...saved, language: 'fa' }),
			update_settings: (args) => (args as { settings: Settings }).settings
		}));
		render(AppShell);

		await page.getByRole('tab', { name: 'تنظیمات' }).click();
		await page.getByRole('button', { name: 'زبان' }).click();
		await page.getByRole('menuitemradio', { name: 'English' }).click();

		await expect.element(page.getByRole('tab', { name: 'Settings' })).toBeVisible();
		expect(document.documentElement.dir).toBe('ltr');
	});
});
