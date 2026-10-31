import { page, userEvent } from 'vitest/browser';
import { afterEach, describe, expect, it } from 'vitest';
import { render } from 'vitest-browser-svelte';
import '../../routes/layout.css';
import { mockTauri, type IpcHandlers } from '$lib/test/tauri-ipc';
import { active, switchLocale } from '$lib/locale-state.svelte';
import type { Settings } from '$lib/settings';

import SettingsScreen from './settings-screen.svelte';

let restore: () => void;
afterEach(() => {
	restore?.();
	switchLocale('en');
});

const defaults: Settings = {
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

function mount(stored: Settings = defaults, overrides: IpcHandlers = {}) {
	const tauri = mockTauri({
		get_settings: () => stored,
		update_settings: (args) => (args as { settings: Settings }).settings,
		...overrides
	});
	restore = tauri.restore;
	render(SettingsScreen);
	return tauri;
}

const updates = (calls: { cmd: string; args: unknown }[]) =>
	calls
		.filter((c) => c.cmd === 'update_settings')
		.map((c) => (c.args as { settings: Settings }).settings);

describe('SettingsScreen', () => {
	it('shows each switch in the state the core saved', async () => {
		mount({ ...defaults, launchAtBoot: true, startHidden: true, closeToTray: false });

		await expect.element(page.getByRole('switch', { name: 'Launch at boot' })).toBeChecked();
		await expect.element(page.getByRole('switch', { name: 'Start hidden' })).toBeChecked();
		await expect.element(page.getByRole('switch', { name: 'Close to tray' })).not.toBeChecked();
	});

	it('saves the whole settings when a switch is flipped', async () => {
		const tauri = mount();

		await page.getByRole('switch', { name: 'Launch at boot' }).click();

		await expect.element(page.getByRole('switch', { name: 'Launch at boot' })).toBeChecked();
		expect(updates(tauri.calls)).toEqual([{ ...defaults, launchAtBoot: true }]);

		await page.getByRole('switch', { name: 'Close to tray' }).click();
		expect(updates(tauri.calls).at(-1)).toEqual({
			...defaults,
			launchAtBoot: true,
			closeToTray: false
		});
	});

	it('puts a switch back and says so when the core refuses the change', async () => {
		mount(defaults, {
			update_settings: () => {
				throw { code: 'autostart-failed', message: 'registry is locked' };
			}
		});

		await page.getByRole('switch', { name: 'Launch at boot' }).click();

		await expect.element(page.getByRole('switch', { name: 'Launch at boot' })).not.toBeChecked();
		await expect.element(page.getByRole('alert')).toHaveTextContent(/starts with Windows/i);
	});

	it('shows the saved Claude.exe path', async () => {
		mount({ ...defaults, claudePath: 'C:\Apps\Claude.exe' });

		await expect
			.element(page.getByRole('textbox', { name: 'Claude.exe path' }))
			.toHaveValue('C:\Apps\Claude.exe');
	});

	it('saves a typed Claude.exe path', async () => {
		const tauri = mount();

		await page.getByRole('textbox', { name: 'Claude.exe path' }).fill('D:\Claude\Claude.exe');
		await page.getByRole('button', { name: 'Save path' }).click();

		expect(updates(tauri.calls)).toEqual([{ ...defaults, claudePath: 'D:\Claude\Claude.exe' }]);
	});

	it('rejects an invalid path with a clear message and keeps the old one', async () => {
		const tauri = mount(
			{ ...defaults, claudePath: 'C:\Apps\Claude.exe' },
			{
				update_settings: () => {
					throw { code: 'claude-path-not-claude', message: 'That file is not Claude.exe' };
				}
			}
		);

		const field = page.getByRole('textbox', { name: 'Claude.exe path' });
		await field.fill('C:\Windows\notepad.exe');
		await page.getByRole('button', { name: 'Save path' }).click();

		await expect.element(page.getByRole('alert')).toHaveTextContent(/isn't Claude\.exe/i);
		expect(updates(tauri.calls)).toHaveLength(1);
		// The next switch change still carries the path the core has saved.
		await page.getByRole('switch', { name: 'Close to tray' }).click();
		expect(updates(tauri.calls).at(-1)?.claudePath).toBe('C:\Apps\Claude.exe');
	});

	it('clears the override when the field is emptied and saved', async () => {
		const tauri = mount({ ...defaults, claudePath: 'C:\Apps\Claude.exe' });

		await page.getByRole('textbox', { name: 'Claude.exe path' }).fill('');
		await page.getByRole('button', { name: 'Save path' }).click();

		expect(updates(tauri.calls)).toEqual([{ ...defaults, claudePath: null }]);
	});

	it('offers English and Persian with the saved language picked', async () => {
		mount({ ...defaults, language: 'fa' });

		await page.getByRole('button', { name: 'Language' }).click();
		await expect.element(page.getByRole('menuitemradio', { name: 'فارسی' })).toBeChecked();
		await expect.element(page.getByRole('menuitemradio', { name: 'English' })).not.toBeChecked();
	});

	it('saves the chosen language and flips the document to right-to-left', async () => {
		const tauri = mount();

		await page.getByRole('button', { name: 'Language' }).click();
		await page.getByRole('menuitemradio', { name: 'فارسی' }).click();

		expect(updates(tauri.calls)).toEqual([{ ...defaults, language: 'fa' }]);
		expect(active.locale).toBe('fa');
		expect(document.documentElement.dir).toBe('rtl');
		expect(document.documentElement.lang).toBe('fa');
	});

	it('goes back to English and left-to-right', async () => {
		switchLocale('fa');
		const tauri = mount({ ...defaults, language: 'fa' });

		await page.getByRole('button', { name: 'زبان' }).click();
		await page.getByRole('menuitemradio', { name: 'English' }).click();

		await expect.poll(() => document.documentElement.dir).toBe('ltr');
		expect(updates(tauri.calls)).toEqual([{ ...defaults, language: 'en' }]);
		expect(active.locale).toBe('en');
	});

	it('keeps the language when the core refuses the change', async () => {
		mount(defaults, {
			update_settings: () => {
				throw { code: 'io', message: 'disk full' };
			}
		});

		await page.getByRole('button', { name: 'Language' }).click();
		await page.getByRole('menuitemradio', { name: 'فارسی' }).click();

		await expect.element(page.getByRole('alert')).toBeVisible();
		expect(active.locale).toBe('en');
		expect(document.documentElement.dir).toBe('ltr');
	});

	it('shows the Claude.exe path field left-to-right even in Persian', async () => {
		switchLocale('fa');
		mount({ ...defaults, language: 'fa' });

		const field = page.getByRole('textbox', { name: 'مسیر Claude.exe' });
		await expect.element(field).toHaveAttribute('dir', 'ltr');
	});

	it('shows the thresholds in Persian digits, with the slider running right-to-left', async () => {
		switchLocale('fa');
		mount({ ...defaults, language: 'fa', warningThreshold: 80, limitThreshold: 100 });

		await expect.element(page.getByText('۸۰٪', { exact: true })).toBeVisible();
		await expect.element(page.getByText('۱۰۰٪', { exact: true })).toBeVisible();
		const thumb = (await page.getByRole('slider').first().element()) as HTMLElement;
		expect(thumb.style.left).toBe('');
		expect(parseFloat(thumb.style.right)).toBeGreaterThan(70);
	});

	it('shows each notification switch in the state the core saved', async () => {
		mount({ ...defaults, notifyWarning: false, notifyReset: false });

		await expect.element(page.getByRole('switch', { name: 'Warning alert' })).not.toBeChecked();
		await expect.element(page.getByRole('switch', { name: 'Limit alert' })).toBeChecked();
		await expect.element(page.getByRole('switch', { name: 'Reset alert' })).not.toBeChecked();
	});

	it('saves the whole settings when a notification switch is flipped', async () => {
		const tauri = mount();

		await page.getByRole('switch', { name: 'Limit alert' }).click();

		await expect.element(page.getByRole('switch', { name: 'Limit alert' })).not.toBeChecked();
		expect(updates(tauri.calls)).toEqual([{ ...defaults, notifyLimited: false }]);
	});

	it('shows the saved thresholds', async () => {
		mount({ ...defaults, warningThreshold: 65, limitThreshold: 90 });

		await expect.element(page.getByText('65%', { exact: true })).toBeVisible();
		await expect.element(page.getByText('90%', { exact: true })).toBeVisible();
	});

	it('saves a warning threshold moved with the keyboard', async () => {
		const tauri = mount();

		await expect.element(page.getByRole('slider').first()).toBeVisible();
		((await page.getByRole('slider').first().element()) as HTMLElement).focus();
		await userEvent.keyboard('{ArrowRight}');

		await expect.poll(() => updates(tauri.calls)).toEqual([{ ...defaults, warningThreshold: 81 }]);
		await expect.element(page.getByText('81%', { exact: true })).toBeVisible();
	});

	it('says so and goes back to the saved value when the core refuses a threshold', async () => {
		const tauri = mount(defaults, {
			update_settings: () => {
				throw { code: 'invalid-thresholds', message: 'nope' };
			}
		});

		await expect.element(page.getByRole('slider').first()).toBeVisible();
		((await page.getByRole('slider').first().element()) as HTMLElement).focus();
		await userEvent.keyboard('{ArrowRight}'.repeat(20));

		await expect.element(page.getByRole('alert')).toHaveTextContent(/below the limit/i);
		await expect.element(page.getByText('80%', { exact: true })).toBeVisible();
		expect(updates(tauri.calls)).toHaveLength(20);
	});

	describe('sign-in routing', () => {
		const routingCalls = (calls: { cmd: string; args: unknown }[]) =>
			calls.filter((c) => c.cmd === 'set_sign_in_routing').map((c) => c.args);
		const routing = (on: boolean, needsDefaultAppsChoice = false) => ({
			set_sign_in_routing: () => ({
				settings: { ...defaults, signInRouting: on },
				needsDefaultAppsChoice
			})
		});

		it('shows the switch in the state the core saved', async () => {
			mount({ ...defaults, signInRouting: true });

			await expect.element(page.getByRole('switch', { name: 'Sign-in routing' })).toBeChecked();
		});

		it('turns routing on through its own command, not the general save', async () => {
			const tauri = mount(defaults, routing(true));

			await page.getByRole('switch', { name: 'Sign-in routing' }).click();

			await expect.element(page.getByRole('switch', { name: 'Sign-in routing' })).toBeChecked();
			expect(routingCalls(tauri.calls)).toEqual([{ enabled: true }]);
			expect(updates(tauri.calls)).toHaveLength(0);
		});

		it('turns routing off', async () => {
			const tauri = mount({ ...defaults, signInRouting: true }, routing(false));

			await page.getByRole('switch', { name: 'Sign-in routing' }).click();

			await expect.element(page.getByRole('switch', { name: 'Sign-in routing' })).not.toBeChecked();
			expect(routingCalls(tauri.calls)).toEqual([{ enabled: false }]);
		});

		it('guides the Default Apps choice Windows leaves', async () => {
			mount(defaults, routing(true, true));

			await page.getByRole('switch', { name: 'Sign-in routing' }).click();

			await expect.element(page.getByText("Sign-in links aren't reaching clawshell")).toBeVisible();
			await expect.element(page.getByText(/set the claude link type to clawshell/i)).toBeVisible();
		});

		it('says nothing about Default Apps when no choice is left', async () => {
			mount(defaults, routing(true, false));

			await page.getByRole('switch', { name: 'Sign-in routing' }).click();

			await expect.element(page.getByRole('switch', { name: 'Sign-in routing' })).toBeChecked();
			await expect
				.element(page.getByText("Sign-in links aren't reaching clawshell"))
				.not.toBeInTheDocument();
		});

		it('drops the Default Apps note when routing is turned off again', async () => {
			let on = true;
			mount(defaults, {
				set_sign_in_routing: () => {
					const change = {
						settings: { ...defaults, signInRouting: on },
						needsDefaultAppsChoice: on
					};
					on = !on;
					return change;
				}
			});

			await page.getByRole('switch', { name: 'Sign-in routing' }).click();
			await expect.element(page.getByText("Sign-in links aren't reaching clawshell")).toBeVisible();
			await page.getByRole('switch', { name: 'Sign-in routing' }).click();

			await expect
				.element(page.getByText("Sign-in links aren't reaching clawshell"))
				.not.toBeInTheDocument();
		});

		it('warns on opening when Windows sends links to another app', async () => {
			mount({ ...defaults, signInRouting: true }, { routing_needs_choice: () => true });

			await expect.element(page.getByText("Sign-in links aren't reaching clawshell")).toBeVisible();
		});

		it('opens Default Apps from the warning', async () => {
			const tauri = mount(
				{ ...defaults, signInRouting: true },
				{ routing_needs_choice: () => true, open_default_apps: () => null }
			);

			await page.getByRole('button', { name: 'Open Default Apps' }).click();

			expect(tauri.calls.filter((c) => c.cmd === 'open_default_apps')).toHaveLength(1);
		});

		it('says nothing when the links reach clawshell', async () => {
			mount({ ...defaults, signInRouting: true }, { routing_needs_choice: () => false });

			await expect.element(page.getByRole('switch', { name: 'Sign-in routing' })).toBeChecked();
			await expect
				.element(page.getByText("Sign-in links aren't reaching clawshell"))
				.not.toBeInTheDocument();
		});

		it('puts the switch back and says so when Windows refuses', async () => {
			mount(defaults, {
				set_sign_in_routing: () => {
					throw { code: 'routing-failed', message: 'access denied' };
				}
			});

			await page.getByRole('switch', { name: 'Sign-in routing' }).click();

			await expect.element(page.getByRole('alert')).toHaveTextContent(/sign-in routing/i);
			await expect.element(page.getByRole('switch', { name: 'Sign-in routing' })).not.toBeChecked();
		});

		it('keeps the new routing state in later general saves', async () => {
			const tauri = mount(defaults, routing(true));

			await page.getByRole('switch', { name: 'Sign-in routing' }).click();
			await page.getByRole('switch', { name: 'Close to tray' }).click();

			expect(updates(tauri.calls).at(-1)).toEqual({
				...defaults,
				signInRouting: true,
				closeToTray: false
			});
		});
	});
});
