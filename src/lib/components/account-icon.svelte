<script lang="ts">
	import { accountIcon, claudeIcon } from '$lib/accounts';

	let {
		badge,
		colour,
		isDefault = false
	}: { badge: string; colour: string; isDefault?: boolean } = $props();

	/** The default account has no badge: it shows Claude's own icon. */
	const icon = $derived(isDefault ? claudeIcon() : accountIcon(badge, colour));
</script>

{#await icon then src}
	<img data-slot="account-badge" alt="" class="size-8" {src} />
{:catch}
	<span
		data-slot="account-badge"
		aria-hidden="true"
		class="flex size-8 items-center justify-center rounded-md text-sm font-semibold text-neutral-950"
		style:background-color={colour}>{badge}</span
	>
{/await}
