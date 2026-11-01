<script lang="ts">
	import { onMount, untrack } from 'svelte';
	import { autoRefresh } from '$lib/auto-refresh.svelte';
	import { useScreenData } from '$lib/screen-data.svelte';
	import { listAccounts, onAccountsChanged, type PooledWindow } from '$lib/accounts';
	import MonthCostCard from '$lib/components/month-cost.svelte';
	import UsageHeatmap from '$lib/components/usage-heatmap.svelte';
	import { monthCost } from '$lib/cost';
	import { heatmap as loadHeatmap, type Heatmap } from '$lib/heatmap';
	import * as Item from '$lib/components/ui/item';
	import { Progress } from '$lib/components/ui/progress';
	import { Separator } from '$lib/components/ui/separator';
	import { Skeleton } from '$lib/components/ui/skeleton';
	import { getSettings } from '$lib/settings';
	import { m } from '$lib/paraglide/messages.js';
	import { formatTag } from '$lib/locale-state.svelte';
	import { formatInteger, formatPercentValue } from '$lib/locale';
	import { formatAgo, formatReset } from '$lib/usage';
	import {
		defaultThresholds,
		usageBarClass,
		usageStage,
		type UsageThresholds
	} from '$lib/usage-stage';

	/**
	 * The sections' data, and whether each has had its first answer or failed. Until then it shows
	 * a skeleton; refreshes, and a language or calendar change that remounts the screen, never
	 * bring one back.
	 */
	const shown = useScreenData().usage;
	/** The warning and limit thresholds that colour the bars; the defaults until the saved ones load. */
	let thresholds = $state<UsageThresholds>(defaultThresholds);
	/** The month is a scan of transcripts, so pushed changes reread it at most this often. */
	const COST_REREAD_MS = 60_000;
	let costReadAt = 0;
	/** Ticks every second so "updated X ago" and the refresh wait stay current. */
	let now = $state(Date.now());
	/** Signed-in accounts exist, whether or not they have a reading yet. */
	const hasSignedIn = $derived(
		shown.list?.accounts.some((a) => a.status !== 'signed-out' && a.status !== 'missing') ?? false
	);

	/** Nothing to draw until some day has a reading, or prices are why the dollars are empty. */
	const hasHeatmapData = (map: Heatmap) =>
		map.noEstimate !== null || map.days.some((d) => d.usd !== null);

	async function loadThresholds() {
		try {
			const saved = await getSettings();
			if (saved) thresholds = saved;
		} catch {
			// Keep the defaults.
		}
	}

	async function load() {
		try {
			shown.list = await listAccounts();
		} catch {
			// The core pushes the next snapshot when it can.
		} finally {
			shown.listReady = true;
		}
	}

	async function loadCost() {
		costReadAt = Date.now();
		// Independent reads of the same transcripts: one failing must not hide the other.
		await Promise.all([
			monthCost()
				.then(
					(next) => (shown.cost = next ?? shown.cost),
					() => {
						// Keep whatever was shown; the next load tries again.
					}
				)
				.finally(() => (shown.costReady = true)),
			loadHeatmap()
				.then(
					(next) => (shown.heatmap = next ?? shown.heatmap),
					() => {}
				)
				.finally(() => (shown.heatmapReady = true))
		]);
	}

	$effect(() => {
		void autoRefresh.tick;
		untrack(() => {
			void load();
			void loadCost();
		});
	});

	onMount(() => {
		void loadThresholds();
		const tick = setInterval(() => (now = Date.now()), 1000);
		const unlisten = onAccountsChanged((next) => {
			shown.list = next;
			shown.listReady = true;
			if (Date.now() - costReadAt >= COST_REREAD_MS) void loadCost();
		});
		return () => {
			clearInterval(tick);
			void unlisten.then((stop) => stop()).catch(() => {});
		};
	});
</script>

