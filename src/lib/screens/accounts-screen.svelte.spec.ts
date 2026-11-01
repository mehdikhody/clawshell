import { page, userEvent } from 'vitest/browser';
import { afterEach, describe, expect, it } from 'vitest';
import { render } from 'vitest-browser-svelte';
import { emit } from '@tauri-apps/api/event';
import '../../routes/layout.css';
import { mockTauri, type IpcHandlers } from '$lib/test/tauri-ipc';
import { deferred } from '$lib/test/deferred';
import { emptyPooled, type AccountList } from '$lib/accounts';
import { addRequest } from '$lib/add-account-request.svelte';

import { switchLocale } from '$lib/locale-state.svelte';
import AccountsScreen from './accounts-screen.svelte';

let restore: () => void;
afterEach(() => restore?.());

const BLUE = '#2563eb';
const palette = [BLUE, '#dc2626', '#16a34a'];

const account = (id: string, label: string, status: string, colour = BLUE) => ({
	id,
	label,
	isDefault: id === 'default',
	badge: label.charAt(0).toUpperCase(),
	colour,
	status,
	copyBlocker: null,
	running: status === 'running',
	email: null,
	usage: null,
	usagePending: false,
	env: [],
	flags: []
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
	const tauri = mockTauri({
		list_accounts: () => list,
		new_account_defaults: () => ({ label: 'Account 5', colour: BLUE }),
		account_icon: (args) => {
			const { badge, colour } = args as { badge: string; colour: string };
			return `data:test,${badge}${colour}`;
		},
		...overrides
	});
	restore = tauri.restore;
	render(AccountsScreen);
	return tauri;
}

/** Which parts of an account row are drawn grey; the usage bars are the row's one unslotted child. */
function greyedParts(row: Element) {
	const greyed = (element: Element | null) =>
		!!element && getComputedStyle(element).filter.includes('grayscale');
	return {
		row: greyed(row),
		media: greyed(row.querySelector(':scope > [data-slot=item-media]')),
		content: greyed(row.querySelector(':scope > [data-slot=item-content]')),
		usage: greyed(row.querySelector(':scope > :not([data-slot])')),
		actions: greyed(row.querySelector(':scope > [data-slot=item-actions]'))
	};
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

			expect(badgeOf(0).getAttribute('src')).toBe(`data:test,A${BLUE}`);
			expect(badgeOf(1).getAttribute('src')).toBe('data:test,W#dc2626');
		});

		it('follows a rename to a new initial', async () => {
			mount();
			await expect.element(page.getByRole('listitem').nth(1)).toBeVisible();
			expect(badgeOf(1).getAttribute('src')).toBe(`data:test,W${BLUE}`);

			await emit('accounts-changed', {
				...threeAccounts,
				accounts: [
					account('id-run', 'Account 2', 'running'),
					account('id-idle', 'Home', 'idle'),
					account('id-out', 'Account 4', 'signed-out')
				]
			});

			await expect.poll(() => badgeOf(1).getAttribute('src')).toBe(`data:test,H${BLUE}`);
		});

		it('changes the colour from the Edit account dialog', async () => {
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

			await page.getByText('Work', { exact: true }).click({ button: 'right' });
			await page.getByRole('menuitem', { name: 'Edit account' }).click();
			await expect.element(page.getByRole('dialog', { name: 'Edit account' })).toBeVisible();
			await expect.element(page.getByRole('button', { name: '#16a34a' })).toBeVisible();
			await expect.element(page.getByRole('button', { name: '#2563eb' })).toBeVisible();
			await page.getByRole('button', { name: '#dc2626' }).click();
			await page.getByRole('button', { name: 'Save' }).click();

			expect(tauri.calls.filter((c) => c.cmd === 'set_account_colour')).toEqual([
				{ cmd: 'set_account_colour', args: { id: 'id-idle', colour: '#dc2626' } }
			]);
			await expect.poll(() => badgeOf(1).getAttribute('src')).toBe('data:test,W#dc2626');
		});

		it('shows a clear message when the account cannot be saved', async () => {
			mount({
				set_account_colour: () => {
					throw { code: 'io', message: 'disk full' };
				}
			});

			await page.getByText('Work', { exact: true }).click({ button: 'right' });
			await page.getByRole('menuitem', { name: 'Edit account' }).click();
			await page.getByRole('button', { name: '#dc2626' }).click();
			await page.getByRole('button', { name: 'Save' }).click();

			await expect.element(page.getByRole('alert')).toHaveTextContent("Couldn't save the account");
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

	it('opens the Add dialog with the default name and colour, then creates the account', async () => {
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
		await expect.element(page.getByRole('dialog', { name: 'Add account' })).toBeVisible();
		await expect.element(page.getByRole('textbox', { name: 'Name' })).toHaveValue('Account 5');
		await page.getByRole('button', { name: 'Create' }).click();

		expect(tauri.calls.filter((c) => c.cmd === 'add_account')).toEqual([
			{ cmd: 'add_account', args: { label: 'Account 5', colour: BLUE } }
		]);
		await expect.element(page.getByRole('listitem').nth(3)).toHaveTextContent('Account 5');
		await expect.element(page.getByRole('dialog')).not.toBeInTheDocument();
	});

	it('creates the account under the name typed and the colour picked', async () => {
		const tauri = mount();
		await expect.element(page.getByRole('listitem').nth(2)).toBeVisible();

		await page.getByRole('button', { name: 'Add account' }).click();
		await page.getByRole('textbox', { name: 'Name' }).fill('Work');
		await page.getByRole('button', { name: '#16a34a' }).click();
		await page.getByRole('button', { name: 'Create' }).click();

		expect(tauri.calls.filter((c) => c.cmd === 'add_account')).toEqual([
			{ cmd: 'add_account', args: { label: 'Work', colour: '#16a34a' } }
		]);
	});

	it('previews the badge letter and colour as the name and colour change', async () => {
		mount();
		await expect.element(page.getByRole('listitem').nth(2)).toBeVisible();

		await page.getByRole('button', { name: 'Add account' }).click();
		const dialog = page.getByRole('dialog', { name: 'Add account' });
		await page.getByRole('textbox', { name: 'Name' }).fill('home');
		await page.getByRole('button', { name: '#dc2626' }).click();

		await expect
			.poll(() =>
				dialog.element().querySelector('[data-slot="account-badge"]')?.getAttribute('src')
			)
			.toBe('data:test,H#dc2626');
	});

	it('cannot create an account with a blank name', async () => {
		const tauri = mount();
		await expect.element(page.getByRole('listitem').nth(2)).toBeVisible();

		await page.getByRole('button', { name: 'Add account' }).click();
		await page.getByRole('textbox', { name: 'Name' }).fill('   ');

		await expect.element(page.getByRole('button', { name: 'Create' })).toBeDisabled();
		expect(tauri.calls.some((c) => c.cmd === 'add_account')).toBe(false);
	});

	it('creates the account when Enter is pressed in the name', async () => {
		const tauri = mount();
		await expect.element(page.getByRole('listitem').nth(2)).toBeVisible();

		await page.getByRole('button', { name: 'Add account' }).click();
		await page.getByRole('textbox', { name: 'Name' }).fill('Home');
		await userEvent.keyboard('{Enter}');

		expect(tauri.calls.filter((c) => c.cmd === 'add_account')).toEqual([
			{ cmd: 'add_account', args: { label: 'Home', colour: BLUE } }
		]);
	});

	it('cancelling the Add dialog creates nothing', async () => {
		const tauri = mount();
		await expect.element(page.getByRole('listitem').nth(2)).toBeVisible();

		await page.getByRole('button', { name: 'Add account' }).click();
		await page.getByRole('textbox', { name: 'Name' }).fill('Other');
		await page.getByRole('button', { name: 'Cancel' }).click();

		expect(tauri.calls.some((c) => c.cmd === 'add_account')).toBe(false);
		await expect.element(page.getByRole('dialog')).not.toBeInTheDocument();
	});

	it('opens the Add dialog when the tray asks for a new account', async () => {
		addRequest.pending = true;
		mount();

		await expect.element(page.getByRole('dialog', { name: 'Add account' })).toBeVisible();
		expect(addRequest.pending).toBe(false);
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
		await page.getByRole('button', { name: 'Create' }).click();

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
		const menuOf = (label: string) => page.getByText(label, { exact: true });

		it('gives every row a menu with Edit account, Open folder and Delete', async () => {
			mount();

			await menuOf('Work').click({ button: 'right' });

			await expect.element(page.getByRole('menuitem', { name: 'Edit account' })).toBeVisible();
			await expect.element(page.getByRole('menuitem', { name: 'Open folder' })).toBeVisible();
			await expect.element(page.getByRole('menuitem', { name: 'Delete' })).toBeVisible();
			await expect.element(menuOf('Account 2')).toBeVisible();
			await expect.element(menuOf('Account 4')).toBeVisible();
		});

		it('does not launch the account when its menu is opened', async () => {
			const tauri = mount({ launch_account: () => 'started' });
			await menuOf('Work').click({ button: 'right' });
			await expect.element(page.getByRole('menuitem', { name: 'Edit account' })).toBeVisible();
			await userEvent.keyboard('{Escape}');

			expect(tauri.calls.some((c) => c.cmd === 'launch_account')).toBe(false);
		});

		it('opens the account folder', async () => {
			const tauri = mount({ open_account_folder: () => null });

			await menuOf('Work').click({ button: 'right' });
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

			await menuOf('Work').click({ button: 'right' });
			await page.getByRole('menuitem', { name: 'Open folder' }).click();

			await expect.element(page.getByRole('alert')).toHaveTextContent("Couldn't open the folder");
		});

		it('offers Quit Claude on a running account and calls quit_account', async () => {
			const tauri = mount({ quit_account: () => null });

			await menuOf('Account 2').click({ button: 'right' });
			await page.getByRole('menuitem', { name: 'Quit Claude' }).click();

			expect(tauri.calls.filter((c) => c.cmd === 'quit_account')).toEqual([
				{ cmd: 'quit_account', args: { id: 'id-run' } }
			]);
		});

		it('offers Quit Claude on the default account when it is running', async () => {
			const tauri = mount(
				{ quit_account: () => null, claude_icon: () => 'data:test,claude' },
				{
					...threeAccounts,
					accounts: [
						account('default', 'Claude', 'running'),
						account('id-idle', 'Work', 'idle')
					] as AccountList['accounts']
				}
			);

			await menuOf('Claude').click({ button: 'right' });
			await page.getByRole('menuitem', { name: 'Quit Claude' }).click();

			expect(tauri.calls.filter((c) => c.cmd === 'quit_account')).toEqual([
				{ cmd: 'quit_account', args: { id: 'default' } }
			]);
		});

		it('hides Quit Claude when the account is not running', async () => {
			mount();

			await menuOf('Work').click({ button: 'right' });

			await expect.element(page.getByRole('menuitem', { name: 'Open folder' })).toBeVisible();
			await expect
				.element(page.getByRole('menuitem', { name: 'Quit Claude' }))
				.not.toBeInTheDocument();
		});

		it('quits without asking for a confirmation', async () => {
			mount({ quit_account: () => null });

			await menuOf('Account 2').click({ button: 'right' });
			await page.getByRole('menuitem', { name: 'Quit Claude' }).click();

			await expect.element(page.getByRole('alertdialog')).not.toBeInTheDocument();
			await expect.element(page.getByRole('dialog')).not.toBeInTheDocument();
		});

		it('shows a clear message when Claude cannot be quit', async () => {
			mount({
				quit_account: () => {
					throw { code: 'io', message: 'Access is denied' };
				}
			});

			await menuOf('Account 2').click({ button: 'right' });
			await page.getByRole('menuitem', { name: 'Quit Claude' }).click();

			await expect.element(page.getByRole('alert')).toHaveTextContent("Couldn't quit Claude");
		});

		it('adds the account shortcuts and says where they went', async () => {
			const tauri = mount({ add_shortcuts: () => null });

			await menuOf('Work').click({ button: 'right' });
			await page.getByRole('menuitem', { name: 'Add shortcuts' }).click();

			expect(tauri.calls.filter((c) => c.cmd === 'add_shortcuts')).toEqual([
				{ cmd: 'add_shortcuts', args: { id: 'id-idle' } }
			]);
			// The drag library keeps its own live region, so pick the notice by its text.
			await expect
				.element(
					page
						.getByRole('status')
						.filter({ hasText: 'Shortcuts added to the Desktop and Start menu' })
				)
				.toBeVisible();
		});

		it('shows a clear message when the shortcuts cannot be added', async () => {
			mount({
				add_shortcuts: () => {
					throw { code: 'shortcuts-failed', message: 'Access is denied' };
				}
			});

			await menuOf('Work').click({ button: 'right' });
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

			await menuOf('Work').click({ button: 'right' });
			await page.getByRole('menuitem', { name: 'Edit account' }).click();
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

			await menuOf('Work').click({ button: 'right' });
			await page.getByRole('menuitem', { name: 'Edit account' }).click();
			await page.getByRole('textbox', { name: 'Name' }).fill('Home');
			await userEvent.keyboard('{Enter}');

			expect(tauri.calls.filter((c) => c.cmd === 'rename_account')).toEqual([
				{ cmd: 'rename_account', args: { id: 'id-idle', label: 'Home' } }
			]);
		});

		it('cannot save an empty name', async () => {
			const tauri = mount();

			await menuOf('Work').click({ button: 'right' });
			await page.getByRole('menuitem', { name: 'Edit account' }).click();
			await page.getByRole('textbox', { name: 'Name' }).fill('   ');

			await expect.element(page.getByRole('button', { name: 'Save' })).toBeDisabled();
			expect(tauri.calls.some((c) => c.cmd === 'rename_account')).toBe(false);
		});

		it('cancelling a rename changes nothing', async () => {
			const tauri = mount();

			await menuOf('Work').click({ button: 'right' });
			await page.getByRole('menuitem', { name: 'Edit account' }).click();
			await page.getByRole('textbox', { name: 'Name' }).fill('Other');
			await page.getByRole('button', { name: 'Cancel' }).click();

			expect(tauri.calls.some((c) => c.cmd === 'rename_account')).toBe(false);
			await expect.element(page.getByRole('listitem').nth(1)).toHaveTextContent('Work');
		});

		it('shows a clear message when a save fails', async () => {
			mount({
				rename_account: () => {
					throw { code: 'io', message: 'disk full' };
				}
			});

			await menuOf('Work').click({ button: 'right' });
			await page.getByRole('menuitem', { name: 'Edit account' }).click();
			await page.getByRole('textbox', { name: 'Name' }).fill('Home');
			await page.getByRole('button', { name: 'Save' }).click();

			await expect.element(page.getByRole('alert')).toHaveTextContent("Couldn't save the account");
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

			await menuOf('Work').click({ button: 'right' });
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

			await menuOf('Work').click({ button: 'right' });
			await page.getByRole('menuitem', { name: 'Delete' }).click();
			await page.getByRole('button', { name: 'Cancel' }).click();

			expect(tauri.calls.some((c) => c.cmd === 'delete_account')).toBe(false);
			expect(page.getByRole('listitem').elements()).toHaveLength(3);
		});

		it('refuses to delete a running account and says why', async () => {
			const tauri = mount();

			await menuOf('Account 2').click({ button: 'right' });
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

			await menuOf('Work').click({ button: 'right' });
			await page.getByRole('menuitem', { name: 'Delete' }).click();
			await page.getByRole('alertdialog').getByRole('button', { name: 'Delete' }).click();

			await expect.element(page.getByRole('alert')).toHaveTextContent('Close Work');
		});
	});

	describe('reordering', () => {
		/** The text of each row, top to bottom, as the page shows it now. */
		const rowTexts = () =>
			[...document.querySelectorAll('[role="listitem"]')].map((row) => row.textContent ?? '');

		/** A point on a row: its top edge, its middle or its bottom edge. */
		function spot(row: Element, where: 'top' | 'middle' | 'bottom') {
			const { left, top, bottom, width } = row.getBoundingClientRect();
			const clientY =
				where === 'top' ? top + 2 : where === 'bottom' ? bottom - 2 : (top + bottom) / 2;
			return { clientX: left + width / 2, clientY };
		}

		/** Sends a pointer event to the page, as the drag library listens for it. */
		function pointer(
			type: string,
			at: { clientX: number; clientY: number },
			target: EventTarget = document
		) {
			target.dispatchEvent(
				new PointerEvent(type, {
					...at,
					bubbles: true,
					button: 0,
					pointerId: 1,
					pointerType: 'mouse',
					isPrimary: true
				})
			);
		}

		it('moves the rows as a row is dragged, and saves the order when it is dropped', async () => {
			const tauri = mount();
			await expect.element(page.getByRole('listitem').nth(2)).toBeVisible();
			const [first, , last] = [...document.querySelectorAll('[role="listitem"]')];
			const over = spot(first, 'top');

			pointer('pointerdown', spot(last, 'middle'), last);
			pointer('pointermove', over);

			await expect.poll(() => rowTexts()[0]).toContain('Account 4');
			expect(tauri.calls.filter((call) => call.cmd === 'reorder_accounts')).toEqual([]);

			pointer('pointerup', over);

			await expect
				.poll(() => tauri.calls.filter((call) => call.cmd === 'reorder_accounts'))
				.toEqual([{ cmd: 'reorder_accounts', args: { ids: ['id-out', 'id-run', 'id-idle'] } }]);
		});

		it('puts the rows back when a drag ends outside the list', async () => {
			const tauri = mount();
			await expect.element(page.getByRole('listitem').nth(2)).toBeVisible();
			const [first, , last] = [...document.querySelectorAll('[role="listitem"]')];

			pointer('pointerdown', spot(last, 'middle'), last);
			pointer('pointermove', spot(first, 'top'));
			await expect.poll(() => rowTexts()[0]).toContain('Account 4');

			// The pointer leaves the list before it is let go.
			pointer('pointermove', { clientX: 1, clientY: 1 });
			pointer('pointerup', { clientX: 1, clientY: 1 });

			await expect.poll(() => rowTexts()[0]).toContain('Account 2');
			expect(tauri.calls.filter((call) => call.cmd === 'reorder_accounts')).toEqual([]);
		});

		it('moves the default account like any other', async () => {
			const tauri = mount(
				{},
				{
					...threeAccounts,
					accounts: [
						account('default', 'Claude', 'idle', '#d97757'),
						account('id-idle', 'Work', 'idle')
					] as AccountList['accounts']
				}
			);
			await expect.element(page.getByRole('listitem').nth(1)).toBeVisible();
			const [first, last] = [...document.querySelectorAll('[role="listitem"]')];
			const below = spot(last, 'bottom');

			pointer('pointerdown', spot(first, 'middle'), first);
			pointer('pointermove', below);
			await expect.poll(() => rowTexts()[1]).toContain('Claude');
			pointer('pointerup', below);

			await expect
				.poll(() => tauri.calls.filter((call) => call.cmd === 'reorder_accounts'))
				.toEqual([{ cmd: 'reorder_accounts', args: { ids: ['id-idle', 'default'] } }]);
		});
	});

	describe('the default account', () => {
		const withDefault: AccountList = {
			...threeAccounts,
			accounts: [
				account('default', 'Claude', 'idle', '#d97757'),
				account('id-idle', 'Work', 'idle')
			] as AccountList['accounts']
		};

		it("shows Claude's own icon instead of a badge", async () => {
			mount({ claude_icon: () => 'data:test,claude' }, withDefault);

			const row = page.getByRole('listitem').nth(0);
			await expect.element(row).toHaveTextContent('Claude');
			await expect
				.element(row.element().querySelector<HTMLElement>('[data-slot="account-badge"]')!)
				.toHaveAttribute('src', 'data:test,claude');
		});

		it('offers edit, folder and launch options, but no shortcuts, colour or delete', async () => {
			mount({}, withDefault);

			await page.getByText('Claude', { exact: true }).click({ button: 'right' });

			await expect.element(page.getByRole('menuitem', { name: 'Edit account' })).toBeVisible();
			await expect.element(page.getByRole('menuitem', { name: 'Open folder' })).toBeVisible();
			await expect
				.element(page.getByRole('menuitem', { name: 'Environment and flags' }))
				.toBeVisible();
			await expect
				.element(page.getByRole('menuitem', { name: 'Add shortcuts' }))
				.not.toBeInTheDocument();
			await expect
				.element(page.getByRole('menuitem', { name: 'Badge colour' }))
				.not.toBeInTheDocument();
			await expect.element(page.getByRole('menuitem', { name: 'Delete' })).not.toBeInTheDocument();
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

		it('is greyed out, but its badge and Remove button keep their colour', async () => {
			mount({}, withMissing);

			const row = page.getByRole('listitem').nth(1);
			await expect.element(row).toHaveTextContent('Folder missing');
			expect(greyedParts(row.element())).toMatchObject({
				row: false,
				media: true,
				content: true,
				actions: false
			});
			expect(greyedParts(page.getByRole('listitem').nth(0).element())).toMatchObject({
				media: false,
				content: false
			});
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
			await page.getByText('Old laptop', { exact: true }).click({ button: 'right' });

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

describe('AccountsScreen loading', () => {
	const skeletons = () => page.getByTestId('account-skeleton');

	it('shows two account-shaped placeholder rows until the list arrives', async () => {
		const answer = deferred<AccountList>();
		mount({ list_accounts: () => answer.promise });

		await expect.element(skeletons().first()).toBeVisible();
		expect(skeletons().elements()).toHaveLength(2);
		// Each placeholder carries the usage block's shape too.
		expect(page.getByTestId('usage-skeleton').elements()).toHaveLength(2);
		await expect.element(page.getByText('Loading…')).not.toBeInTheDocument();

		answer.resolve(threeAccounts);

		await expect.element(page.getByRole('listitem').first()).toBeVisible();
		await expect.element(skeletons()).not.toBeInTheDocument();
	});

	it('replaces the placeholders with the empty state when there are no accounts', async () => {
		mount({}, { ...threeAccounts, accounts: [] });

		await expect.element(page.getByText('No accounts yet')).toBeVisible();
		await expect.element(skeletons()).not.toBeInTheDocument();
	});

	it('never brings the placeholders back on a refresh', async () => {
		mount();
		await expect.element(page.getByRole('listitem').first()).toBeVisible();

		await emit('accounts-changed', { ...threeAccounts, accounts: [] });

		await expect.element(page.getByText('No accounts yet')).toBeVisible();
		await expect.element(skeletons()).not.toBeInTheDocument();
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

	it('greys out a limited account, even while running, but keeps its badges in colour', async () => {
		mount({}, listWith({}, 'limited', true));

		const row = page.getByRole('listitem').nth(0);
		await expect.element(row).toHaveTextContent('Limited');
		expect(greyedParts(row.element())).toEqual({
			row: false,
			media: true,
			content: true,
			usage: true,
			actions: false
		});
	});

	it('does not grey out an account that is not limited', async () => {
		mount({}, listWith({}));

		const row = page.getByRole('listitem').nth(0);
		await expect.element(row).toHaveTextContent('work@example.com');
		expect(greyedParts(row.element())).toEqual({
			row: false,
			media: false,
			content: false,
			usage: false,
			actions: false
		});
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

	it('says there is no usage when the first poll brought none back', async () => {
		mount({}, listWith({ usage: null, email: null }));

		await expect.element(page.getByRole('listitem').nth(0)).toHaveTextContent('No usage data yet');
		await expect.element(page.getByTestId('usage-skeleton')).not.toBeInTheDocument();
	});

	it('shows the email and usage as placeholders until the first poll is tried', async () => {
		mount({}, listWith({ usage: null, email: null, usagePending: true }));

		const row = page.getByRole('listitem').nth(0);
		await expect.element(row.getByTestId('usage-skeleton')).toBeVisible();
		await expect.element(row.getByTestId('email-skeleton')).toBeVisible();
		await expect.element(row).toHaveTextContent('Idle');
		await expect.element(row).not.toHaveTextContent('No usage data yet');
	});

	it('swaps the placeholders for the numbers when the first poll lands', async () => {
		mount({}, listWith({ usage: null, email: null, usagePending: true }));
		const row = page.getByRole('listitem').nth(0);
		await expect.element(row.getByTestId('usage-skeleton')).toBeVisible();

		await emit('accounts-changed', listWith({ email: 'a@example.com' }));

		await expect.element(row).toHaveTextContent(/42%/);
		await expect.element(row).toHaveTextContent('a@example.com');
		await expect.element(row.getByTestId('usage-skeleton')).not.toBeInTheDocument();
		await expect.element(row.getByTestId('email-skeleton')).not.toBeInTheDocument();
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
		await expect.element(row).toHaveTextContent('۱۲ دی، ۱۵:۰۰');
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

	it('lays the account menu out right to left, with item icons on the right', async () => {
		switchLocale('fa');
		mount({}, withUsage);

		await page.getByText('Work', { exact: true }).click({ button: 'right' });

		const menu = page.getByRole('menu').element();
		expect(getComputedStyle(menu).direction).toBe('rtl');
		const item = page.getByRole('menuitem').nth(0).element() as HTMLElement;
		const icon = item.querySelector('svg')!.getBoundingClientRect();
		expect(icon.left).toBeGreaterThan(item.getBoundingClientRect().left + item.offsetWidth / 2);
	});
});
