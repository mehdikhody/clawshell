import { page, userEvent } from 'vitest/browser';
import { afterEach, describe, expect, it } from 'vitest';
import { render } from 'vitest-browser-svelte';
import { emit } from '@tauri-apps/api/event';
import '../../routes/layout.css';
import { mockTauri, type IpcHandlers } from '$lib/test/tauri-ipc';
import type { AccountList } from '$lib/accounts';
import AccountsScreen from './accounts-screen.svelte';

let restore: () => void;
afterEach(() => restore?.());

const account = (id: string, label: string, status: string) => ({
	id,
	label,
	status,
	running: status === 'running'
});

const threeAccounts: AccountList = {
	claudeFound: true,
	accounts: [
		account('id-run', 'Account 2', 'running'),
		account('id-idle', 'Work', 'idle'),
		account('id-out', 'Account 4', 'signed-out')
	] as AccountList['accounts']
};

function mount(overrides: IpcHandlers = {}, list: AccountList = threeAccounts) {
	const tauri = mockTauri({ list_accounts: () => list, ...overrides });
	restore = tauri.restore;
	render(AccountsScreen);
	return tauri;
}

describe('AccountsScreen', () => {
	it('lists each account as a row with its name and status', async () => {
		mount();

		const rows = page.getByRole('listitem');
		await expect.element(rows.nth(0)).toHaveTextContent('Account 2');
		await expect.element(rows.nth(0)).toHaveTextContent('Running');
		await expect.element(rows.nth(1)).toHaveTextContent('Work');
		await expect.element(rows.nth(1)).toHaveTextContent('Idle');
		await expect.element(rows.nth(2)).toHaveTextContent('Account 4');
		await expect.element(rows.nth(2)).toHaveTextContent('Signed out');
		expect(rows.elements()).toHaveLength(3);
	});

	it('shows an empty state when there are no accounts yet', async () => {
		mount({}, { claudeFound: true, accounts: [] });

		await expect.element(page.getByText('No accounts yet')).toBeVisible();
	});

	it('updates a row when the core reports a status change', async () => {
		mount();
		await expect.element(page.getByRole('listitem').nth(1)).toHaveTextContent('Idle');

		await emit('accounts-changed', {
			claudeFound: true,
			accounts: [
				account('id-run', 'Account 2', 'idle'),
				account('id-idle', 'Work', 'running'),
				account('id-out', 'Account 4', 'signed-out')
			]
		});

		await expect.element(page.getByRole('listitem').nth(1)).toHaveTextContent('Running');
		await expect.element(page.getByRole('listitem').nth(0)).toHaveTextContent('Idle');
	});

	it('launches an account on double-click, and only that account', async () => {
		const tauri = mount({ launch_account: () => 'started' });

		await page.getByRole('listitem').nth(1).dblClick();

		const launches = tauri.calls.filter((c) => c.cmd === 'launch_account');
		expect(launches).toEqual([{ cmd: 'launch_account', args: { id: 'id-idle' } }]);
	});

	it('does not launch on a single click', async () => {
		const tauri = mount();

		await page.getByRole('listitem').nth(0).click();

		expect(tauri.calls.some((c) => c.cmd === 'launch_account')).toBe(false);
	});

	it('launches the focused row with Enter', async () => {
		const tauri = mount({ launch_account: () => 'started' });
		const row = page.getByRole('listitem').nth(2);
		await expect.element(row).toBeVisible();
		(row.element() as HTMLElement).focus();

		await userEvent.keyboard('{Enter}');

		expect(tauri.calls.filter((c) => c.cmd === 'launch_account')).toEqual([
			{ cmd: 'launch_account', args: { id: 'id-out' } }
		]);
	});

	it('adds an account and shows it in the list', async () => {
		let added = false;
		const tauri = mount({
			add_account: () => {
				added = true;
				return account('id-new', 'Account 5', 'signed-out');
			},
			list_accounts: () => ({
				claudeFound: true,
				accounts: added
					? [...threeAccounts.accounts, account('id-new', 'Account 5', 'signed-out')]
					: threeAccounts.accounts
			})
		});
		await expect.element(page.getByRole('listitem').nth(2)).toBeVisible();

		await page.getByRole('button', { name: 'Add account' }).click();

		expect(tauri.calls.filter((c) => c.cmd === 'add_account')).toHaveLength(1);
		await expect.element(page.getByRole('listitem').nth(3)).toHaveTextContent('Account 5');
	});

	it('explains that Claude cannot be found and blocks Add', async () => {
		const tauri = mount({}, { claudeFound: false, accounts: [] });

		await expect.element(page.getByText('Claude Desktop not found')).toBeVisible();
		await expect.element(page.getByRole('button', { name: 'Add account' })).toBeDisabled();
		expect(tauri.calls.some((c) => c.cmd === 'add_account')).toBe(false);
	});

	it('shows a clear message when adding fails', async () => {
		mount({
			add_account: () => {
				throw { code: 'claude-not-found', message: 'Claude Desktop was not found' };
			}
		});
		await expect.element(page.getByRole('listitem').nth(0)).toBeVisible();

		await page.getByRole('button', { name: 'Add account' }).click();

		await expect.element(page.getByRole('alert')).toHaveTextContent('Claude Desktop not found');
	});

	it('shows a clear message when launching fails', async () => {
		mount({
			launch_account: () => {
				throw { code: 'launch-failed', message: 'access denied' };
			}
		});

		await page.getByRole('listitem').nth(0).dblClick();

		await expect.element(page.getByRole('alert')).toHaveTextContent("Couldn't open");
	});
});
