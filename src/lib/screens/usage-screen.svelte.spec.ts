import { page } from 'vitest/browser';
import { afterEach, describe, expect, it, vi } from 'vitest';
import { render } from 'vitest-browser-svelte';
import { emit } from '@tauri-apps/api/event';
import '../../routes/layout.css';
import { mockTauri, type IpcHandlers } from '$lib/test/tauri-ipc';
import { pooledWindow } from '$lib/test/pooled';
import { accountCost, monthCost } from '$lib/test/cost';
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

describe('UsageScreen this month', () => {
	const work = accountCost('Work', { estimateUsd: 1, extraUsd: 12.5, totalUsd: 13.5, share: 0.82 });
	const spare = accountCost('Spare', { estimateUsd: 3, totalUsd: 3, share: 0.18 });
	const month = monthCost({
		estimateUsd: 4,
		extraUsd: 12.5,
		totalUsd: 16.5,
		accounts: [work, spare]
	});

	it('asks the core for the month on load', async () => {
		const tauri = mount(listWith(emptyPooled), { month_cost: () => month });

		await expect.element(page.getByText('This month')).toBeVisible();
		expect(tauri.calls.filter((c) => c.cmd === 'month_cost')).toHaveLength(1);
	});

	it('shows the month total with the Code estimate labelled and extra usage apart', async () => {
		mount(listWith(emptyPooled), { month_cost: () => month });

		await expect.element(page.getByText('$16.50', { exact: true })).toBeVisible();
		await expect.element(page.getByText('$4.00 est.')).toBeVisible();
		await expect.element(page.getByText('Code tab')).toBeVisible();
		await expect.element(page.getByText('Extra usage')).toBeVisible();
		await expect.element(page.getByText('$12.50', { exact: true }).first()).toBeVisible();
	});

	it('lists each account with its dollars and share', async () => {
		mount(listWith(emptyPooled), { month_cost: () => month });

		const rows = page.getByRole('listitem');
		await expect.element(rows.filter({ hasText: 'Work' })).toHaveTextContent('$13.50');
		await expect.element(rows.filter({ hasText: 'Work' })).toHaveTextContent('82%');
		await expect.element(rows.filter({ hasText: 'Spare' })).toHaveTextContent('$3.00');
		await expect.element(rows.filter({ hasText: 'Spare' })).toHaveTextContent('18%');
	});

	it('says there is no estimate, never $0, when prices could not be fetched', async () => {
		mount(listWith(emptyPooled), {
			month_cost: () =>
				monthCost({
					estimateUsd: null,
					noEstimate: 'prices-unavailable',
					totalUsd: null,
					extraUsd: 3,
					accounts: [accountCost('Work', { estimateUsd: null, extraUsd: 3, totalUsd: null })]
				})
		});

		await expect.element(page.getByText('No estimate', { exact: true })).toBeVisible();
		await expect.element(page.getByText(/Couldn't fetch model prices/)).toBeVisible();
		await expect.element(page.getByText('$0.00')).not.toBeInTheDocument();
		// Real extra-usage spend is still shown.
		await expect.element(page.getByText('$3.00').first()).toBeVisible();
	});

	it('notes messages left out for lack of a price', async () => {
		mount(listWith(emptyPooled), { month_cost: () => ({ ...month, unpricedMessages: 7 }) });

		await expect.element(page.getByText(/7 messages/)).toBeVisible();
	});

	it('shows a zero month as $0.00 when there is simply no spend', async () => {
		mount(listWith(emptyPooled), {
			month_cost: () => monthCost({ accounts: [accountCost('Work')] })
		});

		await expect.element(page.getByText('$0.00', { exact: true }).first()).toBeVisible();
		await expect.element(page.getByText('No estimate')).not.toBeInTheDocument();
	});

	it('does not rescan the transcripts on every pushed change', async () => {
		const tauri = mount(listWith(emptyPooled), { month_cost: () => month });
		await expect.element(page.getByText('$16.50', { exact: true })).toBeVisible();

		await emit('accounts-changed', listWith(emptyPooled));
		await emit('accounts-changed', listWith(emptyPooled));

		expect(tauri.calls.filter((c) => c.cmd === 'month_cost')).toHaveLength(1);
	});

	it('reads the month again after a manual refresh', async () => {
		const tauri = mount(listWith(emptyPooled), {
			month_cost: () => month,
			refresh_usage: () => ({ refreshed: 1, refreshAllowedAt: Date.now() + 3 * MIN })
		});
		await expect.element(page.getByText('$16.50', { exact: true })).toBeVisible();

		await page.getByRole('button', { name: 'Refresh usage' }).click();

		await vi.waitFor(() =>
			expect(tauri.calls.filter((c) => c.cmd === 'month_cost')).toHaveLength(2)
		);
	});

	it('shows no cost section when the core cannot report one', async () => {
		mount(listWith(emptyPooled), {
			month_cost: () => {
				throw new Error('boom');
			}
		});

		await expect.element(page.getByText('No usage data yet').first()).toBeVisible();
		await expect.element(page.getByText('This month')).not.toBeInTheDocument();
	});
});
