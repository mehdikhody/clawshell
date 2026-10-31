<script lang="ts">
	import IconAlertTriangle from '@tabler/icons-svelte/icons/alert-triangle';
	import * as Alert from '$lib/components/ui/alert';
	import { Button } from '$lib/components/ui/button';
	import * as Dialog from '$lib/components/ui/dialog';
	import { Progress } from '$lib/components/ui/progress';
	import { formatPercentValue } from '$lib/locale';
	import { formatTag } from '$lib/locale-state.svelte';
	import { m } from '$lib/paraglide/messages.js';
	import { installUpdate, updateState } from '$lib/update.svelte';

	let { open = $bindable(false) }: { open?: boolean } = $props();

	const current = $derived(updateState.current);
	/** While the installer runs there is nothing to cancel, so the dialog stays until the restart. */
	const busy = $derived(current.status === 'downloading' || current.status === 'installing');
	/** Null while the download size is unknown, which draws an indeterminate bar. */
	const percent = $derived(
		current.status === 'downloading' && current.total
			? Math.min(100, (current.downloaded / current.total) * 100)
			: null
	);
</script>

{#if current.status !== 'idle'}
	<Dialog.Root
		bind:open
		onOpenChange={(next) => {
			if (!next && busy) open = true;
		}}
	>
		<Dialog.Content showCloseButton={!busy} class="sm:max-w-lg">
			<Dialog.Header>
				<Dialog.Title class="text-base font-semibold">
					{m.update_title({ version: current.update.version })}
				</Dialog.Title>
				{#if current.update.body}
					<Dialog.Description>{m.update_notes()}</Dialog.Description>
				{/if}
			</Dialog.Header>

			{#if current.update.body}
				<div
					data-testid="update-notes"
					class="max-h-48 overflow-y-auto rounded-md border bg-muted/40 p-3 text-xs whitespace-pre-wrap"
				>
					{current.update.body}
				</div>
			{/if}

			{#if current.status === 'downloading'}
				<div class="flex flex-col gap-2">
					<div class="flex items-center justify-between text-xs text-muted-foreground">
						<span>{m.update_downloading()}</span>
						{#if percent !== null}
							<span>{formatPercentValue(percent, formatTag())}</span>
						{/if}
					</div>
					<Progress value={percent} />
				</div>
			{:else if current.status === 'installing'}
				<div class="flex flex-col gap-2">
					<span class="text-xs text-muted-foreground">{m.update_installing()}</span>
					<Progress value={null} />
				</div>
			{:else if current.status === 'failed'}
				<Alert.Root variant="destructive">
					<IconAlertTriangle />
					<Alert.Title>{m.update_failed()}</Alert.Title>
					<Alert.Description>{current.message}</Alert.Description>
				</Alert.Root>
			{/if}

			{#if !busy}
				<Dialog.Footer>
					<Button variant="outline" onclick={() => (open = false)}>{m.update_later()}</Button>
					<Button onclick={() => void installUpdate()}>
						{current.status === 'failed' ? m.update_retry() : m.update_now()}
					</Button>
				</Dialog.Footer>
			{/if}
		</Dialog.Content>
	</Dialog.Root>
{/if}
