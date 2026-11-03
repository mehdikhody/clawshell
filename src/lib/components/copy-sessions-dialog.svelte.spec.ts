import { page, userEvent } from 'vitest/browser';
import { afterEach, describe, expect, it } from 'vitest';
import { render } from 'vitest-browser-svelte';
import '../../routes/layout.css';
import { mockTauri, type IpcHandlers } from '$lib/test/tauri-ipc';
import type { Account } from '$lib/accounts';
import type { CodeSession } from '$lib/sessions';
import CopySessionsDialog from './copy-sessions-dialog.svelte';

let restore: () => void;
afterEach(() => restore?.());

const HOUR_MS = 3_600_000;

const account: Account = {
	id: 'id-work',
	label: 'Work',
	isDefault: false,
	badge: 'W',
	colour: '#2563eb',
	status: 'idle',
	disabled: false,
	copyBlocker: null,
	running: false,
	email: null,
	usage: null,
	usagePending: false,
	env: [],
	flags: []
};

const session = (over: Partial<CodeSession> = {}): CodeSession => ({
	cliSessionId: 'cli-1',
	title: 'Fix the build',
	cwd: 'C:\\work\\app',
	lastActivityAt: Date.now() - 3 * HOUR_MS,
	isArchived: false,
	...over
});

const home: Account = { ...account, id: 'default', label: 'Claude', isDefault: true, badge: 'C' };
const signedOut: Account = {
	...account,
	id: 'id-out',
	label: 'Spare',
	badge: 'S',
	copyBlocker: 'sign-in'
};
const fresh: Account = {
	...account,
	id: 'id-fresh',
	label: 'Fresh',
	badge: 'F',
	copyBlocker: 'open-code-tab'
};
const retired: Account = {
	...account,
	id: 'id-retired',
	label: 'Retired',
	badge: 'R',
	disabled: true,
	copyBlocker: 'disabled'
};

function mount(handlers: IpcHandlers, oncopied: (message: string) => void = () => {}) {
	const tauri = mockTauri(handlers);
	restore = tauri.restore;
	render(CopySessionsDialog, {
		account,
		accounts: [home, account, signedOut, fresh, retired],
		oncopied,
		onclose: () => {}
	});
	return tauri;
}

const sessionsPicker = () => page.getByRole('combobox', { name: /Sessions/ });
const option = (name: string | RegExp) => page.getByRole('option', { name });

/** Opens the sessions picker, ticks (or unticks) each named session, and closes it. */
async function pickSessions(...titles: string[]) {
	await sessionsPicker().click();
	for (const title of titles) await option(new RegExp(title)).click();
	await userEvent.keyboard('{Escape}');
	await expect.element(page.getByRole('listbox')).not.toBeInTheDocument();
}

async function pickTarget(label: string) {
	await page.getByRole('button', { name: /Copy to/ }).click();
	await page.getByRole('menuitemradio', { name: new RegExp(label) }).click();
}

