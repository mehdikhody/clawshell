<script lang="ts">
	import { formatCountShort, formatUsd, formatUsdShort } from '$lib/cost';
	import {
		cellsFor,
		formatDay,
		intensity,
		type Heatmap,
		type HeatmapDay,
		type Level
	} from '$lib/heatmap';
	import * as Item from '$lib/components/ui/item';
	import * as Tooltip from '$lib/components/ui/tooltip';
	import { m } from '$lib/paraglide/messages.js';
	import { active, formatTag } from '$lib/locale-state.svelte';
	import { weekStartOf } from '$lib/locale';

	let { heatmap }: { heatmap: Heatmap } = $props();

	/** The day whose tooltip is open: hover, focus and tap all land here. */
	let openDate = $state<string | null>(null);

	/** The week's first day, so the grid and its labels start on it: Saturday under Jalali. */
	const firstDay = weekStartOf(active.locale, active.calendar);
	const cells = $derived(cellsFor(heatmap.days, firstDay));
	/** Row 0 is the first day of the week. 7 Jan 2024 was a Sunday, so 7 + the day counts from it. */
	const weekdayLabels = Array.from({ length: 7 }, (_, row) =>
		new Intl.DateTimeFormat(formatTag(), { weekday: 'narrow' }).format(
			new Date(2024, 0, 7 + ((firstDay + row) % 7))
		)
	);
	const maxUsd = $derived(Math.max(0, ...heatmap.days.map((d) => d.usd?.total ?? 0)));

	const levelClass: Record<Level, string> = {
		0: 'border-transparent bg-muted',
		1: 'border-transparent bg-primary/25',
		2: 'border-transparent bg-primary/50',
		3: 'border-transparent bg-primary/75',
		4: 'border-transparent bg-primary'
	};
	const noDataClass = 'border-dashed border-muted-foreground/40 bg-transparent';

	const levelOf = (day: HeatmapDay) => intensity(day.usd?.total ?? null, maxUsd);
	const classOf = (level: Level | null) => (level === null ? noDataClass : levelClass[level]);

	const money = (amount: number) => formatUsd(amount, formatTag());
	/** The tooltip's short form, like $12.3K. */
	const moneyShort = (amount: number) => formatUsdShort(amount, formatTag());
	const countShort = (count: number) => formatCountShort(count, formatTag());
	const dateLabel = (date: string) => formatDay(date, formatTag());
	const labelOf = (day: HeatmapDay) =>
		day.usd
			? m.heatmap_cell({
					date: dateLabel(day.date),
					value: m.cost_estimated({ amount: money(day.usd.total) })
				})
			: m.heatmap_cell_no_data({ date: dateLabel(day.date) });
	const setOpen = (date: string, open: boolean) => {
		if (open) openDate = date;
		else if (openDate === date) openDate = null;
	};
</script>

<Item.Root variant="outline" size="sm">
	<Item.Content>
		<Item.Title>{m.heatmap_title()}</Item.Title>

		<Tooltip.Provider delayDuration={0} disableHoverableContent>
			<div
				role="group"
				aria-label={m.heatmap_title()}
				class="mx-auto grid w-max auto-cols-[1.75rem] grid-flow-col grid-rows-[repeat(7,1.75rem)] gap-1 py-1"
			>
				{#each weekdayLabels as label, row (row)}
					<span
						aria-hidden="true"
						class="flex items-center justify-center text-[10px] text-muted-foreground">{label}</span
					>
				{/each}
				{#each cells as day, i (day?.date ?? `future-${i}`)}
					{#if day}
						<Tooltip.Root
							open={openDate === day.date}
							onOpenChange={(open) => setOpen(day.date, open)}
							disableCloseOnTriggerClick
						>
							<Tooltip.Trigger
								aria-label={labelOf(day)}
								onclick={() => setOpen(day.date, true)}
								class="size-7 rounded-sm border outline-none focus-visible:ring-2 focus-visible:ring-ring {classOf(
									levelOf(day)
								)}"
							/>
							<Tooltip.Content
								data-testid="heatmap-tooltip"
								class="pointer-events-none flex-col items-stretch gap-1"
							>
								<p class="font-medium">{dateLabel(day.date)}</p>
								{#if day.usd}
									<p class="tabular-nums">
										{m.cost_estimated({ amount: moneyShort(day.usd.total) })}
									</p>
								{/if}
								{#if day.tokens !== null}
									<p class="tabular-nums">
										{m.heatmap_tokens({ count: countShort(day.tokens.total) })}
									</p>
									{@const used = day.tokens.accounts.filter((a) => a.value > 0)}
									{#if used.length > 0}
										<hr class="my-0.5 border-current/20" />
										<ul class="flex flex-col gap-0.5">
											{#each used as account (account.id)}
												{@const usd = day.usd?.accounts.find((a) => a.id === account.id)}
												<li class="flex justify-between gap-4">
													<span class="flex min-w-0 items-center gap-1.5">
														<span
															class="size-2 shrink-0 rounded-full"
															style:background-color={account.colour}
															aria-hidden="true"
														></span>
														<span class="truncate">{account.label}</span>
													</span>
													<span class="tabular-nums"
														>{countShort(account.value)}{#if usd}&nbsp;· {moneyShort(
																usd.value
															)}{/if}</span
													>
												</li>
											{/each}
										</ul>
									{/if}
								{/if}
								{#if day.usd === null && day.tokens === null}
									<p>{m.heatmap_no_data()}</p>
								{/if}
							</Tooltip.Content>
						</Tooltip.Root>
					{:else}
						<div
							aria-hidden="true"
							data-testid="heatmap-future"
							class="size-7 rounded-sm bg-muted/30"
						></div>
					{/if}
				{/each}
			</div>
		</Tooltip.Provider>

		<div class="flex items-center justify-end gap-1 text-[11px] text-muted-foreground">
			<span class="size-3 rounded-sm border {noDataClass}"></span>
			<span class="me-2">{m.heatmap_no_data()}</span>
			<span>{m.heatmap_legend_less()}</span>
			{#each [0, 1, 2, 3, 4] as const as level (level)}
				<span class="size-3 rounded-sm border {levelClass[level]}"></span>
			{/each}
			<span>{m.heatmap_legend_more()}</span>
		</div>

		{#if heatmap.noEstimate === 'prices-unavailable'}
			<p class="text-xs text-muted-foreground">{m.heatmap_no_prices()}</p>
		{/if}
	</Item.Content>
</Item.Root>
