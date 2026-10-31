<script lang="ts">
	import { ContextMenu as ContextMenuPrimitive } from "bits-ui";
	import IconMinus from '@tabler/icons-svelte/icons/minus';
	import IconCheck from '@tabler/icons-svelte/icons/check';
	import { cn, type WithoutChildrenOrChild } from "$lib/utils.js";
	import type { Snippet } from "svelte";

	let {
		ref = $bindable(null),
		checked = $bindable(false),
		indeterminate = $bindable(false),
		class: className,
		children: childrenProp,
		closeOnSelect = false,
		...restProps
	}: WithoutChildrenOrChild<ContextMenuPrimitive.CheckboxItemProps> & {
		children?: Snippet;
	} = $props();
</script>

<ContextMenuPrimitive.CheckboxItem
	bind:ref
	{closeOnSelect}
	bind:checked
	bind:indeterminate
	data-slot="context-menu-checkbox-item"
	class={cn(
		"focus:bg-accent focus:text-foreground focus:**:text-foreground gap-2 rounded-sm py-1.5 pr-8 pl-2 text-sm data-inset:pl-8 [&_svg:not([class*='size-'])]:size-4 relative flex cursor-default items-center outline-hidden select-none data-disabled:pointer-events-none data-disabled:opacity-50 [&_svg]:pointer-events-none [&_svg]:shrink-0",
		className
	)}
	{...restProps}
>
	{#snippet children({ checked, indeterminate })}
		<span
			class="absolute right-2 flex items-center justify-center pointer-events-none"
			data-slot="context-menu-checkbox-item-indicator"
		>
			{#if indeterminate}
				<IconMinus  />
			{:else if checked}
				<IconCheck  />
			{/if}
		</span>
		{@render childrenProp?.()}
	{/snippet}
</ContextMenuPrimitive.CheckboxItem>
