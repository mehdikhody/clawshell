<script lang="ts">
	import { onMount } from 'svelte';
	import IconAlertTriangle from '@tabler/icons-svelte/icons/alert-triangle';
	import { errorCode } from '$lib/accounts';
	import * as Alert from '$lib/components/ui/alert';
	import { Button } from '$lib/components/ui/button';
	import * as Dialog from '$lib/components/ui/dialog';
	import { m } from '$lib/paraglide/messages.js';
	import { onRoutingOffer, setSignInRouting } from '$lib/settings';

	/** The question, then (Store build only) the one Windows choice left to the user. */
	let step = $state<'closed' | 'offer' | 'default-apps'>('closed');
	let busy = $state(false);
	let failure = $state<string | null>(null);

	async function turnOn() {
		busy = true;
		failure = null;
		try {
			const change = await setSignInRouting(true);
			step = change.needsDefaultAppsChoice ? 'default-apps' : 'closed';
		} catch (error) {
			failure =
				errorCode(error) === 'routing-failed'
					? m.error_routing_failed()
					: m.error_settings_save_failed();
		} finally {
			busy = false;
		}
	}

	onMount(() => {
		// The core sends this once, after the first successful Add.
		const unlisten = onRoutingOffer(() => {
			failure = null;
			step = 'offer';
		});
		return () => void unlisten.then((stop) => stop()).catch(() => {});
	});
</script>

<Dialog.Root
	open={step !== 'closed'}
	onOpenChange={(open) => {
		if (!open) step = 'closed';
	}}
>
	<Dialog.Content class="sm:max-w-sm">
		<Dialog.Header>
			<Dialog.Title>
				{step === 'default-apps' ? m.routing_default_apps_title() : m.routing_offer_title()}
			</Dialog.Title>
			<Dialog.Description>
				{step === 'default-apps' ? m.routing_default_apps_body() : m.routing_offer_body()}
			</Dialog.Description>
		</Dialog.Header>
		{#if failure}
			<Alert.Root variant="destructive">
				<IconAlertTriangle />
				<Alert.Title>{failure}</Alert.Title>
			</Alert.Root>
		{/if}
		<Dialog.Footer>
			{#if step === 'default-apps'}
				<Button onclick={() => (step = 'closed')}>{m.routing_offer_done()}</Button>
			{:else}
				<Button variant="outline" onclick={() => (step = 'closed')}>
					{m.routing_offer_decline()}
				</Button>
				<Button onclick={turnOn} disabled={busy}>{m.routing_offer_accept()}</Button>
			{/if}
		</Dialog.Footer>
	</Dialog.Content>
</Dialog.Root>