describe('Copy sessions dialog', () => {
	it("asks the core for the account's sessions and shows title, folder and last activity", async () => {
		const tauri = mount({ list_sessions: () => [session()] });

		await sessionsPicker().click();

		const row = option(/Fix the build/);
		await expect.element(row).toBeVisible();
		await expect.element(row).toHaveTextContent('app');
		await expect.element(row).toHaveTextContent('3 hours ago');
		expect(tauri.calls.find((c) => c.cmd === 'list_sessions')?.args).toEqual({ id: 'id-work' });
	});

	it('lists the sessions in the order the core gives them and tags only archived ones', async () => {
		mount({
			list_sessions: () => [
				session({ cliSessionId: 'cli-1', title: 'Newest' }),
				session({ cliSessionId: 'cli-2', title: 'Oldest', isArchived: true })
			]
		});

		await sessionsPicker().click();

		const rows = page.getByRole('option');
		await expect.element(rows.nth(0)).toHaveTextContent(/Newest/);
		await expect.element(rows.nth(1)).toHaveTextContent(/Oldest/);
		await expect.element(rows.nth(1)).toHaveTextContent(/Archived/);
		await expect.element(rows.nth(0)).not.toHaveTextContent(/Archived/);
	});

	it('names a session without a folder or a title', async () => {
		mount({ list_sessions: () => [session({ title: '', cwd: '' })] });

		await sessionsPicker().click();

		await expect.element(option(/Untitled session/)).toHaveTextContent('(no folder)');
	});

	it('ticks and unticks several sessions, and the picker sums them up', async () => {
		mount({
			list_sessions: () => [
				session({ cliSessionId: 'cli-1', title: 'First' }),
				session({ cliSessionId: 'cli-2', title: 'Second' })
			]
		});

		await expect.element(sessionsPicker()).toHaveTextContent('Choose sessions');
		await sessionsPicker().click();
		await option(/First/).click();
		await option(/Second/).click();
		await expect.element(option(/First/)).toHaveAttribute('aria-checked', 'true');
		await expect.element(option(/Second/)).toHaveAttribute('aria-checked', 'true');
		await expect.element(sessionsPicker()).toHaveTextContent('2 selected');

		await option(/First/).click();
		await expect.element(option(/First/)).toHaveAttribute('aria-checked', 'false');
		await expect.element(option(/Second/)).toHaveAttribute('aria-checked', 'true');
		// One ticked session is named.
		await expect.element(sessionsPicker()).toHaveTextContent('Second');
	});

	it('ticks every session with Select all, and shows a partial selection on it', async () => {
		mount({
			list_sessions: () => [
				session({ cliSessionId: 'cli-1', title: 'First' }),
				session({ cliSessionId: 'cli-2', title: 'Second' })
			]
		});
		await sessionsPicker().click();
		const all = page.getByRole('checkbox', { name: /Select all/ });

		await option(/First/).click();
		await expect.element(all).toHaveAttribute('aria-checked', 'mixed');

		await all.click();
		await expect.element(option(/First/)).toHaveAttribute('aria-checked', 'true');
		await expect.element(option(/Second/)).toHaveAttribute('aria-checked', 'true');
		await expect.element(all).toBeChecked();
		await expect.element(sessionsPicker()).toHaveTextContent('2 selected');

		await all.click();
		await expect.element(option(/First/)).toHaveAttribute('aria-checked', 'false');
		await expect.element(option(/Second/)).toHaveAttribute('aria-checked', 'false');
	});

	it('has no search box for five sessions or fewer', async () => {
		mount({
			list_sessions: () =>
				[1, 2, 3, 4, 5].map((n) => session({ cliSessionId: `cli-${n}`, title: `Session ${n}` }))
		});

		await sessionsPicker().click();

		await expect.element(option(/Session 5/)).toBeVisible();
		await expect.element(page.getByPlaceholder('Search sessions…')).not.toBeInTheDocument();
	});

	it('searches by title or folder once there are more than five sessions', async () => {
		mount({
			list_sessions: () => [
				...[1, 2, 3, 4, 5].map((n) => session({ cliSessionId: `cli-${n}`, title: `Session ${n}` })),
				session({ cliSessionId: 'cli-6', title: 'Release notes', cwd: 'C:\\work\\site' })
			]
		});

		await sessionsPicker().click();
		const search = page.getByPlaceholder('Search sessions…');

		await search.fill('release');
		await expect.element(option(/Release notes/)).toBeVisible();
		await expect.element(option(/Session 1/)).not.toBeInTheDocument();

		await search.fill('site');
		await expect.element(option(/Release notes/)).toBeVisible();

		await search.fill('nothing like it');
		await expect.element(page.getByText('No sessions match')).toBeVisible();
	});

	it('offers the other accounts as targets, with the reason beside the ones that cannot receive copies', async () => {
		mount({ list_sessions: () => [session()] });

		await page.getByRole('button', { name: /Copy to/ }).click();

		const options = page.getByRole('menuitemradio');
		await expect.element(options.nth(0)).toHaveTextContent('Claude');
		await expect.element(options.nth(0)).not.toHaveAttribute('aria-disabled', 'true');
		await expect.element(options.nth(1)).toHaveTextContent(/Spare\s*Sign in to this account first/);
		await expect.element(options.nth(1)).toHaveAttribute('aria-disabled', 'true');
		await expect
			.element(options.nth(2))
			.toHaveTextContent(/Fresh\s*Open the Code tab in this account once first/);
		await expect.element(options.nth(2)).toHaveAttribute('aria-disabled', 'true');
		await expect.element(options.nth(3)).toHaveTextContent(/Retired\s*Enable this account first/);
		await expect.element(options.nth(3)).toHaveAttribute('aria-disabled', 'true');
		await expect.element(page.getByRole('menuitemradio', { name: /Work/ })).not.toBeInTheDocument();
	});

	it('keeps Copy disabled until a session and a target are chosen', async () => {
		mount({ list_sessions: () => [session()] });
		const copy = page.getByRole('button', { name: 'Copy', exact: true });

		await expect.element(copy).toBeDisabled();
		await pickSessions('Fix the build');
		await expect.element(copy).toBeDisabled();
		await pickTarget('Claude');
		await expect.element(copy).toBeEnabled();
		await pickSessions('Fix the build');
		await expect.element(copy).toBeDisabled();
	});

	it('copies the ticked sessions into the target and reports how many', async () => {
		const results: string[] = [];
		const tauri = mount(
			{
				list_sessions: () => [
					session({ cliSessionId: 'cli-1', title: 'First' }),
					session({ cliSessionId: 'cli-2', title: 'Second' }),
					session({ cliSessionId: 'cli-3', title: 'Third' })
				],
				copy_sessions: () => ({ copied: 2, targetRunning: false })
			},
			(message) => results.push(message)
		);

		await pickSessions('First', 'Third');
		await pickTarget('Claude');
		await page.getByRole('button', { name: 'Copy', exact: true }).click();

		await expect.poll(() => results).toEqual(['Sessions copied: 2']);
		expect(tauri.calls.find((c) => c.cmd === 'copy_sessions')?.args).toEqual({
			sourceId: 'id-work',
			cliSessionIds: ['cli-1', 'cli-3'],
			targetId: 'default'
		});
	});

	it('asks for a restart of the target when its Claude is running', async () => {
		const results: string[] = [];
		mount(
			{
				list_sessions: () => [session()],
				copy_sessions: () => ({ copied: 1, targetRunning: true })
			},
			(message) => results.push(message)
		);

		await pickSessions('Fix the build');
		await pickTarget('Claude');
		await page.getByRole('button', { name: 'Copy', exact: true }).click();

		await expect.poll(() => results).toEqual(['Sessions copied: 1. Restart Claude to see them.']);
	});

	it('stays open with the reason when the copy fails', async () => {
		const results: string[] = [];
		mount(
			{
				list_sessions: () => [session()],
				copy_sessions: () => {
					throw { code: 'copy-open-code-tab', message: 'folder' };
				}
			},
			(message) => results.push(message)
		);

		await pickSessions('Fix the build');
		await pickTarget('Claude');
		await page.getByRole('button', { name: 'Copy', exact: true }).click();

		await expect
			.element(page.getByRole('alert'))
			.toHaveTextContent('Open the Code tab in this account once first');
		expect(results).toEqual([]);
	});

	it('says so when the account has no sessions', async () => {
		mount({ list_sessions: () => [] });

		await expect
			.element(page.getByRole('dialog'))
			.toHaveTextContent('This account has no Code sessions yet.');
		await expect.element(sessionsPicker()).not.toBeInTheDocument();
	});

	it('shows a clear error, and no picker, when Claude has changed its session layout', async () => {
		mount({
			list_sessions: () => {
				throw { code: 'sessions-layout-changed', message: 'layout' };
			}
		});

		await expect.element(page.getByRole('alert')).toHaveTextContent(/look different/);
		await expect.element(sessionsPicker()).not.toBeInTheDocument();
	});

	it('shows a general error when the sessions cannot be read', async () => {
		mount({
			list_sessions: () => {
				throw { code: 'io', message: 'denied' };
			}
		});

		await expect.element(page.getByRole('alert')).toHaveTextContent("Couldn't read the sessions");
	});
});
