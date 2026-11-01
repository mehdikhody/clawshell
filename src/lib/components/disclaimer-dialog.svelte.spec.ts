import { page, userEvent } from 'vitest/browser';
import { afterEach, describe, expect, it } from 'vitest';
import { render } from 'vitest-browser-svelte';
import '../../routes/layout.css';
import { disclaimer } from '$lib/disclaimer.svelte';
import { switchLocale } from '$lib/locale-state.svelte';
import { mockTauri } from '$lib/test/tauri-ipc';
import DisclaimerDialog from './disclaimer-dialog.svelte';

let restore: () => void;
afterEach(() => {
	restore?.();
	switchLocale('en');
	disclaimer.accepted = null;
});

function mount(accepted: boolean | null) {
	const tauri = mockTauri({ accept_disclaimer: () => null, decline_disclaimer: () => null });
	restore = tauri.restore;
	disclaimer.accepted = accepted;
	render(DisclaimerDialog);
	return tauri;
}

const dialog = () => page.getByRole('alertdialog');
const callsTo = (calls: { cmd: string }[], cmd: string) => calls.filter((c) => c.cmd === cmd);

describe('DisclaimerDialog', () => {
	it('shows the notice, with Accept and Exit and no close button, while unaccepted', async () => {
		mount(false);

		await expect.element(dialog()).toBeVisible();
		await expect.element(page.getByRole('heading', { name: 'Before you start' })).toBeVisible();
		await expect.element(dialog().getByText(/unofficial tool/)).toBeVisible();
		await expect.element(page.getByRole('button', { name: 'Accept' })).toBeVisible();
		await expect.element(page.getByRole('button', { name: 'Exit' })).toBeVisible();
		await expect.element(page.getByRole('button', { name: 'Close' })).not.toBeInTheDocument();
	});

	it('is absent once accepted', async () => {
		mount(true);

		await expect.element(dialog()).not.toBeInTheDocument();
	});

	it('is absent until the core has answered', async () => {
		mount(null);

		await expect.element(dialog()).not.toBeInTheDocument();
	});

	it('stays open on Esc', async () => {
		const tauri = mount(false);
		await expect.element(dialog()).toBeVisible();

		await userEvent.keyboard('{Escape}');

		await expect.element(dialog()).toBeVisible();
		expect(tauri.calls.map((c) => c.cmd)).toEqual([]);
	});

	it('stays open when the user clicks outside it', async () => {
		const tauri = mount(false);
		await expect.element(dialog()).toBeVisible();

		await page.elementLocator(document.body).click({ position: { x: 4, y: 4 } });

		await expect.element(dialog()).toBeVisible();
		expect(tauri.calls.map((c) => c.cmd)).toEqual([]);
	});

	it('accepts through the core and closes', async () => {
		const tauri = mount(false);

		await page.getByRole('button', { name: 'Accept' }).click();

		await expect.element(dialog()).not.toBeInTheDocument();
		expect(callsTo(tauri.calls, 'accept_disclaimer')).toHaveLength(1);
		expect(callsTo(tauri.calls, 'decline_disclaimer')).toHaveLength(0);
		expect(disclaimer.accepted).toBe(true);
	});

	it('asks the core to exit on Exit', async () => {
		const tauri = mount(false);

		await page.getByRole('button', { name: 'Exit' }).click();

		expect(callsTo(tauri.calls, 'decline_disclaimer')).toHaveLength(1);
		expect(callsTo(tauri.calls, 'accept_disclaimer')).toHaveLength(0);
	});

	it('stays open when the core cannot save the acceptance', async () => {
		const tauri = mockTauri({
			accept_disclaimer: () => {
				throw { code: 'io', message: 'disk full' };
			}
		});
		restore = tauri.restore;
		disclaimer.accepted = false;
		render(DisclaimerDialog);

		await page.getByRole('button', { name: 'Accept' }).click();

		await expect.element(dialog()).toBeVisible();
		expect(disclaimer.accepted).toBe(false);
	});

	it('is written in the current language', async () => {
		switchLocale('ru');
		mount(false);

		await expect.element(dialog()).toBeVisible();
		await expect.element(page.getByRole('button', { name: 'Принять' })).toBeVisible();
		await expect.element(page.getByRole('button', { name: 'Выйти' })).toBeVisible();
	});

	it.each(['fa', 'ar'] as const)('renders in %s, right to left', async (locale) => {
		switchLocale(locale);
		mount(false);

		await expect.element(dialog()).toBeVisible();
		expect(getComputedStyle(dialog().element()).direction).toBe('rtl');
	});
});
