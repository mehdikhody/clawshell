<script lang="ts">
	import { formatUsd } from '$lib/cost';
	import {
		cellsFor,
		formatDay,
		formatPercent,
		intensity,
		type Heatmap,
		type HeatmapDay,
		type HeatmapMode,
		type Level
	} from '$lib/heatmap';
	import * as Item from '$lib/components/ui/item';
	import * as ToggleGroup from '$lib/components/ui/toggle-group';
	import * as Tooltip from '$lib/components/ui/tooltip';
	import { m } from '$lib/paraglide/messages.js';
	import { formatTag } from '$lib/locale-state.svelte';

	let { heatmap }: { heatmap: Heatmap } = $props();

	let mode = $state<HeatmapMode>('usd');
	/** The day whose tooltip is open: hover, focus and tap all land here. */
	let openDate = $state<string | null>(null);

	const cells = $derived(cellsFor(heatmap.days));
	const maxUsd = $derived(Math.max(0, ...heatmap.days.map((d) => d.usd?.total ?? 0)));

	const levelClass: Record<Level, string> = {
		0: 'border-transparent bg-muted',
		1: 'border-transparent bg-primary/25',
		2: 'border-transparent bg-primary/50',
		3: 'border-transparent bg-primary/75',
		4: 'border-transparent bg-primary'
	};
	const noDataClass = 'border-dashed border-muted-foreground/40 bg-transparent';

	const valuesOf = (day: HeatmapDay) => (mode === 'usd' ? day.usd : day.percent);
	const levelOf = (day: HeatmapDay) => intensity(valuesOf(day)?.total ?? null, mode, maxUsd);
	const classOf = (level: Level | null) => (level === null ? noDataClass : levelClass[level]);

	const money = (amount: number) => formatUsd(amount, formatTag());
	const amount = (value: number) =>
		mode === 'usd' ? money(value) : formatPercent(value, formatTag());
	const total = (value: number) =>
		mode === 'usd'
			? m.cost_estimated({ amount: money(value) })
			: m.heatmap_total_percent({ percent: formatPercent(value, formatTag()) });
	const dateLabel = (date: string) => formatDay(date, formatTag());
	const labelOf = (day: HeatmapDay) => {
		const values = valuesOf(day);
		return values
			? m.heatmap_cell({ date: dateLabel(day.date), value: total(values.total) })
			: m.heatmap_cell_no_data({ date: dateLabel(day.date) });
	};
	const setOpen = (date: string, open: boolean) => {
		if (open) openDate = date;
		else if (openDate === date) openDate = null;
	};
</script>

<Item.Root variant="outline" size="sm">
	<Item.Content>
		<div class="flex items-center justify-between">
			<Item.Title>{m.heatmap_title()}</Item.Title>
			<ToggleGroup.Root
				type="single"
				variant="outline"
				size="sm"
				aria-label={m.heatmap_mode_label()}
				value={mode}
				onValueChange={(next) => {
					// A single-select group reports '' when the pressed item is pressed again.
					if (next === 'usd' || next === 'percent') mode = next;
				}}
			>
				<ToggleGroup.Item value="usd">{m.heatmap_mode_usd()}</ToggleGroup.Item>
				<ToggleGroup.Item value="percent">{m.heatmap_mode_percent()}</ToggleGroup.Item>
			</ToggleGroup.Root>
		</div>

		<Tooltip.Provider delayDuration={0} disableHoverableContent>
			<div
				role="group"
				aria-label={m.heatmap_title()}
				class="mx-auto grid w-max auto-cols-[1.75rem] grid-flow-col grid-rows-[repeat(7,1.75rem)] gap-1 py-1"
			>
				{#each cells as day, i (day?.date ?? `pad-${i}`)}
					{#if day}
						{@const values = valuesOf(day)}
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
								{#if values}
									<p class="tabular-nums">{total(values.total)}</p>
									<ul class="flex flex-col gap-0.5">
										{#each values.accounts as account (account.id)}
											<li class="flex justify-between gap-4">
												<span class="truncate">{account.label}</span>
												<span class="tabular-nums">{amount(account.value)}</span>
											</li>
										{/each}
									</ul>
								{:else}
									<p>{m.heatmap_no_data()}</p>
								{/if}
							</Tooltip.Content>
						</Tooltip.Root>
					{:else}
						<div aria-hidden="true"></div>
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

		{#if mode === 'usd' && heatmap.noEstimate === 'prices-unavailable'}
			<p class="text-xs text-muted-foreground">{m.heatmap_no_prices()}</p>
		{/if}
	</Item.Content>
</Item.Root>
