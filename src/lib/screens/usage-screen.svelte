<script lang="ts">
	import { onMount, untrack } from 'svelte';
	import { autoRefresh } from '$lib/auto-refresh.svelte';
	import {
		listAccounts,
		onAccountsChanged,
		type AccountList,
		type PooledWindow
	} from '$lib/accounts';
	import MonthCostCard from '$lib/components/month-cost.svelte';
	import UsageHeatmap from '$lib/components/usage-heatmap.svelte';
	import { monthCost, type MonthCost } from '$lib/cost';
	import { heatmap as loadHeatmap, type Heatmap } from '$lib/heatmap';
	import * as Item from '$lib/components/ui/item';
	import { Progress } from '$lib/components/ui/progress';
	import { m } from '$lib/paraglide/messages.js';
	import { formatTag } from '$lib/locale-state.svelte';
	import { formatInteger, formatPercentValue } from '$lib/locale';
	import { formatAgo, formatReset } from '$lib/usage';

	let list = $state<AccountList | null>(null);
	let cost = $state<MonthCost | null>(null);
	let heatmap = $state<Heatmap | null>(null);
	/** The month is a scan of transcripts, so pushed changes reread it at most this often. */
	const COST_REREAD_MS = 60_000;
	let costReadAt = 0;
	/** Ticks every second so "updated X ago" and the refresh wait stay current. */
	let now = $state(Date.now());
	/** Signed-in accounts exist, whether or not they have a reading yet. */
	const hasSignedIn = $derived(
		list?.accounts.some((a) => a.status !== 'signed-out' && a.status !== 'missing') ?? false
	);

	/** Nothing to draw until some day has a reading, or prices are why the dollars are empty. */
	const hasHeatmapData = (map: Heatmap) =>
		map.noEstimate !== null || map.days.some((d) => d.usd !== null);

	async function load() {
		try {
			list = await listAccounts();
		} catch {
			// The core pushes the next snapshot when it can.
		}
	}

	async function loadCost() {
		costReadAt = Date.now();
		// Independent reads of the same transcripts: one failing must not hide the other.
		await Promise.all([
			monthCost().then(
				(next) => (cost = next ?? cost),
				() => {
					// Keep whatever was shown; the next load tries again.
				}
			),
			loadHeatmap().then(
				(next) => (heatmap = next ?? heatmap),
				() => {}
			)
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
		const tick = setInterval(() => (now = Date.now()), 1000);
		const unlisten = onAccountsChanged((next) => {
			list = next;
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

	{#if heatmap && hasHeatmapData(heatmap)}
		<UsageHeatmap {heatmap} />
	{/if}

	{#if list}
		{#if hasSignedIn}
			{@render pooledBar(m.usage_five_hour(), list.pooled.fiveHour)}
			{@render pooledBar(m.usage_weekly(), list.pooled.weekly)}
		{:else}
			<div class="flex flex-col items-center gap-1 py-10 text-center">
				<p class="text-sm font-medium">{m.usage_empty()}</p>
				<p class="text-xs text-muted-foreground">{m.usage_empty_hint()}</p>
			</div>
		{/if}
	{/if}

	{#if cost && cost.accounts.length > 0}
		<MonthCostCard {cost} />
	{/if}
</section>

{#snippet pooledBar(label: string, pooled: PooledWindow)}
	<Item.Root variant="outline" size="sm">
		<Item.Content>
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
				<Progress value={Math.round(100 - pooled.remainingPercent)} aria-label={label} />
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
		</Item.Content>
	</Item.Root>
{/snippet}
