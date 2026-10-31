import { page } from 'vitest/browser';
import { formatReset } from '$lib/usage';
import { afterEach, describe, expect, it, vi } from 'vitest';
import { render } from 'vitest-browser-svelte';
import { emit } from '@tauri-apps/api/event';
import '../../routes/layout.css';
import { mockTauri, type IpcHandlers } from '$lib/test/tauri-ipc';
import { pooledWindow } from '$lib/test/pooled';
import { accountCost, monthCost } from '$lib/test/cost';
import { heatmapOf, values } from '$lib/test/heatmap';
import { emptyPooled, type AccountList, type PooledRemaining } from '$lib/accounts';
import { switchLocale } from '$lib/locale-state.svelte';
import { intlLocale } from '$lib/locale';
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
	usage: null,
	env: [],
	flags: []
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
	it('fills each bar with how much of the window is used', async () => {
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
		await expect.element(five).toHaveAttribute('aria-valuenow', '20');
		await expect.element(page.getByText('20%', { exact: true })).toBeVisible();
		const weekly = page.getByRole('progressbar', { name: 'Weekly' });
		await expect.element(weekly).toHaveAttribute('aria-valuenow', '45');
		await expect.element(page.getByText('45%', { exact: true })).toBeVisible();
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

		const expected = formatReset(SOON_ISO, new Date(), 'en');
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
			.toHaveAttribute('aria-valuenow', '67');
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
		await expect.element(page.getByText('$4.00', { exact: true })).toBeVisible();
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

describe('UsageScreen heatmap', () => {
	const MAP = heatmapOf({
		'2026-06-10': {
			usd: values([
				['Work', 6],
				['Spare', 2],
				['Idle', 0]
			]),
			tokens: values([
				['Work', 1_000_000],
				['Spare', 234_567],
				['Idle', 0]
			])
		},
		'2026-06-11': { usd: values([['Work', 0]]), tokens: values([['Work', 0]]) },
		'2026-06-12': { usd: values([['Work', 1]]) }
	});

	const withMap = (map = MAP) => mount(listWith(emptyPooled), { heatmap: () => map });
	const grid = () => page.getByRole('group', { name: 'Last 9 weeks' });
	const tooltip = () => page.getByTestId('heatmap-tooltip');
	const cell = (name: RegExp) => grid().getByRole('button', { name });

	it('asks the core for the heatmap on load and draws each day of the 9 weeks', async () => {
		const tauri = withMap();

		await expect.element(grid()).toBeVisible();
		await expect.element(page.getByText('Last 9 weeks', { exact: true })).toBeVisible();
		expect(grid().getByRole('button').elements()).toHaveLength(58);
		expect(tauri.calls.filter((c) => c.cmd === 'heatmap')).toHaveLength(1);
	});

	it('draws the rest of this week as faint slots that are not days', async () => {
		withMap();
		await expect.element(grid()).toBeVisible();

		// Today is Monday 2026-06-15: Tuesday to Saturday are still to come.
		const slots = grid().getByTestId('heatmap-future').elements() as HTMLElement[];
		expect(slots).toHaveLength(5);
		for (const slot of slots) {
			expect(slot.getAttribute('aria-hidden')).toBe('true');
			expect(slot.tabIndex).toBe(-1);
			expect(getComputedStyle(slot).borderStyle).not.toBe('dashed');
			expect(slot.getBoundingClientRect().width).toBe(
				cell(/Jun 15/)
					.element()
					.getBoundingClientRect().width
			);
		}
	});

	it('starts each week on Sunday, with no blank slot before the first day', async () => {
		withMap();
		await expect.element(grid()).toBeVisible();

		// The grid's children: seven weekday labels, then the cells. Sunday comes first.
		const children = [...grid().element().children];
		const weekday = (day: number) =>
			new Intl.DateTimeFormat('en', { weekday: 'narrow' }).format(new Date(2024, 0, day));
		expect(children[0].textContent).toBe(weekday(7));
		expect(children[6].textContent).toBe(weekday(13));
		expect(children[7].getAttribute('data-testid')).not.toBe('heatmap-future');
	});

	it('labels each day with its Code estimate, and has no measure switch', async () => {
		withMap();

		await expect.element(cell(/Jun 10/)).toHaveAccessibleName(/$8.00/);
		await expect.element(cell(/Jun 12/)).toHaveAccessibleName(/$1.00/);
		await expect.element(page.getByRole('radio', { name: '%' })).not.toBeInTheDocument();
	});

	it('shows the date, the total, the tokens and each account on hover', async () => {
		withMap();

		await cell(/Jun 10/).hover();

		const tip = tooltip();
		await expect.element(tip).toBeVisible();
		await expect.element(tip).toHaveTextContent('Jun 10');
		await expect.element(tip).toHaveTextContent('$8.00');
		await expect.element(tip).toHaveTextContent('1.2M tokens');
		await expect.element(tip).toHaveTextContent(/Work\s*1M · \$6\.00/);
		await expect.element(tip).toHaveTextContent(/Spare\s*234\.6K · \$2\.00/);
	});

	it('divides the totals from the accounts in the tooltip', async () => {
		withMap();

		await cell(/Jun 10/).hover();

		await expect.element(tooltip().getByRole('separator')).toBeVisible();
	});

	it('leaves an account that used no tokens that day out of the tooltip', async () => {
		withMap();

		await cell(/Jun 10/).hover();

		await expect.element(tooltip()).toHaveTextContent(/Spare/);
		await expect.element(tooltip()).not.toHaveTextContent(/Idle/);
	});

	it('shortens big dollar and token numbers in the tooltip, but not in the label', async () => {
		withMap(
			heatmapOf({
				'2026-06-10': {
					usd: values([
						['Work', 12_000],
						['Spare', 345]
					]),
					tokens: values([
						['Work', 4_900_000_000],
						['Spare', 100_000_000]
					])
				}
			})
		);

		await expect.element(cell(/Jun 10/)).toHaveAccessibleName(/\$12,345\.00/);
		await cell(/Jun 10/).hover();

		const tip = tooltip();
		await expect.element(tip).toHaveTextContent('$12.3K');
		await expect.element(tip).toHaveTextContent('5B tokens');
		await expect.element(tip).toHaveTextContent(/Work\s*4\.9B · \$12K/);
		await expect.element(tip).toHaveTextContent(/Spare\s*100M · \$345\.00/);
	});

	it('shows a tooltip on keyboard focus, so a tap works too', async () => {
		withMap();

		await expect.element(cell(/Jun 10/)).toBeVisible();
		(cell(/Jun 10/).element() as HTMLElement).focus();

		await expect.element(tooltip()).toHaveTextContent('$8.00');
	});

	it('opens the tooltip on a click, for taps', async () => {
		withMap();

		await cell(/Jun 10/).click();

		await expect.element(tooltip()).toHaveTextContent(/Spare\s*234\.6K · \$2\.00/);
	});

	it('draws a day with no data differently from a day at zero', async () => {
		withMap();
		const border = (name: RegExp) => getComputedStyle(cell(name).element()).borderStyle;

		await expect.element(cell(/Jun 11/)).toHaveAccessibleName(/$0.00/);
		await expect.element(cell(/Jun 9:/)).toHaveAccessibleName(/no data/i);
		expect(border(/Jun 9:/)).toBe('dashed');
		expect(border(/Jun 11/)).not.toBe('dashed');

		await cell(/Jun 9:/).hover();
		await expect.element(tooltip()).toHaveTextContent('No data');
	});

	it('fits the 400px window without scrolling sideways', async () => {
		await page.viewport(400, 780);
		withMap();
		await expect.element(grid()).toBeVisible();

		const el = grid().element() as HTMLElement;
		expect(el.scrollWidth).toBeLessThanOrEqual(el.clientWidth);
		expect(el.getBoundingClientRect().right).toBeLessThanOrEqual(400);
		expect(document.documentElement.scrollWidth).toBeLessThanOrEqual(400);
	});

	it('says why the dollars are empty when prices could not be fetched, and still shows tokens', async () => {
		withMap(
			heatmapOf(
				{ '2026-06-10': { tokens: values([['Work', 1_000]]) } },
				{
					noEstimate: 'prices-unavailable'
				}
			)
		);

		await expect.element(page.getByText(/no dollar estimate/i)).toBeVisible();
		await cell(/Jun 10/).hover();
		await expect.element(tooltip()).toHaveTextContent('1K tokens');
		await expect.element(tooltip()).toHaveTextContent(/Work\s*1K$/);
	});

	it('shows no heatmap when there is nothing to show', async () => {
		withMap(heatmapOf());

		await expect.element(page.getByText('No usage data yet').first()).toBeVisible();
		await expect.element(grid()).not.toBeInTheDocument();
	});
});

describe('UsageScreen in Persian', () => {
	afterEach(() => switchLocale('en'));

	it('translates the screen and writes percentages and counts in Persian digits', async () => {
		switchLocale('fa');
		mount(
			listWith(
				{
					fiveHour: pooledWindow({ remainingPercent: 80, available: 2, total: 3 }),
					weekly: pooledWindow({ remainingPercent: 55.4, available: 2, total: 3 })
				},
				['idle', 'idle', 'limited']
			)
		);

		await expect.element(page.getByRole('heading', { name: 'مصرف' })).toBeVisible();
		await expect.element(page.getByText('۲۰٪', { exact: true })).toBeVisible();
		await expect.element(page.getByText('۴۵٪', { exact: true })).toBeVisible();
		await expect.element(page.getByText('۲ از ۳ حساب در دسترس').first()).toBeVisible();
	});

	it('writes the month in Persian digits', async () => {
		switchLocale('fa');
		const work = accountCost('Work', {
			estimateUsd: 1,
			extraUsd: 12.5,
			totalUsd: 13.5,
			share: 0.82
		});
		mount(listWith(emptyPooled), {
			month_cost: () =>
				monthCost({ estimateUsd: 1, extraUsd: 12.5, totalUsd: 13.5, accounts: [work] })
		});

		await expect.element(page.getByText('این ماه')).toBeVisible();
		await expect.element(page.getByText('$۱۳٫۵۰', { exact: false }).first()).toBeVisible();
		await expect.element(page.getByText('۸۲٪')).toBeVisible();
	});

	it('lays the heatmap tooltip out right to left', async () => {
		switchLocale('fa');
		mount(listWith(emptyPooled), {
			heatmap: () =>
				heatmapOf({ '2026-06-10': { usd: values([['Work', 1]]), tokens: values([['Work', 10]]) } })
		});
		const day = page.getByRole('group', { name: '۹ هفته گذشته' }).getByRole('button').nth(52);

		await day.hover();

		const tip = page.getByTestId('heatmap-tooltip');
		await expect.element(tip).toBeVisible();
		expect(getComputedStyle(tip.element()).direction).toBe('rtl');
	});

	it('starts each heatmap week on Saturday, with a blank slot before the first day', async () => {
		switchLocale('fa');
		mount(listWith(emptyPooled), {
			heatmap: () => heatmapOf({ '2026-04-19': { usd: values([['Work', 1]]) } })
		});
		const grid = page.getByRole('group', { name: '۹ هفته گذشته' });
		await expect.element(grid).toBeVisible();

		const children = [...grid.element().children];
		const saturday = new Intl.DateTimeFormat(intlLocale('fa'), { weekday: 'narrow' }).format(
			new Date(2024, 0, 6)
		);
		expect(children[0].textContent).toBe(saturday);
		// The first day, 19 April 2026 (۳۰ فروردین), is a Sunday: one row down from Saturday, so
		// the blank slot comes before it in the first column.
		expect(children[7].getAttribute('data-testid')).toBe('heatmap-future');
		expect(children[8].getAttribute('aria-label')).toMatch(/^۳۰ فروردین/);
	});

	it('fills a bar from the right edge', async () => {
		switchLocale('fa');
		mount(
			listWith({
				fiveHour: pooledWindow({ remainingPercent: 75, available: 1, total: 1 }),
				weekly: pooledWindow({ remainingPercent: 75, available: 1, total: 1 })
			})
		);

		const bar = page.getByRole('progressbar', { name: '۵ ساعته' });
		await expect.element(bar).toBeVisible();
		const track = bar.element().getBoundingClientRect();
		const fill = bar
			.element()
			.querySelector('[data-slot=progress-indicator]')!
			.getBoundingClientRect();
		// The filled quarter is the right-hand quarter: the fill box is pushed right by the empty 75%.
		expect(Math.round(fill.left - track.left)).toBe(Math.round(track.width * 0.75));
	});

	it('keeps the layout in right-to-left order', async () => {
		switchLocale('fa');
		mount(
			listWith({
				fiveHour: pooledWindow({ remainingPercent: 50, available: 1, total: 1 }),
				weekly: pooledWindow({ remainingPercent: 50, available: 1, total: 1 })
			})
		);

		const title = page.getByText('۵ ساعته').first();
		const percent = page.getByText('۵۰٪').first();
		await expect.element(title).toBeVisible();
		// The title starts on the right and the figure ends on the left.
		expect(title.element().getBoundingClientRect().left).toBeGreaterThan(
			percent.element().getBoundingClientRect().left
		);
	});
});
