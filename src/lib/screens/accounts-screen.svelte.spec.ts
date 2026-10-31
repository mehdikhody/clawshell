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

	describe('account menu', () => {
		const menuOf = (label: string) => page.getByRole('button', { name: `Actions for ${label}` });

		it('gives every row a menu with Rename, Open folder and Delete', async () => {
			mount();

			await menuOf('Work').click();

			await expect.element(page.getByRole('menuitem', { name: 'Rename' })).toBeVisible();
			await expect.element(page.getByRole('menuitem', { name: 'Open folder' })).toBeVisible();
			await expect.element(page.getByRole('menuitem', { name: 'Delete' })).toBeVisible();
			await expect.element(menuOf('Account 2')).toBeVisible();
			await expect.element(menuOf('Account 4')).toBeVisible();
		});

		it('does not launch the account when its menu is used', async () => {
			const tauri = mount({ launch_account: () => 'started' });
			await expect.element(menuOf('Work')).toBeVisible();
			(menuOf('Work').element() as HTMLElement).focus();

			await userEvent.keyboard('{Enter}');
			await expect.element(page.getByRole('menuitem', { name: 'Rename' })).toBeVisible();
			await userEvent.keyboard('{Escape}');
			await menuOf('Account 4').dblClick();

			expect(tauri.calls.some((c) => c.cmd === 'launch_account')).toBe(false);
		});

		it('opens the account folder', async () => {
			const tauri = mount({ open_account_folder: () => null });

			await menuOf('Work').click();
			await page.getByRole('menuitem', { name: 'Open folder' }).click();

			expect(tauri.calls.filter((c) => c.cmd === 'open_account_folder')).toEqual([
				{ cmd: 'open_account_folder', args: { id: 'id-idle' } }
			]);
		});

		it('shows a clear message when the folder cannot be opened', async () => {
			mount({
				open_account_folder: () => {
					throw { code: 'folder-missing', message: 'The account folder is missing' };
				}
			});

			await menuOf('Work').click();
			await page.getByRole('menuitem', { name: 'Open folder' }).click();

			await expect.element(page.getByRole('alert')).toHaveTextContent("Couldn't open the folder");
		});

		it('renames only the label and shows the new name', async () => {
			let label = 'Work';
			const tauri = mount({
				rename_account: (args) => {
					label = (args as { label: string }).label;
					return account('id-idle', label, 'idle');
				},
				list_accounts: () => ({
					claudeFound: true,
					accounts: [
						account('id-run', 'Account 2', 'running'),
						account('id-idle', label, 'idle'),
						account('id-out', 'Account 4', 'signed-out')
					]
				})
			});

			await menuOf('Work').click();
			await page.getByRole('menuitem', { name: 'Rename' }).click();
			const input = page.getByRole('textbox', { name: 'Name' });
			await expect.element(input).toHaveValue('Work');
			await input.fill('Personal');
			await page.getByRole('button', { name: 'Save' }).click();

			expect(tauri.calls.filter((c) => c.cmd === 'rename_account')).toEqual([
				{ cmd: 'rename_account', args: { id: 'id-idle', label: 'Personal' } }
			]);
			await expect.element(page.getByRole('listitem').nth(1)).toHaveTextContent('Personal');
			await expect.element(page.getByRole('dialog')).not.toBeInTheDocument();
		});

		it('submits a rename with Enter', async () => {
			const tauri = mount({ rename_account: () => account('id-idle', 'Home', 'idle') });

			await menuOf('Work').click();
			await page.getByRole('menuitem', { name: 'Rename' }).click();
			await page.getByRole('textbox', { name: 'Name' }).fill('Home');
			await userEvent.keyboard('{Enter}');

			expect(tauri.calls.filter((c) => c.cmd === 'rename_account')).toEqual([
				{ cmd: 'rename_account', args: { id: 'id-idle', label: 'Home' } }
			]);
		});

		it('cannot save an empty name', async () => {
			const tauri = mount();

			await menuOf('Work').click();
			await page.getByRole('menuitem', { name: 'Rename' }).click();
			await page.getByRole('textbox', { name: 'Name' }).fill('   ');

			await expect.element(page.getByRole('button', { name: 'Save' })).toBeDisabled();
			expect(tauri.calls.some((c) => c.cmd === 'rename_account')).toBe(false);
		});

		it('cancelling a rename changes nothing', async () => {
			const tauri = mount();

			await menuOf('Work').click();
			await page.getByRole('menuitem', { name: 'Rename' }).click();
			await page.getByRole('textbox', { name: 'Name' }).fill('Other');
			await page.getByRole('button', { name: 'Cancel' }).click();

			expect(tauri.calls.some((c) => c.cmd === 'rename_account')).toBe(false);
			await expect.element(page.getByRole('listitem').nth(1)).toHaveTextContent('Work');
		});

		it('shows a clear message when a rename fails', async () => {
			mount({
				rename_account: () => {
					throw { code: 'io', message: 'disk full' };
				}
			});

			await menuOf('Work').click();
			await page.getByRole('menuitem', { name: 'Rename' }).click();
			await page.getByRole('textbox', { name: 'Name' }).fill('Home');
			await page.getByRole('button', { name: 'Save' }).click();

			await expect.element(page.getByRole('alert')).toHaveTextContent("Couldn't rename");
		});

		it('asks before deleting, and deletes only after the user confirms', async () => {
			let deleted = false;
			const tauri = mount({
				delete_account: () => {
					deleted = true;
					return null;
				},
				list_accounts: () => ({
					claudeFound: true,
					accounts: threeAccounts.accounts.filter((a) => !(deleted && a.id === 'id-idle'))
				})
			});

			await menuOf('Work').click();
			await page.getByRole('menuitem', { name: 'Delete' }).click();

			await expect.element(page.getByRole('alertdialog')).toHaveTextContent('Delete Work?');
			expect(tauri.calls.some((c) => c.cmd === 'delete_account')).toBe(false);

			await page.getByRole('alertdialog').getByRole('button', { name: 'Delete' }).click();

			expect(tauri.calls.filter((c) => c.cmd === 'delete_account')).toEqual([
				{ cmd: 'delete_account', args: { id: 'id-idle' } }
			]);
			await expect.poll(() => page.getByRole('listitem').elements().length).toBe(2);
		});

		it('keeps the account when the delete confirmation is cancelled', async () => {
			const tauri = mount();

			await menuOf('Work').click();
			await page.getByRole('menuitem', { name: 'Delete' }).click();
			await page.getByRole('button', { name: 'Cancel' }).click();

			expect(tauri.calls.some((c) => c.cmd === 'delete_account')).toBe(false);
			expect(page.getByRole('listitem').elements()).toHaveLength(3);
		});

		it('refuses to delete a running account and says why', async () => {
			const tauri = mount();

			await menuOf('Account 2').click();
			await page.getByRole('menuitem', { name: 'Delete' }).click();

			await expect.element(page.getByRole('alert')).toHaveTextContent('Close Account 2');
			expect(page.getByRole('alertdialog').elements()).toHaveLength(0);
			expect(tauri.calls.some((c) => c.cmd === 'delete_account')).toBe(false);
		});

		it('shows the core refusal when the account started running after the last update', async () => {
			mount({
				delete_account: () => {
					throw { code: 'account-running', message: 'Close the account first' };
				}
			});

			await menuOf('Work').click();
			await page.getByRole('menuitem', { name: 'Delete' }).click();
			await page.getByRole('alertdialog').getByRole('button', { name: 'Delete' }).click();

			await expect.element(page.getByRole('alert')).toHaveTextContent('Close Work');
		});
	});

	describe('an account whose folder is missing', () => {
		const withMissing: AccountList = {
			claudeFound: true,
			accounts: [
				account('id-idle', 'Work', 'idle'),
				account('id-gone', 'Old laptop', 'missing')
			] as AccountList['accounts']
		};

		it('is flagged and offered for removal', async () => {
			mount({}, withMissing);

			const row = page.getByRole('listitem').nth(1);
			await expect.element(row).toHaveTextContent('Old laptop');
			await expect.element(row).toHaveTextContent('Folder missing');
			await expect.element(row.getByRole('button', { name: 'Remove' })).toBeVisible();
			await expect
				.element(page.getByRole('listitem').nth(0).getByRole('button', { name: 'Remove' }))
				.not.toBeInTheDocument();
		});

		it('is removed after confirmation', async () => {
			let removed = false;
			const tauri = mount(
				{
					delete_account: () => {
						removed = true;
						return null;
					},
					list_accounts: () => ({
						claudeFound: true,
						accounts: removed ? [withMissing.accounts[0]] : withMissing.accounts
					})
				},
				withMissing
			);

			await page.getByRole('button', { name: 'Remove' }).click();
			await expect.element(page.getByRole('alertdialog')).toHaveTextContent('Remove Old laptop?');
			expect(tauri.calls.some((c) => c.cmd === 'delete_account')).toBe(false);
			await page.getByRole('alertdialog').getByRole('button', { name: 'Remove' }).click();

			expect(tauri.calls.filter((c) => c.cmd === 'delete_account')).toEqual([
				{ cmd: 'delete_account', args: { id: 'id-gone' } }
			]);
			await expect.poll(() => page.getByRole('listitem').elements().length).toBe(1);
		});

		it('cannot be launched, and its folder cannot be opened', async () => {
			const tauri = mount({}, withMissing);

			await page.getByRole('listitem').nth(1).dblClick();
			await page.getByRole('button', { name: 'Actions for Old laptop' }).click();

			await expect
				.element(page.getByRole('menuitem', { name: 'Open folder' }))
				.toHaveAttribute('aria-disabled', 'true');
			expect(tauri.calls.some((c) => c.cmd === 'launch_account')).toBe(false);
		});
	});
});
