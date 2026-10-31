<script lang="ts">
	import { accountIcon } from '$lib/accounts';

	let { badge, colour }: { badge: string; colour: string } = $props();
</script>

{#await accountIcon(badge, colour) then src}
	<img data-slot="account-badge" alt="" class="size-8" {src} />
{:catch}
	<span
		data-slot="account-badge"
		aria-hidden="true"
		class="flex size-8 items-center justify-center rounded-md text-sm font-semibold text-neutral-950"
		style:background-color={colour}>{badge}</span
	>
{/await}
