<script lang="ts">
	import { badgeVariants } from '$lib/components/ui/badge';
	import { m } from '$lib/paraglide/messages.js';
	import UpdateDialog from './update-dialog.svelte';

	// Mounted only while an update exists, so this resets whenever the state returns to idle.
	let open = $state(false);
</script>

<button type="button" class={[badgeVariants(), 'cursor-pointer']} onclick={() => (open = true)}>
	{m.update_available()}
</button>
<UpdateDialog bind:open />
