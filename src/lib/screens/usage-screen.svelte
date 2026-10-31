<script lang="ts">
	import { onMount } from 'svelte';
	import IconAlertTriangle from '@tabler/icons-svelte/icons/alert-triangle';
	import IconRefresh from '@tabler/icons-svelte/icons/refresh';
	import {
		listAccounts,
		onAccountsChanged,
		refreshUsage,
		type AccountList,
		type PooledWindow
	} from '$lib/accounts';
	import MonthCostCard from '$lib/components/month-cost.svelte';
	import UsageHeatmap from '$lib/components/usage-heatmap.svelte';
	import { monthCost, type MonthCost } from '$lib/cost';
	import { heatmap as loadHeatmap, type Heatmap } from '$lib/heatmap';
	import * as Alert from '$lib/components/ui/alert';
	import { Button } from '$lib/components/ui/button';
	import * as Item from '$lib/components/ui/item';
	import { Progress } from '$lib/components/ui/progress';
	import { m } from '$lib/paraglide/messages.js';
	import { formatTag } from '$lib/locale-state.svelte';
	import { formatInteger, formatPercentValue } from '$lib/locale';
	import { formatAgo, formatCountdown, formatReset } from '$lib/usage';

	let list = $state<AccountList | null>(null);
	let cost = $state<MonthCost | null>(null);
	let heatmap = $state<Heatmap | null>(null);
	/** The month is a scan of transcripts, so pushed changes reread it at most this often. */
	const COST_REREAD_MS = 60_000;
	let costReadAt = 0;
	let refreshing = $state(false);
	let failure = $state<string | null>(null);
	/** Ticks every second so "updated X ago" and the refresh wait stay current. */
	let now = $state(Date.now());
	const refreshWaitMs = $derived(Math.max(0, (list?.refreshAllowedAt ?? 0) - now));
	/** Signed-in accounts exist, whether or not they have a reading yet. */
	const hasSignedIn = $derived(
		list?.accounts.some((a) => a.status !== 'signed-out' && a.status !== 'missing') ?? false
	);

	/** Nothing to draw until some day has a reading, or prices are why the dollars are empty. */
	const hasHeatmapData = (map: Heatmap) =>
		map.noEstimate !== null || map.days.some((d) => d.usd !== null || d.percent !== null);

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

	async function refreshNow() {
		refreshing = true;
		failure = null;
		try {
			const outcome = await refreshUsage();
			if (list) list = { ...list, refreshAllowedAt: outcome.refreshAllowedAt };
		} catch {
			failure = m.error_refresh_failed();
		} finally {
			refreshing = false;
			await Promise.all([load(), loadCost()]);
		}
	}

	onMount(() => {
		const tick = setInterval(() => (now = Date.now()), 1000);
		void load();
		void loadCost();
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

<section class="flex flex-col gap-3 p-4">
	<header class="flex items-center justify-between">
		<h1 class="text-base font-semibold">{m.tab_usage()}</h1>
		<Button
			size="icon-sm"
			variant="outline"
			aria-label={m.refresh_usage()}
			onclick={refreshNow}
			disabled={refreshing || refreshWaitMs > 0}
		>
			<IconRefresh class={refreshing ? 'animate-spin' : ''} />
		</Button>
	</header>

	{#if refreshWaitMs > 0}
		<p class="text-xs text-muted-foreground">
			{m.refresh_next({ time: formatCountdown(refreshWaitMs, formatTag()) })}
		</p>
	{/if}

	{#if failure}
		<Alert.Root variant="destructive">
			<IconAlertTriangle />
			<Alert.Title>{failure}</Alert.Title>
		</Alert.Root>
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

	{#if heatmap && hasHeatmapData(heatmap)}
		<UsageHeatmap {heatmap} />
	{/if}
</section>

{#snippet pooledBar(label: string, pooled: PooledWindow)}
	<Item.Root variant="outline" size="sm">
		<Item.Content>
			<div class="flex items-baseline justify-between">
				<Item.Title>{label}</Item.Title>
				{#if pooled.remainingPercent !== null}
					<span class="text-lg font-semibold tabular-nums">
						{formatPercentValue(pooled.remainingPercent, formatTag())}
					</span>
				{/if}
			</div>
			{#if pooled.remainingPercent === null}
				<p class="text-xs text-muted-foreground">{m.usage_none()}</p>
			{:else}
				<Progress value={Math.round(pooled.remainingPercent)} aria-label={label} />
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
