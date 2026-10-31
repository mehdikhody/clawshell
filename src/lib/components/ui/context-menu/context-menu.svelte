<script lang="ts">
	import { ContextMenu as ContextMenuPrimitive } from "bits-ui";
	import { active } from "$lib/locale-state.svelte";
	import { directionOf } from "$lib/locale";

	let {
		open = $bindable(false),
		// bits-ui writes dir="ltr" on the menu unless told otherwise, which mirrors items the wrong way.
		dir = directionOf(active.locale),
		...restProps
	}: ContextMenuPrimitive.RootProps = $props();
</script>

<ContextMenuPrimitive.Root bind:open {dir} {...restProps} />
