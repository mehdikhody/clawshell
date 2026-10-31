import { page } from 'vitest/browser';
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
	copyBlocker: null,
	running: false,
	email: null,
	usage: null,
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

function mount(handlers: IpcHandlers) {
	const tauri = mockTauri(handlers);
	restore = tauri.restore;
	render(CopySessionsDialog, { account, onclose: () => {} });
	return tauri;
}

describe('Copy sessions dialog', () => {
	it("asks the core for the account's sessions and shows title, folder and last activity", async () => {
		const tauri = mount({ list_sessions: () => [session()] });

		const row = page.getByRole('checkbox', { name: /Fix the build/ });
		await expect.element(row).toBeVisible();
		await expect.element(page.getByRole('dialog')).toHaveTextContent('app');
		await expect.element(page.getByRole('dialog')).toHaveTextContent('3 hours ago');
		expect(tauri.calls.find((c) => c.cmd === 'list_sessions')?.args).toEqual({ id: 'id-work' });
	});

	it('lists the sessions in the order the core gives them and tags only archived ones', async () => {
		mount({
			list_sessions: () => [
				session({ cliSessionId: 'cli-1', title: 'Newest' }),
				session({ cliSessionId: 'cli-2', title: 'Oldest', isArchived: true })
			]
		});

		const rows = page.getByRole('checkbox');
		await expect.element(rows.nth(0)).toHaveAccessibleName(/Newest/);
		await expect.element(rows.nth(1)).toHaveAccessibleName(/Oldest/);
		await expect.element(rows.nth(1)).toHaveAccessibleName(/Archived/);
		await expect.element(rows.nth(0)).not.toHaveAccessibleName(/Archived/);
	});

	it('names a session without a folder or a title', async () => {
		mount({ list_sessions: () => [session({ title: '', cwd: '' })] });

		await expect.element(page.getByRole('dialog')).toHaveTextContent('Untitled session');
		await expect.element(page.getByRole('dialog')).toHaveTextContent('(no folder)');
	});

	it('ticks and unticks sessions and counts the ticked ones', async () => {
		mount({
			list_sessions: () => [
				session({ cliSessionId: 'cli-1', title: 'First' }),
				session({ cliSessionId: 'cli-2', title: 'Second' })
			]
		});
		const first = page.getByRole('checkbox', { name: /First/ });
		const second = page.getByRole('checkbox', { name: /Second/ });

		await first.click();
		await second.click();
		await expect.element(first).toBeChecked();
		await expect.element(second).toBeChecked();
		await expect.element(page.getByRole('dialog')).toHaveTextContent('2 selected');

		await first.click();
		await expect.element(first).not.toBeChecked();
		await expect.element(second).toBeChecked();
		await expect.element(page.getByRole('dialog')).toHaveTextContent('1 selected');
	});

	it('says so when the account has no sessions', async () => {
		mount({ list_sessions: () => [] });

		await expect
			.element(page.getByRole('dialog'))
			.toHaveTextContent('This account has no Code sessions yet.');
		await expect.element(page.getByRole('checkbox')).not.toBeInTheDocument();
	});

	it('shows a clear error, and no rows, when Claude has changed its session layout', async () => {
		mount({
			list_sessions: () => {
				throw { code: 'sessions-layout-changed', message: 'layout' };
			}
		});

		await expect.element(page.getByRole('alert')).toHaveTextContent(/look different/);
		await expect.element(page.getByRole('checkbox')).not.toBeInTheDocument();
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