<section class="flex flex-col gap-5 p-4">
	<header class="flex items-center justify-between">
		<h1 class="text-base font-semibold">{m.tab_usage()}</h1>
	</header>

	{#if !shown.heatmapReady}
		<Item.Root data-testid="skeleton-heatmap" variant="outline" size="sm" aria-busy="true">
			<Item.Content>
				<Skeleton class="h-4 w-24" />
				<!-- Same grid as the heatmap: 7 rows, 9 weeks of 1.75rem cells. -->
				<div
					class="mx-auto grid w-max auto-cols-[1.75rem] grid-flow-col grid-rows-[repeat(7,1.75rem)] gap-1 py-1"
				>
					{#each { length: 63 } as _, i (i)}
						<Skeleton data-testid="skeleton-day" class="size-7 rounded-sm" />
					{/each}
				</div>
				<Skeleton class="ms-auto h-3 w-40" />
			</Item.Content>
		</Item.Root>
	{:else if shown.heatmap && hasHeatmapData(shown.heatmap)}
		<UsageHeatmap heatmap={shown.heatmap} />
	{/if}

	{#if !shown.listReady}
		<Item.Root data-testid="skeleton-pooled" variant="outline" size="sm" aria-busy="true">
			<Item.Content>
				{#each [0, 1] as n (n)}
					{#if n > 0}
						<Item.Separator />
					{/if}
					<div class="flex flex-col gap-1">
						<div class="flex items-baseline justify-between">
							<Skeleton class="h-4 w-16" />
							<Skeleton class="h-6 w-12" />
						</div>
						<Skeleton data-testid="skeleton-bar" class="h-2 w-full rounded-full" />
						<div class="flex items-center justify-between">
							<Skeleton class="h-3 w-32" />
							<Skeleton class="h-3 w-24" />
						</div>
					</div>
				{/each}
			</Item.Content>
		</Item.Root>
	{:else if shown.list}
		{#if hasSignedIn}
			<Item.Root variant="outline" size="sm">
				<Item.Content>
					{@render pooledWindow(m.usage_five_hour(), shown.list.pooled.fiveHour)}
					<Item.Separator />
					{@render pooledWindow(m.usage_weekly(), shown.list.pooled.weekly)}
				</Item.Content>
			</Item.Root>
		{:else}
			<div class="flex flex-col items-center gap-1 py-10 text-center">
				<p class="text-sm font-medium">{m.usage_empty()}</p>
				<p class="text-xs text-muted-foreground">{m.usage_empty_hint()}</p>
			</div>
		{/if}
	{/if}

	{#if !shown.costReady}
		<Item.Root data-testid="skeleton-cost" variant="outline" size="sm" aria-busy="true">
			<Item.Content>
				<div class="flex items-baseline justify-between">
					<Skeleton class="h-4 w-24" />
					<Skeleton class="h-6 w-16" />
				</div>
				<div class="flex flex-col gap-1.5">
					<Skeleton class="h-3 w-full" />
					<Skeleton class="h-3 w-full" />
					<Skeleton class="h-3 w-full" />
				</div>
				<Separator class="my-2" />
				<div class="flex flex-col gap-4">
					{#each [0, 1] as n (n)}
						<div data-testid="skeleton-account-row" class="flex flex-col gap-2">
							<div class="flex items-baseline justify-between">
								<Skeleton class="h-3 w-24" />
								<Skeleton class="h-3 w-20" />
							</div>
							<Skeleton class="h-2 w-full rounded-full" />
						</div>
					{/each}
				</div>
			</Item.Content>
		</Item.Root>
	{:else if shown.cost && shown.cost.accounts.length > 0}
		<MonthCostCard cost={shown.cost} />
	{/if}
</section>

{#snippet pooledWindow(label: string, pooled: PooledWindow)}
	<div class="flex flex-col gap-1">
		<div class="flex items-baseline justify-between">
			<Item.Title>{label}</Item.Title>
			{#if pooled.remainingPercent !== null}
				<span class="text-lg font-semibold tabular-nums">
					{formatPercentValue(100 - pooled.remainingPercent, formatTag())}
				</span>
			{/if}
		</div>
		{#if pooled.remainingPercent === null}
			<p class="text-xs text-muted-foreground">{m.usage_none()}</p>
		{:else}
			<!-- Fills with use, like Claude's own usage meters. -->
			<Progress
				value={Math.round(100 - pooled.remainingPercent)}
				aria-label={label}
				class={usageBarClass[usageStage(100 - pooled.remainingPercent, thresholds)]}
			/>
			<div class="flex items-center justify-between text-xs text-muted-foreground">
				<span
					>{m.usage_available({
						available: formatInteger(pooled.available, formatTag()),
						total: formatInteger(pooled.total, formatTag())
					})}</span
				>
				{#if pooled.soonestReset}
					<span>
						{m.usage_soonest_reset({
							time: formatReset(pooled.soonestReset, new Date(now), formatTag())
						})}
					</span>
				{/if}
			</div>
			{#if pooled.staleSinceMs !== null}
				<p class="text-[11px] text-muted-foreground">
					{m.usage_pooled_stale({ ago: formatAgo(pooled.staleSinceMs, now, formatTag()) })}
				</p>
			{/if}
		{/if}
	</div>
{/snippet}
