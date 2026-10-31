import { page } from 'vitest/browser';
import { afterEach, describe, expect, it } from 'vitest';
import { render } from 'vitest-browser-svelte';
import { emit } from '@tauri-apps/api/event';
import '../../routes/layout.css';
import { mockTauri, type IpcHandlers } from '$lib/test/tauri-ipc';
import { switchLocale } from '$lib/locale-state.svelte';
import RoutingOffer from './routing-offer.svelte';

let restore: () => void;
afterEach(() => {
	restore?.();
	switchLocale('en');
});

function mount(overrides: IpcHandlers = {}) {
	const tauri = mockTauri({
		set_sign_in_routing: () => ({ settings: {}, needsDefaultAppsChoice: false }),
		...overrides
	});
	restore = tauri.restore;
	render(RoutingOffer);
	return tauri;
}

const routingCalls = (calls: { cmd: string; args: unknown }[]) =>
	calls.filter((c) => c.cmd === 'set_sign_in_routing').map((c) => c.args);

const offer = () => emit('routing-offer');
const title = () => page.getByText('Route browser sign-ins to the right account?');

describe('RoutingOffer', () => {
	it('shows nothing until the core offers routing', async () => {
		mount();

		await expect.element(title()).not.toBeInTheDocument();
	});

	it('offers routing when the first account has been added', async () => {
		mount();

		await offer();

		await expect.element(title()).toBeVisible();
		await expect.element(page.getByRole('button', { name: 'Turn on' })).toBeVisible();
		await expect.element(page.getByRole('button', { name: 'Not now' })).toBeVisible();
	});

	it('turns routing on and closes when the user accepts', async () => {
		const tauri = mount();
		await offer();

		await page.getByRole('button', { name: 'Turn on' }).click();

		await expect.element(title()).not.toBeInTheDocument();
		expect(routingCalls(tauri.calls)).toEqual([{ enabled: true }]);
	});

	it('leaves routing alone when the user declines', async () => {
		const tauri = mount();
		await offer();

		await page.getByRole('button', { name: 'Not now' }).click();

		await expect.element(title()).not.toBeInTheDocument();
		expect(routingCalls(tauri.calls)).toEqual([]);
	});

	it('guides the Default Apps choice on the Store build before closing', async () => {
		mount({ set_sign_in_routing: () => ({ settings: {}, needsDefaultAppsChoice: true }) });
		await offer();

		await page.getByRole('button', { name: 'Turn on' }).click();

		await expect.element(page.getByText('One choice left in Windows')).toBeVisible();
		await expect.element(page.getByText(/set the claude link type to clawshell/i)).toBeVisible();
		await page.getByRole('button', { name: 'Got it' }).click();
		await expect.element(page.getByText('One choice left in Windows')).not.toBeInTheDocument();
	});

	it('says so and stays open when Windows refuses', async () => {
		mount({
			set_sign_in_routing: () => {
				throw { code: 'routing-failed', message: 'access denied' };
			}
		});
		await offer();

		await page.getByRole('button', { name: 'Turn on' }).click();

		await expect.element(page.getByRole('alert')).toHaveTextContent(/sign-in routing/i);
		await expect.element(title()).toBeVisible();
	});
});
