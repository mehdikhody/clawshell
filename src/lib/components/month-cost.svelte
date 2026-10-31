<script lang="ts">
	import { formatCountShort, formatUsdShort, type AccountCost, type MonthCost } from '$lib/cost';
	import * as Item from '$lib/components/ui/item';
	import { Progress } from '$lib/components/ui/progress';
	import { Separator } from '$lib/components/ui/separator';
	import { m } from '$lib/paraglide/messages.js';
	import { formatInteger, formatPercentValue } from '$lib/locale';
	import { formatTag } from '$lib/locale-state.svelte';

	let { cost }: { cost: MonthCost } = $props();

	const usd = (amount: number) => formatUsdShort(amount, formatTag());

	/** An account's dollars; none when it has no estimate and no extra usage either. */
	const dollarsOf = (a: AccountCost) => a.totalUsd ?? (a.extraUsd > 0 ? a.extraUsd : null);
</script>

<Item.Root variant="outline" size="sm">
	<Item.Content>
		<div class="flex items-baseline justify-between">
			<Item.Title>{m.cost_title()}</Item.Title>
			{#if cost.totalUsd !== null}
				<span class="text-lg font-semibold tabular-nums">{usd(cost.totalUsd)}</span>
			{:else}
				<span class="text-lg font-semibold">{m.cost_no_estimate()}</span>
			{/if}
		</div>

		<dl class="flex flex-col gap-0.5 text-xs text-muted-foreground">
			{#if cost.estimateUsd !== null}
				<div class="flex justify-between">
					<dt>{m.cost_code_tab()}</dt>
					<dd class="tabular-nums">{m.cost_estimated({ amount: usd(cost.estimateUsd) })}</dd>
				</div>
			{/if}
			<div class="flex justify-between">
				<dt>{m.cost_extra_usage()}</dt>
				<dd class="tabular-nums">{usd(cost.extraUsd)}</dd>
			</div>
			<div class="flex justify-between">
				<dt>{m.cost_tokens()}</dt>
				<dd class="tabular-nums">{formatCountShort(cost.tokens)}</dd>
			</div>
		</dl>

		{#if cost.noEstimate === 'prices-unavailable'}
			<p class="text-xs text-muted-foreground">{m.cost_prices_unavailable()}</p>
		{/if}
		{#if cost.unpricedMessages > 0}
			<p class="text-[11px] text-muted-foreground">
				{m.cost_unpriced({ count: formatInteger(cost.unpricedMessages, formatTag()) })}
			</p>
		{/if}

		{#if cost.accounts.length > 0}
			<Separator class="my-2" />
			<Item.Group class="gap-2">
				{#each cost.accounts as account (account.id)}
					{@const dollars = dollarsOf(account)}
					<Item.Root size="xs" class="p-0" role="listitem">
						<Item.Content class="group-data-[size=xs]/item:gap-2">
							<div class="flex items-baseline justify-between text-xs">
								<span class="truncate font-medium">{account.label}</span>
								<span class="tabular-nums">
									<span class="text-muted-foreground">{formatCountShort(account.tokens)} · </span>
								{dollars === null ? '—' : usd(dollars)}
									{#if account.share !== null}
										<span class="text-muted-foreground"
											>{formatPercentValue(account.share * 100, formatTag())}</span
										>
									{/if}
								</span>
							</div>
							{#if account.share !== null}
								<Progress value={Math.round(account.share * 100)} aria-label={account.label} />
							{/if}
						</Item.Content>
					</Item.Root>
				{/each}
			</Item.Group>
		{/if}
	</Item.Content>
</Item.Root>
