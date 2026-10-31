<script lang="ts">
	import { DropdownMenu as DropdownMenuPrimitive } from "bits-ui";
	import IconChevronRight from '@tabler/icons-svelte/icons/chevron-right';
	import { cn } from "$lib/utils.js";

	let {
		ref = $bindable(null),
		class: className,
		inset,
		children,
		...restProps
	}: DropdownMenuPrimitive.SubTriggerProps & {
		inset?: boolean;
	} = $props();
</script>

<DropdownMenuPrimitive.SubTrigger
	bind:ref
	data-slot="dropdown-menu-sub-trigger"
	data-inset={inset}
	class={cn(
		"focus:bg-accent focus:text-foreground data-open:bg-accent data-open:text-foreground not-data-[variant=destructive]:focus:**:text-foreground gap-2 rounded-sm px-2 py-1.5 text-sm data-inset:ps-8 [&_svg:not([class*='size-'])]:size-4 flex cursor-default items-center outline-hidden select-none [&_svg]:pointer-events-none [&_svg]:shrink-0",
		className
	)}
	{...restProps}
>
	{@render children?.()}
	<IconChevronRight class="cn-rtl-flip ms-auto" />
</DropdownMenuPrimitive.SubTrigger>
