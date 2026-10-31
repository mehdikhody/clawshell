import { page, userEvent } from 'vitest/browser';
import { afterEach, describe, expect, it } from 'vitest';
import { render } from 'vitest-browser-svelte';
import { emit } from '@tauri-apps/api/event';
import '../../routes/layout.css';
import { mockTauri, type IpcHandlers } from '$lib/test/tauri-ipc';
import { emptyPooled, type AccountList } from '$lib/accounts';

import { switchLocale } from '$lib/locale-state.svelte';
import AccountsScreen from './accounts-screen.svelte';

let restore: () => void;
afterEach(() => restore?.());

const BLUE = '#2563eb';
const palette = [BLUE, '#dc2626', '#16a34a'];

const account = (id: string, label: string, status: string, colour = BLUE) => ({
	id,
	label,
	badge: label.charAt(0).toUpperCase(),
	colour,
	status,
	running: status === 'running',
	email: null,
	usage: null
});

const threeAccounts: AccountList = {
	claudeFound: true,
	refreshAllowedAt: null,
	pooled: emptyPooled,
	palette,
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
	describe('badge', () => {
		const badgeOf = (index: number) =>
			page.getByRole('listitem').nth(index).element().querySelector('[data-slot="account-badge"]')!;

		it('shows each row with the first letter of its name on its colour', async () => {
			mount(
				{},
				{
					...threeAccounts,
					accounts: [
						account('id-run', 'Account 2', 'running', '#2563eb'),
						account('id-idle', 'work', 'idle', '#dc2626')
					] as AccountList['accounts']
				}
			);
			await expect.element(page.getByRole('listitem').nth(1)).toBeVisible();

			expect(badgeOf(0).textContent?.trim()).toBe('A');
			expect(getComputedStyle(badgeOf(0)).backgroundColor).toBe('rgb(37, 99, 235)');
			expect(badgeOf(1).textContent?.trim()).toBe('W');
			expect(getComputedStyle(badgeOf(1)).backgroundColor).toBe('rgb(220, 38, 38)');
		});

		it('follows a rename to a new initial', async () => {
			mount();
			await expect.element(page.getByRole('listitem').nth(1)).toBeVisible();
			expect(badgeOf(1).textContent?.trim()).toBe('W');

			await emit('accounts-changed', {
				...threeAccounts,
				accounts: [
					account('id-run', 'Account 2', 'running'),
					account('id-idle', 'Home', 'idle'),
					account('id-out', 'Account 4', 'signed-out')
				]
			});

			await expect.poll(() => badgeOf(1).textContent?.trim()).toBe('H');
		});

		it('offers the palette in the menu and changes the colour', async () => {
			let colour = BLUE;
			const tauri = mount({
				set_account_colour: (args) => {
					colour = (args as { colour: string }).colour;
					return account('id-idle', 'Work', 'idle', colour);
				},
				list_accounts: () => ({
					...threeAccounts,
					accounts: [
						account('id-run', 'Account 2', 'running'),
						account('id-idle', 'Work', 'idle', colour),
						account('id-out', 'Account 4', 'signed-out')
					]
				})
			});

			await page.getByRole('button', { name: 'Actions for Work' }).click();
			await page.getByRole('menuitem', { name: 'Badge colour' }).click();
			await expect.element(page.getByRole('menuitem', { name: '#16a34a' })).toBeVisible();
			await expect.element(page.getByRole('menuitem', { name: '#2563eb' })).toBeVisible();
			await page.getByRole('menuitem', { name: '#dc2626' }).click();

			expect(tauri.calls.filter((c) => c.cmd === 'set_account_colour')).toEqual([
				{ cmd: 'set_account_colour', args: { id: 'id-idle', colour: '#dc2626' } }
			]);
			await expect
				.poll(() => getComputedStyle(badgeOf(1)).backgroundColor)
				.toBe('rgb(220, 38, 38)');
		});

		it('shows a clear message when the colour cannot be changed', async () => {
			mount({
				set_account_colour: () => {
					throw { code: 'io', message: 'disk full' };
				}
			});

			await page.getByRole('button', { name: 'Actions for Work' }).click();
			await page.getByRole('menuitem', { name: 'Badge colour' }).click();
			await page.getByRole('menuitem', { name: '#dc2626' }).click();

			await expect.element(page.getByRole('alert')).toHaveTextContent("Couldn't change the colour");
		});
	});

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
		mount(
			{},
			{ claudeFound: true, refreshAllowedAt: null, pooled: emptyPooled, palette, accounts: [] }
		);

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
		const tauri = mount(
			{},
			{ claudeFound: false, refreshAllowedAt: null, pooled: emptyPooled, palette, accounts: [] }
		);

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

		it('gives every row a menu with Rename, Open folder, Badge colour and Delete', async () => {
			mount();

			await menuOf('Work').click();

			await expect.element(page.getByRole('menuitem', { name: 'Rename' })).toBeVisible();
			await expect.element(page.getByRole('menuitem', { name: 'Open folder' })).toBeVisible();
			await expect.element(page.getByRole('menuitem', { name: 'Badge colour' })).toBeVisible();
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

		it('adds the account shortcuts and says where they went', async () => {
			const tauri = mount({ add_shortcuts: () => null });

			await menuOf('Work').click();
			await page.getByRole('menuitem', { name: 'Add shortcuts' }).click();

			expect(tauri.calls.filter((c) => c.cmd === 'add_shortcuts')).toEqual([
				{ cmd: 'add_shortcuts', args: { id: 'id-idle' } }
			]);
			await expect
				.element(page.getByRole('status'))
				.toHaveTextContent('Shortcuts added to the Desktop and Start menu');
		});

		it('shows a clear message when the shortcuts cannot be added', async () => {
			mount({
				add_shortcuts: () => {
					throw { code: 'shortcuts-failed', message: 'Access is denied' };
				}
			});

			await menuOf('Work').click();
			await page.getByRole('menuitem', { name: 'Add shortcuts' }).click();

			await expect.element(page.getByRole('alert')).toHaveTextContent("Couldn't add the shortcuts");
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
			refreshAllowedAt: null,
			pooled: emptyPooled,
			palette,
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

		it('cannot be launched, and its folder cannot be opened or shortcut added', async () => {
			const tauri = mount({}, withMissing);

			await page.getByRole('listitem').nth(1).dblClick();
			await page.getByRole('button', { name: 'Actions for Old laptop' }).click();

			await expect
				.element(page.getByRole('menuitem', { name: 'Open folder' }))
				.toHaveAttribute('aria-disabled', 'true');
			await expect
				.element(page.getByRole('menuitem', { name: 'Add shortcuts' }))
				.toHaveAttribute('aria-disabled', 'true');
			expect(tauri.calls.some((c) => c.cmd === 'launch_account')).toBe(false);
		});
	});
});

describe('AccountsScreen usage', () => {
	const MIN = 60_000;
	// Far in the future so the reset always renders as a date plus time.
	const fiveReset = new Date(2030, 0, 1, 15, 0).toISOString();
	const weekReset = new Date(2030, 0, 4, 9, 30).toISOString();

	const usage = (over: Record<string, unknown> = {}) => ({
		fiveHour: { utilization: 42, resetsAt: fiveReset },
		weekly: { utilization: 17.4, resetsAt: weekReset },
		extraUsage: null,
		fetchedAtMs: Date.now(),
		source: 'live',
		...over
	});

	const listWith = (
		extra: Record<string, unknown>,
		status = 'idle',
		running = false,
		list: Record<string, unknown> = {}
	) =>
		({
			claudeFound: true,
			refreshAllowedAt: null,
			pooled: emptyPooled,
			accounts: [
				{
					id: 'id-1',
					label: 'Work',
					status,
					running,
					email: 'work@example.com',
					usage: usage(),
					...extra
				}
			],
			...list
		}) as unknown as AccountList;

	it('shows the email, the 5-hour % with its reset and the weekly % with its reset', async () => {
		mount({}, listWith({}));

		const row = page.getByRole('listitem').nth(0);
		await expect.element(row).toHaveTextContent('work@example.com');
		await expect.element(row).toHaveTextContent(/5-hour\s*42%/);
		await expect.element(row).toHaveTextContent(/Jan 1, 3:00\s?PM/);
		await expect.element(row).toHaveTextContent(/Weekly\s*17%/);
		await expect.element(row).toHaveTextContent(/Jan 4, 9:30\s?AM/);
	});

	it('draws a bar for each window', async () => {
		mount({}, listWith({}));

		const bars = page.getByRole('progressbar');
		await expect.element(bars.nth(0)).toHaveAttribute('aria-valuenow', '42');
		await expect.element(bars.nth(1)).toHaveAttribute('aria-valuenow', '17.4');
	});

	it('shows a Limited badge, and Running beside it when Claude is open on the account', async () => {
		mount(
			{},
			listWith(
				{ usage: usage({ weekly: { utilization: 100, resetsAt: weekReset } }) },
				'limited',
				true
			)
		);

		const row = page.getByRole('listitem').nth(0);
		await expect.element(row).toHaveTextContent('Limited');
		await expect.element(row).toHaveTextContent('Running');
	});

	it('a limited account that is not running shows only Limited', async () => {
		mount({}, listWith({}, 'limited', false));

		const row = page.getByRole('listitem').nth(0);
		await expect.element(row).toHaveTextContent('Limited');
		await expect.element(row).not.toHaveTextContent('Running');
	});

	it('marks a stale snapshot with how long ago it was updated', async () => {
		mount({}, listWith({ usage: usage({ source: 'stale', fetchedAtMs: Date.now() - 5 * MIN }) }));

		const row = page.getByRole('listitem').nth(0);
		await expect.element(row).toHaveTextContent(/Stale, updated 5 minutes ago/);
		await expect.element(row).toHaveTextContent(/42%/);
	});

	it('labels values taken from local history, which have no reset time', async () => {
		mount(
			{},
			listWith({
				usage: usage({
					source: 'local-history',
					fetchedAtMs: Date.now() - 2 * 60 * MIN,
					fiveHour: { utilization: 55, resetsAt: null },
					weekly: { utilization: 60, resetsAt: null }
				})
			})
		);

		const row = page.getByRole('listitem').nth(0);
		await expect.element(row).toHaveTextContent(/From local history, updated 2 hours ago/);
		await expect.element(row).toHaveTextContent(/55%/);
		await expect.element(row).toHaveTextContent(/60%/);
		await expect.element(row).not.toHaveTextContent(/resets/i);
	});

	it('does not label live numbers as stale', async () => {
		mount({}, listWith({}));

		await expect.element(page.getByRole('listitem').nth(0)).toHaveTextContent(/42%/);
		await expect.element(page.getByRole('listitem').nth(0)).not.toHaveTextContent(/stale/i);
	});

	it('says usage is not loaded yet when a signed-in account has no snapshot', async () => {
		mount({}, listWith({ usage: null, email: null }));

		await expect.element(page.getByRole('listitem').nth(0)).toHaveTextContent('No usage data yet');
	});

	it('shows no usage for a signed-out account', async () => {
		mount({}, listWith({ usage: null, email: null }, 'signed-out'));

		const row = page.getByRole('listitem').nth(0);
		await expect.element(row).toHaveTextContent('Signed out');
		await expect.element(row).not.toHaveTextContent('No usage data yet');
	});

	it('updates usage when the core pushes a changed snapshot', async () => {
		mount({}, listWith({}));
		await expect.element(page.getByRole('listitem').nth(0)).toHaveTextContent(/42%/);

		await emit(
			'accounts-changed',
			listWith({ usage: usage({ fiveHour: { utilization: 63, resetsAt: fiveReset } }) })
		);

		await expect.element(page.getByRole('listitem').nth(0)).toHaveTextContent(/63%/);
	});

	describe('manual refresh', () => {
		it('asks the core to refresh when the button is pressed', async () => {
			const tauri = mount(
				{ refresh_usage: () => ({ refreshed: 1, refreshAllowedAt: Date.now() + 3 * MIN }) },
				listWith({})
			);

			await page.getByRole('button', { name: 'Refresh usage' }).click();

			expect(tauri.calls.filter((c) => c.cmd === 'refresh_usage')).toHaveLength(1);
		});

		it('is enabled when a refresh is allowed now', async () => {
			mount({}, listWith({}));

			await expect.element(page.getByRole('button', { name: 'Refresh usage' })).toBeEnabled();
			await expect.element(page.getByText(/Next refresh allowed/)).not.toBeInTheDocument();
		});

		it('is disabled inside the minimum interval and says when the next refresh is allowed', async () => {
			const tauri = mount(
				{},
				listWith({}, 'idle', false, { refreshAllowedAt: Date.now() + 151_500 })
			);

			await expect.element(page.getByRole('button', { name: 'Refresh usage' })).toBeDisabled();
			await expect.element(page.getByText(/Next refresh allowed in 2:3\d/)).toBeVisible();
			expect(tauri.calls.some((c) => c.cmd === 'refresh_usage')).toBe(false);
		});

		it('becomes available again once the wait is over', async () => {
			mount({}, listWith({}, 'idle', false, { refreshAllowedAt: Date.now() + 1500 }));
			await expect.element(page.getByRole('button', { name: 'Refresh usage' })).toBeDisabled();

			await expect
				.element(page.getByRole('button', { name: 'Refresh usage' }), { timeout: 4000 })
				.toBeEnabled();
		});

		it('shows the wait the core reports after a refresh', async () => {
			let allowedAt: number | null = null;
			mount(
				{
					refresh_usage: () => {
						allowedAt = Date.now() + 180_000;
						return { refreshed: 1, refreshAllowedAt: allowedAt };
					},
					list_accounts: () => listWith({}, 'idle', false, { refreshAllowedAt: allowedAt })
				},
				listWith({})
			);

			await page.getByRole('button', { name: 'Refresh usage' }).click();

			await expect.element(page.getByText(/Next refresh allowed in/)).toBeVisible();
			await expect.element(page.getByRole('button', { name: 'Refresh usage' })).toBeDisabled();
		});
	});
});

describe('AccountsScreen in Persian', () => {
	afterEach(() => switchLocale('en'));

	const withUsage = {
		claudeFound: true,
		refreshAllowedAt: null,
		pooled: emptyPooled,
		accounts: [
			{
				id: 'id-1',
				label: 'Work',
				status: 'idle',
				running: false,
				email: null,
				usage: {
					fiveHour: { utilization: 42, resetsAt: new Date(2030, 0, 1, 15, 0).toISOString() },
					weekly: { utilization: 17.4, resetsAt: null },
					extraUsage: null,
					fetchedAtMs: Date.now() - 5 * 60_000,
					source: 'stale'
				}
			}
		]
	} as unknown as AccountList;

	it('translates the rows and writes usage and dates in Persian digits', async () => {
		switchLocale('fa');
		mount({}, withUsage);

		const row = page.getByRole('listitem').nth(0);
		await expect.element(row).toHaveTextContent('آماده');
		await expect.element(row).toHaveTextContent('۴۲٪');
		await expect.element(row).toHaveTextContent('۱۷٪');
		await expect.element(row).toHaveTextContent('۱ ژانویه، ۱۵:۰۰');
		await expect.element(row).toHaveTextContent('۵ دقیقه پیش');
		expect(row.element().textContent).not.toMatch(/[0-9]/);
	});

	it('puts the status badge on the left of a right-aligned name', async () => {
		switchLocale('fa');
		mount({}, withUsage);

		const row = page.getByRole('listitem').nth(0);
		const name = row.getByText('Work');
		const badge = row.getByText('آماده');
		await expect.element(name).toBeVisible();
		expect(name.element().getBoundingClientRect().left).toBeGreaterThan(
			badge.element().getBoundingClientRect().left
		);
	});
});
