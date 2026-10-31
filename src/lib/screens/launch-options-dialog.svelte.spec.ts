import { page } from 'vitest/browser';
import { afterEach, describe, expect, it } from 'vitest';
import { render } from 'vitest-browser-svelte';
import '../../routes/layout.css';
import { mockTauri, type IpcHandlers } from '$lib/test/tauri-ipc';
import { emptyPooled, type Account, type AccountList } from '$lib/accounts';
import AccountsScreen from './accounts-screen.svelte';

let restore: () => void;
afterEach(() => restore?.());

const account = (over: Partial<Account> = {}): Account => ({
	id: 'id-work',
	label: 'Work',
	isDefault: false,
	badge: 'W',
	colour: '#2563eb',
	status: 'idle',
	running: false,
	email: null,
	usage: null,
	env: [],
	flags: [],
	...over
});

const listOf = (a: Account): AccountList => ({
	claudeFound: true,
	refreshAllowedAt: null,
	pooled: emptyPooled,
	palette: ['#2563eb'],
	accounts: [a]
});

function mount(a: Account = account(), overrides: IpcHandlers = {}) {
	const tauri = mockTauri({ list_accounts: () => listOf(a), ...overrides });
	restore = tauri.restore;
	render(AccountsScreen);
	return tauri;
}

async function openEditor(label = 'Work') {
	await page.getByText(label, { exact: true }).click({ button: 'right' });
	await page.getByRole('menuitem', { name: 'Environment and flags' }).click();
}

const saves = (tauri: ReturnType<typeof mount>) =>
	tauri.calls.filter((c) => c.cmd === 'set_launch_options');

describe('Environment and flags editor', () => {
	it('is offered in the account menu and opens with a start-fresh note', async () => {
		mount();

		await openEditor();

		await expect.element(page.getByRole('dialog')).toBeVisible();
		await expect
			.element(page.getByRole('dialog'))
			.toHaveTextContent(/only when this account's Claude starts fresh/i);
	});

	it('shows the saved variables and flags of that account', async () => {
		mount(
			account({
				env: [
					{ name: 'LOG_LEVEL', value: 'debug' },
					{ name: 'MCP_TOKEN', value: 'abc' }
				],
				flags: ['--lang=fa']
			})
		);

		await openEditor();

		await expect
			.element(page.getByRole('textbox', { name: 'Variable name' }).nth(0))
			.toHaveValue('LOG_LEVEL');
		await expect.element(page.getByRole('textbox', { name: 'Value' }).nth(0)).toHaveValue('debug');
		await expect
			.element(page.getByRole('textbox', { name: 'Variable name' }).nth(1))
			.toHaveValue('MCP_TOKEN');
		await expect
			.element(page.getByRole('textbox', { name: 'Flag' }).nth(0))
			.toHaveValue('--lang=fa');
	});

	it('adds a variable and a flag and saves them for that account', async () => {
		const tauri = mount(account(), { set_launch_options: () => account() });

		await openEditor();
		await page.getByRole('button', { name: 'Add variable' }).click();
		await page.getByRole('textbox', { name: 'Variable name' }).fill('MCP_TOKEN');
		await page.getByRole('textbox', { name: 'Value' }).fill('s3cret');
		await page.getByRole('button', { name: 'Add flag' }).click();
		await page.getByRole('textbox', { name: 'Flag' }).fill('--enable-logging');
		await page.getByRole('button', { name: 'Save' }).click();

		expect(saves(tauri)).toEqual([
			{
				cmd: 'set_launch_options',
				args: {
					id: 'id-work',
					options: {
						env: [{ name: 'MCP_TOKEN', value: 's3cret' }],
						flags: ['--enable-logging']
					}
				}
			}
		]);
		await expect.element(page.getByRole('dialog')).not.toBeInTheDocument();
	});

	it('edits and removes entries', async () => {
		const tauri = mount(
			account({
				env: [
					{ name: 'A', value: '1' },
					{ name: 'B', value: '2' }
				],
				flags: ['--one', '--two']
			}),
			{ set_launch_options: () => account() }
		);

		await openEditor();
		await page.getByRole('textbox', { name: 'Value' }).nth(1).fill('3');
		await page.getByRole('button', { name: 'Remove variable 1' }).click();
		await page.getByRole('button', { name: 'Remove flag 1' }).click();
		await page.getByRole('button', { name: 'Save' }).click();

		expect(saves(tauri)[0].args).toEqual({
			id: 'id-work',
			options: { env: [{ name: 'B', value: '3' }], flags: ['--two'] }
		});
	});

	it('ignores rows left completely blank', async () => {
		const tauri = mount(account(), { set_launch_options: () => account() });

		await openEditor();
		await page.getByRole('button', { name: 'Add variable' }).click();
		await page.getByRole('button', { name: 'Add flag' }).click();
		await page.getByRole('button', { name: 'Save' }).click();

		expect(saves(tauri)[0].args).toEqual({
			id: 'id-work',
			options: { env: [], flags: [] }
		});
	});

	it('shows why the core refused an invalid name and keeps the editor open', async () => {
		const tauri = mount(account(), {
			set_launch_options: () => {
				throw { code: 'invalid-env-name', message: 'bad' };
			}
		});

		await openEditor();
		await page.getByRole('button', { name: 'Add variable' }).click();
		await page.getByRole('textbox', { name: 'Variable name' }).fill('1BAD');
		await page.getByRole('textbox', { name: 'Value' }).fill('x');
		await page.getByRole('button', { name: 'Save' }).click();

		await expect
			.element(page.getByRole('alert'))
			.toHaveTextContent(/letters, digits and underscore.*not.*digit/i);
		await expect.element(page.getByRole('dialog')).toBeVisible();
		await expect.element(page.getByRole('textbox', { name: 'Variable name' })).toHaveValue('1BAD');
		expect(saves(tauri)).toHaveLength(1);
	});

	it.each([
		['duplicate-env-name', /used twice/i],
		['reserved-flag', /user-data-dir/i],
		['invalid-env-value', /NUL/i],
		['io', /couldn't save/i]
	])('shows a reason for the refusal %s', async (code, text) => {
		mount(account(), {
			set_launch_options: () => {
				throw { code, message: 'x' };
			}
		});

		await openEditor();
		await page.getByRole('button', { name: 'Save' }).click();

		await expect.element(page.getByRole('alert')).toHaveTextContent(text);
	});

	it('changes nothing when cancelled', async () => {
		const tauri = mount(account({ env: [{ name: 'A', value: '1' }] }));

		await openEditor();
		await page.getByRole('textbox', { name: 'Value' }).fill('changed');
		await page.getByRole('button', { name: 'Cancel' }).click();

		expect(saves(tauri)).toHaveLength(0);
		await expect.element(page.getByRole('dialog')).not.toBeInTheDocument();
	});

	it('starts from the saved options again after a cancelled edit', async () => {
		mount(account({ env: [{ name: 'A', value: '1' }] }));

		await openEditor();
		await page.getByRole('textbox', { name: 'Value' }).fill('changed');
		await page.getByRole('button', { name: 'Cancel' }).click();
		await openEditor();

		await expect.element(page.getByRole('textbox', { name: 'Value' })).toHaveValue('1');
	});
});
