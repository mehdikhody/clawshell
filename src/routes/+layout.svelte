<script lang="ts">
	import './layout.css';
	import DisclaimerDialog from '$lib/components/disclaimer-dialog.svelte';

	let { children } = $props();
</script>

{@render children()}
<DisclaimerDialog />
