import { page } from 'vitest/browser';
import { afterEach, describe, expect, it } from 'vitest';
import { render } from 'vitest-browser-svelte';
import { emit } from '@tauri-apps/api/event';
import '../../routes/layout.css';
import { mockTauri, type IpcHandlers } from '$lib/test/tauri-ipc';
import { pooledWindow } from '$lib/test/pooled';
import { emptyPooled, type AccountList, type PooledRemaining } from '$lib/accounts';
import UsageScreen from './usage-screen.svelte';

let restore: () => void;
afterEach(() => restore?.());

const MIN = 60_000;

const account = (id: string, status: string) => ({
	id,
	label: id,
	status,
	running: false,
	email: null,
	usage: null
});

const listWith = (
	pooled: PooledRemaining,
	statuses: string[] = ['idle'],
	extra: Partial<AccountList> = {}
) =>
	({
		claudeFound: true,
		refreshAllowedAt: null,
		pooled,
		accounts: statuses.map((s, i) => account(`acct-${i}`, s)),
		...extra
	}) as unknown as AccountList;

function mount(list: AccountList, overrides: IpcHandlers = {}) {
	const tauri = mockTauri({ list_accounts: () => list, ...overrides });
	restore = tauri.restore;
	render(UsageScreen);
	return tauri;
}

// Local-time reset a few hours ahead so the formatted clock time is predictable.
const soon = new Date();
soon.setHours(soon.getHours() + 2, 0, 0, 0);
const SOON_ISO = soon.toISOString();

describe('UsageScreen pooled bars', () => {
	it('shows the average remaining percentage for each window', async () => {
		mount(
			listWith(
				{
					fiveHour: pooledWindow({ remainingPercent: 80, available: 3, total: 3 }),
					weekly: pooledWindow({ remainingPercent: 55.4, available: 2, total: 3 })
				},
				['idle', 'idle', 'idle']
			)
		);

		const five = page.getByRole('progressbar', { name: '5-hour' });
		await expect.element(five).toHaveAttribute('aria-valuenow', '80');
		await expect.element(page.getByText('80%', { exact: true })).toBeVisible();
		const weekly = page.getByRole('progressbar', { name: 'Weekly' });
		await expect.element(weekly).toHaveAttribute('aria-valuenow', '55');
		await expect.element(page.getByText('55%', { exact: true })).toBeVisible();
	});

	it('captions each bar with how many accounts are available', async () => {
		mount(
			listWith(
				{
					fiveHour: pooledWindow({ remainingPercent: 50, available: 2, total: 3 }),
					weekly: pooledWindow({ remainingPercent: 50, available: 1, total: 3 })
				},
				['idle', 'idle', 'limited']
			)
		);

		await expect.element(page.getByText('2 of 3 accounts available')).toBeVisible();
		await expect.element(page.getByText('1 of 3 accounts available')).toBeVisible();
	});

	it('shows the soonest reset time for each window', async () => {
		mount(
			listWith({
				fiveHour: pooledWindow({
					remainingPercent: 50,
					available: 1,
					total: 1,
					soonestReset: SOON_ISO
				}),
				weekly: pooledWindow({ remainingPercent: 50, available: 1, total: 1 })
			})
		);

		const expected = new Intl.DateTimeFormat('en', { hour: 'numeric', minute: '2-digit' }).format(
			soon
		);
		await expect.element(page.getByText(`Resets ${expected}`)).toBeVisible();
		// The weekly window has no reset time, so only one reset line appears.
		expect(page.getByText(/^Resets /).elements()).toHaveLength(1);
	});

	it('updates when the core reports a changed pool', async () => {
		mount(
			listWith({
				fiveHour: pooledWindow({ remainingPercent: 80, available: 3, total: 3 }),
				weekly: pooledWindow({ remainingPercent: 80, available: 3, total: 3 })
			})
		);
		await expect.element(page.getByText('3 of 3 accounts available').first()).toBeVisible();

		await emit(
			'accounts-changed',
			listWith({
				fiveHour: pooledWindow({ remainingPercent: 33, available: 1, total: 2 }),
				weekly: pooledWindow({ remainingPercent: 90, available: 2, total: 2 })
			})
		);

		await expect
			.element(page.getByRole('progressbar', { name: '5-hour' }))
			.toHaveAttribute('aria-valuenow', '33');
		await expect.element(page.getByText('1 of 2 accounts available')).toBeVisible();
	});

	it('marks the pool when any input is stale', async () => {
		mount(
			listWith({
				fiveHour: pooledWindow({
					remainingPercent: 80,
					available: 1,
					total: 1,
					staleSinceMs: Date.now() - 10 * MIN
				}),
				weekly: pooledWindow({ remainingPercent: 80, available: 1, total: 1 })
			})
		);

		await expect
			.element(page.getByText(/Includes stale data, updated 10 minutes ago/))
			.toBeVisible();
		expect(page.getByText(/Includes stale data/).elements()).toHaveLength(1);
	});

	it('says so when no account is signed in', async () => {
		mount(listWith(emptyPooled, ['signed-out', 'signed-out']));

		await expect.element(page.getByText('No signed-in accounts')).toBeVisible();
		await expect.element(page.getByRole('progressbar')).not.toBeInTheDocument();
	});

	it('says so when there are no accounts at all', async () => {
		mount(listWith(emptyPooled, []));

		await expect.element(page.getByText('No signed-in accounts')).toBeVisible();
	});

	it('says there is no data yet when accounts are signed in but unread', async () => {
		mount(listWith(emptyPooled, ['idle']));

		await expect.element(page.getByText('No usage data yet').first()).toBeVisible();
		await expect.element(page.getByText('No signed-in accounts')).not.toBeInTheDocument();
	});
});

describe('UsageScreen manual refresh', () => {
	it('asks the core to refresh when the button is pressed', async () => {
		const tauri = mount(listWith(emptyPooled), {
			refresh_usage: () => ({ refreshed: 1, refreshAllowedAt: Date.now() + 3 * MIN })
		});

		await page.getByRole('button', { name: 'Refresh usage' }).click();

		expect(tauri.calls.filter((c) => c.cmd === 'refresh_usage')).toHaveLength(1);
	});

	it('is disabled inside the minimum interval and says when the next refresh is allowed', async () => {
		const tauri = mount(
			listWith(emptyPooled, ['idle'], { refreshAllowedAt: Date.now() + 151_500 })
		);

		await expect.element(page.getByRole('button', { name: 'Refresh usage' })).toBeDisabled();
		await expect.element(page.getByText(/Next refresh allowed in 2:3\d/)).toBeVisible();
		expect(tauri.calls.some((c) => c.cmd === 'refresh_usage')).toBe(false);
	});
});
