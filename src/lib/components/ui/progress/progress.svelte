<script lang="ts">
	import { Progress as ProgressPrimitive } from "bits-ui";
	import { cn, type WithoutChildrenOrChild } from "$lib/utils.js";

	let {
		ref = $bindable(null),
		class: className,
		max = 100,
		value,
		...restProps
	}: WithoutChildrenOrChild<ProgressPrimitive.RootProps> = $props();
</script>

<ProgressPrimitive.Root
	bind:ref
	data-slot="progress"
	class={cn("bg-muted h-1.5 rounded-full relative flex w-full items-center overflow-x-hidden", className)}
	{value}
	{max}
	{...restProps}
>
	{#if value === null}
		<!-- No known value: a short bar sweeps back and forth along the track. -->
		<div
			data-slot="progress-indicator"
			class="bg-primary h-full w-2/5 animate-indeterminate rounded-full"
		></div>
	{:else}
		<div
			data-slot="progress-indicator"
			class="bg-primary size-full flex-1 -translate-x-(--progress-gap) transition-all rtl:translate-x-(--progress-gap)"
			style="--progress-gap: {100 - (100 * (value ?? 0)) / (max ?? 1)}%"
		></div>
	{/if}
</ProgressPrimitive.Root>
