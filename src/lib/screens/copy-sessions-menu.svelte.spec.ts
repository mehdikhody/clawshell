import { page } from 'vitest/browser';
import { afterEach, describe, expect, it } from 'vitest';
import { render } from 'vitest-browser-svelte';
import '../../routes/layout.css';
import { mockTauri } from '$lib/test/tauri-ipc';
import { emptyPooled, type Account, type AccountList } from '$lib/accounts';
import AccountsScreen from './accounts-screen.svelte';

let restore: () => void;
afterEach(() => restore?.());

const account = (over: Partial<Account>): Account => ({
	id: 'id-work',
	label: 'Work',
	isDefault: false,
	badge: 'W',
	colour: '#2563eb',
	status: 'idle',
	copyBlocker: null,
	running: false,
	email: null,
	usage: null,
	usagePending: false,
	env: [],
	flags: [],
	...over
});

const list: AccountList = {
	claudeFound: true,
	refreshAllowedAt: null,
	pooled: emptyPooled,
	palette: ['#2563eb'],
	accounts: [account({ id: 'default', label: 'Claude', isDefault: true }), account({})]
};

describe('Copy sessions menu item', () => {
	for (const label of ['Claude', 'Work']) {
		it(`opens the dialog for ${label}, the default account included`, async () => {
			const tauri = mockTauri({ list_accounts: () => list, list_sessions: () => [] });
			restore = tauri.restore;
			render(AccountsScreen);

			await page.getByText(label, { exact: true }).click({ button: 'right' });
			await page.getByRole('menuitem', { name: 'Copy sessions…' }).click();

			await expect
				.element(page.getByRole('dialog'))
				.toHaveTextContent(`Copy sessions from ${label}`);
			expect(tauri.calls.find((c) => c.cmd === 'list_sessions')?.args).toEqual({
				id: label === 'Claude' ? 'default' : 'id-work'
			});
		});
	}
});
