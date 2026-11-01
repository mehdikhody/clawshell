<script lang="ts">
	import * as AlertDialog from '$lib/components/ui/alert-dialog';
	import { acceptDisclaimer, declineDisclaimer, disclaimer } from '$lib/disclaimer.svelte';
	import { m } from '$lib/paraglide/messages.js';

	let busy = $state(false);

	async function accept() {
		busy = true;
		try {
			await acceptDisclaimer();
		} catch (error) {
			// The dialog stays, so the user can try again.
			console.error('Accepting the disclaimer failed', error);
		} finally {
			busy = false;
		}
	}
</script>

<!-- Open exactly while unaccepted: the dialog's own requests to close are dropped, and Esc and a click outside are ignored. -->
<AlertDialog.Root bind:open={() => disclaimer.accepted === false, () => {}}>
	<AlertDialog.Content
		escapeKeydownBehavior="ignore"
		interactOutsideBehavior="ignore"
		class="sm:max-w-md"
	>
		<AlertDialog.Header>
			<AlertDialog.Title>{m.disclaimer_title()}</AlertDialog.Title>
			<AlertDialog.Description class="flex flex-col gap-2">
				{#each m.disclaimer_body().split('\n\n') as paragraph (paragraph)}
					<span>{paragraph}</span>
				{/each}
			</AlertDialog.Description>
		</AlertDialog.Header>
		<AlertDialog.Footer>
			<AlertDialog.Cancel onclick={() => void declineDisclaimer()}>
				{m.disclaimer_exit()}
			</AlertDialog.Cancel>
			<AlertDialog.Action disabled={busy} onclick={() => void accept()}>
				{m.disclaimer_accept()}
			</AlertDialog.Action>
		</AlertDialog.Footer>
	</AlertDialog.Content>
</AlertDialog.Root>
